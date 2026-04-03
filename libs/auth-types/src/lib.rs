use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AccountId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConversationId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthScope {
    SessionBootstrap,
    SessionRefresh,
    DeviceManage,
    KeysWrite,
    KeysRead,
    MessagingSend,
    MessagingReadMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub issuer: String,
    pub audience: String,
    pub account_id: AccountId,
    pub device_id: DeviceId,
    pub session_id: SessionId,
    pub scopes: Vec<AuthScope>,
    pub issued_at_unix: i64,
    pub expires_at_unix: i64,
    pub token_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedActor {
    pub account_id: AccountId,
    pub device_id: DeviceId,
    pub session_id: SessionId,
    pub scopes: Vec<AuthScope>,
}
