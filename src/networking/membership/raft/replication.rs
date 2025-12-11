// Raft Log Replication
//
// This module implements log replication in Raft:
// - AppendEntries RPC for log replication and heartbeats
// - Commit index tracking and advancement
// - Catch-up mechanism for slow followers
// - Snapshot installation for far-behind followers

use crate::error::{DbError, Result};
use crate::common::NodeId;
use super::{RaftStateData, RaftRole, RaftLog, Term, LogEntry, LogIndex};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

/// AppendEntries request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    /// Leader's term
    pub term: Term,

    /// Leader's node ID
    pub leader_id: NodeId,

    /// Index of log entry immediately preceding new ones
    pub prev_log_index: LogIndex,

    /// Term of prev_log_index entry
    pub prev_log_term: Term,

    /// Log entries to store (empty for heartbeat)
    pub entries: Vec<LogEntry>,

    /// Leader's commit index
    pub leader_commit: LogIndex,
}

/// AppendEntries response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    /// Current term, for leader to update itself
    pub term: Term,

    /// True if follower contained entry matching prev_log_index and prev_log_term
    pub success: bool,

    /// Follower's last log index (for updating next_index)
    pub match_index: LogIndex,

    /// Conflict information for fast rollback
    pub conflict_index: Option<LogIndex>,
    pub conflict_term: Option<Term>,
}

/// InstallSnapshot request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotRequest {
    /// Leader's term
    pub term: Term,

    /// Leader's node ID
    pub leader_id: NodeId,

    /// Index of last log entry in snapshot
    pub last_included_index: LogIndex,

    /// Term of last log entry in snapshot
    pub last_included_term: Term,

    /// Snapshot data
    pub data: Vec<u8>,
}

/// InstallSnapshot response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotResponse {
    /// Current term
    pub term: Term,

    /// Success flag
    pub success: bool,
}

/// Replication manager
pub struct ReplicationManager {
    /// Local node ID
    node_id: NodeId,

    /// Shared Raft state
    state: Arc<RwLock<RaftStateData>>,

    /// Shared Raft log
    log: Arc<RwLock<RaftLog>>,
}

impl ReplicationManager {
    /// Create a new replication manager
    pub fn new(
        node_id: NodeId,
        state: Arc<RwLock<RaftStateData>>,
        log: Arc<RwLock<RaftLog>>,
    ) -> Self {
        Self {
            node_id,
            state,
            log,
        }
    }

    /// Send heartbeats to all followers (as leader)
    pub async fn send_heartbeats(&self) -> Result<()> {
        let state = self.state.read().await;

        if state.role != RaftRole::Leader {
            return Ok(());
        }

        let term = state.current_term;
        let leader_id = self.node_id.clone();
        let commit_index = state.commit_index;
        let members = state.members.clone();
        drop(state);

        let log = self.log.read().await;
        let last_index = log.last_index();
        drop(log);

        // In a real implementation, we would send heartbeats to all followers
        // For now, just log the heartbeat
        tracing::trace!(
            node_id = %self.node_id,
            term = term,
            commit_index = commit_index,
            last_index = last_index,
            "Sending heartbeats"
        );

        Ok(())
    }

    /// Replicate logs to followers (as leader)
    pub async fn replicate_logs(&self) -> Result<()> {
        let mut state = self.state.write().await;

        if state.role != RaftRole::Leader {
            return Err(DbError::InvalidOperation(
                "Only leader can replicate logs".to_string(),
            ));
        }

        let term = state.current_term;
        let commit_index = state.commit_index;
        let members = state.members.clone();

        // For each follower, send AppendEntries with new log entries
        for member_id in &members {
            if member_id == &self.node_id {
                continue;
            }

            let next_index = state.next_index.get(member_id).copied().unwrap_or(1);

            // In a real implementation, we would:
            // 1. Get entries from next_index to last_index
            // 2. Send AppendEntriesRequest to follower
            // 3. Handle response and update next_index/match_index

            tracing::debug!(
                leader = %self.node_id,
                follower = %member_id,
                next_index = next_index,
                "Replicating logs"
            );
        }

        Ok(())
    }

    /// Handle AppendEntries request from leader (as follower)
    pub async fn handle_append_entries(&self, request: AppendEntriesRequest) -> Result<AppendEntriesResponse> {
        let mut state = self.state.write().await;

        // Update term if request has higher term
        if request.term > state.current_term {
            state.current_term = request.term;
            state.role = RaftRole::Follower;
            state.voted_for = None;
            state.leader_id = Some(request.leader_id.clone());
        }

        // Reject if request term is stale
        if request.term < state.current_term {
            return Ok(AppendEntriesResponse {
                term: state.current_term,
                success: false,
                match_index: 0,
                conflict_index: None,
                conflict_term: None,
            });
        }

        // Accept the leader
        state.leader_id = Some(request.leader_id.clone());
        state.last_heartbeat = SystemTime::now();

        let mut log = self.log.write().await;

        // Check if our log contains an entry at prev_log_index with prev_log_term
        if request.prev_log_index > 0 {
            match log.term_at(request.prev_log_index) {
                None => {
                    // We don't have an entry at prev_log_index
                    return Ok(AppendEntriesResponse {
                        term: state.current_term,
                        success: false,
                        match_index: log.last_index(),
                        conflict_index: Some(log.last_index() + 1),
                        conflict_term: None,
                    });
                }
                Some(term) if term != request.prev_log_term => {
                    // Entry exists but term doesn't match
                    // Find first index of conflicting term
                    let mut conflict_index = request.prev_log_index;
                    while conflict_index > 0 {
                        if let Some(t) = log.term_at(conflict_index - 1) {
                            if t != term {
                                break;
                            }
                            conflict_index -= 1;
                        } else {
                            break;
                        }
                    }

                    return Ok(AppendEntriesResponse {
                        term: state.current_term,
                        success: false,
                        match_index: log.last_index(),
                        conflict_index: Some(conflict_index),
                        conflict_term: Some(term),
                    });
                }
                Some(_) => {
                    // Entry matches, continue
                }
            }
        }

        // Append new entries
        if !request.entries.is_empty() {
            // Delete conflicting entries
            let first_new_index = request.entries[0].index;
            if first_new_index <= log.last_index() {
                log.truncate_from(first_new_index)?;
            }

            // Append new entries
            for entry in &request.entries {
                log.append(entry.clone())?;
            }
        }

        let match_index = log.last_index();

        // Update commit index
        if request.leader_commit > state.commit_index {
            state.commit_index = std::cmp::min(request.leader_commit, log.last_index());
        }

        Ok(AppendEntriesResponse {
            term: state.current_term,
            success: true,
            match_index,
            conflict_index: None,
            conflict_term: None,
        })
    }

    /// Handle AppendEntries response from follower (as leader)
    pub async fn handle_append_entries_response(
        &self,
        follower_id: NodeId,
        response: AppendEntriesResponse,
    ) -> Result<()> {
        let mut state = self.state.write().await;

        // Update term if response has higher term
        if response.term > state.current_term {
            state.current_term = response.term;
            state.role = RaftRole::Follower;
            state.voted_for = None;
            state.leader_id = None;
            return Ok(());
        }

        // Only process if we're still the leader
        if state.role != RaftRole::Leader {
            return Ok(());
        }

        if response.success {
            // Update next_index and match_index for this follower
            state.next_index.insert(follower_id.clone(), response.match_index + 1);
            state.match_index.insert(follower_id.clone(), response.match_index);

            // Try to advance commit index
            self.advance_commit_index(&mut state).await?;
        } else {
            // Log didn't match, decrement next_index
            if let Some(conflict_index) = response.conflict_index {
                state.next_index.insert(follower_id.clone(), conflict_index);
            } else {
                let current_next = state.next_index.get(&follower_id).copied().unwrap_or(1);
                state.next_index.insert(follower_id, current_next.saturating_sub(1).max(1));
            }
        }

        Ok(())
    }

    /// Advance commit index based on majority replication
    async fn advance_commit_index(&self, state: &mut RaftStateData) -> Result<()> {
        let log = self.log.read().await;
        let last_index = log.last_index();

        // Find the highest index that is replicated on a majority
        for n in (state.commit_index + 1)..=last_index {
            // Count how many nodes have this entry
            let mut count = 1; // Count ourselves

            for match_index in state.match_index.values() {
                if *match_index >= n {
                    count += 1;
                }
            }

            // Check if we have a majority
            let majority = (state.members.len() / 2) + 1;
            if count >= majority {
                // Only commit entries from current term
                if let Some(term) = log.term_at(n) {
                    if term == state.current_term {
                        state.commit_index = n;
                        tracing::debug!(
                            node_id = %self.node_id,
                            commit_index = n,
                            "Advanced commit index"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Install snapshot on follower
    pub async fn handle_install_snapshot(&self, request: InstallSnapshotRequest) -> Result<InstallSnapshotResponse> {
        let mut state = self.state.write().await;

        // Update term if request has higher term
        if request.term > state.current_term {
            state.current_term = request.term;
            state.role = RaftRole::Follower;
            state.voted_for = None;
        }

        // Reject if request term is stale
        if request.term < state.current_term {
            return Ok(InstallSnapshotResponse {
                term: state.current_term,
                success: false,
            });
        }

        state.leader_id = Some(request.leader_id);

        // Install snapshot
        let snapshot = super::log::Snapshot {
            last_included_index: request.last_included_index,
            last_included_term: request.last_included_term,
            data: request.data,
            created_at: SystemTime::now(),
        };

        let mut log = self.log.write().await;
        log.install_snapshot(snapshot)?;

        // Update commit index and last applied
        state.commit_index = request.last_included_index;
        state.last_applied = request.last_included_index;

        Ok(InstallSnapshotResponse {
            term: state.current_term,
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_append_entries_success() {
        let state = Arc::new(RwLock::new(RaftStateData::new("node1".to_string())));
        let log = Arc::new(RwLock::new(RaftLog::new()));

        let replication = ReplicationManager::new(
            "node1".to_string(),
            state.clone(),
            log.clone(),
        );

        let request = AppendEntriesRequest {
            term: 1,
            leader_id: "leader".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![
                LogEntry::new(1, 1, vec![1, 2, 3]),
            ],
            leader_commit: 0,
        };

        let response = replication.handle_append_entries(request).await.unwrap();
        assert!(response.success);
        assert_eq!(response.match_index, 1);
    }

    #[tokio::test]
    async fn test_append_entries_reject_stale_term() {
        let state = Arc::new(RwLock::new(RaftStateData::new("node1".to_string())));
        let log = Arc::new(RwLock::new(RaftLog::new()));

        // Set current term to 5
        {
            let mut s = state.write().await;
            s.current_term = 5;
        }

        let replication = ReplicationManager::new(
            "node1".to_string(),
            state.clone(),
            log.clone(),
        );

        let request = AppendEntriesRequest {
            term: 3, // Stale term
            leader_id: "leader".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        };

        let response = replication.handle_append_entries(request).await.unwrap();
        assert!(!response.success);
        assert_eq!(response.term, 5);
    }
}
