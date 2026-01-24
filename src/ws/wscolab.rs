use loro::{ LoroDoc, ToJson};
use loro_protocol::{CrdtType, UpdateStatusCode};
use loro_websocket_server::{AuthArgs, CloseConnectionArgs, HandshakeAuthArgs, LoadDocArgs, LoadedDoc, SaveDocArgs, UpdateArgs, UpdatedDoc};
use loro_websocket_server::protocol::Permission;
use tracing::{info, warn, error};
use uuid::Uuid;
use std::{collections::HashMap, pin::Pin};
use std::future::Future;
use serde_cbor;
use crate::routes::auth_middleware::validate_jwt;

use crate::models::ColabPackage;
use crate::{db::dbcolab, clients::app_service_client };
use super::docctx::{DocContext};
use super::userctx::{self};
use super::connctx::{self, ConnCtx};

fn user_has_org_access(principals: &[String], org_id: &str) -> bool {
    let org_prefix = format!("{}/u/", org_id);
    principals.iter().any(|principal| {
        principal.starts_with(&org_prefix) || principal == "r/Colabri-CloudAdmin"
    })
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
        match validate_jwt(token, secret) {
            Ok(token_data) => {
                if let Some(uid) = token_data.claims.get("sub").and_then(|v| v.as_str()) {
                    info!("JWT token validated successfully for user: {}", uid);
                    
                    return match userctx::get_or_fetch_user_ctx_blocking(uid) {
                        Ok(user_ctx) => {
                            if !user_has_org_access(&user_ctx.principals, &org_id) {
                                error!("User {} does not have access to organization {}", uid, org_id);
                                false
                            } else {
                                let conn_ctx = ConnCtx {
                                    uid: uid.to_string(),
                                    org_id: org_id.to_string(),
                                };
                                let conn_ctx_cache = connctx::get_conn_ctx_cache();
                                conn_ctx_cache.insert(args.conn_id, conn_ctx);
                                true
                            }
                        }
                        Err(e) => {
                            error!("Failed to load user context for {}: {}", uid, e);
                            false
                        }
                    };
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
        let conn_ctx_cache = connctx::get_conn_ctx_cache();
        let conn_ctx = match conn_ctx_cache.get(&args.conn_id) {
            Some(ctx) => ctx,
            None => {
                error!("No connection context found for connection_id: {}", args.conn_id);
                return Err("No connection context found".to_string());
            }
        };

        let uid_for_fetch = conn_ctx.uid.clone();
        let org_for_fetch = conn_ctx.org_id.clone();
        let user_ctx = match userctx::get_or_fetch_user_ctx_async(&uid_for_fetch).await {
            Ok(ctx) => ctx,
            Err(e) => {
                error!("Unable to load user context for uid {}: {}", conn_ctx.uid, e);
                return Err(e);
            }
        };
        if !user_has_org_access(&user_ctx.principals, &org_for_fetch) {
            error!("User {} does not have access to organization {}", conn_ctx.uid, org_for_fetch);
            return Err("User lacks access to organization".to_string());
        }

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
        let conn_ctx_cache = connctx::get_conn_ctx_cache();
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
        match crate::services::doc_service::fetch_doc_snapshot_from_db(&org_id, &doc_id).await {
            Ok(Some((snapshot, ctx))) => Ok(LoadedDoc { snapshot: Some(snapshot), ctx: Some(ctx) }),
            Ok(None) => Ok(LoadedDoc { snapshot: None, ctx: None }),
            Err(e) => Err(e),
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

        // Convert snapshot to JSON for storage in statement
        let loro_doc = LoroDoc::new();
        if let Err(e) = loro_doc.import(&snapshot) {
            error!("Failed to import snapshot for document '{}': {}", doc_uuid, e);
            return Err(format!("Failed to import snapshot for document '{}': {}", doc_uuid, e));
        }

        // Get the JSON representations
        let loro_value = loro_doc.get_deep_value();
        let json = loro_value.to_json_value();
        let state_vv = loro_doc.state_vv();
        let state_vv_json = match serde_json::to_value(&state_vv) {
            Ok(val) => val,
            Err(e) => {
                error!("Failed to serialize state_vv for document '{}': {}", doc_uuid, e);
                return Err(format!("Failed to serialize state_vv: {}", e));
            }
        };
        let peer_map_json = match serde_json::to_value(&context.peer_map.clone()) {
            Ok(val) => val,
            Err(e) => {
                error!("Failed to serialize peer_map for document '{}': {}", doc_uuid, e);
                return Err(format!("Failed to serialize peer_map: {}", e));
            }
        };

        // Figure out the type of ColabDocument
        let doc_type: String = json.get("properties").and_then(|props| props.get("type")).and_then(|t| t.as_str()).map(|s| s.to_string()).ok_or_else(|| {
            error!("Document '{}' is missing 'properties.type' field", doc_uuid);
            "Document is missing 'properties.type' field".to_string()
        })?;
        
        // Get database connection
        let db = match dbcolab::get_db() {
            Some(db) => db,
            None => {
                error!("Database not initialized, cannot save document: {}", doc_uuid);
                return Err("Database not initialized".to_string());
            }
        };

        // Save to database with incremented version
        match db.update_colab_doc(&org, doc_uuid, &doc_type, doc_stream_uuid, blob, json, state_vv_json, peer_map_json, &by_prpl).await {
            Ok(_) => {
                info!("Statement updated successfully {}", doc_uuid);
            }
            Err(e) => {
                error!("Failed to update statement '{}': {}", doc_uuid, e);
                return Err(format!("Failed to update statement '{}': {}", doc_uuid, e));
            }
        };        

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
        let org_id = args.workspace;

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
        let is_system_update = conn_id == 0;
        let conn_ctx_cache = connctx::get_conn_ctx_cache();
        let by_prpl: String;
        let user_uid: Option<String>;
        let user_prpls: Vec<String>;
        if !is_system_update {
            let conn_ctx= match conn_ctx_cache.get(&conn_id) {
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
            let uid: String = conn_ctx.uid.clone();
            let conn_org = conn_ctx.org_id.clone();
            info!("Received update from user: {} on doc: {}", uid, room_id);

            let user_ctx = match userctx::get_or_fetch_user_ctx_async(&uid).await {
                Ok(ctx) => ctx,
                Err(e) => {
                    error!("Unable to load user context for uid {}: {}", uid, e);
                    return UpdatedDoc {
                        status: UpdateStatusCode::PermissionDenied,
                        ctx: Some(doc_ctx),
                        doc: None,
                    };
                }
            };
            if !user_has_org_access(&user_ctx.principals, &conn_org) {
                error!("User {} does not have access to organization {}", uid, conn_org);
                return UpdatedDoc {
                    status: UpdateStatusCode::PermissionDenied,
                    ctx: Some(doc_ctx),
                    doc: None,
                };
            }
            user_prpls = user_ctx.principals.clone();
            user_uid = Some(uid.clone());
            by_prpl = match user_ctx.get_user_principal(&org_id) {
                Some(prpl) => prpl,
                None => {
                    error!("No principal found for user {} in organization {}", uid, org_id);
                    return UpdatedDoc {
                        status: UpdateStatusCode::PermissionDenied,
                        ctx: Some(doc_ctx),
                        doc: None,
                    };
                }
            };
        } else {
            info!("Received update from system");
            by_prpl = "s/colabri-system".to_string();
            user_prpls = vec!["s/colabri-system".to_string()];
            user_uid = None;
        }    

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
                // Check if this principal is one of the user principals
                if !user_prpls.contains(found_prpl) {
                    false
                } else {
                    true
                }
            }
            None => {
                // No principal found for this peer, that's fine just add it.
                info!("Adding new peer {} for prpl {} in document {}", updating_peer_id, by_prpl, room_id);
                peer_map.insert(updating_peer_id, by_prpl.clone());
                true
            },
        };

        // If the peer was not ok, reject the update
        if !ok_peer {
            if let Some(uid) = user_uid {
                error!("User {} attempted to update document {} with invalid peer {}", uid, room_id, updating_peer_id);
            } else {
                error!("System attempted to update document {} with invalid peer {}", room_id, updating_peer_id);
            }
            return UpdatedDoc {
                status: UpdateStatusCode::PermissionDenied,
                ctx: Some(doc_ctx),
                doc: None,
            };
        }

        // Update the last updating peer in the document context
        info!("Prpl {} updated document {} with peer {}", by_prpl, room_id, updating_peer_id);
        doc_ctx.last_updating_peer = Some(updating_peer_id);

        // Return OK
        return UpdatedDoc {
            status: UpdateStatusCode::Ok,
            ctx: Some(doc_ctx),
            doc: Some(loro_doc.clone()),
        };
    })
}