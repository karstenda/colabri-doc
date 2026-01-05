use crate::handlers::create_item;
use axum::{routing::post, Router};

/// Create API routes
pub fn create_api_routes() -> Router {
    Router::new().route("/items", post(create_item))
}
