use chrono::{DateTime, Utc};
use reach_auth_types::{AccountId, DeviceId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrekeyResolutionMode {
    CurrentBundleOnly,
    CurrentBundleAndOneTimePrekey,
}

impl PrekeyResolutionMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CurrentBundleOnly => "current_bundle_only",
            Self::CurrentBundleAndOneTimePrekey => "current_bundle_and_one_time_prekey",
        }
    }
}

impl TryFrom<&str> for PrekeyResolutionMode {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "current_bundle_only" => Ok(Self::CurrentBundleOnly),
            "current_bundle_and_one_time_prekey" => Ok(Self::CurrentBundleAndOneTimePrekey),
            invalid => Err(invalid.to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedEnvelope {
    pub envelope_id: Uuid,
    pub sender_account_id: AccountId,
    pub sender_device_id: DeviceId,
    pub recipient_account_id: AccountId,
    pub recipient_device_id: DeviceId,
    pub encrypted_payload: Vec<u8>,
    pub content_type: String,
    pub client_timestamp: DateTime<Utc>,
    pub replay_nonce: Vec<u8>,
    pub payload_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedEncryptedEnvelope {
    #[serde(flatten)]
    pub envelope: EncryptedEnvelope,
    pub accepted_at: DateTime<Utc>,
    pub recipient_bundle_id: Uuid,
    pub recipient_signed_prekey_id: Uuid,
    pub claimed_one_time_prekey_id: Option<Uuid>,
    pub prekey_resolution_mode: PrekeyResolutionMode,
}
