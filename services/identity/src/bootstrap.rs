use axum::Router;

pub fn build_router() -> Router {
    Router::new().nest("/health", crate::http::health_router())
}
