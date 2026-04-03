use reach_config::{FromEnvironment, IdentityServiceConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = IdentityServiceConfig::from_env()?;
    reach_telemetry::init(&config.service.name, &config.telemetry)?;

    let listener = tokio::net::TcpListener::bind(config.http.bind_addr).await?;
    axum::serve(listener, reach_identity_service::bootstrap::build_router()).await?;

    Ok(())
}

