mod cockroach;

use crate::domain::{AcceptedEncryptedEnvelope, EncryptedEnvelope, PrekeyResolutionMode};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

pub use cockroach::CockroachMessagingIngressRepository;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessagingIngressConstraintViolation {
    EnvelopeAlreadyExists,
    ReplayNonceConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipientKeyMaterialFailure {
    CurrentBundleMissing,
    OneTimePrekeyUnavailable,
}

#[derive(Debug, Error)]
pub enum MessagingIngressRepositoryError {
    #[error("messaging-ingress constraint violation: {0:?}")]
    Constraint(MessagingIngressConstraintViolation),
    #[error("recipient key material unavailable: {0:?}")]
    RecipientKeyMaterialUnavailable(RecipientKeyMaterialFailure),
    #[error("invalid stored prekey resolution mode: {0}")]
    InvalidStoredPrekeyResolutionMode(String),
    #[error("key material contract failed: {0}")]
    KeyMaterial(#[source] reach_key_material::KeyMaterialError),
    #[error("database operation failed: {0}")]
    Database(#[source] sqlx::Error),
}

#[derive(Debug, Clone)]
pub struct AcceptEnvelopeRecord {
    pub envelope: EncryptedEnvelope,
    pub accepted_at: DateTime<Utc>,
    pub replay_reserved_at: DateTime<Utc>,
    pub prekey_resolution_mode: PrekeyResolutionMode,
}

#[async_trait]
pub trait AcceptedEnvelopeRepository: Send + Sync {
    async fn get_by_id(
        &self,
        envelope_id: Uuid,
    ) -> Result<Option<AcceptedEncryptedEnvelope>, MessagingIngressRepositoryError>;
}

#[async_trait]
pub trait EnvelopeCommandRepository: Send + Sync {
    async fn accept_envelope(
        &self,
        command: &AcceptEnvelopeRecord,
    ) -> Result<AcceptedEncryptedEnvelope, MessagingIngressRepositoryError>;
}
