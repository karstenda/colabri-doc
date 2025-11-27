use axum::{
    extract::Request,
    response::{IntoResponse, Response},
};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::{debug, error};

/// Bridge handler that extracts the upgraded connection and passes it to loro-websocket-server
pub async fn loro_websocket_bridge(
    req: Request,
    registry: Arc<loro_websocket_server::HubRegistry>,
) -> Response {
    // Check if this is a WebSocket upgrade request
    if !is_upgrade_request(&req) {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "Expected WebSocket upgrade request",
        )
            .into_response();
    }

    // Perform the WebSocket upgrade
    let (response, upgraded) = match hyper::upgrade::on(req).await {
        Ok(upgraded) => {
            let response = Response::builder()
                .status(hyper::StatusCode::SWITCHING_PROTOCOLS)
                .header(hyper::header::UPGRADE, "websocket")
                .header(hyper::header::CONNECTION, "upgrade")
                .body(axum::body::Body::empty())
                .unwrap();
            (response, upgraded)
        }
        Err(e) => {
            error!("Upgrade error: {}", e);
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to upgrade connection",
            )
                .into_response();
        }
    };

    // Spawn a task to handle the upgraded connection with loro-websocket-server
    tokio::spawn(async move {
        let io = TokioIo::new(upgraded);
        // Convert to a type compatible with tokio-tungstenite
        if let Err(e) = handle_loro_connection(io, registry).await {
            error!("Error handling loro connection: {}", e);
        }
    });

    response
}

fn is_upgrade_request(req: &Request) -> bool {
    req.headers()
        .get(hyper::header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false)
}

async fn handle_loro_connection<T>(
    _io: T,
    _registry: Arc<loro_websocket_server::HubRegistry>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // TODO: This needs to be adapted based on loro-websocket-server's API
    // The challenge is that handle_conn expects a TcpStream, not a generic stream
    debug!("Loro connection handler called");
    Ok(())
}
