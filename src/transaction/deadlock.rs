// Deadlock detection for transactions.
//
// This module implements deadlock detection using a wait-for graph.
// It can detect cycles in the dependency graph between transactions
// and select victims for resolution.
//
// # Algorithm
//
// Uses depth-first search (DFS) to detect cycles in the wait-for graph.
// When a deadlock is detected, a victim is selected based on configurable
// policies.
//
// # Example
//
// ```rust,ignore
// let detector = DeadlockDetector::new(Duration::from_secs(1));
// detector.add_wait(txn1, txn2);  // txn1 is waiting for txn2
// if let Some(cycle) = detector.detect_deadlock() {
//     let victim = detector.select_victim(&cycle);
//     // Abort the victim transaction
// }
// ```

use std::fmt;
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::{HashMap};
use std::sync::Arc;
use std::time::{Duration};

use parking_lot::{Mutex, RwLock};

use crate::common::TransactionId;

/// Policy for selecting which transaction to abort in a deadlock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VictimSelectionPolicy {
    /// Abort the youngest transaction (highest ID).
    Youngest,
    /// Abort the oldest transaction (lowest ID).
    Oldest,
    /// Abort the transaction with the least work done.
    LeastWork,
    /// Abort the transaction with the lowest priority.
    LowestPriority,
}

impl Default for VictimSelectionPolicy {
    fn default() -> Self {
        VictimSelectionPolicy::Youngest
    }
}

/// Configuration for deadlock detection.
#[derive(Debug, Clone)]
pub struct DeadlockDetectorConfig {
    /// Minimum interval between detection runs.
    pub detection_interval: Duration,
    /// Policy for victim selection.
    pub victim_policy: VictimSelectionPolicy,
    /// Maximum depth for cycle detection (prevents infinite loops).
    pub max_detection_depth: usize,
}

impl Default for DeadlockDetectorConfig {
    fn default() -> Self {
        Self {
            detection_interval: Duration::from_secs(1),
            victim_policy: VictimSelectionPolicy::Youngest,
            max_detection_depth: 1000,
        }
    }
}

/// Deadlock detector using wait-for graph.
///
/// Maintains a graph of which transactions are waiting for which,
/// and periodically checks for cycles indicating deadlocks.
///
/// # Thread Safety
///
/// All operations are thread-safe via internal locking.
///
/// # Performance
///
/// Detection runs are rate-limited by `detection_interval` to
/// avoid excessive overhead in high-contention scenarios.
pub struct DeadlockDetector {
    /// Wait-for graph: txn_id -> set of transactions it's waiting for.
    wait_for_graph: Arc<RwLock<HashMap<TransactionId, HashSet<TransactionId>>>>,
    /// Configuration.
    config: DeadlockDetectorConfig,
    /// Last detection time.
    last_detection: Arc<Mutex<SystemTime>>,
    /// Statistics.
    stats: Arc<Mutex<DeadlockStats>>,
}

/// Deadlock detection statistics.
#[derive(Debug, Default, Clone)]
pub struct DeadlockStats {
    /// Number of detection runs.
    pub detection_runs: u64,
    /// Number of deadlocks found.
    pub deadlocks_found: u64,
    /// Number of victims aborted.
    pub victims_aborted: u64,
    /// Maximum cycle length detected.
    pub max_cycle_length: usize,
}

impl DeadlockDetector {
    /// Creates a new deadlock detector with default settings.
    pub fn new(detection_interval: Duration) -> Self {
        Self {
            wait_for_graph: Arc::new(RwLock::new(HashMap::new())),
            config: DeadlockDetectorConfig {
                detection_interval,
                ..Default::default()
            },
            last_detection: Arc::new(Mutex::new(SystemTime::UNIX_EPOCH)),
            stats: Arc::new(Mutex::new(DeadlockStats::default())),
        }
    }

    /// Creates a deadlock detector with custom configuration.
    pub fn with_config(config: DeadlockDetectorConfig) -> Self {
        Self {
            wait_for_graph: Arc::new(RwLock::new(HashMap::new())),
            config,
            last_detection: Arc::new(Mutex::new(SystemTime::UNIX_EPOCH)),
            stats: Arc::new(Mutex::new(DeadlockStats::default())),
        }
    }

    /// Adds a wait edge: `waiting_txn` is waiting for `holding_txn`.
    ///
    /// # Arguments
    ///
    /// * `waiting_txn` - The transaction that is waiting.
    /// * `holding_txn` - The transaction that holds the resource.
    pub fn add_wait(&self, waiting_txn: TransactionId, holding_txn: TransactionId) {
        if waiting_txn == holding_txn {
            return; // Can't wait for yourself
        }
        let mut graph = self.wait_for_graph.write();
        graph
            .entry(waiting_txn)
            .or_default()
            .insert(holding_txn);
    }

    /// Removes all wait edges for a transaction.
    ///
    /// Called when a transaction commits, aborts, or acquires its lock.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - The transaction to remove.
    pub fn remove_wait(&self, txn_id: TransactionId) {
        let mut graph = self.wait_for_graph.write();
        graph.remove(&txn_id);

        // Also remove this transaction from others' wait sets
        for wait_set in graph.values_mut() {
            wait_set.remove(&txn_id);
        }
    }

    /// Removes a specific wait edge.
    ///
    /// # Arguments
    ///
    /// * `waiting_txn` - The waiting transaction.
    /// * `holding_txn` - The transaction being waited for.
    pub fn remove_wait_edge(&self, waiting_txn: TransactionId, holding_txn: TransactionId) {
        let mut graph = self.wait_for_graph.write();
        if let Some(wait_set) = graph.get_mut(&waiting_txn) {
            wait_set.remove(&holding_txn);
            if wait_set.is_empty() {
                graph.remove(&waiting_txn);
            }
        }
    }

    /// Detects deadlock by finding cycles in the wait-for graph.
    ///
    /// Rate-limited by `detection_interval` to avoid overhead.
    ///
    /// # Returns
    ///
    /// The cycle of transactions involved in the deadlock, or `None`.
    pub fn detect_deadlock(&self) -> Option<Vec<TransactionId>> {
        // Rate limiting
        let now = SystemTime::now();
        {
            let mut last = self.last_detection.lock();
            let elapsed = now.duration_since(*last).unwrap_or(Duration::ZERO);
            if elapsed < self.config.detection_interval {
                return None;
            }
            *last = now;
        }

        // Update statistics
        self.stats.lock().detection_runs += 1;

        let graph = self.wait_for_graph.read();

        // DFS to detect cycles
        for &txn_id in graph.keys() {
            let mut visited = HashSet::new();
            let mut path = Vec::new();

            if self.has_cycle(txn_id, &graph, &mut visited, &mut path, 0) {
                // Update statistics
                let mut stats = self.stats.lock();
                stats.deadlocks_found += 1;
                stats.max_cycle_length = stats.max_cycle_length.max(path.len());

                return Some(path);
            }
        }

        None
    }

    /// Forces immediate deadlock detection, ignoring rate limiting.
    pub fn force_detect(&self) -> Option<Vec<TransactionId>> {
        self.stats.lock().detection_runs += 1;
        let graph = self.wait_for_graph.read();

        for &txn_id in graph.keys() {
            let mut visited = HashSet::new();
            let mut path = Vec::new();

            if self.has_cycle(txn_id, &graph, &mut visited, &mut path, 0) {
                let mut stats = self.stats.lock();
                stats.deadlocks_found += 1;
                stats.max_cycle_length = stats.max_cycle_length.max(path.len());
                return Some(path);
            }
        }

        None
    }

    /// DFS helper to detect cycles.
    fn has_cycle(
        &self,
        txn_id: TransactionId,
        graph: &HashMap<TransactionId, HashSet<TransactionId>>,
        visited: &mut HashSet<TransactionId>,
        path: &mut Vec<TransactionId>,
        depth: usize,
    ) -> bool {
        // Prevent infinite loops
        if depth > self.config.max_detection_depth {
            return false;
        }

        // Cycle detected: txn_id is already in current path
        if path.contains(&txn_id) {
            path.push(txn_id);
            return true;
        }

        // Already visited in a previous DFS branch
        if visited.contains(&txn_id) {
            return false;
        }

        visited.insert(txn_id);
        path.push(txn_id);

        if let Some(waiting_for) = graph.get(&txn_id) {
            for &next_txn in waiting_for {
                if self.has_cycle(next_txn, graph, visited, path, depth + 1) {
                    return true;
                }
            }
        }

        path.pop();
        false
    }

    /// Selects a victim transaction to abort based on the configured policy.
    ///
    /// # Arguments
    ///
    /// * `cycle` - The cycle of transactions in the deadlock.
    ///
    /// # Returns
    ///
    /// The transaction ID of the selected victim.
    ///
    /// # Panics
    ///
    /// Panics if the cycle is empty.
    pub fn select_victim(&self, cycle: &[TransactionId]) -> TransactionId {
        assert!(!cycle.is_empty(), "Cycle cannot be empty");

        match self.config.victim_policy {
            VictimSelectionPolicy::Youngest => *cycle.iter().max().unwrap(),
            VictimSelectionPolicy::Oldest => *cycle.iter().min().unwrap(),
            // For LeastWork and LowestPriority, we'd need additional info.
            // Default to youngest if info not available.
            VictimSelectionPolicy::LeastWork => *cycle.iter().max().unwrap(),
            VictimSelectionPolicy::LowestPriority => *cycle.iter().max().unwrap(),
        }
    }

    /// Selects victim with custom work metrics.
    ///
    /// # Arguments
    ///
    /// * `cycle` - The cycle of transactions.
    /// * `work_done` - Map of transaction ID to work units done.
    ///
    /// # Returns
    ///
    /// The transaction with the least work done.
    pub fn select_victim_by_work(
        &self,
        cycle: &[TransactionId],
        work_done: &HashMap<TransactionId, usize>,
    ) -> TransactionId {
        cycle
            .iter()
            .min_by_key(|&&txn_id| work_done.get(&txn_id).copied().unwrap_or(0))
            .copied()
            .unwrap_or_else(|| cycle[0])
    }

    /// Records that a victim was aborted.
    pub fn record_victim_aborted(&self) {
        self.stats.lock().victims_aborted += 1;
    }

    /// Returns deadlock detection statistics.
    pub fn stats(&self) -> DeadlockStats {
        self.stats.lock().clone()
    }

    /// Returns the number of transactions in the wait-for graph.
    pub fn waiting_count(&self) -> usize {
        self.wait_for_graph.read().len()
    }

    /// Returns the total number of wait edges.
    pub fn edge_count(&self) -> usize {
        self.wait_for_graph
            .read()
            .values()
            .map(|s| s.len())
            .sum()
    }

    /// Clears the wait-for graph.
    pub fn clear(&self) {
        self.wait_for_graph.write().clear();
    }

    /// Checks if a specific transaction is waiting.
    pub fn is_waiting(&self, txn_id: TransactionId) -> bool {
        self.wait_for_graph.read().contains_key(&txn_id)
    }

    /// Gets the transactions that a given transaction is waiting for.
    pub fn get_waiting_for(&self, txn_id: TransactionId) -> HashSet<TransactionId> {
        self.wait_for_graph
            .read()
            .get(&txn_id)
            .cloned()
            .unwrap_or_default()
    }
}

impl Default for DeadlockDetector {
    fn default() -> Self {
        Self::new(Duration::from_secs(1))
    }
}

impl fmt::Debug for DeadlockDetector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeadlockDetector")
            .field("waiting_count", &self.waiting_count())
            .field("edge_count", &self.edge_count())
            .field("detection_interval", &self.config.detection_interval)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::UNIX_EPOCH;

    #[test]
    fn test_simple_cycle_detection() {
        let detector = DeadlockDetector::new(Duration::from_millis(0));

        // Create a simple cycle: 1 -> 2 -> 3 -> 1
        detector.add_wait(1, 2);
        detector.add_wait(2, 3);
        detector.add_wait(3, 1);

        let cycle = detector.force_detect();
        assert!(cycle.is_some());

        let cycle = cycle.unwrap();
        assert!(cycle.len() >= 3);
        assert!(cycle.contains(&1));
        assert!(cycle.contains(&2));
        assert!(cycle.contains(&3));
    }

    #[test]
    fn test_no_cycle() {
        let detector = DeadlockDetector::new(Duration::from_millis(0));

        // No cycle: 1 -> 2 -> 3
        detector.add_wait(1, 2);
        detector.add_wait(2, 3);

        let cycle = detector.force_detect();
        assert!(cycle.is_none());
    }

    #[test]
    fn test_self_loop_ignored() {
        let detector = DeadlockDetector::new(Duration::from_millis(0));

        // Self-loop should be ignored
        detector.add_wait(1, 1);

        assert_eq!(detector.waiting_count(), 0);
    }

    #[test]
    fn test_remove_wait() {
        let detector = DeadlockDetector::new(Duration::from_millis(0));

        detector.add_wait(1, 2);
        detector.add_wait(2, 3);

        assert_eq!(detector.edge_count(), 2);

        detector.remove_wait(1);

        assert_eq!(detector.edge_count(), 1);
        assert!(!detector.is_waiting(1));
    }

    #[test]
    fn test_victim_selection_youngest() {
        let detector = DeadlockDetector::new(Duration::from_millis(0));
        let cycle = vec![1, 5, 3, 2];

        let victim = detector.select_victim(&cycle);
        assert_eq!(victim, 5); // Youngest = highest ID
    }

    #[test]
    fn test_victim_selection_by_work() {
        let detector = DeadlockDetector::new(Duration::from_millis(0));
        let cycle = vec![1, 2, 3];
        let mut work = HashMap::new();
        work.insert(1, 100);
        work.insert(2, 10);  // Least work
        work.insert(3, 50);

        let victim = detector.select_victim_by_work(&cycle, &work);
        assert_eq!(victim, 2);
    }

    #[test]
    fn test_statistics() {
        let detector = DeadlockDetector::new(Duration::from_millis(0));

        detector.add_wait(1, 2);
        detector.add_wait(2, 1);

        let _ = detector.force_detect();

        let stats = detector.stats();
        assert_eq!(stats.detection_runs, 1);
        assert_eq!(stats.deadlocks_found, 1);
    }
}
