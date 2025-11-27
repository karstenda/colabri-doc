mod models;
mod handlers;
mod routes;
mod docs;
// mod websocket; // No longer needed - using loro-websocket-server directly
mod config;
mod db;
mod ws;

use axum::Router;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use routes::create_api_routes;
use docs::ApiDoc;
use config::Config;
use tracing::{info, error, warn};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use std::panic;
use loro_websocket_server::ServerConfig;

#[tokio::main(flavor = "current_thread")]
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

    // Initialize database connection if URL is provided
    if let Some(db_url) = &config.db_url {
        match db::dbcolab::init_db(db_url).await {
            Ok(_) => info!("Database initialized successfully"),
            Err(e) => {
                error!("Failed to initialize database: {}", e);
                warn!("WebSocket document loading will not be available");
            }
        }
    } else {
        warn!("No database URL configured - WebSocket document loading will not be available");
    }

    // Create API routes
    let api_routes = create_api_routes();

    // Combine all routes
    let app_routes = Router::new()
        // Mount API routes
        .nest("/api", api_routes)
        // Mount Swagger UI
        .merge(SwaggerUi::new("/swagger").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Add tracing layer
        .layer(TraceLayer::new_for_http());

    // Configure loro-websocket-server
    let ws_port = config.websocket_port();
    let ws_addr = format!("{}:{}", config.host, ws_port);
    let ws_config = ServerConfig {
        on_load_document: Some(std::sync::Arc::new(ws::wscolab::on_load_document)),
        on_save_document: None, // TODO: Implement document saving
        save_interval_ms: Some(30_000), // Save every 30 seconds
        default_permission: loro_websocket_server::protocol::Permission::Write,
        authenticate: None, // TODO: Implement authentication if needed
        handshake_auth: None, // TODO: Implement handshake auth if needed
    };

    // Start WebSocket server
    let ws_listener = tokio::net::TcpListener::bind(&ws_addr)
        .await
        .unwrap_or_else(|_| panic!("Failed to bind WebSocket server to {}", ws_addr));
    
    info!("📡 WebSocket server starting on ws://{}", ws_addr);
    
    // Spawn WebSocket server task
    tokio::spawn(async move {
        if let Err(e) = loro_websocket_server::serve_incoming_with_config(ws_listener, ws_config).await {
            error!("WebSocket server error: {}", e);
        }
    });

    // Start the HTTP/API server
    let listener = tokio::net::TcpListener::bind(config.server_address())
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to {}", config.server_address()));
        
    info!("🚀 Server running on http://{}", config.server_address());
    info!("📡 WebSocket available at ws://{}", ws_addr);
    info!("📚 Swagger UI available at http://{}/swagger", config.server_address());

    axum::serve(listener, app_routes)
        .await
        .expect("Server failed to start");
}