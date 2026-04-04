use crate::domain::{KeyBundle, OneTimePrekey, SignedPrekey};
use async_trait::async_trait;
use reach_auth_types::DeviceId;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UploadKeyBundle {
    pub device_id: DeviceId,
    pub identity_key_public: Vec<u8>,
    pub identity_key_alg: String,
    pub signed_prekey_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct UploadSignedPrekey {
    pub device_id: DeviceId,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct UploadOneTimePrekeys {
    pub device_id: DeviceId,
    pub prekeys: Vec<Vec<u8>>,
}

#[async_trait]
pub trait KeyUseCases: Send + Sync {
    async fn upload_key_bundle(
        &self,
        command: UploadKeyBundle,
    ) -> Result<KeyBundle, crate::errors::KeyServiceError>;
    async fn upload_signed_prekey(
        &self,
        command: UploadSignedPrekey,
    ) -> Result<SignedPrekey, crate::errors::KeyServiceError>;
    async fn claim_one_time_prekey(
        &self,
        device_id: DeviceId,
    ) -> Result<OneTimePrekey, crate::errors::KeyServiceError>;
}
