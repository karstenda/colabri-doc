use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// API response for health check
#[derive(Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub message: String,
}