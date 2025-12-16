// WebSocket Helper Functions
//
// Common utilities for WebSocket handlers to reduce duplication

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use super::types::ApiState;

/// Send a welcome message to a newly connected WebSocket client
///
/// This standardizes the welcome message format across all WebSocket endpoints
pub async fn send_welcome_message(socket: &mut WebSocket, connection_type: &str) -> Result<(), ()> {
    let welcome = json!({
        "type": "welcome",
        "connection_type": connection_type,
        "message": format!("Connected to RustyDB {} stream", connection_type),
        "version": "1.0.0",
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });

    if let Ok(welcome_json) = serde_json::to_string(&welcome) {
        socket.send(Message::Text(welcome_json)).await.map_err(|_| ())
    } else {
        Err(())
    }
}

/// Send a JSON message to a WebSocket client
pub async fn send_json_message(
    socket: &mut WebSocket,
    message_type: &str,
    data: serde_json::Value,
) -> Result<(), ()> {
    let message = json!({
        "type": message_type,
        "data": data,
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });

    if let Ok(json_str) = serde_json::to_string(&message) {
        socket.send(Message::Text(json_str)).await.map_err(|_| ())
    } else {
        Err(())
    }
}

/// Send an error message to a WebSocket client
pub async fn send_error_message(
    socket: &mut WebSocket,
    error_code: &str,
    error_message: &str,
) -> Result<(), ()> {
    let error = json!({
        "type": "error",
        "error_code": error_code,
        "message": error_message,
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });

    if let Ok(error_json) = serde_json::to_string(&error) {
        socket.send(Message::Text(error_json)).await.map_err(|_| ())
    } else {
        Err(())
    }
}

/// Generic WebSocket handler wrapper that sends welcome message and handles errors
///
/// This function wraps a custom handler function and provides standardized
/// connection setup, welcome messages, and error handling.
///
/// # Arguments
/// * `socket` - The WebSocket connection
/// * `state` - The API state
/// * `connection_type` - A string describing the connection type (e.g., "query", "metrics")
/// * `handler` - An async function that implements the actual WebSocket logic
///
/// # Example
/// ```ignore
/// pub async fn ws_metrics_stream(ws: WebSocketUpgrade, State(state): State<Arc<ApiState>>) -> Response {
///     ws.on_upgrade(|socket| {
///         websocket_handler_wrapper(socket, state, "metrics", handle_metrics_logic)
///     })
/// }
///
/// async fn handle_metrics_logic(socket: &mut WebSocket, state: Arc<ApiState>) -> Result<(), ()> {
///     // Custom metrics streaming logic
///     Ok(())
/// }
/// ```
pub async fn websocket_handler_wrapper<F, Fut>(
    mut socket: WebSocket,
    state: Arc<ApiState>,
    connection_type: &str,
    handler: F,
) where
    F: FnOnce(WebSocket, Arc<ApiState>) -> Fut,
    Fut: std::future::Future<Output = Result<(), ()>>,
{
    // Send welcome message
    if send_welcome_message(&mut socket, connection_type)
        .await
        .is_err()
    {
        return;
    }

    // Execute the custom handler
    if let Err(_) = handler(socket, state).await {
        // Handler failed, connection will be closed
    }
}

/// Create a streaming WebSocket handler that periodically sends data
///
/// This is a higher-order function that creates WebSocket handlers for
/// periodic data streaming (metrics, events, etc.)
///
/// # Arguments
/// * `socket` - The WebSocket connection
/// * `state` - The API state
/// * `connection_type` - Type of connection for welcome message
/// * `interval_ms` - Milliseconds between updates
/// * `data_fn` - Function to generate data at each interval
pub async fn streaming_websocket_handler<F>(
    mut socket: WebSocket,
    _state: Arc<ApiState>,
    connection_type: &str,
    interval_ms: u64,
    mut data_fn: F,
) where
    F: FnMut() -> serde_json::Value,
{
    use tokio::time::{interval, Duration};

    // Send welcome message
    if send_welcome_message(&mut socket, connection_type)
        .await
        .is_err()
    {
        return;
    }

    // Set up interval for periodic updates
    let mut tick_interval = interval(Duration::from_millis(interval_ms));

    loop {
        tokio::select! {
            _ = tick_interval.tick() => {
                // Generate and send data
                let data = data_fn();
                if send_json_message(&mut socket, "data", data).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(_)) => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Handle incoming WebSocket messages with a custom message handler
///
/// This provides a standard message receiving loop with ping/pong handling
///
/// # Arguments
/// * `socket` - The WebSocket connection
/// * `message_handler` - Function to handle incoming text messages
pub async fn message_loop<F, Fut>(mut socket: WebSocket, mut message_handler: F)
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Some(response) = message_handler(text).await {
                    if socket.send(Message::Text(response)).await.is_err() {
                        break;
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(data)) => {
                if socket.send(Message::Pong(data)).await.is_err() {
                    break;
                }
            }
            Err(_) => break,
            _ => {}
        }
    }
}
