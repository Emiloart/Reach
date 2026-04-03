use crate::domain::{DirectConversation, MessageIntakeRecord};
use async_trait::async_trait;
use reach_auth_types::ConversationId;

#[async_trait]
pub trait DirectConversationRepository: Send + Sync {
    async fn get_by_id(
        &self,
        conversation_id: ConversationId,
    ) -> Result<Option<DirectConversation>, crate::errors::MessagingIngressError>;
    async fn create(
        &self,
        conversation: &DirectConversation,
    ) -> Result<(), crate::errors::MessagingIngressError>;
}

#[async_trait]
pub trait MessageIntakeRepository: Send + Sync {
    async fn create(
        &self,
        record: &MessageIntakeRecord,
        idempotency_key: &[u8],
    ) -> Result<(), crate::errors::MessagingIngressError>;
    async fn exists_by_idempotency_key(
        &self,
        idempotency_key: &[u8],
    ) -> Result<bool, crate::errors::MessagingIngressError>;
}

