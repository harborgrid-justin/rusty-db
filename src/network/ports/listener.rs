//! # Multi-Port Listener Management
//!
//! Manages multiple network listeners with support for:
//! - IPv4 and IPv6 dual-stack
//! - Unix domain sockets
//! - TCP and UDP protocols
//! - Port reuse configuration

use crate::error::{DbError, Result};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Listener configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenerConfig {
    /// Enable IPv6 dual-stack support
    pub enable_ipv6: bool,

    /// Enable Unix domain socket support
    pub enable_unix_sockets: bool,

    /// Enable SO_REUSEPORT
    pub reuse_port: bool,

    /// Enable SO_REUSEADDR
    pub reuse_addr: bool,
}

impl Default for ListenerConfig {
    fn default() -> Self {
        Self {
            enable_ipv6: true,
            enable_unix_sockets: true,
            reuse_port: true,
            reuse_addr: true,
        }
    }
}

/// Protocol type for listeners
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    /// TCP protocol
    Tcp,
    /// UDP protocol
    Udp,
}

/// Active listener instance
pub struct Listener {
    /// Bind address
    addr: SocketAddr,

    /// Protocol type
    protocol: Protocol,

    /// TCP listener (if TCP)
    tcp_listener: Option<Arc<TcpListener>>,

    /// UDP socket (if UDP)
    udp_socket: Option<Arc<UdpSocket>>,
}

impl Listener {
    /// Create a new TCP listener
    pub async fn new_tcp(addr: SocketAddr, config: &ListenerConfig) -> Result<Self> {
        let socket = Self::create_tcp_socket(&addr, config)?;
        socket.bind(addr).map_err(|e| {
            DbError::Network(format!("Failed to bind TCP to {}: {}", addr, e))
        })?;

        let tcp_listener = socket.listen(1024).map_err(|e| {
            DbError::Network(format!("Failed to listen on {}: {}", addr, e))
        })?;

        Ok(Self {
            addr,
            protocol: Protocol::Tcp,
            tcp_listener: Some(Arc::new(tcp_listener)),
            udp_socket: None,
        })
    }

    /// Create a new UDP listener
    pub async fn new_udp(addr: SocketAddr, config: &ListenerConfig) -> Result<Self> {
        let socket = Self::create_udp_socket(&addr, config)?;
        socket.bind(addr).await.map_err(|e| {
            DbError::Network(format!("Failed to bind UDP to {}: {}", addr, e))
        })?;

        Ok(Self {
            addr,
            protocol: Protocol::Udp,
            tcp_listener: None,
            udp_socket: Some(Arc::new(socket)),
        })
    }

    /// Create a TCP socket with configuration
    fn create_tcp_socket(addr: &SocketAddr, config: &ListenerConfig) -> Result<socket2::Socket> {
        let domain = if addr.is_ipv4() {
            socket2::Domain::IPV4
        } else {
            socket2::Domain::IPV6
        };

        let socket = socket2::Socket::new(domain, socket2::Type::STREAM, Some(socket2::Protocol::TCP))
            .map_err(|e| DbError::Network(format!("Failed to create TCP socket: {}", e)))?;

        if config.reuse_addr {
            socket.set_reuse_address(true)
                .map_err(|e| DbError::Network(format!("Failed to set SO_REUSEADDR: {}", e)))?;
        }

        #[cfg(unix)]
        if config.reuse_port {
            socket.set_reuse_port(true)
                .map_err(|e| DbError::Network(format!("Failed to set SO_REUSEPORT: {}", e)))?;
        }

        // Enable dual-stack for IPv6
        if addr.is_ipv6() && config.enable_ipv6 {
            socket.set_only_v6(false)
                .map_err(|e| DbError::Network(format!("Failed to set dual-stack: {}", e)))?;
        }

        socket.set_nonblocking(true)
            .map_err(|e| DbError::Network(format!("Failed to set non-blocking: {}", e)))?;

        Ok(socket)
    }

    /// Create a UDP socket with configuration
    fn create_udp_socket(addr: &SocketAddr, config: &ListenerConfig) -> Result<UdpSocket> {
        let domain = if addr.is_ipv4() {
            socket2::Domain::IPV4
        } else {
            socket2::Domain::IPV6
        };

        let socket = socket2::Socket::new(domain, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))
            .map_err(|e| DbError::Network(format!("Failed to create UDP socket: {}", e)))?;

        if config.reuse_addr {
            socket.set_reuse_address(true)
                .map_err(|e| DbError::Network(format!("Failed to set SO_REUSEADDR: {}", e)))?;
        }

        #[cfg(unix)]
        if config.reuse_port {
            socket.set_reuse_port(true)
                .map_err(|e| DbError::Network(format!("Failed to set SO_REUSEPORT: {}", e)))?;
        }

        if addr.is_ipv6() && config.enable_ipv6 {
            socket.set_only_v6(false)
                .map_err(|e| DbError::Network(format!("Failed to set dual-stack: {}", e)))?;
        }

        socket.set_nonblocking(true)
            .map_err(|e| DbError::Network(format!("Failed to set non-blocking: {}", e)))?;

        let std_socket: std::net::UdpSocket = socket.into();
        UdpSocket::from_std(std_socket)
            .map_err(|e| DbError::Network(format!("Failed to create tokio UdpSocket: {}", e)))
    }

    /// Get the local address
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Get the protocol
    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    /// Get TCP listener reference (if TCP)
    pub fn tcp_listener(&self) -> Option<Arc<TcpListener>> {
        self.tcp_listener.clone()
    }

    /// Get UDP socket reference (if UDP)
    pub fn udp_socket(&self) -> Option<Arc<UdpSocket>> {
        self.udp_socket.clone()
    }
}

/// Manages multiple listeners across different addresses and ports
pub struct ListenerManager {
    config: ListenerConfig,
    listeners: Arc<RwLock<HashMap<SocketAddr, Listener>>>,
    port_listeners: Arc<RwLock<HashMap<u16, Vec<SocketAddr>>>>,
}

impl ListenerManager {
    /// Create a new listener manager
    pub fn new(config: ListenerConfig) -> Self {
        Self {
            config,
            listeners: Arc::new(RwLock::new(HashMap::new())),
            port_listeners: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start TCP listeners on multiple addresses
    pub async fn start_listeners(&mut self, addrs: &[SocketAddr]) -> Result<()> {
        let mut listeners_guard = self.listeners.write().await;
        let mut port_listeners_guard = self.port_listeners.write().await;

        for &addr in addrs {
            if listeners_guard.contains_key(&addr) {
                continue; // Already listening
            }

            let listener = Listener::new_tcp(addr, &self.config).await?;
            let port = addr.port();

            port_listeners_guard
                .entry(port)
                .or_insert_with(Vec::new)
                .push(addr);

            listeners_guard.insert(addr, listener);
        }

        Ok(())
    }

    /// Start a UDP listener on an address
    pub async fn start_udp_listener(&mut self, addr: SocketAddr) -> Result<()> {
        let mut listeners_guard = self.listeners.write().await;
        let mut port_listeners_guard = self.port_listeners.write().await;

        if listeners_guard.contains_key(&addr) {
            return Ok(()); // Already listening
        }

        let listener = Listener::new_udp(addr, &self.config).await?;
        let port = addr.port();

        port_listeners_guard
            .entry(port)
            .or_insert_with(Vec::new)
            .push(addr);

        listeners_guard.insert(addr, listener);

        Ok(())
    }

    /// Stop all listeners on a specific port
    pub async fn stop_listeners(&mut self, port: u16) -> Result<()> {
        let mut listeners_guard = self.listeners.write().await;
        let mut port_listeners_guard = self.port_listeners.write().await;

        if let Some(addrs) = port_listeners_guard.remove(&port) {
            for addr in addrs {
                listeners_guard.remove(&addr);
            }
        }

        Ok(())
    }

    /// Stop a specific listener by address
    pub async fn stop_listener(&mut self, addr: &SocketAddr) -> Result<()> {
        let mut listeners_guard = self.listeners.write().await;
        let mut port_listeners_guard = self.port_listeners.write().await;

        if listeners_guard.remove(addr).is_some() {
            let port = addr.port();
            if let Some(addrs) = port_listeners_guard.get_mut(&port) {
                addrs.retain(|a| a != addr);
                if addrs.is_empty() {
                    port_listeners_guard.remove(&port);
                }
            }
        }

        Ok(())
    }

    /// Get all active listeners
    pub async fn get_listeners(&self) -> Vec<SocketAddr> {
        let listeners_guard = self.listeners.read().await;
        listeners_guard.keys().copied().collect()
    }

    /// Get listeners for a specific port
    pub async fn get_port_listeners(&self, port: u16) -> Vec<SocketAddr> {
        let port_listeners_guard = self.port_listeners.read().await;
        port_listeners_guard
            .get(&port)
            .cloned()
            .unwrap_or_default()
    }

    /// Check if listening on a specific address
    pub async fn is_listening(&self, addr: &SocketAddr) -> bool {
        let listeners_guard = self.listeners.read().await;
        listeners_guard.contains_key(addr)
    }

    /// Get a listener by address
    pub async fn get_listener(&self, addr: &SocketAddr) -> Option<Arc<TcpListener>> {
        let listeners_guard = self.listeners.read().await;
        listeners_guard.get(addr).and_then(|l| l.tcp_listener())
    }

    /// Get a UDP socket by address
    pub async fn get_udp_socket(&self, addr: &SocketAddr) -> Option<Arc<UdpSocket>> {
        let listeners_guard = self.listeners.read().await;
        listeners_guard.get(addr).and_then(|l| l.udp_socket())
    }

    /// Get the number of active listeners
    pub async fn listener_count(&self) -> usize {
        let listeners_guard = self.listeners.read().await;
        listeners_guard.len()
    }

    /// Stop all listeners
    pub async fn stop_all(&mut self) -> Result<()> {
        let mut listeners_guard = self.listeners.write().await;
        let mut port_listeners_guard = self.port_listeners.write().await;

        listeners_guard.clear();
        port_listeners_guard.clear();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tcp_listener_creation() {
        let config = ListenerConfig::default();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

        let listener = Listener::new_tcp(addr, &config).await.unwrap();
        assert_eq!(listener.protocol(), Protocol::Tcp);
        assert!(listener.tcp_listener().is_some());
    }

    #[tokio::test]
    async fn test_listener_manager() {
        let config = ListenerConfig::default();
        let mut manager = ListenerManager::new(config);

        let addr1: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let addr2: SocketAddr = "127.0.0.1:0".parse().unwrap();

        manager.start_listeners(&[addr1, addr2]).await.unwrap();

        assert_eq!(manager.listener_count().await, 2);
    }

    #[tokio::test]
    async fn test_stop_listeners() {
        let config = ListenerConfig::default();
        let mut manager = ListenerManager::new(config);

        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        manager.start_listeners(&[addr]).await.unwrap();

        assert_eq!(manager.listener_count().await, 1);

        manager.stop_listener(&addr).await.unwrap();
        assert_eq!(manager.listener_count().await, 0);
    }
}
