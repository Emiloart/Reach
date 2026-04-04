mod cockroach;

use crate::domain::{RefreshTokenFamily, Session};
use async_trait::async_trait;
use reach_auth_types::SessionId;
use thiserror::Error;

pub use cockroach::CockroachAuthRepository;

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
