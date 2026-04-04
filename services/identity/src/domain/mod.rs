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

impl AccountState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::PendingDeletion => "pending_deletion",
            Self::Suspended => "suspended",
            Self::Purged => "purged",
        }
    }
}

impl TryFrom<&str> for AccountState {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "active" => Ok(Self::Active),
            "pending_deletion" => Ok(Self::PendingDeletion),
            "suspended" => Ok(Self::Suspended),
            "purged" => Ok(Self::Purged),
            invalid => Err(invalid.to_owned()),
        }
    }
}

impl DeviceStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
        }
    }
}

impl TryFrom<&str> for DeviceStatus {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "active" => Ok(Self::Active),
            "revoked" => Ok(Self::Revoked),
            invalid => Err(invalid.to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub account_id: AccountId,
    pub state: AccountState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deletion_requested_at: Option<DateTime<Utc>>,
    pub purge_after: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub device_id: DeviceId,
    pub account_id: AccountId,
    pub device_number: i32,
    pub platform: String,
    pub app_version: String,
    pub status: DeviceStatus,
    pub registered_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}
