use tokio::sync::{RwLock, broadcast};
use crate::models::{ColabDoc, BroadcastUpdateMessage};

#[derive(Debug)]
pub struct ColabDocSession {
    pub id: String,
    pub doc: RwLock<ColabDoc>,
    pub broadcast: RwLock<broadcast::Sender<BroadcastUpdateMessage>>,   
}