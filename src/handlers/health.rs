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

/// Readiness check endpoint
pub async fn ready_check() -> Json<HealthResponse> {
    debug!("Readiness check requested");
    // In a real application, you might check database connectivity,
    // cache availability, or other dependencies here.
    Json(HealthResponse {
        status: "ok".to_string(),
        message: "Service is ready".to_string(),
    })
}