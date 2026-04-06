use crate::repository::KeyRepositoryError;
use reach_identity_lifecycle::IdentityLifecycleError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeyServiceError {
    #[error("insufficient auth scope")]
    InsufficientScope,
    #[error("invalid device id")]
    InvalidDeviceId,
    #[error("invalid signed prekey id")]
    InvalidSignedPrekeyId,
    #[error("invalid signed prekey material")]
    InvalidSignedPrekeyMaterial,
    #[error("invalid signed prekey signature")]
    InvalidSignedPrekeySignature,
    #[error("invalid identity key material")]
    InvalidIdentityKeyMaterial,
    #[error("invalid identity key algorithm")]
    InvalidIdentityKeyAlgorithm,
    #[error("invalid one-time prekey batch")]
    InvalidOneTimePrekeyBatch,
    #[error("invalid one-time prekey material")]
    InvalidOneTimePrekeyMaterial,
    #[error("account not found")]
    AccountNotFound,
    #[error("account is not active")]
    AccountNotActive,
    #[error("device not found")]
    DeviceNotFound,
    #[error("device is not active")]
    DeviceNotActive,
    #[error("device key bundle not found")]
    KeyBundleNotFound,
    #[error("signed prekey not found")]
    SignedPrekeyNotFound,
    #[error("signed prekey already exists")]
    SignedPrekeyAlreadyExists,
    #[error("signed prekey belongs to a different device")]
    SignedPrekeyDeviceMismatch,
    #[error("key bundle already exists")]
    KeyBundleAlreadyExists,
    #[error("no one-time prekeys available")]
    NoAvailableOneTimePrekeys,
    #[error("identity lifecycle read failure: {0}")]
    Lifecycle(#[source] IdentityLifecycleError),
    #[error("key storage failure: {0}")]
    Storage(#[source] KeyRepositoryError),
}
