use axum::{
    routing::post,
    Router,
};
use crate::handlers::create_item;

/// Create API routes
pub fn create_api_routes() -> Router {
    Router::new()
        .route("/items", post(create_item))
}