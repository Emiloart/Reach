use crate::{
    domain::{RefreshTokenFamily, Session, SessionState},
    errors::AuthError,
    repository::{
        AuthCommandRepository, AuthConstraintViolation, AuthRepositoryError,
        RefreshTokenRepository, RotateRefreshFamilyOutcome, RotateRefreshFamilyRecord,
        SessionRepository,
    },
};
use async_trait::async_trait;
use chrono::{Timelike, Utc};
use reach_auth_types::{AccountId, AuthScope, DeviceId, RequestContext, SessionId};
use reach_identity_lifecycle::{
    AccountLifecycleState, DeviceLifecycleStatus, IdentityLifecycleReader,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionInput {
    pub session_id: SessionId,
    pub account_id: AccountId,
    pub device_id: DeviceId,
    pub access_token_jti: Uuid,
    pub access_expires_at: chrono::DateTime<chrono::Utc>,
    pub refresh_family_id: Uuid,
    pub refresh_token_hash: Vec<u8>,
    pub refresh_expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotateRefreshFamilyInput {
    pub session_id: SessionId,
    pub presented_refresh_token_hash: Vec<u8>,
    pub next_refresh_token_hash: Vec<u8>,
    pub next_refresh_expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeSessionInput {
    pub session_id: SessionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatedSession {
    pub session: Session,
    pub refresh_family: RefreshTokenFamily,
}

#[async_trait]
pub trait AuthUseCases: Send + Sync {
    async fn create_session(
        &self,
        context: RequestContext,
        command: CreateSessionInput,
    ) -> Result<CreatedSession, AuthError>;
    async fn rotate_refresh_family(
        &self,
        context: RequestContext,
        command: RotateRefreshFamilyInput,
    ) -> Result<RefreshTokenFamily, AuthError>;
    async fn revoke_session(
        &self,
        context: RequestContext,
        command: RevokeSessionInput,
    ) -> Result<Session, AuthError>;
}

#[derive(Debug, Clone)]
pub struct AuthCommandService<R, L> {
    repository: Arc<R>,
    lifecycle_reader: Arc<L>,
}

impl<R, L> AuthCommandService<R, L> {
    pub fn new(repository: R, lifecycle_reader: L) -> Self {
        Self {
            repository: Arc::new(repository),
            lifecycle_reader: Arc::new(lifecycle_reader),
        }
    }
}

#[async_trait]
impl<R, L> AuthUseCases for AuthCommandService<R, L>
where
    R: SessionRepository + RefreshTokenRepository + AuthCommandRepository + Send + Sync + 'static,
    L: IdentityLifecycleReader + Send + Sync + 'static,
{
    async fn create_session(
        &self,
        context: RequestContext,
        command: CreateSessionInput,
    ) -> Result<CreatedSession, AuthError> {
        authorize(&context, AuthScope::AuthSessionCreate)?;
        validate_session_id(command.session_id)?;
        validate_account_id(command.account_id)?;
        validate_device_id(command.device_id)?;
        validate_access_token_id(command.access_token_jti)?;
        validate_refresh_family_id(command.refresh_family_id)?;
        validate_hash(&command.refresh_token_hash)?;

        let issued_at = db_timestamp();
        if command.access_expires_at <= issued_at {
            return Err(AuthError::InvalidAccessExpiry);
        }
        if command.refresh_expires_at <= issued_at {
            return Err(AuthError::InvalidRefreshExpiry);
        }

        self.ensure_active_identity_device(command.account_id, command.device_id)
            .await?;

        let session = Session {
            session_id: command.session_id,
            account_id: command.account_id,
            device_id: command.device_id,
            state: SessionState::Active,
            issued_at,
            expires_at: command.access_expires_at,
            revoked_at: None,
            last_refreshed_at: None,
            access_token_jti: command.access_token_jti,
        };
        let refresh_family = RefreshTokenFamily {
            family_id: command.refresh_family_id,
            session_id: command.session_id,
            current_token_hash: command.refresh_token_hash,
            previous_token_hash: None,
            rotation_counter: 0,
            compromised_at: None,
            expires_at: command.refresh_expires_at,
        };

        self.repository
            .create_session_with_family(&session, &refresh_family)
            .await
            .map_err(map_auth_repository_error)?;

        Ok(CreatedSession {
            session,
            refresh_family,
        })
    }

    async fn rotate_refresh_family(
        &self,
        context: RequestContext,
        command: RotateRefreshFamilyInput,
    ) -> Result<RefreshTokenFamily, AuthError> {
        authorize(&context, AuthScope::AuthSessionRotate)?;
        validate_session_id(command.session_id)?;
        validate_hash(&command.presented_refresh_token_hash)?;
        validate_hash(&command.next_refresh_token_hash)?;

        if command.presented_refresh_token_hash == command.next_refresh_token_hash {
            return Err(AuthError::InvalidRefreshTokenHash);
        }

        let rotated_at = db_timestamp();
        if command.next_refresh_expires_at <= rotated_at {
            return Err(AuthError::InvalidRefreshExpiry);
        }

        let session = self
            .repository
            .get_session(command.session_id)
            .await
            .map_err(map_auth_repository_error)?
            .ok_or(AuthError::SessionNotFound)?;

        self.ensure_active_identity_device(session.account_id, session.device_id)
            .await?;

        let outcome = self
            .repository
            .rotate_refresh_family(&RotateRefreshFamilyRecord {
                session_id: command.session_id,
                presented_refresh_token_hash: command.presented_refresh_token_hash,
                next_refresh_token_hash: command.next_refresh_token_hash,
                rotated_at,
                next_refresh_expires_at: command.next_refresh_expires_at,
            })
            .await
            .map_err(map_auth_repository_error)?;

        match outcome {
            RotateRefreshFamilyOutcome::Rotated(family) => Ok(family),
            RotateRefreshFamilyOutcome::SessionNotFound => Err(AuthError::SessionNotFound),
            RotateRefreshFamilyOutcome::SessionRevoked => Err(AuthError::SessionRevoked),
            RotateRefreshFamilyOutcome::SessionExpired => Err(AuthError::SessionExpired),
            RotateRefreshFamilyOutcome::RefreshFamilyNotFound => {
                Err(AuthError::RefreshTokenFamilyNotFound)
            }
            RotateRefreshFamilyOutcome::PresentedTokenMismatch => {
                Err(AuthError::RefreshTokenMismatch)
            }
            RotateRefreshFamilyOutcome::RefreshFamilyCompromised => {
                Err(AuthError::RefreshTokenCompromised)
            }
        }
    }

    async fn revoke_session(
        &self,
        context: RequestContext,
        command: RevokeSessionInput,
    ) -> Result<Session, AuthError> {
        authorize(&context, AuthScope::AuthSessionRevoke)?;
        validate_session_id(command.session_id)?;

        let session = self
            .repository
            .get_session(command.session_id)
            .await
            .map_err(map_auth_repository_error)?
            .ok_or(AuthError::SessionNotFound)?;

        match session.state {
            SessionState::Revoked => return Err(AuthError::SessionRevoked),
            SessionState::Expired => return Err(AuthError::SessionExpired),
            SessionState::Active => {}
        }

        let updated = self
            .repository
            .revoke_session(command.session_id)
            .await
            .map_err(map_auth_repository_error)?;

        if !updated {
            return Err(AuthError::SessionRevoked);
        }

        self.repository
            .get_session(command.session_id)
            .await
            .map_err(map_auth_repository_error)?
            .ok_or(AuthError::SessionNotFound)
    }
}

impl<R, L> AuthCommandService<R, L>
where
    L: IdentityLifecycleReader + Send + Sync + 'static,
{
    async fn ensure_active_identity_device(
        &self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<(), AuthError> {
        let account = self
            .lifecycle_reader
            .get_account(account_id)
            .await
            .map_err(AuthError::Lifecycle)?
            .ok_or(AuthError::AccountNotFound)?;

        if account.state != AccountLifecycleState::Active {
            return Err(AuthError::AccountNotActive);
        }

        let device = self
            .lifecycle_reader
            .get_device(device_id)
            .await
            .map_err(AuthError::Lifecycle)?
            .ok_or(AuthError::DeviceNotFound)?;

        if device.account_id != account_id {
            return Err(AuthError::DeviceAccountMismatch);
        }

        if device.status != DeviceLifecycleStatus::Active {
            return Err(AuthError::DeviceNotActive);
        }

        Ok(())
    }
}

fn validate_session_id(session_id: SessionId) -> Result<(), AuthError> {
    if session_id.0.is_nil() {
        return Err(AuthError::InvalidSessionId);
    }

    Ok(())
}

fn authorize(context: &RequestContext, scope: AuthScope) -> Result<(), AuthError> {
    if !context.has_scope(scope) {
        return Err(AuthError::InsufficientScope);
    }

    Ok(())
}

fn validate_account_id(account_id: AccountId) -> Result<(), AuthError> {
    if account_id.0.is_nil() {
        return Err(AuthError::InvalidAccountId);
    }

    Ok(())
}

fn validate_device_id(device_id: DeviceId) -> Result<(), AuthError> {
    if device_id.0.is_nil() {
        return Err(AuthError::InvalidDeviceId);
    }

    Ok(())
}

fn validate_access_token_id(access_token_jti: Uuid) -> Result<(), AuthError> {
    if access_token_jti.is_nil() {
        return Err(AuthError::InvalidAccessTokenId);
    }

    Ok(())
}

fn validate_refresh_family_id(refresh_family_id: Uuid) -> Result<(), AuthError> {
    if refresh_family_id.is_nil() {
        return Err(AuthError::InvalidRefreshFamilyId);
    }

    Ok(())
}

fn validate_hash(hash: &[u8]) -> Result<(), AuthError> {
    if hash.is_empty() || hash.len() > 128 {
        return Err(AuthError::InvalidRefreshTokenHash);
    }

    Ok(())
}

fn db_timestamp() -> chrono::DateTime<chrono::Utc> {
    let timestamp = Utc::now();
    timestamp
        .with_nanosecond((timestamp.nanosecond() / 1_000) * 1_000)
        .expect("timestamp nanoseconds should remain valid after microsecond truncation")
}

fn map_auth_repository_error(error: AuthRepositoryError) -> AuthError {
    match error {
        AuthRepositoryError::Constraint(AuthConstraintViolation::SessionAlreadyExists) => {
            AuthError::SessionAlreadyExists
        }
        AuthRepositoryError::Constraint(
            AuthConstraintViolation::RefreshTokenFamilyAlreadyExists,
        ) => AuthError::RefreshTokenFamilyAlreadyExists,
        other => AuthError::Storage(other),
    }
}
