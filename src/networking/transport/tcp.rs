// TCP transport implementation for P2P communication
//
// Provides enterprise-grade TCP transport with:
// - Connection pooling
// - Automatic reconnection with exponential backoff
// - Keep-alive and timeout management
// - Low-latency optimizations (TCP_NODELAY)

use crate::common::NodeId;
use crate::error::{DbError, Result};
use bytes::BytesMut;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::time::sleep;

/// TCP transport configuration
#[derive(Debug, Clone)]
pub struct TcpConfig {
    /// Bind address for listening
    pub bind_addr: SocketAddr,

    /// Enable TCP_NODELAY (disable Nagle's algorithm)
    pub nodelay: bool,

    /// SO_KEEPALIVE interval
    pub keepalive_interval: Option<Duration>,

    /// Connection timeout
    pub connect_timeout: Duration,

    /// Read timeout
    pub read_timeout: Duration,

    /// Write timeout
    pub write_timeout: Duration,

    /// Maximum number of reconnection attempts
    pub max_reconnect_attempts: u32,

    /// Initial reconnection backoff duration
    pub reconnect_initial_backoff: Duration,

    /// Maximum reconnection backoff duration
    pub reconnect_max_backoff: Duration,

    /// Send buffer size
    pub send_buffer_size: Option<usize>,

    /// Receive buffer size
    pub recv_buffer_size: Option<usize>,
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:9000".parse().unwrap(),
            nodelay: true, // Disable Nagle for low latency
            keepalive_interval: Some(Duration::from_secs(60)),
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
            max_reconnect_attempts: 5,
            reconnect_initial_backoff: Duration::from_millis(100),
            reconnect_max_backoff: Duration::from_secs(30),
            send_buffer_size: Some(256 * 1024), // 256 KB
            recv_buffer_size: Some(256 * 1024), // 256 KB
        }
    }
}

/// TCP transport for P2P communication
pub struct TcpTransport {
    config: TcpConfig,
    listener: Option<TcpListener>,
}

impl TcpTransport {
    /// Create a new TCP transport with the given configuration
    pub fn new(config: TcpConfig) -> Self {
        Self {
            config,
            listener: None,
        }
    }

    /// Start listening for incoming connections
    pub async fn bind(&mut self) -> Result<()> {
        let listener = TcpListener::bind(self.config.bind_addr)
            .await
            .map_err(|e| DbError::Network(format!("Failed to bind TCP listener: {}", e)))?;

        tracing::info!("TCP transport listening on {}", self.config.bind_addr);
        self.listener = Some(listener);
        Ok(())
    }

    /// Accept an incoming connection
    pub async fn accept(&self) -> Result<TcpConnection> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| DbError::Network("TCP listener not initialized".to_string()))?;

        let (stream, peer_addr) = listener
            .accept()
            .await
            .map_err(|e| DbError::Network(format!("Failed to accept connection: {}", e)))?;

        self.configure_socket(&stream)?;

        tracing::debug!("Accepted TCP connection from {}", peer_addr);

        Ok(TcpConnection {
            stream: Arc::new(RwLock::new(stream)),
            peer_addr,
            config: self.config.clone(),
        })
    }

    /// Connect to a remote peer
    pub async fn connect(&self, addr: SocketAddr, peer_id: NodeId) -> Result<TcpConnection> {
        self.connect_with_retry(addr, peer_id).await
    }

    /// Connect with automatic retry and exponential backoff
    async fn connect_with_retry(&self, addr: SocketAddr, peer_id: NodeId) -> Result<TcpConnection> {
        let mut attempt = 0;
        let mut backoff = self.config.reconnect_initial_backoff;

        loop {
            match tokio::time::timeout(self.config.connect_timeout, TcpStream::connect(addr)).await
            {
                Ok(Ok(stream)) => {
                    self.configure_socket(&stream)?;
                    tracing::info!(
                        "Connected to peer {} at {} (attempt {})",
                        peer_id,
                        addr,
                        attempt + 1
                    );

                    return Ok(TcpConnection {
                        stream: Arc::new(RwLock::new(stream)),
                        peer_addr: addr,
                        config: self.config.clone(),
                    });
                }
                Ok(Err(e)) => {
                    attempt += 1;
                    if attempt >= self.config.max_reconnect_attempts {
                        return Err(DbError::Network(format!(
                            "Failed to connect to {} after {} attempts: {}",
                            addr, attempt, e
                        )));
                    }

                    tracing::warn!(
                        "Failed to connect to {} (attempt {}): {}. Retrying in {:?}",
                        addr,
                        attempt,
                        e,
                        backoff
                    );

                    sleep(backoff).await;
                    backoff = std::cmp::min(backoff * 2, self.config.reconnect_max_backoff);
                }
                Err(_) => {
                    attempt += 1;
                    if attempt >= self.config.max_reconnect_attempts {
                        return Err(DbError::Network(format!(
                            "Connection timeout to {} after {} attempts",
                            addr, attempt
                        )));
                    }

                    tracing::warn!(
                        "Connection timeout to {} (attempt {}). Retrying in {:?}",
                        addr,
                        attempt,
                        backoff
                    );

                    sleep(backoff).await;
                    backoff = std::cmp::min(backoff * 2, self.config.reconnect_max_backoff);
                }
            }
        }
    }

    /// Configure TCP socket options
    fn configure_socket(&self, stream: &TcpStream) -> Result<()> {
        // Enable TCP_NODELAY to disable Nagle's algorithm for low latency
        if self.config.nodelay {
            stream
                .set_nodelay(true)
                .map_err(|e| DbError::Network(format!("Failed to set TCP_NODELAY: {}", e)))?;
        }

        // Configure keepalive
        if let Some(keepalive) = self.config.keepalive_interval {
            let socket = socket2::SockRef::from(stream);
            let keepalive = socket2::TcpKeepalive::new()
                .with_time(keepalive)
                .with_interval(keepalive);

            socket
                .set_tcp_keepalive(&keepalive)
                .map_err(|e| DbError::Network(format!("Failed to set keepalive: {}", e)))?;
        }

        Ok(())
    }
}

/// TCP connection wrapper
pub struct TcpConnection {
    stream: Arc<RwLock<TcpStream>>,
    peer_addr: SocketAddr,
    config: TcpConfig,
}

impl TcpConnection {
    /// Get the peer address
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Send data through the connection
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        let mut stream = self.stream.write().await;

        tokio::time::timeout(self.config.write_timeout, stream.write_all(data))
            .await
            .map_err(|_| DbError::Timeout("Write timeout".to_string()))?
            .map_err(|e| DbError::Network(format!("Failed to send data: {}", e)))?;

        stream
            .flush()
            .await
            .map_err(|e| DbError::Network(format!("Failed to flush data: {}", e)))?;

        Ok(())
    }

    /// Receive data from the connection
    pub async fn recv(&self, buf: &mut BytesMut) -> Result<usize> {
        let mut stream = self.stream.write().await;

        let n = tokio::time::timeout(self.config.read_timeout, stream.read_buf(buf))
            .await
            .map_err(|_| DbError::Timeout("Read timeout".to_string()))?
            .map_err(|e| DbError::Network(format!("Failed to receive data: {}", e)))?;

        Ok(n)
    }

    /// Receive exact number of bytes
    pub async fn recv_exact(&self, buf: &mut [u8]) -> Result<()> {
        let mut stream = self.stream.write().await;

        tokio::time::timeout(self.config.read_timeout, stream.read_exact(buf))
            .await
            .map_err(|_| DbError::Timeout("Read timeout".to_string()))?
            .map_err(|e| DbError::Network(format!("Failed to receive exact data: {}", e)))?;

        Ok(())
    }

    /// Close the connection gracefully
    pub async fn close(&self) -> Result<()> {
        let mut stream = self.stream.write().await;
        stream
            .shutdown()
            .await
            .map_err(|e| DbError::Network(format!("Failed to close connection: {}", e)))?;
        Ok(())
    }

    /// Check if connection is still alive
    pub async fn is_alive(&self) -> bool {
        let stream = self.stream.read().await;
        stream.peer_addr().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_config_default() {
        let config = TcpConfig::default();
        assert!(config.nodelay);
        assert_eq!(config.max_reconnect_attempts, 5);
    }

    #[tokio::test]
    async fn test_tcp_transport_creation() {
        let config = TcpConfig::default();
        let _transport = TcpTransport::new(config);
    }

    #[tokio::test]
    async fn test_tcp_bind() {
        let mut config = TcpConfig::default();
        config.bind_addr = "127.0.0.1:0".parse().unwrap(); // Use random port

        let mut transport = TcpTransport::new(config);
        let result = transport.bind().await;
        assert!(result.is_ok());
    }
}
