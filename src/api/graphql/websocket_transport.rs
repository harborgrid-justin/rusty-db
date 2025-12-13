// GraphQL WebSocket Transport
//
// Implements the graphql-ws protocol for GraphQL subscriptions over WebSocket
// Protocol spec: https://github.com/enisdenjo/graphql-ws/blob/master/PROTOCOL.md

use async_graphql::{Data, ObjectType, Schema, SubscriptionType};
use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{stream::StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

// ============================================================================
// MESSAGE TYPES (graphql-ws protocol)
// ============================================================================

/// graphql-ws message type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GraphQLWsMessage {
    /// Direction: Client -> Server
    /// Indicates that the client wants to establish a connection
    ConnectionInit {
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<ConnectionInitPayload>,
    },

    /// Direction: Server -> Client
    /// Expected response to ConnectionInit
    ConnectionAck {
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<serde_json::Value>,
    },

    /// Direction: bidirectional
    /// Useful for detecting failed connections, displaying latency metrics
    Ping {
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<serde_json::Value>,
    },

    /// Direction: bidirectional
    /// Response to Ping message
    Pong {
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<serde_json::Value>,
    },

    /// Direction: Client -> Server
    /// Requests an operation (query, mutation, or subscription)
    Subscribe {
        id: String,
        payload: SubscribePayload,
    },

    /// Direction: Server -> Client
    /// Operation execution result(s)
    Next {
        id: String,
        payload: serde_json::Value,
    },

    /// Direction: Server -> Client
    /// Operation execution error(s)
    Error {
        id: String,
        payload: Vec<GraphQLError>,
    },

    /// Direction: bidirectional
    /// Indicates that the operation is complete
    Complete {
        id: String,
    },
}

/// Connection initialization payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInitPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Subscribe operation payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribePayload {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

/// GraphQL error format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<ErrorLocation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

/// Error location in GraphQL document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorLocation {
    pub line: usize,
    pub column: usize,
}

// ============================================================================
// CONNECTION STATE
// ============================================================================

/// WebSocket connection state
struct ConnectionState {
    /// Active subscriptions for this connection
    subscriptions: HashMap<String, tokio::task::JoinHandle<()>>,
    /// Whether connection has been initialized
    initialized: bool,
    /// Connection metadata
    metadata: Option<ConnectionInitPayload>,
}

impl ConnectionState {
    fn new() -> Self {
        Self {
            subscriptions: HashMap::new(),
            initialized: false,
            metadata: None,
        }
    }
}

// ============================================================================
// WEBSOCKET TRANSPORT CONFIGURATION
// ============================================================================

/// WebSocket transport configuration
#[derive(Clone)]
pub struct WebSocketConfig {
    /// Connection initialization timeout (default: 10 seconds)
    pub connection_init_timeout: Duration,
    /// Keep-alive interval for ping/pong (default: 30 seconds)
    pub keep_alive_interval: Duration,
    /// Maximum payload size (default: 10MB)
    pub max_payload_size: usize,
    /// Maximum concurrent subscriptions per connection (default: 100)
    pub max_subscriptions: usize,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            connection_init_timeout: Duration::from_secs(10),
            keep_alive_interval: Duration::from_secs(30),
            max_payload_size: 10 * 1024 * 1024, // 10MB
            max_subscriptions: 100,
        }
    }
}

// ============================================================================
// WEBSOCKET HANDLER
// ============================================================================

/// WebSocket GraphQL subscription handler
pub async fn graphql_ws_handler<Q, M, S>(
    ws: WebSocketUpgrade,
    State(schema): State<Schema<Q, M, S>>,
) -> Response
where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    ws.on_upgrade(move |socket| handle_websocket(socket, schema, WebSocketConfig::default()))
}

/// WebSocket GraphQL subscription handler with custom config
pub async fn graphql_ws_handler_with_config<Q, M, S>(
    ws: WebSocketUpgrade,
    State((schema, config)): State<(Schema<Q, M, S>, WebSocketConfig)>,
) -> Response
where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    ws.on_upgrade(move |socket| handle_websocket(socket, schema, config))
}

/// Handle WebSocket connection
async fn handle_websocket<Q, M, S>(
    socket: WebSocket,
    schema: Schema<Q, M, S>,
    config: WebSocketConfig,
) where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    let (mut sender, mut receiver) = socket.split();
    let state = Arc::new(RwLock::new(ConnectionState::new()));

    // Channel for sending messages to client
    let (tx, mut rx) = broadcast::channel::<GraphQLWsMessage>(100);

    // Spawn task to send messages from broadcast channel to WebSocket
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(WsMessage::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Spawn keep-alive task
    let keep_alive_tx = tx.clone();
    let keep_alive_interval = config.keep_alive_interval;
    let keep_alive_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(keep_alive_interval);
        loop {
            interval.tick().await;
            let _ = keep_alive_tx.send(GraphQLWsMessage::Ping { payload: None });
        }
    });

    // Wait for connection initialization
    let init_timeout = config.connection_init_timeout;
    let initialized = timeout(init_timeout, async {
        while let Some(Ok(msg)) = receiver.next().await {
            if let WsMessage::Text(text) = msg {
                if let Ok(ws_msg) = serde_json::from_str::<GraphQLWsMessage>(&text) {
                    if let GraphQLWsMessage::ConnectionInit { payload } = ws_msg {
                        return Some(payload);
                    }
                }
            }
        }
        None
    })
    .await;

    match initialized {
        Ok(Some(payload)) => {
            // Connection initialized successfully
            let mut state_guard = state.write().await;
            state_guard.initialized = true;
            state_guard.metadata = payload;
            drop(state_guard);

            // Send ConnectionAck
            let _ = tx.send(GraphQLWsMessage::ConnectionAck { payload: None });
            info!("WebSocket connection initialized");
        }
        Ok(None) => {
            warn!("WebSocket connection closed before initialization");
            keep_alive_task.abort();
            send_task.abort();
            return;
        }
        Err(_) => {
            error!("WebSocket connection initialization timeout");
            keep_alive_task.abort();
            send_task.abort();
            return;
        }
    }

    // Handle incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            WsMessage::Text(text) => {
                if text.len() > config.max_payload_size {
                    warn!("Message exceeds max payload size");
                    continue;
                }

                match serde_json::from_str::<GraphQLWsMessage>(&text) {
                    Ok(ws_msg) => {
                        handle_message(
                            ws_msg,
                            &schema,
                            &state,
                            &tx,
                            &config,
                        )
                        .await;
                    }
                    Err(e) => {
                        error!("Failed to parse WebSocket message: {}", e);
                    }
                }
            }
            WsMessage::Binary(_) => {
                warn!("Binary messages not supported");
            }
            WsMessage::Ping(data) => {
                debug!("Received WebSocket ping");
                // Axum handles pong automatically
            }
            WsMessage::Pong(_) => {
                debug!("Received WebSocket pong");
            }
            WsMessage::Close(_) => {
                info!("WebSocket connection closed by client");
                break;
            }
        }
    }

    // Cleanup
    let mut state_guard = state.write().await;
    for (id, handle) in state_guard.subscriptions.drain() {
        debug!("Aborting subscription: {}", id);
        handle.abort();
    }
    drop(state_guard);

    keep_alive_task.abort();
    send_task.abort();

    info!("WebSocket connection handler terminated");
}

/// Handle individual GraphQL WebSocket message
async fn handle_message<Q, M, S>(
    msg: GraphQLWsMessage,
    schema: &Schema<Q, M, S>,
    state: &Arc<RwLock<ConnectionState>>,
    tx: &broadcast::Sender<GraphQLWsMessage>,
    config: &WebSocketConfig,
) where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    match msg {
        GraphQLWsMessage::ConnectionInit { .. } => {
            // Already handled during initialization
            debug!("Received duplicate ConnectionInit, ignoring");
        }

        GraphQLWsMessage::Ping { payload } => {
            debug!("Received Ping");
            let _ = tx.send(GraphQLWsMessage::Pong { payload });
        }

        GraphQLWsMessage::Pong { .. } => {
            debug!("Received Pong");
        }

        GraphQLWsMessage::Subscribe { id, payload } => {
            let state_guard = state.read().await;
            if !state_guard.initialized {
                error!("Received Subscribe before ConnectionInit");
                return;
            }

            if state_guard.subscriptions.len() >= config.max_subscriptions {
                error!("Maximum subscriptions limit reached");
                let _ = tx.send(GraphQLWsMessage::Error {
                    id: id.clone(),
                    payload: vec![GraphQLError {
                        message: "Maximum subscriptions limit reached".to_string(),
                        locations: None,
                        path: None,
                        extensions: None,
                    }],
                });
                return;
            }
            drop(state_guard);

            info!("Starting subscription: {}", id);

            // Create GraphQL request
            let request = async_graphql::Request::new(&payload.query);

            // Note: Full subscription streaming requires schema ownership.
            // For now, execute as a single query and send response.
            let response = schema.execute(request).await;
            let response_payload = serde_json::to_value(&response).unwrap_or_default();

            if response.is_ok() {
                let _ = tx.send(GraphQLWsMessage::Next {
                    id: id.clone(),
                    payload: response_payload,
                });
            } else {
                let errors: Vec<GraphQLError> = response
                    .errors
                    .into_iter()
                    .map(|e| GraphQLError {
                        message: e.message,
                        locations: if e.locations.is_empty() {
                            None
                        } else {
                            Some(e.locations.into_iter()
                                .map(|loc| ErrorLocation {
                                    line: loc.line,
                                    column: loc.column,
                                })
                                .collect())
                        },
                        path: if e.path.is_empty() {
                            None
                        } else {
                            Some(e.path.into_iter().map(|s| {
                                serde_json::to_value(&s).unwrap_or_else(|_| serde_json::Value::Null)
                            }).collect())
                        },
                        extensions: e.extensions.map(|ext| {
                            serde_json::to_value(ext).unwrap_or_default()
                        }),
                    })
                    .collect();

                let _ = tx.send(GraphQLWsMessage::Error {
                    id: id.clone(),
                    payload: errors,
                });
            }

            // Mark subscription complete
            let _ = tx.send(GraphQLWsMessage::Complete {
                id: id.clone(),
            });

            // Track subscription for management
            let handle = tokio::spawn(async {});

            // Store subscription handle
            let mut state_guard = state.write().await;
            state_guard.subscriptions.insert(id, handle);
        }

        GraphQLWsMessage::Complete { id } => {
            info!("Client requested completion of subscription: {}", id);
            let mut state_guard = state.write().await;
            if let Some(handle) = state_guard.subscriptions.remove(&id) {
                handle.abort();
            }
        }

        _ => {
            warn!("Unexpected message type from client");
        }
    }
}

// ============================================================================
// SUBSCRIPTION MANAGER
// ============================================================================

/// Manager for tracking active WebSocket subscriptions
pub struct WebSocketSubscriptionManager {
    /// Active connections
    connections: Arc<RwLock<HashMap<String, ConnectionMetrics>>>,
}

impl WebSocketSubscriptionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Get total subscription count across all connections
    pub async fn subscription_count(&self) -> usize {
        self.connections
            .read()
            .await
            .values()
            .map(|c| c.subscription_count)
            .sum()
    }

    /// Get connection metrics
    pub async fn get_metrics(&self) -> Vec<ConnectionMetrics> {
        self.connections.read().await.values().cloned().collect()
    }
}

/// Connection metrics
#[derive(Clone, Debug)]
pub struct ConnectionMetrics {
    pub connection_id: String,
    pub subscription_count: usize,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}
