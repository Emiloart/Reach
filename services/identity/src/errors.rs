use crate::repository::IdentityRepositoryError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("insufficient auth scope")]
    InsufficientScope,
    #[error("invalid account id")]
    InvalidAccountId,
    #[error("invalid device id")]
    InvalidDeviceId,
    #[error("invalid device number")]
    InvalidDeviceNumber,
    #[error("invalid platform")]
    InvalidPlatform,
    #[error("invalid app version")]
    InvalidAppVersion,
    #[error("account not found")]
    AccountNotFound,
    #[error("account is not active")]
    AccountNotActive,
    #[error("account already exists")]
    AccountAlreadyExists,
    #[error("device not found")]
    DeviceNotFound,
    #[error("device already exists")]
    DeviceAlreadyExists,
    #[error("device registration conflict")]
    DeviceRegistrationConflict,
    #[error("device is already revoked")]
    DeviceAlreadyRevoked,
    #[error("identity storage failure: {0}")]
    Storage(#[source] IdentityRepositoryError),
}
