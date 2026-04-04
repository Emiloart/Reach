mod cockroach;

use crate::domain::{Account, Device};
use async_trait::async_trait;
use reach_auth_types::{AccountId, DeviceId};
use thiserror::Error;

pub use cockroach::CockroachIdentityRepository;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityConstraintViolation {
    AccountAlreadyExists,
    DeviceAlreadyExists,
    DeviceNumberAlreadyAllocated,
}

#[derive(Debug, Error)]
pub enum IdentityRepositoryError {
    #[error("identity storage constraint violation: {0:?}")]
    Constraint(IdentityConstraintViolation),
    #[error("invalid stored account state: {0}")]
    InvalidAccountState(String),
    #[error("invalid stored device status: {0}")]
    InvalidDeviceStatus(String),
    #[error("database operation failed: {0}")]
    Database(#[source] sqlx::Error),
}

#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn get_by_id(
        &self,
        account_id: AccountId,
    ) -> Result<Option<Account>, IdentityRepositoryError>;
    async fn create(&self, account: &Account) -> Result<(), IdentityRepositoryError>;
}

#[async_trait]
pub trait DeviceRepository: Send + Sync {
    async fn get_by_id(
        &self,
        device_id: DeviceId,
    ) -> Result<Option<Device>, IdentityRepositoryError>;
    async fn create(&self, device: &Device) -> Result<(), IdentityRepositoryError>;
    async fn revoke(
        &self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<bool, IdentityRepositoryError>;
}
