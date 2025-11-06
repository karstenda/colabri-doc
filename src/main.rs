mod models;
mod handlers;
mod routes;
mod docs;
mod websocket;

use axum::{
    Router,
    routing::get,
};

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use routes::create_api_routes;
use docs::ApiDoc;
use websocket::websocket_handler;

#[tokio::main]
async fn main() {
    println!("Starting server ...");

    // Create API routes
    let api_routes = create_api_routes();

    let app_routes = Router::new()
        .nest("/api", api_routes)
        .route("/ws", get(websocket_handler))
        .merge(SwaggerUi::new("/swagger").url("/api-docs/openapi.json", ApiDoc::openapi()));

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");
        
    println!("🚀 Server running on http://localhost:3000");
    println!("📡 WebSocket available at ws://localhost:3000/ws");
    println!("📚 Swagger UI available at http://localhost:3000/swagger");

    axum::serve(listener, app_routes)
        .await
        .expect("Server failed to start");
}