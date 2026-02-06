use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct DocContext {
    pub org: String,
    pub doc_id: uuid::Uuid,
    pub doc_stream_id: uuid::Uuid,
    pub doc_version: u32,
    pub doc_owner: String,
    pub peer_map: HashMap<u64, String>,
    pub last_updating_peer: Option<u64>,
}
