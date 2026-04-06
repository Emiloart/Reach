use crate::application::KeyUseCases;
use axum::Router;
use reach_request_auth::InternalRequestAuthenticator;
use std::sync::Arc;

pub fn build_router(
    use_cases: Arc<dyn KeyUseCases>,
    authenticator: Arc<InternalRequestAuthenticator>,
) -> Router {
    Router::new()
        .nest("/health", crate::http::health_router())
        .nest(
            "/v1/keys",
            crate::http::command_router(use_cases, authenticator),
        )
}
