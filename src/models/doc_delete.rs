use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request payload for deleting a document
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DocumentDeleteRequest {
    #[serde(rename = "byPrpl")]
    pub by_prpl: String,
}

/// Response returned after deleting a document
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DocumentDeleteResponse {
    pub success: bool,
}
