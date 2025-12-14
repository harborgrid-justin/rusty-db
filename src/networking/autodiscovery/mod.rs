// Auto-Discovery Module for RustyDB
//
// Provides zero-configuration node discovery for distributed RustyDB clusters.
// Supports multiple discovery backends including gossip protocols, mDNS, UDP broadcast,
// and Serf-compatible protocols.
//
// # Features
//
// - **Gossip Protocol**: SWIM-based epidemic-style membership and failure detection
// - **mDNS Discovery**: Multicast DNS for LAN environments
// - **UDP Broadcast**: Simple broadcast-based discovery
// - **Beacon Protocol**: Periodic heartbeat-based presence announcement
// - **Serf Protocol**: HashiCorp Serf-compatible discovery
// - **Anti-Entropy**: Merkle tree-based state reconciliation
//
// # Examples
//
// ```rust,no_run
// use rusty_db::networking::autodiscovery::{AutoDiscovery, DiscoveryConfig};
//
// async fn start_discovery() -> Result<()> {
//     let config = DiscoveryConfig::default()
//         .with_port(7946)
//         .with_backend(DiscoveryBackend::Gossip);
//
//     let mut discovery = AutoDiscovery::new(config)?;
//     discovery.start().await?;
//
//     // Receive discovery events
//     while let Some(event) = discovery.next_event().await {
//         match event {
//             DiscoveryEvent::NodeJoined(node) => {
//                 println!("Node joined: {:?}", node);
//             }
//             DiscoveryEvent::NodeLeft(node) => {
//                 println!("Node left: {:?}", node);
//             }
//             _ => {}
//         }
//     }
//
//     Ok(())
// }
// ```

use crate::common::NodeId;
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

pub mod anti_entropy;
pub mod beacon;
pub mod broadcast;
pub mod gossip;
pub mod mdns;
pub mod membership;
pub mod serf;

// Re-exports
pub use anti_entropy::{AntiEntropyEngine, CrdtCounter, MerkleTree};
pub use beacon::BeaconProtocol;
pub use broadcast::BroadcastDiscovery;
pub use gossip::{GossipDiscovery, MemberState};
pub use mdns::MdnsDiscovery;
pub use membership::{Member, MembershipDelta, MembershipList, MembershipSnapshot, VersionVector};
pub use serf::SerfProtocol;

/// Node information exchanged during discovery
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct NodeInfo {
    /// Unique node identifier
    pub id: NodeId,

    /// Node's network address
    pub addr: SocketAddr,

    /// Node metadata (e.g., datacenter, rack, capabilities)
    pub metadata: HashMap<String, String>,

    /// Node protocol version
    pub protocol_version: u32,

    /// Node health status
    pub status: NodeStatus,
}

impl NodeInfo {
    /// Create a new NodeInfo instance
    pub fn new(id: NodeId, addr: SocketAddr) -> Self {
        Self {
            id,
            addr,
            metadata: HashMap::new(),
            protocol_version: 1,
            status: NodeStatus::Alive,
        }
    }

    /// Add metadata to the node
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Node health status
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, bincode::Encode, bincode::Decode,
)]
pub enum NodeStatus {
    /// Node is alive and healthy
    Alive,

    /// Node is suspected to be down
    Suspect,

    /// Node is confirmed dead
    Dead,

    /// Node is leaving gracefully
    Leaving,
}

/// Discovery events that can be subscribed to
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// A new node joined the cluster
    NodeJoined(NodeInfo),

    /// A node left the cluster (graceful)
    NodeLeft(NodeInfo),

    /// A node failed (detected failure)
    NodeFailed(NodeInfo),

    /// A node recovered from suspected state
    NodeRecovered(NodeInfo),

    /// Node metadata updated
    NodeUpdated(NodeInfo),

    /// Membership list changed
    MembershipChanged {
        members: Vec<NodeInfo>,
        total: usize,
    },
}

/// Discovery backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryBackend {
    /// SWIM gossip protocol
    Gossip,

    /// Multicast DNS
    Mdns,

    /// UDP broadcast
    Broadcast,

    /// Beacon protocol
    Beacon,

    /// Serf-compatible protocol
    Serf,
}

/// Configuration for auto-discovery
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Discovery backend to use
    pub backend: DiscoveryBackend,

    /// Local node information
    pub local_node: NodeInfo,

    /// Bind address for discovery
    pub bind_addr: SocketAddr,

    /// Known seed nodes to bootstrap from
    pub seed_nodes: Vec<SocketAddr>,

    /// Gossip interval for epidemic protocols
    pub gossip_interval: Duration,

    /// Failure detection timeout
    pub failure_timeout: Duration,

    /// Enable anti-entropy reconciliation
    pub enable_anti_entropy: bool,

    /// Anti-entropy interval
    pub anti_entropy_interval: Duration,

    /// Maximum number of nodes to gossip with
    pub gossip_fanout: usize,

    /// Enable compression for messages
    pub enable_compression: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            backend: DiscoveryBackend::Gossip,
            local_node: NodeInfo::new(
                uuid::Uuid::new_v4().to_string(),
                "0.0.0.0:7946".parse().expect("valid address"),
            ),
            bind_addr: "0.0.0.0:7946".parse().expect("valid address"),
            seed_nodes: Vec::new(),
            gossip_interval: Duration::from_secs(1),
            failure_timeout: Duration::from_secs(10),
            enable_anti_entropy: true,
            anti_entropy_interval: Duration::from_secs(60),
            gossip_fanout: 3,
            enable_compression: false,
        }
    }
}

impl DiscoveryConfig {
    /// Set the discovery port
    pub fn with_port(mut self, port: u16) -> Self {
        self.bind_addr.set_port(port);
        self.local_node.addr.set_port(port);
        self
    }

    /// Set the discovery backend
    pub fn with_backend(mut self, backend: DiscoveryBackend) -> Self {
        self.backend = backend;
        self
    }

    /// Add a seed node
    pub fn with_seed(mut self, addr: SocketAddr) -> Self {
        self.seed_nodes.push(addr);
        self
    }
}

/// Trait for discovery backends
#[async_trait::async_trait]
pub trait DiscoveryProtocol: Send + Sync {
    /// Start the discovery protocol
    async fn start(&mut self) -> Result<()>;

    /// Stop the discovery protocol
    async fn stop(&mut self) -> Result<()>;

    /// Get the current membership list
    async fn members(&self) -> Result<Vec<NodeInfo>>;

    /// Announce this node to the cluster
    async fn announce(&self) -> Result<()>;

    /// Join the cluster via seed nodes
    async fn join(&mut self, seeds: Vec<SocketAddr>) -> Result<()>;

    /// Leave the cluster gracefully
    async fn leave(&mut self) -> Result<()>;

    /// Subscribe to discovery events
    fn subscribe(&self) -> mpsc::Receiver<DiscoveryEvent>;
}

/// Main auto-discovery coordinator
pub struct AutoDiscovery {
    config: DiscoveryConfig,
    backend: Box<dyn DiscoveryProtocol>,
    #[allow(dead_code)] // Reserved for event broadcasting
    event_tx: mpsc::Sender<DiscoveryEvent>,
    event_rx: Arc<RwLock<Option<mpsc::Receiver<DiscoveryEvent>>>>,
    #[allow(dead_code)] // Reserved for membership tracking
    membership: Arc<RwLock<MembershipList>>,
}

impl AutoDiscovery {
    /// Create a new AutoDiscovery instance
    pub fn new(config: DiscoveryConfig) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::channel(1000);
        let membership = Arc::new(RwLock::new(MembershipList::new()));

        let backend: Box<dyn DiscoveryProtocol> = match config.backend {
            DiscoveryBackend::Gossip => Box::new(GossipDiscovery::new(
                config.clone(),
                event_tx.clone(),
                membership.clone(),
            )?),
            DiscoveryBackend::Mdns => {
                Box::new(MdnsDiscovery::new(config.clone(), event_tx.clone())?)
            }
            DiscoveryBackend::Broadcast => {
                Box::new(BroadcastDiscovery::new(config.clone(), event_tx.clone())?)
            }
            DiscoveryBackend::Beacon => {
                Box::new(BeaconProtocol::new(config.clone(), event_tx.clone())?)
            }
            DiscoveryBackend::Serf => {
                Box::new(SerfProtocol::new(config.clone(), event_tx.clone())?)
            }
        };

        Ok(Self {
            config,
            backend,
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
            membership,
        })
    }

    /// Start auto-discovery
    pub async fn start(&mut self) -> Result<()> {
        self.backend.start().await?;

        // Join cluster if seed nodes provided
        if !self.config.seed_nodes.is_empty() {
            let seeds = self.config.seed_nodes.clone();
            self.backend.join(seeds).await?;
        }

        Ok(())
    }

    /// Stop auto-discovery
    pub async fn stop(&mut self) -> Result<()> {
        self.backend.leave().await?;
        self.backend.stop().await
    }

    /// Get current cluster members
    pub async fn members(&self) -> Result<Vec<NodeInfo>> {
        self.backend.members().await
    }

    /// Get the next discovery event
    pub async fn next_event(&mut self) -> Option<DiscoveryEvent> {
        let mut rx_guard = self.event_rx.write().await;
        if let Some(rx) = rx_guard.as_mut() {
            rx.recv().await
        } else {
            None
        }
    }

    /// Subscribe to discovery events (consumes the receiver)
    pub async fn subscribe(&self) -> Result<mpsc::Receiver<DiscoveryEvent>> {
        let mut rx_guard = self.event_rx.write().await;
        rx_guard
            .take()
            .ok_or_else(|| DbError::InvalidOperation("Event receiver already consumed".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_info_creation() {
        let addr: SocketAddr = "127.0.0.1:7946".parse().unwrap();
        let node = NodeInfo::new("node1".to_string(), addr);

        assert_eq!(node.id, "node1");
        assert_eq!(node.addr, addr);
        assert_eq!(node.status, NodeStatus::Alive);
    }

    #[test]
    fn test_discovery_config_default() {
        let config = DiscoveryConfig::default();

        assert_eq!(config.backend, DiscoveryBackend::Gossip);
        assert_eq!(config.gossip_fanout, 3);
        assert!(config.enable_anti_entropy);
    }

    #[test]
    fn test_discovery_config_builder() {
        let config = DiscoveryConfig::default()
            .with_port(8000)
            .with_backend(DiscoveryBackend::Mdns)
            .with_seed("192.168.1.10:7946".parse().unwrap());

        assert_eq!(config.bind_addr.port(), 8000);
        assert_eq!(config.backend, DiscoveryBackend::Mdns);
        assert_eq!(config.seed_nodes.len(), 1);
    }
}
