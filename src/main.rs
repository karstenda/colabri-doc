use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    http::StatusCode,
    response::{Html, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

/// API response for health check
#[derive(Serialize, Deserialize, ToSchema)]
struct HealthResponse {
    status: String,
    message: String,
}

/// Request body for creating an item
#[derive(Serialize, Deserialize, ToSchema)]
struct CreateItemRequest {
    name: String,
    description: String,
}

/// Response for creating an item
#[derive(Serialize, Deserialize, ToSchema)]
struct CreateItemResponse {
    id: u32,
    name: String,
    description: String,
}

/// Application state
#[derive(Clone)]
struct AppState {
    item_counter: Arc<Mutex<u32>>,
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        message: "Server is running".to_string(),
    })
}

/// Create a new item
#[utoipa::path(
    post,
    path = "/api/items",
    request_body = CreateItemRequest,
    responses(
        (status = 201, description = "Item created successfully", body = CreateItemResponse)
    )
)]
async fn create_item(
    State(state): State<AppState>,
    Json(payload): Json<CreateItemRequest>,
) -> (StatusCode, Json<CreateItemResponse>) {
    let mut counter = state.item_counter.lock().await;
    *counter += 1;
    let id = *counter;

    (
        StatusCode::CREATED,
        Json(CreateItemResponse {
            id,
            name: payload.name,
            description: payload.description,
        }),
    )
}

/// WebSocket handler
async fn websocket_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

/// Handle WebSocket connection
async fn handle_socket(mut socket: WebSocket) {
    // Send a welcome message
    if socket
        .send(Message::Text(String::from(
            "Welcome to the WebSocket server!",
        )))
        .await
        .is_err()
    {
        return;
    }

    // Echo messages back to the client
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    let response = format!("Echo: {}", text);
                    if socket.send(Message::Text(response)).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        } else {
            break;
        }
    }
}

/// Root endpoint
async fn root() -> Html<&'static str> {
    Html(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Colabri Doc Server</title>
            <style>
                body { font-family: Arial, sans-serif; margin: 40px; }
                h1 { color: #333; }
                a { color: #0066cc; text-decoration: none; }
                a:hover { text-decoration: underline; }
                .links { margin-top: 20px; }
                .links a { display: block; margin: 10px 0; }
            </style>
        </head>
        <body>
            <h1>Welcome to Colabri Doc Server</h1>
            <p>This server provides:</p>
            <ul>
                <li>REST API endpoints under /api</li>
                <li>WebSocket connections at /ws</li>
                <li>Swagger documentation at /swagger-ui</li>
            </ul>
            <div class="links">
                <a href="/api/health">Health Check API</a>
                <a href="/swagger-ui">API Documentation (Swagger UI)</a>
            </div>
        </body>
        </html>
        "#,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check,
        create_item,
    ),
    components(
        schemas(HealthResponse, CreateItemRequest, CreateItemResponse)
    ),
    tags(
        (name = "api", description = "API endpoints")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    // Initialize application state
    let state = AppState {
        item_counter: Arc::new(Mutex::new(0)),
    };

    // Create API routes
    let api_routes = Router::new()
        .route("/health", get(health_check))
        .route("/items", post(create_item))
        .with_state(state.clone());

    // Build the main application
    let app = Router::new()
        .route("/", get(root))
        .route("/ws", get(websocket_handler))
        .nest("/api", api_routes)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive());

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");
    println!("🚀 Server running on http://localhost:3000");
    println!("📡 WebSocket available at ws://localhost:3000/ws");
    println!("📚 Swagger UI available at http://localhost:3000/swagger-ui");

    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}
