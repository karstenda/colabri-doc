use tracing::{info, error};
use std::sync::Arc;
use axum::{
    extract::ws::{Message, WebSocket},
};
use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use tokio::sync::Mutex;
use crate::models::{PingMessage, SendMessage};
use chrono::Utc;

/// Handle PingMessage
pub async fn handle_ping_message(_ping_msg: &PingMessage, document_id: String, sender: &Arc<Mutex<SplitSink<WebSocket, Message>>>) {
    // Handle ping message - send a pong message back.
    info!("Ping message received for document {}", document_id);

    // Reply with pong
    let pong = SendMessage::Pong(crate::models::messages::PongMessage { date: Utc::now().to_rfc3339() });
    let pong_msg = serde_json::to_string(&pong).unwrap();
    if sender.lock().await.send(Message::Text(pong_msg)).await.is_err() {
        error!("Failed to send Pong message for document {}", document_id);
        return;
    }
}