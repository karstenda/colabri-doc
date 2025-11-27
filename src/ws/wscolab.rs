use loro::LoroDoc;
use std::sync::Arc;
use tracing::{info, warn, error};
use uuid::Uuid;
use std::pin::Pin;
use std::future::Future;

use crate::{db::dbcolab, models::ColabStatementModel};

/// Load a document from storage
/// 
/// This function is called when a client requests a document that isn't currently loaded.
/// It should load the document data from persistent storage and return it as bytes.
/// 
/// # Arguments
/// * `workspace`
/// * `room_id`
/// * `crdt_type`
/// 
/// # Returns
/// A Result containing an Option with the document bytes, or an error message
pub fn on_load_document(_workspace: String, room_id: String, _crdt_type: loro_websocket_server::protocol::CrdtType) -> Pin<Box<dyn Future<Output = Result<Option<Vec<u8>>, String>> + Send>> {
    Box::pin(async move {
        info!("Loading document: {}", room_id);
    
    // Validate room_id is not empty
    if room_id.is_empty() {
        error!("Empty room ID provided");
        return Err("Room ID cannot be empty".to_string());
    }
    
    // Get database connection
    let db = match dbcolab::get_db() {
        Some(db) => db,
        None => {
            error!("Database not initialized");
            return Err("Database not initialized".to_string());
        }
    };
    
    // Parse room_id
    // Expected format: "org_id/doc_uuid"
    let (org, uuid) = if let Some((org_part, uuid_part)) = room_id.split_once('/') {
        (org_part, uuid_part)
    } else {
        // Default org if not specified
        error!("Wrong formatted room ID provided");
        return Err("Wrong formatted room ID provided".to_string());
    };
    
    // Parse it as an UUID
    let doc_uuid = match Uuid::parse_str(uuid) {
        Ok(uuid) => uuid,
        Err(e) => {
            error!("Invalid document UUID '{}': {}", uuid, e);
            return Err(format!("Invalid document UUID: {}", e));
        }
    };
    
    // Load document from database
    let doc_data = match db.load_statement_doc(org, doc_uuid).await {
        Ok(Some(doc)) => doc,
        Ok(None) => {
            info!("Document not found: {}", doc_uuid.to_string());
            return Ok(None);
        }
        Err(e) => {
            error!("Database error loading document '{}': {}", doc_uuid.to_string(), e);
            return Err(format!("Database error: {}", e));
        }
    };
    
    // Create a new LoroDoc
    let loro_doc;
    
    // Iterate over the streams and search for the stream with name "main" and the highest version.
    let mut main_stream: Option<&Vec<u8>> = None;
    let mut highest_version = -1;
    for stream in &doc_data.streams {
        if stream.name == "main" && stream.version > highest_version {
            if let Some(content) = &stream.content {
                main_stream = Some(content);
                highest_version = stream.version;
            }
        }
    }

    // Check if we found content for the highest main stream
    if main_stream.is_none() {
        if let Some(ref json_value) = doc_data.json {
            // We need to generate the loro doc from the json in the statement.
            // Parse the json as ColabStatementModel
            let stmt_model: ColabStatementModel = match serde_json::from_value(json_value.clone()) {
                Ok(model) => model,
                Err(e) => {
                    error!("Failed to parse statement JSON for document '{}': {}", doc_uuid.to_string(), e);
                    error!("JSON content: {}", serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| "Unable to serialize".to_string()));
                    return Err(format!("Failed to parse JSON: {}", e));
                }
            };
            
            // Convert the statement model to LoroDoc
            loro_doc = match crate::models::colab::stmt_to_loro_doc(&stmt_model) {
                Some(doc) => doc,
                None => {
                    error!("Failed to convert statement model to LoroDoc for document '{}'", doc_uuid.to_string());
                    return Err("Failed to convert model to LoroDoc".to_string());
                }
            };
        }
        // No stream and no json
        else {
            error!("No content found for document '{}'", doc_uuid.to_string());
            return Err("No content found".to_string());
        }
    }
    // Import the content into the LoroDoc
    else {
        loro_doc = LoroDoc::new();
        // Import the content into the LoroDoc
        if let Err(e) = loro_doc.import(main_stream.unwrap()) {
            error!("Failed to import stream version {} for document '{}': {}", 
                    highest_version, doc_uuid.to_string(), e);
            return Err(format!("Failed to import stream: {}", e));
        }
        info!("Loaded stream version {} for document {}", highest_version, doc_uuid.to_string());
    }
    
    // Export the LoroDoc to bytes
    let bytes = match loro_doc.export(loro::ExportMode::Snapshot) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to export document '{}': {}", doc_uuid.to_string(), e);
            return Err(format!("Failed to export document: {}", e));
        }
    };
    info!("Successfully loaded document: {} ({} bytes)", doc_uuid.to_string(), bytes.len());
    Ok(Some(bytes))
    })
}

/// Save a document to storage
/// 
/// This function is called periodically (based on save_interval_ms) to persist
/// the current state of a document to storage.
/// 
/// # Arguments
/// * `doc_id` - The unique identifier of the document to save
/// * `doc` - The LoroDoc instance containing the current document state
pub async fn on_save_document(doc_id: Arc<str>, doc: LoroDoc) {
    info!("Saving document: {}", doc_id);
    
    // TODO: Implement actual document saving logic
    // Example implementation might:
    // 1. Export the document to bytes using doc.export_snapshot() or similar
    // 2. Store the bytes in a database
    // 3. Update metadata (last saved timestamp, version, etc.)
    
    warn!("Document saving not yet implemented for: {}", doc_id);
}

/// Authenticate a client connection
/// 
/// This function is called when a client attempts to connect to the WebSocket server.
/// It should validate the authentication token and return whether the connection is allowed.
/// 
/// # Arguments
/// * `token` - The authentication token provided by the client (e.g., from query params or headers)
/// 
/// # Returns
/// true if the client is authenticated and allowed to connect, false otherwise
pub async fn authenticate(token: Option<String>) -> bool {
    info!("Authenticating client with token: {:?}", token);
    
    // TODO: Implement actual authentication logic
    // Example implementation might:
    // 1. Validate JWT token
    // 2. Check token against database or cache
    // 3. Verify user permissions
    // 4. Return true for valid tokens, false for invalid
    
    // For now, allow all connections (INSECURE - implement proper auth!)
    warn!("Authentication not yet implemented - allowing all connections");
    true
}