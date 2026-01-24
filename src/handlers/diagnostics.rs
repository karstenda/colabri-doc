use crate::{auth::auth, models::{DiagnosticsResponse, ErrorResponse}, ws::{docctx::DocContext, userctx}};
use axum::{extract::{State, Extension}, http::StatusCode, Json};
use loro_websocket_server::{HubRegistry};
use std::sync::Arc;
use loro_protocol::protocol::CrdtType;
use std::sync::{Mutex, OnceLock};
use sysinfo::System;
use tracing::info;

static SYSTEM_MONITOR: OnceLock<Mutex<System>> = OnceLock::new();

/// Export a document
pub async fn diagnostics(
    State(registry): State<Arc<HubRegistry<DocContext>>>,
    Extension(prpls): Extension<Vec<String>>,
) -> Result<(StatusCode, Json<DiagnosticsResponse>), (StatusCode, Json<ErrorResponse>)> {

    // Ensure the user is an org member or service
    let _ = auth::ensure_cloud_admin(&prpls)?;

    // Aggregate diagnostics from the registry
    let mut n_conn: u32 = 0;
    let mut n_rooms: u32 = 0;
    let mut n_doc_rooms: u32 = 0;
    let mut n_ephemeral_rooms: u32 = 0;
    let mut n_dirty_docs: u32 = 0;
    let hubs = registry.hubs().lock().await;
    for (_, hub) in hubs.iter() {
        let h = hub.lock().await;
        for (room_key, doc_state) in h.docs.iter() {
            n_rooms += 1;
            if room_key.crdt == CrdtType::Loro {
                n_doc_rooms += 1;
            }
            if room_key.crdt == CrdtType::LoroEphemeralStore {
                n_ephemeral_rooms += 1;
            }
            if doc_state.dirty {
                n_dirty_docs += 1;
            }
            n_conn += h.subs.get(room_key).map_or(0, |subs_set| subs_set.len()) as u32;
        }
    }

    // Get the user contexts count
    let n_user_ctx = userctx::get_user_ctx_cache().entry_count() as u32;

    // System stats
    let (cpu_usage, memory_alloc, memory_free, memory_total) = {
        let sys_lock = SYSTEM_MONITOR.get_or_init(|| {
            Mutex::new(System::new_all())
        });
        match sys_lock.lock() {
            Ok(mut sys) => {
                sys.refresh_cpu();
                sys.refresh_memory();
                (
                    sys.global_cpu_info().cpu_usage(),
                    sys.used_memory(),
                    sys.free_memory(),
                    sys.total_memory(),
                )
            }
            Err(_) => (0.0, 0, 0, 0)
        }
    };

    info!(
        "Diagnostics: CPU: {:.2}%, Mem: {}/{} MB (Free: {} MB), Conn: {}, Rooms: {}",
        cpu_usage,
        memory_alloc / 1024 / 1024,
        memory_total / 1024 / 1024,
        memory_free / 1024 / 1024,
        n_conn,
        n_rooms
    );

    return Ok((
        StatusCode::OK,
        Json(DiagnosticsResponse {
            n_conn,
            n_rooms,
            n_doc_rooms,
            n_ephemeral_rooms,
            n_dirty_docs,
            n_user_ctx,
            cpu_usage,
            memory_alloc,
            memory_total,
            memory_free,
        }),
    ));
}
