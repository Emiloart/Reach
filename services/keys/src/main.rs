use reach_config::{FromEnvironment, KeyServiceConfig};
use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
use std::{sync::Arc, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = KeyServiceConfig::from_env()?;
    reach_telemetry::init(&config.service.name, &config.telemetry)?;

    let pool = PgPoolOptions::new()
        .min_connections(config.database.min_connections)
        .max_connections(config.database.max_connections)
        .acquire_timeout(Duration::from_secs(config.database.acquire_timeout_seconds))
        .connect(config.database.url.expose_secret())
        .await?;
    let repository = reach_key_service::repository::CockroachKeyRepository::new(pool);
    let use_cases: Arc<dyn reach_key_service::application::KeyUseCases> = Arc::new(
        reach_key_service::application::KeyCommandService::new(repository),
    );

    let listener = tokio::net::TcpListener::bind(config.http.bind_addr).await?;
    axum::serve(
        listener,
        reach_key_service::bootstrap::build_router(use_cases),
    )
    .await?;

    Ok(())
}
