use thiserror::Error;

#[derive(Debug, Error)]
pub enum MessagingIngressError {
    #[error("conversation not found")]
    ConversationNotFound,
    #[error("message idempotency conflict")]
    IdempotencyConflict,
    #[error("ciphertext size exceeds service limits")]
    PayloadTooLarge,
}

