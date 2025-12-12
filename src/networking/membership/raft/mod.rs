// Raft Consensus for Cluster Membership
//
// This module implements the Raft consensus algorithm specifically for managing
// cluster membership configuration. It provides:
// - Strong consistency for membership changes
// - Leader election with randomized timeouts
// - Log replication for configuration changes
// - Joint consensus for safe membership updates
//
// Reference: https://raft.github.io/raft.pdf

use crate::error::{DbError, Result};
use crate::common::NodeId;
use crate::networking::membership::{NodeInfo, RaftConfig, MembershipEvent};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, mpsc};
use tokio::time;

pub mod log;
pub mod election;
pub mod replication;

pub use log::{RaftLog, LogEntry, LogIndex};
pub use election::{ElectionManager, VoteRequest, VoteResponse};
pub use replication::{ReplicationManager, AppendEntriesRequest, AppendEntriesResponse};

/// Raft term number (logical clock)
pub type Term = u64;

/// Raft consensus state machine for membership
pub struct RaftMembership {
    /// Local node ID
    node_id: NodeId,

    /// Current Raft state
    state: Arc<RwLock<RaftStateData>>,

    /// Raft log
    log: Arc<RwLock<RaftLog>>,

    /// Election manager
    election: Arc<ElectionManager>,

    /// Replication manager
    replication: Arc<ReplicationManager>,

    /// Configuration
    config: RaftConfig,

    /// Event broadcaster
    event_tx: mpsc::Sender<MembershipEvent>,

    /// Shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,
}

/// Raft state data
#[derive(Debug, Clone)]
pub struct RaftStateData {
    /// Current state (Follower, Candidate, Leader)
    pub role: RaftRole,

    /// Current term
    pub current_term: Term,

    /// Who we voted for in current term
    pub voted_for: Option<NodeId>,

    /// Current leader (if known)
    pub leader_id: Option<NodeId>,

    /// Index of highest log entry known to be committed
    pub commit_index: LogIndex,

    /// Index of highest log entry applied to state machine
    pub last_applied: LogIndex,

    /// For leaders: next log index to send to each peer
    pub next_index: HashMap<NodeId, LogIndex>,

    /// For leaders: highest log index known to be replicated on each peer
    pub match_index: HashMap<NodeId, LogIndex>,

    /// Current cluster members
    pub members: HashSet<NodeId>,

    /// Last heartbeat time
    pub last_heartbeat: SystemTime,
}

impl RaftStateData {
    fn new(node_id: NodeId) -> Self {
        Self {
            role: RaftRole::Follower,
            current_term: 0,
            voted_for: None,
            leader_id: None,
            commit_index: 0,
            last_applied: 0,
            next_index: HashMap::new(),
            match_index: HashMap::new(),
            members: HashSet::from([node_id]),
            last_heartbeat: SystemTime::now(),
        }
    }
}

/// Raft role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaftRole {
    /// Follower - passively receives log entries
    Follower,

    /// Candidate - requesting votes for leadership
    Candidate,

    /// Leader - manages log replication
    Leader,
}

impl RaftMembership {
    /// Create a new Raft membership manager
    pub fn new(node_id: NodeId, config: RaftConfig) -> (Self, mpsc::Receiver<MembershipEvent>) {
        let (event_tx, event_rx) = mpsc::channel(1000);
        let state = Arc::new(RwLock::new(RaftStateData::new(node_id.clone())));
        let log = Arc::new(RwLock::new(RaftLog::new()));

        let election = Arc::new(ElectionManager::new(
            node_id.clone(),
            config.clone(),
            state.clone(),
            log.clone(),
        ));

        let replication = Arc::new(ReplicationManager::new(
            node_id.clone(),
            state.clone(),
            log.clone(),
        ));

        let membership = Self {
            node_id,
            state,
            log,
            election,
            replication,
            config,
            event_tx,
            shutdown_tx: None,
        };

        (membership, event_rx)
    }

    /// Start the Raft consensus engine
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Start election timeout task
        let election = self.election.clone();
        let _state = self.state.clone();
        let _event_tx = self.event_tx.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(100));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = election.check_election_timeout().await {
                            tracing::error!("Election timeout check failed: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });

        // Start heartbeat task (if leader)
        let replication = self.replication.clone();
        let state = self.state.clone();
        let heartbeat_interval = self.config.heartbeat_interval;
        tokio::spawn(async move {
            let mut interval = time::interval(heartbeat_interval);
            loop {
                interval.tick().await;
                let is_leader = {
                    let state = state.read().await;
                    state.role == RaftRole::Leader
                };

                if is_leader {
                    if let Err(e) = replication.send_heartbeats().await {
                        tracing::error!("Failed to send heartbeats: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the Raft consensus engine
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(()).await;
        }
        Ok(())
    }

    /// Get current term
    pub async fn current_term(&self) -> Term {
        let state = self.state.read().await;
        state.current_term
    }

    /// Get current role
    pub async fn current_role(&self) -> RaftRole {
        let state = self.state.read().await;
        state.role
    }

    /// Check if this node is the leader
    pub async fn is_leader(&self) -> bool {
        let state = self.state.read().await;
        state.role == RaftRole::Leader
    }

    /// Get current leader ID
    pub async fn leader_id(&self) -> Option<NodeId> {
        let state = self.state.read().await;
        state.leader_id.clone()
    }

    /// Propose a membership change
    pub async fn propose_add_node(&self, node_info: NodeInfo) -> Result<()> {
        if !self.is_leader().await {
            return Err(DbError::InvalidOperation(
                "Only leader can propose membership changes".to_string(),
            ));
        }

        // Serialize the add node command
        let command = MembershipCommand::AddNode {
            node_id: node_info.id.clone(),
            node_info,
        };
        let command_bytes = bincode::encode_to_vec(&command, bincode::config::standard())
            .map_err(|e| DbError::Serialization(e.to_string()))?;

        // Append to log
        let mut log = self.log.write().await;
        let state = self.state.write().await;

        let entry = LogEntry::new(
            state.current_term,
            log.last_index() + 1,
            command_bytes,
        );

        log.append(entry)?;

        // Trigger replication
        drop(log);
        drop(state);
        self.replication.replicate_logs().await?;

        Ok(())
    }

    /// Propose removing a node
    pub async fn propose_remove_node(&self, node_id: NodeId) -> Result<()> {
        if !self.is_leader().await {
            return Err(DbError::InvalidOperation(
                "Only leader can propose membership changes".to_string(),
            ));
        }

        let command = MembershipCommand::RemoveNode { node_id };
        let command_bytes = bincode::encode_to_vec(&command, bincode::config::standard())
            .map_err(|e| DbError::Serialization(e.to_string()))?;

        let mut log = self.log.write().await;
        let state = self.state.write().await;

        let entry = LogEntry::new(
            state.current_term,
            log.last_index() + 1,
            command_bytes,
        );

        log.append(entry)?;

        drop(log);
        drop(state);
        self.replication.replicate_logs().await?;

        Ok(())
    }

    /// Apply committed log entries to state machine
    pub async fn apply_committed_entries(&self) -> Result<()> {
        let mut state = self.state.write().await;
        let log = self.log.read().await;

        while state.last_applied < state.commit_index {
            state.last_applied += 1;
            if let Some(entry) = log.get(state.last_applied) {
                // Deserialize and apply command
                if let Ok((command, _)) = bincode::decode_from_slice(&entry.data, bincode::config::standard()) {
                    match command {
                        MembershipCommand::AddNode { node_id, node_info } => {
                            state.members.insert(node_id.clone());
                            let _ = self.event_tx.send(MembershipEvent::NodeJoined {
                                node_id,
                                node_info,
                            }).await;
                        }
                        MembershipCommand::RemoveNode { node_id } => {
                            state.members.remove(&node_id);
                            let _ = self.event_tx.send(MembershipEvent::NodeLeft {
                                node_id,
                                graceful: true,
                            }).await;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get current members
    pub async fn members(&self) -> HashSet<NodeId> {
        let state = self.state.read().await;
        state.members.clone()
    }

    /// Handle vote request from candidate
    pub async fn handle_vote_request(&self, request: VoteRequest) -> Result<VoteResponse> {
        self.election.handle_vote_request(request).await
    }

    /// Handle append entries request from leader
    pub async fn handle_append_entries(&self, request: AppendEntriesRequest) -> Result<AppendEntriesResponse> {
        self.replication.handle_append_entries(request).await
    }
}

/// Membership commands in the Raft log
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum MembershipCommand {
    /// Add a node to the cluster
    AddNode {
        node_id: NodeId,
        node_info: NodeInfo,
    },

    /// Remove a node from the cluster
    RemoveNode {
        node_id: NodeId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::membership::NodeMetadata;
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_raft_creation() {
        let config = RaftConfig::default();
        let (raft, _rx) = RaftMembership::new("node1".to_string(), config);

        assert_eq!(raft.current_term().await, 0);
        assert_eq!(raft.current_role().await, RaftRole::Follower);
        assert!(!raft.is_leader().await);
    }

    #[tokio::test]
    async fn test_propose_requires_leader() {
        let config = RaftConfig::default();
        let (raft, _rx) = RaftMembership::new("node1".to_string(), config);

        let node_info = NodeInfo::new(
            "node2".to_string(),
            "127.0.0.1:7001".parse::<SocketAddr>().unwrap(),
            "127.0.0.1:5433".parse::<SocketAddr>().unwrap(),
            NodeMetadata::default(),
        );

        let result = raft.propose_add_node(node_info).await;
        assert!(result.is_err());
    }
}
