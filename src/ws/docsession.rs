#[derive(Clone, Debug)]
pub struct DocSession {
    pub org: String,
    pub doc_id: uuid::Uuid,
    pub doc_stream_id: uuid::Uuid,
}