use crate::{
    domain::{Account, AccountState, Device, DeviceStatus},
    errors::IdentityError,
    repository::{
        AccountRepository, DeviceRepository, IdentityConstraintViolation, IdentityRepositoryError,
    },
};
use async_trait::async_trait;
use chrono::{Timelike, Utc};
use reach_auth_types::{AccountId, AuthScope, DeviceId, RequestContext};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountInput {
    pub account_id: AccountId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterDeviceInput {
    pub account_id: AccountId,
    pub device_id: DeviceId,
    pub device_number: i32,
    pub platform: String,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeDeviceInput {
    pub account_id: AccountId,
    pub device_id: DeviceId,
}

#[async_trait]
pub trait IdentityUseCases: Send + Sync {
    async fn create_account(
        &self,
        context: RequestContext,
        command: CreateAccountInput,
    ) -> Result<Account, IdentityError>;
    async fn register_device(
        &self,
        context: RequestContext,
        command: RegisterDeviceInput,
    ) -> Result<Device, IdentityError>;
    async fn revoke_device(
        &self,
        context: RequestContext,
        command: RevokeDeviceInput,
    ) -> Result<Device, IdentityError>;
}

#[derive(Debug, Clone)]
pub struct IdentityCommandService<R> {
    repository: Arc<R>,
}

impl<R> IdentityCommandService<R> {
    pub fn new(repository: R) -> Self {
        Self {
            repository: Arc::new(repository),
        }
    }
}

#[async_trait]
impl<R> IdentityUseCases for IdentityCommandService<R>
where
    R: AccountRepository + DeviceRepository + Send + Sync + 'static,
{
    async fn create_account(
        &self,
        context: RequestContext,
        command: CreateAccountInput,
    ) -> Result<Account, IdentityError> {
        authorize(&context, AuthScope::IdentityAccountCreate)?;
        validate_account_id(command.account_id)?;

        let timestamp = db_timestamp();
        let account = Account {
            account_id: command.account_id,
            state: AccountState::Active,
            created_at: timestamp,
            updated_at: timestamp,
            deletion_requested_at: None,
            purge_after: None,
        };

        AccountRepository::create(self.repository.as_ref(), &account)
            .await
            .map_err(map_identity_repository_error)?;

        Ok(account)
    }

    async fn register_device(
        &self,
        context: RequestContext,
        command: RegisterDeviceInput,
    ) -> Result<Device, IdentityError> {
        authorize(&context, AuthScope::IdentityDeviceRegister)?;
        validate_account_id(command.account_id)?;
        validate_device_id(command.device_id)?;
        validate_device_number(command.device_number)?;
        validate_platform(&command.platform)?;
        validate_app_version(&command.app_version)?;

        let account = AccountRepository::get_by_id(self.repository.as_ref(), command.account_id)
            .await
            .map_err(map_identity_repository_error)?
            .ok_or(IdentityError::AccountNotFound)?;

        if account.state != AccountState::Active {
            return Err(IdentityError::AccountNotActive);
        }

        let device = Device {
            device_id: command.device_id,
            account_id: command.account_id,
            device_number: command.device_number,
            platform: command.platform.trim().to_owned(),
            app_version: command.app_version.trim().to_owned(),
            status: DeviceStatus::Active,
            registered_at: db_timestamp(),
            revoked_at: None,
        };

        DeviceRepository::create(self.repository.as_ref(), &device)
            .await
            .map_err(map_identity_repository_error)?;

        Ok(device)
    }

    async fn revoke_device(
        &self,
        context: RequestContext,
        command: RevokeDeviceInput,
    ) -> Result<Device, IdentityError> {
        authorize(&context, AuthScope::IdentityDeviceRevoke)?;
        validate_account_id(command.account_id)?;
        validate_device_id(command.device_id)?;

        let device = DeviceRepository::get_by_id(self.repository.as_ref(), command.device_id)
            .await
            .map_err(map_identity_repository_error)?
            .ok_or(IdentityError::DeviceNotFound)?;

        if device.account_id != command.account_id {
            return Err(IdentityError::DeviceNotFound);
        }

        if device.status == DeviceStatus::Revoked {
            return Err(IdentityError::DeviceAlreadyRevoked);
        }

        let updated = self
            .repository
            .revoke(command.account_id, command.device_id)
            .await
            .map_err(map_identity_repository_error)?;

        if !updated {
            return Err(IdentityError::DeviceAlreadyRevoked);
        }

        DeviceRepository::get_by_id(self.repository.as_ref(), command.device_id)
            .await
            .map_err(map_identity_repository_error)?
            .ok_or(IdentityError::DeviceNotFound)
    }
}

fn validate_account_id(account_id: AccountId) -> Result<(), IdentityError> {
    if account_id.0.is_nil() {
        return Err(IdentityError::InvalidAccountId);
    }

    Ok(())
}

fn authorize(context: &RequestContext, scope: AuthScope) -> Result<(), IdentityError> {
    if !context.has_scope(scope) {
        return Err(IdentityError::InsufficientScope);
    }

    Ok(())
}

fn validate_device_id(device_id: DeviceId) -> Result<(), IdentityError> {
    if device_id.0.is_nil() {
        return Err(IdentityError::InvalidDeviceId);
    }

    Ok(())
}

fn validate_device_number(device_number: i32) -> Result<(), IdentityError> {
    if device_number <= 0 {
        return Err(IdentityError::InvalidDeviceNumber);
    }

    Ok(())
}

fn validate_platform(platform: &str) -> Result<(), IdentityError> {
    let trimmed = platform.trim();

    if trimmed.is_empty() || trimmed.len() > 32 {
        return Err(IdentityError::InvalidPlatform);
    }

    Ok(())
}

fn validate_app_version(app_version: &str) -> Result<(), IdentityError> {
    let trimmed = app_version.trim();

    if trimmed.is_empty() || trimmed.len() > 64 {
        return Err(IdentityError::InvalidAppVersion);
    }

    Ok(())
}

fn db_timestamp() -> chrono::DateTime<chrono::Utc> {
    let timestamp = Utc::now();
    timestamp
        .with_nanosecond((timestamp.nanosecond() / 1_000) * 1_000)
        .expect("timestamp nanoseconds should remain valid after microsecond truncation")
}

fn map_identity_repository_error(error: IdentityRepositoryError) -> IdentityError {
    match error {
        IdentityRepositoryError::Constraint(IdentityConstraintViolation::AccountAlreadyExists) => {
            IdentityError::AccountAlreadyExists
        }
        IdentityRepositoryError::Constraint(IdentityConstraintViolation::DeviceAlreadyExists) => {
            IdentityError::DeviceAlreadyExists
        }
        IdentityRepositoryError::Constraint(
            IdentityConstraintViolation::DeviceNumberAlreadyAllocated,
        ) => IdentityError::DeviceRegistrationConflict,
        other => IdentityError::Storage(other),
    }
}
