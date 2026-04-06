use crate::{
    application::{CreateAccountInput, IdentityUseCases, RegisterDeviceInput, RevokeDeviceInput},
    errors::IdentityError,
};
use axum::{
    extract::{FromRef, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use reach_request_auth::{AuthenticatedRequestContext, InternalRequestAuthenticator};
use serde::{Serialize, Serializer};
use std::sync::Arc;

pub fn health_router() -> Router {
    Router::new()
        .route("/live", get(live))
        .route("/ready", get(ready))
}

pub fn command_router(
    use_cases: Arc<dyn IdentityUseCases>,
    authenticator: Arc<InternalRequestAuthenticator>,
) -> Router {
    Router::new()
        .route("/accounts", post(create_account))
        .route("/devices", post(register_device))
        .route("/devices/revoke", post(revoke_device))
        .with_state(IdentityHttpState {
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
struct IdentityHttpState {
    use_cases: Arc<dyn IdentityUseCases>,
    authenticator: Arc<InternalRequestAuthenticator>,
}

impl FromRef<IdentityHttpState> for Arc<InternalRequestAuthenticator> {
    fn from_ref(input: &IdentityHttpState) -> Self {
        input.authenticator.clone()
    }
}

async fn create_account(
    State(state): State<IdentityHttpState>,
    AuthenticatedRequestContext(context): AuthenticatedRequestContext,
    Json(command): Json<CreateAccountInput>,
) -> Result<Json<crate::domain::Account>, IdentityHttpError> {
    let account = state.use_cases.create_account(context, command).await?;
    Ok(Json(account))
}

async fn register_device(
    State(state): State<IdentityHttpState>,
    AuthenticatedRequestContext(context): AuthenticatedRequestContext,
    Json(command): Json<RegisterDeviceInput>,
) -> Result<Json<crate::domain::Device>, IdentityHttpError> {
    let device = state.use_cases.register_device(context, command).await?;
    Ok(Json(device))
}

async fn revoke_device(
    State(state): State<IdentityHttpState>,
    AuthenticatedRequestContext(context): AuthenticatedRequestContext,
    Json(command): Json<RevokeDeviceInput>,
) -> Result<Json<crate::domain::Device>, IdentityHttpError> {
    let device = state.use_cases.revoke_device(context, command).await?;
    Ok(Json(device))
}

struct IdentityHttpError(IdentityError);

impl From<IdentityError> for IdentityHttpError {
    fn from(value: IdentityError) -> Self {
        Self(value)
    }
}

impl axum::response::IntoResponse for IdentityHttpError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.0 {
            IdentityError::InsufficientScope => StatusCode::FORBIDDEN,
            IdentityError::InvalidAccountId
            | IdentityError::InvalidDeviceId
            | IdentityError::InvalidDeviceNumber
            | IdentityError::InvalidPlatform
            | IdentityError::InvalidAppVersion => StatusCode::BAD_REQUEST,
            IdentityError::AccountNotFound | IdentityError::DeviceNotFound => StatusCode::NOT_FOUND,
            IdentityError::AccountAlreadyExists
            | IdentityError::AccountNotActive
            | IdentityError::DeviceAlreadyExists
            | IdentityError::DeviceRegistrationConflict
            | IdentityError::DeviceAlreadyRevoked => StatusCode::CONFLICT,
            IdentityError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
    #[serde(serialize_with = "serialize_display")]
    message: String,
}

fn error_code(error: &IdentityError) -> &'static str {
    match error {
        IdentityError::InsufficientScope => "insufficient_scope",
        IdentityError::InvalidAccountId => "invalid_account_id",
        IdentityError::InvalidDeviceId => "invalid_device_id",
        IdentityError::InvalidDeviceNumber => "invalid_device_number",
        IdentityError::InvalidPlatform => "invalid_platform",
        IdentityError::InvalidAppVersion => "invalid_app_version",
        IdentityError::AccountNotFound => "account_not_found",
        IdentityError::AccountNotActive => "account_not_active",
        IdentityError::AccountAlreadyExists => "account_already_exists",
        IdentityError::DeviceNotFound => "device_not_found",
        IdentityError::DeviceAlreadyExists => "device_already_exists",
        IdentityError::DeviceRegistrationConflict => "device_registration_conflict",
        IdentityError::DeviceAlreadyRevoked => "device_already_revoked",
        IdentityError::Storage(_) => "storage_failure",
    }
}

fn serialize_display<S>(value: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(value)
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
