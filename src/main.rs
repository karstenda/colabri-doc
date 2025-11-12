mod models;
mod handlers;
mod routes;
mod docs;
mod websocket;
mod config;

use axum::{
    Router,
    routing::get,
};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use routes::create_api_routes;
use docs::ApiDoc;
use websocket::websocket_handler;
use config::Config;
use tracing::{info, error, warn};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use tokio::sync::{RwLock};
use std::{panic, collections::HashMap, sync::Arc};
use models::ColabDocSession;

// Shared application state
type DocumentId = String;
struct AppState {
    docsessions: RwLock<HashMap<DocumentId, ColabDocSession>>,
}

#[tokio::main]
async fn main() {

    // Set panic hook for better error messages
    panic::set_hook(Box::new(|info| {
        eprintln!("PANIC: {info}");
    }));

    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // Default to info level, but allow debug for our app
            "colabri_doc=debug,tower_http=debug,axum::rejection=trace,info".into()
        }))
        .init();

    info!("Starting server...");

    // Load configuration
    let config = Config::load().unwrap_or_else(|e| {
        error!("Failed to load configuration: {}", e);
        warn!("Using default configuration");
        Config::default()
    });

    // Create application state
    let app_state = Arc::new(AppState {
        docsessions: RwLock::new(HashMap::new()),
    });

    // Create API routes
    let api_routes = create_api_routes();

    // Combine all routes
    let app_routes = Router::new()
        // Mount API routes
        .nest("/api", api_routes)
        // Mount WebSocket route
        .route("/ws/doc/:document_id", get(websocket_handler).with_state(app_state))
        // Mount Swagger UI
        .merge(SwaggerUi::new("/swagger").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Add tracing layer
        .layer(TraceLayer::new_for_http());

    // Start the server
    let listener = tokio::net::TcpListener::bind(config.server_address())
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to {}", config.server_address()));
        
    info!("🚀 Server running on http://{}", config.server_address());
    info!("📡 WebSocket available at ws://{}/ws/doc", config.server_address());
    info!("📚 Swagger UI available at http://{}/swagger", config.server_address());

    axum::serve(listener, app_routes)
        .await
        .expect("Server failed to start");
}