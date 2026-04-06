use crate::application::AuthUseCases;
use axum::Router;
use reach_request_auth::InternalRequestAuthenticator;
use std::sync::Arc;

pub fn build_router(
    use_cases: Arc<dyn AuthUseCases>,
    authenticator: Arc<InternalRequestAuthenticator>,
) -> Router {
    Router::new()
        .nest("/health", crate::http::health_router())
        .nest(
            "/v1/auth",
            crate::http::command_router(use_cases, authenticator),
        )
}
