use crate::{
    domain::{Account, AccountState, Device, DeviceStatus},
    repository::{
        AccountRepository, DeviceRepository, IdentityConstraintViolation, IdentityRepositoryError,
    },
};
use async_trait::async_trait;
use reach_auth_types::{AccountId, DeviceId};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone)]
pub struct CockroachIdentityRepository {
    pool: PgPool,
}

impl CockroachIdentityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AccountRepository for CockroachIdentityRepository {
    async fn get_by_id(
        &self,
        account_id: AccountId,
    ) -> Result<Option<Account>, IdentityRepositoryError> {
        let row = sqlx::query_as::<_, AccountRow>(
            r#"
            SELECT
                account_id,
                state,
                created_at,
                updated_at,
                deletion_requested_at,
                purge_after
            FROM identity.accounts
            WHERE account_id = $1
            "#,
        )
        .bind(account_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(IdentityRepositoryError::Database)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn create(&self, account: &Account) -> Result<(), IdentityRepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO identity.accounts (
                account_id,
                state,
                created_at,
                updated_at,
                deletion_requested_at,
                purge_after
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(account.account_id.0)
        .bind(account.state.as_str())
        .bind(account.created_at)
        .bind(account.updated_at)
        .bind(account.deletion_requested_at)
        .bind(account.purge_after)
        .execute(&self.pool)
        .await
        .map_err(map_account_insert_error)?;

        Ok(())
    }
}

#[async_trait]
impl DeviceRepository for CockroachIdentityRepository {
    async fn get_by_id(
        &self,
        device_id: DeviceId,
    ) -> Result<Option<Device>, IdentityRepositoryError> {
        let row = sqlx::query_as::<_, DeviceRow>(
            r#"
            SELECT
                device_id,
                account_id,
                device_number,
                platform,
                app_version,
                status,
                registered_at,
                revoked_at
            FROM identity.devices
            WHERE device_id = $1
            "#,
        )
        .bind(device_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(IdentityRepositoryError::Database)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn create(&self, device: &Device) -> Result<(), IdentityRepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO identity.devices (
                device_id,
                account_id,
                device_number,
                platform,
                app_version,
                status,
                registered_at,
                revoked_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(device.device_id.0)
        .bind(device.account_id.0)
        .bind(device.device_number)
        .bind(&device.platform)
        .bind(&device.app_version)
        .bind(device.status.as_str())
        .bind(device.registered_at)
        .bind(device.revoked_at)
        .execute(&self.pool)
        .await
        .map_err(map_device_insert_error)?;

        Ok(())
    }

    async fn revoke(
        &self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<bool, IdentityRepositoryError> {
        let result = sqlx::query(
            r#"
            UPDATE identity.devices
            SET
                status = 'revoked',
                revoked_at = now()
            WHERE account_id = $1
              AND device_id = $2
              AND status != 'revoked'
            "#,
        )
        .bind(account_id.0)
        .bind(device_id.0)
        .execute(&self.pool)
        .await
        .map_err(IdentityRepositoryError::Database)?;

        Ok(result.rows_affected() == 1)
    }
}

#[derive(Debug, FromRow)]
struct AccountRow {
    account_id: uuid::Uuid,
    state: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deletion_requested_at: Option<chrono::DateTime<chrono::Utc>>,
    purge_after: Option<chrono::DateTime<chrono::Utc>>,
}

impl TryFrom<AccountRow> for Account {
    type Error = IdentityRepositoryError;

    fn try_from(value: AccountRow) -> Result<Self, Self::Error> {
        Ok(Self {
            account_id: AccountId(value.account_id),
            state: AccountState::try_from(value.state.as_str())
                .map_err(IdentityRepositoryError::InvalidAccountState)?,
            created_at: value.created_at,
            updated_at: value.updated_at,
            deletion_requested_at: value.deletion_requested_at,
            purge_after: value.purge_after,
        })
    }
}

#[derive(Debug, FromRow)]
struct DeviceRow {
    device_id: uuid::Uuid,
    account_id: uuid::Uuid,
    device_number: i32,
    platform: String,
    app_version: String,
    status: String,
    registered_at: chrono::DateTime<chrono::Utc>,
    revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl TryFrom<DeviceRow> for Device {
    type Error = IdentityRepositoryError;

    fn try_from(value: DeviceRow) -> Result<Self, Self::Error> {
        Ok(Self {
            device_id: DeviceId(value.device_id),
            account_id: AccountId(value.account_id),
            device_number: value.device_number,
            platform: value.platform,
            app_version: value.app_version,
            status: DeviceStatus::try_from(value.status.as_str())
                .map_err(IdentityRepositoryError::InvalidDeviceStatus)?,
            registered_at: value.registered_at,
            revoked_at: value.revoked_at,
        })
    }
}

fn map_account_insert_error(error: sqlx::Error) -> IdentityRepositoryError {
    if let Some(constraint) = constraint_name(&error) {
        let violation = match constraint {
            "primary" | "accounts_pkey" => Some(IdentityConstraintViolation::AccountAlreadyExists),
            _ => None,
        };

        if let Some(violation) = violation {
            return IdentityRepositoryError::Constraint(violation);
        }
    }

    if is_unique_violation(&error) {
        return IdentityRepositoryError::Constraint(
            IdentityConstraintViolation::AccountAlreadyExists,
        );
    }

    IdentityRepositoryError::Database(error)
}

fn map_device_insert_error(error: sqlx::Error) -> IdentityRepositoryError {
    if let Some(constraint) = constraint_name(&error) {
        let violation = match constraint {
            "primary" | "devices_pkey" => Some(IdentityConstraintViolation::DeviceAlreadyExists),
            "devices_account_device_number_unique" => {
                Some(IdentityConstraintViolation::DeviceNumberAlreadyAllocated)
            }
            _ => None,
        };

        if let Some(violation) = violation {
            return IdentityRepositoryError::Constraint(violation);
        }
    }

    if is_unique_violation(&error) {
        return IdentityRepositoryError::Constraint(
            IdentityConstraintViolation::DeviceAlreadyExists,
        );
    }

    IdentityRepositoryError::Database(error)
}

fn constraint_name(error: &sqlx::Error) -> Option<&str> {
    match error {
        sqlx::Error::Database(database_error) => database_error.constraint().or_else(|| {
            database_error.message().split_whitespace().find(|segment| {
                segment.contains("primary")
                    || segment.contains("devices_account_device_number_unique")
            })
        }),
        _ => None,
    }
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("23505")
    )
}
