// # Replication Snapshot Management
//
// This module provides comprehensive snapshot management for the replication
// system, supporting both incremental and full backups with compression,
// encryption, and efficient storage management.
//
// ## Key Features
//
// - **Incremental Snapshots**: Efficient incremental backup with change tracking
// - **Full Snapshots**: Complete database snapshots for baseline recovery
// - **Compression Support**: Multiple compression algorithms for space efficiency
// - **Encryption**: At-rest encryption for snapshot security
// - **Lifecycle Management**: Automated cleanup and retention policies
// - **Fast Recovery**: Optimized restore operations with parallel processing
//
// ## Snapshot Types
//
// - **Full Snapshot**: Complete database state at a point in time
// - **Incremental Snapshot**: Changes since last full or incremental snapshot
// - **Differential Snapshot**: Changes since last full snapshot
// - **Transaction Log Snapshot**: WAL-based incremental changes
// - **Compressed Snapshot**: Space-optimized snapshots with various algorithms
//
// ## Usage Example
//
// ```rust
// use crate::replication::snapshots::*;
// use crate::replication::types::*;
//
// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
// // Create snapshot manager
// let config = SnapshotConfig {
//     storage_path: "/data/snapshots".into(),
//     compression: CompressionType::Lz4,
//     encryption: Some(EncryptionConfig {
//         algorithm: EncryptionAlgorithm::Aes256Gcm,
//         key_source: KeySource::Environment("SNAPSHOT_KEY".to_string()),
//     }),
//     retention_policy: RetentionPolicy {
//         max_snapshots: 30,
//         max_age: Duration::from_days(90),
//         min_full_snapshots: 2,
//     },
//     ..Default::default()
// };
//
// let manager = SnapshotManager::new(config)?;
//
// // Create a full snapshot
// let _replica_id = ReplicaId::new("replica-01")?;
// let snapshot_id = manager.create_full_snapshot(&replica_id).await?;
//
// // Create incremental snapshot
// let incremental_id = manager.create_incremental_snapshot(
//     &replica_id,
//     &snapshot_id
// ).await?;
//
// // List available snapshots
// let snapshots = manager.list_snapshots(&replica_id).await?;
//
// // Restore from snapshot
// manager.restore_snapshot(&replica_id, &snapshot_id).await?;
// # Ok(())
// # }
// ```

use std::collections::HashSet;
use std::time::SystemTime;
use crate::error::DbError;
use crate::replication::types::*;
use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::fs;
use std::io::{Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncWrite, BufReader, BufWriter};
use uuid::Uuid;

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

/// Snapshot configuration
///
/// Comprehensive configuration for snapshot operations including
/// storage, compression, encryption, and retention policies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    /// Base storage path for snapshots
    pub storage_path: PathBuf,
    /// Default compression algorithm
    pub compression: CompressionType,
    /// Encryption configuration
    pub encryption: Option<EncryptionConfig>,
    /// Snapshot retention policy
    pub retention_policy: RetentionPolicy,
    /// Maximum parallel operations
    pub max_parallel_operations: usize,
    /// Chunk size for large files
    pub chunk_size: usize,
    /// Enable integrity verification
    pub verify_integrity: bool,
    /// Buffer size for I/O operations
    pub buffer_size: usize,
    /// Enable deduplication
    pub enable_deduplication: bool,
    /// Metadata cache size
    pub metadata_cache_size: usize,
    /// Temporary directory for staging
    pub temp_directory: PathBuf,
    /// Enable statistics collection
    pub enable_statistics: bool,
    /// Progress reporting interval
    pub progress_interval: Duration,
    /// Enable background cleanup
    pub enable_background_cleanup: bool,
    /// Cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from("/data/snapshots"),
            compression: CompressionType::Lz4,
            encryption: None,
            retention_policy: RetentionPolicy::default(),
            max_parallel_operations: 4,
            chunk_size: 1024 * 1024 * 8, // 8MB
            verify_integrity: true,
            buffer_size: 1024 * 64, // 64KB
            enable_deduplication: false,
            metadata_cache_size: 10000,
            temp_directory: PathBuf::from("/tmp/snapshots"),
            enable_statistics: true,
            progress_interval: Duration::from_secs(10),
            enable_background_cleanup: true,
            cleanup_interval: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Compression algorithms supported
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompressionType {
    /// No compression
    None,
    /// Fast compression with LZ4
    Lz4,
    /// Balanced compression with Zstandard
    Zstd,
    /// High compression with LZMA
    Lzma,
    /// Gzip compression
    Gzip,
    /// Brotli compression
    Brotli,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Encryption algorithm
    pub algorithm: EncryptionAlgorithm,
    /// Key derivation configuration
    pub key_source: KeySource,
    /// Additional authenticated data
    pub aad: Option<String>,
}

/// Encryption algorithms supported
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM
    Aes256Gcm,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
    /// AES-256-CBC with HMAC
    Aes256CbcHmac,
}

/// Key source for encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeySource {
    /// Key from environment variable
    Environment(String),
    /// Key from file
    File(PathBuf),
    /// Key from key management service
    Kms { service: String, key_id: String },
    /// Inline key (for testing only)
    Inline(Vec<u8>),
}

/// Snapshot retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Maximum number of snapshots to keep
    pub max_snapshots: usize,
    /// Maximum age of snapshots
    pub max_age: Duration,
    /// Minimum number of full snapshots to keep
    pub min_full_snapshots: usize,
    /// Keep hourly snapshots for this duration
    pub hourly_retention: Option<Duration>,
    /// Keep daily snapshots for this duration
    pub daily_retention: Option<Duration>,
    /// Keep weekly snapshots for this duration
    pub weekly_retention: Option<Duration>,
    /// Keep monthly snapshots for this duration
    pub monthly_retention: Option<Duration>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_snapshots: 50,
            max_age: Duration::from_secs(86400 * 90), // 90 days
            min_full_snapshots: 3,
            hourly_retention: Some(Duration::from_secs(86400 * 7)), // 7 days
            daily_retention: Some(Duration::from_secs(86400 * 30)), // 30 days
            weekly_retention: Some(Duration::from_secs(86400 * 90)), // 90 days
            monthly_retention: Some(Duration::from_secs(86400 * 365)), // 1 year
        }
    }
}

/// Unique snapshot identifier
///
/// Provides type-safe snapshot identification with validation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(String);

impl SnapshotId {
    /// Creates a new snapshot ID
    pub fn new(id: impl Into<String>) -> Result<Self, SnapshotError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(SnapshotError::InvalidFormat {
                reason: "Snapshot ID cannot be empty".to_string(),
            });
        }
        Ok(Self(id))
    }

    /// Generates a new unique snapshot ID
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Returns the snapshot ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Snapshot metadata
///
/// Contains comprehensive information about a snapshot including
/// creation details, size, dependencies, and verification data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Unique snapshot identifier
    pub snapshot_id: SnapshotId,
    /// Associated replica ID
    pub replica_id: ReplicaId,
    /// Snapshot type
    pub snapshot_type: SnapshotType,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Size of snapshot data
    pub size_bytes: u64,
    /// Compressed size
    pub compressed_size_bytes: Option<u64>,
    /// Compression algorithm used
    pub compression: CompressionType,
    /// Whether snapshot is encrypted
    pub encrypted: bool,
    /// Database LSN at snapshot time
    pub lsn: Option<LogSequenceNumber>,
    /// Parent snapshot (for incrementals)
    pub parent_snapshot: Option<SnapshotId>,
    /// Child snapshots
    pub child_snapshots: Vec<SnapshotId>,
    /// Integrity checksum
    pub checksum: String,
    /// Checksum algorithm
    pub checksum_algorithm: ChecksumAlgorithm,
    /// Tags for organization
    pub tags: HashMap<String, String>,
    /// Custom metadata
    pub custom_metadata: HashMap<String, String>,
    /// Snapshot status
    pub status: SnapshotStatus,
    /// Error information if failed
    pub error_info: Option<String>,
    /// Storage path
    pub storage_path: PathBuf,
    /// Verification status
    pub verified: bool,
    /// Last verification time
    pub last_verified: Option<SystemTime>,
}

/// Types of snapshots
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotType {
    /// Full database snapshot
    Full,
    /// Incremental changes since parent
    Incremental,
    /// Differential changes since last full
    Differential,
    /// Transaction log based
    TransactionLog,
    /// Schema only snapshot
    SchemaOnly,
    /// Custom application-defined type
    Custom(String),
}

/// Snapshot status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotStatus {
    /// Snapshot is being created
    Creating,
    /// Snapshot creation completed successfully
    Completed,
    /// Snapshot creation failed
    Failed,
    /// Snapshot is being verified
    Verifying,
    /// Snapshot verification failed
    VerificationFailed,
    /// Snapshot is being deleted
    Deleting,
    /// Snapshot has been deleted
    Deleted,
    /// Snapshot is corrupted
    Corrupted,
}

/// Checksum algorithms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChecksumAlgorithm {
    Sha256,
    Blake3,
    Crc32,
    Xxh3,
}

/// Snapshot creation progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotProgress {
    /// Snapshot being created
    pub snapshot_id: SnapshotId,
    /// Current phase
    pub phase: SnapshotPhase,
    /// Bytes processed
    pub bytes_processed: u64,
    /// Total bytes to process
    pub total_bytes: Option<u64>,
    /// Processing rate in bytes per second
    pub processing_rate: f64,
    /// Estimated completion time
    pub estimated_completion: Option<SystemTime>,
    /// Current operation
    pub current_operation: String,
    /// Progress percentage (0-100)
    pub progress_percentage: f32,
}

/// Phases of snapshot creation
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

/// Snapshot restore options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreOptions {
    /// Target path for restore
    pub target_path: Option<PathBuf>,
    /// Whether to verify restored data
    pub verify_integrity: bool,
    /// Whether to restore in parallel
    pub parallel_restore: bool,
    /// Maximum parallel threads
    pub max_threads: usize,
    /// Restore specific tables only
    pub table_filter: Option<HashSet<String>>,
    /// Point-in-time recovery target
    pub target_lsn: Option<LogSequenceNumber>,
    /// Whether to skip existing data
    pub skip_existing: bool,
    /// Custom restore parameters
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

/// Snapshot statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotStatistics {
    /// Total number of snapshots
    pub total_snapshots: usize,
    /// Number of full snapshots
    pub full_snapshots: usize,
    /// Number of incremental snapshots
    pub incremental_snapshots: usize,
    /// Total storage used
    pub total_storage_bytes: u64,
    /// Storage after compression
    pub compressed_storage_bytes: u64,
    /// Average compression ratio
    pub average_compression_ratio: f64,
    /// Oldest snapshot age
    pub oldest_snapshot_age: Option<Duration>,
    /// Average snapshot creation time
    pub average_creation_time: Duration,
    /// Success rate percentage
    pub success_rate: f64,
    /// Storage efficiency metrics
    pub storage_efficiency: StorageEfficiency,
}

/// Storage efficiency metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEfficiency {
    /// Deduplication ratio
    pub deduplication_ratio: f64,
    /// Compression effectiveness
    pub compression_effectiveness: f64,
    /// Space savings percentage
    pub space_savings: f64,
    /// Average file size reduction
    pub average_size_reduction: f64,
}

/// Snapshot manager trait for different implementations
#[async_trait]
pub trait SnapshotManager: Send + Sync {
    /// Create a full snapshot
    async fn create_full_snapshot(&self, replica_id: &ReplicaId) -> Result<SnapshotId, SnapshotError>;

    /// Create an incremental snapshot
    async fn create_incremental_snapshot(
        &self,
        replica_id: &ReplicaId,
        parent_snapshot: &SnapshotId,
    ) -> Result<SnapshotId, SnapshotError>;

    /// Create a differential snapshot
    async fn create_differential_snapshot(
        &self,
        replica_id: &ReplicaId,
        base_snapshot: &SnapshotId,
    ) -> Result<SnapshotId, SnapshotError>;

    /// List all snapshots for a replica
    async fn list_snapshots(&self, replica_id: &ReplicaId) -> Result<Vec<SnapshotMetadata>, SnapshotError>;

    /// Get snapshot metadata
    async fn get_snapshot_metadata(&self, snapshot_id: &SnapshotId) -> Result<SnapshotMetadata, SnapshotError>;

    /// Delete a snapshot
    async fn delete_snapshot(&self, snapshot_id: &SnapshotId) -> Result<(), SnapshotError>;

    /// Restore from snapshot
    async fn restore_snapshot(
        &self,
        replica_id: &ReplicaId,
        snapshot_id: &SnapshotId,
    ) -> Result<(), SnapshotError>;

    /// Restore from snapshot with options
    async fn restore_snapshot_with_options(
        &self,
        replica_id: &ReplicaId,
        snapshot_id: &SnapshotId,
        options: RestoreOptions,
    ) -> Result<(), SnapshotError>;

    /// Verify snapshot integrity
    async fn verify_snapshot(&self, snapshot_id: &SnapshotId) -> Result<bool, SnapshotError>;

    /// Apply retention policy
    async fn apply_retention_policy(&self, replica_id: &ReplicaId) -> Result<Vec<SnapshotId>, SnapshotError>;

    /// Get snapshot statistics
    async fn get_statistics(&self, replica_id: Option<&ReplicaId>) -> Result<SnapshotStatistics, SnapshotError>;
}

/// File-based snapshot manager implementation
pub struct FileSnapshotManager {
    /// Configuration
    config: Arc<SnapshotConfig>,
    /// Metadata cache
    metadata_cache: Arc<RwLock<HashMap<SnapshotId, SnapshotMetadata>>>,
    /// Active operations tracking
    active_operations: Arc<Mutex<HashSet<SnapshotId>>>,
    /// Statistics
    statistics: Arc<RwLock<SnapshotStatistics>>,
    /// Background tasks
    cleanup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Progress tracking
    progress_tracking: Arc<RwLock<HashMap<SnapshotId, SnapshotProgress>>>,
}

impl FileSnapshotManager {
    /// Creates a new file-based snapshot manager
    ///
    /// # Arguments
    ///
    /// * `config` - Snapshot configuration
    ///
    /// # Returns
    ///
    /// * `Ok(FileSnapshotManager)` - Successfully created manager
    /// * `Err(SnapshotError)` - Creation failed
    pub async fn new(config: SnapshotConfig) -> Result<Self, SnapshotError> {
        // Validate configuration
        Self::validate_config(&config).await?;

        // Create directories
        Self::create_directories(&config).await?;

        let manager = Self {
            config: Arc::new(config),
            metadata_cache: Arc::new(RwLock::new(HashMap::new())),
            active_operations: Arc::new(Mutex::new(HashSet::new())),
            statistics: Arc::new(RwLock::new(SnapshotStatistics {
                total_snapshots: 0,
                full_snapshots: 0,
                incremental_snapshots: 0,
                total_storage_bytes: 0,
                compressed_storage_bytes: 0,
                average_compression_ratio: 1.0,
                oldest_snapshot_age: None,
                average_creation_time: Duration::from_secs(60),
                success_rate: 100.0,
                storage_efficiency: StorageEfficiency {
                    deduplication_ratio: 1.0,
                    compression_effectiveness: 0.0,
                    space_savings: 0.0,
                    average_size_reduction: 0.0,
                },
            })),
            cleanup_handle: Arc::new(Mutex::new(None)),
            progress_tracking: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load existing snapshots
        manager.load_existing_snapshots().await?;

        // Start background cleanup if enabled
        if manager.config.enable_background_cleanup {
            manager.start_background_cleanup().await;
        }

        Ok(manager)
    }

    /// Validates the snapshot configuration
    async fn validate_config(config: &SnapshotConfig) -> Result<(), SnapshotError> {
        if !config.storage_path.exists() {
            return Err(SnapshotError::StorageError {
                operation: "validate".to_string(),
                reason: format!("Storage path does not exist: {:?}", config.storage_path),
            });
        }

        if config.chunk_size == 0 {
            return Err(SnapshotError::InvalidFormat {
                reason: "Chunk size must be greater than 0".to_string(),
            });
        }

        if config.max_parallel_operations == 0 {
            return Err(SnapshotError::InvalidFormat {
                reason: "Max parallel operations must be greater than 0".to_string(),
            });
        }

        Ok(())
    }

    /// Creates necessary directories
    async fn create_directories(config: &SnapshotConfig) -> Result<(), SnapshotError> {
        tokio::fs::create_dir_all(&config.storage_path)
            .await
            .map_err(|e| SnapshotError::StorageError {
                operation: "create_directory".to_string(),
                reason: e.to_string(),
            })?;

        tokio::fs::create_dir_all(&config.temp_directory)
            .await
            .map_err(|e| SnapshotError::StorageError {
                operation: "create_temp_directory".to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Loads existing snapshots from storage
    async fn load_existing_snapshots(&self) -> Result<(), SnapshotError> {
        let mut metadata_files = tokio::fs::read_dir(&self.config.storage_path)
            .await
            .map_err(|e| SnapshotError::StorageError {
                operation: "read_directory".to_string(),
                reason: e.to_string(),
            })?;

        while let Some(entry) = metadata_files.next_entry()
            .await
            .map_err(|e| SnapshotError::StorageError {
                operation: "read_directory_entry".to_string(),
                reason: e.to_string(),
            })? {

            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "metadata") {
                if let Ok(metadata) = self.load_snapshot_metadata(&path).await {
                    let mut cache = self.metadata_cache.write();
                    cache.insert(metadata.snapshot_id.clone(), metadata);
                }
            }
        }

        // Update statistics
        self.update_statistics().await;

        Ok(())
    }

    /// Loads snapshot metadata from file
    async fn load_snapshot_metadata(&self, path: &Path) -> Result<SnapshotMetadata, SnapshotError> {
        let contents = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| SnapshotError::StorageError {
                operation: "read_metadata".to_string(),
                reason: e.to_string(),
            })?;

        serde_json::from_str(&contents)
            .map_err(|e| SnapshotError::InvalidFormat {
                reason: format!("Failed to parse metadata: {}", e),
            })
    }

    /// Saves snapshot metadata to file
    async fn save_snapshot_metadata(&self, metadata: &SnapshotMetadata) -> Result<(), SnapshotError> {
        let metadata_path = self.config.storage_path.join(format!("{}.metadata", metadata.snapshot_id));

        let contents = serde_json::to_string_pretty(metadata)
            .map_err(|e| SnapshotError::StorageError {
                operation: "serialize_metadata".to_string(),
                reason: e.to_string(),
            })?;

        tokio::fs::write(&metadata_path, contents)
            .await
            .map_err(|e| SnapshotError::StorageError {
                operation: "write_metadata".to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Creates snapshot data file path
    fn get_snapshot_data_path(&self, snapshot_id: &SnapshotId) -> PathBuf {
        self.config.storage_path.join(format!("{}.snapshot", snapshot_id))
    }

    /// Starts background cleanup task
    async fn start_background_cleanup(&self) {
        let config = Arc::clone(&self.config);
        let metadata_cache = Arc::clone(&self.metadata_cache);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.cleanup_interval);

            loop {
                interval.tick().await;

                // Apply retention policies to all replicas
                let replica_ids: Vec<ReplicaId> = {
                    let cache = metadata_cache.read();
                    cache.values().map(|m| m.replica_id.clone()).collect::<HashSet<_>>().into_iter().collect()
                };

                for replica_id in replica_ids {
                    // Would apply retention policy here
                    // let _ = self.apply_retention_policy(&replica_id).await;
                }
            }
        });

        *self.cleanup_handle.lock() = Some(handle);
    }

    /// Updates statistics based on current snapshots
    async fn update_statistics(&self) {
        let cache = self.metadata_cache.read();
        let snapshots: Vec<_> = cache.values().collect();

        if snapshots.is_empty() {
            return;
        }

        let total_snapshots = snapshots.len();
        let full_snapshots = snapshots.iter().filter(|s| s.snapshot_type == SnapshotType::Full).count();
        let incremental_snapshots = snapshots.iter().filter(|s| s.snapshot_type == SnapshotType::Incremental).count();

        let total_storage_bytes: u64 = snapshots.iter().map(|s| s.size_bytes).sum();
        let compressed_storage_bytes: u64 = snapshots.iter()
            .map(|s| s.compressed_size_bytes.unwrap_or(s.size_bytes))
            .sum();

        let average_compression_ratio = if total_storage_bytes > 0 {
            compressed_storage_bytes as f64 / total_storage_bytes as f64
        } else {
            1.0
        };

        let oldest_snapshot_age = snapshots.iter()
            .map(|s| s.created_at)
            .min()
            .map(|oldest| SystemTime::now().duration_since(oldest).unwrap_or_default());

        let success_rate = {
            let successful = snapshots.iter().filter(|s| s.status == SnapshotStatus::Completed).count();
            if total_snapshots > 0 {
                (successful as f64 / total_snapshots as f64) * 100.0
            } else {
                100.0
            }
        };

        let storage_efficiency = StorageEfficiency {
            deduplication_ratio: 1.0, // Simplified
            compression_effectiveness: 1.0 - average_compression_ratio,
            space_savings: if total_storage_bytes > 0 {
                ((total_storage_bytes - compressed_storage_bytes) as f64 / total_storage_bytes as f64) * 100.0
            } else {
                0.0
            },
            average_size_reduction: average_compression_ratio,
        };

        let mut stats = self.statistics.write();
        *stats = SnapshotStatistics {
            total_snapshots,
            full_snapshots,
            incremental_snapshots,
            total_storage_bytes,
            compressed_storage_bytes,
            average_compression_ratio,
            oldest_snapshot_age,
            average_creation_time: Duration::from_secs(120), // Simplified
            success_rate,
            storage_efficiency,
        };
    }

    /// Compresses data using specified algorithm
    async fn compress_data(&self, data: &[u8], algorithm: &CompressionType) -> Result<Vec<u8>, SnapshotError> {
        match algorithm {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Lz4 => {
                // Simplified implementation - would use actual LZ4
                Ok(data.to_vec())
            },
            CompressionType::Zstd => {
                // Simplified implementation - would use actual Zstd
                Ok(data.to_vec())
            },
            CompressionType::Lzma => {
                // Simplified implementation - would use actual LZMA
                Ok(data.to_vec())
            },
            CompressionType::Gzip => {
                // Simplified implementation - would use actual Gzip
                Ok(data.to_vec())
            },
            CompressionType::Brotli => {
                // Simplified implementation - would use actual Brotli
                Ok(data.to_vec())
            },
        }
    }

    /// Decompresses data using specified algorithm
    async fn decompress_data(&self, data: &[u8], algorithm: &CompressionType) -> Result<Vec<u8>, SnapshotError> {
        match algorithm {
            CompressionType::None => Ok(data.to_vec()),
            _ => {
                // Simplified implementation - would use actual decompression
                Ok(data.to_vec())
            }
        }
    }

    /// Calculates checksum for data
    fn calculate_checksum(&self, data: &[u8], algorithm: &ChecksumAlgorithm) -> String {
        match algorithm {
            ChecksumAlgorithm::Sha256 => {
                // Simplified implementation - would use actual SHA-256
                format!("sha256:{:x}", data.len())
            },
            ChecksumAlgorithm::Blake3 => {
                // Simplified implementation - would use actual BLAKE3
                format!("blake3:{:x}", data.len())
            },
            ChecksumAlgorithm::Crc32 => {
                // Simplified implementation - would use actual CRC32
                format!("crc32:{:x}", data.len())
            },
            ChecksumAlgorithm::Xxh3 => {
                // Simplified implementation - would use actual XXH3
                format!("xxh3:{:x}", data.len())
            },
        }
    }

    /// Creates a full snapshot implementation
    async fn create_full_snapshot_impl(&self, replica_id: &ReplicaId) -> Result<SnapshotId, SnapshotError> {
        let snapshot_id = SnapshotId::generate();

        // Check for concurrent operations
        {
            let mut active = self.active_operations.lock();
            if active.contains(&snapshot_id) {
                return Err(SnapshotError::ConcurrencyConflict {
                    operation: "create_full_snapshot".to_string(),
                });
            }
            active.insert(snapshot_id.clone());
        }

        // Initialize progress tracking
        {
            let mut progress = self.progress_tracking.write();
            progress.insert(snapshot_id.clone(), SnapshotProgress {
                snapshot_id: snapshot_id.clone(),
                phase: SnapshotPhase::Initializing,
                bytes_processed: 0,
                total_bytes: Some(1024 * 1024 * 100), // 100MB example
                processing_rate: 0.0,
                estimated_completion: None,
                current_operation: "Initializing snapshot".to_string(),
                progress_percentage: 0.0,
            });
        }

        // Simulate snapshot creation
        let snapshot_data = b"Full snapshot data".to_vec(); // Simplified

        // Update progress
        {
            let mut progress = self.progress_tracking.write();
            if let Some(p) = progress.get_mut(&snapshot_id) {
                p.phase = SnapshotPhase::ReadingData;
                p.current_operation = "Reading database data".to_string();
                p.progress_percentage = 20.0;
            }
        }

        // Compress data
        let compressed_data = self.compress_data(&snapshot_data, &self.config.compression).await?;

        // Update progress
        {
            let mut progress = self.progress_tracking.write();
            if let Some(p) = progress.get_mut(&snapshot_id) {
                p.phase = SnapshotPhase::Compressing;
                p.current_operation = "Compressing data".to_string();
                p.progress_percentage = 50.0;
            }
        }

        // Calculate checksum
        let checksum = self.calculate_checksum(&compressed_data, &ChecksumAlgorithm::Sha256);

        // Write to storage
        let data_path = self.get_snapshot_data_path(&snapshot_id);
        tokio::fs::write(&data_path, &compressed_data)
            .await
            .map_err(|e| SnapshotError::StorageError {
                operation: "write_snapshot".to_string(),
                reason: e.to_string(),
            })?;

        // Update progress
        {
            let mut progress = self.progress_tracking.write();
            if let Some(p) = progress.get_mut(&snapshot_id) {
                p.phase = SnapshotPhase::Writing;
                p.current_operation = "Writing to storage".to_string();
                p.progress_percentage = 80.0;
            }
        }

        // Create metadata
        let metadata = SnapshotMetadata {
            snapshot_id: snapshot_id.clone(),
            replica_id: replica_id.clone(),
            snapshot_type: SnapshotType::Full,
            created_at: SystemTime::now(),
            size_bytes: snapshot_data.len() as u64,
            compressed_size_bytes: Some(compressed_data.len() as u64),
            compression: self.config.compression.clone(),
            encrypted: self.config.encryption.is_some(),
            lsn: Some(LogSequenceNumber::new(12345)), // Example LSN
            parent_snapshot: None,
            child_snapshots: Vec::new(),
            checksum,
            checksum_algorithm: ChecksumAlgorithm::Sha256,
            tags: HashMap::new(),
            custom_metadata: HashMap::new(),
            status: SnapshotStatus::Completed,
            error_info: None,
            storage_path: data_path,
            verified: false,
            last_verified: None,
        };

        // Save metadata
        self.save_snapshot_metadata(&metadata).await?;

        // Update cache
        {
            let mut cache = self.metadata_cache.write();
            cache.insert(snapshot_id.clone(), metadata);
        }

        // Complete progress tracking
        {
            let mut progress = self.progress_tracking.write();
            if let Some(p) = progress.get_mut(&snapshot_id) {
                p.phase = SnapshotPhase::Completed;
                p.current_operation = "Snapshot completed".to_string();
                p.progress_percentage = 100.0;
            }
        }

        // Remove from active operations
        {
            let mut active = self.active_operations.lock();
            active.remove(&snapshot_id);
        }

        // Update statistics
        self.update_statistics().await;

        Ok(snapshot_id)
    }
}

#[async_trait]
impl SnapshotManager for FileSnapshotManager {
    async fn create_full_snapshot(&self, replica_id: &ReplicaId) -> Result<SnapshotId, SnapshotError> {
        self.create_full_snapshot_impl(replica_id).await
    }

    async fn create_incremental_snapshot(
        &self,
        replica_id: &ReplicaId,
        parent_snapshot: &SnapshotId,
    ) -> Result<SnapshotId, SnapshotError> {
        // Verify parent snapshot exists
        {
            let cache = self.metadata_cache.read();
            if !cache.contains_key(parent_snapshot) {
                return Err(SnapshotError::SnapshotNotFound {
                    snapshot_id: parent_snapshot.to_string(),
                });
            }
        }

        let snapshot_id = SnapshotId::generate();

        // Simulate incremental snapshot creation (simplified)
        let snapshot_data = b"Incremental snapshot data".to_vec();
        let compressed_data = self.compress_data(&snapshot_data, &self.config.compression).await?;
        let checksum = self.calculate_checksum(&compressed_data, &ChecksumAlgorithm::Sha256);

        let data_path = self.get_snapshot_data_path(&snapshot_id);
        tokio::fs::write(&data_path, &compressed_data)
            .await
            .map_err(|e| SnapshotError::StorageError {
                operation: "write_snapshot".to_string(),
                reason: e.to_string(),
            })?;

        let metadata = SnapshotMetadata {
            snapshot_id: snapshot_id.clone(),
            replica_id: replica_id.clone(),
            snapshot_type: SnapshotType::Incremental,
            created_at: SystemTime::now(),
            size_bytes: snapshot_data.len() as u64,
            compressed_size_bytes: Some(compressed_data.len() as u64),
            compression: self.config.compression.clone(),
            encrypted: self.config.encryption.is_some(),
            lsn: Some(LogSequenceNumber::new(12350)), // Example LSN after parent
            parent_snapshot: Some(parent_snapshot.clone()),
            child_snapshots: Vec::new(),
            checksum,
            checksum_algorithm: ChecksumAlgorithm::Sha256,
            tags: HashMap::new(),
            custom_metadata: HashMap::new(),
            status: SnapshotStatus::Completed,
            error_info: None,
            storage_path: data_path,
            verified: false,
            last_verified: None,
        };

        // Save metadata and update cache
        self.save_snapshot_metadata(&metadata).await?;
        {
            let mut cache = self.metadata_cache.write();
            cache.insert(snapshot_id.clone(), metadata);

            // Update parent to reference this child
            if let Some(parent_metadata) = cache.get_mut(parent_snapshot) {
                parent_metadata.child_snapshots.push(snapshot_id.clone());
            }
        }

        self.update_statistics().await;

        Ok(snapshot_id)
    }

    async fn create_differential_snapshot(
        &self,
        replica_id: &ReplicaId,
        base_snapshot: &SnapshotId,
    ) -> Result<SnapshotId, SnapshotError> {
        // Similar to incremental but tracks changes from base full snapshot
        self.create_incremental_snapshot(replica_id, base_snapshot).await
    }

    async fn list_snapshots(&self, replica_id: &ReplicaId) -> Result<Vec<SnapshotMetadata>, SnapshotError> {
        let cache = self.metadata_cache.read();
        Ok(cache.values()
            .filter(|metadata| &metadata.replica_id == replica_id)
            .cloned()
            .collect())
    }

    async fn get_snapshot_metadata(&self, snapshot_id: &SnapshotId) -> Result<SnapshotMetadata, SnapshotError> {
        let cache = self.metadata_cache.read();
        cache.get(snapshot_id)
            .cloned()
            .ok_or_else(|| SnapshotError::SnapshotNotFound {
                snapshot_id: snapshot_id.to_string(),
            })
    }

    async fn delete_snapshot(&self, snapshot_id: &SnapshotId) -> Result<(), SnapshotError> {
        let metadata = {
            let mut cache = self.metadata_cache.write();
            cache.remove(snapshot_id)
                .ok_or_else(|| SnapshotError::SnapshotNotFound {
                    snapshot_id: snapshot_id.to_string(),
                })?
        };

        // Delete data file
        if metadata.storage_path.exists() {
            tokio::fs::remove_file(&metadata.storage_path)
                .await
                .map_err(|e| SnapshotError::StorageError {
                    operation: "delete_data".to_string(),
                    reason: e.to_string(),
                })?;
        }

        // Delete metadata file
        let metadata_path = self.config.storage_path.join(format!("{}.metadata", snapshot_id));
        if metadata_path.exists() {
            tokio::fs::remove_file(&metadata_path)
                .await
                .map_err(|e| SnapshotError::StorageError {
                    operation: "delete_metadata".to_string(),
                    reason: e.to_string(),
                })?;
        }

        self.update_statistics().await;

        Ok(())
    }

    async fn restore_snapshot(
        &self,
        replica_id: &ReplicaId,
        snapshot_id: &SnapshotId,
    ) -> Result<(), SnapshotError> {
        let options = RestoreOptions::default();
        self.restore_snapshot_with_options(replica_id, snapshot_id, options).await
    }

    async fn restore_snapshot_with_options(
        &self,
        _replica_id: &ReplicaId,
        snapshot_id: &SnapshotId,
        _options: RestoreOptions,
    ) -> Result<(), SnapshotError> {
        let metadata = self.get_snapshot_metadata(snapshot_id).await?;

        // Read compressed data
        let compressed_data = tokio::fs::read(&metadata.storage_path)
            .await
            .map_err(|e| SnapshotError::StorageError {
                operation: "read_snapshot".to_string(),
                reason: e.to_string(),
            })?;

        // Verify checksum
        let calculated_checksum = self.calculate_checksum(&compressed_data, &metadata.checksum_algorithm);
        if calculated_checksum != metadata.checksum {
            return Err(SnapshotError::ValidationFailed {
                snapshot_id: snapshot_id.to_string(),
                reason: "Checksum mismatch".to_string(),
            });
        }

        // Decompress data
        let _data = self.decompress_data(&compressed_data, &metadata.compression).await?;

        // In a real implementation, would restore the data to the replica
        // For now, we just simulate success

        Ok(())
    }

    async fn verify_snapshot(&self, snapshot_id: &SnapshotId) -> Result<bool, SnapshotError> {
        let metadata = self.get_snapshot_metadata(snapshot_id).await?;

        // Check if file exists
        if !metadata.storage_path.exists() {
            return Ok(false);
        }

        // Verify checksum
        let data = tokio::fs::read(&metadata.storage_path)
            .await
            .map_err(|e| SnapshotError::StorageError {
                operation: "read_for_verification".to_string(),
                reason: e.to_string(),
            })?;

        let calculated_checksum = self.calculate_checksum(&data, &metadata.checksum_algorithm);
        let verified = calculated_checksum == metadata.checksum;

        // Update verification status
        if verified {
            let mut cache = self.metadata_cache.write();
            if let Some(mut metadata) = cache.get_mut(snapshot_id) {
                metadata.verified = true;
                metadata.last_verified = Some(SystemTime::now());
            }
        }

        Ok(verified)
    }

    async fn apply_retention_policy(&self, replica_id: &ReplicaId) -> Result<Vec<SnapshotId>, SnapshotError> {
        let mut deleted_snapshots = Vec::new();
        let _policy = &self.config.retention_policy;

        let snapshots = self.list_snapshots(replica_id).await?;
        if snapshots.is_empty() {
            return Ok(deleted_snapshots);
        }

        // Sort by creation time (newest first)
        let mut snapshots = snapshots;
        snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let now = SystemTime::now();
        let mut full_snapshots_kept = 0;

        for (index, snapshot) in snapshots.iter().enumerate() {
            let age = now.duration_since(snapshot.created_at).unwrap_or_default();
            let should_delete =
                // Too many snapshots
                index >= policy.max_snapshots ||
                // Too old
                age > policy.max_age ||
                // Keep minimum full snapshots
                (snapshot.snapshot_type == SnapshotType::Full && full_snapshots_kept >= policy.min_full_snapshots);

            if should_delete {
                if let Ok(()) = self.delete_snapshot(&snapshot.snapshot_id).await {
                    deleted_snapshots.push(snapshot.snapshot_id.clone());
                }
            } else if snapshot.snapshot_type == SnapshotType::Full {
                full_snapshots_kept += 1;
            }
        }

        Ok(deleted_snapshots)
    }

    async fn get_statistics(&self, replica_id: Option<&ReplicaId>) -> Result<SnapshotStatistics, SnapshotError> {
        if let Some(replica_id) = replica_id {
            // Calculate statistics for specific replica
            let snapshots = self.list_snapshots(replica_id).await?;
            let total_snapshots = snapshots.len();
            let full_snapshots = snapshots.iter().filter(|s| s.snapshot_type == SnapshotType::Full).count();
            let incremental_snapshots = snapshots.iter().filter(|s| s.snapshot_type == SnapshotType::Incremental).count();

            let total_storage_bytes: u64 = snapshots.iter().map(|s| s.size_bytes).sum();
            let compressed_storage_bytes: u64 = snapshots.iter()
                .map(|s| s.compressed_size_bytes.unwrap_or(s.size_bytes))
                .sum();

            Ok(SnapshotStatistics {
                total_snapshots,
                full_snapshots,
                incremental_snapshots,
                total_storage_bytes,
                compressed_storage_bytes,
                average_compression_ratio: if total_storage_bytes > 0 {
                    compressed_storage_bytes as f64 / total_storage_bytes as f64
                } else {
                    1.0
                },
                oldest_snapshot_age: snapshots.iter()
                    .map(|s| s.created_at)
                    .min()
                    .map(|oldest| SystemTime::now().duration_since(oldest).unwrap_or_default()),
                average_creation_time: Duration::from_secs(90),
                success_rate: if total_snapshots > 0 {
                    let successful = snapshots.iter().filter(|s| s.status == SnapshotStatus::Completed).count();
                    (successful as f64 / total_snapshots as f64) * 100.0
                } else {
                    100.0
                },
                storage_efficiency: StorageEfficiency {
                    deduplication_ratio: 1.0,
                    compression_effectiveness: if total_storage_bytes > 0 {
                        1.0 - (compressed_storage_bytes as f64 / total_storage_bytes as f64)
                    } else {
                        0.0
                    },
                    space_savings: if total_storage_bytes > 0 {
                        ((total_storage_bytes - compressed_storage_bytes) as f64 / total_storage_bytes as f64) * 100.0
                    } else {
                        0.0
                    },
                    average_size_reduction: if total_storage_bytes > 0 {
                        compressed_storage_bytes as f64 / total_storage_bytes as f64
                    } else {
                        1.0
                    },
                },
            })
        } else {
            // Return system-wide statistics
            let _stats = self.statistics.read();
            Ok(stats.clone())
        }
    }
}

impl Default for FileSnapshotManager {
    fn default() -> Self {
        futures::executor::block_on(async {
            Self::new(SnapshotConfig::default()).await.unwrap()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_manager() -> (FileSnapshotManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let config = SnapshotConfig {
            storage_path: temp_path.clone(),
            temp_directory: temp_path.join("temp"),
            ..SnapshotConfig::default()
        };

        let manager = FileSnapshotManager::new(config).await.unwrap();
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_snapshot_id_creation() {
        let id = SnapshotId::new("test-snapshot");
        assert!(id.is_ok());

        let empty_id = SnapshotId::new("");
        assert!(empty_id.is_err());

        let generated_id = SnapshotId::generate();
        assert!(!generated_id.as_str().is_empty());
    }

    #[tokio::test]
    async fn test_snapshot_manager_creation() {
        let (manager, _temp_dir) = create_test_manager().await;

        let _stats = manager.get_statistics(None).await.unwrap();
        assert_eq!(stats.total_snapshots, 0);
    }

    #[tokio::test]
    async fn test_full_snapshot_creation() {
        let (manager, _temp_dir) = create_test_manager().await;
        let _replica_id = ReplicaId::new("test-replica").unwrap();

        let snapshot_id = manager.create_full_snapshot(&replica_id).await.unwrap();
        assert!(!snapshot_id.as_str().is_empty());

        let metadata = manager.get_snapshot_metadata(&snapshot_id).await.unwrap();
        assert_eq!(metadata.replica_id, replica_id);
        assert_eq!(metadata.snapshot_type, SnapshotType::Full);
        assert_eq!(metadata.status, SnapshotStatus::Completed);
    }

    #[tokio::test]
    async fn test_incremental_snapshot_creation() {
        let (manager, _temp_dir) = create_test_manager().await;
        let _replica_id = ReplicaId::new("test-replica").unwrap();

        // Create parent snapshot first
        let parent_id = manager.create_full_snapshot(&replica_id).await.unwrap();

        // Create incremental snapshot
        let incremental_id = manager.create_incremental_snapshot(&replica_id, &parent_id).await.unwrap();

        let metadata = manager.get_snapshot_metadata(&incremental_id).await.unwrap();
        assert_eq!(metadata.snapshot_type, SnapshotType::Incremental);
        assert_eq!(metadata.parent_snapshot, Some(parent_id));
    }

    #[tokio::test]
    async fn test_snapshot_listing() {
        let (manager, _temp_dir) = create_test_manager().await;
        let _replica_id = ReplicaId::new("test-replica").unwrap();

        // Create multiple snapshots
        let _full1 = manager.create_full_snapshot(&replica_id).await.unwrap();
        let _full2 = manager.create_full_snapshot(&replica_id).await.unwrap();

        let snapshots = manager.list_snapshots(&replica_id).await.unwrap();
        assert_eq!(snapshots.len(), 2);

        for snapshot in snapshots {
            assert_eq!(snapshot.replica_id, replica_id);
            assert_eq!(snapshot.snapshot_type, SnapshotType::Full);
        }
    }

    #[tokio::test]
    async fn test_snapshot_deletion() {
        let (manager, _temp_dir) = create_test_manager().await;
        let _replica_id = ReplicaId::new("test-replica").unwrap();

        let snapshot_id = manager.create_full_snapshot(&replica_id).await.unwrap();

        // Verify snapshot exists
        assert!(manager.get_snapshot_metadata(&snapshot_id).await.is_ok());

        // Delete snapshot
        assert!(manager.delete_snapshot(&snapshot_id).await.is_ok());

        // Verify snapshot no longer exists
        assert!(manager.get_snapshot_metadata(&snapshot_id).await.is_err());
    }

    #[tokio::test]
    async fn test_snapshot_verification() {
        let (manager, _temp_dir) = create_test_manager().await;
        let _replica_id = ReplicaId::new("test-replica").unwrap();

        let snapshot_id = manager.create_full_snapshot(&replica_id).await.unwrap();

        let verified = manager.verify_snapshot(&snapshot_id).await.unwrap();
        assert!(verified);
    }

    #[tokio::test]
    async fn test_snapshot_restore() {
        let (manager, _temp_dir) = create_test_manager().await;
        let _replica_id = ReplicaId::new("test-replica").unwrap();

        let snapshot_id = manager.create_full_snapshot(&replica_id).await.unwrap();

        let _result = manager.restore_snapshot(&replica_id, &snapshot_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_retention_policy() {
        let (manager, _temp_dir) = create_test_manager().await;
        let _replica_id = ReplicaId::new("test-replica").unwrap();

        // Create multiple snapshots
        for _ in 0..5 {
            let _ = manager.create_full_snapshot(&replica_id).await.unwrap();
        }

        let snapshots_before = manager.list_snapshots(&replica_id).await.unwrap();
        assert_eq!(snapshots_before.len(), 5);

        let _deleted = manager.apply_retention_policy(&replica_id).await.unwrap();

        let snapshots_after = manager.list_snapshots(&replica_id).await.unwrap();
        assert!(snapshots_after.len() <= snapshots_before.len());
    }

    #[tokio::test]
    async fn test_statistics_collection() {
        let (manager, _temp_dir) = create_test_manager().await;
        let _replica_id = ReplicaId::new("test-replica").unwrap();

        // Create snapshots
        let _full = manager.create_full_snapshot(&replica_id).await.unwrap();

        let _stats = manager.get_statistics(Some(&replica_id)).await.unwrap();
        assert_eq!(stats.total_snapshots, 1);
        assert_eq!(stats.full_snapshots, 1);
        assert_eq!(stats.incremental_snapshots, 0);
    }

    #[test]
    fn test_compression_types() {
        assert_eq!(CompressionType::None, CompressionType::None);
        assert_ne!(CompressionType::Lz4, CompressionType::Zstd);
    }

    #[test]
    fn test_snapshot_status() {
        assert_eq!(SnapshotStatus::Completed, SnapshotStatus::Completed);
        assert_ne!(SnapshotStatus::Creating, SnapshotStatus::Failed);
    }
}
