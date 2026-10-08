use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use tracing::{debug, error};

pub async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    debug!("Client connected via WebSocket");

    // In a real application, you would hook this up to a broadcast channel
    // to stream real-time progress updates. Here we just echo back messages
    // or handle basic ping/pong to keep the connection alive.
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(t) => {
                    debug!("Received text message: {}", t);
                    // Echo back for now
                    if socket.send(Message::Text(t)).await.is_err() {
                        error!("Client disconnected");
                        return;
                    }
                }
                Message::Binary(_) => {
                    debug!("Received binary message");
                }
                Message::Ping(_) | Message::Pong(_) => {
                    // Handled automatically by axum
                }
                Message::Close(_) => {
                    debug!("Client disconnected");
                    return;
                }
            }
        } else {
            error!("Client disconnected");
            return;
        }
    }
}
