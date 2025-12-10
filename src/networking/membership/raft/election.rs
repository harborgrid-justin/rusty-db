// Raft Leader Election
//
// This module implements the leader election mechanism in Raft:
// - Randomized election timeouts to prevent split votes
// - Pre-vote optimization to reduce disruptions
// - Vote request/response handling
// - Leadership transfer support

use crate::error::{DbError, Result};
use crate::common::NodeId;
use crate::networking::membership::RaftConfig;
use super::{RaftStateData, RaftRole, RaftLog, Term};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use rand::Rng;

/// Vote request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
    /// Candidate's term
    pub term: Term,

    /// Candidate requesting vote
    pub candidate_id: NodeId,

    /// Index of candidate's last log entry
    pub last_log_index: u64,

    /// Term of candidate's last log entry
    pub last_log_term: Term,

    /// Pre-vote flag (true for pre-vote phase)
    pub pre_vote: bool,
}

/// Vote response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResponse {
    /// Current term, for candidate to update itself
    pub term: Term,

    /// True if candidate received vote
    pub vote_granted: bool,

    /// Reason if vote was not granted
    pub reason: Option<String>,
}

/// Election manager
pub struct ElectionManager {
    /// Local node ID
    node_id: NodeId,

    /// Configuration
    config: RaftConfig,

    /// Shared Raft state
    state: Arc<RwLock<RaftStateData>>,

    /// Shared Raft log
    log: Arc<RwLock<RaftLog>>,

    /// Election timeout duration (randomized)
    election_timeout: Arc<RwLock<Duration>>,

    /// Last time we received heartbeat or voted
    last_activity: Arc<RwLock<SystemTime>>,
}

impl ElectionManager {
    /// Create a new election manager
    pub fn new(
        node_id: NodeId,
        config: RaftConfig,
        state: Arc<RwLock<RaftStateData>>,
        log: Arc<RwLock<RaftLog>>,
    ) -> Self {
        let timeout = Self::random_election_timeout(&config);

        Self {
            node_id,
            config,
            state,
            log,
            election_timeout: Arc::new(RwLock::new(timeout)),
            last_activity: Arc::new(RwLock::new(SystemTime::now())),
        }
    }

    /// Generate random election timeout
    fn random_election_timeout(config: &RaftConfig) -> Duration {
        let min = config.election_timeout_min.as_millis() as u64;
        let max = config.election_timeout_max.as_millis() as u64;
        let timeout_ms = rand::thread_rng().gen_range(min..=max);
        Duration::from_millis(timeout_ms)
    }

    /// Reset election timeout with new random value
    async fn reset_election_timeout(&self) {
        let timeout = Self::random_election_timeout(&self.config);
        let mut election_timeout = self.election_timeout.write().await;
        *election_timeout = timeout;

        let mut last_activity = self.last_activity.write().await;
        *last_activity = SystemTime::now();
    }

    /// Check if election timeout has elapsed
    pub async fn check_election_timeout(&self) -> Result<()> {
        let last_activity = *self.last_activity.read().await;
        let election_timeout = *self.election_timeout.read().await;

        let elapsed = SystemTime::now()
            .duration_since(last_activity)
            .unwrap_or(Duration::from_secs(0));

        if elapsed >= election_timeout {
            let state = self.state.read().await;
            let role = state.role;
            drop(state);

            // Only followers and candidates can timeout
            if role == RaftRole::Follower || role == RaftRole::Candidate {
                self.start_election().await?;
            }
        }

        Ok(())
    }

    /// Start a new election
    pub async fn start_election(&self) -> Result<()> {
        tracing::info!(node_id = %self.node_id, "Starting election");

        // Transition to candidate state
        let mut state = self.state.write().await;
        state.current_term += 1;
        state.role = RaftRole::Candidate;
        state.voted_for = Some(self.node_id.clone());
        state.leader_id = None;

        let current_term = state.current_term;
        let members = state.members.clone();
        drop(state);

        // Reset election timeout
        self.reset_election_timeout().await;

        // Vote for self
        let mut votes_received = 1;
        let votes_needed = (members.len() / 2) + 1;

        tracing::info!(
            node_id = %self.node_id,
            term = current_term,
            votes_needed = votes_needed,
            "Candidate voting for self"
        );

        // In a real implementation, we would send vote requests to other nodes
        // For now, we'll just check if we have enough votes
        if votes_received >= votes_needed {
            self.become_leader().await?;
        }

        Ok(())
    }

    /// Transition to leader state
    async fn become_leader(&self) -> Result<()> {
        tracing::info!(node_id = %self.node_id, "Becoming leader");

        let mut state = self.state.write().await;
        state.role = RaftRole::Leader;
        state.leader_id = Some(self.node_id.clone());

        // Initialize leader state
        let log = self.log.read().await;
        let next_index = log.last_index() + 1;
        drop(log);

        for member_id in &state.members {
            if member_id != &self.node_id {
                state.next_index.insert(member_id.clone(), next_index);
                state.match_index.insert(member_id.clone(), 0);
            }
        }

        Ok(())
    }

    /// Handle vote request from another candidate
    pub async fn handle_vote_request(&self, request: VoteRequest) -> Result<VoteResponse> {
        let mut state = self.state.write().await;

        // If request term is greater, update our term
        if request.term > state.current_term {
            state.current_term = request.term;
            state.role = RaftRole::Follower;
            state.voted_for = None;
            state.leader_id = None;
        }

        // Reject if request term is less than current term
        if request.term < state.current_term {
            return Ok(VoteResponse {
                term: state.current_term,
                vote_granted: false,
                reason: Some("Stale term".to_string()),
            });
        }

        // Check if we already voted in this term
        if let Some(voted_for) = &state.voted_for {
            if voted_for != &request.candidate_id {
                return Ok(VoteResponse {
                    term: state.current_term,
                    vote_granted: false,
                    reason: Some("Already voted for another candidate".to_string()),
                });
            }
        }

        // Check if candidate's log is at least as up-to-date as ours
        let log = self.log.read().await;
        let our_last_term = log.last_term();
        let our_last_index = log.last_index();
        drop(log);

        let log_ok = (request.last_log_term > our_last_term) ||
            (request.last_log_term == our_last_term && request.last_log_index >= our_last_index);

        if !log_ok {
            return Ok(VoteResponse {
                term: state.current_term,
                vote_granted: false,
                reason: Some("Candidate log not up-to-date".to_string()),
            });
        }

        // Grant vote
        if !request.pre_vote {
            state.voted_for = Some(request.candidate_id.clone());
        }
        drop(state);

        // Reset election timeout since we granted a vote
        self.reset_election_timeout().await;

        Ok(VoteResponse {
            term: request.term,
            vote_granted: true,
            reason: None,
        })
    }

    /// Step down from leader/candidate to follower
    pub async fn step_down(&self, term: Term) -> Result<()> {
        let mut state = self.state.write().await;

        if term > state.current_term {
            state.current_term = term;
            state.voted_for = None;
        }

        if state.role != RaftRole::Follower {
            tracing::info!(
                node_id = %self.node_id,
                term = term,
                "Stepping down to follower"
            );
            state.role = RaftRole::Follower;
            state.leader_id = None;
        }

        Ok(())
    }

    /// Record activity (heartbeat received or vote granted)
    pub async fn record_activity(&self) {
        let mut last_activity = self.last_activity.write().await;
        *last_activity = SystemTime::now();
    }

    /// Transfer leadership to another node (if we're the leader)
    pub async fn transfer_leadership(&self, target_node: NodeId) -> Result<()> {
        let state = self.state.read().await;

        if state.role != RaftRole::Leader {
            return Err(DbError::InvalidOperation(
                "Only leader can transfer leadership".to_string(),
            ));
        }

        if !state.members.contains(&target_node) {
            return Err(DbError::InvalidArgument(
                "Target node is not a member".to_string(),
            ));
        }

        drop(state);

        tracing::info!(
            node_id = %self.node_id,
            target = %target_node,
            "Transferring leadership"
        );

        // In a real implementation, we would:
        // 1. Stop accepting new client requests
        // 2. Replicate all logs to target node
        // 3. Send TimeoutNow message to target
        // 4. Step down once target becomes leader

        // For now, just step down
        self.step_down(0).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vote_request_grant() {
        let config = RaftConfig::default();
        let state = Arc::new(RwLock::new(RaftStateData::new("node1".to_string())));
        let log = Arc::new(RwLock::new(RaftLog::new()));

        let election = ElectionManager::new(
            "node1".to_string(),
            config,
            state.clone(),
            log.clone(),
        );

        let request = VoteRequest {
            term: 1,
            candidate_id: "node2".to_string(),
            last_log_index: 0,
            last_log_term: 0,
            pre_vote: false,
        };

        let response = election.handle_vote_request(request).await.unwrap();
        assert!(response.vote_granted);
    }

    #[tokio::test]
    async fn test_vote_request_reject_stale_term() {
        let config = RaftConfig::default();
        let state = Arc::new(RwLock::new(RaftStateData::new("node1".to_string())));
        let log = Arc::new(RwLock::new(RaftLog::new()));

        let election = ElectionManager::new(
            "node1".to_string(),
            config,
            state.clone(),
            log.clone(),
        );

        // Set current term to 5
        {
            let mut s = state.write().await;
            s.current_term = 5;
        }

        let request = VoteRequest {
            term: 3, // Stale term
            candidate_id: "node2".to_string(),
            last_log_index: 0,
            last_log_term: 0,
            pre_vote: false,
        };

        let response = election.handle_vote_request(request).await.unwrap();
        assert!(!response.vote_granted);
    }
}
