use chrono::{Timelike, Utc};
use reach_auth_types::DeviceId;
use reach_key_service::{
    application::{
        ClaimOneTimePrekeyInput, FetchCurrentBundleInput, KeyCommandService, KeyUseCases,
        PublishKeyBundleInput, PublishOneTimePrekeysInput,
    },
    domain::SignedPrekey,
    errors::KeyServiceError,
    repository::{CockroachKeyRepository, SignedPrekeyRepository},
};
use reach_test_support::CockroachTestContext;
use sqlx::migrate::Migrator;
use std::{collections::HashSet, sync::Arc};
use tokio::{sync::Barrier, task::JoinHandle};
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

fn db_timestamp() -> chrono::DateTime<chrono::Utc> {
    let timestamp = Utc::now();
    timestamp
        .with_nanosecond((timestamp.nanosecond() / 1_000) * 1_000)
        .expect("timestamp nanoseconds should remain valid after microsecond truncation")
}

async fn insert_signed_prekey(
    repository: &CockroachKeyRepository,
    device_id: DeviceId,
) -> Result<SignedPrekey, Box<dyn std::error::Error>> {
    let signed_prekey = SignedPrekey {
        signed_prekey_id: Uuid::now_v7(),
        device_id,
        public_key: vec![1, 2, 3, 4],
        signature: vec![5, 6, 7, 8],
        created_at: db_timestamp(),
        superseded_at: None,
    };

    SignedPrekeyRepository::insert(repository, &signed_prekey).await?;

    Ok(signed_prekey)
}

#[tokio::test]
async fn concurrent_claims_do_not_duplicate_one_time_prekeys(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("keys_application_concurrent_claim", &MIGRATOR)
        .await?;
    let repository = CockroachKeyRepository::new(pool);
    let service = Arc::new(KeyCommandService::new(repository));
    let barrier = Arc::new(Barrier::new(4));
    let device_id = DeviceId(Uuid::now_v7());

    service
        .publish_one_time_prekeys(PublishOneTimePrekeysInput {
            device_id,
            prekeys: vec![vec![1, 1, 1], vec![2, 2, 2]],
        })
        .await?;

    let mut handles: Vec<JoinHandle<Result<Uuid, KeyServiceError>>> = Vec::new();
    for _ in 0..4 {
        let service = Arc::clone(&service);
        let barrier = Arc::clone(&barrier);
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            service
                .claim_one_time_prekey(ClaimOneTimePrekeyInput { device_id })
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
        .provision_database("keys_application_publish_bundle", &MIGRATOR)
        .await?;
    let repository = CockroachKeyRepository::new(pool.clone());
    let service = KeyCommandService::new(repository.clone());
    let device_id = DeviceId(Uuid::now_v7());
    let first_signed_prekey = insert_signed_prekey(&repository, device_id).await?;
    let second_signed_prekey = insert_signed_prekey(&repository, device_id).await?;

    let first_bundle = service
        .publish_key_bundle(PublishKeyBundleInput {
            device_id,
            identity_key_public: vec![9, 9, 9],
            identity_key_alg: "x25519".to_owned(),
            signed_prekey_id: first_signed_prekey.signed_prekey_id,
        })
        .await?;
    let second_bundle = service
        .publish_key_bundle(PublishKeyBundleInput {
            device_id,
            identity_key_public: vec![8, 8, 8],
            identity_key_alg: "x25519".to_owned(),
            signed_prekey_id: second_signed_prekey.signed_prekey_id,
        })
        .await?;
    let current_bundle = service
        .fetch_current_bundle(FetchCurrentBundleInput { device_id })
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
