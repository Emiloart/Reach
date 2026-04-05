use chrono::{Duration, Timelike, Utc};
use reach_auth_service::{
    application::{
        AuthCommandService, AuthUseCases, CreateSessionInput, RevokeSessionInput,
        RotateRefreshFamilyInput,
    },
    domain::SessionState,
    errors::AuthError,
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
async fn session_revocation_marks_session_and_blocks_rotation(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("auth_application_revoke_session", &MIGRATOR)
        .await?;
    let repository = CockroachAuthRepository::new(pool);
    let service = AuthCommandService::new(repository.clone());
    let session_id = SessionId(Uuid::now_v7());
    let current_hash = vec![1, 2, 3, 4];

    service
        .create_session(CreateSessionInput {
            session_id,
            account_id: AccountId(Uuid::now_v7()),
            device_id: DeviceId(Uuid::now_v7()),
            access_token_jti: Uuid::now_v7(),
            access_expires_at: db_timestamp() + Duration::hours(1),
            refresh_family_id: Uuid::now_v7(),
            refresh_token_hash: current_hash.clone(),
            refresh_expires_at: db_timestamp() + Duration::days(7),
        })
        .await?;

    let revoked = service
        .revoke_session(RevokeSessionInput { session_id })
        .await?;
    let stored = repository
        .get_session(session_id)
        .await?
        .expect("session should exist");
    let rotation = service
        .rotate_refresh_family(RotateRefreshFamilyInput {
            session_id,
            presented_refresh_token_hash: current_hash,
            next_refresh_token_hash: vec![9, 9, 9, 9],
            next_refresh_expires_at: db_timestamp() + Duration::days(14),
        })
        .await;

    assert_eq!(revoked.state, SessionState::Revoked);
    assert!(revoked.revoked_at.is_some());
    assert_eq!(stored.state, SessionState::Revoked);
    assert!(matches!(rotation, Err(AuthError::SessionRevoked)));

    Ok(())
}

#[tokio::test]
async fn refresh_family_rotation_updates_hashes_and_counter(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("auth_application_rotate_refresh_family", &MIGRATOR)
        .await?;
    let repository = CockroachAuthRepository::new(pool);
    let service = AuthCommandService::new(repository.clone());
    let session_id = SessionId(Uuid::now_v7());
    let current_hash = vec![1, 2, 3, 4];
    let next_hash = vec![5, 6, 7, 8];

    service
        .create_session(CreateSessionInput {
            session_id,
            account_id: AccountId(Uuid::now_v7()),
            device_id: DeviceId(Uuid::now_v7()),
            access_token_jti: Uuid::now_v7(),
            access_expires_at: db_timestamp() + Duration::hours(1),
            refresh_family_id: Uuid::now_v7(),
            refresh_token_hash: current_hash.clone(),
            refresh_expires_at: db_timestamp() + Duration::days(7),
        })
        .await?;

    let rotated = service
        .rotate_refresh_family(RotateRefreshFamilyInput {
            session_id,
            presented_refresh_token_hash: current_hash.clone(),
            next_refresh_token_hash: next_hash.clone(),
            next_refresh_expires_at: db_timestamp() + Duration::days(14),
        })
        .await?;
    let stored_session = repository
        .get_session(session_id)
        .await?
        .expect("session should exist");

    assert_eq!(rotated.current_token_hash, next_hash);
    assert_eq!(rotated.previous_token_hash, Some(current_hash));
    assert_eq!(rotated.rotation_counter, 1);
    assert!(stored_session.last_refreshed_at.is_some());

    Ok(())
}
