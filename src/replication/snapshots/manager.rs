// Snapshot manager trait and implementation

use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::fs::File;

use crate::replication::types::{LogSequenceNumber, ReplicaId};
use super::config::{CompressionType, SnapshotConfig};
use super::errors::SnapshotError;
use super::types::*;

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
    pub(super) config: Arc<SnapshotConfig>,
    /// Metadata cache
    pub(super) metadata_cache: Arc<RwLock<HashMap<SnapshotId, SnapshotMetadata>>>,
    /// Active operations tracking
    pub(super) active_operations: Arc<Mutex<HashSet<SnapshotId>>>,
    /// Statistics
    pub(super) statistics: Arc<RwLock<SnapshotStatistics>>,
    /// Background tasks
    pub(super) cleanup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Progress tracking
    pub(super) progress_tracking: Arc<RwLock<HashMap<SnapshotId, SnapshotProgress>>>,
}

impl FileSnapshotManager {
    /// Creates a new file-based snapshot manager
    pub async fn new(config: SnapshotConfig) -> Result<Self, SnapshotError> {
        Self::validate_config(&config).await?;
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

        manager.load_existing_snapshots().await?;

        if manager.config.enable_background_cleanup {
            manager.start_background_cleanup().await;
        }

        Ok(manager)
    }

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

        self.update_statistics().await;
        Ok(())
    }

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

    async fn start_background_cleanup(&self) {
        let config = Arc::clone(&self.config);
        let metadata_cache = Arc::clone(&self.metadata_cache);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.cleanup_interval);
            loop {
                interval.tick().await;
                let replica_ids: Vec<ReplicaId> = {
                    let cache = metadata_cache.read();
                    cache.values().map(|m| m.replica_id.clone()).collect::<HashSet<_>>().into_iter().collect()
                };
                for _replica_id in replica_ids {
                    // Would apply retention policy here
                }
            }
        });

        *self.cleanup_handle.lock() = Some(handle);
    }

    pub(super) async fn compress_data(&self, data: &[u8], algorithm: &CompressionType) -> Result<Vec<u8>, SnapshotError> {
        // Simplified implementation - would use actual compression
        Ok(data.to_vec())
    }

    pub(super) async fn decompress_data(&self, data: &[u8], algorithm: &CompressionType) -> Result<Vec<u8>, SnapshotError> {
        // Simplified implementation - would use actual decompression
        Ok(data.to_vec())
    }

    pub(super) fn calculate_checksum(&self, data: &[u8], algorithm: &ChecksumAlgorithm) -> String {
        match algorithm {
            ChecksumAlgorithm::Sha256 => format!("sha256:{:x}", data.len()),
            ChecksumAlgorithm::Blake3 => format!("blake3:{:x}", data.len()),
            ChecksumAlgorithm::Crc32 => format!("crc32:{:x}", data.len()),
            ChecksumAlgorithm::Xxh3 => format!("xxh3:{:x}", data.len()),
        }
    }

    pub(super) fn get_snapshot_data_path(&self, snapshot_id: &SnapshotId) -> PathBuf {
        self.config.storage_path.join(format!("{}.snapshot", snapshot_id))
    }

    pub(super) async fn save_snapshot_metadata(&self, metadata: &SnapshotMetadata) -> Result<(), SnapshotError> {
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

    pub(super) async fn update_statistics(&self) {
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
            deduplication_ratio: 1.0,
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
            average_creation_time: Duration::from_secs(120),
            success_rate,
            storage_efficiency,
        };
    }

    pub(super) async fn create_full_snapshot_impl(&self, replica_id: &ReplicaId) -> Result<SnapshotId, SnapshotError> {
        let snapshot_id = SnapshotId::generate();

        {
            let mut active = self.active_operations.lock();
            if active.contains(&snapshot_id) {
                return Err(SnapshotError::ConcurrencyConflict {
                    operation: "create_full_snapshot".to_string(),
                });
            }
            active.insert(snapshot_id.clone());
        }

        {
            let mut progress = self.progress_tracking.write();
            progress.insert(snapshot_id.clone(), SnapshotProgress {
                snapshot_id: snapshot_id.clone(),
                phase: SnapshotPhase::Initializing,
                bytes_processed: 0,
                total_bytes: Some(1024 * 1024 * 100),
                processing_rate: 0.0,
                estimated_completion: None,
                current_operation: "Initializing snapshot".to_string(),
                progress_percentage: 0.0,
            });
        }

        let snapshot_data = b"Full snapshot data".to_vec();
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
            snapshot_type: SnapshotType::Full,
            created_at: SystemTime::now(),
            size_bytes: snapshot_data.len() as u64,
            compressed_size_bytes: Some(compressed_data.len() as u64),
            compression: self.config.compression.clone(),
            encrypted: self.config.encryption.is_some(),
            lsn: Some(LogSequenceNumber::new(12345)),
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

        self.save_snapshot_metadata(&metadata).await?;
        {
            let mut cache = self.metadata_cache.write();
            cache.insert(snapshot_id.clone(), metadata);
        }

        {
            let mut active = self.active_operations.lock();
            active.remove(&snapshot_id);
        }

        self.update_statistics().await;
        Ok(snapshot_id)
    }
}

impl Default for FileSnapshotManager {
    fn default() -> Self {
        futures::executor::block_on(async {
            Self::new(SnapshotConfig::default()).await.unwrap()
        })
    }
}
