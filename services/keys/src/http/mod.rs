use crate::{
    application::{
        ClaimOneTimePrekeyInput, FetchCurrentBundleInput, KeyUseCases, PublishKeyBundleInput,
        PublishOneTimePrekeysInput, PublishSignedPrekeyInput,
    },
    errors::KeyServiceError,
};
use axum::{
    extract::{FromRef, Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use reach_auth_types::DeviceId;
use reach_request_auth::{AuthenticatedRequestContext, InternalRequestAuthenticator};
use serde::Serialize;
use std::sync::Arc;

pub fn health_router() -> Router {
    Router::new()
        .route("/live", get(live))
        .route("/ready", get(ready))
}

pub fn command_router(
    use_cases: Arc<dyn KeyUseCases>,
    authenticator: Arc<InternalRequestAuthenticator>,
) -> Router {
    Router::new()
        .route("/signed-prekeys/current", post(publish_signed_prekey))
        .route("/bundles/current", post(publish_key_bundle))
        .route("/bundles/current/:device_id", get(fetch_current_bundle))
        .route("/one-time-prekeys", post(publish_one_time_prekeys))
        .route("/one-time-prekeys/claim", post(claim_one_time_prekey))
        .with_state(KeyHttpState {
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
struct KeyHttpState {
    use_cases: Arc<dyn KeyUseCases>,
    authenticator: Arc<InternalRequestAuthenticator>,
}

impl FromRef<KeyHttpState> for Arc<InternalRequestAuthenticator> {
    fn from_ref(input: &KeyHttpState) -> Self {
        input.authenticator.clone()
    }
}

async fn publish_signed_prekey(
    State(state): State<KeyHttpState>,
    AuthenticatedRequestContext(context): AuthenticatedRequestContext,
    Json(command): Json<PublishSignedPrekeyInput>,
) -> Result<Json<crate::domain::SignedPrekey>, KeyHttpError> {
    let signed_prekey = state
        .use_cases
        .publish_signed_prekey(context, command)
        .await?;
    Ok(Json(signed_prekey))
}

async fn publish_key_bundle(
    State(state): State<KeyHttpState>,
    AuthenticatedRequestContext(context): AuthenticatedRequestContext,
    Json(command): Json<PublishKeyBundleInput>,
) -> Result<Json<crate::domain::KeyBundle>, KeyHttpError> {
    let bundle = state.use_cases.publish_key_bundle(context, command).await?;
    Ok(Json(bundle))
}

async fn publish_one_time_prekeys(
    State(state): State<KeyHttpState>,
    AuthenticatedRequestContext(context): AuthenticatedRequestContext,
    Json(command): Json<PublishOneTimePrekeysInput>,
) -> Result<Json<Vec<crate::domain::OneTimePrekey>>, KeyHttpError> {
    let prekeys = state
        .use_cases
        .publish_one_time_prekeys(context, command)
        .await?;
    Ok(Json(prekeys))
}

async fn claim_one_time_prekey(
    State(state): State<KeyHttpState>,
    AuthenticatedRequestContext(context): AuthenticatedRequestContext,
    Json(command): Json<ClaimOneTimePrekeyInput>,
) -> Result<Json<crate::domain::OneTimePrekey>, KeyHttpError> {
    let prekey = state
        .use_cases
        .claim_one_time_prekey(context, command)
        .await?;
    Ok(Json(prekey))
}

async fn fetch_current_bundle(
    State(state): State<KeyHttpState>,
    AuthenticatedRequestContext(context): AuthenticatedRequestContext,
    Path(device_id): Path<DeviceId>,
) -> Result<Json<crate::domain::KeyBundle>, KeyHttpError> {
    let bundle = state
        .use_cases
        .fetch_current_bundle(context, FetchCurrentBundleInput { device_id })
        .await?;
    Ok(Json(bundle))
}

struct KeyHttpError(KeyServiceError);

impl From<KeyServiceError> for KeyHttpError {
    fn from(value: KeyServiceError) -> Self {
        Self(value)
    }
}

impl axum::response::IntoResponse for KeyHttpError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.0 {
            KeyServiceError::InsufficientScope => StatusCode::FORBIDDEN,
            KeyServiceError::InvalidDeviceId
            | KeyServiceError::InvalidSignedPrekeyId
            | KeyServiceError::InvalidSignedPrekeyMaterial
            | KeyServiceError::InvalidSignedPrekeySignature
            | KeyServiceError::InvalidIdentityKeyMaterial
            | KeyServiceError::InvalidIdentityKeyAlgorithm
            | KeyServiceError::InvalidOneTimePrekeyBatch
            | KeyServiceError::InvalidOneTimePrekeyMaterial => StatusCode::BAD_REQUEST,
            KeyServiceError::AccountNotFound
            | KeyServiceError::DeviceNotFound
            | KeyServiceError::KeyBundleNotFound
            | KeyServiceError::SignedPrekeyNotFound => StatusCode::NOT_FOUND,
            KeyServiceError::AccountNotActive
            | KeyServiceError::DeviceNotActive
            | KeyServiceError::SignedPrekeyAlreadyExists
            | KeyServiceError::SignedPrekeyDeviceMismatch
            | KeyServiceError::KeyBundleAlreadyExists
            | KeyServiceError::NoAvailableOneTimePrekeys => StatusCode::CONFLICT,
            KeyServiceError::Lifecycle(_) | KeyServiceError::Storage(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
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

fn error_code(error: &KeyServiceError) -> &'static str {
    match error {
        KeyServiceError::InsufficientScope => "insufficient_scope",
        KeyServiceError::InvalidDeviceId => "invalid_device_id",
        KeyServiceError::InvalidSignedPrekeyId => "invalid_signed_prekey_id",
        KeyServiceError::InvalidSignedPrekeyMaterial => "invalid_signed_prekey_material",
        KeyServiceError::InvalidSignedPrekeySignature => "invalid_signed_prekey_signature",
        KeyServiceError::InvalidIdentityKeyMaterial => "invalid_identity_key_material",
        KeyServiceError::InvalidIdentityKeyAlgorithm => "invalid_identity_key_algorithm",
        KeyServiceError::InvalidOneTimePrekeyBatch => "invalid_one_time_prekey_batch",
        KeyServiceError::InvalidOneTimePrekeyMaterial => "invalid_one_time_prekey_material",
        KeyServiceError::AccountNotFound => "account_not_found",
        KeyServiceError::AccountNotActive => "account_not_active",
        KeyServiceError::DeviceNotFound => "device_not_found",
        KeyServiceError::DeviceNotActive => "device_not_active",
        KeyServiceError::KeyBundleNotFound => "key_bundle_not_found",
        KeyServiceError::SignedPrekeyNotFound => "signed_prekey_not_found",
        KeyServiceError::SignedPrekeyAlreadyExists => "signed_prekey_already_exists",
        KeyServiceError::SignedPrekeyDeviceMismatch => "signed_prekey_device_mismatch",
        KeyServiceError::KeyBundleAlreadyExists => "key_bundle_already_exists",
        KeyServiceError::NoAvailableOneTimePrekeys => "no_available_one_time_prekeys",
        KeyServiceError::Lifecycle(_) => "lifecycle_failure",
        KeyServiceError::Storage(_) => "storage_failure",
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
