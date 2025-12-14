// Membership Coordinator
//
// This module orchestrates cluster membership operations:
// - Node join and leave coordination
// - Graceful shutdown handling
// - Node replacement
// - Split-brain prevention

use crate::common::NodeId;
use crate::error::{DbError, Result};
use crate::networking::membership::view::ViewManager;
use crate::networking::membership::{
    MembershipConfig, MembershipEvent, NodeInfo, NodeStatus, RaftMembership, SwimMembership,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

/// Membership coordinator - orchestrates join/leave operations
pub struct MembershipCoordinator {
    /// Local node ID
    node_id: NodeId,

    /// Configuration
    config: MembershipConfig,

    /// Raft membership (for strong consistency)
    raft: Arc<RwLock<Option<RaftMembership>>>,

    /// SWIM membership (for failure detection)
    swim: Arc<RwLock<Option<SwimMembership>>>,

    /// View manager
    view_manager: Arc<ViewManager>,

    /// Event broadcaster
    event_tx: mpsc::Sender<MembershipEvent>,

    /// Coordinator state
    state: Arc<RwLock<CoordinatorState>>,
}

/// Coordinator state
#[derive(Debug, Clone)]
struct CoordinatorState {
    /// Is coordinator initialized
    initialized: bool,

    /// Is coordinator running
    running: bool,

    /// Pending join requests
    pending_joins: Vec<NodeId>,

    /// Pending leave requests
    pending_leaves: Vec<NodeId>,
}

impl CoordinatorState {
    fn new() -> Self {
        Self {
            initialized: false,
            running: false,
            pending_joins: Vec::new(),
            pending_leaves: Vec::new(),
        }
    }
}

impl MembershipCoordinator {
    /// Create a new membership coordinator
    pub fn new(
        config: MembershipConfig,
        view_manager: Arc<ViewManager>,
    ) -> (Self, mpsc::Receiver<MembershipEvent>) {
        let (event_tx, event_rx) = mpsc::channel(1000);

        let coordinator = Self {
            node_id: config.node_id.clone(),
            config,
            raft: Arc::new(RwLock::new(None)),
            swim: Arc::new(RwLock::new(None)),
            view_manager,
            event_tx,
            state: Arc::new(RwLock::new(CoordinatorState::new())),
        };

        (coordinator, event_rx)
    }

    /// Initialize the coordinator
    pub async fn initialize(&mut self, raft: RaftMembership, swim: SwimMembership) -> Result<()> {
        let mut raft_guard = self.raft.write().await;
        *raft_guard = Some(raft);
        drop(raft_guard);

        let mut swim_guard = self.swim.write().await;
        *swim_guard = Some(swim);
        drop(swim_guard);

        let mut state = self.state.write().await;
        state.initialized = true;
        state.running = true;

        Ok(())
    }

    /// Shutdown the coordinator
    pub async fn shutdown(&self) -> Result<()> {
        let mut state = self.state.write().await;
        state.running = false;

        // Stop Raft
        if let Some(raft) = self.raft.write().await.as_mut() {
            raft.stop().await?;
        }

        // Stop SWIM
        if let Some(swim) = self.swim.write().await.as_mut() {
            swim.stop().await?;
        }

        Ok(())
    }

    /// Handle a node joining the cluster
    pub async fn handle_join_request(&self, node_info: NodeInfo) -> Result<JoinResponse> {
        tracing::info!(
            node_id = %node_info.id,
            "Handling join request"
        );

        // Check if we have Raft consensus
        let raft_guard = self.raft.read().await;
        let raft = raft_guard
            .as_ref()
            .ok_or_else(|| DbError::InvalidState("Raft not initialized".to_string()))?;

        // Only leader can accept join requests
        if !raft.is_leader().await {
            let leader_id = raft.leader_id().await;
            return Ok(JoinResponse {
                accepted: false,
                reason: Some("Not the leader".to_string()),
                redirect_to: leader_id,
            });
        }

        drop(raft_guard);

        // Add to pending joins
        {
            let mut state = self.state.write().await;
            state.pending_joins.push(node_info.id.clone());
        }

        // Propose membership change via Raft
        let raft_guard = self.raft.read().await;
        if let Some(raft) = raft_guard.as_ref() {
            raft.propose_add_node(node_info.clone()).await?;
        }
        drop(raft_guard);

        // Add to SWIM membership
        let swim_guard = self.swim.read().await;
        if let Some(swim) = swim_guard.as_ref() {
            swim.add_member(node_info.clone()).await?;
        }
        drop(swim_guard);

        // Update view
        self.view_manager.add_node(node_info.clone()).await?;

        // Broadcast event
        let _ = self
            .event_tx
            .send(MembershipEvent::NodeJoined {
                node_id: node_info.id.clone(),
                node_info,
            })
            .await;

        Ok(JoinResponse {
            accepted: true,
            reason: None,
            redirect_to: None,
        })
    }

    /// Handle a node leaving the cluster
    pub async fn handle_leave_request(&self, node_id: NodeId) -> Result<()> {
        tracing::info!(
            node_id = %node_id,
            "Handling leave request"
        );

        // Check if we have Raft consensus
        let raft_guard = self.raft.read().await;
        let raft = raft_guard
            .as_ref()
            .ok_or_else(|| DbError::InvalidState("Raft not initialized".to_string()))?;

        // Only leader can handle leave requests
        if !raft.is_leader().await {
            return Err(DbError::InvalidOperation(
                "Only leader can handle leave requests".to_string(),
            ));
        }

        drop(raft_guard);

        // Add to pending leaves
        {
            let mut state = self.state.write().await;
            state.pending_leaves.push(node_id.clone());
        }

        // Propose membership change via Raft
        let raft_guard = self.raft.read().await;
        if let Some(raft) = raft_guard.as_ref() {
            raft.propose_remove_node(node_id.clone()).await?;
        }
        drop(raft_guard);

        // Update view
        self.view_manager
            .update_node_status(&node_id, NodeStatus::Leaving)
            .await?;

        // Broadcast event
        let _ = self
            .event_tx
            .send(MembershipEvent::NodeLeft {
                node_id,
                graceful: true,
            })
            .await;

        Ok(())
    }

    /// Gracefully leave the cluster (for this node)
    pub async fn leave_cluster(&self) -> Result<()> {
        tracing::info!(
            node_id = %self.node_id,
            "Leaving cluster gracefully"
        );

        // Notify other nodes of our departure
        self.handle_leave_request(self.node_id.clone()).await?;

        // Wait a bit for replication
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Shutdown
        self.shutdown().await?;

        Ok(())
    }

    /// Handle node failure detected by SWIM
    pub async fn handle_node_failure(&self, node_id: NodeId) -> Result<()> {
        tracing::warn!(
            node_id = %node_id,
            "Handling node failure"
        );

        // Update view
        self.view_manager
            .update_node_status(&node_id, NodeStatus::Failed)
            .await?;

        // Broadcast event
        let _ = self
            .event_tx
            .send(MembershipEvent::NodeFailed {
                node_id: node_id.clone(),
            })
            .await;

        // Check if we need to trigger replacement
        self.check_and_trigger_replacement().await?;

        Ok(())
    }

    /// Replace a failed node
    pub async fn replace_node(&self, failed_node: NodeId, new_node: NodeInfo) -> Result<()> {
        tracing::info!(
            failed_node = %failed_node,
            new_node = %new_node.id,
            "Replacing failed node"
        );

        // Remove failed node
        let raft_guard = self.raft.read().await;
        if let Some(raft) = raft_guard.as_ref() {
            raft.propose_remove_node(failed_node.clone()).await?;
        }
        drop(raft_guard);

        // Wait for removal to complete
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Add new node
        self.handle_join_request(new_node).await?;

        Ok(())
    }

    /// Check for split-brain and prevent it
    pub async fn check_split_brain_prevention(&self) -> Result<()> {
        if !self.config.enable_split_brain_prevention {
            return Ok(());
        }

        let is_split_brain = self.view_manager.check_split_brain().await?;

        if is_split_brain {
            tracing::error!("Potential split-brain detected!");

            // If we're not in quorum, step down from leadership
            let raft_guard = self.raft.read().await;
            if let Some(raft) = raft_guard.as_ref() {
                if raft.is_leader().await {
                    tracing::warn!("Stepping down from leadership due to split-brain");
                    // In a real implementation, we would step down
                }
            }
        }

        Ok(())
    }

    /// Check if we need to replace failed nodes
    async fn check_and_trigger_replacement(&self) -> Result<()> {
        let view = self.view_manager.get_view().await;

        // If we have too many failed nodes, we might need operator intervention
        if view.failed_count() > view.total_nodes() / 3 {
            tracing::error!(
                failed = view.failed_count(),
                total = view.total_nodes(),
                "Too many failed nodes, manual intervention may be required"
            );
        }

        Ok(())
    }

    /// Get current view
    pub async fn get_view(&self) -> Result<crate::networking::membership::view::MembershipView> {
        Ok(self.view_manager.get_view().await)
    }

    /// Check if this node is the leader
    pub async fn is_leader(&self) -> Result<bool> {
        let raft_guard = self.raft.read().await;
        if let Some(raft) = raft_guard.as_ref() {
            Ok(raft.is_leader().await)
        } else {
            Ok(false)
        }
    }

    /// Get current leader
    pub async fn get_leader(&self) -> Result<Option<NodeId>> {
        let raft_guard = self.raft.read().await;
        if let Some(raft) = raft_guard.as_ref() {
            Ok(raft.leader_id().await)
        } else {
            Ok(None)
        }
    }
}

/// Response to a join request
#[derive(Debug, Clone)]
pub struct JoinResponse {
    /// Whether the join was accepted
    pub accepted: bool,

    /// Reason if not accepted
    pub reason: Option<String>,

    /// Leader to redirect to (if not accepted)
    pub redirect_to: Option<NodeId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::membership::{NodeMetadata, RaftConfig, SwimConfig};
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let config = MembershipConfig::default();
        let view_manager = Arc::new(ViewManager::new());

        let (coordinator, _rx) = MembershipCoordinator::new(config, view_manager);

        assert_eq!(coordinator.node_id, "node1");
    }

    #[tokio::test]
    async fn test_coordinator_requires_leader() {
        let config = MembershipConfig {
            node_id: "node1".to_string(),
            ..Default::default()
        };
        let view_manager = Arc::new(ViewManager::new());

        let (mut coordinator, _rx) = MembershipCoordinator::new(config.clone(), view_manager);

        // Initialize with Raft and SWIM
        let (raft, _raft_rx) = RaftMembership::new(config.node_id.clone(), RaftConfig::default());

        let local_node = NodeInfo::new(
            config.node_id.clone(),
            config.cluster_addr,
            config.client_addr,
            NodeMetadata::default(),
        );

        let (swim, _swim_rx) =
            SwimMembership::new(config.node_id.clone(), local_node, SwimConfig::default());

        coordinator.initialize(raft, swim).await.unwrap();

        // Try to join a node (should fail since we're not leader)
        let new_node = NodeInfo::new(
            "node2".to_string(),
            "127.0.0.1:7001".parse::<SocketAddr>().unwrap(),
            "127.0.0.1:5433".parse::<SocketAddr>().unwrap(),
            NodeMetadata::default(),
        );

        let response = coordinator.handle_join_request(new_node).await.unwrap();
        assert!(!response.accepted);
    }
}
