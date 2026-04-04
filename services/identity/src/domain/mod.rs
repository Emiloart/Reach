use chrono::{DateTime, Utc};
use reach_auth_types::{AccountId, DeviceId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountState {
    Active,
    PendingDeletion,
    Suspended,
    Purged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceStatus {
    Active,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub account_id: AccountId,
    pub state: AccountState,
    pub created_at: DateTime<Utc>,
    pub deletion_requested_at: Option<DateTime<Utc>>,
    pub purge_after: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub device_id: DeviceId,
    pub account_id: AccountId,
    pub device_number: i32,
    pub platform: String,
    pub status: DeviceStatus,
    pub registered_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}
