use crate::domain::{KeyBundle, OneTimePrekey, SignedPrekey};
use async_trait::async_trait;
use reach_auth_types::DeviceId;
use uuid::Uuid;

#[async_trait]
pub trait KeyBundleRepository: Send + Sync {
    async fn get_current(&self, device_id: DeviceId) -> Result<Option<KeyBundle>, crate::errors::KeyServiceError>;
    async fn insert(&self, key_bundle: &KeyBundle) -> Result<(), crate::errors::KeyServiceError>;
    async fn supersede_current(&self, device_id: DeviceId) -> Result<(), crate::errors::KeyServiceError>;
}

#[async_trait]
pub trait SignedPrekeyRepository: Send + Sync {
    async fn get_by_id(
        &self,
        signed_prekey_id: Uuid,
    ) -> Result<Option<SignedPrekey>, crate::errors::KeyServiceError>;
    async fn insert(&self, signed_prekey: &SignedPrekey) -> Result<(), crate::errors::KeyServiceError>;
}

#[async_trait]
pub trait OneTimePrekeyRepository: Send + Sync {
    async fn insert_batch(
        &self,
        prekeys: &[OneTimePrekey],
    ) -> Result<(), crate::errors::KeyServiceError>;
    async fn claim_next_available(
        &self,
        device_id: DeviceId,
    ) -> Result<Option<OneTimePrekey>, crate::errors::KeyServiceError>;
}

