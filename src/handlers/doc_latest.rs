use crate::{auth::auth, models::{DocumentLatestResponse, ErrorResponse}, ws::docctx::DocContext};
use axum::{extract::{State, Path, Extension, Query}, http::StatusCode, Json};
use base64::{engine::general_purpose, Engine as _};
use loro_protocol::CrdtType;
use loro_websocket_server::{HubRegistry, RoomKey};
use std::sync::Arc;
use tracing::error;
use loro::{ToJson, LoroDoc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct OutputFormatQuery {
    format: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum OutputFormat {
    Json,
    Binary,
    Both,
}

impl OutputFormat {
    fn from_query(format: Option<String>) -> Result<Self, String> {
        match format.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
            None => Ok(OutputFormat::Json),
            Some(value) => match value.to_lowercase().as_str() {
                "json" => Ok(OutputFormat::Json),
                "binary" => Ok(OutputFormat::Binary),
                "both" => Ok(OutputFormat::Both),
                other => Err(format!("Invalid output format '{}'. Use 'json', 'binary', or 'both'.", other)),
            },
        }
    }

    fn include_json(self) -> bool {
        matches!(self, OutputFormat::Json | OutputFormat::Both)
    }

    fn include_binary(self) -> bool {
        matches!(self, OutputFormat::Binary | OutputFormat::Both)
    }
}

/// Export a document
pub async fn doc_latest(
    State(registry): State<Arc<HubRegistry<DocContext>>>,
    Extension(prpls): Extension<Vec<String>>,
    Path((org_id, doc_id)): Path<(String, String)>,
    Query(query): Query<OutputFormatQuery>,
) -> Result<(StatusCode, Json<DocumentLatestResponse>), (StatusCode, Json<ErrorResponse>)> {

    let output_format = match OutputFormat::from_query(query.format) {
        Ok(format) => format,
        Err(message) => {
            let status = StatusCode::BAD_REQUEST;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: message,
            })));
        }
    };

    // Ensure the user is an org member or service
    let _ = auth::ensure_service(&prpls, "colabri-app")?;

    // Parse the doc_id as an UUID
    let _doc_uuid = match Uuid::parse_str(&doc_id) {
        Ok(uuid) => uuid,
        Err(e) => {
            error!("Invalid document UUID '{}': {}", doc_id, e);
            let status = StatusCode::BAD_REQUEST;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Invalid document UUID '{}'", doc_id),
            })));
        }
    };

    // Try to get data from memory (Hub)
    let mem_data = {
        let hubs = registry.hubs().lock().await;
        if let Some(hub) = hubs.get(&org_id) {
            let h = hub.lock().await;
            if let Some(doc_state) = h.docs.get(&RoomKey {crdt: CrdtType::Loro, room: doc_id.clone()}) {
                if let (Some(loro_doc), Some(ctx)) = (doc_state.doc.get_loro_doc(), &doc_state.ctx) {
                    let (json, binary_str, version_v, peer_map) = build_doc_payload(&loro_doc, &ctx.peer_map, &doc_id, output_format)?;
                    Some((json, binary_str, version_v, peer_map, ctx.doc_version.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    };

    if let Some((json, binary_str, version_v, peer_map, doc_version)) = mem_data {
        return Ok((
            StatusCode::OK,
            Json(DocumentLatestResponse {
                json,
                binary: binary_str,
                version: doc_version,
                version_v,
                peer_map,
            }),
        ));
    }

    // If not found in memory, try to load from database
    let (snapshot, ctx) = match crate::services::doc_db_service::fetch_doc_snapshot_from_db(&org_id, &doc_id, None).await {
        Ok(Some(res)) => res,
        Ok(None) => {
            error!("Document '{}' not found in organization '{}'", doc_id, org_id);
            let status = StatusCode::NOT_FOUND;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Document '{}' not found in organization '{}'", doc_id, org_id),
            })));
        },
        Err(e) => {
            error!("Error loading document '{}' from database: {}", doc_id, e);
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Error loading document '{}' from database: {}", doc_id, e),
            })));
        }
    };

    // Reconstruct LoroDoc from snapshot
    let loro_doc = LoroDoc::new();
    loro_doc.import(&snapshot).map_err(|e| {
        error!("Failed to import snapshot for document '{}': {}", doc_id, e);
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        (status, Json(ErrorResponse {
            code: status.as_u16(),
            status: status.to_string(),
            error: format!("Failed to import snapshot for document '{}': {}", doc_id, e),
        }))
    })?;

    let (json, binary_str, state_vv_json, peer_map_json) = build_doc_payload(&loro_doc, &ctx.peer_map, &doc_id, output_format)?;

    Ok((
        StatusCode::OK,
        Json(DocumentLatestResponse {
            json,
            binary: binary_str,
            version: ctx.doc_version,
            version_v: state_vv_json,
            peer_map: peer_map_json,
        }),
    ))
}

fn build_doc_payload<P>(
    loro_doc: &LoroDoc,
    peer_map: &P,
    doc_id: &str,
    output_format: OutputFormat,
) -> Result<(Option<serde_json::Value>, Option<String>, serde_json::Value, serde_json::Value), (StatusCode, Json<ErrorResponse>)>
where
    P: Serialize,
{
    let json = if output_format.include_json() {
        let loro_value = loro_doc.get_deep_value();
        Some(loro_value.to_json_value())
    } else {
        None
    };
    let state_vv = loro_doc.state_vv();

    let state_vv_json = serde_json::to_value(&state_vv).map_err(|e| {
        error!("Failed to serialize state_vv for document '{}': {}", doc_id, e);
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        (status, Json(ErrorResponse {
            code: status.as_u16(),
            status: status.to_string(),
            error: format!("Failed to serialize state_vv for document '{}': {}", doc_id, e),
        }))
    })?;

    let peer_map_json = serde_json::to_value(peer_map).map_err(|e| {
        error!("Failed to serialize peer_map for document '{}': {}", doc_id, e);
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        (status, Json(ErrorResponse {
            code: status.as_u16(),
            status: status.to_string(),
            error: format!("Failed to serialize peer_map for document '{}': {}", doc_id, e),
        }))
    })?;

    let binary_str = if output_format.include_binary() {
        let binary_snapshot = loro_doc.export(loro::ExportMode::state_only(None)).map_err(|e| {
            error!("Failed to export latest state for document '{}' to binary: {}", doc_id, e);
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            (status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Failed to export latest state for document '{}' to binary", doc_id),
            }))
        })?;
        Some(general_purpose::STANDARD.encode(&binary_snapshot))
    } else {
        None
    };

    Ok((json, binary_str, state_vv_json, peer_map_json))
}
