// Raft Consensus Protocol Implementation
//
// This module implements the Raft consensus algorithm for distributed systems.
// Raft provides strong consistency guarantees and is used for:
// - Leader election with randomized timeouts
// - Log replication across cluster nodes
// - Membership changes through joint consensus
// - Snapshot and log compaction for efficiency
//
// References:
// - Raft Paper: https://raft.github.io/raft.pdf
// - Diego Ongaro's PhD thesis on consensus

use crate::error::DbError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::time::SystemTime;

// Raft node identifier
pub type RaftNodeId = u64;

// Raft term - logical clock for leader elections
pub type Term = u64;

// Log index
pub type LogIndex = u64;

// Raft node state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaftState {
    // Follower state - receives logs from leader
    Follower,
    // Candidate state - requesting votes for leadership
    Candidate,
    // Leader state - manages log replication
    Leader,
}

// Entry in the replicated log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    // The term when entry was received by leader
    pub term: Term,
    // The index of this entry in the log
    pub index: LogIndex,
    // The command to apply to state machine
    pub command: Vec<u8>,
    // Timestamp when entry was created
    pub timestamp: SystemTime,
    // Client ID for deduplication
    pub client_id: Option<String>,
    // Request ID for deduplication
    pub request_id: Option<u64>,
}

impl LogEntry {
    pub fn new(term: Term, index: LogIndex, command: Vec<u8>) -> Self {
        Self {
            term,
            index,
            command,
            timestamp: SystemTime::now(),
            client_id: None,
            request_id: None,
        }
    }

    pub fn with_client_info(mut self, client_id: String, request_id: u64) -> Self {
        self.client_id = Some(client_id);
        self.request_id = Some(request_id);
        self
    }
}

// Vote request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
    // Candidate's term
    pub term: Term,
    // Candidate requesting vote
    pub candidate_id: RaftNodeId,
    // Index of candidate's last log entry
    pub last_log_index: LogIndex,
    // Term of candidate's last log entry
    pub last_log_term: Term,
}

// Vote response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResponse {
    // Current term for candidate to update itself
    pub term: Term,
    // True if candidate received vote
    pub vote_granted: bool,
}

// Append entries request (heartbeat and log replication)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    // Leader's term
    pub term: Term,
    // So follower can redirect clients
    pub leader_id: RaftNodeId,
    // Index of log entry immediately preceding new ones
    pub prev_log_index: LogIndex,
    // Term of prev_log_index entry
    pub prev_log_term: Term,
    // Log entries to store (empty for heartbeat)
    pub entries: Vec<LogEntry>,
    // Leader's commit index
    pub leader_commit: LogIndex,
}

// Append entries response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    // Current term for leader to update itself
    pub term: Term,
    // True if follower contained entry matching prev_log_index and prev_log_term
    pub success: bool,
    // For optimization: follower's last log index
    pub match_index: Option<LogIndex>,
    // For fast log backtracking on conflicts
    pub conflict_term: Option<Term>,
    pub conflict_index: Option<LogIndex>,
}

// Snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    // Last included index in snapshot
    pub last_included_index: LogIndex,
    // Last included term in snapshot
    pub last_included_term: Term,
    // Latest configuration in snapshot
    pub configuration: ClusterConfiguration,
    // Snapshot creation timestamp
    pub timestamp: SystemTime,
}

// Install snapshot request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotRequest {
    // Leader's term
    pub term: Term,
    // Leader ID for follower redirection
    pub leader_id: RaftNodeId,
    // Snapshot metadata
    pub metadata: SnapshotMetadata,
    // Raw snapshot data
    pub data: Vec<u8>,
    // Byte offset where chunk is positioned in snapshot file
    pub offset: u64,
    // True if this is the last chunk
    pub done: bool,
}

// Install snapshot response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotResponse {
    // Current term for leader to update itself
    pub term: Term,
    // Bytes successfully received up to offset
    pub bytes_stored: u64,
}

// Cluster configuration for membership changes
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct ClusterConfiguration {
    // Current configuration members
    pub members: Vec<RaftNodeId>,
    // Optional new configuration for joint consensus
    pub new_members: Option<Vec<RaftNodeId>>,
}

impl ClusterConfiguration {
    pub fn new(members: Vec<RaftNodeId>) -> Self {
        Self {
            members,
            new_members: None,
        }
    }

    // Check if configuration is in joint consensus mode
    pub fn is_joint_consensus(&self) -> bool {
        self.new_members.is_some()
    }

    // Get all members (old and new) during joint consensus
    pub fn all_members(&self) -> Vec<RaftNodeId> {
        let mut all = self.members.clone();
        if let Some(ref new) = self.new_members {
            for member in new {
                if !all.contains(member) {
                    all.push(*member);
                }
            }
        }
        all
    }

    // Check if we have a quorum in both old and new configurations
    pub fn has_joint_quorum(&self, votes: &HashMap<RaftNodeId, bool>) -> bool {
        if !self.is_joint_consensus() {
            return self.has_quorum(votes);
        }

        let old_quorum = self.count_quorum(&self.members, votes);
        let new_quorum = if let Some(ref new) = self.new_members {
            self.count_quorum(new, votes)
        } else {
            false
        };

        old_quorum && new_quorum
    }

    // Check if we have a simple majority quorum
    pub fn has_quorum(&self, votes: &HashMap<RaftNodeId, bool>) -> bool {
        self.count_quorum(&self.members, votes)
    }

    fn count_quorum(&self, members: &[RaftNodeId], votes: &HashMap<RaftNodeId, bool>) -> bool {
        let yes_votes = members
            .iter()
            .filter(|id| votes.get(id).copied().unwrap_or(false))
            .count();
        yes_votes > members.len() / 2
    }
}

// Persistent state (must survive crashes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentState {
    // Latest term server has seen
    pub current_term: Term,
    // Candidate ID that received vote in current term
    pub voted_for: Option<RaftNodeId>,
    // Log entries
    pub log: Vec<LogEntry>,
    // Snapshot metadata
    pub snapshot_metadata: Option<SnapshotMetadata>,
}

impl PersistentState {
    pub fn new() -> Self {
        Self {
            current_term: 0,
            voted_for: None,
            log: Vec::new(),
            snapshot_metadata: None,
        }
    }

    pub fn last_log_index(&self) -> LogIndex {
        self.log.last().map(|e| e.index).unwrap_or(
            self.snapshot_metadata
                .as_ref()
                .map(|s| s.last_included_index)
                .unwrap_or(0),
        )
    }

    pub fn last_log_term(&self) -> Term {
        self.log.last().map(|e| e.term).unwrap_or(
            self.snapshot_metadata
                .as_ref()
                .map(|s| s.last_included_term)
                .unwrap_or(0),
        )
    }

    // Get log entry at index (accounting for snapshots)
    pub fn get_entry(&self, index: LogIndex) -> Option<&LogEntry> {
        if let Some(ref meta) = self.snapshot_metadata {
            if index <= meta.last_included_index {
                return None; // Entry is in snapshot
            }
            let offset = (index - meta.last_included_index - 1) as usize;
            self.log.get(offset)
        } else {
            if index == 0 {
                return None;
            }
            self.log.get((index - 1) as usize)
        }
    }

    // Get term at specific index
    pub fn get_term(&self, index: LogIndex) -> Option<Term> {
        self.get_entry(index).map(|e| e.term)
    }
}

impl Default for PersistentState {
    fn default() -> Self {
        Self::new()
    }
}

// Volatile state on all servers
#[derive(Debug, Clone)]
pub struct VolatileState {
    // Index of highest log entry known to be committed
    pub commit_index: LogIndex,
    // Index of highest log entry applied to state machine
    pub last_applied: LogIndex,
}

impl VolatileState {
    pub fn new() -> Self {
        Self {
            commit_index: 0,
            last_applied: 0,
        }
    }
}

impl Default for VolatileState {
    fn default() -> Self {
        Self::new()
    }
}

// Volatile state on leaders
#[derive(Debug, Clone)]
pub struct LeaderState {
    // For each server, index of next log entry to send
    pub next_index: HashMap<RaftNodeId, LogIndex>,
    // For each server, index of highest log entry known to be replicated
    pub match_index: HashMap<RaftNodeId, LogIndex>,
    // Batch of entries waiting to be replicated
    pub replication_batch: VecDeque<LogEntry>,
    // Maximum batch size for replication
    pub max_batch_size: usize,
}

impl LeaderState {
    pub fn new(peers: &[RaftNodeId], last_log_index: LogIndex) -> Self {
        let mut next_index = HashMap::new();
        let mut match_index = HashMap::new();

        for peer in peers {
            next_index.insert(*peer, last_log_index + 1);
            match_index.insert(*peer, 0);
        }

        Self {
            next_index,
            match_index,
            replication_batch: VecDeque::new(),
            max_batch_size: 100,
        }
    }

    // Calculate commit index based on match indexes
    pub fn calculate_commit_index(&self, current_commit: LogIndex) -> LogIndex {
        if self.match_index.is_empty() {
            return current_commit;
        }

        let mut indices: Vec<LogIndex> = self.match_index.values().copied().collect();
        indices.sort_unstable();

        // Find median - this is the highest index replicated on a majority
        let majority_index = indices.len() / 2;
        indices
            .get(majority_index)
            .copied()
            .unwrap_or(current_commit)
            .max(current_commit)
    }
}

// Raft configuration parameters
#[derive(Debug, Clone)]
pub struct RaftConfig {
    // This node's ID
    pub node_id: RaftNodeId,
    // Peer node IDs
    pub peers: Vec<RaftNodeId>,
    // Minimum election timeout (randomized)
    pub election_timeout_min: Duration,
    // Maximum election timeout (randomized)
    pub election_timeout_max: Duration,
    // Heartbeat interval (should be << election timeout)
    pub heartbeat_interval: Duration,
    // Maximum entries per AppendEntries RPC
    pub max_entries_per_append: usize,
    // Snapshot threshold (entries before creating snapshot)
    pub snapshot_threshold: usize,
    // Enable batching for log replication
    pub enable_batching: bool,
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            node_id: 0,
            peers: Vec::new(),
            election_timeout_min: Duration::from_millis(150),
            election_timeout_max: Duration::from_millis(300),
            heartbeat_interval: Duration::from_millis(50),
            max_entries_per_append: 100,
            snapshot_threshold: 10000,
            enable_batching: true,
        }
    }
}

// Main Raft node implementation
pub struct RaftNode {
    // Configuration
    config: RaftConfig,
    // Current state (Follower/Candidate/Leader)
    state: RwLock<RaftState>,
    // Persistent state
    persistent: Arc<RwLock<PersistentState>>,
    // Volatile state
    volatile: RwLock<VolatileState>,
    // Leader-specific state
    leader_state: RwLock<Option<LeaderState>>,
    // Current cluster configuration
    configuration: RwLock<ClusterConfiguration>,
    // Current leader ID (if known)
    current_leader: RwLock<Option<RaftNodeId>>,
    // Votes received in current election
    votes_received: RwLock<HashMap<RaftNodeId, bool>>,
}

impl RaftNode {
    pub fn new(config: RaftConfig) -> Self {
        let configuration = ClusterConfiguration::new(config.peers.clone());

        Self {
            config,
            state: RwLock::new(RaftState::Follower),
            persistent: Arc::new(RwLock::new(PersistentState::new())),
            volatile: RwLock::new(VolatileState::new()),
            leader_state: RwLock::new(None),
            configuration: RwLock::new(configuration),
            current_leader: RwLock::new(None),
            votes_received: RwLock::new(HashMap::new()),
        }
    }

    // Get current state
    pub fn get_state(&self) -> RaftState {
        *self.state.read().unwrap()
    }

    // Get current term
    pub fn get_current_term(&self) -> Term {
        self.persistent.read().unwrap().current_term
    }

    // Get current leader ID
    pub fn get_leader(&self) -> Option<RaftNodeId> {
        *self.current_leader.read().unwrap()
    }

    // Convert to candidate and start election
    pub fn start_election(&self) -> Result<VoteRequest, DbError> {
        let mut state = self.state.write().unwrap();
        let mut persistent = self.persistent.write().unwrap();
        let mut votes = self.votes_received.write().unwrap();

        // Increment term
        persistent.current_term += 1;
        let current_term = persistent.current_term;

        // Vote for self
        persistent.voted_for = Some(self.config.node_id);

        // Transition to candidate
        *state = RaftState::Candidate;

        // Reset votes
        votes.clear();
        votes.insert(self.config.node_id, true);

        // Create vote request
        let request = VoteRequest {
            term: current_term,
            candidate_id: self.config.node_id,
            last_log_index: persistent.last_log_index(),
            last_log_term: persistent.last_log_term(),
        };

        Ok(request)
    }

    // Handle vote request
    pub fn handle_vote_request(&self, request: VoteRequest) -> Result<VoteResponse, DbError> {
        let mut persistent = self.persistent.write().unwrap();
        let mut state = self.state.write().unwrap();

        // Update term if we're behind
        if request.term > persistent.current_term {
            persistent.current_term = request.term;
            persistent.voted_for = None;
            *state = RaftState::Follower;
        }

        let mut vote_granted = false;

        if request.term >= persistent.current_term {
            // Check if we can vote for this candidate
            let can_vote = persistent.voted_for.is_none()
                || persistent.voted_for == Some(request.candidate_id);

            // Check if candidate's log is at least as up-to-date as ours
            let log_ok = request.last_log_term > persistent.last_log_term()
                || (request.last_log_term == persistent.last_log_term()
                    && request.last_log_index >= persistent.last_log_index());

            if can_vote && log_ok {
                persistent.voted_for = Some(request.candidate_id);
                vote_granted = true;
            }
        }

        Ok(VoteResponse {
            term: persistent.current_term,
            vote_granted,
        })
    }

    // Handle vote response
    pub fn handle_vote_response(
        &self,
        from: RaftNodeId,
        response: VoteResponse,
    ) -> Result<bool, DbError> {
        let mut state = self.state.write().unwrap();
        let mut persistent = self.persistent.write().unwrap();

        // Update term if we're behind
        if response.term > persistent.current_term {
            persistent.current_term = response.term;
            persistent.voted_for = None;
            *state = RaftState::Follower;
            return Ok(false);
        }

        // Only process if we're still a candidate
        if *state != RaftState::Candidate || response.term != persistent.current_term {
            return Ok(false);
        }

        // Record vote
        let mut votes = self.votes_received.write().unwrap();
        votes.insert(from, response.vote_granted);

        // Check if we have majority
        let config = self.configuration.read().unwrap();
        let won_election = if config.is_joint_consensus() {
            config.has_joint_quorum(&votes)
        } else {
            config.has_quorum(&votes)
        };

        if won_election {
            *state = RaftState::Leader;
            drop(votes);
            drop(config);
            self.become_leader()?;
            return Ok(true);
        }

        Ok(false)
    }

    // Transition to leader state
    fn become_leader(&self) -> Result<(), DbError> {
        let mut current_leader = self.current_leader.write().unwrap();
        *current_leader = Some(self.config.node_id);

        let persistent = self.persistent.read().unwrap();
        let last_log_index = persistent.last_log_index();

        let mut leader_state = self.leader_state.write().unwrap();
        *leader_state = Some(LeaderState::new(&self.config.peers, last_log_index));

        Ok(())
    }

    // Handle append entries request
    pub fn handle_append_entries(
        &self,
        request: AppendEntriesRequest,
    ) -> Result<AppendEntriesResponse, DbError> {
        let mut state = self.state.write().unwrap();
        let mut persistent = self.persistent.write().unwrap();
        let mut current_leader = self.current_leader.write().unwrap();

        // Update term if we're behind
        if request.term > persistent.current_term {
            persistent.current_term = request.term;
            persistent.voted_for = None;
            *state = RaftState::Follower;
        }

        // Reject if term is old
        if request.term < persistent.current_term {
            return Ok(AppendEntriesResponse {
                term: persistent.current_term,
                success: false,
                match_index: None,
                conflict_term: None,
                conflict_index: None,
            });
        }

        // Valid leader for this term
        *current_leader = Some(request.leader_id);
        *state = RaftState::Follower;

        // Check if we have the previous log entry
        if request.prev_log_index > 0 {
            if let Some(term) = persistent.get_term(request.prev_log_index) {
                if term != request.prev_log_term {
                    // Log doesn't contain an entry at prev_log_index with matching term
                    return Ok(AppendEntriesResponse {
                        term: persistent.current_term,
                        success: false,
                        match_index: None,
                        conflict_term: Some(term),
                        conflict_index: Some(request.prev_log_index),
                    });
                }
            } else {
                // Don't have entry at prev_log_index
                return Ok(AppendEntriesResponse {
                    term: persistent.current_term,
                    success: false,
                    match_index: None,
                    conflict_term: None,
                    conflict_index: Some(persistent.last_log_index()),
                });
            }
        }

        // Append new entries
        if !request.entries.is_empty() {
            // Find insertion point
            let start_index = request.prev_log_index + 1;

            // Remove conflicting entries
            let base_offset = if let Some(ref meta) = persistent.snapshot_metadata {
                meta.last_included_index
            } else {
                0
            };

            if start_index > base_offset {
                let truncate_point = (start_index - base_offset - 1) as usize;
                persistent.log.truncate(truncate_point);
            }

            // Append new entries
            for entry in request.entries {
                persistent.log.push(entry);
            }
        }

        // Update commit index
        let mut volatile = self.volatile.write().unwrap();
        if request.leader_commit > volatile.commit_index {
            volatile.commit_index = request.leader_commit.min(persistent.last_log_index());
        }

        Ok(AppendEntriesResponse {
            term: persistent.current_term,
            success: true,
            match_index: Some(persistent.last_log_index()),
            conflict_term: None,
            conflict_index: None,
        })
    }

    // Send append entries to a peer
    pub fn send_append_entries(&self, peer: RaftNodeId) -> Result<AppendEntriesRequest, DbError> {
        let persistent = self.persistent.read().unwrap();
        let leader_state_guard = self.leader_state.read().unwrap();
        let volatile = self.volatile.read().unwrap();

        let leader_state = leader_state_guard
            .as_ref()
            .ok_or_else(|| DbError::Internal("Not a leader".into()))?;

        let next_index = leader_state.next_index.get(&peer).copied().unwrap_or(1);
        let prev_log_index = next_index - 1;
        let prev_log_term = if prev_log_index > 0 {
            persistent.get_term(prev_log_index).unwrap_or(0)
        } else {
            0
        };

        // Collect entries to send
        let mut entries = Vec::new();
        let base_offset = if let Some(ref meta) = persistent.snapshot_metadata {
            meta.last_included_index
        } else {
            0
        };

        if next_index > base_offset {
            let start = (next_index - base_offset - 1) as usize;
            let end = (start + self.config.max_entries_per_append).min(persistent.log.len());

            for i in start..end {
                if let Some(entry) = persistent.log.get(i) {
                    entries.push(entry.clone());
                }
            }
        }

        Ok(AppendEntriesRequest {
            term: persistent.current_term,
            leader_id: self.config.node_id,
            prev_log_index,
            prev_log_term,
            entries,
            leader_commit: volatile.commit_index,
        })
    }

    // Handle append entries response from follower
    pub fn handle_append_entries_response(
        &self,
        peer: RaftNodeId,
        response: AppendEntriesResponse,
    ) -> Result<(), DbError> {
        let mut persistent = self.persistent.write().unwrap();
        let mut state = self.state.write().unwrap();

        // Update term if we're behind
        if response.term > persistent.current_term {
            persistent.current_term = response.term;
            persistent.voted_for = None;
            *state = RaftState::Follower;
            return Ok(());
        }

        // Only process if we're still the leader
        if *state != RaftState::Leader {
            return Ok(());
        }

        let mut leader_state_guard = self.leader_state.write().unwrap();
        let leader_state = leader_state_guard
            .as_mut()
            .ok_or_else(|| DbError::Internal("Not a leader".into()))?;

        if response.success {
            // Update next_index and match_index
            if let Some(match_index) = response.match_index {
                leader_state.next_index.insert(peer, match_index + 1);
                leader_state.match_index.insert(peer, match_index);

                // Update commit index
                let new_commit = leader_state.calculate_commit_index(0);
                let mut volatile = self.volatile.write().unwrap();
                if new_commit > volatile.commit_index {
                    // Only commit entries from current term
                    if let Some(entry) = persistent.get_entry(new_commit) {
                        if entry.term == persistent.current_term {
                            volatile.commit_index = new_commit;
                        }
                    }
                }
            }
        } else {
            // Decrement next_index for this peer
            let current_next = leader_state.next_index.get(&peer).copied().unwrap_or(1);

            // Fast backtracking using conflict information
            let new_next = if let Some(conflict_index) = response.conflict_index {
                conflict_index
            } else {
                current_next.saturating_sub(1).max(1)
            };

            leader_state.next_index.insert(peer, new_next);
        }

        Ok(())
    }

    // Append command to log (leader only)
    pub fn append_command(&self, command: Vec<u8>) -> Result<LogIndex, DbError> {
        let state = self.state.read().unwrap();
        if *state != RaftState::Leader {
            return Err(DbError::Internal("Not a leader".into()));
        }

        let mut persistent = self.persistent.write().unwrap();
        let index = persistent.last_log_index() + 1;
        let term = persistent.current_term;

        let entry = LogEntry::new(term, index, command);
        persistent.log.push(entry);

        Ok(index)
    }

    // Create snapshot of state machine
    pub fn create_snapshot(
        &self,
        last_included_index: LogIndex,
        last_included_term: Term,
        _snapshot_data: Vec<u8>,
    ) -> Result<(), DbError> {
        let mut persistent = self.persistent.write().unwrap();
        let config = self.configuration.read().unwrap();

        let metadata = SnapshotMetadata {
            last_included_index,
            last_included_term,
            configuration: config.clone(),
            timestamp: SystemTime::now(),
        };

        // Update snapshot metadata
        persistent.snapshot_metadata = Some(metadata);

        // Trim log entries included in snapshot
        let base_offset = last_included_index;
        if base_offset > 0 {
            let new_log: Vec<LogEntry> = persistent
                .log
                .iter()
                .filter(|e| e.index > last_included_index)
                .cloned()
                .collect();
            persistent.log = new_log;
        }

        Ok(())
    }

    // Install snapshot from leader
    pub fn install_snapshot(
        &self,
        request: InstallSnapshotRequest,
    ) -> Result<InstallSnapshotResponse, DbError> {
        let mut persistent = self.persistent.write().unwrap();
        let mut state = self.state.write().unwrap();

        // Update term if we're behind
        if request.term > persistent.current_term {
            persistent.current_term = request.term;
            persistent.voted_for = None;
            *state = RaftState::Follower;
        }

        if request.term < persistent.current_term {
            return Ok(InstallSnapshotResponse {
                term: persistent.current_term,
                bytes_stored: 0,
            });
        }

        if request.done {
            // Install the snapshot
            persistent.snapshot_metadata = Some(request.metadata.clone());

            // Discard entire log
            persistent.log.clear();

            // Reset state machine with snapshot data
            // (Application-specific logic would go here)

            // Update commit and last applied
            let mut volatile = self.volatile.write().unwrap();
            volatile.commit_index = request.metadata.last_included_index;
            volatile.last_applied = request.metadata.last_included_index;
        }

        Ok(InstallSnapshotResponse {
            term: persistent.current_term,
            bytes_stored: request.offset + request.data.len() as u64,
        })
    }

    // Begin membership change (joint consensus)
    pub fn begin_membership_change(&self, new_members: Vec<RaftNodeId>) -> Result<(), DbError> {
        let state = self.state.read().unwrap();
        if *state != RaftState::Leader {
            return Err(DbError::Internal("Not a leader".into()));
        }

        let mut config = self.configuration.write().unwrap();
        config.new_members = Some(new_members);

        // Log the C_old,new configuration
        let config_entry = bincode::encode_to_vec(&*config, bincode::config::standard())
            .map_err(|e| DbError::Internal(format!("Serialization error: {}", e)))?;

        drop(config);
        drop(state);
        self.append_command(config_entry)?;

        Ok(())
    }

    // Finalize membership change
    pub fn finalize_membership_change(&self) -> Result<(), DbError> {
        let state = self.state.read().unwrap();
        if *state != RaftState::Leader {
            return Err(DbError::Internal("Not a leader".into()));
        }

        let mut config = self.configuration.write().unwrap();
        if let Some(new_members) = config.new_members.take() {
            config.members = new_members;

            // Log the C_new configuration
            let config_entry = bincode::encode_to_vec(&*config, bincode::config::standard())
                .map_err(|e| DbError::Internal(format!("Serialization error: {}", e)))?;

            drop(config);
            drop(state);
            self.append_command(config_entry)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_configuration_quorum() {
        let config = ClusterConfiguration::new(vec![1, 2, 3]);
        let mut votes = HashMap::new();
        votes.insert(1, true);
        votes.insert(2, true);
        assert!(config.has_quorum(&votes));
    }

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new(1, 1, vec![1, 2, 3]);
        assert_eq!(entry.term, 1);
        assert_eq!(entry.index, 1);
        assert_eq!(entry.command, vec![1, 2, 3]);
    }

    #[test]
    fn test_persistent_state_last_log() {
        let mut state = PersistentState::new();
        state.log.push(LogEntry::new(1, 1, vec![]));
        state.log.push(LogEntry::new(2, 2, vec![]));
        assert_eq!(state.last_log_index(), 2);
        assert_eq!(state.last_log_term(), 2);
    }
}
