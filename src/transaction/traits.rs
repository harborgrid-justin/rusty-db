// Transaction traits for extensibility.
//
// This module defines traits that enable trait-driven design,
// allowing for testability and alternative implementations.

use std::time::Duration;

use crate::common::TransactionId;

use super::error::TransactionResult;
use super::types::{IsolationLevel, LockMode, TransactionState};

/// Trait for transaction lifecycle management.
///
/// Implement this trait to create custom transaction managers
/// with different storage backends or behaviors.
pub trait TransactionLifecycle: Send + Sync {
    /// Begins a new transaction.
    fn begin(&self) -> TransactionResult<TransactionId>;

    /// Begins a transaction with specific isolation level.
    fn begin_with_isolation(&self, level: IsolationLevel) -> TransactionResult<TransactionId>;

    /// Commits a transaction.
    fn commit(&self, txn_id: TransactionId) -> TransactionResult<()>;

    /// Aborts a transaction.
    fn abort(&self, txn_id: TransactionId) -> TransactionResult<()>;

    /// Gets the state of a transaction.
    fn get_state(&self, txn_id: TransactionId) -> Option<TransactionState>;

    /// Checks if a transaction is active.
    fn is_active(&self, txn_id: TransactionId) -> bool;
}

/// Trait for lock management.
///
/// Implement this trait to create custom lock managers
/// with different locking strategies.
pub trait LockManagement: Send + Sync {
    /// Acquires a lock on a resource.
    fn acquire_lock(
        &self,
        txn_id: TransactionId,
        resource: String,
        mode: LockMode,
    ) -> TransactionResult<()>;

    /// Releases a lock on a resource.
    fn release_lock(&self, txn_id: TransactionId, resource: &str) -> TransactionResult<()>;

    /// Releases all locks for a transaction.
    fn release_all_locks(&self, txn_id: TransactionId) -> TransactionResult<()>;

    /// Checks if a resource is locked.
    fn is_locked(&self, resource: &str) -> bool;

    /// Gets the lock mode held by a transaction on a resource.
    fn get_lock_mode(&self, txn_id: TransactionId, resource: &str) -> Option<LockMode>;
}

/// Trait for deadlock detection.
pub trait DeadlockDetection: Send + Sync {
    /// Adds a wait edge to the wait-for graph.
    fn add_wait(&self, waiting: TransactionId, holding: TransactionId);

    /// Removes wait edges for a transaction.
    fn remove_wait(&self, txn_id: TransactionId);

    /// Detects deadlock and returns cycle if found.
    fn detect(&self) -> Option<Vec<TransactionId>>;

    /// Selects a victim from a deadlock cycle.
    fn select_victim(&self, cycle: &[TransactionId]) -> TransactionId;
}

/// Trait for WAL operations.
pub trait WriteAheadLog: Send + Sync {
    /// Appends an entry to the log.
    fn append(&self, data: &[u8]) -> TransactionResult<u64>;

    /// Flushes buffered entries to disk.
    fn flush(&self) -> TransactionResult<()>;

    /// Gets the current LSN.
    fn current_lsn(&self) -> u64;
}

/// Trait for snapshot management.
pub trait SnapshotManagement: Send + Sync {
    /// Creates a snapshot for a transaction.
    fn create_snapshot(&self, txn_id: TransactionId);

    /// Checks if a version is visible to a transaction.
    fn is_visible(&self, txn_id: TransactionId, versiontxn_id: TransactionId) -> bool;

    /// Removes a snapshot.
    fn remove_snapshot(&self, txn_id: TransactionId);
}

/// Trait for transaction event listeners.
///
/// Implement this trait to receive notifications about
/// transaction lifecycle events.
pub trait TransactionEventListener: Send + Sync {
    /// Called when a transaction begins.
    fn on_begin(&self, txn_id: TransactionId);

    /// Called when a transaction commits.
    fn on_commit(&self, txn_id: TransactionId);

    /// Called when a transaction aborts.
    fn on_abort(&self, txn_id: TransactionId);

    /// Called when a deadlock is detected.
    fn on_deadlock(&self, cycle: &[TransactionId]);
}

/// Trait for recovery operations.
pub trait Recovery: Send + Sync {
    /// Performs crash recovery.
    fn recover(&self) -> TransactionResult<()>;

    /// Creates a checkpoint.
    fn checkpoint(&self, active_txns: Vec<TransactionId>) -> TransactionResult<u64>;
}

/// Trait for transaction validation (OCC).
pub trait Validation: Send + Sync {
    /// Records a read operation.
    fn record_read(&self, txn_id: TransactionId, key: &str);

    /// Records a write operation.
    fn record_write(&self, txn_id: TransactionId, key: &str);

    /// Validates a transaction before commit.
    fn validate(&self, txn_id: TransactionId) -> bool;
}

/// Trait for timeout management.
pub trait TimeoutManagement: Send + Sync {
    /// Sets a timeout for a transaction.
    fn set_timeout(&self, txn_id: TransactionId, duration: Duration);

    /// Checks if a transaction has timed out.
    fn is_timed_out(&self, txn_id: TransactionId) -> bool;

    /// Clears a timeout.
    fn clear_timeout(&self, txn_id: TransactionId);

    /// Gets all timed-out transactions.
    fn get_timed_out(&self) -> Vec<TransactionId>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct MockTransactionManager {
        next_id: AtomicU64,
    }

    impl MockTransactionManager {
        fn new() -> Self {
            Self {
                next_id: AtomicU64::new(1),
            }
        }
    }

    impl TransactionLifecycle for MockTransactionManager {
        fn begin(&self) -> TransactionResult<TransactionId> {
            Ok(self.next_id.fetch_add(1, Ordering::SeqCst))
        }

        fn begin_with_isolation(&self, _level: IsolationLevel) -> TransactionResult<TransactionId> {
            self.begin()
        }

        fn commit(&self, _txn_id: TransactionId) -> TransactionResult<()> {
            Ok(())
        }

        fn abort(&self, _txn_id: TransactionId) -> TransactionResult<()> {
            Ok(())
        }

        fn get_state(&self, _txn_id: TransactionId) -> Option<TransactionState> {
            Some(TransactionState::Active)
        }

        fn is_active(&self, _txn_id: TransactionId) -> bool {
            true
        }
    }

    #[test]
    fn test_mock_transaction_manager() {
        let tm = MockTransactionManager::new();

        let id1 = tm.begin().unwrap();
        let id2 = tm.begin().unwrap();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert!(tm.is_active(id1));
    }
}
