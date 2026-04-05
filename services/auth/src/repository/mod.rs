mod cockroach;

use crate::domain::{RefreshTokenFamily, Session};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
pub use cockroach::CockroachAuthRepository;
use reach_auth_types::SessionId;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthConstraintViolation {
    SessionAlreadyExists,
    RefreshTokenFamilyAlreadyExists,
}

#[derive(Debug, Error)]
pub enum AuthRepositoryError {
    #[error("auth storage constraint violation: {0:?}")]
    Constraint(AuthConstraintViolation),
    #[error("invalid stored session state: {0}")]
    InvalidSessionState(String),
    #[error("database operation failed: {0}")]
    Database(#[source] sqlx::Error),
}

#[derive(Debug, Clone)]
pub struct RotateRefreshFamilyRecord {
    pub session_id: SessionId,
    pub presented_refresh_token_hash: Vec<u8>,
    pub next_refresh_token_hash: Vec<u8>,
    pub rotated_at: DateTime<Utc>,
    pub next_refresh_expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RotateRefreshFamilyOutcome {
    Rotated(RefreshTokenFamily),
    SessionNotFound,
    SessionRevoked,
    SessionExpired,
    RefreshFamilyNotFound,
    PresentedTokenMismatch,
    RefreshFamilyCompromised,
}

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn get_session(
        &self,
        session_id: SessionId,
    ) -> Result<Option<Session>, AuthRepositoryError>;
    async fn create_session(&self, session: &Session) -> Result<(), AuthRepositoryError>;
    async fn revoke_session(&self, session_id: SessionId) -> Result<bool, AuthRepositoryError>;
}

#[async_trait]
pub trait RefreshTokenRepository: Send + Sync {
    async fn get_family_by_session(
        &self,
        session_id: SessionId,
    ) -> Result<Option<RefreshTokenFamily>, AuthRepositoryError>;
    async fn create_family(&self, family: &RefreshTokenFamily) -> Result<(), AuthRepositoryError>;
    async fn mark_compromised(&self, session_id: SessionId) -> Result<bool, AuthRepositoryError>;
}

#[async_trait]
pub trait AuthCommandRepository: Send + Sync {
    async fn create_session_with_family(
        &self,
        session: &Session,
        family: &RefreshTokenFamily,
    ) -> Result<(), AuthRepositoryError>;
    async fn rotate_refresh_family(
        &self,
        command: &RotateRefreshFamilyRecord,
    ) -> Result<RotateRefreshFamilyOutcome, AuthRepositoryError>;
}
