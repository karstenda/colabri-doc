use tokio::sync::{RwLock, broadcast};
use std::collections::HashSet;
use crate::models::{ColabDoc, BroadcastUpdateMessage};

#[derive(Debug)]
pub struct ColabDocSession {
    pub id: String,
    pub doc: RwLock<ColabDoc>,
    pub broadcast: RwLock<broadcast::Sender<BroadcastUpdateMessage>>,
    pub active_connections: RwLock<HashSet<String>>, // Track active connection IDs
}