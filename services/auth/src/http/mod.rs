use crate::{
    application::{AuthUseCases, CreateSessionInput, RevokeSessionInput, RotateRefreshFamilyInput},
    errors::AuthError,
};
use axum::{
    extract::{FromRef, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use reach_request_auth::{AuthenticatedRequestContext, InternalRequestAuthenticator};
use serde::Serialize;
use std::sync::Arc;

pub fn health_router() -> Router {
    Router::new()
        .route("/live", get(live))
        .route("/ready", get(ready))
}

pub fn command_router(
    use_cases: Arc<dyn AuthUseCases>,
    authenticator: Arc<InternalRequestAuthenticator>,
) -> Router {
    Router::new()
        .route("/sessions", post(create_session))
        .route("/sessions/revoke", post(revoke_session))
        .route("/refresh-families/rotate", post(rotate_refresh_family))
        .with_state(AuthHttpState {
            use_cases,
            authenticator,
        })
}

async fn live() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn ready() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ready" })
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Clone)]
struct AuthHttpState {
    use_cases: Arc<dyn AuthUseCases>,
    authenticator: Arc<InternalRequestAuthenticator>,
}

impl FromRef<AuthHttpState> for Arc<InternalRequestAuthenticator> {
    fn from_ref(input: &AuthHttpState) -> Self {
        input.authenticator.clone()
    }
}

async fn create_session(
    State(state): State<AuthHttpState>,
    AuthenticatedRequestContext(context): AuthenticatedRequestContext,
    Json(command): Json<CreateSessionInput>,
) -> Result<Json<crate::application::CreatedSession>, AuthHttpError> {
    let created_session = state.use_cases.create_session(context, command).await?;
    Ok(Json(created_session))
}

async fn revoke_session(
    State(state): State<AuthHttpState>,
    AuthenticatedRequestContext(context): AuthenticatedRequestContext,
    Json(command): Json<RevokeSessionInput>,
) -> Result<Json<crate::domain::Session>, AuthHttpError> {
    let session = state.use_cases.revoke_session(context, command).await?;
    Ok(Json(session))
}

async fn rotate_refresh_family(
    State(state): State<AuthHttpState>,
    AuthenticatedRequestContext(context): AuthenticatedRequestContext,
    Json(command): Json<RotateRefreshFamilyInput>,
) -> Result<Json<crate::domain::RefreshTokenFamily>, AuthHttpError> {
    let family = state
        .use_cases
        .rotate_refresh_family(context, command)
        .await?;
    Ok(Json(family))
}

struct AuthHttpError(AuthError);

impl From<AuthError> for AuthHttpError {
    fn from(value: AuthError) -> Self {
        Self(value)
    }
}

impl axum::response::IntoResponse for AuthHttpError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.0 {
            AuthError::InsufficientScope => StatusCode::FORBIDDEN,
            AuthError::InvalidSessionId
            | AuthError::InvalidAccountId
            | AuthError::InvalidDeviceId
            | AuthError::InvalidAccessTokenId
            | AuthError::InvalidRefreshFamilyId
            | AuthError::InvalidAccessExpiry
            | AuthError::InvalidRefreshTokenHash
            | AuthError::InvalidRefreshExpiry => StatusCode::BAD_REQUEST,
            AuthError::AccountNotFound
            | AuthError::DeviceNotFound
            | AuthError::SessionNotFound
            | AuthError::RefreshTokenFamilyNotFound => StatusCode::NOT_FOUND,
            AuthError::AccountNotActive
            | AuthError::DeviceNotActive
            | AuthError::DeviceAccountMismatch
            | AuthError::SessionRevoked
            | AuthError::SessionExpired
            | AuthError::SessionAlreadyExists
            | AuthError::RefreshTokenFamilyAlreadyExists
            | AuthError::RefreshTokenCompromised
            | AuthError::RefreshTokenMismatch => StatusCode::CONFLICT,
            AuthError::Lifecycle(_) | AuthError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (
            status,
            Json(ErrorResponse {
                code: error_code(&self.0),
                message: self.0.to_string(),
            }),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    code: &'static str,
    message: String,
}

fn error_code(error: &AuthError) -> &'static str {
    match error {
        AuthError::InsufficientScope => "insufficient_scope",
        AuthError::InvalidSessionId => "invalid_session_id",
        AuthError::InvalidAccountId => "invalid_account_id",
        AuthError::InvalidDeviceId => "invalid_device_id",
        AuthError::InvalidAccessTokenId => "invalid_access_token_id",
        AuthError::InvalidRefreshFamilyId => "invalid_refresh_family_id",
        AuthError::InvalidAccessExpiry => "invalid_access_expiry",
        AuthError::InvalidRefreshTokenHash => "invalid_refresh_token_hash",
        AuthError::InvalidRefreshExpiry => "invalid_refresh_expiry",
        AuthError::AccountNotFound => "account_not_found",
        AuthError::AccountNotActive => "account_not_active",
        AuthError::DeviceNotFound => "device_not_found",
        AuthError::DeviceNotActive => "device_not_active",
        AuthError::DeviceAccountMismatch => "device_account_mismatch",
        AuthError::SessionNotFound => "session_not_found",
        AuthError::SessionRevoked => "session_revoked",
        AuthError::SessionExpired => "session_expired",
        AuthError::SessionAlreadyExists => "session_already_exists",
        AuthError::RefreshTokenFamilyNotFound => "refresh_token_family_not_found",
        AuthError::RefreshTokenFamilyAlreadyExists => "refresh_token_family_already_exists",
        AuthError::RefreshTokenCompromised => "refresh_token_compromised",
        AuthError::RefreshTokenMismatch => "refresh_token_mismatch",
        AuthError::Lifecycle(_) => "lifecycle_failure",
        AuthError::Storage(_) => "storage_failure",
    }
}

#[cfg(test)]
mod tests {
    use super::health_router;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn live_endpoint_returns_ok() {
        let response = health_router()
            .oneshot(Request::builder().uri("/live").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn ready_endpoint_returns_ok() {
        let response = health_router()
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
