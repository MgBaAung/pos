use crate::restaurant::domain::kitchen_orders::KitchenOrderEvent;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::{Arc, OnceLock};
use tokio::sync::broadcast;

/// Process-wide handle to the kitchen event channel so services outside the
/// WebSocket state (e.g. `SalesService`) can push KDS events without threading
/// the sender through every router's state type.
static KITCHEN_SENDER: OnceLock<broadcast::Sender<KitchenOrderEvent>> = OnceLock::new();

/// Initialize the global kitchen sender once at startup.
pub fn init_global_kitchen_sender(sender: broadcast::Sender<KitchenOrderEvent>) {
    let _ = KITCHEN_SENDER.set(sender);
}

/// Returns the global kitchen sender if it has been initialized.
pub fn global_kitchen_sender() -> Option<broadcast::Sender<KitchenOrderEvent>> {
    KITCHEN_SENDER.get().cloned()
}

pub struct KitchenBroadcastManager {
    sender: broadcast::Sender<KitchenOrderEvent>,
}

impl KitchenBroadcastManager {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self { sender }
    }

    pub fn get_sender(&self) -> broadcast::Sender<KitchenOrderEvent> {
        self.sender.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<KitchenOrderEvent> {
        self.sender.subscribe()
    }
}

impl Clone for KitchenBroadcastManager {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

pub async fn kitchen_websocket_handler(
    ws: WebSocketUpgrade,
    State(broadcast_manager): State<Arc<KitchenBroadcastManager>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, broadcast_manager))
}

async fn handle_socket(socket: WebSocket, broadcast_manager: Arc<KitchenBroadcastManager>) {
    tracing::info!("Kitchen WebSocket connection established");

    // Subscribe to kitchen events before splitting the socket.
    let mut receiver = broadcast_manager.subscribe();
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Send initial connection message
    if ws_sender
        .send(Message::Text(
            serde_json::json!({
                "type": "connected",
                "message": "Kitchen display connected",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })
            .to_string(),
        ))
        .await
        .is_err()
    {
        tracing::error!("Failed to send initial connection message");
        return;
    }

    loop {
        tokio::select! {
            // Forward kitchen events to this client as they are broadcast.
            event = receiver.recv() => {
                match event {
                    Ok(event) => {
                        let message = serde_json::to_string(&event).unwrap_or_else(|_| {
                            serde_json::json!({ "error": "Failed to serialize event" }).to_string()
                        });
                        if ws_sender.send(Message::Text(message)).await.is_err() {
                            tracing::info!("Kitchen WebSocket client disconnected");
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!("Kitchen WebSocket lagged, skipped {} events", skipped);
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            // Handle messages coming from the client (ping/pong, close).
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                            if value.get("type").and_then(|t| t.as_str()) == Some("ping") {
                                let pong = serde_json::json!({
                                    "type": "pong",
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                })
                                .to_string();
                                if ws_sender.send(Message::Text(pong)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!("Kitchen WebSocket connection closed by client");
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    tracing::info!("Kitchen WebSocket connection terminated");
}

// Extension methods for easy integration
pub fn create_kitchen_broadcast_manager() -> Arc<KitchenBroadcastManager> {
    Arc::new(KitchenBroadcastManager::new())
}
