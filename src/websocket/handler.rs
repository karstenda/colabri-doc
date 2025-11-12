
use std::sync::Arc;
use axum::{
    extract::{Path, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::Response,
};
use loro::LoroDoc;
use tokio::sync::broadcast;
use tracing::{info, error};
use futures_util::{StreamExt, SinkExt};
use uuid::Uuid;

use crate::{AppState, models::{ColabDoc, ColabDocSession}, websocket::msg_update_handler::handle_update_message};
use crate::models::{ReceivedMessage, BroadcastUpdateMessage};
use crate::websocket::msg_load_handler::handle_load_message;
use crate::websocket::msg_ping_handler::handle_ping_message;


/// WebSocket handler
pub async fn websocket_handler(
    Path(document_id): Path<String>,
    ws: WebSocketUpgrade,
    app_state: axum::extract::State<Arc<AppState>>,
) -> Response {
    info!("New WebSocket connection attempt");
    ws.on_upgrade(move |socket| handle_socket(socket, document_id, app_state.0))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, document_id: String, app_state: Arc<AppState>) {
    
    // Generate unique connection ID to identify this client
    let connection_id = Uuid::new_v4().to_string();
    let listen_connection_id = connection_id.clone();
    let broadcast_connection_id = connection_id.clone();
    let cleanup_connection_id = connection_id.clone();

    // Clone variables for cleanup (before they get moved into tasks)
    let listen_document_id = document_id.clone();
    let broadcast_document_id = document_id.clone();
    let cleanup_document_id = document_id.clone();

    // Log connection establishment
    info!("WebSocket connection established for document_id: {} with connection_id: {}", document_id, connection_id);
    // Split the socket into sender and receiver
    let (sender, mut receiver) = socket.split();

    // As we will need a reference to sender in multiple tasks, wrap it in an Arc and Mutex
    let sender1 = Arc::new(tokio::sync::Mutex::new(sender));
    let sender2 = sender1.clone();

    // Get or create broadcast channel for this document
    // First, ensure the document session exists
    {
        let mut docsessions = app_state.docsessions.write().await;
        docsessions
            .entry(document_id.clone())
            .or_insert_with(|| {
                ColabDocSession {
                    id: document_id.clone(),
                    doc: tokio::sync::RwLock::new(ColabDoc {
                        name: "test".to_string(),
                        id: document_id.clone(),
                        loro_doc: {
                            let loro_doc = LoroDoc::new();
                            loro_doc.get_text("text");
                            loro_doc
                        }

                    }),
                    broadcast: {
                        let (bc, _rx) = broadcast::channel::<BroadcastUpdateMessage>(100);
                        tokio::sync::RwLock::new(bc)
                    },
                    active_connections: tokio::sync::RwLock::new(std::collections::HashSet::new()),
                }
            });
    } // Drop the write lock here
    
    // Add this connection to the active connections
    {
        let docsessions = app_state.docsessions.read().await;
        if let Some(session) = docsessions.get(&document_id) {
            let mut active_connections = session.active_connections.write().await;
            active_connections.insert(connection_id.clone());
            info!("Added connection {} for document {}. Total active connections: {}", 
                  connection_id, document_id, active_connections.len());
        }
    }
    
    // Now get a reference to the broadcast receiver
    let mut rbc = {
        let docsessions_read = app_state.docsessions.read().await;
        let docsession = docsessions_read.get(&document_id).unwrap();
        let bc = docsession.broadcast.read().await;
        bc.subscribe()
    }; // All locks are dropped here

    // Start an async task to listen to the websocket for incoming messages
    // Does this as a separate asynchronous task
    let app_state_ref = app_state.clone();
    let mut send_task = tokio::spawn(async move {

        // Listen for incoming messages
        // Use pattern matching to only process text messages
        // ❌ Binary message arrives → Pattern doesn't match, loop continues to next iteration
        // ❌ Error occurs → Pattern doesn't match, loop continues to next iteration
        // ❌ Stream ends (None) → Pattern doesn't match, loop exits
        // ✅ Text message arrives → Pattern matches, process the message
        while let Some(Ok(Message::Text(msg))) = receiver.next().await {

            // Parse the incoming message as JSON
            let json_msg: ReceivedMessage = match serde_json::from_str(&msg) {
                Ok(json_msg) => {
                    info!("Received message for document {}: {:?}", listen_document_id, json_msg);
                    json_msg
                }
                Err(e) => {
                    error!("Failed to parse message for document {}: {}", listen_document_id, e);
                    continue;
                }
            };

            // Handle different message types
            match json_msg {
                ReceivedMessage::Load(load_msg) => {
                    handle_load_message(&load_msg, document_id.clone(), &sender1, &app_state_ref).await;
                    continue;
                }
                ReceivedMessage::Update(update_msg) => {
                    handle_update_message(&update_msg, listen_document_id.clone(), listen_connection_id.clone(), &app_state_ref).await;
                    continue;
                }
                ReceivedMessage::Ping(ping_msg) => {
                    handle_ping_message(&ping_msg, listen_document_id.clone(), &sender1).await;
                    continue;

                }
            }
        }
    });

    // Start a task to monitor whether there are broadcast messages and send to client
    let mut recv_task = tokio::spawn(async move {
        while let Ok(broadcast_msg) = rbc.recv().await {
            
            // Skip messages from this connection to prevent echo
            if broadcast_msg.sender_id == broadcast_connection_id {
                continue;
            }

            // Serialize the update message and send to client
            let complete_msg = ReceivedMessage::Update(broadcast_msg.update);
            let update_msg_text = serde_json::to_string(&complete_msg).unwrap();
            
            // Send the message
            if sender2.lock().await.send(Message::Text(update_msg_text)).await.is_err() {
                break;
            }
        }
    });

    // Wait for either task to finish (and finish the other)
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
    
    // Cleanup: Remove this connection and potentially the entire session
    cleanup_connection(&cleanup_document_id, &cleanup_connection_id, &app_state).await;
    
    // Log disconnection
    info!("WebSocket connection terminated for document {} connection {}", cleanup_document_id, cleanup_connection_id);
    
}

/// Cleanup connection and potentially remove the entire document session
async fn cleanup_connection(document_id: &str, connection_id: &str, app_state: &Arc<AppState>) {
    let should_remove_session = {
        let docsessions = app_state.docsessions.read().await;
        if let Some(session) = docsessions.get(document_id) {
            // Remove this connection from active connections
            let mut active_connections = session.active_connections.write().await;
            active_connections.remove(connection_id);
            
            let remaining_connections = active_connections.len();
            info!("Removed connection {} for document {}. Remaining connections: {}", 
                  connection_id, document_id, remaining_connections);
            
            // Check if this was the last connection
            remaining_connections == 0
        } else {
            false
        }
    };
    
    // If no more connections, remove the entire document session
    if should_remove_session {
        let mut docsessions = app_state.docsessions.write().await;
        docsessions.remove(document_id);
        info!("Removed document session for {} (no active connections)", document_id);
    }
}