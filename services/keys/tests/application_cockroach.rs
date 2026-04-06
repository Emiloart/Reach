use reach_auth_types::{
    AccountId, AuthScope, DeviceId, InternalServicePrincipal, Principal, RequestContext,
};
use reach_identity_lifecycle::CockroachIdentityLifecycleReader;
use reach_identity_service::{
    application::{
        CreateAccountInput, IdentityCommandService, IdentityUseCases, RegisterDeviceInput,
        RevokeDeviceInput,
    },
    repository::CockroachIdentityRepository,
};
use reach_key_service::{
    application::{
        ClaimOneTimePrekeyInput, FetchCurrentBundleInput, KeyCommandService, KeyUseCases,
        PublishKeyBundleInput, PublishOneTimePrekeysInput, PublishSignedPrekeyInput,
    },
    errors::KeyServiceError,
    repository::{CockroachKeyRepository, SignedPrekeyRepository},
};
use reach_test_support::CockroachTestContext;
use sqlx::migrate::Migrator;
use std::{collections::HashSet, sync::Arc};
use tokio::{sync::Barrier, task::JoinHandle};
use uuid::Uuid;

static KEYS_MIGRATOR: Migrator = sqlx::migrate!("./migrations");

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

async fn provision_identity_device(
    repository: &CockroachKeyRepository,
    account_id: AccountId,
    device_id: DeviceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let identity =
        IdentityCommandService::new(CockroachIdentityRepository::new(repository.pool().clone()));

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

    Ok(())
}

async fn revoke_identity_device(
    repository: &CockroachKeyRepository,
    account_id: AccountId,
    device_id: DeviceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let identity =
        IdentityCommandService::new(CockroachIdentityRepository::new(repository.pool().clone()));

    identity
        .revoke_device(
            internal_context(&[AuthScope::IdentityDeviceRevoke]),
            RevokeDeviceInput {
                account_id,
                device_id,
            },
        )
        .await?;

    Ok(())
}

#[tokio::test]
async fn concurrent_claims_do_not_duplicate_one_time_prekeys(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("keys_application_concurrent_claim", &KEYS_MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;

    let repository = CockroachKeyRepository::new(pool);
    let account_id = AccountId(Uuid::now_v7());
    let device_id = DeviceId(Uuid::now_v7());
    provision_identity_device(&repository, account_id, device_id).await?;

    let lifecycle_reader = CockroachIdentityLifecycleReader::new(repository.pool().clone());
    let service = Arc::new(KeyCommandService::new(repository, lifecycle_reader));
    let barrier = Arc::new(Barrier::new(4));

    service
        .publish_one_time_prekeys(
            internal_context(&[AuthScope::KeysOneTimePrekeysPublish]),
            PublishOneTimePrekeysInput {
                device_id,
                prekeys: vec![vec![1, 1, 1], vec![2, 2, 2]],
            },
        )
        .await?;

    let mut handles: Vec<JoinHandle<Result<Uuid, KeyServiceError>>> = Vec::new();
    for _ in 0..4 {
        let service = Arc::clone(&service);
        let barrier = Arc::clone(&barrier);
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            service
                .claim_one_time_prekey(
                    internal_context(&[AuthScope::KeysOneTimePrekeyClaim]),
                    ClaimOneTimePrekeyInput { device_id },
                )
                .await
                .map(|prekey| prekey.prekey_id)
        }));
    }

    let mut claimed_ids = HashSet::new();
    let mut success_count = 0usize;
    let mut exhausted_count = 0usize;
    for handle in handles {
        let result = handle
            .await
            .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
        match result {
            Ok(prekey_id) => {
                claimed_ids.insert(prekey_id);
                success_count += 1;
            }
            Err(KeyServiceError::NoAvailableOneTimePrekeys) => exhausted_count += 1,
            Err(other) => return Err(Box::new(other) as Box<dyn std::error::Error>),
        }
    }

    assert_eq!(success_count, 2);
    assert_eq!(claimed_ids.len(), 2);
    assert_eq!(exhausted_count, 2);

    Ok(())
}

#[tokio::test]
async fn publishing_new_bundle_supersedes_existing_current_bundle(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("keys_application_publish_bundle", &KEYS_MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;

    let repository = CockroachKeyRepository::new(pool.clone());
    let account_id = AccountId(Uuid::now_v7());
    let device_id = DeviceId(Uuid::now_v7());
    provision_identity_device(&repository, account_id, device_id).await?;

    let lifecycle_reader = CockroachIdentityLifecycleReader::new(repository.pool().clone());
    let service = KeyCommandService::new(repository.clone(), lifecycle_reader);
    let first_signed_prekey = service
        .publish_signed_prekey(
            internal_context(&[AuthScope::KeysSignedPrekeyPublish]),
            PublishSignedPrekeyInput {
                device_id,
                public_key: vec![1, 2, 3, 4],
                signature: vec![5, 6, 7, 8],
            },
        )
        .await?;
    let second_signed_prekey = service
        .publish_signed_prekey(
            internal_context(&[AuthScope::KeysSignedPrekeyPublish]),
            PublishSignedPrekeyInput {
                device_id,
                public_key: vec![9, 9, 9, 9],
                signature: vec![6, 6, 6, 6],
            },
        )
        .await?;

    let first_bundle = service
        .publish_key_bundle(
            internal_context(&[AuthScope::KeysBundlePublish]),
            PublishKeyBundleInput {
                device_id,
                identity_key_public: vec![9, 9, 9],
                identity_key_alg: "x25519".to_owned(),
                signed_prekey_id: first_signed_prekey.signed_prekey_id,
            },
        )
        .await?;
    let second_bundle = service
        .publish_key_bundle(
            internal_context(&[AuthScope::KeysBundlePublish]),
            PublishKeyBundleInput {
                device_id,
                identity_key_public: vec![8, 8, 8],
                identity_key_alg: "x25519".to_owned(),
                signed_prekey_id: second_signed_prekey.signed_prekey_id,
            },
        )
        .await?;
    let current_bundle = service
        .fetch_current_bundle(
            internal_context(&[AuthScope::KeysBundleRead]),
            FetchCurrentBundleInput { device_id },
        )
        .await?;
    let current_count: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
        FROM keys.key_bundles
        WHERE device_id = $1
          AND is_current = true
        "#,
    )
    .bind(device_id.0)
    .fetch_one(&pool)
    .await?;
    let first_bundle_superseded_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        r#"
        SELECT superseded_at
        FROM keys.key_bundles
        WHERE bundle_id = $1
        "#,
    )
    .bind(first_bundle.bundle_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(first_bundle.bundle_version, 1);
    assert_eq!(second_bundle.bundle_version, 2);
    assert_eq!(current_bundle.bundle_id, second_bundle.bundle_id);
    assert_eq!(current_count, 1);
    assert!(first_bundle_superseded_at.is_some());

    Ok(())
}

#[tokio::test]
async fn publishing_signed_prekey_supersedes_existing_current_prekey(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("keys_application_publish_signed_prekey", &KEYS_MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;

    let repository = CockroachKeyRepository::new(pool.clone());
    let account_id = AccountId(Uuid::now_v7());
    let device_id = DeviceId(Uuid::now_v7());
    provision_identity_device(&repository, account_id, device_id).await?;

    let lifecycle_reader = CockroachIdentityLifecycleReader::new(repository.pool().clone());
    let service = KeyCommandService::new(repository.clone(), lifecycle_reader);

    let first = service
        .publish_signed_prekey(
            internal_context(&[AuthScope::KeysSignedPrekeyPublish]),
            PublishSignedPrekeyInput {
                device_id,
                public_key: vec![1, 2, 3],
                signature: vec![4, 5, 6],
            },
        )
        .await?;
    let second = service
        .publish_signed_prekey(
            internal_context(&[AuthScope::KeysSignedPrekeyPublish]),
            PublishSignedPrekeyInput {
                device_id,
                public_key: vec![7, 8, 9],
                signature: vec![1, 1, 1],
            },
        )
        .await?;
    let current = SignedPrekeyRepository::get_current(&repository, device_id)
        .await?
        .expect("current signed prekey should exist");
    let current_count: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
        FROM keys.signed_prekeys
        WHERE device_id = $1
          AND superseded_at IS NULL
        "#,
    )
    .bind(device_id.0)
    .fetch_one(&pool)
    .await?;
    let first_superseded_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        r#"
        SELECT superseded_at
        FROM keys.signed_prekeys
        WHERE signed_prekey_id = $1
        "#,
    )
    .bind(first.signed_prekey_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(current.signed_prekey_id, second.signed_prekey_id);
    assert_eq!(current_count, 1);
    assert!(first_superseded_at.is_some());

    Ok(())
}

#[tokio::test]
async fn publishing_signed_prekey_rejects_invalid_device_id(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database(
            "keys_application_invalid_signed_prekey_device",
            &KEYS_MIGRATOR,
        )
        .await?;
    apply_identity_schema(&pool).await?;

    let repository = CockroachKeyRepository::new(pool);
    let lifecycle_reader = CockroachIdentityLifecycleReader::new(repository.pool().clone());
    let service = KeyCommandService::new(repository, lifecycle_reader);

    let result = service
        .publish_signed_prekey(
            internal_context(&[AuthScope::KeysSignedPrekeyPublish]),
            PublishSignedPrekeyInput {
                device_id: DeviceId(Uuid::nil()),
                public_key: vec![1, 2, 3],
                signature: vec![4, 5, 6],
            },
        )
        .await;

    assert!(matches!(result, Err(KeyServiceError::InvalidDeviceId)));

    Ok(())
}

#[tokio::test]
async fn publishing_signed_prekey_rejects_revoked_device() -> Result<(), Box<dyn std::error::Error>>
{
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database(
            "keys_application_revoked_signed_prekey_device",
            &KEYS_MIGRATOR,
        )
        .await?;
    apply_identity_schema(&pool).await?;

    let repository = CockroachKeyRepository::new(pool);
    let account_id = AccountId(Uuid::now_v7());
    let device_id = DeviceId(Uuid::now_v7());
    provision_identity_device(&repository, account_id, device_id).await?;
    revoke_identity_device(&repository, account_id, device_id).await?;

    let lifecycle_reader = CockroachIdentityLifecycleReader::new(repository.pool().clone());
    let service = KeyCommandService::new(repository, lifecycle_reader);

    let result = service
        .publish_signed_prekey(
            internal_context(&[AuthScope::KeysSignedPrekeyPublish]),
            PublishSignedPrekeyInput {
                device_id,
                public_key: vec![1, 2, 3],
                signature: vec![4, 5, 6],
            },
        )
        .await;

    assert!(matches!(result, Err(KeyServiceError::DeviceNotActive)));

    Ok(())
}

#[tokio::test]
async fn publishing_bundle_rejects_signed_prekey_from_different_device(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("keys_application_bundle_device_mismatch", &KEYS_MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;

    let repository = CockroachKeyRepository::new(pool);
    let first_account_id = AccountId(Uuid::now_v7());
    let first_device_id = DeviceId(Uuid::now_v7());
    let second_account_id = AccountId(Uuid::now_v7());
    let second_device_id = DeviceId(Uuid::now_v7());
    provision_identity_device(&repository, first_account_id, first_device_id).await?;
    provision_identity_device(&repository, second_account_id, second_device_id).await?;

    let lifecycle_reader = CockroachIdentityLifecycleReader::new(repository.pool().clone());
    let service = KeyCommandService::new(repository, lifecycle_reader);

    let foreign_signed_prekey = service
        .publish_signed_prekey(
            internal_context(&[AuthScope::KeysSignedPrekeyPublish]),
            PublishSignedPrekeyInput {
                device_id: second_device_id,
                public_key: vec![8, 8, 8],
                signature: vec![9, 9, 9],
            },
        )
        .await?;

    let result = service
        .publish_key_bundle(
            internal_context(&[AuthScope::KeysBundlePublish]),
            PublishKeyBundleInput {
                device_id: first_device_id,
                identity_key_public: vec![1, 1, 1],
                identity_key_alg: "x25519".to_owned(),
                signed_prekey_id: foreign_signed_prekey.signed_prekey_id,
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(KeyServiceError::SignedPrekeyDeviceMismatch)
    ));

    Ok(())
}
