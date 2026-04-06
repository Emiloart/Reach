use crate::application::MessagingIngressUseCases;
use axum::Router;
use reach_request_auth::InternalRequestAuthenticator;
use std::sync::Arc;

pub fn build_router(
    use_cases: Arc<dyn MessagingIngressUseCases>,
    authenticator: Arc<InternalRequestAuthenticator>,
) -> Router {
    Router::new()
        .nest("/health", crate::http::health_router())
        .nest(
            "/v1/messaging-ingress",
            crate::http::command_router(use_cases, authenticator),
        )
}
