#[derive(Debug)]
pub struct ColabDocSession {
    pub id: String,
    pub doc: RwLock<ColabDoc>,
    pub broadcast: RwLock<broadcast::Sender<BroadcastMessage>>,   
}