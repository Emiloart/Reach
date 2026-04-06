use crate::repository::AuthRepositoryError;
use reach_identity_lifecycle::IdentityLifecycleError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("insufficient auth scope")]
    InsufficientScope,
    #[error("invalid session id")]
    InvalidSessionId,
    #[error("invalid account id")]
    InvalidAccountId,
    #[error("invalid device id")]
    InvalidDeviceId,
    #[error("invalid access token id")]
    InvalidAccessTokenId,
    #[error("invalid refresh family id")]
    InvalidRefreshFamilyId,
    #[error("invalid access token expiry")]
    InvalidAccessExpiry,
    #[error("invalid refresh token hash")]
    InvalidRefreshTokenHash,
    #[error("invalid refresh token expiry")]
    InvalidRefreshExpiry,
    #[error("account not found")]
    AccountNotFound,
    #[error("account is not active")]
    AccountNotActive,
    #[error("device not found")]
    DeviceNotFound,
    #[error("device is not active")]
    DeviceNotActive,
    #[error("device does not belong to account")]
    DeviceAccountMismatch,
    #[error("session not found")]
    SessionNotFound,
    #[error("session is revoked")]
    SessionRevoked,
    #[error("session is expired")]
    SessionExpired,
    #[error("session already exists")]
    SessionAlreadyExists,
    #[error("refresh token family not found")]
    RefreshTokenFamilyNotFound,
    #[error("refresh token family already exists")]
    RefreshTokenFamilyAlreadyExists,
    #[error("refresh token family is compromised")]
    RefreshTokenCompromised,
    #[error("presented refresh token does not match current family")]
    RefreshTokenMismatch,
    #[error("identity lifecycle read failure: {0}")]
    Lifecycle(#[source] IdentityLifecycleError),
    #[error("auth storage failure: {0}")]
    Storage(#[source] AuthRepositoryError),
}
