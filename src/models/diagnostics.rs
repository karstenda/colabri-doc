
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response for diagnostics information
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DiagnosticsResponse {
    pub n_conn: u32,
    pub n_rooms: u32,
    pub n_doc_rooms: u32,
    pub n_ephemeral_rooms: u32,
    pub n_dirty_docs: u32,
    pub n_user_ctx: u32,
    pub cpu_usage: f32,
    pub memory_alloc: u64,
    pub memory_total: u64,
    pub memory_free: u64,
}