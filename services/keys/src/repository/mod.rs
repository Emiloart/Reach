mod cockroach;

use crate::domain::{KeyBundle, OneTimePrekey, SignedPrekey};
use async_trait::async_trait;
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
