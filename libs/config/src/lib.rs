use config::{Config, Environment};
use secrecy::SecretString;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::net::SocketAddr;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEnvironment {
    Development,
    Staging,
    Production,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceMetadata {
    pub name: String,
    pub environment: RuntimeEnvironment,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpServerConfig {
    pub bind_addr: SocketAddr,
    pub request_timeout_seconds: u64,
    pub shutdown_grace_period_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostgresConfig {
    pub url: SecretString,
    pub min_connections: u32,
    pub max_connections: u32,
    pub acquire_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValkeyConfig {
    pub url: SecretString,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JetStreamConfig {
    pub url: SecretString,
    pub stream_prefix: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TelemetryConfig {
    pub log_json: bool,
    pub log_filter: String,
    pub otlp_endpoint: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SignerConfig {
    pub issuer: String,
    pub audience: String,
    pub active_key_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdentityServiceConfig {
    pub service: ServiceMetadata,
    pub http: HttpServerConfig,
    pub database: PostgresConfig,
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthServiceConfig {
    pub service: ServiceMetadata,
    pub http: HttpServerConfig,
    pub database: PostgresConfig,
    pub valkey: ValkeyConfig,
    pub telemetry: TelemetryConfig,
    pub signer: SignerConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyServiceConfig {
    pub service: ServiceMetadata,
    pub http: HttpServerConfig,
    pub database: PostgresConfig,
    pub valkey: ValkeyConfig,
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessagingIngressServiceConfig {
    pub service: ServiceMetadata,
    pub http: HttpServerConfig,
    pub database: PostgresConfig,
    pub valkey: ValkeyConfig,
    pub jetstream: JetStreamConfig,
    pub telemetry: TelemetryConfig,
}

pub trait FromEnvironment: Sized + DeserializeOwned {
    const PREFIX: &'static str;

    fn from_env() -> Result<Self, ConfigError> {
        load_from_env(Self::PREFIX)
    }
}

impl FromEnvironment for IdentityServiceConfig {
    const PREFIX: &'static str = "REACH_IDENTITY";
}

impl FromEnvironment for AuthServiceConfig {
    const PREFIX: &'static str = "REACH_AUTH";
}

impl FromEnvironment for KeyServiceConfig {
    const PREFIX: &'static str = "REACH_KEYS";
}

impl FromEnvironment for MessagingIngressServiceConfig {
    const PREFIX: &'static str = "REACH_MESSAGING_INGRESS";
}

pub fn load_from_env<T>(prefix: &str) -> Result<T, ConfigError>
where
    T: DeserializeOwned,
{
    let configuration = Config::builder()
        .add_source(
            Environment::with_prefix(prefix)
                .separator("__")
                .try_parsing(true),
        )
        .build()
        .map_err(ConfigError::Build)?;

    configuration.try_deserialize().map_err(ConfigError::Deserialize)
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to build configuration source: {0}")]
    Build(config::ConfigError),
    #[error("failed to deserialize configuration: {0}")]
    Deserialize(config::ConfigError),
}

