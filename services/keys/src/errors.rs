use crate::repository::KeyRepositoryError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeyServiceError {
    #[error("invalid device id")]
    InvalidDeviceId,
    #[error("invalid signed prekey id")]
    InvalidSignedPrekeyId,
    #[error("invalid identity key material")]
    InvalidIdentityKeyMaterial,
    #[error("invalid identity key algorithm")]
    InvalidIdentityKeyAlgorithm,
    #[error("invalid one-time prekey batch")]
    InvalidOneTimePrekeyBatch,
    #[error("invalid one-time prekey material")]
    InvalidOneTimePrekeyMaterial,
    #[error("device key bundle not found")]
    KeyBundleNotFound,
    #[error("signed prekey not found")]
    SignedPrekeyNotFound,
    #[error("signed prekey belongs to a different device")]
    SignedPrekeyDeviceMismatch,
    #[error("key bundle already exists")]
    KeyBundleAlreadyExists,
    #[error("no one-time prekeys available")]
    NoAvailableOneTimePrekeys,
    #[error("key storage failure: {0}")]
    Storage(#[source] KeyRepositoryError),
}
