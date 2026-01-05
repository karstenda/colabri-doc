mod docs;
mod handlers;
mod models;
mod routes;
// mod websocket; // No longer needed - using loro-websocket-server directly
mod clients;
mod config;
mod db;
mod ws;

use axum::Router;
use config::Config;
use docs::ApiDoc;
use loro_websocket_server::ServerConfig;
use routes::create_api_routes;
use std::panic;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

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
    let app_config = Config::load().unwrap_or_else(|e| {
        error!("Failed to load configuration: {}", e);
        warn!("Using default configuration");
        Config::default()
    });

    // Initialize global configuration
    if let Err(e) = config::init_config(app_config) {
        error!("Failed to initialize global configuration: {}", e);
        return;
    }

    let config = config::get_config();

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

    // Initialize user context cache
    ws::wscolab::init_user_ctx_cache();

    // Initialize connection context cache
    ws::wscolab::init_conn_ctx_cache();

    // Initialize App Service Client
    if let Some(secret) = &config.cloud_auth_jwt_secret {
        if let Err(e) = clients::app_service_client::init_app_service_client(
            config.app_service_url(),
            secret.clone(),
            config.cloud_service_name.clone(),
        ) {
            error!("Failed to initialize AppServiceClient: {}", e);
        } else {
            info!("AppServiceClient initialized successfully");
        }
    } else {
        warn!("cloud_auth_jwt_secret not configured - AppServiceClient not initialized");
    }

    // Create API routes
    let api_routes = create_api_routes();

    // Combine all routes
    let app_routes = Router::new()
        .route("/health", axum::routing::get(handlers::health_check))
        .route("/ready", axum::routing::get(handlers::ready_check))
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
        on_save_document: Some(std::sync::Arc::new(ws::wscolab::on_save_document)),
        save_interval_ms: Some(30_000), // Save every 30 seconds
        default_permission: loro_websocket_server::protocol::Permission::Write,
        authenticate: Some(std::sync::Arc::new(ws::wscolab::on_authenticate)),
        handshake_auth: Some(std::sync::Arc::new(ws::wscolab::on_auth_handshake)),
        on_close_connection: Some(std::sync::Arc::new(ws::wscolab::on_close_connection)),
        on_update: Some(std::sync::Arc::new(ws::wscolab::on_update)),
        ..Default::default()
    };

    // Start WebSocket server
    let ws_listener = tokio::net::TcpListener::bind(&ws_addr)
        .await
        .unwrap_or_else(|_| panic!("Failed to bind WebSocket server to {}", ws_addr));

    info!("📡 WebSocket server starting on ws://{}", ws_addr);

    // Spawn WebSocket server task
    tokio::spawn(async move {
        if let Err(e) =
            loro_websocket_server::serve_incoming_with_config(ws_listener, ws_config).await
        {
            error!("WebSocket server error: {}", e);
        }
    });

    // Start the HTTP/API server
    let listener = tokio::net::TcpListener::bind(config.server_address())
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to {}", config.server_address()));

    info!("🚀 Server running on http://{}", config.server_address());
    info!("📡 WebSocket available at ws://{}", ws_addr);
    info!(
        "📚 Swagger UI available at http://{}/swagger",
        config.server_address()
    );

    axum::serve(listener, app_routes)
        .await
        .expect("Server failed to start");

    println!("DEBUG: Server exited");
}
