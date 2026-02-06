use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;


/// Request for getting a specific document version
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DocumentVersionRequest {
    #[serde(rename = "version")]
    pub version: u32,
    #[serde(rename = "versionV")]
    pub version_v: Option<HashMap<u64, i32>>,
}


/// Response for getting a specific document version
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DocumentVersionResponse {
    pub json: serde_json::value::Value,
    #[serde(rename = "versionV")]
    pub version_v: serde_json::value::Value,
    #[serde(rename = "peerMap")]
    pub peer_map: serde_json::value::Value,
}
