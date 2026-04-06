use crate::{
    domain::{AcceptedEncryptedEnvelope, EncryptedEnvelope, PrekeyResolutionMode},
    errors::{map_repository_error, MessagingIngressError},
    repository::{AcceptEnvelopeRecord, EnvelopeCommandRepository},
};
use async_trait::async_trait;
use chrono::{Duration, Timelike, Utc};
use reach_auth_types::{AccountId, AuthScope, DeviceId, RequestContext};
use reach_identity_lifecycle::{
    AccountLifecycleState, DeviceLifecycleStatus, IdentityLifecycleReader,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

const MAX_ENCRYPTED_PAYLOAD_BYTES: usize = 131_072;
const MAX_CONTENT_TYPE_LEN: usize = 64;
const MAX_PAYLOAD_VERSION_LEN: usize = 32;
const MAX_REPLAY_NONCE_LEN: usize = 64;
const MIN_REPLAY_NONCE_LEN: usize = 16;
const MAX_CLIENT_TIMESTAMP_AGE_DAYS: i64 = 7;
const MAX_CLIENT_TIMESTAMP_FUTURE_MINUTES: i64 = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptEncryptedEnvelopeInput {
    pub envelope_id: Uuid,
    pub sender_account_id: AccountId,
    pub sender_device_id: DeviceId,
    pub recipient_account_id: AccountId,
    pub recipient_device_id: DeviceId,
    pub encrypted_payload: Vec<u8>,
    pub content_type: String,
    pub client_timestamp: chrono::DateTime<chrono::Utc>,
    pub replay_nonce: Vec<u8>,
    pub payload_version: String,
    pub prekey_resolution_mode: PrekeyResolutionMode,
}

#[async_trait]
pub trait MessagingIngressUseCases: Send + Sync {
    async fn accept_encrypted_envelope(
        &self,
        context: RequestContext,
        command: AcceptEncryptedEnvelopeInput,
    ) -> Result<AcceptedEncryptedEnvelope, MessagingIngressError>;
}

#[derive(Debug, Clone)]
pub struct MessagingIngressCommandService<R, L> {
    repository: Arc<R>,
    lifecycle_reader: Arc<L>,
}

impl<R, L> MessagingIngressCommandService<R, L> {
    pub fn new(repository: R, lifecycle_reader: L) -> Self {
        Self {
            repository: Arc::new(repository),
            lifecycle_reader: Arc::new(lifecycle_reader),
        }
    }
}

#[async_trait]
impl<R, L> MessagingIngressUseCases for MessagingIngressCommandService<R, L>
where
    R: EnvelopeCommandRepository + Send + Sync + 'static,
    L: IdentityLifecycleReader + Send + Sync + 'static,
{
    async fn accept_encrypted_envelope(
        &self,
        context: RequestContext,
        command: AcceptEncryptedEnvelopeInput,
    ) -> Result<AcceptedEncryptedEnvelope, MessagingIngressError> {
        authorize(&context, AuthScope::MessagingIngressEnvelopeAccept)?;
        validate_command(&command)?;

        ensure_active_account_device_pair(
            self.lifecycle_reader.as_ref(),
            command.sender_account_id,
            command.sender_device_id,
            AccountDeviceRole::Sender,
        )
        .await?;
        ensure_active_account_device_pair(
            self.lifecycle_reader.as_ref(),
            command.recipient_account_id,
            command.recipient_device_id,
            AccountDeviceRole::Recipient,
        )
        .await?;

        let accepted_at = db_timestamp();
        let envelope = EncryptedEnvelope {
            envelope_id: command.envelope_id,
            sender_account_id: command.sender_account_id,
            sender_device_id: command.sender_device_id,
            recipient_account_id: command.recipient_account_id,
            recipient_device_id: command.recipient_device_id,
            encrypted_payload: command.encrypted_payload,
            content_type: command.content_type.trim().to_owned(),
            client_timestamp: command.client_timestamp,
            replay_nonce: command.replay_nonce,
            payload_version: command.payload_version.trim().to_owned(),
        };

        self.repository
            .accept_envelope(&AcceptEnvelopeRecord {
                envelope,
                accepted_at,
                replay_reserved_at: accepted_at,
                prekey_resolution_mode: command.prekey_resolution_mode,
            })
            .await
            .map_err(map_repository_error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccountDeviceRole {
    Sender,
    Recipient,
}

async fn ensure_active_account_device_pair<L>(
    lifecycle_reader: &L,
    account_id: AccountId,
    device_id: DeviceId,
    role: AccountDeviceRole,
) -> Result<(), MessagingIngressError>
where
    L: IdentityLifecycleReader + Send + Sync + 'static,
{
    let account = lifecycle_reader
        .get_account(account_id)
        .await
        .map_err(MessagingIngressError::Lifecycle)?
        .ok_or(match role {
            AccountDeviceRole::Sender => MessagingIngressError::SenderAccountNotFound,
            AccountDeviceRole::Recipient => MessagingIngressError::RecipientAccountNotFound,
        })?;

    if account.state != AccountLifecycleState::Active {
        return Err(match role {
            AccountDeviceRole::Sender => MessagingIngressError::SenderAccountNotActive,
            AccountDeviceRole::Recipient => MessagingIngressError::RecipientAccountNotActive,
        });
    }

    let device = lifecycle_reader
        .get_device(device_id)
        .await
        .map_err(MessagingIngressError::Lifecycle)?
        .ok_or(match role {
            AccountDeviceRole::Sender => MessagingIngressError::SenderDeviceNotFound,
            AccountDeviceRole::Recipient => MessagingIngressError::RecipientDeviceNotFound,
        })?;

    if device.status != DeviceLifecycleStatus::Active {
        return Err(match role {
            AccountDeviceRole::Sender => MessagingIngressError::SenderDeviceNotActive,
            AccountDeviceRole::Recipient => MessagingIngressError::RecipientDeviceNotActive,
        });
    }

    if device.account_id != account_id {
        return Err(match role {
            AccountDeviceRole::Sender => MessagingIngressError::SenderDeviceAccountMismatch,
            AccountDeviceRole::Recipient => MessagingIngressError::RecipientDeviceAccountMismatch,
        });
    }

    Ok(())
}

fn authorize(context: &RequestContext, scope: AuthScope) -> Result<(), MessagingIngressError> {
    if !context.has_scope(scope) {
        return Err(MessagingIngressError::InsufficientScope);
    }

    Ok(())
}

fn validate_command(command: &AcceptEncryptedEnvelopeInput) -> Result<(), MessagingIngressError> {
    if command.envelope_id.is_nil() {
        return Err(MessagingIngressError::InvalidEnvelopeId);
    }

    if command.sender_account_id.0.is_nil() {
        return Err(MessagingIngressError::InvalidSenderAccountId);
    }

    if command.sender_device_id.0.is_nil() {
        return Err(MessagingIngressError::InvalidSenderDeviceId);
    }

    if command.recipient_account_id.0.is_nil() {
        return Err(MessagingIngressError::InvalidRecipientAccountId);
    }

    if command.recipient_device_id.0.is_nil() {
        return Err(MessagingIngressError::InvalidRecipientDeviceId);
    }

    if command.encrypted_payload.is_empty() {
        return Err(MessagingIngressError::EmptyEncryptedPayload);
    }

    if command.encrypted_payload.len() > MAX_ENCRYPTED_PAYLOAD_BYTES {
        return Err(MessagingIngressError::PayloadTooLarge);
    }

    validate_token_label(
        command.content_type.trim(),
        MAX_CONTENT_TYPE_LEN,
        MessagingIngressError::InvalidContentType,
    )?;
    validate_token_label(
        command.payload_version.trim(),
        MAX_PAYLOAD_VERSION_LEN,
        MessagingIngressError::InvalidPayloadVersion,
    )?;

    if command.replay_nonce.len() < MIN_REPLAY_NONCE_LEN
        || command.replay_nonce.len() > MAX_REPLAY_NONCE_LEN
    {
        return Err(MessagingIngressError::InvalidReplayNonce);
    }

    let now = Utc::now();
    if command.client_timestamp < now - Duration::days(MAX_CLIENT_TIMESTAMP_AGE_DAYS) {
        return Err(MessagingIngressError::ClientTimestampTooOld);
    }

    if command.client_timestamp > now + Duration::minutes(MAX_CLIENT_TIMESTAMP_FUTURE_MINUTES) {
        return Err(MessagingIngressError::ClientTimestampTooFarInFuture);
    }

    Ok(())
}

fn validate_token_label(
    value: &str,
    max_length: usize,
    error: MessagingIngressError,
) -> Result<(), MessagingIngressError> {
    if value.is_empty() || value.len() > max_length {
        return Err(error);
    }

    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    {
        return Err(error);
    }

    Ok(())
}

fn db_timestamp() -> chrono::DateTime<chrono::Utc> {
    let timestamp = Utc::now();
    timestamp
        .with_nanosecond((timestamp.nanosecond() / 1_000) * 1_000)
        .expect("timestamp nanoseconds should remain valid after microsecond truncation")
}
