// Cluster Topology Module
//
// SWIM protocol implementation, cluster membership, and partition detection

use std::collections::HashMap;
use std::time::Duration;

use super::{NodeId, NodeInfo, NodeState};

// ============================================================================
// SWIM Protocol
// ============================================================================

#[derive(Debug, Clone)]
pub struct SwimConfig {
    pub protocol_period: Duration,
    pub suspect_timeout: Duration,
    pub indirect_probes: usize,
}

impl Default for SwimConfig {
    fn default() -> Self {
        Self {
            protocol_period: Duration::from_millis(1000),
            suspect_timeout: Duration::from_millis(5000),
            indirect_probes: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SwimMessage {
    Ping {
        from: NodeId,
        sequence: u64,
    },
    Ack {
        from: NodeId,
        sequence: u64,
    },
    PingReq {
        from: NodeId,
        target: NodeId,
        sequence: u64,
    },
}

#[derive(Debug, Clone)]
pub struct NodeUpdate {
    pub node_id: NodeId,
    pub state: NodeState,
    pub incarnation: u64,
}

// ============================================================================
// Cluster Topology Manager
// ============================================================================

pub struct ClusterTopologyManager {
    nodes: HashMap<NodeId, NodeInfo>,
    local_node_id: NodeId,
}

impl ClusterTopologyManager {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            local_node_id: 0,
        }
    }

    pub fn with_local_node(local_node_id: NodeId) -> Self {
        Self {
            nodes: HashMap::new(),
            local_node_id,
        }
    }

    pub fn add_node(&mut self, node: NodeInfo) {
        self.nodes.insert(node.id, node);
    }

    pub fn remove_node(&mut self, node_id: NodeId) -> Option<NodeInfo> {
        self.nodes.remove(&node_id)
    }

    pub fn get_node(&self, node_id: NodeId) -> Option<&NodeInfo> {
        self.nodes.get(&node_id)
    }

    pub fn get_node_mut(&mut self, node_id: NodeId) -> Option<&mut NodeInfo> {
        self.nodes.get_mut(&node_id)
    }

    pub fn update_node_state(&mut self, node_id: NodeId, state: NodeState) -> bool {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.state = state;
            true
        } else {
            false
        }
    }

    pub fn list_nodes(&self) -> Vec<&NodeInfo> {
        self.nodes.values().collect()
    }

    pub fn alive_nodes(&self) -> Vec<&NodeInfo> {
        self.nodes
            .values()
            .filter(|n| n.state == NodeState::Alive)
            .collect()
    }

    pub fn dead_nodes(&self) -> Vec<&NodeInfo> {
        self.nodes
            .values()
            .filter(|n| n.state == NodeState::Dead)
            .collect()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn metrics(&self) -> TopologyMetrics {
        let total_nodes = self.nodes.len();
        let alive_nodes = self.nodes.values().filter(|n| n.state == NodeState::Alive).count();
        let dead_nodes = self.nodes.values().filter(|n| n.state == NodeState::Dead).count();

        TopologyMetrics {
            total_nodes,
            alive_nodes,
            dead_nodes,
        }
    }
}

impl Default for ClusterTopologyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct TopologyMetrics {
    pub total_nodes: usize,
    pub alive_nodes: usize,
    pub dead_nodes: usize,
}

// ============================================================================
// Partition Detection
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionStatus {
    Healthy,
    Degraded,
    Partitioned,
}

#[derive(Debug, Clone)]
pub struct QuorumConfig {
    pub min_nodes: usize,
    pub replication_factor: usize,
}

impl Default for QuorumConfig {
    fn default() -> Self {
        Self {
            min_nodes: 3,
            replication_factor: 3,
        }
    }
}

pub struct PartitionDetector {
    config: QuorumConfig,
}

impl PartitionDetector {
    pub fn new(config: QuorumConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &QuorumConfig {
        &self.config
    }

    pub fn detect_partition(&self, alive_nodes: usize, total_nodes: usize) -> PartitionStatus {
        if alive_nodes >= self.config.min_nodes {
            if alive_nodes == total_nodes {
                PartitionStatus::Healthy
            } else {
                PartitionStatus::Degraded
            }
        } else {
            PartitionStatus::Partitioned
        }
    }

    pub fn has_quorum(&self, alive_nodes: usize) -> bool {
        alive_nodes >= self.config.min_nodes
    }
}

// ============================================================================
// Membership Events
// ============================================================================

#[derive(Debug, Clone)]
pub enum MembershipEvent {
    NodeJoined(NodeId),
    NodeLeft(NodeId),
    NodeFailed(NodeId),
    NodeUpdated(NodeId),
    TopologyChanged,
}
