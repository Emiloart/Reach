use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("session not found")]
    SessionNotFound,
    #[error("session is revoked")]
    SessionRevoked,
    #[error("refresh token family is compromised")]
    RefreshTokenCompromised,
    #[error("insufficient auth scope")]
    InsufficientScope,
}

