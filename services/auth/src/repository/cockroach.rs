use crate::{
    domain::{RefreshTokenFamily, Session, SessionState},
    repository::{
        AuthConstraintViolation, AuthRepositoryError, RefreshTokenRepository, SessionRepository,
    },
};
use async_trait::async_trait;
use reach_auth_types::{AccountId, DeviceId, SessionId};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone)]
pub struct CockroachAuthRepository {
    pool: PgPool,
}

impl CockroachAuthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionRepository for CockroachAuthRepository {
    async fn get_session(
        &self,
        session_id: SessionId,
    ) -> Result<Option<Session>, AuthRepositoryError> {
        let row = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT
                session_id,
                account_id,
                device_id,
                state,
                issued_at,
                expires_at,
                revoked_at,
                last_refreshed_at,
                access_token_jti
            FROM auth.sessions
            WHERE session_id = $1
            "#,
        )
        .bind(session_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(AuthRepositoryError::Database)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn create_session(&self, session: &Session) -> Result<(), AuthRepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO auth.sessions (
                session_id,
                account_id,
                device_id,
                state,
                issued_at,
                expires_at,
                revoked_at,
                last_refreshed_at,
                access_token_jti
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(session.session_id.0)
        .bind(session.account_id.0)
        .bind(session.device_id.0)
        .bind(session.state.as_str())
        .bind(session.issued_at)
        .bind(session.expires_at)
        .bind(session.revoked_at)
        .bind(session.last_refreshed_at)
        .bind(session.access_token_jti)
        .execute(&self.pool)
        .await
        .map_err(map_session_insert_error)?;

        Ok(())
    }

    async fn revoke_session(&self, session_id: SessionId) -> Result<bool, AuthRepositoryError> {
        let result = sqlx::query(
            r#"
            UPDATE auth.sessions
            SET
                state = 'revoked',
                revoked_at = now()
            WHERE session_id = $1
              AND state != 'revoked'
            "#,
        )
        .bind(session_id.0)
        .execute(&self.pool)
        .await
        .map_err(AuthRepositoryError::Database)?;

        Ok(result.rows_affected() == 1)
    }
}

#[async_trait]
impl RefreshTokenRepository for CockroachAuthRepository {
    async fn get_family_by_session(
        &self,
        session_id: SessionId,
    ) -> Result<Option<RefreshTokenFamily>, AuthRepositoryError> {
        let row = sqlx::query_as::<_, RefreshTokenFamilyRow>(
            r#"
            SELECT
                family_id,
                session_id,
                current_token_hash,
                previous_token_hash,
                rotation_counter,
                compromised_at,
                expires_at
            FROM auth.refresh_token_families
            WHERE session_id = $1
            "#,
        )
        .bind(session_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(AuthRepositoryError::Database)?;

        Ok(row.map(Into::into))
    }

    async fn create_family(&self, family: &RefreshTokenFamily) -> Result<(), AuthRepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO auth.refresh_token_families (
                family_id,
                session_id,
                current_token_hash,
                previous_token_hash,
                rotation_counter,
                compromised_at,
                expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(family.family_id)
        .bind(family.session_id.0)
        .bind(&family.current_token_hash)
        .bind(&family.previous_token_hash)
        .bind(family.rotation_counter)
        .bind(family.compromised_at)
        .bind(family.expires_at)
        .execute(&self.pool)
        .await
        .map_err(map_refresh_family_insert_error)?;

        Ok(())
    }

    async fn mark_compromised(&self, session_id: SessionId) -> Result<bool, AuthRepositoryError> {
        let result = sqlx::query(
            r#"
            UPDATE auth.refresh_token_families
            SET compromised_at = now()
            WHERE session_id = $1
              AND compromised_at IS NULL
            "#,
        )
        .bind(session_id.0)
        .execute(&self.pool)
        .await
        .map_err(AuthRepositoryError::Database)?;

        Ok(result.rows_affected() == 1)
    }
}

#[derive(Debug, FromRow)]
struct SessionRow {
    session_id: uuid::Uuid,
    account_id: uuid::Uuid,
    device_id: uuid::Uuid,
    state: String,
    issued_at: chrono::DateTime<chrono::Utc>,
    expires_at: chrono::DateTime<chrono::Utc>,
    revoked_at: Option<chrono::DateTime<chrono::Utc>>,
    last_refreshed_at: Option<chrono::DateTime<chrono::Utc>>,
    access_token_jti: uuid::Uuid,
}

impl TryFrom<SessionRow> for Session {
    type Error = AuthRepositoryError;

    fn try_from(value: SessionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            session_id: SessionId(value.session_id),
            account_id: AccountId(value.account_id),
            device_id: DeviceId(value.device_id),
            state: SessionState::try_from(value.state.as_str())
                .map_err(AuthRepositoryError::InvalidSessionState)?,
            issued_at: value.issued_at,
            expires_at: value.expires_at,
            revoked_at: value.revoked_at,
            last_refreshed_at: value.last_refreshed_at,
            access_token_jti: value.access_token_jti,
        })
    }
}

#[derive(Debug, FromRow)]
struct RefreshTokenFamilyRow {
    family_id: uuid::Uuid,
    session_id: uuid::Uuid,
    current_token_hash: Vec<u8>,
    previous_token_hash: Option<Vec<u8>>,
    rotation_counter: i64,
    compromised_at: Option<chrono::DateTime<chrono::Utc>>,
    expires_at: chrono::DateTime<chrono::Utc>,
}

impl From<RefreshTokenFamilyRow> for RefreshTokenFamily {
    fn from(value: RefreshTokenFamilyRow) -> Self {
        Self {
            family_id: value.family_id,
            session_id: SessionId(value.session_id),
            current_token_hash: value.current_token_hash,
            previous_token_hash: value.previous_token_hash,
            rotation_counter: value.rotation_counter,
            compromised_at: value.compromised_at,
            expires_at: value.expires_at,
        }
    }
}

fn map_session_insert_error(error: sqlx::Error) -> AuthRepositoryError {
    if is_unique_violation(&error) {
        return AuthRepositoryError::Constraint(AuthConstraintViolation::SessionAlreadyExists);
    }

    AuthRepositoryError::Database(error)
}

fn map_refresh_family_insert_error(error: sqlx::Error) -> AuthRepositoryError {
    if is_unique_violation(&error) {
        return AuthRepositoryError::Constraint(
            AuthConstraintViolation::RefreshTokenFamilyAlreadyExists,
        );
    }

    AuthRepositoryError::Database(error)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("23505")
    )
}
