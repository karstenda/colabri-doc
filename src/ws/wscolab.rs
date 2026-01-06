use loro::{ LoroDoc, ToJson};
use loro_protocol::{CrdtType, UpdateStatusCode};
use loro_websocket_server::{AuthArgs, CloseConnectionArgs, HandshakeAuthArgs, LoadDocArgs, LoadedDoc, SaveDocArgs, UpdateArgs, UpdatedDoc};
use loro_websocket_server::protocol::Permission;
use tracing::{info, warn, error};
use uuid::Uuid;
use std::{collections::HashMap, pin::Pin};
use std::future::Future;
use moka::sync::Cache;
use std::time::Duration;
use std::sync::OnceLock;
use serde_cbor;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

use crate::models::ColabPackage;
use crate::{db::dbcolab::{self, DocumentStreamRow}, clients::app_service_client ,models::ColabStatementModel};
use super::doccontext::DocContext;
use super::userctx::UserCtx;
use super::connctx::ConnCtx;

/// Global cache instances
static USER_CTX_CACHE: OnceLock<Cache<String, UserCtx>> = OnceLock::new();
static CONN_CTX_CACHE: OnceLock<Cache<u64, ConnCtx>> = OnceLock::new();

/// Initialize the user cache
/// 
/// This should be called once at application startup.
/// The cache will automatically evict entries after 5 minutes of inactivity.
pub fn init_user_ctx_cache() {
    USER_CTX_CACHE.get_or_init(|| {
        Cache::builder()
            .max_capacity(100000) // Adjust based on your needs
            .time_to_idle(Duration::from_secs(5*60)) //5 minutes TTL
            .build()
    });
    info!("User cache initialized");
}

/// Get the user cache instance
fn get_user_ctx_cache() -> &'static Cache<String, UserCtx> {
    USER_CTX_CACHE.get().expect("User cache not initialized. Call init_user_ctx_cache() first.")
}

/// Initialize the connection context cache
/// 
/// This should be called once at application startup.
/// The cache will automatically evict entries after 3 hours of inactivity.
pub fn init_conn_ctx_cache() {
    CONN_CTX_CACHE.get_or_init(|| {
        Cache::builder()
            .max_capacity(100000) // Adjust based on your needs
            .time_to_idle(Duration::from_secs(3*60*60)) // 3 hours TTL
            .build()
    });
    info!("Connection context cache initialized");
}

/// Get the connection context cache instance
fn get_conn_ctx_cache() -> &'static Cache<u64, ConnCtx> {
    CONN_CTX_CACHE.get().expect("Connection context cache not initialized. Call init_conn_ctx_cache() first.")
}

/// Authenticate a client
///
/// This function is called during the WebSocket handshake to authenticate the client.
/// It should check whether the request is made with a valid cookie from a trusted origin.
/// # Arguments
/// * `workspace_id` - The ID of the workspace the client is trying to access
/// * `token` - An optional authentication token provided by the loro-protocol framework (not used)
/// * `request` - The WebSocket handshake request
/// # Returns
pub fn on_auth_handshake(args: HandshakeAuthArgs) -> bool {
    let org_id = args.workspace;
    
    // Extract cookies from the request headers
    let mut cookie_map: HashMap<String, String> = HashMap::new();
    if let Some(header) = args.request.headers().get("Cookie") {
        if let Ok(s) = header.to_str() {
            for cookie in cookie::Cookie::split_parse(s) {
                if let Ok(c) = cookie {
                    cookie_map.insert(c.name().to_string(), c.value().to_string());
                }
            }
        }
    }

    // Check if there's an 'auth_token' cookie
    let auth_token = cookie_map.get("auth_token");
    if auth_token.is_none() {
        error!("No auth_token cookie found in handshake request");
        return false;
    } 

    // Validate the auth_token as a JWT token
    let token = auth_token.unwrap();
    let config = crate::config::get_config();

    if let Some(secret) = &config.cloud_auth_jwt_secret {
        let validation = Validation::new(Algorithm::HS256);
        
        match decode::<serde_json::Value>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation
        ) {
            Ok(token_data) => {
                if let Some(uid) = token_data.claims.get("sub").and_then(|v| v.as_str()) {
                    info!("JWT token validated successfully for user: {}", uid);
                    
                    if let Some(client) = app_service_client::get_app_service_client() {
                        let client = client.clone();
                        let uid_string = uid.to_string();
                        
                        // Use block_in_place to run async code synchronously
                        let result = tokio::task::block_in_place(move || {
                            let (tx, rx) = std::sync::mpsc::channel();
                            tokio::spawn(async move {
                                let res = client.get_prpls(&uid_string).await;
                                let _ = tx.send(res);
                            });
                            rx.recv()
                        });

                        match result {
                            Ok(Ok(prpls_json)) => {
                                info!("Retrieved principals for user {}: {}", uid, prpls_json);
                                
                                // Parse principals from JSON
                                let principals: Vec<String> = if let Some(prpls_val) = prpls_json.get("prpls") {
                                    serde_json::from_value(prpls_val.clone())
                                        .unwrap_or_else(|e| {
                                            error!("Failed to parse principals array from 'prpls' field: {}", e);
                                            Vec::new()
                                        })
                                } else {
                                    serde_json::from_value(prpls_json)
                                        .unwrap_or_else(|e| {
                                            error!("Failed to parse principals JSON: {}", e);
                                            Vec::new()
                                        })
                                };
                                
                                // Ensure there's at least one principal for the organization
                                // Iterate over principals and check for one that starts with "org:{workspace_id}:"
                                let mut has_org_principal = false;
                                for principal in &principals {
                                    if principal.starts_with(&format!("{}/u/", org_id)) {
                                        has_org_principal = true;
                                        break;
                                    } else if principal.eq("r/Colabri-CloudAdmin") {
                                        has_org_principal = true;
                                        break;
                                    }
                                }

                                if !has_org_principal {
                                    error!("User {} does not have access to organization {}", uid, org_id);
                                    return false;
                                }

                                // Store in user cache (sync)
                                let user_ctx = UserCtx {
                                    principals,
                                };
                                let user_ctx_cache = get_user_ctx_cache();
                                user_ctx_cache.insert(uid.to_string(), user_ctx);

                                // Store in connection context cache (sync)
                                let conn_ctx = ConnCtx {
                                    uid: uid.to_string(),
                                    org_id: org_id.to_string(),
                                };
                                let conn_ctx_cache = get_conn_ctx_cache();
                                conn_ctx_cache.insert(args.conn_id, conn_ctx);
                                true
                            }
                            Ok(Err(e)) => {
                                error!("Failed to retrieve principals for user {}: {}", uid, e);
                                false
                            }
                            Err(e) => {
                                error!("Failed to receive result from async task: {}", e);
                                false
                            }
                        }
                    } else {
                        error!("App service client not initialized");
                        false
                    }
                } else {
                    error!("Can't extract a UID from the JWT token");
                    false
                }
            },
            Err(e) => {
                error!("JWT validation failed: {}", e);
                false
            }
        }
    } else {
        warn!("No JWT secret configured, skipping validation (INSECURE)");
        true
    }
}


/// Authenticate a client for a specific document
/// 
/// # Arguments
/// * `args` - Authentication arguments
pub fn on_authenticate(args: AuthArgs) -> Pin<Box<dyn Future<Output = Result<Option<Permission>, String>> + Send>> {
    Box::pin(async move {

        // Get the doc_id
        let doc_id: String = args.room;

        // Get the connection context from the cache
        let conn_ctx_cache = get_conn_ctx_cache();
        let conn_ctx = match conn_ctx_cache.get(&args.conn_id) {
            Some(ctx) => ctx,
            None => {
                error!("No connection context found for connection_id: {}", args.conn_id);
                return Err("No connection context found".to_string());
            }
        };

        // Get the user context from the cache
        let user_ctx_cache = get_user_ctx_cache();
        let user_ctx = match user_ctx_cache.get(&conn_ctx.uid) {
            Some(ctx) => ctx,
            None => {
                error!("No user context found for uid: {}", conn_ctx.uid);
                return Err("No user context found".to_string());
            }
        };

        // Check if the user can view the document
        let db = match dbcolab::get_db() {
            Some(db) => db,
            None => {
                error!("Database not initialized");
                return Err("Database not initialized".to_string());
            }
        };
        let doc_uuid = match Uuid::parse_str(&doc_id) {
            Ok(uuid) => uuid,
            Err(e) => {
                error!("Invalid document UUID '{}': {}", doc_id, e);
                return Err(format!("Invalid document UUID: {}", e));
            }
        };
        // Make the DB call to see if the user can view the document
        let _ = match db.get_viewable_document(&conn_ctx.org_id, doc_uuid, &user_ctx.principals).await {
            Ok(Some(_)) => {
                // The document was found, return Write permission
                return Ok(Some(Permission::Write))
            },
            Ok(None) => {
                info!("User {} does not have access to document {}", conn_ctx.uid, doc_id);
                // Deny access
                return Ok(None);
            }
            Err(e) => {
                error!("Database error checking access for user {} to document {}: {}", conn_ctx.uid, doc_id, e);
                return Err(format!("Database error: {}", e));
            }
        };
    })
}

/// Hanlde the closing of a connection
/// 
/// # Arguments
/// * `args` - Close Connection arguments
pub fn on_close_connection(args: CloseConnectionArgs) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
    Box::pin(async move {
        let conn_id = args.conn_id;
        // Remove from connection context cache
        let conn_ctx_cache = get_conn_ctx_cache();
        conn_ctx_cache.invalidate(&conn_id);
        info!("Connection context removed for connection_id: {}", conn_id);
        Ok(())
    })
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
pub fn on_load_document(args: LoadDocArgs) -> Pin<Box<dyn Future<Output = Result<LoadedDoc<DocContext>, String>> + Send>> {
    let doc_id = args.room;
    let org_id = args.workspace;
    Box::pin(async move {
        info!("Loading document: {}", doc_id);

        // Parse the doc_id as an UUID
        let doc_uuid = match Uuid::parse_str(&doc_id) {
            Ok(uuid) => uuid,
            Err(e) => {
                error!("Invalid document UUID '{}': {}", doc_id, e);
                return Err(format!("Invalid document UUID: {}", e));
            }
        };
        
        // Get database connection
        let db = match dbcolab::get_db() {
            Some(db) => db,
            None => {
                error!("Database not initialized");
                return Err("Database not initialized".to_string());
            }
        };
        
        // Load document from database
        let doc_data = match db.load_statement_doc(&org_id, doc_uuid).await {
            Ok(Some(doc)) => doc,
            Ok(None) => {
                info!("Document not found: {}", doc_uuid.to_string());
                return Ok(LoadedDoc { snapshot: None, ctx: None });
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
                let loro_doc = match crate::models::colabdoc::stmt_to_loro_doc(&stmt_model) {
                    Some(doc) => doc,
                    None => {
                        error!("Failed to convert statement model to LoroDoc for document '{}'", doc_uuid.to_string());
                        return Err("Failed to convert model to LoroDoc".to_string());
                    }
                };

                // Export the LoroDoc as a byte stream
                let snapshot = loro_doc.export(loro::ExportMode::Snapshot).unwrap();

                // Create the peer map with the current peer
                let mut peer_map: HashMap<u64, String> = HashMap::new();
                peer_map.insert(loro_doc.peer_id(), "s/colabri-doc".to_string());

                // Put it in a ColabPackage
                // Create the ColabPackage to store in the database
                let colab_package = ColabPackage {
                    snapshot: snapshot.clone(),
                    peer_map: peer_map,
                };

                // Serialize the ColabPackage to CBOR
                let blob = match serde_cbor::to_vec(&colab_package) {
                    Ok(data) => data,
                    Err(e) => {
                        error!("Failed to serialize ColabPackage for document '{}': {}", doc_id, e);
                        return Err(format!("Failed to serialize ColabPackage: {}", e));
                    }
                };

                // Store the generated snapshot as a new stream in the database
                let docstream_id = match db.insert_statement_doc_stream(
                    &org_id,
                    doc_uuid,
                    blob
                ).await {
                    Ok(id) => id,
                    Err(e) => {
                        error!("Failed to insert document stream for document '{}': {}", doc_uuid.to_string(), e);
                        return Err(format!("Failed to insert document stream: {}", e));
                    }
                };

                // Create the peer map with the current peer
                let mut peer_map: HashMap<u64, String> = HashMap::new();
                peer_map.insert(loro_doc.peer_id(), "s/colabri-doc".to_string());

                // Create DocContext
                let context = DocContext {
                    org: org_id.clone(),
                    doc_id: doc_uuid.clone(),
                    doc_stream_id: docstream_id.clone(),
                    peer_map: peer_map,
                    last_updating_peer: Some(loro_doc.peer_id()),
                };

                return Ok(LoadedDoc { snapshot: Some(snapshot), ctx: Some(context) });
            }
            // No stream and no json
            else {
                error!("No content found for document '{}'", doc_uuid.to_string());
                return Err("No content found".to_string());
            }
        }
        // Import the content into the LoroDoc
        else {

            // Deserialize the CBOR formatted "main_stream_bytes" into a ColabPackage
            let colab_package : ColabPackage = match serde_cbor::from_slice(&main_stream_bytes.unwrap()) {
                Ok(pkg) => pkg,
                Err(e) => {
                    error!("Failed to deserialize ColabPackage for document '{}': {}", doc_uuid.to_string(), e);
                    return Err(format!("Failed to deserialize ColabPackage: {}", e));
                }
            };

            // Get the peer map
            let loro_snapshot = colab_package.snapshot;
            let peer_map = colab_package.peer_map;

            // Create DocContext
            let context = DocContext {
                org: org_id.clone(),
                doc_id: doc_uuid.clone(),
                doc_stream_id: main_stream.unwrap().id.clone(),
                peer_map: peer_map,
                last_updating_peer: None,
            };

            info!("Successfully loaded document: {} ({} bytes)", doc_uuid.to_string(), main_stream_bytes.unwrap().len());
            return Ok(LoadedDoc { snapshot: Some(loro_snapshot), ctx: Some(context) });
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
pub fn on_save_document(args: SaveDocArgs<DocContext>) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
    let doc_id = args.room;
    let crdt = args.crdt;
    let snapshot = args.data;
    let context = args.ctx.clone();
    Box::pin(async move {

        // Validate CRDT type
        if crdt != CrdtType::Loro {
            warn!("Unsupported CRDT type for saving document: {:?}", crdt);
            return Ok(());
        }

        // Start saving the loro document
        info!("Saving loro document for room: {}", doc_id);

        // Check if context is available
        let mut context = match context {
            Some(ctx) => ctx,
            None => {
                error!("No doc context available when saving for document: {}", doc_id);
                return Err("No doc context available when saving".to_string());
            }
        };

        // Get document identifiers
        let org = context.org.clone();
        let doc_uuid = context.doc_id.clone();
        let doc_stream_uuid = context.doc_stream_id.clone();

        // Get the principal that updated the document most recently
        let updating_peer_id = match context.last_updating_peer {
            Some(pid) => pid,
            None => {
                // No updating peer, nothing to save
                info!("Aborting save. No last updating peer found in context for document: {}", doc_uuid);
                return Ok(());
            }
        };
        let by_prpl = match context.peer_map.get(&updating_peer_id) {
            Some(prpl) => prpl.clone(),
            None => {
                error!("Error Saving. No principal found for updating peer {} in document: {}", updating_peer_id, doc_uuid);
                return Err("No principal found for updating peer".to_string());
            }
        };

        // Create the ColabPackage to store in the database
        let colab_package = ColabPackage {
            snapshot: snapshot.clone(),
            peer_map: context.peer_map.clone(),
        };

        // Serialize the ColabPackage to CBOR
        let blob = match serde_cbor::to_vec(&colab_package) {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to serialize ColabPackage for document '{}': {}", doc_id, e);
                return Err(format!("Failed to serialize ColabPackage: {}", e));
            }
        };

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
        match db.update_statement(&org, doc_uuid, doc_stream_uuid, blob, json, &by_prpl).await {
            Ok(_) => {
                info!("Statement updated successfully {}", doc_uuid);
            }
            Err(e) => {
                error!("Failed to update statement '{}': {}", doc_uuid, e);
                return Err(format!("Failed to update statement '{}': {}", doc_uuid, e));
            }
        }

        // Clear the last updating peer in the context
        context.last_updating_peer = None;

        // Call the app service sync endpoint to notify about the update
        if let Some(client) = app_service_client::get_app_service_client() {
            let client = client.clone();
            let org_clone = org.clone();
            let doc_uuid_clone = doc_uuid.clone();
            tokio::spawn(async move {
                match client.sync_document(&org_clone, &doc_uuid_clone).await {
                    Ok(_) => {
                        info!("Successfully notified app service about document update: {}", doc_uuid_clone);
                    }
                    Err(e) => {
                        error!("Failed to notify app service about document update '{}': {}", doc_uuid_clone, e);
                    }
                }
            });
        }

        return Ok(());
    })
}

/// Handle document updates
/// 
/// This function is called whenever a client sends updates to a document.
/// It should validate the updates.
pub fn on_update(args: UpdateArgs<DocContext>) -> Pin<Box<dyn Future<Output = UpdatedDoc<DocContext>> + Send + 'static>> {
    Box::pin(async move {
        
        // Get the connection ID
        let conn_id = args.conn_id;
        let room_id = args.room;

        // We're currently only interested in Loro updates
        if args.crdt != CrdtType::Loro {
            return UpdatedDoc {
                status: UpdateStatusCode::Ok,
                ctx: args.ctx,
                doc: None,
            };
        }

        // Check if the UID of this peer matches the current UID of this connection
        let mut doc_ctx = match args.ctx {
            Some(ctx) => ctx,
            None => {
                error!("When updating document: No context available for document: {} ({} updates)", room_id, args.updates.len());
                return UpdatedDoc {
                    status: UpdateStatusCode::Unknown,
                    ctx: None,
                    doc: None,
                };
            }
        };

        // Figure out which user is behind this connection
        let conn_ctx_cache = get_conn_ctx_cache();
        let conn_ctx = match conn_ctx_cache.get(&conn_id) {
            Some(ctx) => ctx,
            None => {
                error!("No connection context found for connection_id: {}", conn_id);
                return UpdatedDoc {
                    status: UpdateStatusCode::PermissionDenied,
                    ctx: Some(doc_ctx),
                    doc: None,
                };
            }
        };
        let uid = &conn_ctx.uid;
        info!("Received update from user: {} on doc: {}", uid, room_id);

        // Ensure we have a loro document
        let loro_doc = match args.doc {
            Some(ref doc) => doc,
            None => {
                error!("No LoroDoc available while processing update for doc: {}", room_id);
                return UpdatedDoc {
                    status: UpdateStatusCode::Unknown,
                    ctx: Some(doc_ctx),
                    doc: None,
                };
            }
        };

        // Get the initial peers in the document
        let init_version_vector = loro_doc.oplog_vv();

        // Apply the updates
        let _ = loro_doc.import_batch(&args.updates);

        // Get the updated version vector
        let updated_version_vector = loro_doc.oplog_vv();

        // Figure out which peer did the update by comparing the version vectors
        let mut updating_peer: Option<u64> = None;
        for peer_id in updated_version_vector.keys().cloned() {
            let updated_version = updated_version_vector.get(&peer_id).unwrap();
            let init_version = init_version_vector.get(&peer_id).cloned().unwrap_or(0);
            if updated_version > &init_version {
                updating_peer = Some(peer_id);
                break;
            }
        }

        // Make sure we found the updating peer
        let updating_peer_id = match updating_peer {
            Some(pid) => pid,
            None => {
                info!("Update resulted in no operations for doc: {}", room_id);
                return UpdatedDoc {
                    status: UpdateStatusCode::Ok,
                    ctx: Some(doc_ctx),
                    doc: Some(loro_doc.clone()),
                };
            }
        };

        // Check if this peer is already known in the peer_map in the document context
        let peer_map = &mut doc_ctx.peer_map;
        let ok_peer = match peer_map.get(&updating_peer_id) {
            Some(found_prpl) => {
                // Generate the prpl for this user
                let allowed_prpl = format!("{}/u/{}", conn_ctx.org_id, uid);
                if *found_prpl != allowed_prpl {
                    false
                } else {
                    true
                }
            }
            None => {
                // No principal found for this peer, that's fine just add it.
                info!("Adding new peer {} for user {} in document {}", updating_peer_id, uid, room_id);
                peer_map.insert(updating_peer_id, format!("{}/u/{}", conn_ctx.org_id, uid));
                true
            },
        };

        // If the peer was not ok, reject the update
        if !ok_peer {
            error!("User {} attempted to update document {} with invalid peer {}", uid, room_id, updating_peer_id);
            return UpdatedDoc {
                status: UpdateStatusCode::PermissionDenied,
                ctx: Some(doc_ctx),
                doc: None,
            };
        }

        // Update the last updating peer in the document context
        info!("User {} updated document {} with peer {}", uid, room_id, updating_peer_id);
        doc_ctx.last_updating_peer = Some(updating_peer_id);

        // Return OK
        return UpdatedDoc {
            status: UpdateStatusCode::Ok,
            ctx: Some(doc_ctx),
            doc: Some(loro_doc.clone()),
        };
    })
}