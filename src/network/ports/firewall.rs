//! # Firewall-Friendly Configuration
//!
//! Provides firewall traversal capabilities for database communication.
//!
//! ## Features
//!
//! - **Port Probing**: Test port accessibility through firewalls
//! - **Fallback Selection**: Automatically select accessible ports
//! - **HTTP/HTTPS Tunneling**: Tunnel database traffic over HTTP(S)
//! - **WebSocket Upgrade**: Use WebSocket for bi-directional communication

use crate::error::{DbError, Result};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use serde::{Deserialize, Serialize};

/// Firewall configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallConfig {
    /// Enable port probing
    pub enable_probing: bool,

    /// Enable HTTP/WebSocket tunneling
    pub enable_tunneling: bool,

    /// Probe timeout in milliseconds
    pub probe_timeout_ms: u64,
}

impl Default for FirewallConfig {
    fn default() -> Self {
        Self {
            enable_probing: true,
            enable_tunneling: false,
            probe_timeout_ms: 5000,
        }
    }
}

/// Port probe result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeResult {
    /// Port is accessible
    Accessible,

    /// Port is blocked or filtered
    Blocked,

    /// Connection timed out
    Timeout,

    /// Connection refused
    Refused,

    /// Unknown/error
    Error,
}

/// Port probing utility
pub struct PortProbe {
    timeout_duration: Duration,
}

impl PortProbe {
    /// Create a new port probe
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            timeout_duration: Duration::from_millis(timeout_ms),
        }
    }

    /// Probe if a TCP port is accessible
    pub async fn probe_tcp(&self, addr: SocketAddr) -> ProbeResult {
        match timeout(self.timeout_duration, TcpStream::connect(addr)).await {
            Ok(Ok(_)) => ProbeResult::Accessible,
            Ok(Err(e)) => {
                if e.kind() == std::io::ErrorKind::ConnectionRefused {
                    ProbeResult::Refused
                } else {
                    ProbeResult::Error
                }
            }
            Err(_) => ProbeResult::Timeout,
        }
    }

    /// Probe multiple ports and return the first accessible one
    pub async fn probe_first_accessible(&self, addrs: &[SocketAddr]) -> Option<SocketAddr> {
        for &addr in addrs {
            if self.probe_tcp(addr).await == ProbeResult::Accessible {
                return Some(addr);
            }
        }
        None
    }

    /// Probe all ports and return accessibility results
    pub async fn probe_all(&self, addrs: &[SocketAddr]) -> Vec<(SocketAddr, ProbeResult)> {
        let mut results = Vec::new();
        for &addr in addrs {
            let result = self.probe_tcp(addr).await;
            results.push((addr, result));
        }
        results
    }

    /// Probe ports in parallel
    pub async fn probe_parallel(&self, addrs: &[SocketAddr]) -> Vec<(SocketAddr, ProbeResult)> {
        use futures::future::join_all;

        let probes: Vec<_> = addrs.iter().map(|&addr| {
            let timeout_duration = self.timeout_duration;
            async move {
                let result = match timeout(timeout_duration, TcpStream::connect(addr)).await {
                    Ok(Ok(_)) => ProbeResult::Accessible,
                    Ok(Err(e)) => {
                        if e.kind() == std::io::ErrorKind::ConnectionRefused {
                            ProbeResult::Refused
                        } else {
                            ProbeResult::Error
                        }
                    }
                    Err(_) => ProbeResult::Timeout,
                };
                (addr, result)
            }
        }).collect();

        join_all(probes).await
    }
}

impl Default for PortProbe {
    fn default() -> Self {
        Self::new(5000)
    }
}

/// Fallback port selector
pub struct FallbackPortSelector {
    /// Preferred ports in order
    preferred_ports: Vec<u16>,

    /// Well-known firewall-friendly ports
    well_known_ports: Vec<u16>,
}

impl FallbackPortSelector {
    /// Create a new fallback port selector
    pub fn new(preferred_ports: Vec<u16>) -> Self {
        Self {
            preferred_ports,
            well_known_ports: vec![
                443,  // HTTPS
                80,   // HTTP
                8080, // HTTP alternate
                8443, // HTTPS alternate
                3306, // MySQL (often allowed)
                5432, // PostgreSQL
            ],
        }
    }

    /// Get all candidate ports in priority order
    pub fn get_candidates(&self) -> Vec<u16> {
        let mut ports = self.preferred_ports.clone();

        // Add well-known ports that aren't already in preferred
        for &port in &self.well_known_ports {
            if !ports.contains(&port) {
                ports.push(port);
            }
        }

        ports
    }

    /// Select the best port from probe results
    pub fn select_best(&self, results: &[(SocketAddr, ProbeResult)]) -> Option<SocketAddr> {
        let candidates = self.get_candidates();

        // First, try preferred ports that are accessible
        for &port in &candidates {
            for &(addr, result) in results {
                if addr.port() == port && result == ProbeResult::Accessible {
                    return Some(addr);
                }
            }
        }

        // Fall back to any accessible port
        for &(addr, result) in results {
            if result == ProbeResult::Accessible {
                return Some(addr);
            }
        }

        None
    }
}

impl Default for FallbackPortSelector {
    fn default() -> Self {
        Self::new(vec![5432])
    }
}

/// HTTP/WebSocket tunneling support
pub struct TunnelingSupport {
    /// Enable HTTP tunneling
    enable_http: bool,

    /// Enable WebSocket tunneling
    enable_websocket: bool,
}

impl TunnelingSupport {
    /// Create new tunneling support
    pub fn new(enable_http: bool, enable_websocket: bool) -> Self {
        Self {
            enable_http,
            enable_websocket,
        }
    }

    /// Check if tunneling is enabled
    pub fn is_enabled(&self) -> bool {
        self.enable_http || self.enable_websocket
    }

    /// Get available tunneling methods
    pub fn get_methods(&self) -> Vec<&str> {
        let mut methods = Vec::new();
        if self.enable_http {
            methods.push("http");
        }
        if self.enable_websocket {
            methods.push("websocket");
        }
        methods
    }

    /// Create a WebSocket upgrade request
    pub fn create_websocket_upgrade_request(&self, host: &str, path: &str) -> String {
        use base64::Engine;
        let key = base64::engine::general_purpose::STANDARD.encode(rand::random::<[u8; 16]>());

        format!(
            "GET {} HTTP/1.1\r\n\
             Host: {}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {}\r\n\
             Sec-WebSocket-Version: 13\r\n\
             \r\n",
            path, host, key
        )
    }
}

impl Default for TunnelingSupport {
    fn default() -> Self {
        Self::new(false, true)
    }
}

/// Firewall manager coordinates firewall traversal
pub struct FirewallManager {
    config: FirewallConfig,
    probe: PortProbe,
    fallback_selector: FallbackPortSelector,
    tunneling: TunnelingSupport,
}

impl FirewallManager {
    /// Create a new firewall manager
    pub fn new(config: FirewallConfig) -> Self {
        let probe = PortProbe::new(config.probe_timeout_ms);
        let fallback_selector = FallbackPortSelector::default();
        let tunneling = TunnelingSupport::new(
            config.enable_tunneling,
            config.enable_tunneling,
        );

        Self {
            config,
            probe,
            fallback_selector,
            tunneling,
        }
    }

    /// Probe if a port is accessible
    pub async fn probe_port(&self, addr: &SocketAddr) -> Result<bool> {
        if !self.config.enable_probing {
            return Ok(true); // Assume accessible if probing disabled
        }

        let result = self.probe.probe_tcp(*addr).await;
        Ok(result == ProbeResult::Accessible)
    }

    /// Find the best accessible port from a list
    pub async fn find_accessible_port(&self, addrs: &[SocketAddr]) -> Result<SocketAddr> {
        if !self.config.enable_probing {
            return addrs.first()
                .copied()
                .ok_or_else(|| DbError::InvalidInput("No addresses provided".to_string()));
        }

        // Probe all ports in parallel
        let results = self.probe.probe_parallel(addrs).await;

        // Select the best port
        self.fallback_selector.select_best(&results)
            .ok_or_else(|| DbError::Network("No accessible ports found".to_string()))
    }

    /// Get probe results for multiple addresses
    pub async fn get_probe_results(&self, addrs: &[SocketAddr]) -> Vec<(SocketAddr, ProbeResult)> {
        if !self.config.enable_probing {
            return addrs.iter().map(|&addr| (addr, ProbeResult::Accessible)).collect();
        }

        self.probe.probe_parallel(addrs).await
    }

    /// Check if tunneling is available
    pub fn is_tunneling_enabled(&self) -> bool {
        self.config.enable_tunneling && self.tunneling.is_enabled()
    }

    /// Get available tunneling methods
    pub fn get_tunneling_methods(&self) -> Vec<&str> {
        if self.config.enable_tunneling {
            self.tunneling.get_methods()
        } else {
            Vec::new()
        }
    }

    /// Create a WebSocket upgrade request
    pub fn create_websocket_upgrade(&self, host: &str, path: &str) -> Result<String> {
        if !self.is_tunneling_enabled() {
            return Err(DbError::Configuration("Tunneling not enabled".to_string()));
        }

        Ok(self.tunneling.create_websocket_upgrade_request(host, path))
    }

    /// Generate fallback port candidates
    pub fn get_fallback_candidates(&self, base_port: u16) -> Vec<u16> {
        let selector = FallbackPortSelector::new(vec![base_port]);
        selector.get_candidates()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firewall_config() {
        let config = FirewallConfig::default();
        assert!(config.enable_probing);
        assert_eq!(config.probe_timeout_ms, 5000);
    }

    #[test]
    fn test_fallback_selector() {
        let selector = FallbackPortSelector::new(vec![5432, 6000]);
        let candidates = selector.get_candidates();

        assert!(candidates.contains(&5432));
        assert!(candidates.contains(&6000));
        assert!(candidates.contains(&443)); // Well-known port
    }

    #[test]
    fn test_tunneling_support() {
        let tunneling = TunnelingSupport::new(true, true);
        assert!(tunneling.is_enabled());

        let methods = tunneling.get_methods();
        assert_eq!(methods.len(), 2);
        assert!(methods.contains(&"http"));
        assert!(methods.contains(&"websocket"));
    }

    #[test]
    fn test_websocket_upgrade_request() {
        let tunneling = TunnelingSupport::default();
        let request = tunneling.create_websocket_upgrade_request("example.com", "/ws");

        assert!(request.contains("GET /ws HTTP/1.1"));
        assert!(request.contains("Host: example.com"));
        assert!(request.contains("Upgrade: websocket"));
        assert!(request.contains("Sec-WebSocket-Key:"));
    }

    #[tokio::test]
    async fn test_firewall_manager() {
        let config = FirewallConfig::default();
        let manager = FirewallManager::new(config);

        let candidates = manager.get_fallback_candidates(5432);
        assert!(candidates.contains(&5432));
        assert!(candidates.contains(&443));
    }

    #[test]
    fn test_port_probe_creation() {
        let probe = PortProbe::new(3000);
        assert_eq!(probe.timeout_duration, Duration::from_millis(3000));
    }
}
