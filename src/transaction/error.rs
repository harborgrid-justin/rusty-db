//! Transaction-specific error types.
//!
//! This module provides structured error handling for the transaction
//! subsystem using `thiserror` for ergonomic error definitions.
//!
//! # Error Categories
//!
//! - **Locking errors**: Lock conflicts, timeouts, deadlocks
//! - **State errors**: Invalid state transitions
//! - **Validation errors**: Constraint violations
//! - **I/O errors**: WAL and storage failures
//! - **Recovery errors**: Checkpoint and recovery failures

use std::io;
use thiserror::Error;

use crate::common::TransactionId;

use super::types::{IsolationLevel, LockMode, TransactionState};

/// Result type alias for transaction operations.
pub type TransactionResult<T> = std::result::Result<T, TransactionError>;

/// Comprehensive error type for transaction operations.
///
/// Each variant captures specific context to aid in debugging
/// and enables appropriate error handling strategies.
#[derive(Debug, Error)]
pub enum TransactionError {
    // =========================================================================
    // Lock-related errors
    // =========================================================================
    
    /// Lock acquisition timed out.
    #[error("Lock timeout: transaction {txn_id} timed out waiting for {lock_mode} lock on '{resource}'")]
    LockTimeout {
        txn_id: TransactionId,
        resource: String,
        lock_mode: LockMode,
    },

    /// Lock conflict with another transaction.
    #[error("Lock conflict: transaction {requesting_txn} cannot acquire {requested_mode} lock on '{resource}' - held by transaction {holding_txn} with {held_mode} lock")]
    LockConflict {
        requesting_txn: TransactionId,
        holding_txn: TransactionId,
        resource: String,
        requested_mode: LockMode,
        held_mode: LockMode,
    },

    /// Deadlock detected between transactions.
    #[error("Deadlock detected: cycle involves transactions {}", format_txn_cycle(cycle))]
    Deadlock {
        /// The transactions involved in the deadlock cycle.
        cycle: Vec<TransactionId>,
        /// The transaction selected as the victim.
        victim: TransactionId,
    },

    /// Lock upgrade failed.
    #[error("Lock upgrade failed: cannot upgrade from {from} to {to} on '{resource}'")]
    LockUpgradeFailed {
        resource: String,
        from: LockMode,
        to: LockMode,
    },

    // =========================================================================
    // Transaction state errors
    // =========================================================================

    /// Transaction not found.
    #[error("Transaction {0} not found")]
    TransactionNotFound(TransactionId),

    /// Invalid state transition.
    #[error("Invalid state transition: transaction {txn_id} cannot transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        txn_id: TransactionId,
        from: TransactionState,
        to: TransactionState,
    },

    /// Transaction already committed.
    #[error("Transaction {0} has already been committed")]
    AlreadyCommitted(TransactionId),

    /// Transaction already aborted.
    #[error("Transaction {0} has already been aborted")]
    AlreadyAborted(TransactionId),

    /// Transaction is read-only but write was attempted.
    #[error("Transaction {0} is read-only; write operation not permitted")]
    ReadOnlyTransaction(TransactionId),

    // =========================================================================
    // Validation errors
    // =========================================================================

    /// Savepoint not found.
    #[error("Savepoint '{name}' not found in transaction {txn_id}")]
    SavepointNotFound { txn_id: TransactionId, name: String },

    /// Transaction exceeded maximum duration.
    #[error("Transaction {txn_id} exceeded maximum duration of {max_duration:?}")]
    TransactionTimeout {
        txn_id: TransactionId,
        max_duration: std::time::Duration,
    },

    /// Transaction exceeded maximum operations.
    #[error("Transaction {txn_id} exceeded maximum operations limit of {max_ops}")]
    TooManyOperations {
        txn_id: TransactionId,
        max_ops: usize,
    },

    /// Transaction exceeded memory limit.
    #[error("Transaction {txn_id} exceeded memory limit of {limit_bytes} bytes")]
    MemoryLimitExceeded {
        txn_id: TransactionId,
        limit_bytes: usize,
    },

    /// Required isolation level not met.
    #[error("Transaction {txn_id} requires isolation level {required:?}, but has {actual:?}")]
    IsolationLevelMismatch {
        txn_id: TransactionId,
        required: IsolationLevel,
        actual: IsolationLevel,
    },

    /// Optimistic concurrency control validation failed.
    #[error("Validation failed for transaction {txn_id}: read set conflict on key '{key}'")]
    ValidationFailed { txn_id: TransactionId, key: String },

    // =========================================================================
    // I/O and persistence errors
    // =========================================================================

    /// WAL write failed.
    #[error("Failed to write to WAL: {0}")]
    WalWriteError(#[source] io::Error),

    /// WAL read failed.
    #[error("Failed to read from WAL: {0}")]
    WalReadError(#[source] io::Error),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Deserialization error.
    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    /// Checkpoint creation failed.
    #[error("Failed to create checkpoint at LSN {lsn}: {reason}")]
    CheckpointFailed { lsn: u64, reason: String },

    // =========================================================================
    // Recovery errors
    // =========================================================================

    /// Recovery failed.
    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),

    /// Redo operation failed.
    #[error("Redo failed for transaction {txn_id} at LSN {lsn}: {reason}")]
    RedoFailed {
        txn_id: TransactionId,
        lsn: u64,
        reason: String,
    },

    /// Undo operation failed.
    #[error("Undo failed for transaction {txn_id} at LSN {lsn}: {reason}")]
    UndoFailed {
        txn_id: TransactionId,
        lsn: u64,
        reason: String,
    },

    // =========================================================================
    // Distributed transaction errors
    // =========================================================================

    /// Two-phase commit prepare failed.
    #[error("Prepare phase failed for transaction {txn_id}: participant '{participant}' did not respond")]
    PreparePhaseTimeout {
        txn_id: TransactionId,
        participant: String,
    },

    /// Two-phase commit participant not found.
    #[error("Participant '{participant}' not found for transaction {txn_id}")]
    ParticipantNotFound {
        txn_id: TransactionId,
        participant: String,
    },

    /// Distributed transaction coordination error.
    #[error("Coordination error for global transaction {txn_id}: {reason}")]
    CoordinationError { txn_id: TransactionId, reason: String },

    // =========================================================================
    // Internal errors
    // =========================================================================

    /// Internal error (should not occur in normal operation).
    #[error("Internal error: {0}")]
    InternalError(String),

    /// Generic error with context.
    #[error("{context}: {message}")]
    Generic { context: String, message: String },
}

impl TransactionError {
    /// Creates a lock timeout error.
    pub fn lock_timeout(txn_id: TransactionId, resource: impl Into<String>, lock_mode: LockMode) -> Self {
        TransactionError::LockTimeout {
            txn_id,
            resource: resource.into(),
            lock_mode,
        }
    }

    /// Creates a lock conflict error.
    pub fn lock_conflict(
        requesting_txn: TransactionId,
        holding_txn: TransactionId,
        resource: impl Into<String>,
        requested_mode: LockMode,
        held_mode: LockMode,
    ) -> Self {
        TransactionError::LockConflict {
            requesting_txn,
            holding_txn,
            resource: resource.into(),
            requested_mode,
            held_mode,
        }
    }

    /// Creates a deadlock error.
    pub fn deadlock(cycle: Vec<TransactionId>, victim: TransactionId) -> Self {
        TransactionError::Deadlock { cycle, victim }
    }

    /// Creates a transaction not found error.
    pub fn not_found(txn_id: TransactionId) -> Self {
        TransactionError::TransactionNotFound(txn_id)
    }

    /// Creates an invalid state transition error.
    pub fn invalid_state(txn_id: TransactionId, from: TransactionState, to: TransactionState) -> Self {
        TransactionError::InvalidStateTransition { txn_id, from, to }
    }

    /// Creates a savepoint not found error.
    pub fn savepoint_not_found(txn_id: TransactionId, name: impl Into<String>) -> Self {
        TransactionError::SavepointNotFound {
            txn_id,
            name: name.into(),
        }
    }

    /// Creates a generic error with context.
    pub fn generic(context: impl Into<String>, message: impl Into<String>) -> Self {
        TransactionError::Generic {
            context: context.into(),
            message: message.into(),
        }
    }

    /// Returns true if this error indicates the transaction should be retried.
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            TransactionError::LockTimeout { .. }
                | TransactionError::LockConflict { .. }
                | TransactionError::Deadlock { .. }
                | TransactionError::ValidationFailed { .. }
        )
    }

    /// Returns true if this is a deadlock-related error.
    pub fn is_deadlock(&self) -> bool {
        matches!(self, TransactionError::Deadlock { .. })
    }

    /// Returns true if this is a lock-related error.
    pub fn is_lock_error(&self) -> bool {
        matches!(
            self,
            TransactionError::LockTimeout { .. }
                | TransactionError::LockConflict { .. }
                | TransactionError::LockUpgradeFailed { .. }
                | TransactionError::Deadlock { .. }
        )
    }

    /// Returns the transaction ID if this error is associated with one.
    pub fn transaction_id(&self) -> Option<TransactionId> {
        match self {
            TransactionError::LockTimeout { txn_id, .. } => Some(*txn_id),
            TransactionError::LockConflict { requesting_txn, .. } => Some(*requesting_txn),
            TransactionError::Deadlock { victim, .. } => Some(*victim),
            TransactionError::TransactionNotFound(id) => Some(*id),
            TransactionError::InvalidStateTransition { txn_id, .. } => Some(*txn_id),
            TransactionError::AlreadyCommitted(id) => Some(*id),
            TransactionError::AlreadyAborted(id) => Some(*id),
            TransactionError::ReadOnlyTransaction(id) => Some(*id),
            TransactionError::SavepointNotFound { txn_id, .. } => Some(*txn_id),
            TransactionError::TransactionTimeout { txn_id, .. } => Some(*txn_id),
            TransactionError::TooManyOperations { txn_id, .. } => Some(*txn_id),
            TransactionError::MemoryLimitExceeded { txn_id, .. } => Some(*txn_id),
            TransactionError::IsolationLevelMismatch { txn_id, .. } => Some(*txn_id),
            TransactionError::ValidationFailed { txn_id, .. } => Some(*txn_id),
            TransactionError::RedoFailed { txn_id, .. } => Some(*txn_id),
            TransactionError::UndoFailed { txn_id, .. } => Some(*txn_id),
            TransactionError::PreparePhaseTimeout { txn_id, .. } => Some(*txn_id),
            TransactionError::ParticipantNotFound { txn_id, .. } => Some(*txn_id),
            TransactionError::CoordinationError { txn_id, .. } => Some(*txn_id),
            _ => None,
        }
    }
}

/// Helper function to format transaction cycle for display.
fn format_txn_cycle(cycle: &[TransactionId]) -> String {
    cycle
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(" -> ")
}

impl From<io::Error> for TransactionError {
    fn from(err: io::Error) -> Self {
        TransactionError::WalWriteError(err)
    }
}

impl From<serde_json::Error> for TransactionError {
    fn from(err: serde_json::Error) -> Self {
        TransactionError::SerializationError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_timeout_error() {
        let err = TransactionError::lock_timeout(1, "table1", LockMode::Exclusive);
        assert!(err.is_lock_error());
        assert!(err.is_retriable());
        assert_eq!(err.transaction_id(), Some(1));
    }

    #[test]
    fn test_deadlock_error() {
        let err = TransactionError::deadlock(vec![1, 2, 3, 1], 2);
        assert!(err.is_deadlock());
        assert!(err.is_retriable());
        assert_eq!(err.transaction_id(), Some(2));
    }

    #[test]
    fn test_error_display() {
        let err = TransactionError::not_found(42);
        assert_eq!(err.to_string(), "Transaction 42 not found");
    }

    #[test]
    fn test_non_retriable_error() {
        let err = TransactionError::AlreadyCommitted(1);
        assert!(!err.is_retriable());
    }
}
