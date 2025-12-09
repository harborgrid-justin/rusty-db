// Transaction manager for coordinating transaction lifecycle.
//
// This module provides the core transaction management functionality,
// including beginning, committing, and aborting transactions.
//
// # Example
//
// ```rust,ignore
// let manager = TransactionManager::new();
// let txn_id = manager.begin()?;
// // ... perform operations ...
// manager.commit(txn_id)?;
// ```

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};

use crate::common::TransactionId;

use super::error::{TransactionError, TransactionResult};
use super::lock_manager::LockManager;
use super::types::{IsolationLevel, Transaction, TransactionState};

/// Transaction manager for lifecycle management.
///
/// Coordinates transaction begin, commit, and abort operations.
/// Integrates with the lock manager for 2PL enforcement.
///
/// # Thread Safety
///
/// All operations are thread-safe via internal locking.
pub struct TransactionManager {
    /// Next transaction ID to assign.
    next_txn_id: Arc<Mutex<TransactionId>>,
    /// Map of active transactions.
    active_txns: Arc<RwLock<HashMap<TransactionId, Transaction>>>,
    /// Lock manager for 2PL.
    lock_manager: Arc<LockManager>,
    /// Default isolation level for new transactions.
    default_isolation: IsolationLevel,
}

impl TransactionManager {
    /// Creates a new transaction manager.
    pub fn new() -> Self {
        Self {
            next_txn_id: Arc::new(Mutex::new(1)),
            active_txns: Arc::new(RwLock::new(HashMap::new())),
            lock_manager: Arc::new(LockManager::new()),
            default_isolation: IsolationLevel::ReadCommitted,
        }
    }

    /// Creates a transaction manager with a shared lock manager.
    pub fn with_lock_manager(lock_manager: Arc<LockManager>) -> Self {
        Self {
            next_txn_id: Arc::new(Mutex::new(1)),
            active_txns: Arc::new(RwLock::new(HashMap::new())),
            lock_manager,
            default_isolation: IsolationLevel::ReadCommitted,
        }
    }

    /// Creates a transaction manager with custom default isolation.
    pub fn with_isolation(default_isolation: IsolationLevel) -> Self {
        Self {
            next_txn_id: Arc::new(Mutex::new(1)),
            active_txns: Arc::new(RwLock::new(HashMap::new())),
            lock_manager: Arc::new(LockManager::new()),
            default_isolation,
        }
    }

    /// Begins a new transaction with default isolation level.
    ///
    /// # Returns
    ///
    /// The ID of the newly created transaction.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let txn_id = manager.begin()?;
    /// ```
    pub fn begin(&self) -> TransactionResult<TransactionId> {
        self.begin_with_isolation(self.default_isolation)
    }

    /// Begins a new transaction with specified isolation level.
    ///
    /// # Arguments
    ///
    /// * `isolation_level` - The isolation level for this transaction.
    ///
    /// # Returns
    ///
    /// The ID of the newly created transaction.
    pub fn begin_with_isolation(
        &self,
        isolation_level: IsolationLevel,
    ) -> TransactionResult<TransactionId> {
        let txn_id = {
            let mut next_id = self.next_txn_id.lock();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let txn = Transaction::new(txn_id, isolation_level);
        self.active_txns.write().insert(txn_id, txn);

        Ok(txn_id)
    }

    /// Begins a read-only transaction.
    ///
    /// Read-only transactions can have better performance and
    /// never need to acquire exclusive locks.
    pub fn begin_readonly(&self) -> TransactionResult<TransactionId> {
        let txn_id = {
            let mut next_id = self.next_txn_id.lock();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let txn = Transaction::new_readonly(txn_id, self.default_isolation);
        self.active_txns.write().insert(txn_id, txn);

        Ok(txn_id)
    }

    /// Commits a transaction.
    ///
    /// Releases all locks and marks the transaction as committed.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - The transaction to commit.
    ///
    /// # Errors
    ///
    /// Returns `TransactionError::TransactionNotFound` if the transaction
    /// doesn't exist.
    pub fn commit(&self, txn_id: TransactionId) -> TransactionResult<()> {
        // Update transaction state
        {
            let mut active_txns = self.active_txns.write();

            let txn = active_txns
                .get_mut(&txn_id)
                .ok_or_else(|| TransactionError::not_found(txn_id))?;

            if txn.state == TransactionState::Committed {
                return Err(TransactionError::AlreadyCommitted(txn_id));
            }
            if txn.state == TransactionState::Aborted {
                return Err(TransactionError::AlreadyAborted(txn_id));
            }

            txn.state = TransactionState::Committing;
        }

        // Release all locks
        self.lock_manager.release_all_locks(txn_id)?;

        // Finalize commit
        {
            let mut active_txns = self.active_txns.write();
            if let Some(txn) = active_txns.get_mut(&txn_id) {
                txn.state = TransactionState::Committed;
            }
            active_txns.remove(&txn_id);
        }

        Ok(())
    }

    /// Aborts a transaction.
    ///
    /// Releases all locks and marks the transaction as aborted.
    /// Any changes made by the transaction should be rolled back.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - The transaction to abort.
    ///
    /// # Errors
    ///
    /// Returns `TransactionError::TransactionNotFound` if the transaction
    /// doesn't exist.
    pub fn abort(&self, txn_id: TransactionId) -> TransactionResult<()> {
        // Update transaction state
        {
            let mut active_txns = self.active_txns.write();

            let txn = active_txns
                .get_mut(&txn_id)
                .ok_or_else(|| TransactionError::not_found(txn_id))?;

            if txn.state == TransactionState::Committed {
                return Err(TransactionError::AlreadyCommitted(txn_id));
            }
            if txn.state == TransactionState::Aborted {
                return Err(TransactionError::AlreadyAborted(txn_id));
            }

            txn.state = TransactionState::Aborting;
        }

        // Release all locks
        self.lock_manager.release_all_locks(txn_id)?;

        // Finalize abort
        {
            let mut active_txns = self.active_txns.write();
            if let Some(txn) = active_txns.get_mut(&txn_id) {
                txn.state = TransactionState::Aborted;
            }
            active_txns.remove(&txn_id);
        }

        Ok(())
    }

    /// Gets a reference to the lock manager.
    pub fn get_lock_manager(&self) -> Arc<LockManager> {
        Arc::clone(&self.lock_manager)
    }

    /// Gets a clone of a transaction's metadata.
    pub fn get_transaction(&self, txn_id: TransactionId) -> Option<Transaction> {
        self.active_txns.read().get(&txn_id).cloned()
    }

    /// Gets the state of a transaction.
    pub fn get_state(&self, txn_id: TransactionId) -> Option<TransactionState> {
        self.active_txns.read().get(&txn_id).map(|t| t.state)
    }

    /// Checks if a transaction is active.
    pub fn is_active(&self, txn_id: TransactionId) -> bool {
        self.active_txns.read().contains_key(&txn_id)
    }

    /// Returns the number of active transactions.
    pub fn active_count(&self) -> usize {
        self.active_txns.read().len()
    }

    /// Returns the IDs of all active transactions.
    pub fn active_transaction_ids(&self) -> Vec<TransactionId> {
        self.active_txns.read().keys().copied().collect()
    }

    /// Gets the minimum active transaction ID.
    ///
    /// Useful for garbage collection of old versions.
    pub fn min_active_txn(&self) -> Option<TransactionId> {
        self.active_txns.read().keys().min().copied()
    }

    /// Updates the activity timestamp for a transaction.
    pub fn touch(&self, txn_id: TransactionId) {
        if let Some(txn) = self.active_txns.write().get_mut(&txn_id) {
            txn.update_activity();
        }
    }

    /// Records a read operation for a transaction.
    pub fn record_read(&self, txn_id: TransactionId, key: String) {
        if let Some(txn) = self.active_txns.write().get_mut(&txn_id) {
            txn.read_set.insert(key);
            txn.update_activity();
        }
    }

    /// Records a write operation for a transaction.
    pub fn record_write(&self, txn_id: TransactionId, key: String) {
        if let Some(txn) = self.active_txns.write().get_mut(&txn_id) {
            txn.write_set.insert(key);
            txn.update_activity();
        }
    }

    /// Gets the read set for a transaction.
    pub fn get_read_set(&self, txn_id: TransactionId) -> Vec<String> {
        self.active_txns
            .read()
            .get(&txn_id)
            .map(|t| t.read_set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Gets the write set for a transaction.
    pub fn get_write_set(&self, txn_id: TransactionId) -> Vec<String> {
        self.active_txns
            .read()
            .get(&txn_id)
            .map(|t| t.write_set.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for TransactionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransactionManager")
            .field("active_count", &self.active_count())
            .field("next_txn_id", &*self.next_txn_id.lock())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_begin_transaction() {
        let tm = TransactionManager::new();

        let txn_id = tm.begin().unwrap();
        assert_eq!(txn_id, 1);

        let txn_id2 = tm.begin().unwrap();
        assert_eq!(txn_id2, 2);
    }

    #[test]
    fn test_commit_transaction() {
        let tm = TransactionManager::new();

        let txn_id = tm.begin().unwrap();
        assert!(tm.is_active(txn_id));

        tm.commit(txn_id).unwrap();
        assert!(!tm.is_active(txn_id));
    }

    #[test]
    fn test_abort_transaction() {
        let tm = TransactionManager::new();

        let txn_id = tm.begin().unwrap();
        tm.abort(txn_id).unwrap();

        assert!(!tm.is_active(txn_id));
    }

    #[test]
    fn test_transaction_not_found() {
        let tm = TransactionManager::new();

        let _result = tm.commit(999);
        assert!(matches!(result, Err(TransactionError::TransactionNotFound(999))));
    }

    #[test]
    fn test_double_commit() {
        let tm = TransactionManager::new();

        let txn_id = tm.begin().unwrap();
        tm.commit(txn_id).unwrap();

        // Transaction is removed after commit, so it's not found
        let _result = tm.commit(txn_id);
        assert!(matches!(result, Err(TransactionError::TransactionNotFound(_))));
    }

    #[test]
    fn test_active_count() {
        let tm = TransactionManager::new();

        assert_eq!(tm.active_count(), 0);

        let txn1 = tm.begin().unwrap();
        let txn2 = tm.begin().unwrap();
        assert_eq!(tm.active_count(), 2);

        tm.commit(txn1).unwrap();
        assert_eq!(tm.active_count(), 1);

        tm.abort(txn2).unwrap();
        assert_eq!(tm.active_count(), 0);
    }

    #[test]
    fn test_read_write_sets() {
        let tm = TransactionManager::new();

        let txn_id = tm.begin().unwrap();

        tm.record_read(txn_id, "key1".to_string());
        tm.record_write(txn_id, "key2".to_string());

        let reads = tm.get_read_set(txn_id);
        let writes = tm.get_write_set(txn_id);

        assert!(reads.contains(&"key1".to_string()));
        assert!(writes.contains(&"key2".to_string()));
    }

    #[test]
    fn test_min_active_txn() {
        let tm = TransactionManager::new();

        assert!(tm.min_active_txn().is_none());

        let _txn1 = tm.begin().unwrap();
        let _txn2 = tm.begin().unwrap();
        let _txn3 = tm.begin().unwrap();

        assert_eq!(tm.min_active_txn(), Some(1));
    }
}
