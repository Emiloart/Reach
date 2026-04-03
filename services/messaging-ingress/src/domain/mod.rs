use chrono::{DateTime, Utc};
use reach_auth_types::{AccountId, ConversationId, DeviceId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectConversationState {
    Active,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectConversation {
    pub conversation_id: ConversationId,
    pub participant_a_account_id: AccountId,
    pub participant_b_account_id: AccountId,
    pub state: DirectConversationState,
    pub default_expire_after_seconds: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageIntakeRecord {
    pub message_id: Uuid,
    pub conversation_id: ConversationId,
    pub sender_account_id: AccountId,
    pub sender_device_id: DeviceId,
    pub client_message_id: Uuid,
    pub ciphertext_size_bytes: i32,
    pub server_received_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

