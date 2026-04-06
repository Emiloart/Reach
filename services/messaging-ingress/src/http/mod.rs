use crate::{
    application::{AcceptEncryptedEnvelopeInput, MessagingIngressUseCases},
    errors::MessagingIngressError,
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
    use_cases: Arc<dyn MessagingIngressUseCases>,
    authenticator: Arc<InternalRequestAuthenticator>,
) -> Router {
    Router::new()
        .route("/envelopes", post(accept_encrypted_envelope))
        .with_state(MessagingIngressHttpState {
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
struct MessagingIngressHttpState {
    use_cases: Arc<dyn MessagingIngressUseCases>,
    authenticator: Arc<InternalRequestAuthenticator>,
}

impl FromRef<MessagingIngressHttpState> for Arc<InternalRequestAuthenticator> {
    fn from_ref(input: &MessagingIngressHttpState) -> Self {
        input.authenticator.clone()
    }
}

async fn accept_encrypted_envelope(
    State(state): State<MessagingIngressHttpState>,
    AuthenticatedRequestContext(context): AuthenticatedRequestContext,
    Json(command): Json<AcceptEncryptedEnvelopeInput>,
) -> Result<Json<crate::domain::AcceptedEncryptedEnvelope>, MessagingIngressHttpError> {
    let envelope = state
        .use_cases
        .accept_encrypted_envelope(context, command)
        .await?;
    Ok(Json(envelope))
}

struct MessagingIngressHttpError(MessagingIngressError);

impl From<MessagingIngressError> for MessagingIngressHttpError {
    fn from(value: MessagingIngressError) -> Self {
        Self(value)
    }
}

impl axum::response::IntoResponse for MessagingIngressHttpError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.0 {
            MessagingIngressError::InsufficientScope => StatusCode::FORBIDDEN,
            MessagingIngressError::InvalidEnvelopeId
            | MessagingIngressError::InvalidSenderAccountId
            | MessagingIngressError::InvalidSenderDeviceId
            | MessagingIngressError::InvalidRecipientAccountId
            | MessagingIngressError::InvalidRecipientDeviceId
            | MessagingIngressError::PayloadTooLarge
            | MessagingIngressError::EmptyEncryptedPayload
            | MessagingIngressError::InvalidContentType
            | MessagingIngressError::InvalidPayloadVersion
            | MessagingIngressError::InvalidReplayNonce
            | MessagingIngressError::ClientTimestampTooOld
            | MessagingIngressError::ClientTimestampTooFarInFuture => StatusCode::BAD_REQUEST,
            MessagingIngressError::SenderAccountNotFound
            | MessagingIngressError::SenderDeviceNotFound
            | MessagingIngressError::RecipientAccountNotFound
            | MessagingIngressError::RecipientDeviceNotFound => StatusCode::NOT_FOUND,
            MessagingIngressError::SenderAccountNotActive
            | MessagingIngressError::SenderDeviceNotActive
            | MessagingIngressError::SenderDeviceAccountMismatch
            | MessagingIngressError::RecipientAccountNotActive
            | MessagingIngressError::RecipientDeviceNotActive
            | MessagingIngressError::RecipientDeviceAccountMismatch
            | MessagingIngressError::RecipientBundleUnavailable
            | MessagingIngressError::RecipientOneTimePrekeyUnavailable
            | MessagingIngressError::EnvelopeAlreadyExists
            | MessagingIngressError::ReplayNonceConflict => StatusCode::CONFLICT,
            MessagingIngressError::Lifecycle(_) | MessagingIngressError::Storage(_) => {
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

fn error_code(error: &MessagingIngressError) -> &'static str {
    match error {
        MessagingIngressError::InsufficientScope => "insufficient_scope",
        MessagingIngressError::InvalidEnvelopeId => "invalid_envelope_id",
        MessagingIngressError::InvalidSenderAccountId => "invalid_sender_account_id",
        MessagingIngressError::InvalidSenderDeviceId => "invalid_sender_device_id",
        MessagingIngressError::InvalidRecipientAccountId => "invalid_recipient_account_id",
        MessagingIngressError::InvalidRecipientDeviceId => "invalid_recipient_device_id",
        MessagingIngressError::PayloadTooLarge => "payload_too_large",
        MessagingIngressError::EmptyEncryptedPayload => "empty_encrypted_payload",
        MessagingIngressError::InvalidContentType => "invalid_content_type",
        MessagingIngressError::InvalidPayloadVersion => "invalid_payload_version",
        MessagingIngressError::InvalidReplayNonce => "invalid_replay_nonce",
        MessagingIngressError::ClientTimestampTooOld => "client_timestamp_too_old",
        MessagingIngressError::ClientTimestampTooFarInFuture => {
            "client_timestamp_too_far_in_future"
        }
        MessagingIngressError::SenderAccountNotFound => "sender_account_not_found",
        MessagingIngressError::SenderAccountNotActive => "sender_account_not_active",
        MessagingIngressError::SenderDeviceNotFound => "sender_device_not_found",
        MessagingIngressError::SenderDeviceNotActive => "sender_device_not_active",
        MessagingIngressError::SenderDeviceAccountMismatch => "sender_device_account_mismatch",
        MessagingIngressError::RecipientAccountNotFound => "recipient_account_not_found",
        MessagingIngressError::RecipientAccountNotActive => "recipient_account_not_active",
        MessagingIngressError::RecipientDeviceNotFound => "recipient_device_not_found",
        MessagingIngressError::RecipientDeviceNotActive => "recipient_device_not_active",
        MessagingIngressError::RecipientDeviceAccountMismatch => {
            "recipient_device_account_mismatch"
        }
        MessagingIngressError::RecipientBundleUnavailable => "recipient_bundle_unavailable",
        MessagingIngressError::RecipientOneTimePrekeyUnavailable => {
            "recipient_one_time_prekey_unavailable"
        }
        MessagingIngressError::EnvelopeAlreadyExists => "envelope_already_exists",
        MessagingIngressError::ReplayNonceConflict => "replay_nonce_conflict",
        MessagingIngressError::Lifecycle(_) => "lifecycle_failure",
        MessagingIngressError::Storage(_) => "storage_failure",
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
