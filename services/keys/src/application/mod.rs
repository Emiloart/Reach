use crate::{
    domain::{KeyBundle, OneTimePrekey, OneTimePrekeyState},
    errors::KeyServiceError,
    repository::{
        KeyBundleCommandRepository, KeyBundleRepository, KeyConstraintViolation,
        KeyRepositoryError, OneTimePrekeyRepository, PublishCurrentKeyBundleRecord,
        SignedPrekeyRepository,
    },
};
use async_trait::async_trait;
use chrono::{Timelike, Utc};
use reach_auth_types::DeviceId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

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
    async fn publish_key_bundle(
        &self,
        command: PublishKeyBundleInput,
    ) -> Result<KeyBundle, KeyServiceError>;
    async fn publish_one_time_prekeys(
        &self,
        command: PublishOneTimePrekeysInput,
    ) -> Result<Vec<OneTimePrekey>, KeyServiceError>;
    async fn claim_one_time_prekey(
        &self,
        command: ClaimOneTimePrekeyInput,
    ) -> Result<OneTimePrekey, KeyServiceError>;
    async fn fetch_current_bundle(
        &self,
        command: FetchCurrentBundleInput,
    ) -> Result<KeyBundle, KeyServiceError>;
}

#[derive(Debug, Clone)]
pub struct KeyCommandService<R> {
    repository: Arc<R>,
}

impl<R> KeyCommandService<R> {
    pub fn new(repository: R) -> Self {
        Self {
            repository: Arc::new(repository),
        }
    }
}

#[async_trait]
impl<R> KeyUseCases for KeyCommandService<R>
where
    R: KeyBundleRepository
        + KeyBundleCommandRepository
        + SignedPrekeyRepository
        + OneTimePrekeyRepository
        + Send
        + Sync
        + 'static,
{
    async fn publish_key_bundle(
        &self,
        command: PublishKeyBundleInput,
    ) -> Result<KeyBundle, KeyServiceError> {
        validate_device_id(command.device_id)?;
        validate_signed_prekey_id(command.signed_prekey_id)?;
        validate_identity_key_material(&command.identity_key_public)?;
        validate_identity_key_algorithm(&command.identity_key_alg)?;

        let signed_prekey = self
            .repository
            .get_by_id(command.signed_prekey_id)
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
        command: PublishOneTimePrekeysInput,
    ) -> Result<Vec<OneTimePrekey>, KeyServiceError> {
        validate_device_id(command.device_id)?;
        validate_one_time_prekey_batch(&command.prekeys)?;

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
        command: ClaimOneTimePrekeyInput,
    ) -> Result<OneTimePrekey, KeyServiceError> {
        validate_device_id(command.device_id)?;

        self.repository
            .claim_next_available(command.device_id)
            .await
            .map_err(map_key_repository_error)?
            .ok_or(KeyServiceError::NoAvailableOneTimePrekeys)
    }

    async fn fetch_current_bundle(
        &self,
        command: FetchCurrentBundleInput,
    ) -> Result<KeyBundle, KeyServiceError> {
        validate_device_id(command.device_id)?;

        self.repository
            .get_current(command.device_id)
            .await
            .map_err(map_key_repository_error)?
            .ok_or(KeyServiceError::KeyBundleNotFound)
    }
}

fn validate_device_id(device_id: DeviceId) -> Result<(), KeyServiceError> {
    if device_id.0.is_nil() {
        return Err(KeyServiceError::InvalidDeviceId);
    }

    Ok(())
}

fn validate_signed_prekey_id(signed_prekey_id: Uuid) -> Result<(), KeyServiceError> {
    if signed_prekey_id.is_nil() {
        return Err(KeyServiceError::InvalidSignedPrekeyId);
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
        KeyRepositoryError::Constraint(KeyConstraintViolation::KeyBundleAlreadyExists) => {
            KeyServiceError::KeyBundleAlreadyExists
        }
        other => KeyServiceError::Storage(other),
    }
}
