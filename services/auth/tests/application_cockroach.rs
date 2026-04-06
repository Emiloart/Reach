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
use reach_auth_types::{
    AccountId, AuthScope, DeviceId, InternalServicePrincipal, Principal, RequestContext, SessionId,
};
use reach_identity_lifecycle::CockroachIdentityLifecycleReader;
use reach_identity_service::application::{
    CreateAccountInput, IdentityCommandService, IdentityUseCases, RegisterDeviceInput,
    RevokeDeviceInput,
};
use reach_identity_service::repository::CockroachIdentityRepository;
use reach_test_support::CockroachTestContext;
use sqlx::migrate::Migrator;
use uuid::Uuid;

static AUTH_MIGRATOR: Migrator = sqlx::migrate!("./migrations");

fn db_timestamp() -> chrono::DateTime<chrono::Utc> {
    let timestamp = Utc::now();
    timestamp
        .with_nanosecond((timestamp.nanosecond() / 1_000) * 1_000)
        .expect("timestamp nanoseconds should remain valid after microsecond truncation")
}

fn internal_context(scopes: &[AuthScope]) -> RequestContext {
    RequestContext {
        principal: Principal::InternalService(InternalServicePrincipal {
            service_name: "reach-tests".to_owned(),
            scopes: scopes.to_vec(),
        }),
        request_id: Some("req-test".to_owned()),
    }
}

async fn apply_identity_schema(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("CREATE SCHEMA IF NOT EXISTS identity")
        .execute(pool)
        .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS identity.accounts (
            account_id UUID PRIMARY KEY,
            state STRING NOT NULL,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL,
            deletion_requested_at TIMESTAMPTZ NULL,
            purge_after TIMESTAMPTZ NULL
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS identity.devices (
            device_id UUID PRIMARY KEY,
            account_id UUID NOT NULL,
            device_number INT4 NOT NULL,
            platform STRING NOT NULL,
            app_version STRING NOT NULL,
            status STRING NOT NULL,
            registered_at TIMESTAMPTZ NOT NULL,
            revoked_at TIMESTAMPTZ NULL,
            CONSTRAINT devices_account_device_number_unique UNIQUE (account_id, device_number)
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[tokio::test]
async fn session_revocation_marks_session_and_blocks_rotation(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("auth_application_revoke_session", &AUTH_MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;
    let repository = CockroachAuthRepository::new(pool);
    let lifecycle_reader = CockroachIdentityLifecycleReader::new(repository.pool().clone());
    let service = AuthCommandService::new(repository.clone(), lifecycle_reader);
    let identity =
        IdentityCommandService::new(CockroachIdentityRepository::new(repository.pool().clone()));
    let session_id = SessionId(Uuid::now_v7());
    let account_id = AccountId(Uuid::now_v7());
    let device_id = DeviceId(Uuid::now_v7());
    let current_hash = vec![1, 2, 3, 4];
    let create_session_context = internal_context(&[AuthScope::AuthSessionCreate]);
    let revoke_session_context = internal_context(&[AuthScope::AuthSessionRevoke]);
    let rotate_context = internal_context(&[AuthScope::AuthSessionRotate]);

    identity
        .create_account(
            internal_context(&[AuthScope::IdentityAccountCreate]),
            CreateAccountInput { account_id },
        )
        .await?;
    identity
        .register_device(
            internal_context(&[AuthScope::IdentityDeviceRegister]),
            RegisterDeviceInput {
                account_id,
                device_id,
                device_number: 1,
                platform: "ios".to_owned(),
                app_version: "1.0.0".to_owned(),
            },
        )
        .await?;

    service
        .create_session(
            create_session_context,
            CreateSessionInput {
                session_id,
                account_id,
                device_id,
                access_token_jti: Uuid::now_v7(),
                access_expires_at: db_timestamp() + Duration::hours(1),
                refresh_family_id: Uuid::now_v7(),
                refresh_token_hash: current_hash.clone(),
                refresh_expires_at: db_timestamp() + Duration::days(7),
            },
        )
        .await?;

    let revoked = service
        .revoke_session(revoke_session_context, RevokeSessionInput { session_id })
        .await?;
    let stored = repository
        .get_session(session_id)
        .await?
        .expect("session should exist");
    let rotation = service
        .rotate_refresh_family(
            rotate_context,
            RotateRefreshFamilyInput {
                session_id,
                presented_refresh_token_hash: current_hash,
                next_refresh_token_hash: vec![9, 9, 9, 9],
                next_refresh_expires_at: db_timestamp() + Duration::days(14),
            },
        )
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
        .provision_database("auth_application_rotate_refresh_family", &AUTH_MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;
    let repository = CockroachAuthRepository::new(pool);
    let lifecycle_reader = CockroachIdentityLifecycleReader::new(repository.pool().clone());
    let service = AuthCommandService::new(repository.clone(), lifecycle_reader);
    let identity =
        IdentityCommandService::new(CockroachIdentityRepository::new(repository.pool().clone()));
    let session_id = SessionId(Uuid::now_v7());
    let account_id = AccountId(Uuid::now_v7());
    let device_id = DeviceId(Uuid::now_v7());
    let current_hash = vec![1, 2, 3, 4];
    let next_hash = vec![5, 6, 7, 8];

    identity
        .create_account(
            internal_context(&[AuthScope::IdentityAccountCreate]),
            CreateAccountInput { account_id },
        )
        .await?;
    identity
        .register_device(
            internal_context(&[AuthScope::IdentityDeviceRegister]),
            RegisterDeviceInput {
                account_id,
                device_id,
                device_number: 1,
                platform: "ios".to_owned(),
                app_version: "1.0.0".to_owned(),
            },
        )
        .await?;

    service
        .create_session(
            internal_context(&[AuthScope::AuthSessionCreate]),
            CreateSessionInput {
                session_id,
                account_id,
                device_id,
                access_token_jti: Uuid::now_v7(),
                access_expires_at: db_timestamp() + Duration::hours(1),
                refresh_family_id: Uuid::now_v7(),
                refresh_token_hash: current_hash.clone(),
                refresh_expires_at: db_timestamp() + Duration::days(7),
            },
        )
        .await?;

    let rotated = service
        .rotate_refresh_family(
            internal_context(&[AuthScope::AuthSessionRotate]),
            RotateRefreshFamilyInput {
                session_id,
                presented_refresh_token_hash: current_hash.clone(),
                next_refresh_token_hash: next_hash.clone(),
                next_refresh_expires_at: db_timestamp() + Duration::days(14),
            },
        )
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

#[tokio::test]
async fn create_session_rejects_revoked_device() -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("auth_application_revoked_device", &AUTH_MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;

    let repository = CockroachAuthRepository::new(pool);
    let lifecycle_reader = CockroachIdentityLifecycleReader::new(repository.pool().clone());
    let service = AuthCommandService::new(repository.clone(), lifecycle_reader);
    let identity =
        IdentityCommandService::new(CockroachIdentityRepository::new(repository.pool().clone()));
    let account_id = AccountId(Uuid::now_v7());
    let device_id = DeviceId(Uuid::now_v7());

    identity
        .create_account(
            internal_context(&[AuthScope::IdentityAccountCreate]),
            CreateAccountInput { account_id },
        )
        .await?;
    identity
        .register_device(
            internal_context(&[AuthScope::IdentityDeviceRegister]),
            RegisterDeviceInput {
                account_id,
                device_id,
                device_number: 1,
                platform: "ios".to_owned(),
                app_version: "1.0.0".to_owned(),
            },
        )
        .await?;
    identity
        .revoke_device(
            internal_context(&[AuthScope::IdentityDeviceRevoke]),
            RevokeDeviceInput {
                account_id,
                device_id,
            },
        )
        .await?;

    let result = service
        .create_session(
            internal_context(&[AuthScope::AuthSessionCreate]),
            CreateSessionInput {
                session_id: SessionId(Uuid::now_v7()),
                account_id,
                device_id,
                access_token_jti: Uuid::now_v7(),
                access_expires_at: db_timestamp() + Duration::hours(1),
                refresh_family_id: Uuid::now_v7(),
                refresh_token_hash: vec![1, 2, 3, 4],
                refresh_expires_at: db_timestamp() + Duration::days(7),
            },
        )
        .await;

    assert!(matches!(result, Err(AuthError::DeviceNotActive)));

    Ok(())
}
