// Membership View Management
//
// This module provides a consistent view of the cluster membership.
// Features:
// - Versioned membership views
// - View change notifications
// - Quorum calculations
// - Split-brain detection

use crate::error::{DbError, Result};
use crate::common::NodeId;
use crate::networking::membership::{NodeInfo, NodeStatus, MembershipEvent};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{RwLock, mpsc};

/// Membership view - a consistent snapshot of cluster state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipView {
    /// View version (monotonically increasing)
    pub version: u64,

    /// All known nodes
    pub nodes: HashMap<NodeId, NodeInfo>,

    /// Active nodes (healthy and operational)
    pub active_nodes: HashSet<NodeId>,

    /// Suspected nodes (possibly failed)
    pub suspected_nodes: HashSet<NodeId>,

    /// Failed nodes
    pub failed_nodes: HashSet<NodeId>,

    /// Current leader (if any)
    pub leader: Option<NodeId>,

    /// Raft term (for Raft-based views)
    pub term: u64,

    /// When this view was created
    pub created_at: SystemTime,

    /// Cluster health status
    pub health: ClusterHealth,
}

impl MembershipView {
    /// Create a new empty view
    pub fn new() -> Self {
        Self {
            version: 0,
            nodes: HashMap::new(),
            active_nodes: HashSet::new(),
            suspected_nodes: HashSet::new(),
            failed_nodes: HashSet::new(),
            leader: None,
            term: 0,
            created_at: SystemTime::now(),
            health: ClusterHealth::Unknown,
        }
    }

    /// Get total number of nodes
    pub fn total_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Get number of active nodes
    pub fn active_count(&self) -> usize {
        self.active_nodes.len()
    }

    /// Get number of suspected nodes
    pub fn suspected_count(&self) -> usize {
        self.suspected_nodes.len()
    }

    /// Get number of failed nodes
    pub fn failed_count(&self) -> usize {
        self.failed_nodes.len()
    }

    /// Check if a node is active
    pub fn is_active(&self, node_id: &NodeId) -> bool {
        self.active_nodes.contains(node_id)
    }

    /// Check if a node is suspected
    pub fn is_suspected(&self, node_id: &NodeId) -> bool {
        self.suspected_nodes.contains(node_id)
    }

    /// Check if a node is failed
    pub fn is_failed(&self, node_id: &NodeId) -> bool {
        self.failed_nodes.contains(node_id)
    }

    /// Get node information
    pub fn get_node(&self, node_id: &NodeId) -> Option<&NodeInfo> {
        self.nodes.get(node_id)
    }

    /// Calculate quorum size (majority)
    pub fn quorum_size(&self) -> usize {
        (self.total_nodes() / 2) + 1
    }

    /// Check if we have quorum of active nodes
    pub fn has_quorum(&self) -> bool {
        self.active_count() >= self.quorum_size()
    }

    /// Get all active node IDs
    pub fn get_active_nodes(&self) -> Vec<NodeId> {
        self.active_nodes.iter().cloned().collect()
    }

    /// Update cluster health status
    pub fn update_health(&mut self) {
        let active_ratio = self.active_count() as f64 / self.total_nodes() as f64;

        self.health = if active_ratio >= 0.9 {
            ClusterHealth::Healthy
        } else if active_ratio >= 0.5 {
            ClusterHealth::Degraded
        } else {
            ClusterHealth::Unhealthy
        };
    }
}

impl Default for MembershipView {
    fn default() -> Self {
        Self::new()
    }
}

/// Cluster health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClusterHealth {
    /// All nodes are healthy
    Healthy,

    /// Some nodes are unhealthy but cluster is operational
    Degraded,

    /// Cluster is not operational
    Unhealthy,

    /// Health status is unknown
    Unknown,
}

/// View manager - maintains and updates the membership view
pub struct ViewManager {
    /// Current view
    view: Arc<RwLock<MembershipView>>,

    /// View change listeners
    listeners: Arc<RwLock<Vec<mpsc::Sender<MembershipView>>>>,

    /// View history (for debugging and analysis)
    history: Arc<RwLock<ViewHistory>>,
}

impl ViewManager {
    /// Create a new view manager
    pub fn new() -> Self {
        Self {
            view: Arc::new(RwLock::new(MembershipView::new())),
            listeners: Arc::new(RwLock::new(Vec::new())),
            history: Arc::new(RwLock::new(ViewHistory::new(100))),
        }
    }

    /// Get current view
    pub async fn get_view(&self) -> MembershipView {
        self.view.read().await.clone()
    }

    /// Update the view with a new node
    pub async fn add_node(&self, node: NodeInfo) -> Result<()> {
        let mut view = self.view.write().await;
        let node_id = node.id.clone();

        view.nodes.insert(node_id.clone(), node.clone());

        match node.status {
            NodeStatus::Active => {
                view.active_nodes.insert(node_id.clone());
            }
            NodeStatus::Suspected => {
                view.suspected_nodes.insert(node_id.clone());
            }
            NodeStatus::Failed | NodeStatus::Left => {
                view.failed_nodes.insert(node_id.clone());
            }
            _ => {}
        }

        view.version += 1;
        view.created_at = SystemTime::now();
        view.update_health();

        let new_view = view.clone();
        drop(view);

        // Record in history
        self.history.write().await.record(new_view.clone());

        // Notify listeners
        self.notify_listeners(new_view).await;

        Ok(())
    }

    /// Remove a node from the view
    pub async fn remove_node(&self, node_id: &NodeId) -> Result<()> {
        let mut view = self.view.write().await;

        view.nodes.remove(node_id);
        view.active_nodes.remove(node_id);
        view.suspected_nodes.remove(node_id);
        view.failed_nodes.remove(node_id);

        view.version += 1;
        view.created_at = SystemTime::now();
        view.update_health();

        let new_view = view.clone();
        drop(view);

        // Record in history
        self.history.write().await.record(new_view.clone());

        // Notify listeners
        self.notify_listeners(new_view).await;

        Ok(())
    }

    /// Update node status
    pub async fn update_node_status(&self, node_id: &NodeId, new_status: NodeStatus) -> Result<()> {
        let mut view = self.view.write().await;

        if let Some(node) = view.nodes.get_mut(node_id) {
            let old_status = node.status;
            node.status = new_status;

            // Update status sets
            view.active_nodes.remove(node_id);
            view.suspected_nodes.remove(node_id);
            view.failed_nodes.remove(node_id);

            match new_status {
                NodeStatus::Active => {
                    view.active_nodes.insert(node_id.clone());
                }
                NodeStatus::Suspected => {
                    view.suspected_nodes.insert(node_id.clone());
                }
                NodeStatus::Failed | NodeStatus::Left => {
                    view.failed_nodes.insert(node_id.clone());
                }
                _ => {}
            }

            view.version += 1;
            view.created_at = SystemTime::now();
            view.update_health();

            let new_view = view.clone();
            drop(view);

            // Record in history
            self.history.write().await.record(new_view.clone());

            // Notify listeners
            self.notify_listeners(new_view).await;
        }

        Ok(())
    }

    /// Set the cluster leader
    pub async fn set_leader(&self, leader_id: Option<NodeId>, term: u64) -> Result<()> {
        let mut view = self.view.write().await;
        view.leader = leader_id;
        view.term = term;
        view.version += 1;
        view.created_at = SystemTime::now();

        let new_view = view.clone();
        drop(view);

        // Record in history
        self.history.write().await.record(new_view.clone());

        // Notify listeners
        self.notify_listeners(new_view).await;

        Ok(())
    }

    /// Subscribe to view changes
    pub async fn subscribe(&self) -> mpsc::Receiver<MembershipView> {
        let (tx, rx) = mpsc::channel(100);
        let mut listeners = self.listeners.write().await;
        listeners.push(tx);
        rx
    }

    /// Notify all listeners of view change
    async fn notify_listeners(&self, view: MembershipView) {
        let mut listeners = self.listeners.write().await;

        // Remove closed channels
        listeners.retain(|tx| !tx.is_closed());

        // Send to all active listeners
        for tx in listeners.iter() {
            let _ = tx.send(view.clone()).await;
        }
    }

    /// Get view history
    pub async fn get_history(&self) -> Vec<MembershipView> {
        self.history.read().await.get_all()
    }

    /// Check for potential split-brain scenario
    pub async fn check_split_brain(&self) -> Result<bool> {
        let view = self.view.read().await;

        // Simple check: if we have less than quorum, we might be in minority partition
        if !view.has_quorum() {
            tracing::warn!(
                active_nodes = view.active_count(),
                quorum = view.quorum_size(),
                "Potential split-brain: insufficient quorum"
            );
            return Ok(true);
        }

        Ok(false)
    }
}

impl Default for ViewManager {
    fn default() -> Self {
        Self::new()
    }
}

/// View history for debugging and analysis
struct ViewHistory {
    views: Vec<MembershipView>,
    max_size: usize,
}

impl ViewHistory {
    fn new(max_size: usize) -> Self {
        Self {
            views: Vec::new(),
            max_size,
        }
    }

    fn record(&mut self, view: MembershipView) {
        self.views.push(view);

        // Keep only recent history
        if self.views.len() > self.max_size {
            self.views.remove(0);
        }
    }

    fn get_all(&self) -> Vec<MembershipView> {
        self.views.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::membership::NodeMetadata;
    use std::net::SocketAddr;

    #[test]
    fn test_empty_view() {
        let view = MembershipView::new();
        assert_eq!(view.version, 0);
        assert_eq!(view.total_nodes(), 0);
        assert_eq!(view.active_count(), 0);
    }

    #[test]
    fn test_quorum_calculation() {
        let mut view = MembershipView::new();

        // 3 nodes: quorum = 2
        for i in 1..=3 {
            let node = NodeInfo::new(
                format!("node{}", i),
                format!("127.0.0.1:{}", 7000 + i).parse::<SocketAddr>().unwrap(),
                format!("127.0.0.1:{}", 5432 + i).parse::<SocketAddr>().unwrap(),
                NodeMetadata::default(),
            );
            view.nodes.insert(node.id.clone(), node.clone());
            view.active_nodes.insert(node.id);
        }

        assert_eq!(view.quorum_size(), 2);
        assert!(view.has_quorum());

        // Remove one active node
        view.active_nodes.remove("node3");
        assert!(view.has_quorum()); // Still have 2 active

        // Remove another
        view.active_nodes.remove("node2");
        assert!(!view.has_quorum()); // Only 1 active
    }

    #[tokio::test]
    async fn test_view_manager() {
        let manager = ViewManager::new();

        let node1 = NodeInfo::new(
            "node1".to_string(),
            "127.0.0.1:7000".parse::<SocketAddr>().unwrap(),
            "127.0.0.1:5432".parse::<SocketAddr>().unwrap(),
            NodeMetadata::default(),
        );

        manager.add_node(node1).await.unwrap();

        let view = manager.get_view().await;
        assert_eq!(view.version, 1);
        assert_eq!(view.total_nodes(), 1);
    }

    #[tokio::test]
    async fn test_view_subscriptions() {
        let manager = ViewManager::new();
        let mut rx = manager.subscribe().await;

        let node1 = NodeInfo::new(
            "node1".to_string(),
            "127.0.0.1:7000".parse::<SocketAddr>().unwrap(),
            "127.0.0.1:5432".parse::<SocketAddr>().unwrap(),
            NodeMetadata::default(),
        );

        manager.add_node(node1).await.unwrap();

        // Should receive view update
        let view = rx.recv().await.unwrap();
        assert_eq!(view.version, 1);
    }
}
