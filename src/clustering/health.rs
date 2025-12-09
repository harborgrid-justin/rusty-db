use crate::clustering::node::{NodeId, NodeInfo};
/// Cluster Health and Status Management Module
///
/// This module provides types and functionality for monitoring and managing
/// the health and status of cluster nodes and the overall cluster.

use crate::error::DbError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Overall cluster status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClusterStatus {
    /// Cluster is fully operational
    Healthy,
    /// Cluster is operational but degraded
    Degraded,
    /// Cluster has lost quorum
    Failed,
}

/// Cluster health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterHealth {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub degraded_nodes: usize,
    pub failed_nodes: usize,
    pub has_leader: bool,
    pub has_quorum: bool,
    pub cluster_status: ClusterStatus,
}

impl ClusterHealth {
    /// Create a new cluster health report
    pub fn new() -> Self {
        Self {
            total_nodes: 0,
            healthy_nodes: 0,
            degraded_nodes: 0,
            failed_nodes: 0,
            has_leader: false,
            has_quorum: false,
            cluster_status: ClusterStatus::Failed,
        }
    }

    /// Update health based on node information
    pub fn update_from_nodes(&mut self, nodes: &HashMap<NodeId, NodeInfo>, has_leader: bool) {
        self.total_nodes = nodes.len();
        self.healthy_nodes = 0;
        self.degraded_nodes = 0;
        self.failed_nodes = 0;
        self.has_leader = has_leader;

        for node_info in nodes.values() {
            match node_info.status {
                crate::clustering::node::NodeStatus::Healthy => self.healthy_nodes += 1,
                crate::clustering::node::NodeStatus::Degraded => self.degraded_nodes += 1,
                crate::clustering::node::NodeStatus::Unreachable |
                crate::clustering::node::NodeStatus::ShuttingDown |
                crate::clustering::node::NodeStatus::Failed => self.failed_nodes += 1,
            }
        }

        // Determine quorum (simple majority)
        let quorum_size = (self.total_nodes / 2) + 1;
        self.has_quorum = self.healthy_nodes + self.degraded_nodes >= quorum_size;

        // Determine overall status
        self.cluster_status = if !self.has_quorum {
            ClusterStatus::Failed
        } else if self.failed_nodes > 0 {
            ClusterStatus::Degraded
        } else {
            ClusterStatus::Healthy
        };
    }

    /// Check if cluster is operational
    pub fn is_operational(&self) -> bool {
        matches!(self.cluster_status, ClusterStatus::Healthy | ClusterStatus::Degraded)
    }
}

/// Health issue type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthIssueType {
    HighCpuUsage,
    HighMemoryUsage,
    HighDiskUsage,
    NetworkPartition,
    NodeUnreachable,
    DataInconsistency,
}

/// Issue severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Health issue details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthIssue {
    pub node_id: NodeId,
    pub issue_type: HealthIssueType,
    pub severity: IssueSeverity,
    pub message: String,
    pub timestamp: SystemTime,
}

/// Trait for health monitoring
pub trait HealthMonitor {
    /// Perform health check
    fn check_health(&self) -> Result<ClusterHealth, DbError>;

    /// Get detailed health metrics
    fn get_health_metrics(&self) -> Result<HealthMetrics, DbError>;
}

/// Health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub cluster_status: ClusterStatus,
    pub node_metrics: HashMap<NodeId, NodeHealthMetrics>,
    pub overall_cpu_usage: f32,
    pub overall_memory_usage: f32,
    pub overall_disk_usage: f32,
    pub active_connections: usize,
    pub queries_per_second: f64,
    pub response_time_ms: f64,
}

/// Node-specific health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealthMetrics {
    pub node_id: NodeId,
    pub status: crate::clustering::node::NodeStatus,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub active_connections: usize,
    pub uptime_seconds: u64,
    pub last_heartbeat: SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clustering::node::{NodeInfo, NodeStatus};
    use std::collections::HashMap;

    #[test]
    fn test_cluster_health_new() {
        let health = ClusterHealth::new();
        assert_eq!(health.cluster_status, ClusterStatus::Failed);
        assert_eq!(health.total_nodes, 0);
        assert!(!health.has_leader);
        assert!(!health.has_quorum);
    }

    #[test]
    fn test_cluster_health_update() {
        let mut health = ClusterHealth::new();
        let mut nodes = HashMap::new();

        let node1 = NodeInfo::new(
            NodeId::new("node1".to_string()),
            "127.0.0.1".to_string(),
            5432,
        );
        let node2 = NodeInfo {
            status: NodeStatus::Degraded,
            ..NodeInfo::new(
                NodeId::new("node2".to_string()),
                "127.0.0.2".to_string(),
                5432,
            )
        };

        nodes.insert(node1.id.clone(), node1);
        nodes.insert(node2.id.clone(), node2);

        health.update_from_nodes(&nodes, true);

        assert_eq!(health.total_nodes, 2);
        assert_eq!(health.healthy_nodes, 1);
        assert_eq!(health.degraded_nodes, 1);
        assert_eq!(health.failed_nodes, 0);
        assert!(health.has_leader);
        assert!(health.has_quorum);
        assert_eq!(health.cluster_status, ClusterStatus::Degraded);
    }

    #[test]
    fn test_cluster_health_operational() {
        let mut health = ClusterHealth::new();
        health.cluster_status = ClusterStatus::Healthy;
        assert!(health.is_operational());

        health.cluster_status = ClusterStatus::Degraded;
        assert!(health.is_operational());

        health.cluster_status = ClusterStatus::Failed;
        assert!(!health.is_operational());
    }
}
