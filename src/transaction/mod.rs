// Enterprise Transaction Management Module
//
// This module provides comprehensive transaction management capabilities
// for a database system, including:
//
// - **ACID Transactions**: Full ACID compliance with multiple isolation levels.
// - **MVCC**: Multi-Version Concurrency Control for non-blocking reads.
// - **2PL**: Two-Phase Locking for serializable isolation.
// - **WAL**: Write-Ahead Logging for durability and crash recovery.
// - **Deadlock Detection**: Automatic deadlock detection and resolution.
// - **Distributed Transactions**: Two-Phase Commit (2PC) protocol.
// - **OCC**: Optimistic Concurrency Control for low-contention workloads.
//
// # Module Organization
//
// The transaction module is organized into focused submodules:
//
// | Module | Responsibility |
// |--------|----------------|
// | [`types`] | Core types: `Transaction`, `IsolationLevel`, `LockMode` |
// | [`error`] | Transaction-specific error types |
// | [`manager`] | Transaction lifecycle management |
// | [`lock_manager`] | Lock acquisition and release |
// | [`wal_manager`] | Write-ahead log operations |
// | [`version_store`] | MVCC version storage |
// | [`deadlock`] | Deadlock detection and resolution |
// | [`snapshot`] | Snapshot isolation management |
// | [`recovery_manager`] | Crash recovery and checkpointing |
// | [`two_phase_commit`] | Distributed transaction coordination |
// | [`occ_manager`] | Optimistic concurrency control |
// | [`statistics`] | Performance metrics and monitoring |
// | [`timeout`] | Transaction timeout management |
// | [`traits`] | Extensibility traits |
//
// # Quick Start
//
// ```rust,ignore
// use rusty_db::transaction::{TransactionManager, IsolationLevel, LockMode};
//
// // Create a transaction manager
// let manager = TransactionManager::new();
//
// // Begin a transaction
// let txn_id = manager.begin()?;
//
// // Acquire locks and perform operations
// let lock_manager = manager.get_lock_manager();
// lock_manager.acquire_lock(txn_id, "table.row1".to_string(), LockMode::Exclusive)?;
//
// // Commit the transaction
// manager.commit(txn_id)?;
// ```
//
// # Architecture
//
// ```text
// ┌─────────────────────────────────────────────────────────────┐
// │                    TransactionManager                        │
// │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
// │  │ LockManager │  │ WALManager  │  │ DeadlockDetector    │  │
// │  └─────────────┘  └─────────────┘  └─────────────────────┘  │
// │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
// │  │VersionStore │  │ Snapshot    │  │ RecoveryManager     │  │
// │  │             │  │ Manager     │  │                     │  │
// │  └─────────────┘  └─────────────┘  └─────────────────────┘  │
// └─────────────────────────────────────────────────────────────┘
// ```

// =============================================================================
// Submodule declarations
// =============================================================================

// Core types and errors
pub mod types;
pub mod error;
pub mod traits;

// Transaction lifecycle
pub mod manager;
pub mod lock_manager;
pub mod timeout;

// Durability and recovery
pub mod wal_manager;
pub mod recovery_manager;

// Concurrency control
pub mod version_store;
pub mod snapshot;
pub mod deadlock;
pub mod occ_manager;

// Distributed transactions
pub mod two_phase_commit;

// Monitoring
pub mod statistics;

// Legacy submodules (existing)
pub mod mvcc;
pub mod distributed;
pub mod wal;
pub mod locks;
pub mod recovery;
pub mod occ;

// =============================================================================
// Re-exports for convenient access
// =============================================================================

// Re-export common types for legacy submodule compatibility
pub use crate::common::{TransactionId, LogSequenceNumber};

// Core types
pub use types::{
    IsolationLevel,
    LockGranularity,
    LockMode,
    Savepoint,
    Transaction,
    TransactionState,
    Version,
};

// Error types
pub use error::{TransactionError, TransactionResult};

// Traits
pub use traits::{
    DeadlockDetection,
    LockManagement,
    Recovery,
    SnapshotManagement,
    TimeoutManagement,
    TransactionEventListener,
    TransactionLifecycle,
    Validation,
    WriteAheadLog,
};

// Transaction manager
pub use manager::TransactionManager;

// Lock management
pub use lock_manager::{
    LockEscalationManager,
    LockManager,
    LockRequest,
    LockTableEntry,
    ReadWriteLockManager,
};

// WAL
pub use wal_manager::{WALConfig, WALEntry, WALManager};

// Version store
pub use version_store::{GarbageCollector, GCStats, VersionStore};

// Deadlock detection
pub use deadlock::{
    DeadlockDetector,
    DeadlockDetectorConfig,
    DeadlockStats,
    VictimSelectionPolicy,
};

// Snapshot isolation
pub use snapshot::{Snapshot, SnapshotManager};

// Recovery
pub use recovery_manager::{RecoveryManager, RecoveryStats};

// Two-phase commit
pub use two_phase_commit::{
    ParticipantInfo,
    ParticipantState,
    TwoPhaseCommitCoordinator,
    TwoPhaseCommitStats,
};

// OCC
pub use occ_manager::{OCCStats, OptimisticConcurrencyControl};

// Statistics
pub use statistics::{
    LockStatistics,
    LockStatisticsSummary,
    StatisticsSummary,
    TransactionStatistics,
};

// Timeout
pub use timeout::TimeoutManager;

// =============================================================================
// Module-level tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_lifecycle() {
        let tm = TransactionManager::new();

        // Begin transaction
        let txn_id = tm.begin().unwrap();
        assert!(tm.is_active(txn_id));

        // Commit
        tm.commit(txn_id).unwrap();
        assert!(!tm.is_active(txn_id));
    }

    #[test]
    fn test_lock_manager_integration() {
        let tm = TransactionManager::new();
        let txn_id = tm.begin().unwrap();

        let lm = tm.get_lock_manager();
        lm.acquire_lock(txn_id, "resource1".to_string(), LockMode::Shared)
            .unwrap();

        assert!(lm.is_locked("resource1"));

        tm.commit(txn_id).unwrap();
        assert!(!lm.is_locked("resource1"));
    }

    #[test]
    fn test_isolation_levels() {
        assert_eq!(IsolationLevel::default(), IsolationLevel::ReadCommitted);

        let tm = TransactionManager::with_isolation(IsolationLevel::Serializable);
        let txn_id = tm.begin().unwrap();

        let txn = tm.get_transaction(txn_id).unwrap();
        assert_eq!(txn.isolation_level, IsolationLevel::Serializable);
    }

    #[test]
    fn test_deadlock_detection() {
        let detector = DeadlockDetector::default();

        // Create a cycle: 1 -> 2 -> 1
        detector.add_wait(1, 2);
        detector.add_wait(2, 1);

        let cycle = detector.force_detect();
        assert!(cycle.is_some());
    }

    #[test]
    fn test_statistics() {
        let _stats = TransactionStatistics::new();

        stats.record_begin();
        stats.record_commit(10);

        let summary = stats.get_summary();
        assert_eq!(summary.total_commits, 1);
    }
}
