use loro::{ LoroDoc, ToJson};
use loro_protocol::CrdtType;
use tracing::{info, warn, error};
use uuid::Uuid;
use std::pin::Pin;
use std::future::Future;
use moka::future::Cache;
use std::time::Duration;
use tokio::sync::OnceCell;

use crate::{db::dbcolab::{self, DocumentStreamRow}, models::ColabStatementModel};
use super::docsession::DocSession;

/// Global cache instance
static DOC_CACHE: OnceCell<Cache<String, DocSession>> = OnceCell::const_new();

/// Initialize the document cache
/// 
/// This should be called once at application startup.
/// The cache will automatically evict entries after 5 minutes of inactivity.
pub async fn init_doc_cache() {
    DOC_CACHE.get_or_init(|| async {
        Cache::builder()
            .max_capacity(1000000) // Adjust based on your needs
            .time_to_idle(Duration::from_secs(300)) // 5 minutes TTL
            .build()
    }).await;
    info!("Document cache initialized");
}

/// Get the document cache instance
fn get_doc_cache() -> &'static Cache<String, DocSession> {
    DOC_CACHE.get().expect("Document cache not initialized. Call init_doc_cache() first.")
}

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
        
        // Get cache to check for session info
        let cache = get_doc_cache();
        
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
        
        // Iterate over the streams and search for the stream with name "main" and the highest version.
        let mut main_stream: Option<&DocumentStreamRow> = None;
        let mut main_stream_bytes: Option<&Vec<u8>> = None;
        let mut highest_version = -1;
        for stream in &doc_data.streams {
            if stream.name == "main" && stream.version > highest_version {
                if let Some(content) = &stream.content {
                    main_stream_bytes = Some(content);
                    main_stream = Some(stream);
                    highest_version = stream.version;
                }
            }
        }

        // Check if we found content for the highest main stream
        if main_stream_bytes.is_none() || main_stream.is_none() {
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
                let loro_doc = match crate::models::colab::stmt_to_loro_doc(&stmt_model) {
                    Some(doc) => doc,
                    None => {
                        error!("Failed to convert statement model to LoroDoc for document '{}'", doc_uuid.to_string());
                        return Err("Failed to convert model to LoroDoc".to_string());
                    }
                };

                // Export the LoroDoc as a byte stream
                let snapshot = loro_doc.export(loro::ExportMode::Snapshot).unwrap();

                // Store the generated snapshot as a new stream in the database
                let docstream_id = match db.insert_statement_doc_stream(
                    org,
                    doc_uuid,
                    snapshot.clone()
                ).await {
                    Ok(id) => id,
                    Err(e) => {
                        error!("Failed to insert document stream for document '{}': {}", doc_uuid.to_string(), e);
                        return Err(format!("Failed to insert document stream: {}", e));
                    }
                };

                // Let's add it to the docsession cache.
                let session = DocSession {
                    org: org.to_string(),
                    doc_id: doc_uuid.clone(),
                    doc_stream_id: docstream_id.clone()
                };
                cache.insert(room_id.clone(), session).await;

                return Ok(Some(snapshot));
            }
            // No stream and no json
            else {
                error!("No content found for document '{}'", doc_uuid.to_string());
                return Err("No content found".to_string());
            }
        }
        // Import the content into the LoroDoc
        else {

            // Let's add it to the docsession cache.
            let session = DocSession {
                org: org.to_string(),
                doc_id: doc_uuid.clone(),
                doc_stream_id: main_stream.unwrap().id.clone(),
            };
            cache.insert(room_id.clone(), session).await;


            info!("Successfully loaded document: {} ({} bytes)", doc_uuid.to_string(), main_stream_bytes.unwrap().len());
            return Ok(main_stream_bytes.cloned())
        }
    })
}

/// Save a document to storage
/// 
/// This function is called periodically (based on save_interval_ms) to persist
/// the current state of a document to storage.
/// 
/// # Arguments
/// * `doc_id` - The unique identifier of the document to save (format: "org_id/doc_uuid")
/// * `doc` - The LoroDoc instance containing the current document state
pub fn on_save_document(_workspace: String, room_id: String, crdt: CrdtType, snapshot: Vec<u8>) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
    Box::pin(async move {

        // Validate CRDT type
        if crdt != CrdtType::Loro {
            warn!("Unsupported CRDT type for saving document: {:?}", crdt);
            return Ok(());
        }

        // Start saving the loro document
        info!("Saving loro document for room: {}", room_id);

        // Get the cache
        let cache = get_doc_cache();
        let doc_session_res = cache.get(&room_id).await;
        let org: String;
        let doc_uuid: Uuid;
        let doc_stream_uuid: uuid::Uuid;
        if !doc_session_res.is_none() {
            let doc_session = doc_session_res.unwrap();
            org = doc_session.org.clone();
            doc_uuid = doc_session.doc_id.clone();
            doc_stream_uuid = doc_session.doc_stream_id.clone();
        } else {
            error!("No session found in cache for document: {}", room_id);
            return Err("No session found in cache".to_string());
        }

        // Get database connection
        let db = match dbcolab::get_db() {
            Some(db) => db,
            None => {
                error!("Database not initialized, cannot save document: {}", doc_uuid);
                return Err("Database not initialized".to_string());
            }
        };

        // Convert snapshot to JSON for storage in statement
        let loro_doc = LoroDoc::new();
        if let Err(e) = loro_doc.import(&snapshot) {
            error!("Failed to import snapshot for document '{}': {}", doc_uuid, e);
            return Err(format!("Failed to import snapshot for document '{}': {}", doc_uuid, e));
        }

        // Get the JSON representation
        let loro_value = loro_doc.get_deep_value();
        let json = loro_value.to_json_value();
        
        // Save to database with incremented version
        match db.update_statement(&org, doc_uuid, doc_stream_uuid, snapshot, json).await {
            Ok(_) => {
                info!("Statement updated successfully {}", doc_uuid);
            }
            Err(e) => {
                error!("Failed to update statement '{}': {}", doc_uuid, e);
                return Err(format!("Failed to update statement '{}': {}", doc_uuid, e));
            }
        }

        // Touch the cache entry to reset its TTL
        _ = cache.get(&room_id).await;
        return Ok(());
    })
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