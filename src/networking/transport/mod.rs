//! Transport layer for P2P communication
//!
//! This module provides the transport abstraction for peer-to-peer communication
//! in RustyDB's distributed architecture. It supports multiple transport protocols
//! with enterprise-grade reliability features.
//!
//! # Features
//!
//! - **Multiple Transports**: TCP and QUIC support
//! - **Connection Pooling**: Efficient connection reuse with configurable limits
//! - **Auto-Reconnection**: Exponential backoff retry logic
//! - **Health Monitoring**: Automatic detection and cleanup of failed connections
//! - **Load Balancing**: Multiple connection selection strategies
//! - **Metrics**: Comprehensive connection and throughput tracking
//!
//! # Example
//!
//! ```rust,no_run
//! use rusty_db::networking::transport::{TcpTransport, TcpConfig, ConnectionPool, PoolConfig};
//! use std::sync::Arc;
//!
//! # async fn example() -> rusty_db::Result<()> {
//! // Create TCP transport
//! let config = TcpConfig::default();
//! let mut transport = TcpTransport::new(config);
//! transport.bind().await?;
//!
//! // Create connection pool
//! let pool_config = PoolConfig::default();
//! let pool = Arc::new(ConnectionPool::new(pool_config));
//!
//! // Start health check background task
//! let _health_task = pool.clone().start_health_check_task();
//!
//! // Accept connections
//! // let tcp_conn = transport.accept().await?;
//!
//! # Ok(())
//! # }
//! ```

pub mod connection;
pub mod pool;
pub mod quic;
pub mod tcp;

pub use connection::{Connection, ConnectionState, TransportType};
pub use pool::{ConnectionPool, PoolConfig, PoolStatistics, SelectionStrategy};
pub use quic::{QuicConfig, QuicConnection, QuicTransport};
pub use tcp::{TcpConfig, TcpConnection, TcpTransport};

use crate::common::NodeId;
use crate::error::Result;
use async_trait::async_trait;
use std::net::SocketAddr;

/// Transport trait abstraction
///
/// This trait defines the common interface that all transport implementations
/// must provide, enabling polymorphic usage of different transport types.
#[async_trait]
pub trait Transport: Send + Sync {
    /// The connection type returned by this transport
    type Connection: Send + Sync;

    /// Start listening for incoming connections
    async fn bind(&mut self) -> Result<()>;

    /// Accept an incoming connection
    async fn accept(&self) -> Result<Self::Connection>;

    /// Connect to a remote peer
    async fn connect(&self, addr: SocketAddr, peer_id: NodeId) -> Result<Self::Connection>;

    /// Get the local address this transport is bound to
    fn local_addr(&self) -> Option<SocketAddr>;
}

/// Transport manager for handling multiple transport types
pub struct TransportManager {
    tcp_transport: Option<TcpTransport>,
    quic_transport: Option<QuicTransport>,
    connection_pool: ConnectionPool,
}

impl TransportManager {
    /// Create a new transport manager
    pub fn new(pool_config: PoolConfig) -> Self {
        Self {
            tcp_transport: None,
            quic_transport: None,
            connection_pool: ConnectionPool::new(pool_config),
        }
    }

    /// Enable TCP transport
    pub fn with_tcp(mut self, config: TcpConfig) -> Self {
        self.tcp_transport = Some(TcpTransport::new(config));
        self
    }

    /// Enable QUIC transport
    pub fn with_quic(mut self, config: QuicConfig) -> Self {
        self.quic_transport = Some(QuicTransport::new(config));
        self
    }

    /// Initialize all enabled transports
    pub async fn initialize(&mut self) -> Result<()> {
        if let Some(tcp) = &mut self.tcp_transport {
            tcp.bind().await?;
            tracing::info!("TCP transport initialized");
        }

        if let Some(quic) = &mut self.quic_transport {
            match quic.bind().await {
                Ok(_) => tracing::info!("QUIC transport initialized"),
                Err(e) => tracing::warn!("QUIC transport initialization failed: {}", e),
            }
        }

        Ok(())
    }

    /// Get the connection pool
    pub fn pool(&self) -> &ConnectionPool {
        &self.connection_pool
    }

    /// Check if TCP transport is enabled
    pub fn has_tcp(&self) -> bool {
        self.tcp_transport.is_some()
    }

    /// Check if QUIC transport is enabled
    pub fn has_quic(&self) -> bool {
        self.quic_transport.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_manager_creation() {
        let pool_config = PoolConfig::default();
        let manager = TransportManager::new(pool_config);
        assert!(!manager.has_tcp());
        assert!(!manager.has_quic());
    }

    #[test]
    fn test_transport_manager_with_tcp() {
        let pool_config = PoolConfig::default();
        let tcp_config = TcpConfig::default();
        let manager = TransportManager::new(pool_config).with_tcp(tcp_config);
        assert!(manager.has_tcp());
        assert!(!manager.has_quic());
    }

    #[test]
    fn test_transport_manager_with_quic() {
        let pool_config = PoolConfig::default();
        let quic_config = QuicConfig::default();
        let manager = TransportManager::new(pool_config).with_quic(quic_config);
        assert!(!manager.has_tcp());
        assert!(manager.has_quic());
    }

    #[test]
    fn test_transport_manager_with_both() {
        let pool_config = PoolConfig::default();
        let tcp_config = TcpConfig::default();
        let quic_config = QuicConfig::default();

        let manager = TransportManager::new(pool_config)
            .with_tcp(tcp_config)
            .with_quic(quic_config);

        assert!(manager.has_tcp());
        assert!(manager.has_quic());
    }

    #[tokio::test]
    async fn test_transport_manager_initialize_tcp() {
        let pool_config = PoolConfig::default();
        let mut tcp_config = TcpConfig::default();
        tcp_config.bind_addr = "127.0.0.1:0".parse().unwrap();

        let mut manager = TransportManager::new(pool_config).with_tcp(tcp_config);

        let result = manager.initialize().await;
        assert!(result.is_ok());
    }
}
