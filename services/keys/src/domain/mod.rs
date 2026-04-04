use chrono::{DateTime, Utc};
use reach_auth_types::DeviceId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OneTimePrekeyState {
    Available,
    Claimed,
}

impl OneTimePrekeyState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Claimed => "claimed",
        }
    }
}

impl TryFrom<&str> for OneTimePrekeyState {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "available" => Ok(Self::Available),
            "claimed" => Ok(Self::Claimed),
            invalid => Err(invalid.to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyBundle {
    pub bundle_id: Uuid,
    pub device_id: DeviceId,
    pub bundle_version: i64,
    pub identity_key_public: Vec<u8>,
    pub identity_key_alg: String,
    pub signed_prekey_id: Uuid,
    pub published_at: DateTime<Utc>,
    pub superseded_at: Option<DateTime<Utc>>,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedPrekey {
    pub signed_prekey_id: Uuid,
    pub device_id: DeviceId,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub superseded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneTimePrekey {
    pub prekey_id: Uuid,
    pub device_id: DeviceId,
    pub public_key: Vec<u8>,
    pub state: OneTimePrekeyState,
    pub created_at: DateTime<Utc>,
    pub claimed_at: Option<DateTime<Utc>>,
}
