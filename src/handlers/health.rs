use axum::Json;
use crate::models::HealthResponse;
use tracing::debug;

/// Health check endpoint
pub async fn health_check() -> Json<HealthResponse> {
    debug!("Health check requested");
    Json(HealthResponse {
        status: "ok".to_string(),
        message: "Server is running".to_string(),
    })
}