// Lock management for transaction concurrency control.
//
// This module implements various locking strategies for managing
// concurrent access to resources:
//
// - **Two-Phase Locking (2PL)**: Standard lock manager.
// - **Read-Write Locks**: Optimized for read-heavy workloads.
// - **Lock Escalation**: Automatic upgrade from row to table locks.
//
// # Example
//
// ```rust,ignore
// let lm = LockManager::new();
// lm.acquire_lock(txn_id, "table.row1", LockMode::Shared)?;
// // ... perform operations ...
// lm.release_all_locks(txn_id)?;
// ```

use std::fmt;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::collections::{HashMap};
use std::sync::Arc;
use std::time::SystemTime;

use parking_lot::RwLock;

use crate::common::TransactionId;

use super::error::{TransactionError, TransactionResult};
use super::types::LockMode;

/// Lock request in the wait queue.
#[derive(Debug, Clone)]
pub struct LockRequest {
    /// The requesting transaction.
    pub txn_id: TransactionId,
    /// The requested lock mode.
    pub mode: LockMode,
    /// When the request was made.
    pub timestamp: SystemTime,
}

impl LockRequest {
    /// Creates a new lock request.
    pub fn new(txn_id: TransactionId, mode: LockMode) -> Self {
        Self {
            txn_id,
            mode,
            timestamp: SystemTime::now(),
        }
    }
}

/// Entry in the lock table for a single resource.
#[derive(Debug, Clone)]
pub struct LockTableEntry {
    /// Current lock holders: (transaction ID, lock mode).
    pub holders: Vec<(TransactionId, LockMode)>,
    /// Queue of waiting requests.
    pub waiters: VecDeque<LockRequest>,
}

impl LockTableEntry {
    /// Creates a new empty lock table entry.
    pub fn new() -> Self {
        Self {
            holders: Vec::new(),
            waiters: VecDeque::new(),
        }
    }

    /// Checks if a new lock mode is compatible with current holders.
    pub fn is_compatible(&self, mode: &LockMode) -> bool {
        for (_, holder_mode) in &self.holders {
            if !mode.is_compatible(holder_mode) {
                return false;
            }
        }
        true
    }

    /// Checks if a transaction already holds a lock on this resource.
    pub fn is_held_by(&self, txn_id: TransactionId) -> Option<LockMode> {
        self.holders
            .iter()
            .find(|(id, _)| *id == txn_id)
            .map(|(_, mode)| *mode)
    }

    /// Returns true if the resource is unlocked.
    pub fn is_free(&self) -> bool {
        self.holders.is_empty()
    }
}

impl Default for LockTableEntry {
    fn default() -> Self {
        Self::new()
    }
}

/// Lock manager implementing two-phase locking (2PL).
///
/// Provides lock acquisition and release for transaction isolation.
/// Supports shared and exclusive locks with conflict detection.
///
/// # Thread Safety
///
/// All operations are thread-safe via internal RwLock.
///
/// # Two-Phase Locking
///
/// Transactions must follow 2PL protocol:
/// 1. Growing phase: Acquire locks, no releases.
/// 2. Shrinking phase: Release locks, no acquisitions.
pub struct LockManager {
    /// Lock table: resource -> lock holders.
    lock_table: Arc<RwLock<HashMap<String, Vec<(TransactionId, LockMode)>>>>,
    /// Transaction locks: txn_id -> set of held resources.
    txn_locks: Arc<RwLock<HashMap<TransactionId<String>>>>,
}

impl LockManager {
    /// Creates a new lock manager.
    pub fn new() -> Self {
        Self {
            lock_table: Arc::new(RwLock::new(HashMap::new())),
            txn_locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Acquires a lock on a resource.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - The transaction requesting the lock.
    /// * `resource` - The resource identifier.
    /// * `mode` - The lock mode to acquire.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the lock was granted, or an error if there's a conflict.
    ///
    /// # Errors
    ///
    /// Returns `TransactionError::LockConflict` if the lock cannot be granted.
    pub fn acquire_lock(
        &self,
        txn_id: TransactionId,
        resource: String,
        mode: LockMode,
    ) -> TransactionResult<()> {
        let mut lock_table = self.lock_table.write();
        let mut txn_locks = self.txn_locks.write();

        let holders = lock_table.entry(resource.clone()).or_default();

        // Check if already holding a lock
        if let Some(pos) = holders.iter().position(|(id, _)| *id == txn_id) {
            let current_mode = holders[pos].1;
            if mode.strength() <= current_mode.strength() {
                // Already have equal or stronger lock
                return Ok(());
            }
            // Need to upgrade
            if holders.len() == 1 {
                // Only holder, can upgrade
                holders[pos].1 = mode;
                return Ok(());
            }
            // Cannot upgrade while others hold locks
            let other_holder = holders.iter().find(|(id, _)| *id != txn_id);
            if let Some((other_id, other_mode)) = other_holder {
                return Err(TransactionError::lock_conflict(
                    txn_id,
                    *other_id,
                    resource,
                    mode,
                    *other_mode,
                ));
            }
        }

        // Check for conflicts with existing holders
        for &(holder_id, holder_mode) in holders.iter() {
            if holder_id != txn_id {
                // Check compatibility
                if mode == LockMode::Exclusive || holder_mode == LockMode::Exclusive {
                    return Err(TransactionError::lock_conflict(
                        txn_id,
                        holder_id,
                        resource,
                        mode,
                        holder_mode,
                    ));
                }
            }
        }

        // Grant lock
        holders.push((txn_id, mode));
        txn_locks.entry(txn_id).or_default().insert(resource);

        Ok(())
    }

    /// Attempts to acquire a lock without blocking.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if lock was granted, `Ok(false)` if would block.
    pub fn try_acquire_lock(
        &self,
        txn_id: TransactionId,
        resource: String,
        mode: LockMode,
    ) -> TransactionResult<bool> {
        match self.acquire_lock(txn_id, resource, mode) {
            Ok(()) => Ok(true),
            Err(TransactionError::LockConflict { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Releases a specific lock.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - The transaction releasing the lock.
    /// * `resource` - The resource to unlock.
    pub fn release_lock(&self, txn_id: TransactionId, resource: &str) -> TransactionResult<()> {
        let mut lock_table = self.lock_table.write();
        let mut txn_locks = self.txn_locks.write();

        if let Some(holders) = lock_table.get_mut(resource) {
            holders.retain(|(id, _)| *id != txn_id);
            if holders.is_empty() {
                lock_table.remove(resource);
            }
        }

        if let Some(locks) = txn_locks.get_mut(&txn_id) {
            locks.remove(resource);
        }

        Ok(())
    }

    /// Releases all locks held by a transaction.
    ///
    /// Called when a transaction commits or aborts.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - The transaction to release locks for.
    pub fn release_all_locks(&self, txn_id: TransactionId) -> TransactionResult<()> {
        // Get all locks for this transaction
        let resources: Vec<String> = {
            let txn_locks = self.txn_locks.read();
            match txn_locks.get(&txn_id) {
                Some(locks) => locks.iter().cloned().collect(),
                None => return Ok(()),
            }
        };

        // Release each lock
        for resource in resources {
            self.release_lock(txn_id, &resource)?;
        }

        // Remove transaction entry
        self.txn_locks.write().remove(&txn_id);

        Ok(())
    }

    /// Returns the locks held by a transaction.
    pub fn get_locks(&self, txn_id: TransactionId) -> HashSet<String> {
        self.txn_locks
            .read()
            .get(&txn_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Returns the number of locks held by a transaction.
    pub fn lock_count(&self, txn_id: TransactionId) -> usize {
        self.txn_locks
            .read()
            .get(&txn_id)
            .map(|s| s.len())
            .unwrap_or(0)
    }

    /// Returns total number of locked resources.
    pub fn total_locked_resources(&self) -> usize {
        self.lock_table.read().len()
    }

    /// Checks if a resource is locked.
    pub fn is_locked(&self, resource: &str) -> bool {
        self.lock_table.read().contains_key(resource)
    }

    /// Gets the holders of a lock on a resource.
    pub fn get_holders(&self, resource: &str) -> Vec<(TransactionId, LockMode)> {
        self.lock_table
            .read()
            .get(resource)
            .cloned()
            .unwrap_or_default()
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for LockManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LockManager")
            .field("total_resources", &self.total_locked_resources())
            .finish()
    }
}

/// Read-write lock manager for optimized concurrency.
///
/// Provides fair read-write locks with writer preference to prevent starvation.
pub struct ReadWriteLockManager {
    /// Lock state for each resource.
    locks: Arc<RwLock<HashMap<String, RWLockState>>>,
}

/// State of a read-write lock.
#[derive(Debug, Clone)]
struct RWLockState {
    /// Set of readers.
    readers: HashSet<TransactionId>,
    /// Current writer (if any).
    writer: Option<TransactionId>,
    /// Queue of waiting writers (for fairness).
    waiting_writers: VecDeque<TransactionId>,
}

impl RWLockState {
    fn new() -> Self {
        Self {
            readers: HashSet::new(),
            writer: None,
            waiting_writers: VecDeque::new(),
        }
    }
}

impl ReadWriteLockManager {
    /// Creates a new read-write lock manager.
    pub fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Acquires a read lock.
    ///
    /// Read locks are granted if:
    /// - No writer holds the lock.
    /// - No writers are waiting (to prevent writer starvation).
    pub fn acquire_read_lock(
        &self,
        txn_id: TransactionId,
        resource: String,
    ) -> TransactionResult<()> {
        let mut locks = self.locks.write();
        let lock = locks.entry(resource.clone()).or_insert_with(RWLockState::new);

        // Can acquire read if no writer and no waiting writers
        if lock.writer.is_none() && lock.waiting_writers.is_empty() {
            lock.readers.insert(txn_id);
            Ok(())
        } else {
            Err(TransactionError::lock_timeout(txn_id, resource, LockMode::Shared))
        }
    }

    /// Acquires a write lock.
    ///
    /// Write locks are granted if:
    /// - No readers hold the lock.
    /// - No other writer holds the lock.
    pub fn acquire_write_lock(
        &self,
        txn_id: TransactionId,
        resource: String,
    ) -> TransactionResult<()> {
        let mut locks = self.locks.write();
        let lock = locks.entry(resource.clone()).or_insert_with(RWLockState::new);

        if lock.writer.is_none() && lock.readers.is_empty() {
            lock.writer = Some(txn_id);
            Ok(())
        } else {
            // Add to waiting queue
            if !lock.waiting_writers.contains(&txn_id) {
                lock.waiting_writers.push_back(txn_id);
            }
            Err(TransactionError::lock_timeout(txn_id, resource, LockMode::Exclusive))
        }
    }

    /// Releases a read lock.
    pub fn release_read_lock(&self, txn_id: TransactionId, resource: &str) {
        let mut locks = self.locks.write();

        if let Some(lock) = locks.get_mut(resource) {
            lock.readers.remove(&txn_id);

            // Grant waiting writer if no more readers
            if lock.readers.is_empty() && !lock.waiting_writers.is_empty() {
                if let Some(waiting_writer) = lock.waiting_writers.pop_front() {
                    lock.writer = Some(waiting_writer);
                }
            }
        }
    }

    /// Releases a write lock.
    pub fn release_write_lock(&self, txn_id: TransactionId, resource: &str) {
        let mut locks = self.locks.write();

        if let Some(lock) = locks.get_mut(resource) {
            if lock.writer == Some(txn_id) {
                lock.writer = None;

                // Grant next waiting writer
                if !lock.waiting_writers.is_empty() {
                    if let Some(waiting_writer) = lock.waiting_writers.pop_front() {
                        lock.writer = Some(waiting_writer);
                    }
                }
            }
        }
    }

    /// Releases all locks held by a transaction.
    pub fn release_all(&self, txn_id: TransactionId) {
        let mut locks = self.locks.write();

        for (_, lock) in locks.iter_mut() {
            lock.readers.remove(&txn_id);
            if lock.writer == Some(txn_id) {
                lock.writer = None;
            }
            lock.waiting_writers.retain(|&id| id != txn_id);
        }
    }
}

impl Default for ReadWriteLockManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Lock escalation manager.
///
/// Automatically escalates row locks to table locks when a threshold
/// is exceeded to reduce lock overhead.
pub struct LockEscalationManager {
    /// Threshold for escalation (number of row locks before escalating).
    escalation_threshold: usize,
    /// Count of row locks per (transaction, table).
    row_lock_count: Arc<RwLock<HashMap<(TransactionId, String), usize>>>,
}

impl LockEscalationManager {
    /// Creates a new escalation manager.
    ///
    /// # Arguments
    ///
    /// * `escalation_threshold` - Number of row locks before escalating.
    pub fn new(escalation_threshold: usize) -> Self {
        Self {
            escalation_threshold,
            row_lock_count: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Records a row lock and returns whether escalation should occur.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - The transaction.
    /// * `table` - The table name.
    ///
    /// # Returns
    ///
    /// `true` if the threshold has been reached and escalation should occur.
    pub fn record_row_lock(&self, txn_id: TransactionId, table: String) -> bool {
        let mut counts = self.row_lock_count.write();
        let count = counts.entry((txn_id, table)).or_insert(0);
        *count += 1;
        *count >= self.escalation_threshold
    }

    /// Checks if escalation should occur for a transaction/table pair.
    pub fn should_escalate(&self, txn_id: TransactionId, table: &str) -> bool {
        let counts = self.row_lock_count.read();
        counts
            .get(&(txn_id, table.to_string()))
            .map(|&c| c >= self.escalation_threshold)
            .unwrap_or(false)
    }

    /// Clears lock counts for a transaction.
    pub fn clear_locks(&self, txn_id: TransactionId) {
        let mut counts = self.row_lock_count.write();
        counts.retain(|(tid, _), _| *tid != txn_id);
    }

    /// Returns the current row lock count for a transaction/table pair.
    pub fn get_count(&self, txn_id: TransactionId, table: &str) -> usize {
        self.row_lock_count
            .read()
            .get(&(txn_id, table.to_string()))
            .copied()
            .unwrap_or(0)
    }

    /// Returns the escalation threshold.
    pub fn threshold(&self) -> usize {
        self.escalation_threshold
    }
}

impl Default for LockEscalationManager {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_manager_basic() {
        let lm = LockManager::new();

        // Acquire shared lock
        assert!(lm.acquire_lock(1, "r1".to_string(), LockMode::Shared).is_ok());

        // Another transaction can also acquire shared
        assert!(lm.acquire_lock(2, "r1".to_string(), LockMode::Shared).is_ok());

        // Verify lock counts
        assert_eq!(lm.lock_count(1), 1);
        assert_eq!(lm.lock_count(2), 1);
    }

    #[test]
    fn test_lock_conflict() {
        let lm = LockManager::new();

        // Acquire exclusive lock
        assert!(lm.acquire_lock(1, "r1".to_string(), LockMode::Exclusive).is_ok());

        // Another transaction cannot acquire
        let result = lm.acquire_lock(2, "r1".to_string(), LockMode::Shared);
        assert!(result.is_err());
    }

    #[test]
    fn test_release_all_locks() {
        let lm = LockManager::new();

        lm.acquire_lock(1, "r1".to_string(), LockMode::Shared).unwrap();
        lm.acquire_lock(1, "r2".to_string(), LockMode::Shared).unwrap();

        assert_eq!(lm.lock_count(1), 2);

        lm.release_all_locks(1).unwrap();

        assert_eq!(lm.lock_count(1), 0);
    }

    #[test]
    fn test_lock_upgrade() {
        let lm = LockManager::new();

        // Acquire shared, then upgrade to exclusive
        lm.acquire_lock(1, "r1".to_string(), LockMode::Shared).unwrap();

        // When alone, upgrade should succeed
        assert!(lm.acquire_lock(1, "r1".to_string(), LockMode::Exclusive).is_ok());
    }

    #[test]
    fn test_escalation_manager() {
        let em = LockEscalationManager::new(5);

        for _i in 0..4 {
            assert!(!em.record_row_lock(1, "table1".to_string()));
        }

        // 5th lock should trigger escalation
        assert!(em.record_row_lock(1, "table1".to_string()));
    }

    #[test]
    fn test_rw_lock_manager() {
        let rwlm = ReadWriteLockManager::new();

        // Multiple readers OK
        assert!(rwlm.acquire_read_lock(1, "r1".to_string()).is_ok());
        assert!(rwlm.acquire_read_lock(2, "r1".to_string()).is_ok());

        // Writer blocked while readers exist
        assert!(rwlm.acquire_write_lock(3, "r1".to_string()).is_err());

        // Release readers
        rwlm.release_read_lock(1, "r1");
        rwlm.release_read_lock(2, "r1");
    }
}
