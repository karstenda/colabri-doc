use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response for exporting a document
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DocumentLatestResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json: Option<serde_json::value::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<String>,
    pub version: u32,
    #[serde(rename = "versionV")]
    pub version_v: serde_json::value::Value,
    #[serde(rename = "peerMap")]
    pub peer_map: serde_json::value::Value,
}
