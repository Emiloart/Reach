use crate::application::IdentityUseCases;
use axum::Router;
use reach_request_auth::InternalRequestAuthenticator;
use std::sync::Arc;

pub fn build_router(
    use_cases: Arc<dyn IdentityUseCases>,
    authenticator: Arc<InternalRequestAuthenticator>,
) -> Router {
    Router::new()
        .nest("/health", crate::http::health_router())
        .nest(
            "/v1/identity",
            crate::http::command_router(use_cases, authenticator),
        )
}
