use crate::{
    auth::auth,
    db::dbcolab,
    models::{DocumentDeleteRequest, DocumentDeleteResponse, ErrorResponse},
    ws::docctx::DocContext,
};
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use loro_protocol::CrdtType;
use loro_websocket_server::HubRegistry;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

/// Delete a document by marking it deleted in the DB and force closing the room
pub async fn doc_delete(
    State(registry): State<Arc<HubRegistry<DocContext>>>,
    Extension(prpls): Extension<Vec<String>>,
    Path((org_id, doc_id)): Path<(String, String)>,
    Json(request): Json<DocumentDeleteRequest>,
) -> Result<(StatusCode, Json<DocumentDeleteResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Ensure the caller is a trusted service
    let _ = auth::ensure_service(&prpls, "colabri-app")?;

    let by_prpl = request.by_prpl;

    // Parse document id
    let doc_uuid = match Uuid::parse_str(&doc_id) {
        Ok(uuid) => uuid,
        Err(e) => {
            error!("Invalid document UUID '{}': {}", doc_id, e);
            let status = StatusCode::BAD_REQUEST;
            return Err((
                status,
                Json(ErrorResponse {
                    code: status.as_u16(),
                    status: status.to_string(),
                    error: format!("Invalid document UUID '{}'", doc_id),
                }),
            ));
        }
    };

    // Fetch database handle
    let db = match dbcolab::get_db() {
        Some(db) => db,
        None => {
            error!("Database not initialized");
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            return Err((
                status,
                Json(ErrorResponse {
                    code: status.as_u16(),
                    status: status.to_string(),
                    error: "Database not initialized".to_string(),
                }),
            ));
        }
    };

    // Mark document as deleted
    match db.delete_colab_doc(&org_id, &doc_uuid, &by_prpl).await {
        Ok(_) => info!("Document '{}' marked as deleted", doc_id),
        Err(e) => {
            error!("Failed to delete document '{}': {}", doc_id, e);
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            return Err((
                status,
                Json(ErrorResponse {
                    code: status.as_u16(),
                    status: status.to_string(),
                    error: format!("Failed to delete document '{}': {}", doc_id, e),
                }),
            ));
        }
    }

    // Force close the room to evict connected users
    registry
        .close_room(&org_id, CrdtType::Loro, &doc_id, true)
        .await;
    info!(
        "Force closed room for document '{}' in org '{}' after deletion",
        doc_id, org_id
    );

    Ok((
        StatusCode::OK,
        Json(DocumentDeleteResponse { success: true }),
    ))
}
