use crate::{
    domain::{AcceptedEncryptedEnvelope, EncryptedEnvelope, PrekeyResolutionMode},
    repository::{
        AcceptEnvelopeRecord, AcceptedEnvelopeRepository, EnvelopeCommandRepository,
        MessagingIngressConstraintViolation, MessagingIngressRepositoryError,
        RecipientKeyMaterialFailure,
    },
};
use async_trait::async_trait;
use reach_auth_types::{AccountId, DeviceId};
use reach_key_material::{claim_next_available_one_time_prekey, fetch_current_key_bundle};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CockroachMessagingIngressRepository {
    pool: PgPool,
}

impl CockroachMessagingIngressRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl AcceptedEnvelopeRepository for CockroachMessagingIngressRepository {
    async fn get_by_id(
        &self,
        envelope_id: Uuid,
    ) -> Result<Option<AcceptedEncryptedEnvelope>, MessagingIngressRepositoryError> {
        let row = sqlx::query_as::<_, AcceptedEnvelopeRow>(
            r#"
            SELECT
                envelope_id,
                sender_account_id,
                sender_device_id,
                recipient_account_id,
                recipient_device_id,
                encrypted_payload,
                content_type,
                client_timestamp,
                replay_nonce,
                payload_version,
                accepted_at,
                recipient_bundle_id,
                recipient_signed_prekey_id,
                claimed_one_time_prekey_id,
                prekey_resolution_mode
            FROM messaging_ingress.accepted_envelopes
            WHERE envelope_id = $1
            "#,
        )
        .bind(envelope_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(MessagingIngressRepositoryError::Database)?;

        row.map(TryInto::try_into).transpose()
    }
}

#[async_trait]
impl EnvelopeCommandRepository for CockroachMessagingIngressRepository {
    async fn accept_envelope(
        &self,
        command: &AcceptEnvelopeRecord,
    ) -> Result<AcceptedEncryptedEnvelope, MessagingIngressRepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(MessagingIngressRepositoryError::Database)?;

        sqlx::query(
            r#"
            INSERT INTO messaging_ingress.envelope_replay_records (
                envelope_id,
                sender_account_id,
                sender_device_id,
                replay_nonce,
                reserved_at
            )
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(command.envelope.envelope_id)
        .bind(command.envelope.sender_account_id.0)
        .bind(command.envelope.sender_device_id.0)
        .bind(&command.envelope.replay_nonce)
        .bind(command.replay_reserved_at)
        .execute(&mut *transaction)
        .await
        .map_err(map_replay_insert_error)?;

        let current_bundle =
            fetch_current_key_bundle(&mut *transaction, command.envelope.recipient_device_id)
                .await
                .map_err(MessagingIngressRepositoryError::KeyMaterial)?
                .ok_or(
                    MessagingIngressRepositoryError::RecipientKeyMaterialUnavailable(
                        RecipientKeyMaterialFailure::CurrentBundleMissing,
                    ),
                )?;

        let claimed_one_time_prekey_id = match command.prekey_resolution_mode {
            PrekeyResolutionMode::CurrentBundleOnly => None,
            PrekeyResolutionMode::CurrentBundleAndOneTimePrekey => {
                let claimed = claim_next_available_one_time_prekey(
                    &mut *transaction,
                    command.envelope.recipient_device_id,
                    command.accepted_at,
                )
                .await
                .map_err(MessagingIngressRepositoryError::KeyMaterial)?
                .ok_or(
                    MessagingIngressRepositoryError::RecipientKeyMaterialUnavailable(
                        RecipientKeyMaterialFailure::OneTimePrekeyUnavailable,
                    ),
                )?;

                Some(claimed.prekey_id)
            }
        };

        let accepted = sqlx::query_as::<_, AcceptedEnvelopeRow>(
            r#"
            INSERT INTO messaging_ingress.accepted_envelopes (
                envelope_id,
                sender_account_id,
                sender_device_id,
                recipient_account_id,
                recipient_device_id,
                encrypted_payload,
                content_type,
                client_timestamp,
                replay_nonce,
                payload_version,
                accepted_at,
                recipient_bundle_id,
                recipient_signed_prekey_id,
                claimed_one_time_prekey_id,
                prekey_resolution_mode
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
            )
            RETURNING
                envelope_id,
                sender_account_id,
                sender_device_id,
                recipient_account_id,
                recipient_device_id,
                encrypted_payload,
                content_type,
                client_timestamp,
                replay_nonce,
                payload_version,
                accepted_at,
                recipient_bundle_id,
                recipient_signed_prekey_id,
                claimed_one_time_prekey_id,
                prekey_resolution_mode
            "#,
        )
        .bind(command.envelope.envelope_id)
        .bind(command.envelope.sender_account_id.0)
        .bind(command.envelope.sender_device_id.0)
        .bind(command.envelope.recipient_account_id.0)
        .bind(command.envelope.recipient_device_id.0)
        .bind(&command.envelope.encrypted_payload)
        .bind(&command.envelope.content_type)
        .bind(command.envelope.client_timestamp)
        .bind(&command.envelope.replay_nonce)
        .bind(&command.envelope.payload_version)
        .bind(command.accepted_at)
        .bind(current_bundle.bundle_id)
        .bind(current_bundle.signed_prekey_id)
        .bind(claimed_one_time_prekey_id)
        .bind(command.prekey_resolution_mode.as_str())
        .fetch_one(&mut *transaction)
        .await
        .map_err(MessagingIngressRepositoryError::Database)?;

        transaction
            .commit()
            .await
            .map_err(MessagingIngressRepositoryError::Database)?;

        accepted.try_into()
    }
}

#[derive(Debug, FromRow)]
struct AcceptedEnvelopeRow {
    envelope_id: Uuid,
    sender_account_id: Uuid,
    sender_device_id: Uuid,
    recipient_account_id: Uuid,
    recipient_device_id: Uuid,
    encrypted_payload: Vec<u8>,
    content_type: String,
    client_timestamp: chrono::DateTime<chrono::Utc>,
    replay_nonce: Vec<u8>,
    payload_version: String,
    accepted_at: chrono::DateTime<chrono::Utc>,
    recipient_bundle_id: Uuid,
    recipient_signed_prekey_id: Uuid,
    claimed_one_time_prekey_id: Option<Uuid>,
    prekey_resolution_mode: String,
}

impl TryFrom<AcceptedEnvelopeRow> for AcceptedEncryptedEnvelope {
    type Error = MessagingIngressRepositoryError;

    fn try_from(value: AcceptedEnvelopeRow) -> Result<Self, Self::Error> {
        Ok(Self {
            envelope: EncryptedEnvelope {
                envelope_id: value.envelope_id,
                sender_account_id: AccountId(value.sender_account_id),
                sender_device_id: DeviceId(value.sender_device_id),
                recipient_account_id: AccountId(value.recipient_account_id),
                recipient_device_id: DeviceId(value.recipient_device_id),
                encrypted_payload: value.encrypted_payload,
                content_type: value.content_type,
                client_timestamp: value.client_timestamp,
                replay_nonce: value.replay_nonce,
                payload_version: value.payload_version,
            },
            accepted_at: value.accepted_at,
            recipient_bundle_id: value.recipient_bundle_id,
            recipient_signed_prekey_id: value.recipient_signed_prekey_id,
            claimed_one_time_prekey_id: value.claimed_one_time_prekey_id,
            prekey_resolution_mode: PrekeyResolutionMode::try_from(
                value.prekey_resolution_mode.as_str(),
            )
            .map_err(MessagingIngressRepositoryError::InvalidStoredPrekeyResolutionMode)?,
        })
    }
}

fn map_replay_insert_error(error: sqlx::Error) -> MessagingIngressRepositoryError {
    match unique_constraint_name(&error) {
        Some("envelope_replay_records_pkey") => MessagingIngressRepositoryError::Constraint(
            MessagingIngressConstraintViolation::EnvelopeAlreadyExists,
        ),
        Some("envelope_replay_records_sender_device_replay_nonce_key") => {
            MessagingIngressRepositoryError::Constraint(
                MessagingIngressConstraintViolation::ReplayNonceConflict,
            )
        }
        _ => MessagingIngressRepositoryError::Database(error),
    }
}

fn unique_constraint_name(error: &sqlx::Error) -> Option<&str> {
    match error {
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("23505") =>
        {
            database_error.constraint()
        }
        _ => None,
    }
}
