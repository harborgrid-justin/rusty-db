//! Snapshot management for snapshot isolation.
//!
//! This module provides snapshot creation and management for
//! implementing snapshot isolation (SI) in MVCC.
//!
//! # Example
//!
//! ```rust,ignore
//! let mgr = SnapshotManager::new();
//! let snapshot = mgr.create_snapshot(txn_id, active_txns);
//! if mgr.is_visible(&snapshot, other_txn_id) {
//!     // Other transaction's writes are visible
//! }
//! ```

use std::collections::{BTreeMap};
use std::sync::Arc;
use std::time::SystemTime;

use parking_lot::{Mutex, RwLock};

use crate::common::TransactionId;

/// A point-in-time snapshot for consistent reads.
///
/// Contains information about which transactions were active
/// when the snapshot was taken, enabling visibility decisions.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Unique snapshot identifier.
    pub id: u64,
    /// The transaction that owns this snapshot.
    pub txn_id: TransactionId,
    /// When the snapshot was created.
    pub timestamp: SystemTime,
    /// Set of transactions that were active when snapshot was taken.
    pub active_txns: HashSet<TransactionId>,
    /// The minimum transaction ID visible to this snapshot.
    pub min_txn_id: TransactionId,
    /// The maximum transaction ID when snapshot was taken.
    pub max_txn_id: TransactionId,
}

impl Snapshot {
    /// Creates a new snapshot.
    pub fn new(
        id: u64,
        txn_id: TransactionId,
        active_txns: HashSet<TransactionId>,
    ) -> Self {
        let min_txn_id = *active_txns.iter().min().unwrap_or(&0);
        let max_txn_id = *active_txns.iter().max().unwrap_or(&txn_id);

        Self {
            id,
            txn_id,
            timestamp: SystemTime::now(),
            active_txns,
            min_txn_id,
            max_txn_id,
        }
    }

    /// Checks if a transaction's changes are visible in this snapshot.
    ///
    /// A transaction's changes are visible if:
    /// 1. It's the snapshot's own transaction (read-your-writes).
    /// 2. It committed before the snapshot was taken AND was not active
    ///    when the snapshot was created.
    pub fn is_visible(&self, txn_id: TransactionId) -> bool {
        // Read your own writes
        if txn_id == self.txn_id {
            return true;
        }

        // Not visible if transaction was active when snapshot was taken
        if self.active_txns.contains(&txn_id) {
            return false;
        }

        // Visible if transaction committed before snapshot
        // (i.e., has a lower ID and wasn't in the active set)
        txn_id < self.txn_id
    }
}

/// Manager for transaction snapshots.
///
/// Creates and tracks snapshots for snapshot isolation.
/// Thread-safe for concurrent access.
pub struct SnapshotManager {
    /// Active snapshots: txn_id -> snapshot.
    snapshots: Arc<RwLock<BTreeMap<TransactionId, Snapshot>>>,
    /// Next snapshot ID.
    next_snapshot_id: Arc<Mutex<u64>>,
}

impl SnapshotManager {
    /// Creates a new snapshot manager.
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(BTreeMap::new())),
            next_snapshot_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Creates a snapshot for a transaction.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - The transaction creating the snapshot.
    /// * `active_txns` - Set of currently active transactions.
    ///
    /// # Returns
    ///
    /// The newly created snapshot.
    pub fn create_snapshot(
        &self,
        txn_id: TransactionId,
        active_txns: HashSet<TransactionId>,
    ) -> Snapshot {
        let id = {
            let mut next = self.next_snapshot_id.lock();
            let id = *next;
            *next += 1;
            id
        };

        let snapshot = Snapshot::new(id, txn_id, active_txns);
        self.snapshots.write().insert(txn_id, snapshot.clone());
        snapshot
    }

    /// Gets a snapshot for a transaction.
    pub fn get_snapshot(&self, txn_id: TransactionId) -> Option<Snapshot> {
        self.snapshots.read().get(&txn_id).cloned()
    }

    /// Removes a snapshot when transaction completes.
    pub fn remove_snapshot(&self, txn_id: TransactionId) {
        self.snapshots.write().remove(&txn_id);
    }

    /// Checks if a transaction's changes are visible in a snapshot.
    ///
    /// # Arguments
    ///
    /// * `snapshot` - The snapshot to check against.
    /// * `txn_id` - The transaction whose visibility to check.
    pub fn is_visible(&self, snapshot: &Snapshot, txn_id: TransactionId) -> bool {
        snapshot.is_visible(txn_id)
    }

    /// Returns the number of active snapshots.
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.read().len()
    }

    /// Gets the oldest active snapshot's transaction ID.
    ///
    /// Useful for garbage collection: versions older than this
    /// may still be needed.
    pub fn oldest_snapshot_txn(&self) -> Option<TransactionId> {
        self.snapshots.read().keys().next().copied()
    }

    /// Clears all snapshots.
    pub fn clear(&self) {
        self.snapshots.write().clear();
    }
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_visibility_own_writes() {
        let active = HashSet::new();
        let snapshot = Snapshot::new(1, 10, active);

        // Can see own writes
        assert!(snapshot.is_visible(10));
    }

    #[test]
    fn test_snapshot_visibility_older_committed() {
        let active = HashSet::new();
        let snapshot = Snapshot::new(1, 10, active);

        // Can see older committed transactions
        assert!(snapshot.is_visible(5));
        assert!(snapshot.is_visible(9));
    }

    #[test]
    fn test_snapshot_visibility_active_not_visible() {
        let mut active = HashSet::new();
        active.insert(5);
        active.insert(7);

        let snapshot = Snapshot::new(1, 10, active);

        // Cannot see transactions that were active
        assert!(!snapshot.is_visible(5));
        assert!(!snapshot.is_visible(7));
    }

    #[test]
    fn test_snapshot_manager_create_get() {
        let mgr = SnapshotManager::new();
        let active = HashSet::new();

        let snapshot = mgr.create_snapshot(1, active);
        assert_eq!(snapshot.txn_id, 1);

        let retrieved = mgr.get_snapshot(1);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().txn_id, 1);
    }

    #[test]
    fn test_snapshot_manager_remove() {
        let mgr = SnapshotManager::new();
        let active = HashSet::new();

        mgr.create_snapshot(1, active);
        assert_eq!(mgr.snapshot_count(), 1);

        mgr.remove_snapshot(1);
        assert_eq!(mgr.snapshot_count(), 0);
    }
}
