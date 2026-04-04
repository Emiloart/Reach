use reach_config::{FromEnvironment, MessagingIngressServiceConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = MessagingIngressServiceConfig::from_env()?;
    reach_telemetry::init(&config.service.name, &config.telemetry)?;

    let listener = tokio::net::TcpListener::bind(config.http.bind_addr).await?;
    axum::serve(
        listener,
        reach_messaging_ingress_service::bootstrap::build_router(),
    )
    .await?;

    Ok(())
}
