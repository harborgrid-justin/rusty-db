// Two-Phase Commit (2PC) protocol implementation.
//
// This module implements the two-phase commit protocol for
// distributed transactions across multiple participants.
//
// # Protocol Phases
//
// 1. **Prepare Phase**: Coordinator asks all participants to prepare.
// 2. **Commit Phase**: If all prepared, coordinator sends commit; else abort.
//
// # Example
//
// ```rust,ignore
// let coordinator = TwoPhaseCommitCoordinator::new(Duration::from_secs(30));
// coordinator.register_participant(txn_id, participant);
// if coordinator.prepare_phase(txn_id)? {
//     coordinator.commit_phase(txn_id)?;
// } else {
//     coordinator.abort_phase(txn_id)?;
// }
// ```

use std::time::SystemTime;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::common::TransactionId;

use super::error::{TransactionError, TransactionResult};

/// State of a participant in 2PC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantState {
    /// Initial state.
    Idle,
    /// Preparing to commit.
    Preparing,
    /// Ready to commit (voted yes).
    Prepared,
    /// Transaction committed.
    Committed,
    /// Transaction aborted.
    Aborted,
    /// Communication failure.
    Failed,
}

impl Default for ParticipantState {
    fn default() -> Self {
        ParticipantState::Idle
    }
}

/// Information about a 2PC participant.
#[derive(Debug, Clone)]
pub struct ParticipantInfo {
    /// Unique identifier for the participant.
    pub id: String,
    /// Current state of the participant.
    pub state: ParticipantState,
    /// Last successful communication time.
    pub last_contact: SystemTime,
    /// Endpoint for communication.
    pub endpoint: Option<String>,
}

impl ParticipantInfo {
    /// Creates a new participant info.
    pub fn new(id: String) -> Self {
        Self {
            id,
            state: ParticipantState::Idle,
            last_contact: SystemTime::now(),
            endpoint: None,
        }
    }

    /// Creates a participant with an endpoint.
    pub fn with_endpoint(id: String, endpoint: String) -> Self {
        Self {
            id,
            state: ParticipantState::Idle,
            last_contact: SystemTime::now(),
            endpoint: Some(endpoint),
        }
    }

    /// Updates the last contact time.
    pub fn touch(&mut self) {
        self.last_contact = SystemTime::now();
    }
}

/// Two-Phase Commit Coordinator.
///
/// Manages distributed transactions across multiple participants
/// using the 2PC protocol.
pub struct TwoPhaseCommitCoordinator {
    /// Participants per transaction.
    participants: Arc<RwLock<HashMap<TransactionId, Vec<ParticipantInfo>>>>,
    /// Timeout for prepare phase.
    prepare_timeout: Duration,
    /// Statistics.
    stats: Arc<RwLock<TwoPhaseCommitStats>>,
}

/// Statistics for 2PC operations.
#[derive(Debug, Default, Clone)]
pub struct TwoPhaseCommitStats {
    /// Total transactions coordinated.
    pub total_transactions: u64,
    /// Successfully committed.
    pub committed: u64,
    /// Aborted transactions.
    pub aborted: u64,
    /// Prepare phase failures.
    pub prepare_failures: u64,
    /// Communication timeouts.
    pub timeouts: u64,
}

impl TwoPhaseCommitCoordinator {
    /// Creates a new 2PC coordinator.
    ///
    /// # Arguments
    ///
    /// * `prepare_timeout` - Maximum time to wait for prepare responses.
    pub fn new(prepare_timeout: Duration) -> Self {
        Self {
            participants: Arc::new(RwLock::new(HashMap::new())),
            prepare_timeout,
            stats: Arc::new(RwLock::new(TwoPhaseCommitStats::default())),
        }
    }

    /// Registers a participant for a transaction.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - The transaction ID.
    /// * `participant` - Information about the participant.
    pub fn register_participant(&self, txn_id: TransactionId, participant: ParticipantInfo) {
        let mut participants = self.participants.write();
        participants.entry(txn_id).or_default().push(participant);
    }

    /// Registers multiple participants at once.
    pub fn register_participants(
        &self,
        txn_id: TransactionId,
        new_participants: Vec<ParticipantInfo>,
    ) {
        let mut participants = self.participants.write();
        let entry = participants.entry(txn_id).or_default();
        entry.extend(new_participants);
    }

    /// Executes the prepare phase.
    ///
    /// Asks all participants if they can commit.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if all participants voted yes.
    /// `Ok(false)` if any participant voted no or timed out.
    pub fn prepare_phase(&self, txn_id: TransactionId) -> TransactionResult<bool> {
        let mut participants = self.participants.write();

        let participant_list = participants.get_mut(&txn_id).ok_or_else(|| {
            TransactionError::ParticipantNotFound {
                txn_id,
                participant: "any".to_string(),
            }
        })?;

        // Send prepare to all participants
        for participant in participant_list.iter_mut() {
            participant.state = ParticipantState::Preparing;

            // In production: send actual prepare message
            // Simulate checking timeout
            let elapsed = SystemTime::now()
                .duration_since(participant.last_contact)
                .unwrap_or(Duration::ZERO);

            if elapsed > self.prepare_timeout {
                participant.state = ParticipantState::Failed;
                self.stats.write().prepare_failures += 1;
                self.stats.write().timeouts += 1;
                return Ok(false);
            }

            // Simulate successful prepare
            participant.state = ParticipantState::Prepared;
            participant.touch();
        }

        // Check if all participants are prepared
        let all_prepared = participant_list
            .iter()
            .all(|p| p.state == ParticipantState::Prepared);

        if !all_prepared {
            self.stats.write().prepare_failures += 1;
        }

        Ok(all_prepared)
    }

    /// Executes the commit phase.
    ///
    /// Sends commit to all participants.
    pub fn commit_phase(&self, txn_id: TransactionId) -> TransactionResult<()> {
        let mut participants = self.participants.write();

        let participant_list = participants.get_mut(&txn_id).ok_or_else(|| {
            TransactionError::ParticipantNotFound {
                txn_id,
                participant: "any".to_string(),
            }
        })?;

        for participant in participant_list.iter_mut() {
            // In production: send commit message
            participant.state = ParticipantState::Committed;
            participant.touch();
        }

        // Update statistics and cleanup
        self.stats.write().total_transactions += 1;
        self.stats.write().committed += 1;
        participants.remove(&txn_id);

        Ok(())
    }

    /// Executes the abort phase.
    ///
    /// Sends abort to all participants.
    pub fn abort_phase(&self, txn_id: TransactionId) -> TransactionResult<()> {
        let mut participants = self.participants.write();

        if let Some(participant_list) = participants.get_mut(&txn_id) {
            for participant in participant_list.iter_mut() {
                // In production: send abort message
                participant.state = ParticipantState::Aborted;
                participant.touch();
            }
        }

        // Update statistics and cleanup
        self.stats.write().total_transactions += 1;
        self.stats.write().aborted += 1;
        participants.remove(&txn_id);

        Ok(())
    }

    /// Gets the participants for a transaction.
    pub fn get_participants(&self, txn_id: TransactionId) -> Vec<ParticipantInfo> {
        self.participants
            .read()
            .get(&txn_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Returns the number of active distributed transactions.
    pub fn active_count(&self) -> usize {
        self.participants.read().len()
    }

    /// Returns 2PC statistics.
    pub fn stats(&self) -> TwoPhaseCommitStats {
        self.stats.read().clone()
    }

    /// Cleans up an abandoned transaction.
    pub fn cleanup(&self, txn_id: TransactionId) {
        self.participants.write().remove(&txn_id);
    }
}

impl Default for TwoPhaseCommitCoordinator {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}

impl std::fmt::Debug for TwoPhaseCommitCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TwoPhaseCommitCoordinator")
            .field("prepare_timeout", &self.prepare_timeout)
            .field("active_transactions", &self.active_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_participant() {
        let coord = TwoPhaseCommitCoordinator::new(Duration::from_secs(30));

        let participant = ParticipantInfo::new("node1".to_string());
        coord.register_participant(1, participant);

        let participants = coord.get_participants(1);
        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0].id, "node1");
    }

    #[test]
    fn test_prepare_phase_success() {
        let coord = TwoPhaseCommitCoordinator::new(Duration::from_secs(30));

        let p1 = ParticipantInfo::new("node1".to_string());
        let p2 = ParticipantInfo::new("node2".to_string());
        coord.register_participants(1, vec![p1, p2]);

        let _result = coord.prepare_phase(1);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_commit_phase() {
        let coord = TwoPhaseCommitCoordinator::new(Duration::from_secs(30));

        let participant = ParticipantInfo::new("node1".to_string());
        coord.register_participant(1, participant);

        coord.prepare_phase(1).unwrap();
        coord.commit_phase(1).unwrap();

        // Transaction should be cleaned up
        assert_eq!(coord.active_count(), 0);

        let _stats = coord.stats();
        assert_eq!(stats.committed, 1);
    }

    #[test]
    fn test_abort_phase() {
        let coord = TwoPhaseCommitCoordinator::new(Duration::from_secs(30));

        let participant = ParticipantInfo::new("node1".to_string());
        coord.register_participant(1, participant);

        coord.abort_phase(1).unwrap();

        assert_eq!(coord.active_count(), 0);

        let _stats = coord.stats();
        assert_eq!(stats.aborted, 1);
    }
}
