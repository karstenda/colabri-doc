use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response for creating an item
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DocumentExportResponse {
    pub json: serde_json::value::Value,
    pub version_v: serde_json::value::Value,
    pub peer_map: serde_json::value::Value,
}
