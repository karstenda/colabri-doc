use serde::{Deserialize, Serialize};
use utoipa::ToSchema;


/// Response for moving a document to the library
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DocumentMoveLibRequest {
    #[serde(rename = "libraryId")]
    pub library_id: String,
    #[serde(rename = "byPrpl")]
    pub by_prpl: String,
}

/// Response for moving a document to the library
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DocumentMoveLibResponse {
    pub success: bool,
}



