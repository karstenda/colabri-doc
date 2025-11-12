use tracing::{info, error};
use loro::LoroDoc;
use std::sync::Arc;
use axum::{
    extract::ws::{Message, WebSocket},
};
use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use tokio::sync::Mutex;
use crate::models::{LoadMessage, SendMessage, SerializedColabDoc};
use crate::AppState;

/// Handle LoadMessage
pub async fn handle_load_message(load_msg: &LoadMessage, document_id: String, sender: &Arc<Mutex<SplitSink<WebSocket, Message>>>, app_state: &Arc<AppState>) {

    // Handle load message - Load the document and send back
    info!("Load message received for document {}: user={}, peer={}", document_id, load_msg.user, load_msg.peer);


    // Load the document from the session
    let loro_snapshot: Vec<u8>;
    let docsessions_read = app_state.docsessions.read().await;
    if let Some(docsession) = docsessions_read.get(&document_id) {
        let colab_doc = docsession.doc.write().await;
        let loro_doc = &colab_doc.loro_doc;

        // Create a snapshot of the loro document.
        let snapshot = match loro_doc.export(loro::ExportMode::Snapshot) {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to export LoroDoc for document {}: {}", document_id, e);
                return;
            }
        };
        loro_snapshot = snapshot;
    } else {
        error!("Document session not found for {}", document_id);
        return;
    }

    // Create a ColabDoc
    let colab_doc = SerializedColabDoc {
        name: document_id.clone(),
        id: document_id.clone(),
        loro_doc: loro_snapshot,
    };

    // Send Init message back to client
    let init_msg = SendMessage::Init(crate::models::messages::InitMessage { colab_doc });
    let init_msg_text = serde_json::to_string(&init_msg).unwrap();

    if sender.lock().await.send(Message::Text(init_msg_text)).await.is_err() {
        error!("Failed to send Init message for document {}", document_id);
        return;
    }
}