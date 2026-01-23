use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// API response for health check
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ReadyResponse {
    pub status: String,
    pub message: String,
}
