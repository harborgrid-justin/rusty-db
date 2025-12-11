//! # Port Management Module
//!
//! Enterprise-grade port management for RustyDB distributed database.
//!
//! ## Features
//!
//! - **Dynamic Port Allocation**: Multiple allocation strategies (sequential, random, hash-based)
//! - **Multi-Protocol Support**: TCP, UDP, Unix domain sockets, IPv4/IPv6 dual-stack
//! - **NAT Traversal**: STUN, UPnP, NAT-PMP, ICE-lite
//! - **Firewall-Friendly**: Port probing, fallback selection, HTTP/WebSocket tunneling
//! - **Service Discovery**: Hostname resolution, SRV records, load-balanced endpoints
//! - **Health Monitoring**: Port availability checks, conflict detection, exhaustion monitoring
//!
//! ## Example
//!
//! ```rust,no_run
//! use rusty_db::network::ports::{PortManager, PortConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = PortConfig::default();
//!     let mut manager = PortManager::new(config);
//!
//!     // Allocate a port
//!     let port = manager.allocate_port().await?;
//!     println!("Allocated port: {}", port);
//!
//!     // Start listener
//!     manager.start_listener(port).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod allocator;
pub mod listener;
pub mod nat;
pub mod firewall;
pub mod resolver;
pub mod mapping;
pub mod health;

use crate::error::{DbError, Result};
use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr, IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

pub use allocator::{PortAllocator, AllocationStrategy};
pub use listener::{ListenerManager, Listener, ListenerConfig};
pub use nat::{NatTraversal, StunClient, UpnpClient, NatMapping};
pub use firewall::{FirewallManager, PortProbe, FirewallConfig};
pub use resolver::{AddressResolver, ResolverConfig, ResolvedEndpoint};
pub use mapping::{PortMappingService, ServiceRegistry, PortMapping};
pub use health::{PortHealthChecker, HealthStatus, HealthCheckConfig};

/// Port configuration for the port manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortConfig {
    /// Base port for database services
    pub base_port: u16,

    /// Port range for dynamic allocation
    pub port_range_start: u16,
    pub port_range_end: u16,

    /// Enable IPv6 support
    pub enable_ipv6: bool,

    /// Enable Unix domain socket support
    pub enable_unix_sockets: bool,

    /// Enable NAT traversal
    pub enable_nat_traversal: bool,

    /// Enable firewall-friendly features
    pub enable_firewall_friendly: bool,

    /// Bind addresses (e.g., ["0.0.0.0:5432", "[::]:5432"])
    pub bind_addresses: Vec<String>,

    /// Port reuse configuration
    pub reuse_port: bool,
    pub reuse_addr: bool,

    /// Health check interval in seconds
    pub health_check_interval: u64,
}

impl Default for PortConfig {
    fn default() -> Self {
        Self {
            base_port: 5432,
            port_range_start: 6000,
            port_range_end: 7000,
            enable_ipv6: true,
            enable_unix_sockets: true,
            enable_nat_traversal: false,
            enable_firewall_friendly: true,
            bind_addresses: vec![
                "0.0.0.0:5432".to_string(),
            ],
            reuse_port: true,
            reuse_addr: true,
            health_check_interval: 60,
        }
    }
}

/// Service type for port allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceType {
    /// Main database service
    Database,
    /// Cluster communication
    Cluster,
    /// Replication
    Replication,
    /// Admin/monitoring
    Admin,
    /// API gateway
    Api,
    /// Custom service
    Custom,
}

/// Port manager coordinates all port-related operations
pub struct PortManager {
    config: PortConfig,
    allocator: Arc<RwLock<PortAllocator>>,
    listener_manager: Arc<RwLock<ListenerManager>>,
    nat_traversal: Option<Arc<RwLock<NatTraversal>>>,
    firewall_manager: Arc<RwLock<FirewallManager>>,
    resolver: Arc<RwLock<AddressResolver>>,
    mapping_service: Arc<RwLock<PortMappingService>>,
    health_checker: Arc<RwLock<PortHealthChecker>>,
    allocated_ports: Arc<RwLock<HashMap<ServiceType, Vec<u16>>>>,
}

impl PortManager {
    /// Create a new port manager
    pub fn new(config: PortConfig) -> Self {
        let allocator = Arc::new(RwLock::new(PortAllocator::new(
            config.port_range_start,
            config.port_range_end,
            AllocationStrategy::Sequential,
        )));

        let listener_config = ListenerConfig {
            enable_ipv6: config.enable_ipv6,
            enable_unix_sockets: config.enable_unix_sockets,
            reuse_port: config.reuse_port,
            reuse_addr: config.reuse_addr,
        };
        let listener_manager = Arc::new(RwLock::new(ListenerManager::new(listener_config)));

        let nat_traversal = if config.enable_nat_traversal {
            Some(Arc::new(RwLock::new(NatTraversal::new())))
        } else {
            None
        };

        let firewall_config = FirewallConfig {
            enable_probing: config.enable_firewall_friendly,
            enable_tunneling: config.enable_firewall_friendly,
            probe_timeout_ms: 5000,
        };
        let firewall_manager = Arc::new(RwLock::new(FirewallManager::new(firewall_config)));

        let resolver_config = ResolverConfig {
            cache_ttl: 300,
            max_cache_size: 1000,
        };
        let resolver = Arc::new(RwLock::new(AddressResolver::new(resolver_config)));

        let mapping_service = Arc::new(RwLock::new(PortMappingService::new()));

        let health_config = HealthCheckConfig {
            check_interval: config.health_check_interval,
            timeout_ms: 5000,
        };
        let health_checker = Arc::new(RwLock::new(PortHealthChecker::new(health_config)));

        Self {
            config,
            allocator,
            listener_manager,
            nat_traversal,
            firewall_manager,
            resolver,
            mapping_service,
            health_checker,
            allocated_ports: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Allocate a port for a specific service type
    pub async fn allocate_port(&self, service_type: ServiceType) -> Result<u16> {
        let port = self.allocator.write().await.allocate()
            .ok_or_else(|| DbError::ResourceExhausted("No ports available".to_string()))?;

        // Register the port
        let mut allocated = self.allocated_ports.write().await;
        allocated.entry(service_type).or_insert_with(Vec::new).push(port);

        // Register with mapping service
        self.mapping_service.write().await.register_port(
            service_type,
            port,
            format!("{:?}", service_type),
        );

        Ok(port)
    }

    /// Release a previously allocated port
    pub async fn release_port(&self, service_type: ServiceType, port: u16) -> Result<()> {
        self.allocator.write().await.release(port);

        let mut allocated = self.allocated_ports.write().await;
        if let Some(ports) = allocated.get_mut(&service_type) {
            ports.retain(|&p| p != port);
        }

        self.mapping_service.write().await.unregister_port(port);

        Ok(())
    }

    /// Start a listener on the specified port
    pub async fn start_listener(&self, port: u16) -> Result<()> {
        let addrs: Vec<SocketAddr> = self.config.bind_addresses
            .iter()
            .filter_map(|s| s.parse().ok())
            .map(|mut addr: SocketAddr| {
                addr.set_port(port);
                addr
            })
            .collect();

        if addrs.is_empty() {
            return Err(DbError::Configuration("No valid bind addresses".to_string()));
        }

        self.listener_manager.write().await.start_listeners(&addrs).await?;

        // Set up NAT traversal if enabled
        if let Some(nat) = &self.nat_traversal {
            let mut nat_guard = nat.write().await;
            nat_guard.setup_port_mapping(port).await?;
        }

        Ok(())
    }

    /// Stop listener on the specified port
    pub async fn stop_listener(&self, port: u16) -> Result<()> {
        self.listener_manager.write().await.stop_listeners(port).await?;

        // Clean up NAT mappings
        if let Some(nat) = &self.nat_traversal {
            let mut nat_guard = nat.write().await;
            nat_guard.remove_port_mapping(port).await?;
        }

        Ok(())
    }

    /// Resolve an address to a list of endpoints
    pub async fn resolve_address(&self, address: &str) -> Result<Vec<ResolvedEndpoint>> {
        self.resolver.write().await.resolve(address).await
    }

    /// Check health of all allocated ports
    pub async fn check_health(&self) -> Result<HashMap<u16, HealthStatus>> {
        let allocated = self.allocated_ports.read().await;
        let all_ports: Vec<u16> = allocated.values()
            .flat_map(|v| v.iter().copied())
            .collect();

        self.health_checker.write().await.check_ports(&all_ports).await
    }

    /// Get all allocated ports by service type
    pub async fn get_allocated_ports(&self, service_type: ServiceType) -> Vec<u16> {
        let allocated = self.allocated_ports.read().await;
        allocated.get(&service_type).cloned().unwrap_or_default()
    }

    /// Get port mapping information
    pub async fn get_port_mapping(&self, port: u16) -> Option<PortMapping> {
        self.mapping_service.read().await.get_mapping(port)
    }

    /// Probe if a port is accessible through firewall
    pub async fn probe_port(&self, addr: &SocketAddr) -> Result<bool> {
        self.firewall_manager.write().await.probe_port(addr).await
    }

    /// Get external IP address (via STUN)
    pub async fn get_external_ip(&self) -> Result<IpAddr> {
        if let Some(nat) = &self.nat_traversal {
            nat.write().await.get_external_ip().await
        } else {
            Err(DbError::Configuration("NAT traversal not enabled".to_string()))
        }
    }

    /// Shutdown all port operations
    pub async fn shutdown(&self) -> Result<()> {
        // Stop all listeners
        let allocated = self.allocated_ports.read().await;
        let all_ports: Vec<u16> = allocated.values()
            .flat_map(|v| v.iter().copied())
            .collect();

        for port in all_ports {
            let _ = self.stop_listener(port).await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_port_allocation() {
        let config = PortConfig::default();
        let manager = PortManager::new(config);

        let port = manager.allocate_port(ServiceType::Database).await.unwrap();
        assert!(port >= 6000 && port <= 7000);

        manager.release_port(ServiceType::Database, port).await.unwrap();
    }

    #[tokio::test]
    async fn test_multiple_services() {
        let config = PortConfig::default();
        let manager = PortManager::new(config);

        let db_port = manager.allocate_port(ServiceType::Database).await.unwrap();
        let cluster_port = manager.allocate_port(ServiceType::Cluster).await.unwrap();
        let api_port = manager.allocate_port(ServiceType::Api).await.unwrap();

        assert_ne!(db_port, cluster_port);
        assert_ne!(db_port, api_port);
        assert_ne!(cluster_port, api_port);

        let allocated_db = manager.get_allocated_ports(ServiceType::Database).await;
        assert_eq!(allocated_db.len(), 1);
        assert_eq!(allocated_db[0], db_port);
    }
}
