// Transaction statistics and monitoring.
//
// This module provides statistics collection for transaction operations,
// enabling performance monitoring and capacity planning.
//
// # Unified Statistics Interface
//
// All statistics types implement the `ComponentStats` trait for consistent
// access patterns across the transaction layer.
//
// # Example
//
// ```rust,ignore
// let stats = TransactionStatistics::new();
// stats.record_begin();
// // ... transaction operations ...
// stats.record_commit(latency_ms);
// let summary = stats.get_summary();
// ```

use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Common trait for all statistics components in the transaction layer.
///
/// This trait provides a unified interface for statistics collection,
/// enabling consistent monitoring across different transaction components.
pub trait ComponentStats: Send + Sync {
    /// Type of summary produced by this statistics component.
    type Summary: Clone + Send + Sync;

    /// Gets a snapshot of current statistics.
    fn get_summary(&self) -> Self::Summary;

    /// Resets all statistics counters to zero.
    fn reset(&self);

    /// Returns a human-readable description of the component.
    fn component_name(&self) -> &'static str;
}

/// Transaction statistics collector.
///
/// Thread-safe collector for transaction metrics.
pub struct TransactionStatistics {
    /// Total committed transactions.
    total_commits: Arc<Mutex<u64>>,
    /// Total aborted transactions.
    total_aborts: Arc<Mutex<u64>>,
    /// Total deadlocks detected.
    total_deadlocks: Arc<Mutex<u64>>,
    /// Total lock timeouts.
    total_timeouts: Arc<Mutex<u64>>,
    /// Currently active transactions.
    active_count: Arc<Mutex<u64>>,
    /// Commit latency samples (milliseconds).
    commit_latency_ms: Arc<Mutex<Vec<u64>>>,
}

impl TransactionStatistics {
    /// Creates a new statistics collector.
    pub fn new() -> Self {
        Self {
            total_commits: Arc::new(Mutex::new(0)),
            total_aborts: Arc::new(Mutex::new(0)),
            total_deadlocks: Arc::new(Mutex::new(0)),
            total_timeouts: Arc::new(Mutex::new(0)),
            active_count: Arc::new(Mutex::new(0)),
            commit_latency_ms: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Records a transaction begin.
    pub fn record_begin(&self) {
        *self.active_count.lock() += 1;
    }

    /// Records a transaction commit with latency.
    pub fn record_commit(&self, latency_ms: u64) {
        *self.total_commits.lock() += 1;
        let mut active = self.active_count.lock();
        if *active > 0 {
            *active -= 1;
        }

        let mut latencies = self.commit_latency_ms.lock();
        latencies.push(latency_ms);
        // Keep only last 10000 samples
        if latencies.len() > 10000 {
            latencies.remove(0);
        }
    }

    /// Records a transaction abort.
    pub fn record_abort(&self) {
        *self.total_aborts.lock() += 1;
        let mut active = self.active_count.lock();
        if *active > 0 {
            *active -= 1;
        }
    }

    /// Records a deadlock detection.
    pub fn record_deadlock(&self) {
        *self.total_deadlocks.lock() += 1;
    }

    /// Records a lock timeout.
    pub fn record_timeout(&self) {
        *self.total_timeouts.lock() += 1;
    }

    /// Gets a summary of all statistics.
    pub fn get_summary(&self) -> StatisticsSummary {
        let latencies = self.commit_latency_ms.lock();
        let avg_latency = if !latencies.is_empty() {
            latencies.iter().sum::<u64>() / latencies.len() as u64
        } else {
            0
        };

        StatisticsSummary {
            total_commits: *self.total_commits.lock(),
            total_aborts: *self.total_aborts.lock(),
            total_deadlocks: *self.total_deadlocks.lock(),
            total_timeouts: *self.total_timeouts.lock(),
            active_transactions: *self.active_count.lock(),
            avg_commit_latency_ms: avg_latency,
            abort_rate: self.calculate_abort_rate(),
        }
    }

    /// Calculates the abort rate (aborts / total).
    fn calculate_abort_rate(&self) -> f64 {
        let commits = *self.total_commits.lock() as f64;
        let aborts = *self.total_aborts.lock() as f64;
        let total = commits + aborts;

        if total > 0.0 {
            aborts / total
        } else {
            0.0
        }
    }

    /// Resets all statistics.
    pub fn reset(&self) {
        *self.total_commits.lock() = 0;
        *self.total_aborts.lock() = 0;
        *self.total_deadlocks.lock() = 0;
        *self.total_timeouts.lock() = 0;
        self.commit_latency_ms.lock().clear();
    }

    /// Returns the p99 commit latency.
    pub fn p99_latency(&self) -> u64 {
        let mut latencies = self.commit_latency_ms.lock().clone();
        if latencies.is_empty() {
            return 0;
        }
        latencies.sort_unstable();
        let idx = (latencies.len() as f64 * 0.99) as usize;
        latencies
            .get(idx.min(latencies.len() - 1))
            .copied()
            .unwrap_or(0)
    }
}

impl Default for TransactionStatistics {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentStats for TransactionStatistics {
    type Summary = StatisticsSummary;

    fn get_summary(&self) -> Self::Summary {
        self.get_summary()
    }

    fn reset(&self) {
        self.reset();
    }

    fn component_name(&self) -> &'static str {
        "TransactionStatistics"
    }
}

/// Summary of transaction statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsSummary {
    /// Total committed transactions.
    pub total_commits: u64,
    /// Total aborted transactions.
    pub total_aborts: u64,
    /// Total deadlocks detected.
    pub total_deadlocks: u64,
    /// Total lock timeouts.
    pub total_timeouts: u64,
    /// Currently active transactions.
    pub active_transactions: u64,
    /// Average commit latency in milliseconds.
    pub avg_commit_latency_ms: u64,
    /// Abort rate (0.0 to 1.0).
    pub abort_rate: f64,
}

impl StatisticsSummary {
    /// Returns the total number of completed transactions.
    pub fn total_completed(&self) -> u64 {
        self.total_commits + self.total_aborts
    }

    /// Returns the commit rate (commits / total).
    pub fn commit_rate(&self) -> f64 {
        1.0 - self.abort_rate
    }
}

/// Lock statistics collector.
pub struct LockStatistics {
    /// Total lock requests.
    lock_requests: Arc<Mutex<u64>>,
    /// Total lock waits.
    lock_waits: Arc<Mutex<u64>>,
    /// Total lock timeouts.
    lock_timeouts: Arc<Mutex<u64>>,
    /// Total deadlocks detected.
    deadlocks_detected: Arc<Mutex<u64>>,
    /// Total lock escalations performed.
    lock_escalations: Arc<Mutex<u64>>,
    /// Total row locks escalated.
    rows_escalated: Arc<Mutex<u64>>,
    /// Wait time samples (milliseconds).
    wait_times_ms: Arc<Mutex<Vec<u64>>>,
}

impl LockStatistics {
    /// Creates a new lock statistics collector.
    pub fn new() -> Self {
        Self {
            lock_requests: Arc::new(Mutex::new(0)),
            lock_waits: Arc::new(Mutex::new(0)),
            lock_timeouts: Arc::new(Mutex::new(0)),
            deadlocks_detected: Arc::new(Mutex::new(0)),
            lock_escalations: Arc::new(Mutex::new(0)),
            rows_escalated: Arc::new(Mutex::new(0)),
            wait_times_ms: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Records a lock request.
    pub fn record_request(&self) {
        *self.lock_requests.lock() += 1;
    }

    /// Records a lock wait with duration.
    pub fn record_wait(&self, wait_time_ms: u64) {
        *self.lock_waits.lock() += 1;
        let mut times = self.wait_times_ms.lock();
        times.push(wait_time_ms);
        if times.len() > 10000 {
            times.remove(0);
        }
    }

    /// Records a lock timeout.
    pub fn record_timeout(&self) {
        *self.lock_timeouts.lock() += 1;
    }

    /// Records a deadlock.
    pub fn record_deadlock(&self) {
        *self.deadlocks_detected.lock() += 1;
    }

    /// Records a lock escalation.
    ///
    /// # Arguments
    ///
    /// * `row_count` - Number of row locks that were escalated to a table lock.
    pub fn record_escalation(&self, row_count: usize) {
        *self.lock_escalations.lock() += 1;
        *self.rows_escalated.lock() += row_count as u64;
    }

    /// Gets a summary of lock statistics.
    pub fn get_summary(&self) -> LockStatisticsSummary {
        let wait_times = self.wait_times_ms.lock();
        let avg_wait = if !wait_times.is_empty() {
            wait_times.iter().sum::<u64>() / wait_times.len() as u64
        } else {
            0
        };

        LockStatisticsSummary {
            total_requests: *self.lock_requests.lock(),
            total_waits: *self.lock_waits.lock(),
            total_timeouts: *self.lock_timeouts.lock(),
            total_deadlocks: *self.deadlocks_detected.lock(),
            total_escalations: *self.lock_escalations.lock(),
            total_rows_escalated: *self.rows_escalated.lock(),
            avg_wait_time_ms: avg_wait,
        }
    }
}

impl Default for LockStatistics {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentStats for LockStatistics {
    type Summary = LockStatisticsSummary;

    fn get_summary(&self) -> Self::Summary {
        self.get_summary()
    }

    fn reset(&self) {
        *self.lock_requests.lock() = 0;
        *self.lock_waits.lock() = 0;
        *self.lock_timeouts.lock() = 0;
        *self.deadlocks_detected.lock() = 0;
        *self.lock_escalations.lock() = 0;
        *self.rows_escalated.lock() = 0;
        self.wait_times_ms.lock().clear();
    }

    fn component_name(&self) -> &'static str {
        "LockStatistics"
    }
}

/// Summary of lock statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockStatisticsSummary {
    /// Total lock requests.
    pub total_requests: u64,
    /// Total lock waits.
    pub total_waits: u64,
    /// Total lock timeouts.
    pub total_timeouts: u64,
    /// Total deadlocks detected.
    pub total_deadlocks: u64,
    /// Total lock escalations performed (row -> table).
    pub total_escalations: u64,
    /// Total row locks escalated to table locks.
    pub total_rows_escalated: u64,
    /// Average wait time in milliseconds.
    pub avg_wait_time_ms: u64,
}

impl LockStatisticsSummary {
    /// Returns the contention rate (waits / requests).
    pub fn contention_rate(&self) -> f64 {
        if self.total_requests > 0 {
            self.total_waits as f64 / self.total_requests as f64
        } else {
            0.0
        }
    }

    /// Returns the escalation rate (escalations / requests).
    pub fn escalation_rate(&self) -> f64 {
        if self.total_requests > 0 {
            self.total_escalations as f64 / self.total_requests as f64
        } else {
            0.0
        }
    }

    /// Returns the average row locks per escalation.
    pub fn avg_rows_per_escalation(&self) -> f64 {
        if self.total_escalations > 0 {
            self.total_rows_escalated as f64 / self.total_escalations as f64
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_statistics() {
        let stats = TransactionStatistics::new();

        stats.record_begin();
        stats.record_begin();
        stats.record_commit(10);
        stats.record_abort();

        let summary = stats.get_summary();
        assert_eq!(summary.total_commits, 1);
        assert_eq!(summary.total_aborts, 1);
        assert_eq!(summary.active_transactions, 0);
        assert!((summary.abort_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_lock_statistics() {
        let stats = LockStatistics::new();

        stats.record_request();
        stats.record_request();
        stats.record_wait(5);
        stats.record_timeout();

        let summary = stats.get_summary();
        assert_eq!(summary.total_requests, 2);
        assert_eq!(summary.total_waits, 1);
        assert_eq!(summary.total_timeouts, 1);
    }

    #[test]
    fn test_reset_statistics() {
        let stats = TransactionStatistics::new();

        stats.record_begin();
        stats.record_commit(10);
        stats.reset();

        let summary = stats.get_summary();
        assert_eq!(summary.total_commits, 0);
    }
}
