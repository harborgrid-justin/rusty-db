// Cluster Membership Management Module
//
// This module provides enterprise-grade cluster membership management with:
// - Raft consensus for configuration management
// - SWIM protocol for failure detection
// - Consistent membership views across the cluster
// - Bootstrap and join orchestration
//
// Architecture:
// - RaftMembership: Provides strong consistency for membership changes
// - SwimMembership: Provides efficient failure detection via gossip
// - MembershipView: Provides consistent view of cluster state
// - MembershipCoordinator: Orchestrates join/leave operations
// - Bootstrap: Handles cluster initialization

use crate::common::NodeId;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};

pub mod bootstrap;
pub mod coordinator;
pub mod raft;
pub mod swim;
pub mod view;

// Re-exports for convenience
pub use coordinator::MembershipCoordinator;
pub use raft::RaftMembership;
pub use swim::SwimMembership;
pub use view::MembershipView;

/// Node information in the cluster
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct NodeInfo {
    /// Unique node identifier
    pub id: NodeId,

    /// Network address for cluster communication
    pub cluster_addr: SocketAddr,

    /// Network address for client connections
    pub client_addr: SocketAddr,

    /// Current node status
    pub status: NodeStatus,

    /// Node metadata (version, capabilities, etc.)
    pub metadata: NodeMetadata,

    /// When this node joined the cluster
    pub joined_at: SystemTime,

    /// Last heartbeat timestamp
    pub last_heartbeat: SystemTime,
}

impl NodeInfo {
    /// Create a new node info
    pub fn new(
        id: NodeId,
        cluster_addr: SocketAddr,
        client_addr: SocketAddr,
        metadata: NodeMetadata,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            cluster_addr,
            client_addr,
            status: NodeStatus::Joining,
            metadata,
            joined_at: now,
            last_heartbeat: now,
        }
    }

    /// Check if node is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.status, NodeStatus::Active)
    }

    /// Check if node is suspected of failure
    pub fn is_suspected(&self) -> bool {
        matches!(self.status, NodeStatus::Suspected)
    }

    /// Check if node has failed
    pub fn is_failed(&self) -> bool {
        matches!(self.status, NodeStatus::Failed)
    }

    /// Update heartbeat timestamp
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = SystemTime::now();
        if self.status == NodeStatus::Suspected {
            self.status = NodeStatus::Active;
        }
    }
}

/// Node status in the cluster
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Hash,
    bincode::Encode,
    bincode::Decode,
)]
pub enum NodeStatus {
    /// Node is joining the cluster
    Joining,

    /// Node is active and healthy
    Active,

    /// Node is suspected of failure
    Suspected,

    /// Node has failed
    Failed,

    /// Node is leaving the cluster
    Leaving,

    /// Node has left the cluster
    Left,
}

/// Node metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct NodeMetadata {
    /// Database version
    pub version: String,

    /// Node role (primary, replica, etc.)
    pub role: NodeRole,

    /// Node capabilities
    pub capabilities: HashSet<String>,

    /// Data center or availability zone
    pub datacenter: Option<String>,

    /// Rack identifier for topology awareness
    pub rack: Option<String>,

    /// Custom tags
    pub tags: HashMap<String, String>,
}

impl Default for NodeMetadata {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            role: NodeRole::Replica,
            capabilities: HashSet::new(),
            datacenter: None,
            rack: None,
            tags: HashMap::new(),
        }
    }
}

/// Node role in the cluster
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Hash,
    bincode::Encode,
    bincode::Decode,
)]
pub enum NodeRole {
    /// Primary/master node
    Primary,

    /// Replica node
    Replica,

    /// Read-only replica
    ReadReplica,

    /// Witness node (for quorum only)
    Witness,
}

/// Membership configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipConfig {
    /// Local node ID
    pub node_id: NodeId,

    /// Local cluster address
    pub cluster_addr: SocketAddr,

    /// Local client address
    pub client_addr: SocketAddr,

    /// Node metadata
    pub metadata: NodeMetadata,

    /// Seed nodes for bootstrapping
    pub seed_nodes: Vec<SocketAddr>,

    /// Raft configuration
    pub raft_config: RaftConfig,

    /// SWIM configuration
    pub swim_config: SwimConfig,

    /// Failure detection timeout
    pub failure_timeout: Duration,

    /// Heartbeat interval
    pub heartbeat_interval: Duration,

    /// Enable split-brain prevention
    pub enable_split_brain_prevention: bool,
}

impl Default for MembershipConfig {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            cluster_addr: "127.0.0.1:7000".parse().unwrap(),
            client_addr: "127.0.0.1:5432".parse().unwrap(),
            metadata: NodeMetadata::default(),
            seed_nodes: Vec::new(),
            raft_config: RaftConfig::default(),
            swim_config: SwimConfig::default(),
            failure_timeout: Duration::from_secs(30),
            heartbeat_interval: Duration::from_secs(5),
            enable_split_brain_prevention: true,
        }
    }
}

/// Raft consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    /// Election timeout range (randomized within this range)
    pub election_timeout_min: Duration,
    pub election_timeout_max: Duration,

    /// Heartbeat interval (should be << election timeout)
    pub heartbeat_interval: Duration,

    /// Maximum number of log entries per AppendEntries RPC
    pub max_entries_per_append: usize,

    /// Enable pre-vote to reduce disruptions
    pub enable_pre_vote: bool,

    /// Log compaction threshold
    pub snapshot_threshold: u64,

    /// Maximum log size before compaction
    pub max_log_size: u64,
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            election_timeout_min: Duration::from_millis(150),
            election_timeout_max: Duration::from_millis(300),
            heartbeat_interval: Duration::from_millis(50),
            max_entries_per_append: 100,
            enable_pre_vote: true,
            snapshot_threshold: 10000,
            max_log_size: 100 * 1024 * 1024, // 100MB
        }
    }
}

/// SWIM protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwimConfig {
    /// Protocol period (time between membership updates)
    pub protocol_period: Duration,

    /// Suspicion multiplier for failure detection
    pub suspicion_multiplier: u32,

    /// Number of nodes to probe indirectly
    pub indirect_probe_size: usize,

    /// Number of gossip targets per period
    pub gossip_fanout: usize,

    /// Gossip retransmit multiplier
    pub gossip_retransmit_mult: u32,
}

impl Default for SwimConfig {
    fn default() -> Self {
        Self {
            protocol_period: Duration::from_millis(1000),
            suspicion_multiplier: 4,
            indirect_probe_size: 3,
            gossip_fanout: 3,
            gossip_retransmit_mult: 4,
        }
    }
}

/// Cluster membership events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MembershipEvent {
    /// A node joined the cluster
    NodeJoined {
        node_id: NodeId,
        node_info: NodeInfo,
    },

    /// A node left the cluster
    NodeLeft { node_id: NodeId, graceful: bool },

    /// A node failed
    NodeFailed { node_id: NodeId },

    /// A node became suspected
    NodeSuspected { node_id: NodeId },

    /// A node recovered from suspicion
    NodeRecovered { node_id: NodeId },

    /// A new leader was elected
    LeaderElected { leader_id: NodeId, term: u64 },

    /// Membership view changed
    ViewChanged { version: u64, member_count: usize },
}

/// Trait for cluster membership management
#[async_trait::async_trait]
pub trait ClusterMembership: Send + Sync {
    /// Initialize the membership manager
    async fn initialize(&mut self) -> Result<()>;

    /// Shutdown the membership manager
    async fn shutdown(&mut self) -> Result<()>;

    /// Get the current membership view
    async fn get_view(&self) -> Result<MembershipView>;

    /// Get information about a specific node
    async fn get_node(&self, node_id: &NodeId) -> Result<Option<NodeInfo>>;

    /// Get all active nodes
    async fn get_active_nodes(&self) -> Result<Vec<NodeInfo>>;

    /// Check if a node is alive
    async fn is_node_alive(&self, node_id: &NodeId) -> Result<bool>;

    /// Get the current leader (if any)
    async fn get_leader(&self) -> Result<Option<NodeId>>;

    /// Check if this node is the leader
    async fn is_leader(&self) -> Result<bool>;

    /// Subscribe to membership events
    async fn subscribe(&self) -> Result<tokio::sync::mpsc::Receiver<MembershipEvent>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_info_creation() {
        let metadata = NodeMetadata::default();
        let node = NodeInfo::new(
            "node1".to_string(),
            "127.0.0.1:7000".parse().unwrap(),
            "127.0.0.1:5432".parse().unwrap(),
            metadata,
        );

        assert_eq!(node.id, "node1");
        assert_eq!(node.status, NodeStatus::Joining);
    }

    #[test]
    fn test_node_status_checks() {
        let mut node = NodeInfo::new(
            "node1".to_string(),
            "127.0.0.1:7000".parse().unwrap(),
            "127.0.0.1:5432".parse().unwrap(),
            NodeMetadata::default(),
        );

        node.status = NodeStatus::Active;
        assert!(node.is_healthy());
        assert!(!node.is_suspected());
        assert!(!node.is_failed());

        node.status = NodeStatus::Suspected;
        assert!(!node.is_healthy());
        assert!(node.is_suspected());

        node.update_heartbeat();
        assert!(node.is_healthy());
    }
}
