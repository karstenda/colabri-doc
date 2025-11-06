use axum::{
    routing::{get, post},
    Router,
};
use crate::handlers::{health_check, create_item};

/// Create API routes
pub fn create_api_routes() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/items", post(create_item))
}