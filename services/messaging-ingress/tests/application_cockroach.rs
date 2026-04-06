use chrono::{Duration, Timelike, Utc};
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
        KeyCommandService, KeyUseCases, PublishKeyBundleInput, PublishOneTimePrekeysInput,
        PublishSignedPrekeyInput,
    },
    repository::CockroachKeyRepository,
};
use reach_messaging_ingress_service::{
    application::{
        AcceptEncryptedEnvelopeInput, MessagingIngressCommandService, MessagingIngressUseCases,
    },
    domain::PrekeyResolutionMode,
    errors::MessagingIngressError,
    repository::CockroachMessagingIngressRepository,
};
use reach_test_support::CockroachTestContext;
use sqlx::migrate::Migrator;
use std::sync::Arc;
use tokio::{sync::Barrier, task::JoinHandle};
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

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

async fn apply_keys_schema(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("CREATE SCHEMA IF NOT EXISTS keys")
        .execute(pool)
        .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS keys.key_bundles (
            bundle_id UUID PRIMARY KEY,
            device_id UUID NOT NULL,
            bundle_version INT8 NOT NULL,
            identity_key_public BYTES NOT NULL,
            identity_key_alg STRING NOT NULL,
            signed_prekey_id UUID NOT NULL,
            published_at TIMESTAMPTZ NOT NULL,
            superseded_at TIMESTAMPTZ NULL,
            is_current BOOL NOT NULL,
            CONSTRAINT key_bundles_device_version_unique UNIQUE (device_id, bundle_version)
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS keys.signed_prekeys (
            signed_prekey_id UUID PRIMARY KEY,
            device_id UUID NOT NULL,
            public_key BYTES NOT NULL,
            signature BYTES NOT NULL,
            created_at TIMESTAMPTZ NOT NULL,
            superseded_at TIMESTAMPTZ NULL
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS keys.one_time_prekeys (
            prekey_id UUID PRIMARY KEY,
            device_id UUID NOT NULL,
            public_key BYTES NOT NULL,
            state STRING NOT NULL,
            created_at TIMESTAMPTZ NOT NULL,
            claimed_at TIMESTAMPTZ NULL
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS key_bundles_current_device_idx
            ON keys.key_bundles (device_id)
            WHERE is_current = true
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS signed_prekeys_current_device_idx
            ON keys.signed_prekeys (device_id)
            WHERE superseded_at IS NULL
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        ALTER TABLE keys.key_bundles
            ADD CONSTRAINT key_bundles_signed_prekey_fk
            FOREIGN KEY (signed_prekey_id)
            REFERENCES keys.signed_prekeys (signed_prekey_id)
        "#,
    )
    .execute(pool)
    .await
    .ok();

    Ok(())
}

async fn provision_account_and_device(
    pool: &sqlx::PgPool,
    account_id: AccountId,
    device_id: DeviceId,
    device_number: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let identity = IdentityCommandService::new(CockroachIdentityRepository::new(pool.clone()));

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
                device_number,
                platform: "ios".to_owned(),
                app_version: "1.0.0".to_owned(),
            },
        )
        .await?;

    Ok(())
}

async fn revoke_device(
    pool: &sqlx::PgPool,
    account_id: AccountId,
    device_id: DeviceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let identity = IdentityCommandService::new(CockroachIdentityRepository::new(pool.clone()));

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

async fn provision_recipient_keys(
    pool: &sqlx::PgPool,
    device_id: DeviceId,
    include_one_time_prekeys: bool,
) -> Result<Uuid, Box<dyn std::error::Error>> {
    let repository = CockroachKeyRepository::new(pool.clone());
    let lifecycle_reader = CockroachIdentityLifecycleReader::new(pool.clone());
    let service = KeyCommandService::new(repository, lifecycle_reader);

    let signed_prekey = service
        .publish_signed_prekey(
            internal_context(&[AuthScope::KeysSignedPrekeyPublish]),
            PublishSignedPrekeyInput {
                device_id,
                public_key: vec![1, 2, 3, 4],
                signature: vec![5, 6, 7, 8],
            },
        )
        .await?;
    let bundle = service
        .publish_key_bundle(
            internal_context(&[AuthScope::KeysBundlePublish]),
            PublishKeyBundleInput {
                device_id,
                identity_key_public: vec![9, 9, 9, 9],
                identity_key_alg: "x25519".to_owned(),
                signed_prekey_id: signed_prekey.signed_prekey_id,
            },
        )
        .await?;

    if include_one_time_prekeys {
        service
            .publish_one_time_prekeys(
                internal_context(&[AuthScope::KeysOneTimePrekeysPublish]),
                PublishOneTimePrekeysInput {
                    device_id,
                    prekeys: vec![vec![8, 8, 8, 8]],
                },
            )
            .await?;
    }
    Ok(bundle.bundle_id)
}

fn build_service(
    pool: &sqlx::PgPool,
) -> MessagingIngressCommandService<
    CockroachMessagingIngressRepository,
    CockroachIdentityLifecycleReader,
> {
    let repository = CockroachMessagingIngressRepository::new(pool.clone());
    let lifecycle_reader = CockroachIdentityLifecycleReader::new(pool.clone());
    MessagingIngressCommandService::new(repository, lifecycle_reader)
}

fn valid_command(
    sender_account_id: AccountId,
    sender_device_id: DeviceId,
    recipient_account_id: AccountId,
    recipient_device_id: DeviceId,
    prekey_resolution_mode: PrekeyResolutionMode,
) -> AcceptEncryptedEnvelopeInput {
    AcceptEncryptedEnvelopeInput {
        envelope_id: Uuid::now_v7(),
        sender_account_id,
        sender_device_id,
        recipient_account_id,
        recipient_device_id,
        encrypted_payload: vec![1, 2, 3, 4, 5, 6],
        content_type: "sealed_box".to_owned(),
        client_timestamp: db_timestamp(),
        replay_nonce: vec![7; 16],
        payload_version: "v1".to_owned(),
        prekey_resolution_mode,
    }
}

#[tokio::test]
async fn valid_envelope_acceptance_persists_replay_and_resolution_metadata(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("messaging_ingress_valid_acceptance", &MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;
    apply_keys_schema(&pool).await?;

    let sender_account_id = AccountId(Uuid::now_v7());
    let sender_device_id = DeviceId(Uuid::now_v7());
    let recipient_account_id = AccountId(Uuid::now_v7());
    let recipient_device_id = DeviceId(Uuid::now_v7());

    provision_account_and_device(&pool, sender_account_id, sender_device_id, 1).await?;
    provision_account_and_device(&pool, recipient_account_id, recipient_device_id, 1).await?;
    let expected_bundle_id = provision_recipient_keys(&pool, recipient_device_id, true).await?;

    let service = build_service(&pool);
    let accepted = service
        .accept_encrypted_envelope(
            internal_context(&[AuthScope::MessagingIngressEnvelopeAccept]),
            valid_command(
                sender_account_id,
                sender_device_id,
                recipient_account_id,
                recipient_device_id,
                PrekeyResolutionMode::CurrentBundleAndOneTimePrekey,
            ),
        )
        .await?;

    let accepted_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM messaging_ingress.accepted_envelopes")
            .fetch_one(&pool)
            .await?;
    let replay_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM messaging_ingress.envelope_replay_records")
            .fetch_one(&pool)
            .await?;

    assert_eq!(accepted.recipient_bundle_id, expected_bundle_id);
    assert!(accepted.claimed_one_time_prekey_id.is_some());
    assert_eq!(accepted_count, 1);
    assert_eq!(replay_count, 1);

    Ok(())
}

#[tokio::test]
async fn sender_lifecycle_rejection_blocks_acceptance() -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("messaging_ingress_sender_rejection", &MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;
    apply_keys_schema(&pool).await?;

    let sender_account_id = AccountId(Uuid::now_v7());
    let sender_device_id = DeviceId(Uuid::now_v7());
    let recipient_account_id = AccountId(Uuid::now_v7());
    let recipient_device_id = DeviceId(Uuid::now_v7());

    provision_account_and_device(&pool, sender_account_id, sender_device_id, 1).await?;
    provision_account_and_device(&pool, recipient_account_id, recipient_device_id, 1).await?;
    provision_recipient_keys(&pool, recipient_device_id, false).await?;
    revoke_device(&pool, sender_account_id, sender_device_id).await?;

    let service = build_service(&pool);
    let result = service
        .accept_encrypted_envelope(
            internal_context(&[AuthScope::MessagingIngressEnvelopeAccept]),
            valid_command(
                sender_account_id,
                sender_device_id,
                recipient_account_id,
                recipient_device_id,
                PrekeyResolutionMode::CurrentBundleOnly,
            ),
        )
        .await;

    assert!(matches!(
        result,
        Err(MessagingIngressError::SenderDeviceNotActive)
    ));

    Ok(())
}

#[tokio::test]
async fn recipient_lifecycle_rejection_blocks_acceptance() -> Result<(), Box<dyn std::error::Error>>
{
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("messaging_ingress_recipient_rejection", &MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;
    apply_keys_schema(&pool).await?;

    let sender_account_id = AccountId(Uuid::now_v7());
    let sender_device_id = DeviceId(Uuid::now_v7());
    let recipient_account_id = AccountId(Uuid::now_v7());
    let recipient_device_id = DeviceId(Uuid::now_v7());

    provision_account_and_device(&pool, sender_account_id, sender_device_id, 1).await?;
    provision_account_and_device(&pool, recipient_account_id, recipient_device_id, 1).await?;
    provision_recipient_keys(&pool, recipient_device_id, false).await?;
    revoke_device(&pool, recipient_account_id, recipient_device_id).await?;

    let service = build_service(&pool);
    let result = service
        .accept_encrypted_envelope(
            internal_context(&[AuthScope::MessagingIngressEnvelopeAccept]),
            valid_command(
                sender_account_id,
                sender_device_id,
                recipient_account_id,
                recipient_device_id,
                PrekeyResolutionMode::CurrentBundleOnly,
            ),
        )
        .await;

    assert!(matches!(
        result,
        Err(MessagingIngressError::RecipientDeviceNotActive)
    ));

    Ok(())
}

#[tokio::test]
async fn missing_recipient_bundle_rejects_acceptance() -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("messaging_ingress_missing_bundle", &MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;
    apply_keys_schema(&pool).await?;

    let sender_account_id = AccountId(Uuid::now_v7());
    let sender_device_id = DeviceId(Uuid::now_v7());
    let recipient_account_id = AccountId(Uuid::now_v7());
    let recipient_device_id = DeviceId(Uuid::now_v7());

    provision_account_and_device(&pool, sender_account_id, sender_device_id, 1).await?;
    provision_account_and_device(&pool, recipient_account_id, recipient_device_id, 1).await?;

    let service = build_service(&pool);
    let result = service
        .accept_encrypted_envelope(
            internal_context(&[AuthScope::MessagingIngressEnvelopeAccept]),
            valid_command(
                sender_account_id,
                sender_device_id,
                recipient_account_id,
                recipient_device_id,
                PrekeyResolutionMode::CurrentBundleOnly,
            ),
        )
        .await;

    assert!(matches!(
        result,
        Err(MessagingIngressError::RecipientBundleUnavailable)
    ));

    Ok(())
}

#[tokio::test]
async fn missing_one_time_prekey_rejects_when_required() -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("messaging_ingress_missing_one_time_prekey", &MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;
    apply_keys_schema(&pool).await?;

    let sender_account_id = AccountId(Uuid::now_v7());
    let sender_device_id = DeviceId(Uuid::now_v7());
    let recipient_account_id = AccountId(Uuid::now_v7());
    let recipient_device_id = DeviceId(Uuid::now_v7());

    provision_account_and_device(&pool, sender_account_id, sender_device_id, 1).await?;
    provision_account_and_device(&pool, recipient_account_id, recipient_device_id, 1).await?;
    provision_recipient_keys(&pool, recipient_device_id, false).await?;

    let service = build_service(&pool);
    let result = service
        .accept_encrypted_envelope(
            internal_context(&[AuthScope::MessagingIngressEnvelopeAccept]),
            valid_command(
                sender_account_id,
                sender_device_id,
                recipient_account_id,
                recipient_device_id,
                PrekeyResolutionMode::CurrentBundleAndOneTimePrekey,
            ),
        )
        .await;

    let accepted_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM messaging_ingress.accepted_envelopes")
            .fetch_one(&pool)
            .await?;
    let replay_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM messaging_ingress.envelope_replay_records")
            .fetch_one(&pool)
            .await?;

    assert!(matches!(
        result,
        Err(MessagingIngressError::RecipientOneTimePrekeyUnavailable)
    ));
    assert_eq!(accepted_count, 0);
    assert_eq!(replay_count, 0);

    Ok(())
}

#[tokio::test]
async fn replay_nonce_is_deduplicated_durably() -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("messaging_ingress_replay_nonce", &MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;
    apply_keys_schema(&pool).await?;

    let sender_account_id = AccountId(Uuid::now_v7());
    let sender_device_id = DeviceId(Uuid::now_v7());
    let recipient_account_id = AccountId(Uuid::now_v7());
    let recipient_device_id = DeviceId(Uuid::now_v7());

    provision_account_and_device(&pool, sender_account_id, sender_device_id, 1).await?;
    provision_account_and_device(&pool, recipient_account_id, recipient_device_id, 1).await?;
    provision_recipient_keys(&pool, recipient_device_id, false).await?;

    let service = build_service(&pool);
    let first = valid_command(
        sender_account_id,
        sender_device_id,
        recipient_account_id,
        recipient_device_id,
        PrekeyResolutionMode::CurrentBundleOnly,
    );
    let mut second = valid_command(
        sender_account_id,
        sender_device_id,
        recipient_account_id,
        recipient_device_id,
        PrekeyResolutionMode::CurrentBundleOnly,
    );
    second.replay_nonce = first.replay_nonce.clone();

    service
        .accept_encrypted_envelope(
            internal_context(&[AuthScope::MessagingIngressEnvelopeAccept]),
            first,
        )
        .await?;
    let result = service
        .accept_encrypted_envelope(
            internal_context(&[AuthScope::MessagingIngressEnvelopeAccept]),
            second,
        )
        .await;

    assert!(matches!(
        result,
        Err(MessagingIngressError::ReplayNonceConflict)
    ));

    Ok(())
}

#[tokio::test]
async fn duplicate_envelope_id_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("messaging_ingress_duplicate_envelope", &MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;
    apply_keys_schema(&pool).await?;

    let sender_account_id = AccountId(Uuid::now_v7());
    let sender_device_id = DeviceId(Uuid::now_v7());
    let recipient_account_id = AccountId(Uuid::now_v7());
    let recipient_device_id = DeviceId(Uuid::now_v7());

    provision_account_and_device(&pool, sender_account_id, sender_device_id, 1).await?;
    provision_account_and_device(&pool, recipient_account_id, recipient_device_id, 1).await?;
    provision_recipient_keys(&pool, recipient_device_id, false).await?;

    let service = build_service(&pool);
    let first = valid_command(
        sender_account_id,
        sender_device_id,
        recipient_account_id,
        recipient_device_id,
        PrekeyResolutionMode::CurrentBundleOnly,
    );
    let mut second = valid_command(
        sender_account_id,
        sender_device_id,
        recipient_account_id,
        recipient_device_id,
        PrekeyResolutionMode::CurrentBundleOnly,
    );
    second.envelope_id = first.envelope_id;

    service
        .accept_encrypted_envelope(
            internal_context(&[AuthScope::MessagingIngressEnvelopeAccept]),
            first,
        )
        .await?;
    let result = service
        .accept_encrypted_envelope(
            internal_context(&[AuthScope::MessagingIngressEnvelopeAccept]),
            second,
        )
        .await;

    assert!(matches!(
        result,
        Err(MessagingIngressError::EnvelopeAlreadyExists)
    ));

    Ok(())
}

#[tokio::test]
async fn oversized_payload_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("messaging_ingress_payload_too_large", &MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;
    apply_keys_schema(&pool).await?;

    let sender_account_id = AccountId(Uuid::now_v7());
    let sender_device_id = DeviceId(Uuid::now_v7());
    let recipient_account_id = AccountId(Uuid::now_v7());
    let recipient_device_id = DeviceId(Uuid::now_v7());

    provision_account_and_device(&pool, sender_account_id, sender_device_id, 1).await?;
    provision_account_and_device(&pool, recipient_account_id, recipient_device_id, 1).await?;
    provision_recipient_keys(&pool, recipient_device_id, false).await?;

    let service = build_service(&pool);
    let mut command = valid_command(
        sender_account_id,
        sender_device_id,
        recipient_account_id,
        recipient_device_id,
        PrekeyResolutionMode::CurrentBundleOnly,
    );
    command.encrypted_payload = vec![0; 131_073];

    let result = service
        .accept_encrypted_envelope(
            internal_context(&[AuthScope::MessagingIngressEnvelopeAccept]),
            command,
        )
        .await;

    assert!(matches!(
        result,
        Err(MessagingIngressError::PayloadTooLarge)
    ));

    Ok(())
}

#[tokio::test]
async fn old_client_timestamp_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("messaging_ingress_old_timestamp", &MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;
    apply_keys_schema(&pool).await?;

    let sender_account_id = AccountId(Uuid::now_v7());
    let sender_device_id = DeviceId(Uuid::now_v7());
    let recipient_account_id = AccountId(Uuid::now_v7());
    let recipient_device_id = DeviceId(Uuid::now_v7());

    provision_account_and_device(&pool, sender_account_id, sender_device_id, 1).await?;
    provision_account_and_device(&pool, recipient_account_id, recipient_device_id, 1).await?;
    provision_recipient_keys(&pool, recipient_device_id, false).await?;

    let service = build_service(&pool);
    let mut command = valid_command(
        sender_account_id,
        sender_device_id,
        recipient_account_id,
        recipient_device_id,
        PrekeyResolutionMode::CurrentBundleOnly,
    );
    command.client_timestamp = Utc::now() - Duration::days(8);

    let result = service
        .accept_encrypted_envelope(
            internal_context(&[AuthScope::MessagingIngressEnvelopeAccept]),
            command,
        )
        .await;

    assert!(matches!(
        result,
        Err(MessagingIngressError::ClientTimestampTooOld)
    ));

    Ok(())
}

#[tokio::test]
async fn concurrent_replay_submissions_only_accept_once() -> Result<(), Box<dyn std::error::Error>>
{
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("messaging_ingress_concurrent_replay", &MIGRATOR)
        .await?;
    apply_identity_schema(&pool).await?;
    apply_keys_schema(&pool).await?;

    let sender_account_id = AccountId(Uuid::now_v7());
    let sender_device_id = DeviceId(Uuid::now_v7());
    let recipient_account_id = AccountId(Uuid::now_v7());
    let recipient_device_id = DeviceId(Uuid::now_v7());

    provision_account_and_device(&pool, sender_account_id, sender_device_id, 1).await?;
    provision_account_and_device(&pool, recipient_account_id, recipient_device_id, 1).await?;
    provision_recipient_keys(&pool, recipient_device_id, false).await?;

    let service = Arc::new(build_service(&pool));
    let barrier = Arc::new(Barrier::new(4));
    let base = valid_command(
        sender_account_id,
        sender_device_id,
        recipient_account_id,
        recipient_device_id,
        PrekeyResolutionMode::CurrentBundleOnly,
    );
    let mut handles: Vec<JoinHandle<Result<Uuid, MessagingIngressError>>> = Vec::new();

    for _ in 0..4 {
        let service = Arc::clone(&service);
        let barrier = Arc::clone(&barrier);
        let mut command = base.clone();
        command.envelope_id = Uuid::now_v7();
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            service
                .accept_encrypted_envelope(
                    internal_context(&[AuthScope::MessagingIngressEnvelopeAccept]),
                    command,
                )
                .await
                .map(|accepted| accepted.envelope.envelope_id)
        }));
    }

    let mut success_count = 0usize;
    let mut replay_conflicts = 0usize;
    for handle in handles {
        let result = handle
            .await
            .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
        match result {
            Ok(_accepted_id) => success_count += 1,
            Err(MessagingIngressError::ReplayNonceConflict) => replay_conflicts += 1,
            Err(other) => return Err(Box::new(other) as Box<dyn std::error::Error>),
        }
    }

    let accepted_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM messaging_ingress.accepted_envelopes")
            .fetch_one(&pool)
            .await?;

    assert_eq!(accepted_count, 1);
    assert_eq!(success_count, 1);
    assert_eq!(replay_conflicts, 3);

    Ok(())
}
