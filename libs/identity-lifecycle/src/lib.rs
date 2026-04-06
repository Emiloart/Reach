use async_trait::async_trait;
use reach_auth_types::{AccountId, DeviceId};
use sqlx::{FromRow, PgPool};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountLifecycleState {
    Active,
    PendingDeletion,
    Suspended,
    Purged,
}

impl TryFrom<&str> for AccountLifecycleState {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "active" => Ok(Self::Active),
            "pending_deletion" => Ok(Self::PendingDeletion),
            "suspended" => Ok(Self::Suspended),
            "purged" => Ok(Self::Purged),
            invalid => Err(invalid.to_owned()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceLifecycleStatus {
    Active,
    Revoked,
}

impl TryFrom<&str> for DeviceLifecycleStatus {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "active" => Ok(Self::Active),
            "revoked" => Ok(Self::Revoked),
            invalid => Err(invalid.to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountLifecycle {
    pub account_id: AccountId,
    pub state: AccountLifecycleState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceLifecycle {
    pub device_id: DeviceId,
    pub account_id: AccountId,
    pub status: DeviceLifecycleStatus,
}

#[derive(Debug, Error)]
pub enum IdentityLifecycleError {
    #[error("invalid stored account lifecycle state: {0}")]
    InvalidAccountState(String),
    #[error("invalid stored device lifecycle status: {0}")]
    InvalidDeviceStatus(String),
    #[error("database operation failed: {0}")]
    Database(#[source] sqlx::Error),
}

#[async_trait]
pub trait IdentityLifecycleReader: Send + Sync {
    async fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<AccountLifecycle>, IdentityLifecycleError>;

    async fn get_device(
        &self,
        device_id: DeviceId,
    ) -> Result<Option<DeviceLifecycle>, IdentityLifecycleError>;
}

#[derive(Debug, Clone)]
pub struct CockroachIdentityLifecycleReader {
    pool: PgPool,
}

impl CockroachIdentityLifecycleReader {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IdentityLifecycleReader for CockroachIdentityLifecycleReader {
    async fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<AccountLifecycle>, IdentityLifecycleError> {
        let row = sqlx::query_as::<_, AccountLifecycleRow>(
            r#"
            SELECT account_id, state
            FROM identity.accounts
            WHERE account_id = $1
            "#,
        )
        .bind(account_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(IdentityLifecycleError::Database)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn get_device(
        &self,
        device_id: DeviceId,
    ) -> Result<Option<DeviceLifecycle>, IdentityLifecycleError> {
        let row = sqlx::query_as::<_, DeviceLifecycleRow>(
            r#"
            SELECT device_id, account_id, status
            FROM identity.devices
            WHERE device_id = $1
            "#,
        )
        .bind(device_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(IdentityLifecycleError::Database)?;

        row.map(TryInto::try_into).transpose()
    }
}

#[derive(Debug, FromRow)]
struct AccountLifecycleRow {
    account_id: uuid::Uuid,
    state: String,
}

impl TryFrom<AccountLifecycleRow> for AccountLifecycle {
    type Error = IdentityLifecycleError;

    fn try_from(value: AccountLifecycleRow) -> Result<Self, Self::Error> {
        Ok(Self {
            account_id: AccountId(value.account_id),
            state: AccountLifecycleState::try_from(value.state.as_str())
                .map_err(IdentityLifecycleError::InvalidAccountState)?,
        })
    }
}

#[derive(Debug, FromRow)]
struct DeviceLifecycleRow {
    device_id: uuid::Uuid,
    account_id: uuid::Uuid,
    status: String,
}

impl TryFrom<DeviceLifecycleRow> for DeviceLifecycle {
    type Error = IdentityLifecycleError;

    fn try_from(value: DeviceLifecycleRow) -> Result<Self, Self::Error> {
        Ok(Self {
            device_id: DeviceId(value.device_id),
            account_id: AccountId(value.account_id),
            status: DeviceLifecycleStatus::try_from(value.status.as_str())
                .map_err(IdentityLifecycleError::InvalidDeviceStatus)?,
        })
    }
}
