use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts, HeaderMap, StatusCode},
    Json,
};
use reach_auth_types::{InternalServicePrincipal, Principal, RequestContext};
use reach_config::InternalAuthConfig;
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use thiserror::Error;

const SERVICE_NAME_HEADER: &str = "x-reach-service";
const REQUEST_ID_HEADER: &str = "x-request-id";
const BEARER_PREFIX: &str = "Bearer ";

#[derive(Debug, Clone)]
pub struct InternalRequestAuthenticator {
    credentials: Vec<InternalServiceCredential>,
}

#[derive(Debug, Clone)]
struct InternalServiceCredential {
    service_name: String,
    token: SecretString,
    scopes: Vec<reach_auth_types::AuthScope>,
}

impl InternalRequestAuthenticator {
    pub fn from_config(config: &InternalAuthConfig) -> Result<Self, RequestAuthBootstrapError> {
        if config.service_tokens.is_empty() {
            return Err(RequestAuthBootstrapError::NoServiceTokensConfigured);
        }

        let mut credentials = Vec::with_capacity(config.service_tokens.len());
        for credential in &config.service_tokens {
            if credential.service_name.trim().is_empty() {
                return Err(RequestAuthBootstrapError::InvalidServiceName);
            }

            if credential.token.expose_secret().is_empty() {
                return Err(RequestAuthBootstrapError::EmptyServiceToken(
                    credential.service_name.clone(),
                ));
            }

            if credentials
                .iter()
                .any(|configured: &InternalServiceCredential| {
                    configured.service_name == credential.service_name
                })
            {
                return Err(RequestAuthBootstrapError::DuplicateServiceName(
                    credential.service_name.clone(),
                ));
            }

            credentials.push(InternalServiceCredential {
                service_name: credential.service_name.clone(),
                token: credential.token.clone(),
                scopes: credential.scopes.clone(),
            });
        }

        Ok(Self { credentials })
    }

    pub fn authenticate(
        &self,
        headers: &HeaderMap,
        request_id: Option<String>,
    ) -> Result<RequestContext, RequestAuthError> {
        let service_name = headers
            .get(SERVICE_NAME_HEADER)
            .ok_or(RequestAuthError::MissingServiceName)?
            .to_str()
            .map_err(|_| RequestAuthError::InvalidServiceNameHeader)?
            .trim();

        if service_name.is_empty() {
            return Err(RequestAuthError::InvalidServiceNameHeader);
        }

        let authorization = headers
            .get(AUTHORIZATION)
            .ok_or(RequestAuthError::MissingAuthorization)?
            .to_str()
            .map_err(|_| RequestAuthError::InvalidAuthorizationHeader)?;

        let token = authorization
            .strip_prefix(BEARER_PREFIX)
            .ok_or(RequestAuthError::InvalidAuthorizationScheme)?;

        let configured = self
            .credentials
            .iter()
            .find(|credential| credential.service_name == service_name)
            .ok_or(RequestAuthError::UnknownService)?;

        if !secrets_match(configured.token.expose_secret(), token) {
            return Err(RequestAuthError::InvalidCredential);
        }

        Ok(RequestContext {
            principal: Principal::InternalService(InternalServicePrincipal {
                service_name: configured.service_name.clone(),
                scopes: configured.scopes.clone(),
            }),
            request_id,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticatedRequestContext(pub RequestContext);

impl AuthenticatedRequestContext {
    pub fn into_inner(self) -> RequestContext {
        self.0
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedRequestContext
where
    Arc<InternalRequestAuthenticator>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = RequestAuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let authenticator = Arc::<InternalRequestAuthenticator>::from_ref(state);
        let request_id = parts
            .headers
            .get(REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);

        authenticator
            .authenticate(&parts.headers, request_id)
            .map(Self)
            .map_err(|_error| RequestAuthRejection)
    }
}

#[derive(Debug, Error)]
pub enum RequestAuthBootstrapError {
    #[error("internal auth service token list must not be empty")]
    NoServiceTokensConfigured,
    #[error("internal auth service name must not be empty")]
    InvalidServiceName,
    #[error("internal auth token for service `{0}` must not be empty")]
    EmptyServiceToken(String),
    #[error("internal auth service name `{0}` is duplicated")]
    DuplicateServiceName(String),
}

#[derive(Debug, Error)]
pub enum RequestAuthError {
    #[error("missing x-reach-service header")]
    MissingServiceName,
    #[error("invalid x-reach-service header")]
    InvalidServiceNameHeader,
    #[error("missing authorization header")]
    MissingAuthorization,
    #[error("invalid authorization header")]
    InvalidAuthorizationHeader,
    #[error("unsupported authorization scheme")]
    InvalidAuthorizationScheme,
    #[error("unknown internal service")]
    UnknownService,
    #[error("invalid internal service credential")]
    InvalidCredential,
}

pub struct RequestAuthRejection;

impl axum::response::IntoResponse for RequestAuthRejection {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::UNAUTHORIZED,
            Json(RequestAuthErrorResponse {
                code: "request_authentication_failed",
                message: "request authentication failed".to_owned(),
            }),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize)]
struct RequestAuthErrorResponse {
    code: &'static str,
    message: String,
}

fn secrets_match(expected: &str, presented: &str) -> bool {
    if expected.len() != presented.len() {
        return false;
    }

    expected.as_bytes().ct_eq(presented.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::{InternalRequestAuthenticator, RequestAuthError};
    use axum::http::{header::AUTHORIZATION, HeaderMap, HeaderValue};
    use reach_auth_types::AuthScope;
    use reach_config::{InternalAuthConfig, InternalServiceCredentialConfig};
    use secrecy::SecretString;

    fn authenticator() -> InternalRequestAuthenticator {
        InternalRequestAuthenticator::from_config(&InternalAuthConfig {
            service_tokens: vec![InternalServiceCredentialConfig {
                service_name: "reach-tests".to_owned(),
                token: SecretString::new("super-secret".to_owned().into_boxed_str()),
                scopes: vec![AuthScope::IdentityAccountCreate],
            }],
        })
        .expect("authenticator should build")
    }

    #[test]
    fn authenticate_rejects_missing_authorization_header() {
        let headers = HeaderMap::new();

        let error = authenticator()
            .authenticate(&headers, None)
            .expect_err("authentication should fail");

        assert!(matches!(error, RequestAuthError::MissingServiceName));
    }

    #[test]
    fn authenticate_accepts_valid_internal_service_credentials() {
        let mut headers = HeaderMap::new();
        headers.insert("x-reach-service", HeaderValue::from_static("reach-tests"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer super-secret"),
        );

        let context = authenticator()
            .authenticate(&headers, Some("req-1".to_owned()))
            .expect("authentication should succeed");

        assert_eq!(context.principal.service_name(), Some("reach-tests"));
        assert!(context.has_scope(AuthScope::IdentityAccountCreate));
        assert_eq!(context.request_id.as_deref(), Some("req-1"));
    }
}
