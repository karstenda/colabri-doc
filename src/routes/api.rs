use crate::{handlers::{doc_export, doc_clear_acl, diagnostics}, ws::docctx::DocContext, routes::auth_middleware::auth_middleware};
use axum::{routing::{get, post}, Router, middleware};
use loro_websocket_server::HubRegistry;
use std::sync::Arc;

/// Create API routes
pub fn create_api_routes(registry: Arc<HubRegistry<DocContext>>) -> Router {
    Router::<Arc<HubRegistry<DocContext>>>::new()
        .route("/v1/diagnostics", get(diagnostics))
        .route("/v1/:org_id/documents/:doc_id/export", get(doc_export))
        .route("/v1/:org_id/documents/:doc_id/clear-acl", post(doc_clear_acl))
        .route_layer(middleware::from_fn(auth_middleware)) // Applies to all routes added above
        .with_state(registry)
}
