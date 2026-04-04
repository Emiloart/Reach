use reach_config::{AuthServiceConfig, FromEnvironment};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AuthServiceConfig::from_env()?;
    reach_telemetry::init(&config.service.name, &config.telemetry)?;

    let listener = tokio::net::TcpListener::bind(config.http.bind_addr).await?;
    axum::serve(listener, reach_auth_service::bootstrap::build_router()).await?;

    Ok(())
}
