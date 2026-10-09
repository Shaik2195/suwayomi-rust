use axum::{
    extract::State,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use tracing::debug;
use suwayomi_downloader::queue::DownloadQueue;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(queue): State<DownloadQueue>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, queue))
}

async fn handle_socket(socket: WebSocket, queue: DownloadQueue) {
    debug!("Client connected via WebSocket");

    let (mut sender, mut receiver) = socket.split();
    let mut subscriber = queue.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = subscriber.recv().await {
            if let Ok(msg) = serde_json::to_string(&event) {
                if sender.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Ping(_) => {
                    // We shouldn't need to manually send pong if axum handles it, 
                    // but we can just ignore ping/pong manually here or process text
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    };

    debug!("Client disconnected");
}
