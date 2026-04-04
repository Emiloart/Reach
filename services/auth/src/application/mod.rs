use crate::domain::{RefreshTokenFamily, Session};
use async_trait::async_trait;
use reach_auth_types::{AccountId, DeviceId, SessionId};

#[derive(Debug, Clone)]
pub struct BootstrapSession {
    pub account_id: AccountId,
    pub device_id: DeviceId,
}

#[derive(Debug, Clone)]
pub struct RefreshSession {
    pub session_id: SessionId,
    pub presented_refresh_token_hash: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RevokeSession {
    pub session_id: SessionId,
}

#[async_trait]
pub trait AuthUseCases: Send + Sync {
    async fn bootstrap_session(
        &self,
        command: BootstrapSession,
    ) -> Result<Session, crate::errors::AuthError>;
    async fn refresh_session(
        &self,
        command: RefreshSession,
    ) -> Result<RefreshTokenFamily, crate::errors::AuthError>;
    async fn revoke_session(&self, command: RevokeSession) -> Result<(), crate::errors::AuthError>;
}
