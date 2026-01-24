use crate::{handlers::doc_export, handlers::diagnostics, ws::docctx::DocContext, routes::auth_middleware::auth_middleware};
use axum::{routing::get, Router, middleware};
use loro_websocket_server::HubRegistry;
use std::sync::Arc;

/// Create API routes
pub fn create_api_routes(registry: Arc<HubRegistry<DocContext>>) -> Router {
    Router::<Arc<HubRegistry<DocContext>>>::new()
        .route("/v1/diagnostics", get(diagnostics))
        .route("/v1/:org_id/documents/:doc_id/export", get(doc_export))
        .route_layer(middleware::from_fn(auth_middleware)) // Applies to all routes added above
        .with_state(registry)
}
