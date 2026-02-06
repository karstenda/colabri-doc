use std::sync::Arc;
use loro_protocol::CrdtType;
use loro_websocket_server::{HubRegistry, RoomKey};
use loro::LoroDoc;
use crate::ws::docctx::DocContext;
use tracing::{info};


// Edit a document by opening it in the Hub, applying the edit_callback, and then making sure to close it
pub async fn edit_doc(registry: Arc<HubRegistry<DocContext>>, org_id: &str, doc_id: &str, edit_callback: impl FnOnce(&LoroDoc) -> Result<(), String> + Send, force_close: bool) -> Result<(), String> {

    // Do the edit
    let edit_result = registry.edit_loro_doc(org_id, doc_id, edit_callback, Some(true)).await;
    let peer_id = match edit_result {
        Ok(peer_id) => peer_id,
        Err(e) => return Err(format!("Failed to edit document: {}", e)),
    };
    info!("Edited document {} in org {}, peer_id: {}", doc_id, org_id, peer_id);

    // Add the peer_id to the DocContext's peer_map with a value of "colabri-doc" to indicate that this edit was made by the colabri-doc service.
    // This way, when we look at the peer_map in the future, we can see which edits were made by the service and which were made by real users.
    {
        let hubs = registry.hubs().lock().await;
        if let Some(hub) = hubs.get(org_id) {
            let mut h = hub.lock().await;
            if let Some(doc_state) = h.docs.get_mut(&RoomKey { crdt: CrdtType::Loro, room: doc_id.to_string() }) {
                if let Some(ctx) = doc_state.ctx.as_mut() {
                    ctx.peer_map.insert(peer_id, "s/colabri-doc".to_string());
                }
            }
        }
    }
    info!("Updated the peer map for document {} in org {}, peer_id: {}, prpl: {}", doc_id, org_id, peer_id, "s/colabri-doc");

    // Close the room.
    registry.close_room(&org_id,  CrdtType::Loro, &doc_id, force_close).await;
    info!("Closed room for document {} in org {}, force_close: {}", doc_id, org_id, force_close);
    
    return Ok(());
}