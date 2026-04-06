mod cockroach;

use crate::domain::{KeyBundle, OneTimePrekey, SignedPrekey};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reach_auth_types::DeviceId;
use thiserror::Error;
use uuid::Uuid;

pub use cockroach::CockroachKeyRepository;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyConstraintViolation {
    KeyBundleAlreadyExists,
    SignedPrekeyAlreadyExists,
    OneTimePrekeyAlreadyExists,
}

#[derive(Debug, Error)]
pub enum KeyRepositoryError {
    #[error("key storage constraint violation: {0:?}")]
    Constraint(KeyConstraintViolation),
    #[error("invalid stored one-time prekey state: {0}")]
    InvalidOneTimePrekeyState(String),
    #[error("database operation failed: {0}")]
    Database(#[source] sqlx::Error),
}

#[derive(Debug, Clone)]
pub struct PublishSignedPrekeyRecord {
    pub signed_prekey_id: Uuid,
    pub device_id: DeviceId,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PublishCurrentKeyBundleRecord {
    pub bundle_id: Uuid,
    pub device_id: DeviceId,
    pub identity_key_public: Vec<u8>,
    pub identity_key_alg: String,
    pub signed_prekey_id: Uuid,
    pub published_at: DateTime<Utc>,
}

#[async_trait]
pub trait KeyBundleRepository: Send + Sync {
    async fn get_current(
        &self,
        device_id: DeviceId,
    ) -> Result<Option<KeyBundle>, KeyRepositoryError>;
    async fn insert(&self, key_bundle: &KeyBundle) -> Result<(), KeyRepositoryError>;
    async fn supersede_current(&self, device_id: DeviceId) -> Result<u64, KeyRepositoryError>;
}

#[async_trait]
pub trait SignedPrekeyRepository: Send + Sync {
    async fn get_current(
        &self,
        device_id: DeviceId,
    ) -> Result<Option<SignedPrekey>, KeyRepositoryError>;
    async fn get_by_id(
        &self,
        signed_prekey_id: Uuid,
    ) -> Result<Option<SignedPrekey>, KeyRepositoryError>;
    async fn insert(&self, signed_prekey: &SignedPrekey) -> Result<(), KeyRepositoryError>;
}

#[async_trait]
pub trait OneTimePrekeyRepository: Send + Sync {
    async fn insert_batch(&self, prekeys: &[OneTimePrekey]) -> Result<(), KeyRepositoryError>;
    async fn claim_next_available(
        &self,
        device_id: DeviceId,
    ) -> Result<Option<OneTimePrekey>, KeyRepositoryError>;
}

#[async_trait]
pub trait KeyBundleCommandRepository: Send + Sync {
    async fn publish_current_bundle(
        &self,
        command: &PublishCurrentKeyBundleRecord,
    ) -> Result<KeyBundle, KeyRepositoryError>;
}

#[async_trait]
pub trait SignedPrekeyCommandRepository: Send + Sync {
    async fn publish_current_signed_prekey(
        &self,
        command: &PublishSignedPrekeyRecord,
    ) -> Result<SignedPrekey, KeyRepositoryError>;
}
