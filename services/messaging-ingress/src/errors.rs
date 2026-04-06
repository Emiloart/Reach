use crate::repository::{
    MessagingIngressConstraintViolation, MessagingIngressRepositoryError,
    RecipientKeyMaterialFailure,
};
use reach_identity_lifecycle::IdentityLifecycleError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MessagingIngressError {
    #[error("insufficient scope")]
    InsufficientScope,
    #[error("invalid envelope id")]
    InvalidEnvelopeId,
    #[error("invalid sender account id")]
    InvalidSenderAccountId,
    #[error("invalid sender device id")]
    InvalidSenderDeviceId,
    #[error("invalid recipient account id")]
    InvalidRecipientAccountId,
    #[error("invalid recipient device id")]
    InvalidRecipientDeviceId,
    #[error("encrypted payload exceeds service limits")]
    PayloadTooLarge,
    #[error("encrypted payload must not be empty")]
    EmptyEncryptedPayload,
    #[error("invalid content type")]
    InvalidContentType,
    #[error("invalid payload version")]
    InvalidPayloadVersion,
    #[error("invalid replay nonce")]
    InvalidReplayNonce,
    #[error("client timestamp is too old")]
    ClientTimestampTooOld,
    #[error("client timestamp is too far in the future")]
    ClientTimestampTooFarInFuture,
    #[error("sender account not found")]
    SenderAccountNotFound,
    #[error("sender account not active")]
    SenderAccountNotActive,
    #[error("sender device not found")]
    SenderDeviceNotFound,
    #[error("sender device not active")]
    SenderDeviceNotActive,
    #[error("sender device account mismatch")]
    SenderDeviceAccountMismatch,
    #[error("recipient account not found")]
    RecipientAccountNotFound,
    #[error("recipient account not active")]
    RecipientAccountNotActive,
    #[error("recipient device not found")]
    RecipientDeviceNotFound,
    #[error("recipient device not active")]
    RecipientDeviceNotActive,
    #[error("recipient device account mismatch")]
    RecipientDeviceAccountMismatch,
    #[error("recipient current bundle unavailable")]
    RecipientBundleUnavailable,
    #[error("recipient one-time prekey unavailable")]
    RecipientOneTimePrekeyUnavailable,
    #[error("envelope already exists")]
    EnvelopeAlreadyExists,
    #[error("replay nonce conflict")]
    ReplayNonceConflict,
    #[error("lifecycle resolution failed: {0}")]
    Lifecycle(#[source] IdentityLifecycleError),
    #[error("storage operation failed: {0}")]
    Storage(#[source] MessagingIngressRepositoryError),
}

pub fn map_repository_error(error: MessagingIngressRepositoryError) -> MessagingIngressError {
    match error {
        MessagingIngressRepositoryError::Constraint(
            MessagingIngressConstraintViolation::EnvelopeAlreadyExists,
        ) => MessagingIngressError::EnvelopeAlreadyExists,
        MessagingIngressRepositoryError::Constraint(
            MessagingIngressConstraintViolation::ReplayNonceConflict,
        ) => MessagingIngressError::ReplayNonceConflict,
        MessagingIngressRepositoryError::RecipientKeyMaterialUnavailable(
            RecipientKeyMaterialFailure::CurrentBundleMissing,
        ) => MessagingIngressError::RecipientBundleUnavailable,
        MessagingIngressRepositoryError::RecipientKeyMaterialUnavailable(
            RecipientKeyMaterialFailure::OneTimePrekeyUnavailable,
        ) => MessagingIngressError::RecipientOneTimePrekeyUnavailable,
        other => MessagingIngressError::Storage(other),
    }
}
