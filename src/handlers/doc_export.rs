use crate::{auth::auth, models::{DocumentExportResponse, ErrorResponse}, ws::docctx::DocContext};
use axum::{extract::{State, Path, Extension}, http::StatusCode, Json};
use loro_protocol::CrdtType;
use loro_websocket_server::{HubRegistry, RoomKey};
use std::sync::Arc;
use tracing::error;
use loro::{ToJson, LoroDoc};
use uuid::Uuid;

/// Export a document
pub async fn doc_export(
    State(registry): State<Arc<HubRegistry<DocContext>>>,
    Extension(prpls): Extension<Vec<String>>,
    Path((org_id, doc_id)): Path<(String, String)>,
) -> Result<(StatusCode, Json<DocumentExportResponse>), (StatusCode, Json<ErrorResponse>)> {

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
                if let (Some(doc), Some(ctx)) = (doc_state.doc.get_loro_doc(), &doc_state.ctx) {
                    let loro_value = doc.get_deep_value();
                    let json = loro_value.to_json_value();
                    let state_vv = doc.state_vv();
                    
                    let state_vv_json = serde_json::to_value(&state_vv).map_err(|e| {
                        error!("Failed to serialize state_vv for document '{}': {}", &doc_id, e);
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                            code: 500,
                            status: "Internal Server Error".to_string(),
                            error: format!("Failed to serialize state_vv for document '{}': {}", &doc_id, e),
                        }))
                    })?;
                    
                    let peer_map_json = serde_json::to_value(&ctx.peer_map).map_err(|e| {
                        error!("Failed to serialize peer_map for document '{}': {}", &doc_id, e);
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                            code: 500,
                            status: "Internal Server Error".to_string(),
                            error: format!("Failed to serialize peer_map for document '{}': {}", &doc_id, e),
                        }))
                    })?;

                    Some((json, state_vv_json, peer_map_json))
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

    if let Some((json, version_v, peer_map)) = mem_data {
        return Ok((
            StatusCode::OK,
            Json(DocumentExportResponse {
                json,
                version_v,
                peer_map,
            }),
        ));
    }

    // If not found in memory, try to load from database
    let (snapshot, ctx) = match crate::services::doc_service::fetch_doc_snapshot_from_db(&org_id, &doc_id).await {
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

    // Extract data
    let loro_value = loro_doc.get_deep_value();
    let json = loro_value.to_json_value();
    let state_vv = loro_doc.state_vv();
    let state_vv_json = serde_json::to_value(&state_vv).map_err(|e| {
        error!("Failed to serialize state_vv for document '{}': {}", &doc_id, e);
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        (status, Json(ErrorResponse {
            code: status.as_u16(),
            status: status.to_string(),
            error: format!("Failed to serialize state_vv for document '{}': {}", &doc_id, e),
        }))
    })?;
    let peer_map_json = serde_json::to_value(&ctx.peer_map).map_err(|e| {
        error!("Failed to serialize peer_map for document '{}': {}", &doc_id, e);
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        (status, Json(ErrorResponse {
            code: status.as_u16(),
            status: status.to_string(),
            error: format!("Failed to serialize peer_map for document '{}': {}", &doc_id, e),
        }))
    })?;

    Ok((
        StatusCode::OK,
        Json(DocumentExportResponse {
            json: json,
            version_v: state_vv_json,
            peer_map: peer_map_json,
        }),
    ))
}
