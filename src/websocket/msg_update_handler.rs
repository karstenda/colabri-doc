use std::sync::Arc;
use tracing::{info, error};
use crate::{AppState, models::{UpdateMessage, BroadcastUpdateMessage}};

/// Handle UpdateMessage
pub async fn handle_update_message(update_msg: &UpdateMessage, document_id: String, connection_id: String, app_state: &Arc<AppState>) {

    // Log the update message
    info!("Update message received for document {}: user={}, peer={}", document_id, update_msg.user, update_msg.peer);

    // Get the delta
    let delta = &update_msg.delta;
    
    // Update the document
    let docsessions_read = app_state.docsessions.read().await;
    if let Some(docsession) = docsessions_read.get(&document_id) {
        let colab_doc = docsession.doc.write().await;
        let loro_doc = &colab_doc.loro_doc;

        // Apply the delta to the loro document
        match loro_doc.import(&delta) {
            Ok(_) => {
                info!("Successfully applied delta to document {}", document_id);
            }
            Err(e) => {
                error!("Failed to apply delta to document {}: {}", document_id, e);
                return;
            }
        }
    } else {
        error!("Document session not found for {}", document_id);
        return;
    }

    // Create an update message to broadcast
    let broadcast_msg = BroadcastUpdateMessage {
        sender_id: connection_id,
        update: update_msg.clone(),
    };

    // Do the actual broadcasting
    let docsessions_read = app_state.docsessions.read().await;
    if let Some(docsession) = docsessions_read.get(&document_id) {
        let bc = docsession.broadcast.read().await;
        if let Err(e) = bc.send(broadcast_msg) {
            error!("Failed to broadcast for {document_id}: {e}");
        }
    } else {
        error!("Document session not found for {document_id}");
    }
}