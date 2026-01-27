use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response for creating an item
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DocumentLibraryMoveRequest {
    pub doc_id: String,
}

/// Response for creating an item
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DocumentLibraryMoveResponse {
    pub success: bool,
}
