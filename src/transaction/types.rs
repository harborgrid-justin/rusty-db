// Core transaction types and domain models.
//
// This module defines the fundamental types used throughout the transaction
// management system. Each type is designed to be:
// - Strongly typed for safety
// - Serializable for persistence
// - Well-documented for clarity
//
// # Example
//
// ```rust
// use crate::transaction::types::{Transaction, IsolationLevel, TransactionState};
//
// let txn = Transaction::new(1, IsolationLevel::ReadCommitted);
// assert_eq!(txn.state, TransactionState::Active);
// ```

use std::collections::HashSet;
use std::fmt;
use std::mem::size_of;
use std::time::Duration;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::common::{LogSequenceNumber, TransactionId};

/// Isolation level for transactions.
///
/// Defines the degree to which a transaction is isolated from
/// modifications made by concurrent transactions.
///
/// # Isolation Levels (from weakest to strongest)
///
/// - `ReadUncommitted`: Allows dirty reads
/// - `ReadCommitted`: Only sees committed data
/// - `RepeatableRead`: Consistent reads within transaction
/// - `Serializable`: Full serializable isolation
/// - `SnapshotIsolation`: MVCC-based snapshot consistency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// Allows dirty reads (uncommitted data visible).
    ReadUncommitted,
    /// Only committed data is visible.
    ReadCommitted,
    /// Repeated reads return same result within transaction.
    RepeatableRead,
    /// Transactions appear to execute serially.
    Serializable,
    /// Point-in-time snapshot consistency (MVCC).
    SnapshotIsolation,
}

impl Default for IsolationLevel {
    fn default() -> Self {
        IsolationLevel::ReadCommitted
    }
}

impl fmt::Display for IsolationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IsolationLevel::ReadUncommitted => write!(f, "READ UNCOMMITTED"),
            IsolationLevel::ReadCommitted => write!(f, "READ COMMITTED"),
            IsolationLevel::RepeatableRead => write!(f, "REPEATABLE READ"),
            IsolationLevel::Serializable => write!(f, "SERIALIZABLE"),
            IsolationLevel::SnapshotIsolation => write!(f, "SNAPSHOT"),
        }
    }
}

/// Transaction lifecycle state.
///
/// Represents the current phase of a transaction in its lifecycle.
/// State transitions follow a well-defined state machine:
///
/// ```text
/// Active -> Growing -> Shrinking -> Committing -> Committed
///                                -> Aborting   -> Aborted
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionState {
    /// Transaction is actively executing operations.
    Active,
    /// Two-phase locking: acquiring locks (growing phase).
    Growing,
    /// Two-phase locking: releasing locks (shrinking phase).
    Shrinking,
    /// Two-phase commit: preparing to commit.
    Preparing,
    /// Two-phase commit: prepared and waiting for commit decision.
    Prepared,
    /// Transaction is in the process of committing.
    Committing,
    /// Transaction has successfully committed.
    Committed,
    /// Transaction is in the process of aborting.
    Aborting,
    /// Transaction has been aborted.
    Aborted,
    /// Transaction state is unknown (recovery scenario).
    Unknown,
}

impl Default for TransactionState {
    fn default() -> Self {
        TransactionState::Active
    }
}

impl TransactionState {
    /// Returns true if the transaction is in a terminal state.
    #[inline]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TransactionState::Committed | TransactionState::Aborted
        )
    }

    /// Returns true if the transaction can still accept operations.
    #[inline]
    pub fn is_active(&self) -> bool {
        matches!(self, TransactionState::Active | TransactionState::Growing)
    }
}

/// Lock mode with fine-grained control.
///
/// Implements a hierarchical locking scheme compatible with
/// intent locks for better concurrency control.
///
/// # Lock Compatibility Matrix
///
/// |     | S | X | IS | IX | SIX | U |
/// |-----|---|---|----|----|-----|---|
/// | S   | ✓ | ✗ | ✓  | ✗  | ✗   | ✓ |
/// | X   | ✗ | ✗ | ✗  | ✗  | ✗   | ✗ |
/// | IS  | ✓ | ✗ | ✓  | ✓  | ✓   | ✓ |
/// | IX  | ✗ | ✗ | ✓  | ✓  | ✗   | ✗ |
/// | SIX | ✗ | ✗ | ✓  | ✗  | ✗   | ✗ |
/// | U   | ✓ | ✗ | ✓  | ✗  | ✗   | ✗ |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LockMode {
    /// Shared lock (S) - Read lock.
    Shared,
    /// Exclusive lock (X) - Write lock.
    Exclusive,
    /// Intent Shared (IS) - Intent to acquire S locks on descendants.
    IntentShared,
    /// Intent Exclusive (IX) - Intent to acquire X locks on descendants.
    IntentExclusive,
    /// Shared Intent Exclusive (SIX) - S lock with intent for X locks.
    SharedIntentExclusive,
    /// Update lock (U) - Prevents deadlocks during upgrade from S to X.
    Update,
}

impl LockMode {
    /// Check if two lock modes are compatible.
    ///
    /// # Arguments
    ///
    /// * `other` - The lock mode to check compatibility with.
    ///
    /// # Returns
    ///
    /// `true` if the locks can be held simultaneously, `false` otherwise.
    pub fn is_compatible(&self, other: &LockMode) -> bool {
        use LockMode::*;
        matches!(
            (self, other),
            (Shared, Shared)
                | (Shared, IntentShared)
                | (Shared, Update)
                | (IntentShared, Shared)
                | (IntentShared, IntentShared)
                | (IntentShared, IntentExclusive)
                | (IntentShared, SharedIntentExclusive)
                | (IntentShared, Update)
                | (IntentExclusive, IntentShared)
                | (IntentExclusive, IntentExclusive)
                | (SharedIntentExclusive, IntentShared)
                | (Update, Shared)
                | (Update, IntentShared)
        )
    }

    /// Get the strength/priority of the lock.
    ///
    /// Higher values indicate stronger locks.
    ///
    /// # Returns
    ///
    /// A value from 1 (weakest) to 6 (strongest).
    pub fn strength(&self) -> u8 {
        match self {
            LockMode::Shared => 1,
            LockMode::IntentShared => 2,
            LockMode::Update => 3,
            LockMode::IntentExclusive => 4,
            LockMode::SharedIntentExclusive => 5,
            LockMode::Exclusive => 6,
        }
    }

    /// Check if this lock mode can be upgraded to another.
    pub fn can_upgrade_to(&self, target: &LockMode) -> bool {
        target.strength() > self.strength()
    }
}

impl fmt::Display for LockMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockMode::Shared => write!(f, "S"),
            LockMode::Exclusive => write!(f, "X"),
            LockMode::IntentShared => write!(f, "IS"),
            LockMode::IntentExclusive => write!(f, "IX"),
            LockMode::SharedIntentExclusive => write!(f, "SIX"),
            LockMode::Update => write!(f, "U"),
        }
    }
}

/// Lock granularity levels.
///
/// Defines the scope/level at which locks can be acquired.
/// Finer granularity allows more concurrency but has higher overhead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LockGranularity {
    /// Row-level lock (finest granularity).
    Row,
    /// Page-level lock.
    Page,
    /// Table-level lock.
    Table,
    /// Database-level lock (coarsest granularity).
    Database,
}

impl LockGranularity {
    /// Returns the granularity level as a numeric value.
    /// Higher values indicate coarser granularity.
    pub fn level(&self) -> u8 {
        match self {
            LockGranularity::Row => 1,
            LockGranularity::Page => 2,
            LockGranularity::Table => 3,
            LockGranularity::Database => 4,
        }
    }
}

/// Version information for MVCC (Multi-Version Concurrency Control).
///
/// Each version represents a snapshot of data at a specific point in time,
/// enabling non-blocking reads and consistent snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    /// Transaction that created this version.
    pub txn_id: TransactionId,
    /// Timestamp when this version was created.
    pub timestamp: SystemTime,
    /// Log sequence number for recovery.
    pub lsn: LogSequenceNumber,
    /// The actual data content.
    pub data: Vec<u8>,
    /// Whether this version represents a deletion.
    pub is_deleted: bool,
}

impl Version {
    /// Creates a new version.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - The creating transaction's ID.
    /// * `lsn` - The log sequence number.
    /// * `data` - The version data.
    /// * `is_deleted` - Whether this is a delete marker.
    pub fn new(
        txn_id: TransactionId,
        lsn: LogSequenceNumber,
        data: Vec<u8>,
        is_deleted: bool,
    ) -> Self {
        Self {
            txn_id,
            timestamp: SystemTime::now(),
            lsn,
            data,
            is_deleted,
        }
    }

    /// Returns the size of this version in bytes.
    pub fn size(&self) -> usize {
        size_of::<Self>() + self.data.len()
    }
}

/// Savepoint for partial rollback within a transaction.
///
/// Savepoints allow rolling back to a specific point within a transaction
/// without aborting the entire transaction.
///
/// # Example
///
/// ```rust
/// // In a transaction:
/// // INSERT INTO table1 ...
/// // SAVEPOINT sp1
/// // INSERT INTO table2 ...
/// // ROLLBACK TO sp1  -- undoes only table2 insert
/// // COMMIT  -- commits table1 insert
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Savepoint {
    /// Unique identifier for this savepoint.
    pub id: u64,
    /// User-defined name for the savepoint.
    pub name: String,
    /// The transaction this savepoint belongs to.
    pub txn_id: TransactionId,
    /// LSN at the time of savepoint creation.
    pub lsn: LogSequenceNumber,
    /// Timestamp when savepoint was created.
    pub timestamp: SystemTime,
}

impl Savepoint {
    /// Creates a new savepoint.
    pub fn new(id: u64, name: String, txn_id: TransactionId, lsn: LogSequenceNumber) -> Self {
        Self {
            id,
            name,
            txn_id,
            lsn,
            timestamp: SystemTime::now(),
        }
    }
}

/// Transaction metadata with comprehensive tracking.
///
/// Represents a database transaction with all associated metadata
/// for tracking locks, read/write sets, and savepoints.
///
/// # Invariants
///
/// - `start_lsn` is set when the transaction begins.
/// - `end_lsn` is set only when the transaction commits or aborts.
/// - `held_locks` contains only locks currently held by this transaction.
/// - `savepoints` is ordered by creation time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique transaction identifier.
    pub id: TransactionId,
    /// Current state of the transaction.
    pub state: TransactionState,
    /// Isolation level for this transaction.
    pub isolation_level: IsolationLevel,
    /// Timestamp when transaction started.
    pub start_time: SystemTime,
    /// Timestamp of last activity.
    pub last_activity: SystemTime,
    /// Set of resources currently locked by this transaction.
    pub held_locks: HashSet<String>,
    /// Set of keys read by this transaction (for conflict detection).
    pub read_set: HashSet<String>,
    /// Set of keys written by this transaction (for conflict detection).
    pub write_set: HashSet<String>,
    /// LSN when transaction began.
    pub start_lsn: LogSequenceNumber,
    /// LSN when transaction ended (commit/abort).
    pub end_lsn: Option<LogSequenceNumber>,
    /// Stack of savepoints within this transaction.
    pub savepoints: Vec<Savepoint>,
    /// Whether this transaction is read-only.
    pub is_readonly: bool,
    /// Optional timeout duration for this transaction.
    pub timeout_duration: Option<Duration>,
    /// Parent transaction ID for nested transactions.
    pub parent_txn: Option<TransactionId>,
}

impl Transaction {
    /// Creates a new transaction with the specified ID and isolation level.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique transaction identifier.
    /// * `isolation_level` - Desired isolation level.
    ///
    /// # Example
    ///
    /// ```rust
    /// let txn = Transaction::new(1, IsolationLevel::ReadCommitted);
    /// assert_eq!(txn.state, TransactionState::Active);
    /// ```
    pub fn new(id: TransactionId, isolation_level: IsolationLevel) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            state: TransactionState::Active,
            isolation_level,
            start_time: now,
            last_activity: now,
            held_locks: HashSet::new(),
            read_set: HashSet::new(),
            write_set: HashSet::new(),
            start_lsn: 0,
            end_lsn: None,
            savepoints: Vec::new(),
            is_readonly: false,
            timeout_duration: None,
            parent_txn: None,
        }
    }

    /// Creates a new read-only transaction.
    pub fn new_readonly(id: TransactionId, isolation_level: IsolationLevel) -> Self {
        let mut txn = Self::new(id, isolation_level);
        txn.is_readonly = true;
        txn
    }

    /// Creates a nested transaction under a parent.
    pub fn new_nested(
        id: TransactionId,
        parent_id: TransactionId,
        isolation_level: IsolationLevel,
    ) -> Self {
        let mut txn = Self::new(id, isolation_level);
        txn.parent_txn = Some(parent_id);
        txn
    }

    /// Checks if the transaction has timed out.
    ///
    /// # Returns
    ///
    /// `true` if the transaction has exceeded its timeout duration.
    pub fn is_timed_out(&self) -> bool {
        if let Some(timeout) = self.timeout_duration {
            if let Ok(elapsed) = SystemTime::now().duration_since(self.last_activity) {
                return elapsed > timeout;
            }
        }
        false
    }

    /// Updates the last activity timestamp.
    pub fn update_activity(&mut self) {
        self.last_activity = SystemTime::now();
    }

    /// Adds a savepoint to this transaction.
    ///
    /// # Arguments
    ///
    /// * `name` - User-defined name for the savepoint.
    /// * `lsn` - Current log sequence number.
    ///
    /// # Returns
    ///
    /// The newly created savepoint.
    pub fn add_savepoint(&mut self, name: String, lsn: LogSequenceNumber) -> Savepoint {
        let sp = Savepoint::new(self.savepoints.len() as u64, name, self.id, lsn);
        self.savepoints.push(sp.clone());
        sp
    }

    /// Gets a savepoint by name.
    pub fn get_savepoint(&self, name: &str) -> Option<&Savepoint> {
        self.savepoints.iter().find(|sp| sp.name == name)
    }

    /// Returns the duration since the transaction started.
    pub fn duration(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.start_time)
            .unwrap_or(Duration::ZERO)
    }

    /// Returns the number of operations (reads + writes) in this transaction.
    pub fn operation_count(&self) -> usize {
        self.read_set.len() + self.write_set.len()
    }

    /// Checks if this is a nested transaction.
    pub fn is_nested(&self) -> bool {
        self.parent_txn.is_some()
    }

    /// Estimates the memory footprint of this transaction.
    pub fn estimated_size(&self) -> usize {
        size_of::<Transaction>()
            + self.held_locks.len() * size_of::<String>()
            + self.read_set.len() * size_of::<String>()
            + self.write_set.len() * size_of::<String>()
            + self.savepoints.len() * size_of::<Savepoint>()
    }
}

#[cfg(test)]
mod tests {
    use crate::common::LockMode;
    use crate::network::distributed::TransactionState;
    use crate::transaction::{Transaction, Version};
    use crate::IsolationLevel;

    #[test]
    fn test_isolation_level_default() {
        assert_eq!(IsolationLevel::default(), IsolationLevel::ReadCommitted);
    }

    #[test]
    fn test_transaction_state_terminal() {
        assert!(TransactionState::Committed.is_terminal());
        assert!(TransactionState::Aborted.is_terminal());
        assert!(!TransactionState::Active.is_terminal());
    }

    #[test]
    fn test_lock_mode_compatibility() {
        assert!(LockMode::Shared.is_compatible(&LockMode::Shared));
        assert!(!LockMode::Shared.is_compatible(&LockMode::Exclusive));
        assert!(!LockMode::Exclusive.is_compatible(&LockMode::Exclusive));
        assert!(LockMode::IntentShared.is_compatible(&LockMode::IntentExclusive));
    }

    #[test]
    fn test_lock_mode_strength() {
        assert!(LockMode::Exclusive.strength() > LockMode::Shared.strength());
        assert!(LockMode::Update.strength() > LockMode::Shared.strength());
    }

    #[test]
    use crate::transaction::types::{IsolationLevel as OtherIsolationLevel, TransactionState};
    use crate::transaction::Transaction;

    fn test_transaction_new() {
        let txn = Transaction::new(1, IsolationLevel::Serializable);
        assert_eq!(txn.id, 1);
        assert_eq!(txn.state, TransactionState::Active);
        assert_eq!(txn.isolation_level, IsolationLevel::Serializable);
        assert!(!txn.is_readonly);
    }

    #[test]
    fn test_transaction_savepoint() {
        let mut txn = Transaction::new(1, IsolationLevel::ReadCommitted);
        let sp = txn.add_savepoint("sp1".to_string(), 100);

        assert_eq!(sp.name, "sp1");
        assert_eq!(txn.savepoints.len(), 1);
        assert!(txn.get_savepoint("sp1").is_some());
        assert!(txn.get_savepoint("sp2").is_none());
    }

    #[test]
    fn test_version_size() {
        let v = Version::new(1, 100, vec![0u8; 1024], false);
        assert!(v.size() >= 1024);
    }
}
