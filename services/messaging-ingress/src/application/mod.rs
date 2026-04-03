use crate::domain::{DirectConversation, MessageIntakeRecord};
use async_trait::async_trait;
use reach_auth_types::{AccountId, ConversationId, DeviceId};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreateDirectConversation {
    pub participant_a_account_id: AccountId,
    pub participant_b_account_id: AccountId,
    pub default_expire_after_seconds: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct SubmitEncryptedMessage {
    pub conversation_id: ConversationId,
    pub sender_account_id: AccountId,
    pub sender_device_id: DeviceId,
    pub client_message_id: Uuid,
    pub ciphertext_size_bytes: i32,
    pub expires_at_unix: Option<i64>,
    pub idempotency_key: Vec<u8>,
}

#[async_trait]
pub trait MessagingIngressUseCases: Send + Sync {
    async fn create_direct_conversation(
        &self,
        command: CreateDirectConversation,
    ) -> Result<DirectConversation, crate::errors::MessagingIngressError>;
    async fn submit_encrypted_message(
        &self,
        command: SubmitEncryptedMessage,
    ) -> Result<MessageIntakeRecord, crate::errors::MessagingIngressError>;
}

