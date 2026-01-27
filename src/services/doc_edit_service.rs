use std::sync::Arc;
use loro_protocol::CrdtType;
use loro_websocket_server::{HubRegistry, RoomKey};
use tracing::error;
use loro::LoroDoc;
use crate::ws::docctx::DocContext;

// Create a defer close struct to close the document room when dropped
struct DeferClose {
    registry: Arc<HubRegistry<DocContext>>,
    org_id: String,
    doc_id: String,
}

impl Drop for DeferClose {
    fn drop(&mut self) {
        let registry = self.registry.clone();
        let org_id = self.org_id.clone();
        let doc_id = self.doc_id.clone();
        tokio::spawn(async move {
            if !registry.close_room(&org_id, CrdtType::Loro, &doc_id, false).await {
                error!("Failed to close document '{}'", doc_id);
            }
        });
    }
}

// Edit a document by opening it in the Hub, applying the edit_callback, and then making sure to closie it
pub async fn edit_doc(registry: Arc<HubRegistry<DocContext>>, org_id: &str, doc_id: &str, edit_callback: impl FnOnce(&LoroDoc) -> Result<(), String>) -> Result<(), String> {

    // Open the document room
    registry.open_room(&org_id, CrdtType::Loro, &doc_id).await;

    // Whatever happens, make sure to close the document room after we're done
    let _defer_close = DeferClose {
        registry: registry.clone(),
        org_id: org_id.to_string(),
        doc_id: doc_id.to_string(),
    };

    // Target the document in the Hub and apply the edit callback
    {
        let hubs = registry.hubs().lock().await;
        if let Some(hub) = hubs.get(org_id) {
            let h = hub.lock().await;
            if let Some(doc_state) = h.docs.get(&RoomKey {crdt: CrdtType::Loro, room: doc_id.to_string()}) {
                if let Some(doc) = doc_state.doc.get_loro_doc() {
                    return edit_callback(&doc);
                }
            }
        }
    }

    Ok(())
}