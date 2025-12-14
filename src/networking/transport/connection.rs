// Connection abstraction for P2P communication
//
// This module provides a unified connection abstraction that works with
// different transport types (TCP, QUIC) and tracks connection state and metrics.

use crate::common::NodeId;
use crate::error::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::RwLock;

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection is being established
    Connecting,
    /// Connection is active and ready
    Active,
    /// Connection is idle (no recent activity)
    Idle,
    /// Connection is being closed
    Closing,
    /// Connection is closed
    Closed,
    /// Connection failed
    Failed,
}

/// Transport type used by the connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportType {
    /// TCP transport
    Tcp,
    /// QUIC transport
    Quic,
}

/// Connection metadata and statistics
#[derive(Debug)]
pub struct Connection {
    /// Peer node identifier
    peer_id: NodeId,

    /// Transport type
    transport: TransportType,

    /// Current connection state
    state: RwLock<ConnectionState>,

    /// Time when connection was created
    created_at: Instant,

    /// Time of last activity (send or receive)
    last_activity: RwLock<Instant>,

    /// Total bytes sent through this connection
    bytes_sent: AtomicU64,

    /// Total bytes received through this connection
    bytes_received: AtomicU64,

    /// Number of messages sent
    messages_sent: AtomicU64,

    /// Number of messages received
    messages_received: AtomicU64,
}

impl Connection {
    /// Create a new connection
    pub fn new(peer_id: NodeId, transport: TransportType) -> Self {
        let now = Instant::now();
        Self {
            peer_id,
            transport,
            state: RwLock::new(ConnectionState::Connecting),
            created_at: now,
            last_activity: RwLock::new(now),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
        }
    }

    /// Get the peer node ID
    pub fn peer_id(&self) -> &NodeId {
        &self.peer_id
    }

    /// Get the transport type
    pub fn transport_type(&self) -> TransportType {
        self.transport
    }

    /// Get the current connection state
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Set the connection state
    pub async fn set_state(&self, new_state: ConnectionState) -> Result<()> {
        let mut state = self.state.write().await;
        *state = new_state;
        Ok(())
    }

    /// Get time since creation
    pub fn uptime(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }

    /// Get time since last activity
    pub async fn idle_time(&self) -> std::time::Duration {
        self.last_activity.read().await.elapsed()
    }

    /// Update last activity timestamp
    pub async fn update_activity(&self) {
        let mut last_activity = self.last_activity.write().await;
        *last_activity = Instant::now();
    }

    /// Record bytes sent
    pub fn record_bytes_sent(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Record bytes received
    pub fn record_bytes_received(&self, bytes: u64) {
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Get total bytes sent
    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    /// Get total bytes received
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received.load(Ordering::Relaxed)
    }

    /// Get total messages sent
    pub fn messages_sent(&self) -> u64 {
        self.messages_sent.load(Ordering::Relaxed)
    }

    /// Get total messages received
    pub fn messages_received(&self) -> u64 {
        self.messages_received.load(Ordering::Relaxed)
    }

    /// Check if connection is healthy
    pub async fn is_healthy(&self) -> bool {
        matches!(
            self.state().await,
            ConnectionState::Active | ConnectionState::Idle
        )
    }

    /// Check if connection should be closed due to inactivity
    pub async fn should_close_idle(&self, idle_timeout: std::time::Duration) -> bool {
        self.idle_time().await > idle_timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_creation() {
        let conn = Connection::new("node1".to_string(), TransportType::Tcp);

        assert_eq!(conn.peer_id(), "node1");
        assert_eq!(conn.transport_type(), TransportType::Tcp);
        assert_eq!(conn.state().await, ConnectionState::Connecting);
        assert_eq!(conn.bytes_sent(), 0);
        assert_eq!(conn.bytes_received(), 0);
    }

    #[tokio::test]
    async fn test_connection_state_transition() {
        let conn = Connection::new("node1".to_string(), TransportType::Tcp);

        assert_eq!(conn.state().await, ConnectionState::Connecting);

        conn.set_state(ConnectionState::Active).await.unwrap();
        assert_eq!(conn.state().await, ConnectionState::Active);
        assert!(conn.is_healthy().await);

        conn.set_state(ConnectionState::Closed).await.unwrap();
        assert_eq!(conn.state().await, ConnectionState::Closed);
        assert!(!conn.is_healthy().await);
    }

    #[tokio::test]
    async fn test_connection_metrics() {
        let conn = Connection::new("node1".to_string(), TransportType::Tcp);

        conn.record_bytes_sent(100);
        conn.record_bytes_sent(200);
        assert_eq!(conn.bytes_sent(), 300);
        assert_eq!(conn.messages_sent(), 2);

        conn.record_bytes_received(150);
        assert_eq!(conn.bytes_received(), 150);
        assert_eq!(conn.messages_received(), 1);
    }

    #[tokio::test]
    async fn test_idle_timeout() {
        let conn = Connection::new("node1".to_string(), TransportType::Tcp);

        // Initially should not timeout
        assert!(
            !conn
                .should_close_idle(std::time::Duration::from_secs(10))
                .await
        );

        // Update activity
        conn.update_activity().await;
        assert!(
            !conn
                .should_close_idle(std::time::Duration::from_secs(10))
                .await
        );
    }
}
