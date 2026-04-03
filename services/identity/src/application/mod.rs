use crate::domain::{Account, Device};
use async_trait::async_trait;
use reach_auth_types::{AccountId, DeviceId};

#[derive(Debug, Clone)]
pub struct CreateAccount;

#[derive(Debug, Clone)]
pub struct RegisterLinkedDevice {
    pub account_id: AccountId,
    pub device_id: DeviceId,
    pub device_number: i32,
}

#[derive(Debug, Clone)]
pub struct RevokeDevice {
    pub account_id: AccountId,
    pub device_id: DeviceId,
}

#[async_trait]
pub trait IdentityUseCases: Send + Sync {
    async fn create_account(&self, command: CreateAccount) -> Result<Account, crate::errors::IdentityError>;
    async fn register_linked_device(
        &self,
        command: RegisterLinkedDevice,
    ) -> Result<Device, crate::errors::IdentityError>;
    async fn revoke_device(&self, command: RevokeDevice) -> Result<(), crate::errors::IdentityError>;
}

