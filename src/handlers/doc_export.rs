use crate::{auth::auth, models::{DocumentExportResponse, ErrorResponse}, ws::docctx::DocContext};
use axum::{extract::{State, Path, Extension}, http::StatusCode, Json};
use loro_protocol::CrdtType;
use loro_websocket_server::{HubRegistry, RoomKey};
use std::sync::Arc;
use tracing::error;
use loro::ToJson;

/// Export a document
pub async fn doc_export(
    State(registry): State<Arc<HubRegistry<DocContext>>>,
    Extension(prpls): Extension<Vec<String>>,
    Path((org_id, doc_id)): Path<(String, String)>,
) -> Result<(StatusCode, Json<DocumentExportResponse>), (StatusCode, Json<ErrorResponse>)> {

    // Ensure the user is an org member or service
    let _ = auth::ensure_service(&prpls, "colabri-app")?;

    // Get the hub for the organization
    let hubs = registry.hubs().lock().await;
    let hub = match hubs.get(&org_id) {
        Some(hub) => hub,
        None => {
            error!("Organization '{}' not found", org_id);
            let status = StatusCode::NOT_FOUND;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Organization '{}' not found", org_id),
            })));
        }
    };

    // Get the document state
    let h = hub.lock().await;
    let doc_state = match h.docs.get(&RoomKey {crdt: CrdtType::Loro, room: doc_id.clone()}) {
        Some(doc_state) => doc_state,
        None => {
            error!("Document '{}' not found in organization '{}'", doc_id, org_id);
            let status = StatusCode::NOT_FOUND;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Document '{}' not found in organization '{}'", doc_id, org_id),
            })));
        }
    };

    // Get the loro document
    let loro_doc = match doc_state.doc.get_loro_doc() {
        Some(doc) => doc,
        None => {
            error!("Loro document not found for document '{}' in organization '{}'", doc_id, org_id);
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Loro document not found for document '{}' in organization '{}'", doc_id, org_id),
            })));
        }
    };

    // Get the document context
    let doc_ctx = match &doc_state.ctx {
        Some(ctx) => ctx,
        None => {
            error!("Document context not found for document '{}' in organization '{}'", doc_id, org_id);
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Document context not found for document '{}' in organization '{}'", doc_id, org_id),
            })));
        }
    };

    // Get the JSON representations
    let loro_value = loro_doc.get_deep_value();
    let json = loro_value.to_json_value();
    let state_vv = loro_doc.state_vv();
    let state_vv_json = match serde_json::to_value(&state_vv) {
        Ok(val) => val,
        Err(e) => {
            error!("Failed to serialize state_vv for document '{}': {}", &doc_id, e);
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Failed to serialize state_vv for document '{}': {}", &doc_id, e),
            })));
        }
    };
    let peer_map_json = match serde_json::to_value(&doc_ctx.peer_map.clone()) {
        Ok(val) => val,
        Err(e) => {
            error!("Failed to serialize peer_map for document '{}': {}", &doc_id, e);
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Failed to serialize peer_map for document '{}': {}", &doc_id, e),
            })));
        }
    };


    Ok((
        StatusCode::OK,
        Json(DocumentExportResponse {
            json: json,
            version_v: state_vv_json,
            peer_map: peer_map_json,
        }),
    ))
}
