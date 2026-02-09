use crate::{auth::auth, models::{DocumentVersionResponse, DocumentVersionRequest, ErrorResponse}, ws::docctx::DocContext};
use axum::{extract::{State, Path, Extension}, http::StatusCode, Json};
use base64::{engine::general_purpose, Engine as _};
use loro_protocol::CrdtType;
use loro_websocket_server::{HubRegistry, RoomKey};
use std::{collections::HashMap, sync::Arc};
use tracing::{error, warn};
use loro::{LoroDoc, ToJson, VersionVector};
use uuid::Uuid;
use crate::services::doc_db_service;

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


/// Get the version of a document
pub async fn doc_version(
    State(registry): State<Arc<HubRegistry<DocContext>>>,
    Extension(prpls): Extension<Vec<String>>,
    Path((org_id, doc_id)): Path<(String, String)>,
    Json(request): Json<DocumentVersionRequest>,
) -> Result<(StatusCode, Json<DocumentVersionResponse>), (StatusCode, Json<ErrorResponse>)> {

    let output_format = match OutputFormat::from_query(request.format.clone()) {
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
            warn!("Invalid document UUID '{}': {}", doc_id, e);
            let status = StatusCode::BAD_REQUEST;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Invalid document UUID '{}'", doc_id),
            })));
        }
    };

    // Extract version info from request
    let version = request.version;
    let version_v = request.version_v;

    // We need the loro_doc of the specified version
    let mut target_loro_doc: Option<LoroDoc> = None;
    let mut target_peer_map: Option<HashMap<u64, String>> = None;

    // 1. Check if the document of that targeted version is currently open in the Hub.
    let hubs = registry.hubs().lock().await;
    if let Some(hub) = hubs.get(&org_id) {
        let h = hub.lock().await;
        if let Some(doc_state) = h.docs.get(&RoomKey {crdt: CrdtType::Loro, room: doc_id.clone()}) {
            if let (Some(doc), Some(ctx)) = (doc_state.doc.get_loro_doc(), &doc_state.ctx) {
                if ctx.doc_version == version {
                    target_loro_doc = Some(doc.clone());
                    target_peer_map = Some(ctx.peer_map.clone());
                }
            }
        }
    }
    
    // 2. If not currently loaded, we try to load the document of that version from the database.
    if target_loro_doc.is_none() {
        let (snapshot, ctx) = match doc_db_service::fetch_doc_snapshot_from_db(&org_id, &doc_id, Some(version)).await {
            Ok(Some(res)) => res,
            Ok(None) => {
                warn!("Document '{}' with version {} not found in organization '{}'", doc_id, version, org_id);
                let status = StatusCode::NOT_FOUND;
                return Err((status, Json(ErrorResponse {
                    code: status.as_u16(),
                    status: status.to_string(),
                    error: format!("Document '{}' with version {} not found in organization '{}'", doc_id, version, org_id),
                })));
            },
            Err(e) => {
                error!("Error loading document '{}' in org '{}' with version {} from database: {}", doc_id, org_id, version, e);
                let status = StatusCode::INTERNAL_SERVER_ERROR;
                return Err((status, Json(ErrorResponse {
                    code: status.as_u16(),
                    status: status.to_string(),
                    error: format!("Error loading document '{}' in org '{}' with version {} from database: {}", doc_id, org_id, version, e),
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
        target_loro_doc = Some(loro_doc);
        target_peer_map = Some(ctx.peer_map.clone());
    }


    // Make sure we have found the loro_doc of the specified version, if not return an error. This should not happen because we check both in memory and database, but we want to be sure before proceeding.
    let loro_doc = match target_loro_doc {
        Some(doc) => doc,
        None => {
            error!("Document '{}' with version {} not found in organization '{}'", doc_id, version, org_id);
            let status = StatusCode::NOT_FOUND;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Document '{}' with version {} not found in organization '{}'", doc_id, version, org_id),
            })));
        }
    };


    // Now we have the target_loro_doc, if a version vector is specified ...
    let frontiers = match &version_v {
        Some(vv) => {
            // go back to the specific point in time specified by version_v. 
            let loro_version_v = VersionVector::from_iter(vv.clone());
            let frontier_result = std::panic::catch_unwind(|| loro_doc.vv_to_frontiers(&loro_version_v));
            let frontiers = match frontier_result {
                Ok(frontiers) => frontiers,
                Err(e) => {
                    error!("Failed to compute frontiers for version vector: {:?}", e);
                    let status = StatusCode::INTERNAL_SERVER_ERROR;
                    return Err((status, Json(ErrorResponse {
                        code: status.as_u16(),
                        status: status.to_string(),
                        error: format!("Failed to compute frontiers for specified version vector"),
                    })));
                }
            };
            frontiers
        },
        None => {
            // If no version vector is specified, use the current state of the document
            loro_doc.state_frontiers()
        }
    };

    // Checkout the loro_doc to the computed frontiers. This will allow us to get the state of the document at the specified version vector.
    match loro_doc.checkout(&frontiers) {
        Ok(()) => {},
        Err(e) => {
            error!("Failed to checkout document '{}' with version '{}' to version vector: {}", doc_id, version, e);
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Failed to checkout document '{}' to specified version vector", doc_id),
            })));
        }
    };
    

    let binary_str = if output_format.include_binary() {
        let binary_snapshot = loro_doc.export(loro::ExportMode::state_only(Some(&frontiers))).map_err(|e| {
            error!("Failed to export document '{}' with version '{}' to binary: {}", doc_id, version, e);
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            (status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Failed to export document '{}' with version '{}' to binary", doc_id, version),
            }))
        })?;
        Some(general_purpose::STANDARD.encode(&binary_snapshot))
    } else {
        None
    };

    let json = if output_format.include_json() {
        let loro_value = loro_doc.get_deep_value();
        Some(loro_value.to_json_value())
    } else {
        None
    };

    // Serialize the peer map
    let peer_map = serde_json::to_value(target_peer_map.unwrap_or_default()).map_err(|e| {
        error!("Failed to serialize peer_map for document '{}' and version '{}': {}", &doc_id, version, e);
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        (status, Json(ErrorResponse {
            code: status.as_u16(),
            status: status.to_string(),
            error: format!("Failed to serialize peer_map for document '{}' and version '{}': {}", &doc_id, version, e),
        }))
    })?;

    let version_v_json = match &version_v {
        Some(vv) => {
            let loro_version_v = VersionVector::from_iter(vv.clone());
            serde_json::to_value(&loro_version_v).map_err(|e| {
                error!("Failed to serialize specified version_v: {}", e);
                let status = StatusCode::INTERNAL_SERVER_ERROR;
                (status, Json(ErrorResponse {
                    code: status.as_u16(),
                    status: status.to_string(),
                    error: format!("Failed to serialize specified version_v: {}", e),
                }))
            })?
        },
        None => {serde_json::to_value(loro_doc.state_vv()).map_err(|e| {
                error!("Failed to serialize version_v for document '{}': {}", &doc_id, e);
                let status = StatusCode::INTERNAL_SERVER_ERROR;
                (status, Json(ErrorResponse {
                    code: status.as_u16(),
                    status: status.to_string(),
                    error: format!("Failed to serialize version_v for document '{}': {}", &doc_id, e),
                }))
            })?},
    };


    // Return the result
    return Ok((
        StatusCode::OK,
        Json(DocumentVersionResponse {
            json,
            binary: binary_str,
            version: version,
            version_v: version_v_json,
            peer_map,
        }),
    ));
    

}
