use crate::domain::{RefreshTokenFamily, Session};
use async_trait::async_trait;
use reach_auth_types::SessionId;

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn get_session(&self, session_id: SessionId) -> Result<Option<Session>, crate::errors::AuthError>;
    async fn upsert_session(&self, session: &Session) -> Result<(), crate::errors::AuthError>;
    async fn revoke_session(&self, session_id: SessionId) -> Result<(), crate::errors::AuthError>;
}

#[async_trait]
pub trait RefreshTokenRepository: Send + Sync {
    async fn get_family_by_session(
        &self,
        session_id: SessionId,
    ) -> Result<Option<RefreshTokenFamily>, crate::errors::AuthError>;
    async fn upsert_family(
        &self,
        family: &RefreshTokenFamily,
    ) -> Result<(), crate::errors::AuthError>;
    async fn mark_compromised(&self, session_id: SessionId) -> Result<(), crate::errors::AuthError>;
}

