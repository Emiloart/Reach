use crate::domain::{Account, Device};
use async_trait::async_trait;
use reach_auth_types::{AccountId, DeviceId};

#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn get_by_id(&self, account_id: AccountId) -> Result<Option<Account>, crate::errors::IdentityError>;
    async fn create(&self, account: &Account) -> Result<(), crate::errors::IdentityError>;
}

#[async_trait]
pub trait DeviceRepository: Send + Sync {
    async fn get_by_id(&self, device_id: DeviceId) -> Result<Option<Device>, crate::errors::IdentityError>;
    async fn create(&self, device: &Device) -> Result<(), crate::errors::IdentityError>;
    async fn revoke(&self, account_id: AccountId, device_id: DeviceId) -> Result<(), crate::errors::IdentityError>;
}

