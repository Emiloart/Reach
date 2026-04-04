use chrono::{DateTime, Utc};
use reach_auth_types::{AccountId, DeviceId, SessionId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Active,
    Revoked,
    Expired,
}

impl SessionState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
        }
    }
}

impl TryFrom<&str> for SessionState {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "active" => Ok(Self::Active),
            "revoked" => Ok(Self::Revoked),
            "expired" => Ok(Self::Expired),
            invalid => Err(invalid.to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub session_id: SessionId,
    pub account_id: AccountId,
    pub device_id: DeviceId,
    pub state: SessionState,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub last_refreshed_at: Option<DateTime<Utc>>,
    pub access_token_jti: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshTokenFamily {
    pub family_id: Uuid,
    pub session_id: SessionId,
    pub current_token_hash: Vec<u8>,
    pub previous_token_hash: Option<Vec<u8>>,
    pub rotation_counter: i64,
    pub compromised_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}
