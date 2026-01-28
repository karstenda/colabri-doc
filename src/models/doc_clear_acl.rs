use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response for creating an item
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DocumentClearAclResponse {
    pub success: bool,
}
