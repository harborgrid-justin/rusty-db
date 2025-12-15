// # WebSocket Connection Management
//
// This module provides connection management for WebSocket connections in RustyDB.
//
// ## Features
//
// - **Connection Pool**: Manage multiple active WebSocket connections
// - **Heartbeat**: Automatic ping/pong keepalive mechanism
// - **Graceful Shutdown**: Proper connection cleanup and resource release
// - **Connection Metadata**: Track connection state, authentication, and statistics
//
// ## Usage
//
// ```rust
// use rusty_db::websocket::connection::{WebSocketConnection, ConnectionPool};
// use tokio_tungstenite::WebSocketStream;
//
// // Create connection pool
// let pool = ConnectionPool::new(1000);
//
// // Add connection
// let conn = WebSocketConnection::new(stream);
// pool.add_connection(conn).await;
// ```

use super::message::WebSocketMessage;
use super::protocol::{Protocol, ProtocolHandler};
use crate::error::{DbError, Result};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;

// ============================================================================
// Connection State
// ============================================================================

/// WebSocket connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection is being established
    Connecting,

    /// Connection is active and ready
    Connected,

    /// Connection is gracefully closing
    Closing,

    /// Connection is closed
    Closed,

    /// Connection encountered an error
    Error,
}

// ============================================================================
// Connection Metadata
// ============================================================================

/// Connection metadata and statistics
#[derive(Debug, Clone)]
pub struct ConnectionMetadata {
    /// Unique connection ID
    pub id: String,

    /// Client remote address
    pub remote_addr: Option<String>,

    /// Connection state
    pub state: ConnectionState,

    /// Protocol being used
    pub protocol: Protocol,

    /// When the connection was established
    pub connected_at: Instant,

    /// Last activity timestamp
    pub last_activity: Instant,

    /// Total messages sent
    pub messages_sent: u64,

    /// Total messages received
    pub messages_received: u64,

    /// Total bytes sent
    pub bytes_sent: u64,

    /// Total bytes received
    pub bytes_received: u64,

    /// User ID (if authenticated)
    pub user_id: Option<String>,

    /// Session ID (if authenticated)
    pub session_id: Option<String>,

    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl ConnectionMetadata {
    /// Create new connection metadata
    pub fn new(id: String, protocol: Protocol) -> Self {
        let now = Instant::now();
        Self {
            id,
            remote_addr: None,
            state: ConnectionState::Connecting,
            protocol,
            connected_at: now,
            last_activity: now,
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            user_id: None,
            session_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Get connection duration
    pub fn duration(&self) -> Duration {
        self.connected_at.elapsed()
    }

    /// Get idle duration
    pub fn idle_duration(&self) -> Duration {
        self.last_activity.elapsed()
    }

    /// Record sent message
    pub fn record_sent(&mut self, bytes: usize) {
        self.messages_sent += 1;
        self.bytes_sent += bytes as u64;
        self.touch();
    }

    /// Record received message
    pub fn record_received(&mut self, bytes: usize) {
        self.messages_received += 1;
        self.bytes_received += bytes as u64;
        self.touch();
    }
}

// ============================================================================
// WebSocket Connection
// ============================================================================

/// WebSocket connection handler
pub struct WebSocketConnection {
    /// Connection metadata
    pub metadata: Arc<RwLock<ConnectionMetadata>>,

    /// Protocol handler
    protocol_handler: Box<dyn ProtocolHandler + Send + Sync>,

    /// Message sender
    tx: mpsc::UnboundedSender<WebSocketMessage>,

    /// Shutdown signal sender
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl WebSocketConnection {
    /// Create a new WebSocket connection
    pub fn new(
        id: String,
        protocol: Protocol,
        tx: mpsc::UnboundedSender<WebSocketMessage>,
    ) -> Self {
        let metadata = ConnectionMetadata::new(id, protocol);
        let protocol_handler = protocol.create_handler();

        Self {
            metadata: Arc::new(RwLock::new(metadata)),
            protocol_handler,
            tx,
            shutdown_tx: None,
        }
    }

    /// Get connection ID
    pub async fn id(&self) -> String {
        self.metadata.read().await.id.clone()
    }

    /// Get connection state
    pub async fn state(&self) -> ConnectionState {
        self.metadata.read().await.state
    }

    /// Set connection state
    pub async fn set_state(&self, state: ConnectionState) {
        self.metadata.write().await.state = state;
    }

    /// Send a message to the client
    pub async fn send(&self, msg: WebSocketMessage) -> Result<()> {
        let msg_size = self.estimate_message_size(&msg);

        self.tx
            .send(msg)
            .map_err(|e| DbError::Network(format!("Failed to send message: {}", e)))?;

        self.metadata.write().await.record_sent(msg_size);
        Ok(())
    }

    /// Handle an incoming message
    pub async fn handle_message(&self, msg: WebSocketMessage) -> Result<()> {
        let msg_size = self.estimate_message_size(&msg);
        self.metadata.write().await.record_received(msg_size);

        // Handle control messages
        if msg.is_control() {
            return self.handle_control_message(&msg).await;
        }

        // Process through protocol handler
        if let Some(response) = self.protocol_handler.handle_message(&msg)? {
            self.send(response).await?;
        }

        Ok(())
    }

    /// Handle control messages (ping, pong, close)
    async fn handle_control_message(&self, msg: &WebSocketMessage) -> Result<()> {
        match msg {
            WebSocketMessage::Ping(data) => {
                // Respond with pong
                self.send(WebSocketMessage::pong(data.clone())).await?;
            }
            WebSocketMessage::Pong(_) => {
                // Pong received, connection is alive
                self.metadata.write().await.touch();
            }
            WebSocketMessage::Close { code, reason } => {
                // Handle close request
                tracing::info!(
                    "Connection {} closing: code={}, reason={}",
                    self.id().await,
                    code,
                    reason
                );
                self.set_state(ConnectionState::Closing).await;
            }
            _ => {}
        }
        Ok(())
    }

    /// Estimate message size for statistics
    fn estimate_message_size(&self, msg: &WebSocketMessage) -> usize {
        match msg {
            WebSocketMessage::Text(text) => text.len(),
            WebSocketMessage::Binary(data) => data.len(),
            WebSocketMessage::Ping(data) => data.len(),
            WebSocketMessage::Pong(data) => data.len(),
            _ => 0,
        }
    }

    /// Close the connection gracefully
    pub async fn close(&self, code: u16, reason: String) -> Result<()> {
        self.set_state(ConnectionState::Closing).await;
        self.send(WebSocketMessage::close(code, reason)).await?;

        // Send shutdown signal if available
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(()).await;
        }

        Ok(())
    }

    /// Get connection statistics
    pub async fn statistics(&self) -> ConnectionStats {
        let meta = self.metadata.read().await;
        ConnectionStats {
            id: meta.id.clone(),
            state: meta.state,
            protocol: meta.protocol,
            duration: meta.duration(),
            idle_duration: meta.idle_duration(),
            messages_sent: meta.messages_sent,
            messages_received: meta.messages_received,
            bytes_sent: meta.bytes_sent,
            bytes_received: meta.bytes_received,
        }
    }
}

/// Connection statistics snapshot
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub id: String,
    pub state: ConnectionState,
    pub protocol: Protocol,
    pub duration: Duration,
    pub idle_duration: Duration,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

// ============================================================================
// Connection Pool
// ============================================================================

/// Connection pool for managing multiple WebSocket connections
pub struct ConnectionPool {
    /// Active connections
    connections: Arc<RwLock<HashMap<String, Arc<WebSocketConnection>>>>,

    /// Maximum number of connections
    max_connections: usize,

    /// Connection timeout (idle duration before disconnect)
    connection_timeout: Duration,

    /// Heartbeat interval
    heartbeat_interval: Duration,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(max_connections: usize) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            max_connections,
            connection_timeout: Duration::from_secs(300), // 5 minutes
            heartbeat_interval: Duration::from_secs(30),  // 30 seconds
        }
    }

    /// Create a connection pool with custom settings
    pub fn with_config(
        max_connections: usize,
        connection_timeout: Duration,
        heartbeat_interval: Duration,
    ) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            max_connections,
            connection_timeout,
            heartbeat_interval,
        }
    }

    /// Add a connection to the pool
    pub async fn add_connection(&self, connection: Arc<WebSocketConnection>) -> Result<()> {
        let mut connections = self.connections.write().await;

        // Check if pool is full
        if connections.len() >= self.max_connections {
            return Err(DbError::ResourceExhausted(
                "Connection pool is full".to_string(),
            ));
        }

        let id = connection.id().await;
        connections.insert(id.clone(), connection);

        tracing::info!(
            "Connection {} added to pool (total: {})",
            id,
            connections.len()
        );
        Ok(())
    }

    /// Remove a connection from the pool
    pub async fn remove_connection(&self, id: &str) -> Option<Arc<WebSocketConnection>> {
        let mut connections = self.connections.write().await;
        let connection = connections.remove(id);

        if connection.is_some() {
            tracing::info!(
                "Connection {} removed from pool (total: {})",
                id,
                connections.len()
            );
        }

        connection
    }

    /// Get a connection by ID
    pub async fn get_connection(&self, id: &str) -> Option<Arc<WebSocketConnection>> {
        let connections = self.connections.read().await;
        connections.get(id).cloned()
    }

    /// Get all connection IDs
    pub async fn connection_ids(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }

    /// Get number of active connections
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// Broadcast a message to all connections
    pub async fn broadcast(&self, msg: WebSocketMessage) -> Result<()> {
        let connections = self.connections.read().await;

        for connection in connections.values() {
            if let Err(e) = connection.send(msg.clone()).await {
                tracing::error!(
                    "Failed to broadcast to connection {}: {}",
                    connection.id().await,
                    e
                );
            }
        }

        Ok(())
    }

    /// Send a message to specific connections
    pub async fn multicast(&self, ids: &[String], msg: WebSocketMessage) -> Result<()> {
        let connections = self.connections.read().await;

        for id in ids {
            if let Some(connection) = connections.get(id) {
                if let Err(e) = connection.send(msg.clone()).await {
                    tracing::error!("Failed to send to connection {}: {}", id, e);
                }
            }
        }

        Ok(())
    }

    /// Get pool statistics
    pub async fn statistics(&self) -> PoolStats {
        let connections = self.connections.read().await;
        let count = connections.len();

        let mut total_messages_sent = 0;
        let mut total_messages_received = 0;
        let mut total_bytes_sent = 0;
        let mut total_bytes_received = 0;

        for conn in connections.values() {
            let stats = conn.statistics().await;
            total_messages_sent += stats.messages_sent;
            total_messages_received += stats.messages_received;
            total_bytes_sent += stats.bytes_sent;
            total_bytes_received += stats.bytes_received;
        }

        PoolStats {
            active_connections: count,
            max_connections: self.max_connections,
            total_messages_sent,
            total_messages_received,
            total_bytes_sent,
            total_bytes_received,
        }
    }

    /// Start heartbeat task for keeping connections alive
    pub async fn start_heartbeat(&self) {
        let connections = self.connections.clone();
        let interval_duration = self.heartbeat_interval;

        tokio::spawn(async move {
            let mut ticker = interval(interval_duration);
            loop {
                ticker.tick().await;

                let conns = connections.read().await;
                for connection in conns.values() {
                    // Send ping to keep connection alive
                    if let Err(e) = connection.send(WebSocketMessage::ping(vec![])).await {
                        tracing::error!(
                            "Failed to send heartbeat to {}: {}",
                            connection.id().await,
                            e
                        );
                    }
                }
            }
        });
    }

    /// Start cleanup task for removing idle/dead connections
    pub async fn start_cleanup(&self) {
        let connections = self.connections.clone();
        let timeout = self.connection_timeout;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(60)); // Check every minute
            loop {
                ticker.tick().await;

                let mut to_remove = Vec::new();

                {
                    let conns = connections.read().await;
                    for (id, connection) in conns.iter() {
                        let meta = connection.metadata.read().await;

                        // Check if connection is idle
                        if meta.idle_duration() > timeout {
                            to_remove.push(id.clone());
                        }

                        // Check if connection is closed or errored
                        if matches!(meta.state, ConnectionState::Closed | ConnectionState::Error) {
                            to_remove.push(id.clone());
                        }
                    }
                }

                // Remove idle/dead connections
                if !to_remove.is_empty() {
                    let mut conns = connections.write().await;
                    for id in to_remove {
                        conns.remove(&id);
                        tracing::info!("Cleaned up idle/dead connection: {}", id);
                    }
                }
            }
        });
    }

    /// Shutdown all connections gracefully
    pub async fn shutdown(&self) -> Result<()> {
        let connections = self.connections.write().await;

        for connection in connections.values() {
            connection
                .close(1000, "Server shutting down".to_string())
                .await?;
        }

        tracing::info!("All connections shut down gracefully");
        Ok(())
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub active_connections: usize,
    pub max_connections: usize,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_metadata() {
        let mut meta = ConnectionMetadata::new("test-1".to_string(), Protocol::JsonRpc);

        assert_eq!(meta.id, "test-1");
        assert_eq!(meta.state, ConnectionState::Connecting);
        assert_eq!(meta.messages_sent, 0);

        meta.record_sent(100);
        assert_eq!(meta.messages_sent, 1);
        assert_eq!(meta.bytes_sent, 100);

        meta.record_received(200);
        assert_eq!(meta.messages_received, 1);
        assert_eq!(meta.bytes_received, 200);
    }

    #[tokio::test]
    async fn test_connection_pool() {
        let pool = ConnectionPool::new(10);
        assert_eq!(pool.connection_count().await, 0);

        let (tx, _rx) = mpsc::unbounded_channel();
        let conn = Arc::new(WebSocketConnection::new(
            "test-1".to_string(),
            Protocol::JsonRpc,
            tx,
        ));

        pool.add_connection(conn).await.unwrap();
        assert_eq!(pool.connection_count().await, 1);

        let ids = pool.connection_ids().await;
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "test-1");
    }

    #[tokio::test]
    async fn test_connection_pool_max_limit() {
        let pool = ConnectionPool::new(2);

        for i in 0..2 {
            let (tx, _rx) = mpsc::unbounded_channel();
            let conn = Arc::new(WebSocketConnection::new(
                format!("conn-{}", i),
                Protocol::JsonRpc,
                tx,
            ));
            pool.add_connection(conn).await.unwrap();
        }

        assert_eq!(pool.connection_count().await, 2);

        // Try to add one more (should fail)
        let (tx, _rx) = mpsc::unbounded_channel();
        let conn = Arc::new(WebSocketConnection::new(
            "conn-overflow".to_string(),
            Protocol::JsonRpc,
            tx,
        ));

        let result = pool.add_connection(conn).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pool_statistics() {
        let pool = ConnectionPool::new(10);

        let (tx, _rx) = mpsc::unbounded_channel();
        let conn = Arc::new(WebSocketConnection::new(
            "test-1".to_string(),
            Protocol::JsonRpc,
            tx,
        ));

        pool.add_connection(conn.clone()).await.unwrap();

        // Send some messages
        conn.send(WebSocketMessage::text("test")).await.unwrap();

        let stats = pool.statistics().await;
        assert_eq!(stats.active_connections, 1);
        assert!(stats.total_messages_sent > 0);
    }
}
