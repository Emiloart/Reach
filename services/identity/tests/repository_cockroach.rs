use chrono::{Timelike, Utc};
use reach_auth_types::{AccountId, DeviceId};
use reach_identity_service::{
    domain::{Account, AccountState, Device, DeviceStatus},
    repository::{
        AccountRepository, CockroachIdentityRepository, DeviceRepository,
        IdentityConstraintViolation, IdentityRepositoryError,
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
async fn account_creation_persists_and_reads_back() -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("identity_account_create", &MIGRATOR)
        .await?;
    let repository = CockroachIdentityRepository::new(pool);

    let timestamp = db_timestamp();
    let account = Account {
        account_id: AccountId(Uuid::now_v7()),
        state: AccountState::Active,
        created_at: timestamp,
        updated_at: timestamp,
        deletion_requested_at: None,
        purge_after: None,
    };

    AccountRepository::create(&repository, &account).await?;

    let fetched = AccountRepository::get_by_id(&repository, account.account_id).await?;

    assert_eq!(fetched, Some(account));

    Ok(())
}

#[tokio::test]
async fn duplicate_account_creation_returns_constraint_error(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("identity_account_duplicate", &MIGRATOR)
        .await?;
    let repository = CockroachIdentityRepository::new(pool);

    let timestamp = db_timestamp();
    let account = Account {
        account_id: AccountId(Uuid::now_v7()),
        state: AccountState::Active,
        created_at: timestamp,
        updated_at: timestamp,
        deletion_requested_at: None,
        purge_after: None,
    };

    AccountRepository::create(&repository, &account).await?;
    let duplicate = AccountRepository::create(&repository, &account).await;

    assert!(matches!(
        duplicate,
        Err(IdentityRepositoryError::Constraint(
            IdentityConstraintViolation::AccountAlreadyExists
        ))
    ));

    Ok(())
}

#[tokio::test]
async fn device_registration_persists_and_reads_back() -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("identity_device_register", &MIGRATOR)
        .await?;
    let repository = CockroachIdentityRepository::new(pool);

    let timestamp = db_timestamp();
    let account = Account {
        account_id: AccountId(Uuid::now_v7()),
        state: AccountState::Active,
        created_at: timestamp,
        updated_at: timestamp,
        deletion_requested_at: None,
        purge_after: None,
    };
    AccountRepository::create(&repository, &account).await?;

    let device = Device {
        device_id: DeviceId(Uuid::now_v7()),
        account_id: account.account_id,
        device_number: 1,
        platform: "ios".to_owned(),
        app_version: "1.0.0".to_owned(),
        status: DeviceStatus::Active,
        registered_at: timestamp,
        revoked_at: None,
    };

    DeviceRepository::create(&repository, &device).await?;

    let fetched = DeviceRepository::get_by_id(&repository, device.device_id).await?;

    assert_eq!(fetched, Some(device));

    Ok(())
}
