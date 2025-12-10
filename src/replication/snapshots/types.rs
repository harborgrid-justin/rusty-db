// Core snapshot types

use crate::replication::types::{LogSequenceNumber, ReplicaId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

use super::config::CompressionType;
use super::errors::SnapshotError;

// Unique snapshot identifier
//
// Provides type-safe snapshot identification with validation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(String);

impl SnapshotId {
    // Creates a new snapshot ID
    pub fn new(id: impl Into<String>) -> Result<Self, SnapshotError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(SnapshotError::InvalidFormat {
                reason: "Snapshot ID cannot be empty".to_string(),
            });
        }
        Ok(Self(id))
    }

    // Generates a new unique snapshot ID
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    // Returns the snapshot ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Snapshot metadata
//
// Contains comprehensive information about a snapshot including
// creation details, size, dependencies, and verification data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    // Unique snapshot identifier
    pub snapshot_id: SnapshotId,
    // Associated replica ID
    pub replica_id: ReplicaId,
    // Snapshot type
    pub snapshot_type: SnapshotType,
    // Creation timestamp
    pub created_at: SystemTime,
    // Size of snapshot data
    pub size_bytes: u64,
    // Compressed size
    pub compressed_size_bytes: Option<u64>,
    // Compression algorithm used
    pub compression: CompressionType,
    // Whether snapshot is encrypted
    pub encrypted: bool,
    // Database LSN at snapshot time
    pub lsn: Option<LogSequenceNumber>,
    // Parent snapshot (for incrementals)
    pub parent_snapshot: Option<SnapshotId>,
    // Child snapshots
    pub child_snapshots: Vec<SnapshotId>,
    // Integrity checksum
    pub checksum: String,
    // Checksum algorithm
    pub checksum_algorithm: ChecksumAlgorithm,
    // Tags for organization
    pub tags: HashMap<String, String>,
    // Custom metadata
    pub custom_metadata: HashMap<String, String>,
    // Snapshot status
    pub status: SnapshotStatus,
    // Error information if failed
    pub error_info: Option<String>,
    // Storage path
    pub storage_path: PathBuf,
    // Verification status
    pub verified: bool,
    // Last verification time
    pub last_verified: Option<SystemTime>,
}

// Types of snapshots
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotType {
    // Full database snapshot
    Full,
    // Incremental changes since parent
    Incremental,
    // Differential changes since last full
    Differential,
    // Transaction log based
    TransactionLog,
    // Schema only snapshot
    SchemaOnly,
    // Custom application-defined type
    Custom(String),
}

// Snapshot status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotStatus {
    // Snapshot is being created
    Creating,
    // Snapshot creation completed successfully
    Completed,
    // Snapshot creation failed
    Failed,
    // Snapshot is being verified
    Verifying,
    // Snapshot verification failed
    VerificationFailed,
    // Snapshot is being deleted
    Deleting,
    // Snapshot has been deleted
    Deleted,
    // Snapshot is corrupted
    Corrupted,
}

// Checksum algorithms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChecksumAlgorithm {
    Sha256,
    Blake3,
    Crc32,
    Xxh3,
}

// Snapshot creation progress information
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotProgress {
    // Snapshot being created
    pub snapshot_id: SnapshotId,
    // Current phase
    pub phase: SnapshotPhase,
    // Bytes processed
    pub bytes_processed: u64,
    // Total bytes to process
    pub total_bytes: Option<u64>,
    // Processing rate in bytes per second
    pub processing_rate: f64,
    // Estimated completion time
    pub estimated_completion: Option<SystemTime>,
    // Current operation
    pub current_operation: String,
    // Progress percentage (0-100)
    pub progress_percentage: f32,
}

// Phases of snapshot creation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotPhase {
    Initializing,
    ReadingData,
    Compressing,
    Encrypting,
    Writing,
    Verifying,
    Finalizing,
    Completed,
}

// Snapshot restore options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreOptions {
    // Target path for restore
    pub target_path: Option<PathBuf>,
    // Whether to verify restored data
    pub verify_integrity: bool,
    // Whether to restore in parallel
    pub parallel_restore: bool,
    // Maximum parallel threads
    pub max_threads: usize,
    // Restore specific tables only
    pub table_filter: Option<HashSet<String>>,
    // Point-in-time recovery target
    pub target_lsn: Option<LogSequenceNumber>,
    // Whether to skip existing data
    pub skip_existing: bool,
    // Custom restore parameters
    pub custom_options: HashMap<String, String>,
}

impl Default for RestoreOptions {
    fn default() -> Self {
        Self {
            target_path: None,
            verify_integrity: true,
            parallel_restore: true,
            max_threads: 4,
            table_filter: None,
            target_lsn: None,
            skip_existing: false,
            custom_options: HashMap::new(),
        }
    }
}

// Snapshot statistics
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotStatistics {
    // Total number of snapshots
    pub total_snapshots: usize,
    // Number of full snapshots
    pub full_snapshots: usize,
    // Number of incremental snapshots
    pub incremental_snapshots: usize,
    // Total storage used
    pub total_storage_bytes: u64,
    // Storage after compression
    pub compressed_storage_bytes: u64,
    // Average compression ratio
    pub average_compression_ratio: f64,
    // Oldest snapshot age
    pub oldest_snapshot_age: Option<Duration>,
    // Average snapshot creation time
    pub average_creation_time: Duration,
    // Success rate percentage
    pub success_rate: f64,
    // Storage efficiency metrics
    pub storage_efficiency: StorageEfficiency,
}

// Storage efficiency metrics
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEfficiency {
    // Deduplication ratio
    pub deduplication_ratio: f64,
    // Compression effectiveness
    pub compression_effectiveness: f64,
    // Space savings percentage
    pub space_savings: f64,
    // Average file size reduction
    pub average_size_reduction: f64,
}
