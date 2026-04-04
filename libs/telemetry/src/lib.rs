use reach_config::TelemetryConfig;
use thiserror::Error;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init(service_name: &str, config: &TelemetryConfig) -> Result<(), TelemetryError> {
    if config.otlp_endpoint.is_some() {
        return Err(TelemetryError::OtlpNotYetImplemented);
    }

    let env_filter =
        EnvFilter::try_new(config.log_filter.clone()).unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true);

    let registry = tracing_subscriber::registry().with(env_filter);

    if config.log_json {
        registry.with(fmt_layer.json()).try_init()?;
    } else {
        registry.with(fmt_layer).try_init()?;
    }

    tracing::info!(service = service_name, "telemetry initialized");

    Ok(())
}

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("OTLP exporter wiring is not implemented yet")]
    OtlpNotYetImplemented,
    #[error("failed to initialize tracing subscriber: {0}")]
    Initialization(#[from] tracing_subscriber::util::TryInitError),
}
