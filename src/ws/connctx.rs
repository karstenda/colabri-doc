use moka::sync::Cache;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::info;

#[derive(Clone, Debug)]
pub struct ConnCtx {
    pub uid: String,
    pub org_id: String,
}

/// Global connection context cache
static CONN_CTX_CACHE: OnceLock<Cache<u64, ConnCtx>> = OnceLock::new();

/// Initialize the connection context cache.
/// Should be called once at startup.
pub fn init_conn_ctx_cache() {
    CONN_CTX_CACHE.get_or_init(|| {
        Cache::builder()
            .max_capacity(100_000)
            .time_to_idle(Duration::from_secs(3 * 60 * 60))
            .build()
    });
    info!("Connection context cache initialized");
}

/// Get the global connection context cache instance.
pub fn get_conn_ctx_cache() -> &'static Cache<u64, ConnCtx> {
    CONN_CTX_CACHE
        .get()
        .expect("Connection context cache not initialized. Call init_conn_ctx_cache() first.")
}
