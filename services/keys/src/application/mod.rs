use crate::{
    domain::{KeyBundle, OneTimePrekey, OneTimePrekeyState, SignedPrekey},
    errors::KeyServiceError,
    repository::{
        KeyBundleCommandRepository, KeyBundleRepository, KeyConstraintViolation,
        KeyRepositoryError, OneTimePrekeyRepository, PublishCurrentKeyBundleRecord,
        PublishSignedPrekeyRecord, SignedPrekeyCommandRepository, SignedPrekeyRepository,
    },
};
use async_trait::async_trait;
use chrono::{Timelike, Utc};
use reach_auth_types::{AuthScope, DeviceId, RequestContext};
use reach_identity_lifecycle::{
    AccountLifecycleState, DeviceLifecycleStatus, IdentityLifecycleReader,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishSignedPrekeyInput {
    pub device_id: DeviceId,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishKeyBundleInput {
    pub device_id: DeviceId,
    pub identity_key_public: Vec<u8>,
    pub identity_key_alg: String,
    pub signed_prekey_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishOneTimePrekeysInput {
    pub device_id: DeviceId,
    pub prekeys: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimOneTimePrekeyInput {
    pub device_id: DeviceId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchCurrentBundleInput {
    pub device_id: DeviceId,
}

#[async_trait]
pub trait KeyUseCases: Send + Sync {
    async fn publish_signed_prekey(
        &self,
        context: RequestContext,
        command: PublishSignedPrekeyInput,
    ) -> Result<SignedPrekey, KeyServiceError>;
    async fn publish_key_bundle(
        &self,
        context: RequestContext,
        command: PublishKeyBundleInput,
    ) -> Result<KeyBundle, KeyServiceError>;
    async fn publish_one_time_prekeys(
        &self,
        context: RequestContext,
        command: PublishOneTimePrekeysInput,
    ) -> Result<Vec<OneTimePrekey>, KeyServiceError>;
    async fn claim_one_time_prekey(
        &self,
        context: RequestContext,
        command: ClaimOneTimePrekeyInput,
    ) -> Result<OneTimePrekey, KeyServiceError>;
    async fn fetch_current_bundle(
        &self,
        context: RequestContext,
        command: FetchCurrentBundleInput,
    ) -> Result<KeyBundle, KeyServiceError>;
}

#[derive(Debug, Clone)]
pub struct KeyCommandService<R, L> {
    repository: Arc<R>,
    lifecycle_reader: Arc<L>,
}

impl<R, L> KeyCommandService<R, L> {
    pub fn new(repository: R, lifecycle_reader: L) -> Self {
        Self {
            repository: Arc::new(repository),
            lifecycle_reader: Arc::new(lifecycle_reader),
        }
    }
}

#[async_trait]
impl<R, L> KeyUseCases for KeyCommandService<R, L>
where
    R: KeyBundleRepository
        + KeyBundleCommandRepository
        + SignedPrekeyRepository
        + SignedPrekeyCommandRepository
        + OneTimePrekeyRepository
        + Send
        + Sync
        + 'static,
    L: IdentityLifecycleReader + Send + Sync + 'static,
{
    async fn publish_signed_prekey(
        &self,
        context: RequestContext,
        command: PublishSignedPrekeyInput,
    ) -> Result<SignedPrekey, KeyServiceError> {
        authorize(&context, AuthScope::KeysSignedPrekeyPublish)?;
        validate_device_id(command.device_id)?;
        validate_signed_prekey_material(&command.public_key)?;
        validate_signed_prekey_signature(&command.signature)?;

        self.ensure_active_device(command.device_id).await?;

        self.repository
            .publish_current_signed_prekey(&PublishSignedPrekeyRecord {
                signed_prekey_id: Uuid::now_v7(),
                device_id: command.device_id,
                public_key: command.public_key,
                signature: command.signature,
                created_at: db_timestamp(),
            })
            .await
            .map_err(map_key_repository_error)
    }

    async fn publish_key_bundle(
        &self,
        context: RequestContext,
        command: PublishKeyBundleInput,
    ) -> Result<KeyBundle, KeyServiceError> {
        authorize(&context, AuthScope::KeysBundlePublish)?;
        validate_device_id(command.device_id)?;
        validate_signed_prekey_id(command.signed_prekey_id)?;
        validate_identity_key_material(&command.identity_key_public)?;
        validate_identity_key_algorithm(&command.identity_key_alg)?;

        self.ensure_active_device(command.device_id).await?;

        let signed_prekey =
            SignedPrekeyRepository::get_by_id(self.repository.as_ref(), command.signed_prekey_id)
                .await
                .map_err(map_key_repository_error)?
                .ok_or(KeyServiceError::SignedPrekeyNotFound)?;

        if signed_prekey.device_id != command.device_id {
            return Err(KeyServiceError::SignedPrekeyDeviceMismatch);
        }

        self.repository
            .publish_current_bundle(&PublishCurrentKeyBundleRecord {
                bundle_id: Uuid::now_v7(),
                device_id: command.device_id,
                identity_key_public: command.identity_key_public,
                identity_key_alg: command.identity_key_alg.trim().to_owned(),
                signed_prekey_id: command.signed_prekey_id,
                published_at: db_timestamp(),
            })
            .await
            .map_err(map_key_repository_error)
    }

    async fn publish_one_time_prekeys(
        &self,
        context: RequestContext,
        command: PublishOneTimePrekeysInput,
    ) -> Result<Vec<OneTimePrekey>, KeyServiceError> {
        authorize(&context, AuthScope::KeysOneTimePrekeysPublish)?;
        validate_device_id(command.device_id)?;
        validate_one_time_prekey_batch(&command.prekeys)?;

        self.ensure_active_device(command.device_id).await?;

        let created_at = db_timestamp();
        let prekeys = command
            .prekeys
            .into_iter()
            .map(|public_key| OneTimePrekey {
                prekey_id: Uuid::now_v7(),
                device_id: command.device_id,
                public_key,
                state: OneTimePrekeyState::Available,
                created_at,
                claimed_at: None,
            })
            .collect::<Vec<_>>();

        self.repository
            .insert_batch(&prekeys)
            .await
            .map_err(map_key_repository_error)?;

        Ok(prekeys)
    }

    async fn claim_one_time_prekey(
        &self,
        context: RequestContext,
        command: ClaimOneTimePrekeyInput,
    ) -> Result<OneTimePrekey, KeyServiceError> {
        authorize(&context, AuthScope::KeysOneTimePrekeyClaim)?;
        validate_device_id(command.device_id)?;

        self.ensure_active_device(command.device_id).await?;

        self.repository
            .claim_next_available(command.device_id)
            .await
            .map_err(map_key_repository_error)?
            .ok_or(KeyServiceError::NoAvailableOneTimePrekeys)
    }

    async fn fetch_current_bundle(
        &self,
        context: RequestContext,
        command: FetchCurrentBundleInput,
    ) -> Result<KeyBundle, KeyServiceError> {
        authorize(&context, AuthScope::KeysBundleRead)?;
        validate_device_id(command.device_id)?;

        self.ensure_active_device(command.device_id).await?;

        KeyBundleRepository::get_current(self.repository.as_ref(), command.device_id)
            .await
            .map_err(map_key_repository_error)?
            .ok_or(KeyServiceError::KeyBundleNotFound)
    }
}

impl<R, L> KeyCommandService<R, L>
where
    L: IdentityLifecycleReader + Send + Sync + 'static,
{
    async fn ensure_active_device(&self, device_id: DeviceId) -> Result<(), KeyServiceError> {
        let device = self
            .lifecycle_reader
            .get_device(device_id)
            .await
            .map_err(KeyServiceError::Lifecycle)?
            .ok_or(KeyServiceError::DeviceNotFound)?;

        if device.status != DeviceLifecycleStatus::Active {
            return Err(KeyServiceError::DeviceNotActive);
        }

        let account = self
            .lifecycle_reader
            .get_account(device.account_id)
            .await
            .map_err(KeyServiceError::Lifecycle)?
            .ok_or(KeyServiceError::AccountNotFound)?;

        if account.state != AccountLifecycleState::Active {
            return Err(KeyServiceError::AccountNotActive);
        }

        Ok(())
    }
}

fn validate_device_id(device_id: DeviceId) -> Result<(), KeyServiceError> {
    if device_id.0.is_nil() {
        return Err(KeyServiceError::InvalidDeviceId);
    }

    Ok(())
}

fn authorize(context: &RequestContext, scope: AuthScope) -> Result<(), KeyServiceError> {
    if !context.has_scope(scope) {
        return Err(KeyServiceError::InsufficientScope);
    }

    Ok(())
}

fn validate_signed_prekey_id(signed_prekey_id: Uuid) -> Result<(), KeyServiceError> {
    if signed_prekey_id.is_nil() {
        return Err(KeyServiceError::InvalidSignedPrekeyId);
    }

    Ok(())
}

fn validate_signed_prekey_material(public_key: &[u8]) -> Result<(), KeyServiceError> {
    if public_key.is_empty() || public_key.len() > 512 {
        return Err(KeyServiceError::InvalidSignedPrekeyMaterial);
    }

    Ok(())
}

fn validate_signed_prekey_signature(signature: &[u8]) -> Result<(), KeyServiceError> {
    if signature.is_empty() || signature.len() > 1024 {
        return Err(KeyServiceError::InvalidSignedPrekeySignature);
    }

    Ok(())
}

fn validate_identity_key_material(identity_key_public: &[u8]) -> Result<(), KeyServiceError> {
    if identity_key_public.is_empty() || identity_key_public.len() > 512 {
        return Err(KeyServiceError::InvalidIdentityKeyMaterial);
    }

    Ok(())
}

fn validate_identity_key_algorithm(identity_key_alg: &str) -> Result<(), KeyServiceError> {
    let trimmed = identity_key_alg.trim();

    if trimmed.is_empty() || trimmed.len() > 32 {
        return Err(KeyServiceError::InvalidIdentityKeyAlgorithm);
    }

    Ok(())
}

fn validate_one_time_prekey_batch(prekeys: &[Vec<u8>]) -> Result<(), KeyServiceError> {
    if prekeys.is_empty() || prekeys.len() > 128 {
        return Err(KeyServiceError::InvalidOneTimePrekeyBatch);
    }

    if prekeys
        .iter()
        .any(|prekey| prekey.is_empty() || prekey.len() > 512)
    {
        return Err(KeyServiceError::InvalidOneTimePrekeyMaterial);
    }

    Ok(())
}

fn db_timestamp() -> chrono::DateTime<chrono::Utc> {
    let timestamp = Utc::now();
    timestamp
        .with_nanosecond((timestamp.nanosecond() / 1_000) * 1_000)
        .expect("timestamp nanoseconds should remain valid after microsecond truncation")
}

fn map_key_repository_error(error: KeyRepositoryError) -> KeyServiceError {
    match error {
        KeyRepositoryError::Constraint(KeyConstraintViolation::SignedPrekeyAlreadyExists) => {
            KeyServiceError::SignedPrekeyAlreadyExists
        }
        KeyRepositoryError::Constraint(KeyConstraintViolation::KeyBundleAlreadyExists) => {
            KeyServiceError::KeyBundleAlreadyExists
        }
        other => KeyServiceError::Storage(other),
    }
}
