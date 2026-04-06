use crate::{
    domain::{KeyBundle, OneTimePrekey, OneTimePrekeyState, SignedPrekey},
    repository::{
        KeyBundleCommandRepository, KeyBundleRepository, KeyConstraintViolation,
        KeyRepositoryError, OneTimePrekeyRepository, PublishCurrentKeyBundleRecord,
        PublishSignedPrekeyRecord, SignedPrekeyCommandRepository, SignedPrekeyRepository,
    },
};
use async_trait::async_trait;
use reach_auth_types::DeviceId;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CockroachKeyRepository {
    pool: PgPool,
}

impl CockroachKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl KeyBundleRepository for CockroachKeyRepository {
    async fn get_current(
        &self,
        device_id: DeviceId,
    ) -> Result<Option<KeyBundle>, KeyRepositoryError> {
        let row = sqlx::query_as::<_, KeyBundleRow>(
            r#"
            SELECT
                bundle_id,
                device_id,
                bundle_version,
                identity_key_public,
                identity_key_alg,
                signed_prekey_id,
                published_at,
                superseded_at,
                is_current
            FROM keys.key_bundles
            WHERE device_id = $1
              AND is_current = true
            "#,
        )
        .bind(device_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(KeyRepositoryError::Database)?;

        Ok(row.map(Into::into))
    }

    async fn insert(&self, key_bundle: &KeyBundle) -> Result<(), KeyRepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO keys.key_bundles (
                bundle_id,
                device_id,
                bundle_version,
                identity_key_public,
                identity_key_alg,
                signed_prekey_id,
                published_at,
                superseded_at,
                is_current
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(key_bundle.bundle_id)
        .bind(key_bundle.device_id.0)
        .bind(key_bundle.bundle_version)
        .bind(&key_bundle.identity_key_public)
        .bind(&key_bundle.identity_key_alg)
        .bind(key_bundle.signed_prekey_id)
        .bind(key_bundle.published_at)
        .bind(key_bundle.superseded_at)
        .bind(key_bundle.is_current)
        .execute(&self.pool)
        .await
        .map_err(map_key_bundle_insert_error)?;

        Ok(())
    }

    async fn supersede_current(&self, device_id: DeviceId) -> Result<u64, KeyRepositoryError> {
        let result = sqlx::query(
            r#"
            UPDATE keys.key_bundles
            SET
                is_current = false,
                superseded_at = now()
            WHERE device_id = $1
              AND is_current = true
            "#,
        )
        .bind(device_id.0)
        .execute(&self.pool)
        .await
        .map_err(KeyRepositoryError::Database)?;

        Ok(result.rows_affected())
    }
}

#[async_trait]
impl SignedPrekeyRepository for CockroachKeyRepository {
    async fn get_current(
        &self,
        device_id: DeviceId,
    ) -> Result<Option<SignedPrekey>, KeyRepositoryError> {
        let row = sqlx::query_as::<_, SignedPrekeyRow>(
            r#"
            SELECT
                signed_prekey_id,
                device_id,
                public_key,
                signature,
                created_at,
                superseded_at
            FROM keys.signed_prekeys
            WHERE device_id = $1
              AND superseded_at IS NULL
            "#,
        )
        .bind(device_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(KeyRepositoryError::Database)?;

        Ok(row.map(Into::into))
    }

    async fn get_by_id(
        &self,
        signed_prekey_id: Uuid,
    ) -> Result<Option<SignedPrekey>, KeyRepositoryError> {
        let row = sqlx::query_as::<_, SignedPrekeyRow>(
            r#"
            SELECT
                signed_prekey_id,
                device_id,
                public_key,
                signature,
                created_at,
                superseded_at
            FROM keys.signed_prekeys
            WHERE signed_prekey_id = $1
            "#,
        )
        .bind(signed_prekey_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(KeyRepositoryError::Database)?;

        Ok(row.map(Into::into))
    }

    async fn insert(&self, signed_prekey: &SignedPrekey) -> Result<(), KeyRepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO keys.signed_prekeys (
                signed_prekey_id,
                device_id,
                public_key,
                signature,
                created_at,
                superseded_at
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(signed_prekey.signed_prekey_id)
        .bind(signed_prekey.device_id.0)
        .bind(&signed_prekey.public_key)
        .bind(&signed_prekey.signature)
        .bind(signed_prekey.created_at)
        .bind(signed_prekey.superseded_at)
        .execute(&self.pool)
        .await
        .map_err(map_signed_prekey_insert_error)?;

        Ok(())
    }
}

#[async_trait]
impl SignedPrekeyCommandRepository for CockroachKeyRepository {
    async fn publish_current_signed_prekey(
        &self,
        command: &PublishSignedPrekeyRecord,
    ) -> Result<SignedPrekey, KeyRepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(KeyRepositoryError::Database)?;

        sqlx::query(
            r#"
            UPDATE keys.signed_prekeys
            SET superseded_at = $2
            WHERE device_id = $1
              AND superseded_at IS NULL
            "#,
        )
        .bind(command.device_id.0)
        .bind(command.created_at)
        .execute(&mut *transaction)
        .await
        .map_err(KeyRepositoryError::Database)?;

        let signed_prekey = sqlx::query_as::<_, SignedPrekeyRow>(
            r#"
            INSERT INTO keys.signed_prekeys (
                signed_prekey_id,
                device_id,
                public_key,
                signature,
                created_at,
                superseded_at
            )
            VALUES ($1, $2, $3, $4, $5, NULL)
            RETURNING
                signed_prekey_id,
                device_id,
                public_key,
                signature,
                created_at,
                superseded_at
            "#,
        )
        .bind(command.signed_prekey_id)
        .bind(command.device_id.0)
        .bind(&command.public_key)
        .bind(&command.signature)
        .bind(command.created_at)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_signed_prekey_insert_error)?;

        transaction
            .commit()
            .await
            .map_err(KeyRepositoryError::Database)?;

        Ok(signed_prekey.into())
    }
}

#[async_trait]
impl OneTimePrekeyRepository for CockroachKeyRepository {
    async fn insert_batch(&self, prekeys: &[OneTimePrekey]) -> Result<(), KeyRepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(KeyRepositoryError::Database)?;

        for prekey in prekeys {
            sqlx::query(
                r#"
                INSERT INTO keys.one_time_prekeys (
                    prekey_id,
                    device_id,
                    public_key,
                    state,
                    created_at,
                    claimed_at
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(prekey.prekey_id)
            .bind(prekey.device_id.0)
            .bind(&prekey.public_key)
            .bind(prekey.state.as_str())
            .bind(prekey.created_at)
            .bind(prekey.claimed_at)
            .execute(&mut *transaction)
            .await
            .map_err(map_one_time_prekey_insert_error)?;
        }

        transaction
            .commit()
            .await
            .map_err(KeyRepositoryError::Database)?;

        Ok(())
    }

    async fn claim_next_available(
        &self,
        device_id: DeviceId,
    ) -> Result<Option<OneTimePrekey>, KeyRepositoryError> {
        let row = sqlx::query_as::<_, OneTimePrekeyRow>(
            r#"
            UPDATE keys.one_time_prekeys
            SET
                state = 'claimed',
                claimed_at = now()
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
                public_key,
                state,
                created_at,
                claimed_at
            "#,
        )
        .bind(device_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(KeyRepositoryError::Database)?;

        row.map(TryInto::try_into).transpose()
    }
}

#[async_trait]
impl KeyBundleCommandRepository for CockroachKeyRepository {
    async fn publish_current_bundle(
        &self,
        command: &PublishCurrentKeyBundleRecord,
    ) -> Result<KeyBundle, KeyRepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(KeyRepositoryError::Database)?;

        let current_bundle_version = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT bundle_version
            FROM keys.key_bundles
            WHERE device_id = $1
            ORDER BY bundle_version DESC
            LIMIT 1
            FOR UPDATE
            "#,
        )
        .bind(command.device_id.0)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(KeyRepositoryError::Database)?;

        sqlx::query(
            r#"
            UPDATE keys.key_bundles
            SET
                is_current = false,
                superseded_at = $2
            WHERE device_id = $1
              AND is_current = true
            "#,
        )
        .bind(command.device_id.0)
        .bind(command.published_at)
        .execute(&mut *transaction)
        .await
        .map_err(KeyRepositoryError::Database)?;

        let next_bundle_version = current_bundle_version.unwrap_or(0) + 1;
        let key_bundle = sqlx::query_as::<_, KeyBundleRow>(
            r#"
            INSERT INTO keys.key_bundles (
                bundle_id,
                device_id,
                bundle_version,
                identity_key_public,
                identity_key_alg,
                signed_prekey_id,
                published_at,
                superseded_at,
                is_current
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NULL, true)
            RETURNING
                bundle_id,
                device_id,
                bundle_version,
                identity_key_public,
                identity_key_alg,
                signed_prekey_id,
                published_at,
                superseded_at,
                is_current
            "#,
        )
        .bind(command.bundle_id)
        .bind(command.device_id.0)
        .bind(next_bundle_version)
        .bind(&command.identity_key_public)
        .bind(&command.identity_key_alg)
        .bind(command.signed_prekey_id)
        .bind(command.published_at)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_key_bundle_insert_error)?;

        transaction
            .commit()
            .await
            .map_err(KeyRepositoryError::Database)?;

        Ok(key_bundle.into())
    }
}

#[derive(Debug, FromRow)]
struct KeyBundleRow {
    bundle_id: Uuid,
    device_id: Uuid,
    bundle_version: i64,
    identity_key_public: Vec<u8>,
    identity_key_alg: String,
    signed_prekey_id: Uuid,
    published_at: chrono::DateTime<chrono::Utc>,
    superseded_at: Option<chrono::DateTime<chrono::Utc>>,
    is_current: bool,
}

impl From<KeyBundleRow> for KeyBundle {
    fn from(value: KeyBundleRow) -> Self {
        Self {
            bundle_id: value.bundle_id,
            device_id: DeviceId(value.device_id),
            bundle_version: value.bundle_version,
            identity_key_public: value.identity_key_public,
            identity_key_alg: value.identity_key_alg,
            signed_prekey_id: value.signed_prekey_id,
            published_at: value.published_at,
            superseded_at: value.superseded_at,
            is_current: value.is_current,
        }
    }
}

#[derive(Debug, FromRow)]
struct SignedPrekeyRow {
    signed_prekey_id: Uuid,
    device_id: Uuid,
    public_key: Vec<u8>,
    signature: Vec<u8>,
    created_at: chrono::DateTime<chrono::Utc>,
    superseded_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<SignedPrekeyRow> for SignedPrekey {
    fn from(value: SignedPrekeyRow) -> Self {
        Self {
            signed_prekey_id: value.signed_prekey_id,
            device_id: DeviceId(value.device_id),
            public_key: value.public_key,
            signature: value.signature,
            created_at: value.created_at,
            superseded_at: value.superseded_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct OneTimePrekeyRow {
    prekey_id: Uuid,
    device_id: Uuid,
    public_key: Vec<u8>,
    state: String,
    created_at: chrono::DateTime<chrono::Utc>,
    claimed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl TryFrom<OneTimePrekeyRow> for OneTimePrekey {
    type Error = KeyRepositoryError;

    fn try_from(value: OneTimePrekeyRow) -> Result<Self, Self::Error> {
        Ok(Self {
            prekey_id: value.prekey_id,
            device_id: DeviceId(value.device_id),
            public_key: value.public_key,
            state: OneTimePrekeyState::try_from(value.state.as_str())
                .map_err(KeyRepositoryError::InvalidOneTimePrekeyState)?,
            created_at: value.created_at,
            claimed_at: value.claimed_at,
        })
    }
}

fn map_key_bundle_insert_error(error: sqlx::Error) -> KeyRepositoryError {
    if is_unique_violation(&error) {
        return KeyRepositoryError::Constraint(KeyConstraintViolation::KeyBundleAlreadyExists);
    }

    KeyRepositoryError::Database(error)
}

fn map_signed_prekey_insert_error(error: sqlx::Error) -> KeyRepositoryError {
    if is_unique_violation(&error) {
        return KeyRepositoryError::Constraint(KeyConstraintViolation::SignedPrekeyAlreadyExists);
    }

    KeyRepositoryError::Database(error)
}

fn map_one_time_prekey_insert_error(error: sqlx::Error) -> KeyRepositoryError {
    if is_unique_violation(&error) {
        return KeyRepositoryError::Constraint(KeyConstraintViolation::OneTimePrekeyAlreadyExists);
    }

    KeyRepositoryError::Database(error)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("23505")
    )
}
