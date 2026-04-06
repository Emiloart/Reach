use reach_config::{FromEnvironment, MessagingIngressServiceConfig};
use reach_identity_lifecycle::CockroachIdentityLifecycleReader;
use reach_request_auth::InternalRequestAuthenticator;
use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
use std::{sync::Arc, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = MessagingIngressServiceConfig::from_env()?;
    reach_telemetry::init(&config.service.name, &config.telemetry)?;

    let pool = PgPoolOptions::new()
        .min_connections(config.database.min_connections)
        .max_connections(config.database.max_connections)
        .acquire_timeout(Duration::from_secs(config.database.acquire_timeout_seconds))
        .connect(config.database.url.expose_secret())
        .await?;
    let authenticator = Arc::new(InternalRequestAuthenticator::from_config(
        &config.internal_auth,
    )?);
    let repository =
        reach_messaging_ingress_service::repository::CockroachMessagingIngressRepository::new(pool);
    let lifecycle_reader = CockroachIdentityLifecycleReader::new(repository.pool().clone());
    let use_cases: Arc<dyn reach_messaging_ingress_service::application::MessagingIngressUseCases> =
        Arc::new(
            reach_messaging_ingress_service::application::MessagingIngressCommandService::new(
                repository,
                lifecycle_reader,
            ),
        );

    let listener = tokio::net::TcpListener::bind(config.http.bind_addr).await?;
    axum::serve(
        listener,
        reach_messaging_ingress_service::bootstrap::build_router(use_cases, authenticator),
    )
    .await?;

    Ok(())
}
