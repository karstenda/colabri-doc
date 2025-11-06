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

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use routes::create_api_routes;
use docs::ApiDoc;
use websocket::websocket_handler;
use config::Config;

#[tokio::main]
async fn main() {
    println!("Starting server ...");

    // Load configuration
    let config = Config::load().unwrap_or_else(|e| {
        eprintln!("Failed to load configuration: {}", e);
        eprintln!("Using default configuration");
        Config::default()
    });

    // Create API routes
    let api_routes = create_api_routes();

    let app_routes = Router::new()
        .nest("/api", api_routes)
        .route("/ws", get(websocket_handler))
        .merge(SwaggerUi::new("/swagger").url("/api-docs/openapi.json", ApiDoc::openapi()));

    // Start the server
    let listener = tokio::net::TcpListener::bind(config.server_address())
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to {}", config.server_address()));
        
    println!("🚀 Server running on http://{}", config.server_address());
    println!("📡 WebSocket available at ws://{}/ws", config.server_address());
    println!("📚 Swagger UI available at http://{}/swagger", config.server_address());

    axum::serve(listener, app_routes)
        .await
        .expect("Server failed to start");
}