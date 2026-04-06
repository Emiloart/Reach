use chrono::{DateTime, Utc};
use reach_auth_types::DeviceId;
use sqlx::{FromRow, Postgres};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentKeyBundle {
    pub bundle_id: Uuid,
    pub device_id: DeviceId,
    pub signed_prekey_id: Uuid,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedOneTimePrekey {
    pub prekey_id: Uuid,
    pub device_id: DeviceId,
    pub claimed_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum KeyMaterialError {
    #[error("database operation failed: {0}")]
    Database(#[source] sqlx::Error),
}

pub async fn fetch_current_key_bundle<'e, E>(
    executor: E,
    device_id: DeviceId,
) -> Result<Option<CurrentKeyBundle>, KeyMaterialError>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let row = sqlx::query_as::<_, CurrentKeyBundleRow>(
        r#"
        SELECT
            bundle_id,
            device_id,
            signed_prekey_id,
            published_at
        FROM keys.key_bundles
        WHERE device_id = $1
          AND is_current = true
        "#,
    )
    .bind(device_id.0)
    .fetch_optional(executor)
    .await
    .map_err(KeyMaterialError::Database)?;

    Ok(row.map(Into::into))
}

pub async fn claim_next_available_one_time_prekey<'e, E>(
    executor: E,
    device_id: DeviceId,
    claimed_at: DateTime<Utc>,
) -> Result<Option<ClaimedOneTimePrekey>, KeyMaterialError>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let row = sqlx::query_as::<_, ClaimedOneTimePrekeyRow>(
        r#"
        UPDATE keys.one_time_prekeys
        SET
            state = 'claimed',
            claimed_at = $2
        WHERE prekey_id = (
            SELECT prekey_id
            FROM keys.one_time_prekeys
            WHERE device_id = $1
              AND state = 'available'
            ORDER BY created_at ASC
            LIMIT 1
        )
        RETURNING
            prekey_id,
            device_id,
            claimed_at
        "#,
    )
    .bind(device_id.0)
    .bind(claimed_at)
    .fetch_optional(executor)
    .await
    .map_err(KeyMaterialError::Database)?;

    Ok(row.map(Into::into))
}

#[derive(Debug, FromRow)]
struct CurrentKeyBundleRow {
    bundle_id: Uuid,
    device_id: Uuid,
    signed_prekey_id: Uuid,
    published_at: DateTime<Utc>,
}

impl From<CurrentKeyBundleRow> for CurrentKeyBundle {
    fn from(value: CurrentKeyBundleRow) -> Self {
        Self {
            bundle_id: value.bundle_id,
            device_id: DeviceId(value.device_id),
            signed_prekey_id: value.signed_prekey_id,
            published_at: value.published_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct ClaimedOneTimePrekeyRow {
    prekey_id: Uuid,
    device_id: Uuid,
    claimed_at: DateTime<Utc>,
}

impl From<ClaimedOneTimePrekeyRow> for ClaimedOneTimePrekey {
    fn from(value: ClaimedOneTimePrekeyRow) -> Self {
        Self {
            prekey_id: value.prekey_id,
            device_id: DeviceId(value.device_id),
            claimed_at: value.claimed_at,
        }
    }
}
