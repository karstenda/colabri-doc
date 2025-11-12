use tracing::{info, error};
use tokio::sync::broadcast::Sender;
use crate::models::{UpdateMessage, BroadcastMessage};

/// Handle UpdateMessage
pub async fn handle_update_message(update_msg: &UpdateMessage, document_id: String, connection_id: String, bc: &Sender<BroadcastMessage>) {

    // Log the update message
    info!("Update message received for document {}: user={}, peer={}", document_id, update_msg.user, update_msg.peer);

    // Handle update message - Check, Apply and Broadcast
    let broadcast_msg = BroadcastMessage {
        sender_id: connection_id,
        content: serde_json::to_string(update_msg).unwrap(),
    };
    
    if let Err(e) = bc.send(broadcast_msg) {
        error!("Failed to broadcast for {document_id}: {e}");
    }
}