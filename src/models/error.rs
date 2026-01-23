use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response for an error
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub code: u16,
    pub status: String,
    pub error: String,
}