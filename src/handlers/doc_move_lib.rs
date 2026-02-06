use crate::{auth::auth, models::{DocumentMoveLibRequest, DocumentMoveLibResponse, ErrorResponse}, services::doc_edit_service, ws::docctx::DocContext};
use axum::{Json, extract::{Extension, Path, State}, http::StatusCode};
use loro_websocket_server::HubRegistry;
use std::sync::Arc;
use tracing::{error, info};
use loro::{LoroDoc, LoroMap};
use uuid::Uuid;
use crate::db::dbcolab;

/// Clear ACLs for a document
pub async fn doc_move_lib(
    State(registry): State<Arc<HubRegistry<DocContext>>>,
    Extension(prpls): Extension<Vec<String>>,
    Path((org_id, doc_id)): Path<(String, String)>,
    Json(request): Json<DocumentMoveLibRequest>,
) -> Result<(StatusCode, Json<DocumentMoveLibResponse>), (StatusCode, Json<ErrorResponse>)> {

    // Ensure the user is an org member or service
    let _ = auth::ensure_service(&prpls, "colabri-app")?;

    // Extract library_id from request
    let library_id_string = request.library_id;
    let by_prpl = request.by_prpl;

    // Parse the doc_id as an UUID
    let doc_uuid = match Uuid::parse_str(&doc_id) {
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

    // Parse the library_id as an UUID
    let lib_uuid = match Uuid::parse_str(&library_id_string) {
        Ok(uuid) => uuid,
        Err(e) => {
            error!("Invalid library UUID '{}': {}", library_id_string, e);
            let status = StatusCode::BAD_REQUEST;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Invalid library UUID '{}'", library_id_string),
            })));
        }
    };

    // Let's first move the document in the database, and if that succeeds, we edit the document in the Hub to clear the ACLs and force close it.
    let db = match dbcolab::get_db() {
        Some(db) => db,
        None => {
            error!("Database not initialized");
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: "Database not initialized".to_string(),
            })));
        }
    };
    match db.move_colab_doc_to_lib(&org_id, &lib_uuid, &doc_uuid, &by_prpl).await {
        Ok(_) => info!("Document '{}' moved to library '{}'", doc_id, library_id_string),
        Err(e) => {
            error!("Failed to move document '{}' to library '{}': {}", doc_id, library_id_string, e);
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            return Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Failed to move document '{}' to library '{}': {}", doc_id, library_id_string, e),
            })));
        }
    }


    // Edit the document ... remove all ACLs and force close the room to kick all users out and prevent further edits.
    let result = doc_edit_service::edit_doc(registry, &org_id, &doc_id, |doc: &LoroDoc| {
        let props = doc.get_map("properties");

        // Use if let to safely get the type string without panicking unwrap()
        if let Some(type_val) = props.get("type") {
            // Safely convert to value value then string
            let type_str = type_val.as_value()
                .and_then(|v| v.as_string().map(|s| s.to_string()))
                .ok_or_else(|| format!("Document type property is not a string"))?;

            // Match directly on string since ColabModelType doesn't implement FromStr
            match type_str.as_str() {
                "colab-statement" => {
                    // Reset ACLs for known types
                    reset_acls_statement_doc(doc)?;
                },
                "colab-sheet" => {
                    reset_acls_sheet_doc(doc)?;
                },
                _ => {
                    return Err(format!("Unknown or unsupported document type: {}", type_str));
                }
            }
        } else {
             return Err(format!("Document type property not found for document '{}'", doc_id));
        }

        // Commit the changes to the document
        doc.commit();
        // Return success
        Ok(())
    }, true).await;

    match result {
        Ok(_) => 
            Ok((
                StatusCode::OK,
                Json(DocumentMoveLibResponse {
                    success: true,
                }),
            )),
        Err(e) => {
            error!("Failed to clear ACLs for document '{}': {}", doc_id, e);
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            Err((status, Json(ErrorResponse {
                code: status.as_u16(),
                status: status.to_string(),
                error: format!("Failed to clear ACLs for document '{}': {}", doc_id, e),
            })))
        }
    }
}


fn reset_acls_statement_doc(doc: &LoroDoc) -> Result<(), String> {
    let acls = doc.get_map("acls");
    acls.clear().map_err(|e| format!("Failed to clear ACLs: {}", e))?;

    // Iterate over the languages
    let content = doc.get_map("content");
    let keys: Vec<String> = content.keys().map(|k| k.to_string()).collect();
    
    // Iterate over all keys in content
    for lang_code in keys {
        if let Some(val) = content.get(&lang_code) {
            if let Some(container) = val.as_container() {
                if let Some(map) = container.as_map() {
                // Clear the ACLs for the language
                    if let Some(acls_val) = map.get("acls") {
                        if let Some(acls_container) = acls_val.as_container() {
                            if let Some(acls_map) = acls_container.as_map() {
                                acls_map.clear().map_err(|e| format!("Failed to clear ACLs for language '{}': {}", lang_code, e))?;
                                info!("Cleared ACLs for language '{}'", lang_code);
                            }
                        }
                    }
                }
            }
        }
    }
    info!("Cleared ACLs for statement document");
    Ok(())
}

fn reset_acls_sheet_doc(doc: &LoroDoc) -> Result<(), String> {
    
    info!("Resetting ACLs for sheet document");
    
    let acls = doc.get_map("acls");
    acls.clear().map_err(|e| format!("Failed to clear ACLs: {}", e))?;
    info!("Cleared top-level ACLs for sheet document");

    // Iterate over the blocks
    let content: loro::LoroMovableList = doc.get_movable_list("content");
    

    // Iterate over all keys in content
    for i in 0..content.len() {
        if let Some(val) = content.get(i) {
            if let Some(container) = val.as_container() {
                if let Some(block) = container.as_map() {

                    // Clear the ACLs for the block
                    if let Some(acls_val) = block.get("acls") {
                        if let Some(acls_container) = acls_val.as_container() {
                            if let Some(acls_map) = acls_container.as_map() {
                                acls_map.clear().map_err(|e| format!("Failed to clear ACLs for block '{}': {}", i, e))?;
                            }
                        }
                    }

                    let block_type_str = block.get("type")
                        .ok_or_else(|| "Block missing 'type' field".to_string())?
                        .as_value()
                        .ok_or_else(|| "'type' is not a value".to_string())?
                        .as_string()
                        .map(|v| v.to_string())
                        .ok_or_else(|| "'type' is not a string".to_string())?;

                    if block_type_str == "statement-grid" {
                        // Safely get rows list
                        let rows_val = block.get("rows")
                            .ok_or_else(|| "Rows not found in statement-grid".to_string())?;

                        let rows_container = rows_val.as_container()
                            .ok_or_else(|| "Rows is not a container".to_string())?;
                        let rows = rows_container.as_movable_list()
                            .ok_or_else(|| "Rows is not a movable list".to_string())?;

                        for r in 0..rows.len() {
                            let row_val = rows.get(r)
                                .ok_or_else(|| "No row found on this index".to_string())?;
                            let row_container = row_val.as_container()
                                .ok_or_else(|| "The row is not persisted as a container".to_string())?;
                            let row = row_container.as_map()
                                .ok_or_else(|| "The row is not persisted as a map".to_string())?;

                            let row_type_val = row.get("type")
                                .ok_or_else(|| "Row missing 'type' field".to_string())?;
                            let row_type_value = row_type_val.as_value()
                                .ok_or_else(|| "'type' is not a value".to_string())?;
                            let row_type = row_type_value.as_string()
                                .map(|v| v.to_string())
                                .ok_or_else(|| "'type' is not a string".to_string())?;

                            if row_type != "local" {
                                continue;
                            } else {
                                let statement_val = row.get("statement")
                                    .ok_or_else(|| "Row missing 'statement' field".to_string())?;
                                let statement_container = statement_val.as_container()
                                    .ok_or_else(|| "'statement' is not a container".to_string())?;
                                let statement = statement_container.as_map()
                                    .ok_or_else(|| "'statement' is not a map".to_string())?;

                                reset_acls_statement(statement)?;
                            }
                        }
                    }

                    // Log cleared block ACLs
                    info!("Cleared ACLs for block '{}'", i);
                }
            }
        }
    }

    Ok(())
}


fn reset_acls_statement(map: &LoroMap) -> Result<(), String> {
    
    // Get the statement top acls
    let acls_val = map.get("acls")
        .ok_or_else(|| "Could not find top acls on the statement".to_string())?;
    let acls_container = acls_val.as_container()
        .ok_or_else(|| "Top acls on statement is not a container".to_string())?;
    let acls = acls_container.as_map()
        .ok_or_else(|| "Top acls on statement is not a map".to_string())?;

    let properties_val = map.get("properties")
        .ok_or_else(|| "Could not find properties map on the statement".to_string())?;
    let properties_container = properties_val.as_container()
        .ok_or_else(|| "Properties on statement is not a container".to_string())?;
    let properties = properties_container.as_map()
        .ok_or_else(|| "Properties on statement is not a map".to_string())?;

    let content_type_val = properties.get("contentType")
        .ok_or_else(|| "Could not find content type property on the statement".to_string())?;
    let content_type = content_type_val.as_value()
        .ok_or_else(|| "Content type property on statement is not a value".to_string())?;
    let content_type_str = content_type.as_string()
        .ok_or_else(|| "Content type property on statement is not a string".to_string())?;
    let content_type = content_type_str.to_string();

    // Clear them
    acls.clear().map_err(|e| format!("Failed to clear ACLs: {}", e))?;

    // Get the content map
    let content_val = map.get("content")
        .ok_or_else(|| "Could not find content map on the statement".to_string())?;
    let content_container = content_val.as_container()
        .ok_or_else(|| "Content on statement is not a container".to_string())?;
    let content = content_container.as_map()
        .ok_or_else(|| "Content on statement is not a map".to_string())?;


    // Iterate over the languages
    let keys: Vec<String> = content.keys().map(|k| k.to_string()).collect();
    
    // Iterate over all keys in content
    for lang_code in keys {
        if let Some(val) = content.get(&lang_code) {
            if let Some(container) = val.as_container() {
                if let Some(map) = container.as_map() {
                // Clear the ACLs for the language
                    if let Some(acls_val) = map.get("acls") {
                        if let Some(acls_container) = acls_val.as_container() {
                            if let Some(acls_map) = acls_container.as_map() {
                                acls_map.clear().map_err(|e| format!("Failed to clear ACLs for language '{}': {}", lang_code, e))?;
                                info!("Cleared ACLs for language '{}'", lang_code);
                            }
                        }
                    }
                }
            }
        }
    }
    info!("Cleared ACLs for statement document with content type '{}'", content_type);
    Ok(())
}