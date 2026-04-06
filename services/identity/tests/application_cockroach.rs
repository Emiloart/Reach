use reach_auth_types::{
    AccountId, AuthScope, DeviceId, InternalServicePrincipal, Principal, RequestContext,
};
use reach_identity_service::{
    application::{
        CreateAccountInput, IdentityCommandService, IdentityUseCases, RegisterDeviceInput,
        RevokeDeviceInput,
    },
    domain::DeviceStatus,
    errors::IdentityError,
    repository::CockroachIdentityRepository,
};
use reach_test_support::CockroachTestContext;
use sqlx::migrate::Migrator;
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

fn internal_context(scopes: &[AuthScope]) -> RequestContext {
    RequestContext {
        principal: Principal::InternalService(InternalServicePrincipal {
            service_name: "reach-tests".to_owned(),
            scopes: scopes.to_vec(),
        }),
        request_id: Some("req-test".to_owned()),
    }
}

#[tokio::test]
async fn duplicate_device_registration_returns_conflict() -> Result<(), Box<dyn std::error::Error>>
{
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("identity_application_duplicate_device", &MIGRATOR)
        .await?;
    let repository = CockroachIdentityRepository::new(pool);
    let service = IdentityCommandService::new(repository);
    let account_id = AccountId(Uuid::now_v7());
    let create_account_context = internal_context(&[AuthScope::IdentityAccountCreate]);
    let register_device_context = internal_context(&[AuthScope::IdentityDeviceRegister]);

    service
        .create_account(create_account_context, CreateAccountInput { account_id })
        .await?;
    service
        .register_device(
            register_device_context.clone(),
            RegisterDeviceInput {
                account_id,
                device_id: DeviceId(Uuid::now_v7()),
                device_number: 1,
                platform: "ios".to_owned(),
                app_version: "1.0.0".to_owned(),
            },
        )
        .await?;

    let duplicate = service
        .register_device(
            register_device_context,
            RegisterDeviceInput {
                account_id,
                device_id: DeviceId(Uuid::now_v7()),
                device_number: 1,
                platform: "android".to_owned(),
                app_version: "1.0.0".to_owned(),
            },
        )
        .await;

    assert!(matches!(
        duplicate,
        Err(IdentityError::DeviceRegistrationConflict)
    ));

    Ok(())
}

#[tokio::test]
async fn revoked_device_cannot_be_revoked_twice() -> Result<(), Box<dyn std::error::Error>> {
    let test_context = CockroachTestContext::start().await?;
    let pool = test_context
        .provision_database("identity_application_revoke_device", &MIGRATOR)
        .await?;
    let repository = CockroachIdentityRepository::new(pool);
    let service = IdentityCommandService::new(repository);
    let account_id = AccountId(Uuid::now_v7());
    let device_id = DeviceId(Uuid::now_v7());
    let create_account_context = internal_context(&[AuthScope::IdentityAccountCreate]);
    let register_device_context = internal_context(&[AuthScope::IdentityDeviceRegister]);
    let revoke_device_context = internal_context(&[AuthScope::IdentityDeviceRevoke]);

    service
        .create_account(create_account_context, CreateAccountInput { account_id })
        .await?;
    service
        .register_device(
            register_device_context,
            RegisterDeviceInput {
                account_id,
                device_id,
                device_number: 1,
                platform: "ios".to_owned(),
                app_version: "1.0.0".to_owned(),
            },
        )
        .await?;

    let revoked = service
        .revoke_device(
            revoke_device_context.clone(),
            RevokeDeviceInput {
                account_id,
                device_id,
            },
        )
        .await?;
    let second_revoke = service
        .revoke_device(
            revoke_device_context,
            RevokeDeviceInput {
                account_id,
                device_id,
            },
        )
        .await;

    assert_eq!(revoked.status, DeviceStatus::Revoked);
    assert!(revoked.revoked_at.is_some());
    assert!(matches!(
        second_revoke,
        Err(IdentityError::DeviceAlreadyRevoked)
    ));

    Ok(())
}
