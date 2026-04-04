use reach_config::{FromEnvironment, KeyServiceConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = KeyServiceConfig::from_env()?;
    reach_telemetry::init(&config.service.name, &config.telemetry)?;

    let listener = tokio::net::TcpListener::bind(config.http.bind_addr).await?;
    axum::serve(listener, reach_key_service::bootstrap::build_router()).await?;

    Ok(())
}
