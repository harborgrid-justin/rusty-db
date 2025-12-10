// Distributed Transaction Coordinator
// Implements two-phase commit, saga pattern, distributed deadlock detection,
// and cross-shard transaction routing for distributed database systems

use std::collections::VecDeque;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::time::Instant;
use std::sync::Mutex;
use std::collections::{HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime};
use parking_lot::{RwLock};
use serde::{Deserialize, Serialize};
use crate::error::DbError;
use super::TransactionId;

/// Distributed transaction identifier
pub type GlobalTxnId = (u64, u32); // (sequence, coordinator_node_id)

/// Participant node in a distributed transaction
#[repr(C)]
#[repr(align(64))] // Cache-line aligned for hot-path performance
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ParticipantNode {
    pub node_id: u32,
    pub address: String,
    pub shard_id: Option<u32>,
}

impl ParticipantNode {
    #[inline]
    pub fn new(node_id: u32, address: String, shard_id: Option<u32>) -> Self {
        Self { node_id, address, shard_id }
    }
}

/// Two-Phase Commit Protocol States
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TwoPhaseCommitState {
    /// Initial state - transaction is being prepared
    Preparing,
    /// Waiting for participant votes
    Voting,
    /// All participants voted yes, committing
    Committing,
    /// At least one participant voted no, aborting
    Aborting,
    /// Transaction committed successfully
    Committed,
    /// Transaction aborted
    Aborted,
    /// Uncertain state (timeout or failure)
    Uncertain,
}

/// Participant vote in 2PC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vote {
    Yes,
    No,
    Timeout,
}

/// Two-Phase Commit Message Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TwoPhaseMessage {
    /// Coordinator -> Participant: Prepare to commit
    Prepare {
        global_txn_id: GlobalTxnId,
        operations: Vec<u8>, // Serialized operations
    },
    /// Participant -> Coordinator: Vote response
    VoteResponse {
        global_txn_id: GlobalTxnId,
        participant: ParticipantNode,
        vote: Vote,
    },
    /// Coordinator -> Participant: Final decision
    Commit {
        global_txn_id: GlobalTxnId,
    },
    /// Coordinator -> Participant: Final decision
    Abort {
        global_txn_id: GlobalTxnId,
    },
    /// Participant -> Coordinator: Acknowledgment
    Ack {
        global_txn_id: GlobalTxnId,
        participant: ParticipantNode,
    },
}

/// Two-Phase Commit Coordinator with Presumed Abort Optimization
pub struct TwoPhaseCommitCoordinator {
    /// Coordinator node ID
    node_id: u32,
    /// Next transaction sequence number
    next_txn_seq: Arc<AtomicU64>,
    /// Active distributed transactions
    active_txns: Arc<RwLock<HashMap<GlobalTxnId, DistributedTransaction>>>,
    /// Transaction log for recovery
    txn_log: Arc<Mutex<TransactionLog>>,
    /// Configuration
    config: TwoPhaseConfig,
    /// Statistics
    stats: Arc<RwLock<TwoPhaseStats>>,
}

#[derive(Debug, Clone)]
pub struct TwoPhaseConfig {
    /// Timeout for prepare phase (milliseconds)
    pub prepare_timeout_ms: u64,
    /// Timeout for commit phase (milliseconds)
    pub commit_timeout_ms: u64,
    /// Enable presumed abort optimization
    pub presumed_abort: bool,
    /// Maximum concurrent distributed transactions
    pub max_concurrent_txns: usize,
}

impl Default for TwoPhaseConfig {
    fn default() -> Self {
        Self {
            prepare_timeout_ms: 5000,
            commit_timeout_ms: 10000,
            presumed_abort: true,
            max_concurrent_txns: 1000,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TwoPhaseStats {
    pub total_transactions: u64,
    pub committed_transactions: u64,
    pub aborted_transactions: u64,
    pub timeout_aborts: u64,
    pub participant_aborts: u64,
    pub avg_prepare_time_ms: f64,
    pub avg_commit_time_ms: f64,
}

/// Distributed transaction state
#[derive(Debug, Clone)]
pub struct DistributedTransaction {
    pub global_txn_id: GlobalTxnId,
    pub state: TwoPhaseCommitState,
    pub participants: Vec<ParticipantNode>,
    pub votes: HashMap<u32, Vote>,
    pub start_time: Instant,
    pub prepare_time: Option<Instant>,
    pub commit_time: Option<Instant>,
    pub operations: Vec<u8>,
}

impl TwoPhaseCommitCoordinator {
    /// Create a new coordinator
    pub fn new(node_id: u32, config: TwoPhaseConfig) -> Self {
        Self {
            node_id,
            next_txn_seq: Arc::new(AtomicU64::new(1)),
            active_txns: Arc::new(RwLock::new(HashMap::new())),
            txn_log: Arc::new(Mutex::new(TransactionLog::new())),
            config,
            stats: Arc::new(RwLock::new(TwoPhaseStats::default())),
        }
    }

    /// Begin a new distributed transaction
    pub fn begin_transaction(
        &self,
        participants: Vec<ParticipantNode>,
        operations: Vec<u8>,
    ) -> std::result::Result<GlobalTxnId, DbError> {
        // Check concurrent transaction limit
        if self.active_txns.read().len() >= self.config.max_concurrent_txns {
            return Err(DbError::Transaction(
                "Maximum concurrent distributed transactions reached".to_string()
            ));
        }

        let seq = self.next_txn_seq.fetch_add(1, Ordering::SeqCst);
        let global_txn_id = (seq, self.node_id);

        let txn = DistributedTransaction {
            global_txn_id,
            state: TwoPhaseCommitState::Preparing,
            participants: participants.clone(),
            votes: HashMap::new(),
            start_time: Instant::now(),
            prepare_time: None,
            commit_time: None,
            operations,
        };

        self.active_txns.write().insert(global_txn_id, txn);
        self.stats.write().total_transactions += 1;

        Ok(global_txn_id)
    }

    /// Execute prepare phase
    pub async fn prepare_phase(&self, global_txn_id: GlobalTxnId) -> std::result::Result<bool, DbError> {
        let mut txn = {
            let txns = self.active_txns.read();
            txns.get(&global_txn_id)
                .ok_or_else(|| DbError::Transaction("Transaction not found".to_string()))?
                .clone()
        };

        txn.state = TwoPhaseCommitState::Voting;
        txn.prepare_time = Some(Instant::now());

        // Send prepare messages to all participants
        let _prepare_msg = TwoPhaseMessage::Prepare {
            global_txn_id,
            operations: txn.operations.clone(),
        };

        // Simulate sending prepare messages (in real implementation, use network)
        for participant in &txn.participants {
            // In production, this would be an async network call
            let vote = self.simulate_participant_vote(participant);
            txn.votes.insert(participant.node_id, vote);
        }

        // Check votes
        let all_yes = txn.votes.values().all(|v| *v == Vote::Yes);

        // Update transaction state
        self.active_txns.write().insert(global_txn_id, txn.clone());

        // Log prepare outcome
        if all_yes {
            self.txn_log.lock().unwrap().log_prepared(global_txn_id);
        } else if self.config.presumed_abort {
            // With presumed abort, we don't need to log abort
            self.txn_log.lock().unwrap().log_aborted(global_txn_id);
        }

        Ok(all_yes)
    }

    /// Execute commit phase
    pub async fn commit_phase(&self, global_txn_id: GlobalTxnId, commit: bool) -> std::result::Result<(), DbError> {
        let mut txn = {
            let txns = self.active_txns.read();
            txns.get(&global_txn_id)
                .ok_or_else(|| DbError::Transaction("Transaction not found".to_string()))?
                .clone()
        };

        if commit {
            txn.state = TwoPhaseCommitState::Committing;
            txn.commit_time = Some(Instant::now());

            // Send commit messages to all participants
            let _commit_msg = TwoPhaseMessage::Commit { global_txn_id };

            // Simulate sending commit messages
            for participant in &txn.participants {
                // In production, this would be an async network call
                self.simulate_participant_ack(participant);
            }

            txn.state = TwoPhaseCommitState::Committed;
            self.txn_log.lock().unwrap().log_committed(global_txn_id);
            self.stats.write().committed_transactions += 1;
        } else {
            txn.state = TwoPhaseCommitState::Aborting;

            // Send abort messages to all participants
            let _abort_msg = TwoPhaseMessage::Abort { global_txn_id };

            // Simulate sending abort messages
            for participant in &txn.participants {
                self.simulate_participant_ack(participant);
            }

            txn.state = TwoPhaseCommitState::Aborted;
            self.stats.write().aborted_transactions += 1;
        }

        // Update statistics
        if let Some(prepare_time) = txn.prepare_time {
            let prepare_duration = prepare_time.duration_since(txn.start_time).as_millis() as f64;
            let mut stats = self.stats.write();
            stats.avg_prepare_time_ms =
                (stats.avg_prepare_time_ms * (stats.total_transactions - 1) as f64 + prepare_duration)
                    / stats.total_transactions as f64;
        }

        if let Some(commit_time) = txn.commit_time {
            if let Some(prepare_time) = txn.prepare_time {
                let commit_duration = commit_time.duration_since(prepare_time).as_millis() as f64;
                let mut stats = self.stats.write();
                stats.avg_commit_time_ms =
                    (stats.avg_commit_time_ms * (stats.committed_transactions - 1) as f64 + commit_duration)
                        / stats.committed_transactions.max(1) as f64;
            }
        }

        // Remove from active transactions
        self.active_txns.write().remove(&global_txn_id);

        Ok(())
    }

    /// Execute full 2PC protocol
    pub async fn execute_2pc(&self, global_txn_id: GlobalTxnId) -> std::result::Result<bool, DbError> {
        let can_commit = self.prepare_phase(global_txn_id).await?;
        self.commit_phase(global_txn_id, can_commit).await?;
        Ok(can_commit)
    }

    /// Get statistics
    pub fn get_stats(&self) -> TwoPhaseStats {
        self.stats.read().clone()
    }

    // Simulation helpers (replace with actual network calls in production)
    fn simulate_participant_vote(&self, _participant: &ParticipantNode) -> Vote {
        // Simulate random vote for testing
        Vote::Yes
    }

    fn simulate_participant_ack(&self, _participant: &ParticipantNode) {
        // Simulate acknowledgment
    }
}

/// Saga Pattern for Long-Running Distributed Transactions
/// Provides compensation-based transaction management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStep {
    pub name: String,
    pub participant: ParticipantNode,
    pub operation: Vec<u8>,
    pub compensation: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SagaState {
    Running,
    Compensating,
    Completed,
    Compensated,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Saga {
    pub saga_id: u64,
    pub steps: Vec<SagaStep>,
    pub completed_steps: Vec<usize>,
    pub state: SagaState,
    pub start_time: Instant,
}

pub struct SagaCoordinator {
    #[allow(dead_code)]
    node_id: u32,
    next_saga_id: Arc<AtomicU64>,
    active_sagas: Arc<RwLock<HashMap<u64, Saga>>>,
    config: SagaConfig,
    stats: Arc<RwLock<SagaStats>>,
}

#[derive(Debug, Clone)]
pub struct SagaConfig {
    pub max_concurrent_sagas: usize,
    pub step_timeout_ms: u64,
    pub max_retry_attempts: u32,
}

impl Default for SagaConfig {
    fn default() -> Self {
        Self {
            max_concurrent_sagas: 500,
            step_timeout_ms: 30000,
            max_retry_attempts: 3,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SagaStats {
    pub total_sagas: u64,
    pub completed_sagas: u64,
    pub compensated_sagas: u64,
    pub failed_sagas: u64,
    pub avg_execution_time_ms: f64,
}

impl SagaCoordinator {
    pub fn new(node_id: u32, config: SagaConfig) -> Self {
        Self {
            node_id,
            next_saga_id: Arc::new(AtomicU64::new(1)),
            active_sagas: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(SagaStats::default())),
        }
    }

    /// Begin a new saga
    pub fn begin_saga(&self, steps: Vec<SagaStep>) -> std::result::Result<u64, DbError> {
        if self.active_sagas.read().len() >= self.config.max_concurrent_sagas {
            return Err(DbError::Transaction(
                "Maximum concurrent sagas reached".to_string()
            ));
        }

        let saga_id = self.next_saga_id.fetch_add(1, Ordering::SeqCst);
        let saga = Saga {
            saga_id,
            steps,
            completed_steps: Vec::new(),
            state: SagaState::Running,
            start_time: Instant::now(),
        };

        self.active_sagas.write().insert(saga_id, saga);
        self.stats.write().total_sagas += 1;

        Ok(saga_id)
    }

    /// Execute saga steps
    pub async fn execute_saga(&self, saga_id: u64) -> std::result::Result<bool, DbError> {
        let mut saga = {
            let sagas = self.active_sagas.read();
            sagas.get(&saga_id)
                .ok_or_else(|| DbError::Transaction("Saga not found".to_string()))?
                .clone()
        };

        // Execute each step in order
        for (idx, step) in saga.steps.iter().enumerate() {
            // Simulate step execution
            let success = self.execute_saga_step(step).await?;

            if success {
                saga.completed_steps.push(idx);
                self.active_sagas.write().insert(saga_id, saga.clone());
            } else {
                // Step failed, start compensation
                return self.compensate_saga(saga_id).await;
            }
        }

        // All steps completed successfully
        saga.state = SagaState::Completed;
        let execution_time = saga.start_time.elapsed().as_millis() as f64;

        let mut stats = self.stats.write();
        stats.completed_sagas += 1;
        stats.avg_execution_time_ms =
            (stats.avg_execution_time_ms * (stats.completed_sagas - 1) as f64 + execution_time)
                / stats.completed_sagas as f64;

        self.active_sagas.write().remove(&saga_id);
        Ok(true)
    }

    /// Compensate saga (execute compensation actions in reverse order)
    async fn compensate_saga(&self, saga_id: u64) -> std::result::Result<bool, DbError> {
        let mut saga = {
            let sagas = self.active_sagas.read();
            sagas.get(&saga_id)
                .ok_or_else(|| DbError::Transaction("Saga not found".to_string()))?
                .clone()
        };

        saga.state = SagaState::Compensating;

        // Execute compensations in reverse order
        for &step_idx in saga.completed_steps.iter().rev() {
            let step = &saga.steps[step_idx];
            self.execute_compensation(step).await?;
        }

        saga.state = SagaState::Compensated;
        self.stats.write().compensated_sagas += 1;
        self.active_sagas.write().remove(&saga_id);

        Ok(false)
    }

    async fn execute_saga_step(&self, _step: &SagaStep) -> std::result::Result<bool, DbError> {
        // Simulate step execution
        Ok(true)
    }

    async fn execute_compensation(&self, _step: &SagaStep) -> std::result::Result<(), DbError> {
        // Simulate compensation execution
        Ok(())
    }

    pub fn get_stats(&self) -> SagaStats {
        self.stats.read().clone()
    }
}

/// Distributed Deadlock Detector using Wait-For Graph
pub struct DistributedDeadlockDetector {
    #[allow(dead_code)]
    node_id: u32,
    /// Local wait-for graph
    local_graph: Arc<RwLock<WaitForGraph>>,
    /// Global wait-for graph (merged from all nodes)
    global_graph: Arc<RwLock<WaitForGraph>>,
    /// Detection configuration
    config: DeadlockConfig,
    /// Statistics
    stats: Arc<RwLock<DeadlockStats>>,
}

#[derive(Debug, Clone)]
pub struct DeadlockConfig {
    pub detection_interval_ms: u64,
    pub enable_distributed_detection: bool,
    pub victim_selection_strategy: VictimSelectionStrategy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VictimSelectionStrategy {
    YoungestFirst,
    OldestFirst,
    MinimumCost,
    Random,
}

impl Default for DeadlockConfig {
    fn default() -> Self {
        Self {
            detection_interval_ms: 1000,
            enable_distributed_detection: true,
            victim_selection_strategy: VictimSelectionStrategy::YoungestFirst,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeadlockStats {
    pub deadlocks_detected: u64,
    pub local_deadlocks: u64,
    pub distributed_deadlocks: u64,
    pub transactions_aborted: u64,
}

/// Wait-For Graph for deadlock detection
#[derive(Debug, Clone)]
pub struct WaitForGraph {
    /// Edges: txn_id -> set of transactions it's waiting for
    edges: HashMap<TransactionId, HashSet<TransactionId>>,
    /// Transaction timestamps for victim selection
    timestamps: HashMap<TransactionId, SystemTime>,
}

impl WaitForGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            timestamps: HashMap::new(),
        }
    }

    /// Add a wait edge
    pub fn add_edge(&mut self, waiter: TransactionId, holder: TransactionId) {
        self.edges.entry(waiter).or_insert_with(HashSet::new).insert(holder);
        self.timestamps.entry(waiter).or_insert_with(SystemTime::now);
        self.timestamps.entry(holder).or_insert_with(SystemTime::now);
    }

    /// Remove a transaction from the graph
    pub fn remove_transaction(&mut self, txn_id: TransactionId) {
        self.edges.remove(&txn_id);
        self.timestamps.remove(&txn_id);

        // Remove from other waiting lists
        for waiting_set in self.edges.values_mut() {
            waiting_set.remove(&txn_id);
        }
    }

    /// Detect cycles using DFS
    pub fn detect_cycle(&self) -> Option<Vec<TransactionId>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for &txn_id in self.edges.keys() {
            if !visited.contains(&txn_id) {
                if let Some(cycle) = self.dfs_cycle(txn_id, &mut visited, &mut rec_stack, &mut path) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    fn dfs_cycle(
        &self,
        txn_id: TransactionId,
        visited: &mut HashSet<TransactionId>,
        rec_stack: &mut HashSet<TransactionId>,
        path: &mut Vec<TransactionId>,
    ) -> Option<Vec<TransactionId>> {
        visited.insert(txn_id);
        rec_stack.insert(txn_id);
        path.push(txn_id);

        if let Some(waiting_for) = self.edges.get(&txn_id) {
            for &next_txn in waiting_for {
                if !visited.contains(&next_txn) {
                    if let Some(cycle) = self.dfs_cycle(next_txn, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(&next_txn) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|&id| id == next_txn).unwrap();
                    return Some(path[cycle_start..].to_vec());
                }
            }
        }

        path.pop();
        rec_stack.remove(&txn_id);
        None
    }

    /// Select victim from a cycle based on strategy
    pub fn select_victim(
        &self,
        cycle: &[TransactionId],
        strategy: VictimSelectionStrategy,
    ) -> TransactionId {
        match strategy {
            VictimSelectionStrategy::YoungestFirst => {
                // Select transaction with most recent timestamp
                *cycle.iter()
                    .max_by_key(|&&txn_id| self.timestamps.get(&txn_id))
                    .unwrap()
            }
            VictimSelectionStrategy::OldestFirst => {
                // Select transaction with oldest timestamp
                *cycle.iter()
                    .min_by_key(|&&txn_id| self.timestamps.get(&txn_id))
                    .unwrap()
            }
            VictimSelectionStrategy::MinimumCost | VictimSelectionStrategy::Random => {
                // For simplicity, use youngest first
                *cycle.iter()
                    .max_by_key(|&&txn_id| self.timestamps.get(&txn_id))
                    .unwrap()
            }
        }
    }
}

impl DistributedDeadlockDetector {
    pub fn new(node_id: u32, config: DeadlockConfig) -> Self {
        Self {
            node_id,
            local_graph: Arc::new(RwLock::new(WaitForGraph::new())),
            global_graph: Arc::new(RwLock::new(WaitForGraph::new())),
            config,
            stats: Arc::new(RwLock::new(DeadlockStats::default())),
        }
    }

    /// Add a wait dependency
    pub fn add_wait(&self, waiter: TransactionId, holder: TransactionId) {
        self.local_graph.write().add_edge(waiter, holder);
    }

    /// Remove a transaction
    pub fn remove_transaction(&self, txn_id: TransactionId) {
        self.local_graph.write().remove_transaction(txn_id);
        self.global_graph.write().remove_transaction(txn_id);
    }

    /// Run deadlock detection
    pub fn detect_deadlock(&self) -> Option<TransactionId> {
        // Check local graph first
        if let Some(cycle) = self.local_graph.read().detect_cycle() {
            let victim = self.local_graph.read().select_victim(&cycle, self.config.victim_selection_strategy);

            let mut stats = self.stats.write();
            stats.deadlocks_detected += 1;
            stats.local_deadlocks += 1;
            stats.transactions_aborted += 1;

            return Some(victim);
        }

        // Check global graph if distributed detection enabled
        if self.config.enable_distributed_detection {
            if let Some(cycle) = self.global_graph.read().detect_cycle() {
                let victim = self.global_graph.read().select_victim(&cycle, self.config.victim_selection_strategy);

                let mut stats = self.stats.write();
                stats.deadlocks_detected += 1;
                stats.distributed_deadlocks += 1;
                stats.transactions_aborted += 1;

                return Some(victim);
            }
        }

        None
    }

    /// Merge local graph into global graph (for distributed detection)
    pub fn update_global_graph(&self, remote_graph: WaitForGraph) {
        let mut global = self.global_graph.write();
        let local = self.local_graph.read();

        // Merge local graph
        for (&waiter, holders) in &local.edges {
            for &holder in holders {
                global.add_edge(waiter, holder);
            }
        }

        // Merge remote graph
        for (&waiter, holders) in &remote_graph.edges {
            for &holder in holders {
                global.add_edge(waiter, holder);
            }
        }
    }

    pub fn get_stats(&self) -> DeadlockStats {
        self.stats.read().clone()
    }
}

/// Cross-Shard Transaction Router
pub struct CrossShardRouter {
    #[allow(dead_code)]
    node_id: u32,
    /// Shard mapping: key range -> shard
    shard_map: Arc<RwLock<BTreeMap<Vec<u8>, u32>>>,
    /// Shard to node mapping
    shard_nodes: Arc<RwLock<HashMap<u32, ParticipantNode>>>,
    /// Statistics
    stats: Arc<RwLock<RouterStats>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouterStats {
    pub total_routed: u64,
    pub single_shard_txns: u64,
    pub cross_shard_txns: u64,
    pub avg_shards_per_txn: f64,
}

impl CrossShardRouter {
    pub fn new(node_id: u32) -> Self {
        Self {
            node_id,
            shard_map: Arc::new(RwLock::new(BTreeMap::new())),
            shard_nodes: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(RouterStats::default())),
        }
    }

    /// Register a shard
    pub fn register_shard(&self, key_range: Vec<u8>, shard_id: u32, node: ParticipantNode) {
        self.shard_map.write().insert(key_range, shard_id);
        self.shard_nodes.write().insert(shard_id, node);
    }

    /// Route a key to its shard
    pub fn route_key(&self, key: &[u8]) -> Option<u32> {
        let shard_map = self.shard_map.read();
        for (range, &shard_id) in shard_map.iter() {
            if key >= range.as_slice() {
                return Some(shard_id);
            }
        }
        None
    }

    /// Get participants for a set of keys
    pub fn get_participants(&self, keys: &[Vec<u8>]) -> Vec<ParticipantNode> {
        let mut shards = HashSet::new();

        for key in keys {
            if let Some(shard_id) = self.route_key(key) {
                shards.insert(shard_id);
            }
        }

        let shard_nodes = self.shard_nodes.read();
        let participants: Vec<_> = shards
            .iter()
            .filter_map(|shard_id| shard_nodes.get(shard_id).cloned())
            .collect();

        // Update statistics
        let mut stats = self.stats.write();
        stats.total_routed += 1;

        if participants.len() == 1 {
            stats.single_shard_txns += 1;
        } else if participants.len() > 1 {
            stats.cross_shard_txns += 1;
        }

        stats.avg_shards_per_txn =
            (stats.avg_shards_per_txn * (stats.total_routed - 1) as f64 + participants.len() as f64)
                / stats.total_routed as f64;

        participants
    }

    pub fn get_stats(&self) -> RouterStats {
        self.stats.read().clone()
    }
}

/// Transaction log for 2PC recovery
struct TransactionLog {
    entries: VecDeque<LogEntry>,
    max_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LogEntry {
    global_txn_id: GlobalTxnId,
    state: TwoPhaseCommitState,
    timestamp: SystemTime,
}

impl TransactionLog {
    fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries: 10000,
        }
    }

    fn log_prepared(&mut self, global_txn_id: GlobalTxnId) {
        self.add_entry(global_txn_id, TwoPhaseCommitState::Preparing);
    }

    fn log_committed(&mut self, global_txn_id: GlobalTxnId) {
        self.add_entry(global_txn_id, TwoPhaseCommitState::Committed);
    }

    fn log_aborted(&mut self, global_txn_id: GlobalTxnId) {
        self.add_entry(global_txn_id, TwoPhaseCommitState::Aborted);
    }

    fn add_entry(&mut self, global_txn_id: GlobalTxnId, state: TwoPhaseCommitState) {
        let entry = LogEntry {
            global_txn_id,
            state,
            timestamp: SystemTime::now(),
        };

        self.entries.push_back(entry);

        if self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wait_for_graph_cycle_detection() {
        let mut graph = WaitForGraph::new();

        // Create a cycle: 1 -> 2 -> 3 -> 1
        graph.add_edge(1, 2);
        graph.add_edge(2, 3);
        graph.add_edge(3, 1);

        let cycle = graph.detect_cycle();
        assert!(cycle.is_some());
        assert!(cycle.unwrap().contains(&1));
    }

    #[test]
    fn test_cross_shard_routing() {
        let router = CrossShardRouter::new(1);

        let node1 = ParticipantNode::new(1, "localhost:5001".to_string(), Some(1));
        let node2 = ParticipantNode::new(2, "localhost:5002".to_string(), Some(2));

        router.register_shard(vec![0], 1, node1);
        router.register_shard(vec![128], 2, node2);

        let key1 = vec![50];
        let key2 = vec![200];

        assert_eq!(router.route_key(&key1), Some(1));
        assert_eq!(router.route_key(&key2), Some(2));

        let participants = router.get_participants(&vec![key1, key2]);
        assert_eq!(participants.len(), 2);
    }
}


