/// Node Management Module
///
/// This module provides types and functionality for managing cluster nodes,
/// including node identification, roles, status, and basic operations.

use std::fmt;
use std::time::SystemTime;
use crate::error::DbError;
use serde::{Deserialize, Serialize};
use std::time::{Duration};

/// Node identifier - a unique string identifier for cluster nodes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    /// Create a new NodeId
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Node role in the cluster
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeRole {
    /// Leader node - handles write operations
    Leader,
    /// Follower node - replicates from leader
    Follower,
    /// Candidate node - attempting to become leader
    Candidate,
    /// Observer node - read-only, doesn't participate in consensus
    Observer,
}

/// Node status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Node is healthy and operational
    Healthy,
    /// Node is experiencing issues but still operational
    Degraded,
    /// Node is not responding
    Unreachable,
    /// Node is shutting down
    ShuttingDown,
    /// Node has failed
    Failed,
}

/// Node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: NodeId,
    pub address: String,
    pub port: u16,
    pub role: NodeRole,
    pub status: NodeStatus,
    pub last_heartbeat: SystemTime,
    pub data_version: u64,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub active_connections: usize,
}

impl NodeInfo {
    /// Create a new NodeInfo with default values
    ///
    /// # Arguments
    /// * `id` - Unique node identifier
    /// * `address` - Network address of the node
    /// * `port` - Network port of the node
    ///
    /// # Returns
    /// A new NodeInfo instance with default role Follower and status Healthy
    pub fn new(id: NodeId, address: String, port: u16) -> Self {
        Self {
            id,
            address,
            port,
            role: NodeRole::Follower,
            status: NodeStatus::Healthy,
            last_heartbeat: SystemTime::now(),
            data_version: 0,
            cpu_usage: 0.0,
            memory_usage: 0.0,
            disk_usage: 0.0,
            active_connections: 0,
        }
    }

    /// Check if the node is alive based on heartbeat timeout
    ///
    /// # Arguments
    /// * `timeout` - Maximum allowed time since last heartbeat
    ///
    /// # Returns
    /// true if the node is considered alive, false otherwise
    pub fn is_alive(&self, timeout: Duration) -> bool {
        match self.last_heartbeat.elapsed() {
            Ok(elapsed) => elapsed < timeout,
            Err(_) => false,
        }
    }

    /// Update the node's heartbeat timestamp
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = SystemTime::now();
    }

    /// Update resource usage metrics
    ///
    /// # Arguments
    /// * `cpu` - CPU usage percentage (0.0 - 100.0)
    /// * `memory` - Memory usage percentage (0.0 - 100.0)
    /// * `disk` - Disk usage percentage (0.0 - 100.0)
    pub fn update_resources(&mut self, cpu: f32, memory: f32, disk: f32) {
        self.cpu_usage = cpu;
        self.memory_usage = memory;
        self.disk_usage = disk;
    }
}

/// Trait for node lifecycle management
pub trait NodeLifecycle {
    /// Initialize the node
    fn initialize(&mut self) -> Result<(), DbError>;

    /// Shutdown the node gracefully
    fn shutdown(&mut self) -> Result<(), DbError>;

    /// Check node health
    fn health_check(&self) -> NodeStatus;
}

impl NodeLifecycle for NodeInfo {
    fn initialize(&mut self) -> Result<(), DbError> {
        self.status = NodeStatus::Healthy;
        self.update_heartbeat();
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), DbError> {
        self.status = NodeStatus::ShuttingDown;
        Ok(())
    }

    fn health_check(&self) -> NodeStatus {
        // Simple health check based on resource usage
        if self.cpu_usage > 95.0 || self.memory_usage > 95.0 || self.disk_usage > 95.0 {
            NodeStatus::Degraded
        } else {
            NodeStatus::Healthy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_node_id() {
        let id = NodeId::new("node1".to_string());
        assert_eq!(id.as_str(), "node1");
        assert_eq!(format!("{}", id), "node1");
    }

    #[test]
    fn test_node_info_creation() {
        let node = NodeInfo::new(
            NodeId::new("test-node".to_string()),
            "127.0.0.1".to_string(),
            8080,
        );
        assert_eq!(node.id.as_str(), "test-node");
        assert_eq!(node.address, "127.0.0.1");
        assert_eq!(node.port, 8080);
        assert_eq!(node.role, NodeRole::Follower);
        assert_eq!(node.status, NodeStatus::Healthy);
    }

    #[test]
    fn test_node_alive() {
        let mut node = NodeInfo::new(
            NodeId::new("test".to_string()),
            "127.0.0.1".to_string(),
            8080,
        );
        assert!(node.is_alive(Duration::from_secs(60)));

        // Simulate old heartbeat
        node.last_heartbeat = SystemTime::now() - Duration::from_secs(120);
        assert!(!node.is_alive(Duration::from_secs(60)));
    }

    #[test]
    fn test_node_lifecycle() {
        let mut node = NodeInfo::new(
            NodeId::new("test".to_string()),
            "127.0.0.1".to_string(),
            8080,
        );

        assert!(node.initialize().is_ok());
        assert_eq!(node.status, NodeStatus::Healthy);

        assert!(node.shutdown().is_ok());
        assert_eq!(node.status, NodeStatus::ShuttingDown);
    }
}
