// Failover Module
//
// Failover coordination, Raft leader election, and session migration

use std::time::Duration;

use super::NodeId;

// ============================================================================
// Failover Coordinator
// ============================================================================

pub struct FailoverCoordinator {
    primary: Option<NodeId>,
    replicas: Vec<NodeId>,
    failover_count: u64,
    total_failover_time_ms: u64,
}

impl FailoverCoordinator {
    pub fn new() -> Self {
        Self {
            primary: None,
            replicas: Vec::new(),
            failover_count: 0,
            total_failover_time_ms: 0,
        }
    }

    pub fn set_primary(&mut self, node_id: NodeId) {
        self.primary = Some(node_id);
    }

    pub fn primary(&self) -> Option<NodeId> {
        self.primary
    }

    pub fn add_replica(&mut self, node_id: NodeId) {
        if !self.replicas.contains(&node_id) {
            self.replicas.push(node_id);
        }
    }

    pub fn remove_replica(&mut self, node_id: NodeId) {
        if let Some(pos) = self.replicas.iter().position(|&id| id == node_id) {
            self.replicas.remove(pos);
        }
    }

    pub fn replicas(&self) -> &[NodeId] {
        &self.replicas
    }

    pub fn promote_replica_to_primary(&mut self) -> Option<NodeId> {
        if let Some(new_primary) = self.replicas.first().copied() {
            self.replicas.remove(0);
            self.primary = Some(new_primary);
            self.failover_count += 1;
            Some(new_primary)
        } else {
            None
        }
    }

    pub fn record_failover(&mut self, duration_ms: u64) {
        self.failover_count += 1;
        self.total_failover_time_ms += duration_ms;
    }

    pub fn metrics(&self) -> FailoverMetrics {
        let avg_failover_time_ms = if self.failover_count > 0 {
            self.total_failover_time_ms as f64 / self.failover_count as f64
        } else {
            0.0
        };

        FailoverMetrics {
            failover_count: self.failover_count,
            avg_failover_time_ms,
        }
    }
}

impl Default for FailoverCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct FailoverMetrics {
    pub failover_count: u64,
    pub avg_failover_time_ms: f64,
}

// ============================================================================
// Raft Leader Election
// ============================================================================

pub struct RaftLeaderElection {
    term: u64,
    voted_for: Option<NodeId>,
    leader: Option<NodeId>,
}

impl RaftLeaderElection {
    pub fn new() -> Self {
        Self {
            term: 0,
            voted_for: None,
            leader: None,
        }
    }

    pub fn term(&self) -> u64 {
        self.term
    }

    pub fn increment_term(&mut self) {
        self.term += 1;
        self.voted_for = None; // Clear vote for new term
    }

    pub fn vote_for(&mut self, node_id: NodeId) -> bool {
        if self.voted_for.is_none() {
            self.voted_for = Some(node_id);
            true
        } else {
            false
        }
    }

    pub fn voted_for(&self) -> Option<NodeId> {
        self.voted_for
    }

    pub fn set_leader(&mut self, node_id: NodeId) {
        self.leader = Some(node_id);
    }

    pub fn leader(&self) -> Option<NodeId> {
        self.leader
    }

    pub fn clear_leader(&mut self) {
        self.leader = None;
    }
}

impl Default for RaftLeaderElection {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Session Migration
// ============================================================================

pub struct SessionMigrationManager {
    migrations_performed: u64,
    failed_migrations: u64,
}

impl SessionMigrationManager {
    pub fn new() -> Self {
        Self {
            migrations_performed: 0,
            failed_migrations: 0,
        }
    }

    pub fn record_successful_migration(&mut self) {
        self.migrations_performed += 1;
    }

    pub fn record_failed_migration(&mut self) {
        self.failed_migrations += 1;
    }

    pub fn migrations_performed(&self) -> u64 {
        self.migrations_performed
    }

    pub fn failed_migrations(&self) -> u64 {
        self.failed_migrations
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.migrations_performed + self.failed_migrations;
        if total > 0 {
            self.migrations_performed as f64 / total as f64
        } else {
            0.0
        }
    }
}

impl Default for SessionMigrationManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Transaction Recovery
// ============================================================================

pub struct TransactionRecoveryManager {
    transactions_recovered: u64,
    recovery_failures: u64,
}

impl TransactionRecoveryManager {
    pub fn new() -> Self {
        Self {
            transactions_recovered: 0,
            recovery_failures: 0,
        }
    }

    pub fn record_successful_recovery(&mut self) {
        self.transactions_recovered += 1;
    }

    pub fn record_failed_recovery(&mut self) {
        self.recovery_failures += 1;
    }

    pub fn transactions_recovered(&self) -> u64 {
        self.transactions_recovered
    }

    pub fn recovery_failures(&self) -> u64 {
        self.recovery_failures
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.transactions_recovered + self.recovery_failures;
        if total > 0 {
            self.transactions_recovered as f64 / total as f64
        } else {
            0.0
        }
    }
}

impl Default for TransactionRecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Rolling Restart Coordinator
// ============================================================================

pub struct RollingRestartCoordinator {
    restart_delay: Duration,
    nodes_restarted: u64,
}

impl RollingRestartCoordinator {
    pub fn new(restart_delay: Duration) -> Self {
        Self {
            restart_delay,
            nodes_restarted: 0,
        }
    }

    pub fn restart_delay(&self) -> Duration {
        self.restart_delay
    }

    pub fn set_restart_delay(&mut self, delay: Duration) {
        self.restart_delay = delay;
    }

    pub fn record_restart(&mut self) {
        self.nodes_restarted += 1;
    }

    pub fn nodes_restarted(&self) -> u64 {
        self.nodes_restarted
    }
}

impl Default for RollingRestartCoordinator {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}
