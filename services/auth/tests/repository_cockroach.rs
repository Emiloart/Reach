use chrono::{Duration, Timelike, Utc};
use reach_auth_service::{
    domain::{Session, SessionState},
    repository::{CockroachAuthRepository, SessionRepository},
};
use reach_auth_types::{AccountId, DeviceId, SessionId};
use reach_test_support::CockroachTestContext;
use sqlx::migrate::Migrator;
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

fn db_timestamp() -> chrono::DateTime<chrono::Utc> {
    let timestamp = Utc::now();
    timestamp
        .with_nanosecond((timestamp.nanosecond() / 1_000) * 1_000)
        .expect("timestamp nanoseconds should remain valid after microsecond truncation")
}

#[tokio::test]
async fn session_creation_and_lookup_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("auth_session_create", &MIGRATOR)
        .await?;
    let repository = CockroachAuthRepository::new(pool);

    let issued_at = db_timestamp();
    let session = Session {
        session_id: SessionId(Uuid::now_v7()),
        account_id: AccountId(Uuid::now_v7()),
        device_id: DeviceId(Uuid::now_v7()),
        state: SessionState::Active,
        issued_at,
        expires_at: issued_at + Duration::hours(1),
        revoked_at: None,
        last_refreshed_at: None,
        access_token_jti: Uuid::now_v7(),
    };

    repository.create_session(&session).await?;

    let fetched = repository.get_session(session.session_id).await?;

    assert_eq!(fetched, Some(session));

    Ok(())
}
