use chrono::{Timelike, Utc};
use reach_auth_types::DeviceId;
use reach_key_service::{
    domain::{KeyBundle, OneTimePrekey, OneTimePrekeyState, SignedPrekey},
    repository::{
        CockroachKeyRepository, KeyBundleRepository, OneTimePrekeyRepository,
        SignedPrekeyRepository,
    },
};
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
async fn key_bundle_storage_and_fetch_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("keys_bundle_store", &MIGRATOR)
        .await?;
    let repository = CockroachKeyRepository::new(pool);

    let device_id = DeviceId(Uuid::now_v7());
    let signed_prekey = SignedPrekey {
        signed_prekey_id: Uuid::now_v7(),
        device_id,
        public_key: vec![1, 2, 3, 4],
        signature: vec![5, 6, 7, 8],
        created_at: db_timestamp(),
        superseded_at: None,
    };
    SignedPrekeyRepository::insert(&repository, &signed_prekey).await?;

    let key_bundle = KeyBundle {
        bundle_id: Uuid::now_v7(),
        device_id,
        bundle_version: 1,
        identity_key_public: vec![9, 10, 11],
        identity_key_alg: "x25519".to_owned(),
        signed_prekey_id: signed_prekey.signed_prekey_id,
        published_at: db_timestamp(),
        superseded_at: None,
        is_current: true,
    };
    KeyBundleRepository::insert(&repository, &key_bundle).await?;

    let fetched = KeyBundleRepository::get_current(&repository, device_id).await?;
    let fetched_prekey =
        SignedPrekeyRepository::get_by_id(&repository, signed_prekey.signed_prekey_id).await?;

    assert_eq!(fetched, Some(key_bundle));
    assert_eq!(fetched_prekey, Some(signed_prekey));

    Ok(())
}

#[tokio::test]
async fn one_time_prekeys_store_and_claim_in_order() -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("keys_prekeys_claim", &MIGRATOR)
        .await?;
    let repository = CockroachKeyRepository::new(pool);

    let device_id = DeviceId(Uuid::now_v7());
    let created_at = db_timestamp();
    let first = OneTimePrekey {
        prekey_id: Uuid::now_v7(),
        device_id,
        public_key: vec![1, 1, 1],
        state: OneTimePrekeyState::Available,
        created_at,
        claimed_at: None,
    };
    let second = OneTimePrekey {
        prekey_id: Uuid::now_v7(),
        device_id,
        public_key: vec![2, 2, 2],
        state: OneTimePrekeyState::Available,
        created_at: created_at + chrono::Duration::milliseconds(1),
        claimed_at: None,
    };

    repository
        .insert_batch(&[first.clone(), second.clone()])
        .await?;

    let claimed = repository.claim_next_available(device_id).await?;
    let next_claimed = repository.claim_next_available(device_id).await?;

    assert_eq!(
        claimed.as_ref().map(|prekey| prekey.prekey_id),
        Some(first.prekey_id)
    );
    assert_eq!(
        claimed.as_ref().map(|prekey| prekey.state),
        Some(OneTimePrekeyState::Claimed)
    );
    assert_eq!(
        next_claimed.as_ref().map(|prekey| prekey.prekey_id),
        Some(second.prekey_id)
    );

    Ok(())
}
