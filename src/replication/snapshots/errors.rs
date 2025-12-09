// Snapshot management specific errors

use thiserror::Error;

/// Snapshot management specific errors
#[derive(Error, Debug)]
pub enum SnapshotError {
    #[error("Snapshot not found: {snapshot_id}")]
    SnapshotNotFound { snapshot_id: String },

    #[error("Invalid snapshot format: {reason}")]
    InvalidFormat { reason: String },

    #[error("Compression failed: {algorithm} - {reason}")]
    CompressionFailed { algorithm: String, reason: String },

    #[error("Decompression failed: {algorithm} - {reason}")]
    DecompressionFailed { algorithm: String, reason: String },

    #[error("Encryption failed: {reason}")]
    EncryptionFailed { reason: String },

    #[error("Decryption failed: {reason}")]
    DecryptionFailed { reason: String },

    #[error("Storage operation failed: {operation} - {reason}")]
    StorageError { operation: String, reason: String },

    #[error("Restore operation failed: {reason}")]
    RestoreError { reason: String },

    #[error("Snapshot validation failed: {snapshot_id} - {reason}")]
    ValidationFailed { snapshot_id: String, reason: String },

    #[error("Retention policy violation: {reason}")]
    RetentionViolation { reason: String },

    #[error("Concurrent operation conflict: {operation}")]
    ConcurrencyConflict { operation: String },

    #[error("Insufficient storage space: required {required_bytes}, available {available_bytes}")]
    InsufficientStorage { required_bytes: u64, available_bytes: u64 },
}
