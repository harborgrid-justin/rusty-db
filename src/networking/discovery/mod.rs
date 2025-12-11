//! Service Discovery Module
//!
//! Provides comprehensive service discovery capabilities for RustyDB distributed database.
//! Supports multiple discovery backends including DNS, Kubernetes, Consul, etcd, and cloud providers.
//!
//! # Overview
//!
//! Service discovery enables nodes in a distributed database cluster to find and communicate
//! with each other. This module provides a pluggable architecture supporting multiple discovery
//! mechanisms that can be used independently or in combination.
//!
//! # Supported Backends
//!
//! - **DNS**: SRV/A/AAAA record-based discovery with TTL caching
//! - **Static**: File-based seed lists with hot reload
//! - **Kubernetes**: Native K8s service discovery via API
//! - **Consul**: HashiCorp Consul integration
//! - **etcd**: Distributed KV-based discovery
//! - **Cloud**: AWS, Azure, GCP instance discovery
//!
//! # Example
//!
//! ```rust,no_run
//! use rusty_db::networking::discovery::{DiscoveryConfig, Registry};
//!
//! # async fn example() -> rusty_db::error::Result<()> {
//! let config = DiscoveryConfig::default();
//! let mut registry = Registry::new(config);
//! registry.start().await?;
//!
//! let nodes = registry.discover_nodes().await?;
//! for node in nodes {
//!     println!("Discovered node: {}:{}", node.address, node.port);
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::{DbError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub mod dns;
pub mod static_list;
pub mod kubernetes;
pub mod consul;
pub mod etcd;
pub mod cloud;
pub mod registry;

// Re-exports for convenience
pub use registry::Registry;

/// Represents a discovered node in the cluster
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Node {
    /// Unique node identifier
    pub id: String,

    /// IP address of the node
    pub address: IpAddr,

    /// Port number
    pub port: u16,

    /// Node datacenter or availability zone
    pub datacenter: Option<String>,

    /// Node rack identifier
    pub rack: Option<String>,

    /// Node health status
    pub health: HealthStatus,

    /// Custom metadata tags
    pub metadata: HashMap<String, String>,

    /// Last seen timestamp (Unix epoch seconds)
    pub last_seen: u64,
}

impl Node {
    /// Creates a new node with the given parameters
    pub fn new(id: String, address: IpAddr, port: u16) -> Self {
        Self {
            id,
            address,
            port,
            datacenter: None,
            rack: None,
            health: HealthStatus::Unknown,
            metadata: HashMap::new(),
            last_seen: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Returns the full address as a string (IP:port)
    pub fn full_address(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }

    /// Checks if the node is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.health, HealthStatus::Healthy)
    }

    /// Updates the last seen timestamp to now
    pub fn touch(&mut self) {
        self.last_seen = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

/// Health status of a discovered node
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HealthStatus {
    /// Node is healthy and available
    Healthy,

    /// Node is degraded but still functional
    Degraded,

    /// Node is unhealthy or unreachable
    Unhealthy,

    /// Health status is unknown
    Unknown,
}

/// Event emitted when node state changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryEvent {
    /// A new node was discovered
    NodeAdded(Node),

    /// An existing node was updated
    NodeUpdated(Node),

    /// A node was removed or became unreachable
    NodeRemoved(String), // node_id

    /// Health status of a node changed
    HealthChanged {
        node_id: String,
        old_status: HealthStatus,
        new_status: HealthStatus,
    },
}

/// Configuration for service discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Enable DNS-based discovery
    pub enable_dns: bool,

    /// DNS configuration
    pub dns_config: Option<DnsConfig>,

    /// Enable static seed list discovery
    pub enable_static: bool,

    /// Static configuration
    pub static_config: Option<StaticConfig>,

    /// Enable Kubernetes discovery
    pub enable_kubernetes: bool,

    /// Kubernetes configuration
    pub kubernetes_config: Option<KubernetesConfig>,

    /// Enable Consul discovery
    pub enable_consul: bool,

    /// Consul configuration
    pub consul_config: Option<ConsulConfig>,

    /// Enable etcd discovery
    pub enable_etcd: bool,

    /// etcd configuration
    pub etcd_config: Option<EtcdConfig>,

    /// Enable cloud provider discovery
    pub enable_cloud: bool,

    /// Cloud configuration
    pub cloud_config: Option<CloudConfig>,

    /// Refresh interval for periodic discovery
    pub refresh_interval: Duration,

    /// Timeout for discovery operations
    pub discovery_timeout: Duration,

    /// Maximum age before a node is considered stale
    pub node_ttl: Duration,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enable_dns: false,
            dns_config: None,
            enable_static: true,
            static_config: Some(StaticConfig::default()),
            enable_kubernetes: false,
            kubernetes_config: None,
            enable_consul: false,
            consul_config: None,
            enable_etcd: false,
            etcd_config: None,
            enable_cloud: false,
            cloud_config: None,
            refresh_interval: Duration::from_secs(30),
            discovery_timeout: Duration::from_secs(10),
            node_ttl: Duration::from_secs(300),
        }
    }
}

/// DNS discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// Service name for SRV lookup
    pub service_name: String,

    /// Domain for DNS queries
    pub domain: String,

    /// Custom DNS servers (if empty, use system defaults)
    pub nameservers: Vec<String>,

    /// Refresh interval for DNS lookups
    pub refresh_interval: Duration,

    /// Enable DNS caching
    pub enable_cache: bool,

    /// Protocol (tcp or udp)
    pub protocol: String,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            service_name: "rustydb".to_string(),
            domain: "cluster.local".to_string(),
            nameservers: vec![],
            refresh_interval: Duration::from_secs(30),
            enable_cache: true,
            protocol: "tcp".to_string(),
        }
    }
}

/// Static seed list configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticConfig {
    /// List of seed nodes (format: "host:port")
    pub seed_nodes: Vec<String>,

    /// Path to seed file (optional)
    pub seed_file: Option<String>,

    /// Enable hot reload of seed file
    pub hot_reload: bool,

    /// Hot reload check interval
    pub reload_interval: Duration,
}

impl Default for StaticConfig {
    fn default() -> Self {
        Self {
            seed_nodes: vec!["127.0.0.1:5432".to_string()],
            seed_file: None,
            hot_reload: false,
            reload_interval: Duration::from_secs(60),
        }
    }
}

/// Kubernetes discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesConfig {
    /// Kubernetes namespace
    pub namespace: String,

    /// Service name
    pub service_name: String,

    /// Label selector
    pub label_selector: Option<String>,

    /// Port name to use
    pub port_name: String,

    /// Use in-cluster config
    pub in_cluster: bool,

    /// Kubeconfig path (if not in-cluster)
    pub kubeconfig_path: Option<String>,
}

impl Default for KubernetesConfig {
    fn default() -> Self {
        Self {
            namespace: "default".to_string(),
            service_name: "rustydb".to_string(),
            label_selector: Some("app=rustydb".to_string()),
            port_name: "database".to_string(),
            in_cluster: true,
            kubeconfig_path: None,
        }
    }
}

/// Consul discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsulConfig {
    /// Consul agent address
    pub address: String,

    /// Service name to register/discover
    pub service_name: String,

    /// Service tags
    pub tags: Vec<String>,

    /// Datacenter
    pub datacenter: Option<String>,

    /// Enable health checks
    pub enable_health_check: bool,

    /// Health check interval
    pub health_check_interval: Duration,

    /// ACL token (if required)
    pub token: Option<String>,
}

impl Default for ConsulConfig {
    fn default() -> Self {
        Self {
            address: "http://127.0.0.1:8500".to_string(),
            service_name: "rustydb".to_string(),
            tags: vec!["database".to_string()],
            datacenter: None,
            enable_health_check: true,
            health_check_interval: Duration::from_secs(10),
            token: None,
        }
    }
}

/// etcd discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtcdConfig {
    /// etcd endpoints
    pub endpoints: Vec<String>,

    /// Key prefix for service registration
    pub key_prefix: String,

    /// Lease TTL in seconds
    pub lease_ttl: u64,

    /// Username for authentication
    pub username: Option<String>,

    /// Password for authentication
    pub password: Option<String>,
}

impl Default for EtcdConfig {
    fn default() -> Self {
        Self {
            endpoints: vec!["http://127.0.0.1:2379".to_string()],
            key_prefix: "/rustydb/nodes".to_string(),
            lease_ttl: 60,
            username: None,
            password: None,
        }
    }
}

/// Cloud provider discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    /// Cloud provider type
    pub provider: CloudProvider,

    /// Region
    pub region: String,

    /// Tag filters for instance discovery
    pub tag_filters: HashMap<String, String>,

    /// Auto-scaling group name (AWS)
    pub asg_name: Option<String>,

    /// VM scale set name (Azure)
    pub vmss_name: Option<String>,

    /// Instance group name (GCP)
    pub instance_group: Option<String>,
}

/// Supported cloud providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudProvider {
    /// Amazon Web Services
    AWS,

    /// Microsoft Azure
    Azure,

    /// Google Cloud Platform
    GCP,
}

/// Trait for service discovery backends
#[async_trait]
pub trait ServiceDiscovery: Send + Sync {
    /// Initialize the discovery backend
    async fn initialize(&mut self) -> Result<()>;

    /// Discover available nodes
    async fn discover_nodes(&self) -> Result<Vec<Node>>;

    /// Register this node with the discovery service
    async fn register_node(&self, node: &Node) -> Result<()>;

    /// Deregister this node from the discovery service
    async fn deregister_node(&self, node_id: &str) -> Result<()>;

    /// Update node information
    async fn update_node(&self, node: &Node) -> Result<()>;

    /// Check health of a specific node
    async fn health_check(&self, node_id: &str) -> Result<HealthStatus>;

    /// Shutdown the discovery backend
    async fn shutdown(&mut self) -> Result<()>;

    /// Get the backend name
    fn name(&self) -> &str;
}

/// Shared state for discovery backends
pub type SharedDiscoveryState = Arc<RwLock<DiscoveryState>>;

/// Internal state for discovery system
#[derive(Debug)]
pub struct DiscoveryState {
    /// Currently known nodes
    pub nodes: HashMap<String, Node>,

    /// Event listeners
    pub event_listeners: Vec<tokio::sync::mpsc::UnboundedSender<DiscoveryEvent>>,
}

impl DiscoveryState {
    /// Creates a new discovery state
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            event_listeners: Vec::new(),
        }
    }

    /// Adds a node to the state and notifies listeners
    pub fn add_node(&mut self, node: Node) {
        let event = if self.nodes.contains_key(&node.id) {
            DiscoveryEvent::NodeUpdated(node.clone())
        } else {
            DiscoveryEvent::NodeAdded(node.clone())
        };

        self.nodes.insert(node.id.clone(), node);
        self.notify_listeners(event);
    }

    /// Removes a node from the state and notifies listeners
    pub fn remove_node(&mut self, node_id: &str) {
        if self.nodes.remove(node_id).is_some() {
            self.notify_listeners(DiscoveryEvent::NodeRemoved(node_id.to_string()));
        }
    }

    /// Notifies all event listeners
    fn notify_listeners(&self, event: DiscoveryEvent) {
        for listener in &self.event_listeners {
            let _ = listener.send(event.clone());
        }
    }

    /// Adds an event listener
    pub fn add_listener(&mut self, listener: tokio::sync::mpsc::UnboundedSender<DiscoveryEvent>) {
        self.event_listeners.push(listener);
    }
}

impl Default for DiscoveryState {
    fn default() -> Self {
        Self::new()
    }
}
