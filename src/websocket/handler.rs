use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};

/// WebSocket handler
pub async fn websocket_handler(ws: WebSocketUpgrade) -> Response {
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