use crate::{handlers::{doc_latest, doc_version, doc_move_lib, doc_delete, diagnostics}, ws::docctx::DocContext, routes::auth_middleware::auth_middleware};
use axum::{routing::{get, post, delete}, Router, middleware};
use loro_websocket_server::HubRegistry;
use std::sync::Arc;

/// Create API routes
pub fn create_api_routes(registry: Arc<HubRegistry<DocContext>>) -> Router {
    Router::<Arc<HubRegistry<DocContext>>>::new()
        .route("/v1/diagnostics", get(diagnostics))
        .route("/v1/:org_id/documents/:doc_id", get(doc_latest))
        .route("/v1/:org_id/documents/:doc_id/version", post(doc_version))
        .route("/v1/:org_id/documents/:doc_id/move-lib", post(doc_move_lib))
        .route("/v1/:org_id/documents/:doc_id", delete(doc_delete))
        .route_layer(middleware::from_fn(auth_middleware)) // Applies to all routes added above
        .with_state(registry)
}
