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
    IdentityAccountCreate,
    IdentityDeviceRegister,
    IdentityDeviceRevoke,
    AuthSessionCreate,
    AuthSessionRevoke,
    AuthSessionRotate,
    KeysSignedPrekeyPublish,
    KeysBundlePublish,
    KeysOneTimePrekeysPublish,
    KeysOneTimePrekeyClaim,
    KeysBundleRead,
    MessagingIngressEnvelopeAccept,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalServicePrincipal {
    pub service_name: String,
    pub scopes: Vec<AuthScope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserDevicePrincipal {
    pub account_id: AccountId,
    pub device_id: DeviceId,
    pub session_id: SessionId,
    pub scopes: Vec<AuthScope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "principal_type", rename_all = "snake_case")]
pub enum Principal {
    InternalService(InternalServicePrincipal),
    UserDevice(UserDevicePrincipal),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestContext {
    pub principal: Principal,
    pub request_id: Option<String>,
}

impl Principal {
    pub fn scopes(&self) -> &[AuthScope] {
        match self {
            Self::InternalService(principal) => &principal.scopes,
            Self::UserDevice(principal) => &principal.scopes,
        }
    }

    pub fn account_id(&self) -> Option<AccountId> {
        match self {
            Self::InternalService(_) => None,
            Self::UserDevice(principal) => Some(principal.account_id),
        }
    }

    pub fn device_id(&self) -> Option<DeviceId> {
        match self {
            Self::InternalService(_) => None,
            Self::UserDevice(principal) => Some(principal.device_id),
        }
    }

    pub fn session_id(&self) -> Option<SessionId> {
        match self {
            Self::InternalService(_) => None,
            Self::UserDevice(principal) => Some(principal.session_id),
        }
    }

    pub fn service_name(&self) -> Option<&str> {
        match self {
            Self::InternalService(principal) => Some(principal.service_name.as_str()),
            Self::UserDevice(_) => None,
        }
    }
}

impl RequestContext {
    pub fn has_scope(&self, scope: AuthScope) -> bool {
        self.principal.scopes().contains(&scope)
    }
}
