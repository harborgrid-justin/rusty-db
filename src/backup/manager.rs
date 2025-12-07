// Backup Manager - Enterprise-grade backup orchestration
// Handles full, incremental, and differential backups with block-level change tracking

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions, create_dir_all};
use std::io::{Write, Read, BufReader, BufWriter, Seek, SeekFrom};
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::collections::{HashMap, HashSet, BTreeMap, VecDeque};
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use crate::Result;
use crate::error::DbError;

/// Backup type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BackupType {
    /// Full backup - complete copy of all data
    Full,
    /// Incremental backup - only changes since last backup
    Incremental,
    /// Differential backup - changes since last full backup
    Differential,
    /// Archive log backup - transaction logs only
    ArchiveLog,
}

/// Backup status tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupStatus {
    Pending,
    Running { progress_pct: f64 },
    Completed { duration_secs: u64 },
    Failed { error: String },
    Cancelled,
}

/// Backup metadata for tracking and cataloging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub backup_id: String,
    pub backup_type: BackupType,
    pub start_time: SystemTime,
    pub end_time: Option<SystemTime>,
    pub status: BackupStatus,
    pub size_bytes: u64,
    pub compressed_size_bytes: u64,
    pub num_files: usize,
    pub database_name: String,
    pub database_version: String,
    pub scn: u64, // System Change Number (Oracle-style)
    pub parent_backup_id: Option<String>,
    pub checkpoint_lsn: u64, // Log Sequence Number
    pub backup_path: PathBuf,
    pub encryption_enabled: bool,
    pub compression_enabled: bool,
    pub compression_ratio: f64,
    pub tags: Vec<String>,
    pub retention_until: Option<SystemTime>,
    pub block_change_map: Option<BlockChangeMap>,
}

impl BackupMetadata {
    pub fn new(backup_id: String, backup_type: BackupType, database_name: String) -> Self {
        Self {
            backup_id,
            backup_type,
            start_time: SystemTime::now(),
            end_time: None,
            status: BackupStatus::Pending,
            size_bytes: 0,
            compressed_size_bytes: 0,
            num_files: 0,
            database_name,
            database_version: env!("CARGO_PKG_VERSION").to_string(),
            scn: 0,
            parent_backup_id: None,
            checkpoint_lsn: 0,
            backup_path: PathBuf::new(),
            encryption_enabled: false,
            compression_enabled: false,
            compression_ratio: 1.0,
            tags: Vec::new(),
            retention_until: None,
            block_change_map: None,
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        self.end_time.and_then(|end| end.duration_since(self.start_time).ok())
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.status, BackupStatus::Completed { .. })
    }

    pub fn is_expired(&self) -> bool {
        if let Some(retention_until) = self.retention_until {
            SystemTime::now() > retention_until
        } else {
            false
        }
    }
}

/// Block-level change tracking for incremental backups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockChangeMap {
    /// Bitmap tracking which blocks have changed
    changed_blocks: HashMap<u64, BlockChangeInfo>,
    /// Starting SCN for this change map
    start_scn: u64,
    /// Ending SCN for this change map
    end_scn: u64,
    /// Total number of blocks tracked
    total_blocks: u64,
    /// Number of changed blocks
    changed_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockChangeInfo {
    pub block_id: u64,
    pub file_id: u32,
    pub change_scn: u64,
    pub operation: BlockOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockOperation {
    Insert,
    Update,
    Delete,
    Split,
    Merge,
}

impl BlockChangeMap {
    pub fn new(start_scn: u64, total_blocks: u64) -> Self {
        Self {
            changed_blocks: HashMap::new(),
            start_scn,
            end_scn: start_scn,
            total_blocks,
            changed_count: 0,
        }
    }

    pub fn mark_block_changed(&mut self, block_id: u64, file_id: u32, scn: u64, operation: BlockOperation) {
        if !self.changed_blocks.contains_key(&block_id) {
            self.changed_count += 1;
        }
        self.changed_blocks.insert(block_id, BlockChangeInfo {
            block_id,
            file_id,
            change_scn: scn,
            operation,
        });
        self.end_scn = scn.max(self.end_scn);
    }

    pub fn is_block_changed(&self, block_id: u64) -> bool {
        self.changed_blocks.contains_key(&block_id)
    }

    pub fn get_changed_blocks(&self) -> Vec<&BlockChangeInfo> {
        self.changed_blocks.values().collect()
    }

    pub fn change_percentage(&self) -> f64 {
        if self.total_blocks == 0 {
            0.0
        } else {
            (self.changed_count as f64 / self.total_blocks as f64) * 100.0
        }
    }

    pub fn merge(&mut self, other: &BlockChangeMap) {
        for (block_id, info) in &other.changed_blocks {
            if !self.changed_blocks.contains_key(block_id) {
                self.changed_count += 1;
            }
            self.changed_blocks.insert(*block_id, info.clone());
        }
        self.end_scn = self.end_scn.max(other.end_scn);
    }
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub backup_dir: PathBuf,
    pub max_parallel_streams: usize,
    pub buffer_size: usize,
    pub compression_enabled: bool,
    pub compression_level: u32,
    pub encryption_enabled: bool,
    pub verify_after_backup: bool,
    pub block_size: usize,
    pub enable_change_tracking: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            backup_dir: PathBuf::from("/var/backups/rustydb"),
            max_parallel_streams: 4,
            buffer_size: 1024 * 1024, // 1MB
            compression_enabled: true,
            compression_level: 6,
            encryption_enabled: false,
            verify_after_backup: true,
            block_size: 8192,
            enable_change_tracking: true,
        }
    }
}

/// Retention policy for backup lifecycle management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub keep_hourly: usize,
    pub keep_daily: usize,
    pub keep_weekly: usize,
    pub keep_monthly: usize,
    pub keep_yearly: usize,
    pub max_backups: Option<usize>,
    pub max_age_days: Option<u64>,
    pub min_free_space_gb: Option<u64>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            keep_hourly: 24,
            keep_daily: 7,
            keep_weekly: 4,
            keep_monthly: 12,
            keep_yearly: 5,
            max_backups: Some(100),
            max_age_days: Some(365),
            min_free_space_gb: Some(10),
        }
    }
}

impl RetentionPolicy {
    pub fn should_keep(&self, backup: &BackupMetadata, existing_backups: &[BackupMetadata]) -> bool {
        // Check age
        if let Some(max_age_days) = self.max_age_days {
            if let Ok(age) = SystemTime::now().duration_since(backup.start_time) {
                if age.as_secs() > max_age_days * 86400 {
                    return false;
                }
            }
        }

        // Check retention until time
        if backup.is_expired() {
            return false;
        }

        // Count backups in different time windows
        let now = SystemTime::now();
        let backup_age = now.duration_since(backup.start_time).unwrap_or_default();

        // Keep based on time window
        if backup_age.as_secs() < 3600 * self.keep_hourly as u64 {
            return true;
        }
        if backup_age.as_secs() < 86400 * self.keep_daily as u64 {
            return true;
        }
        if backup_age.as_secs() < 86400 * 7 * self.keep_weekly as u64 {
            return true;
        }
        if backup_age.as_secs() < 86400 * 30 * self.keep_monthly as u64 {
            return true;
        }
        if backup_age.as_secs() < 86400 * 365 * self.keep_yearly as u64 {
            return true;
        }

        // Check max backups limit
        if let Some(max_backups) = self.max_backups {
            if existing_backups.len() >= max_backups {
                return false;
            }
        }

        false
    }

    pub fn apply(&self, backups: &mut Vec<BackupMetadata>) {
        backups.retain(|backup| self.should_keep(backup, backups));
    }
}

/// Main backup manager orchestrating all backup operations
pub struct BackupManager {
    config: BackupConfig,
    retention_policy: RetentionPolicy,
    backups: Arc<RwLock<BTreeMap<String, BackupMetadata>>>,
    change_maps: Arc<RwLock<HashMap<String, BlockChangeMap>>>,
    current_scn: Arc<Mutex<u64>>,
    active_backups: Arc<RwLock<HashSet<String>>>,
}

impl BackupManager {
    pub fn new(config: BackupConfig, retention_policy: RetentionPolicy) -> Result<Self> {
        create_dir_all(&config.backup_dir).map_err(|e| {
            DbError::BackupError(format!("Failed to create backup directory: {}", e))
        })?;

        Ok(Self {
            config,
            retention_policy,
            backups: Arc::new(RwLock::new(BTreeMap::new())),
            change_maps: Arc::new(RwLock::new(HashMap::new())),
            current_scn: Arc::new(Mutex::new(1)),
            active_backups: Arc::new(RwLock::new(HashSet::new())),
        })
    }

    /// Generate a new SCN (System Change Number)
    pub fn generate_scn(&self) -> u64 {
        let mut scn = self.current_scn.lock();
        *scn += 1;
        *scn
    }

    /// Create a full backup
    pub fn create_full_backup(&self, database_name: &str) -> Result<String> {
        let backup_id = self.generate_backup_id("FULL");
        let mut metadata = BackupMetadata::new(
            backup_id.clone(),
            BackupType::Full,
            database_name.to_string(),
        );

        metadata.scn = self.generate_scn();
        metadata.backup_path = self.config.backup_dir.join(&backup_id);

        // Mark as active
        self.active_backups.write().insert(backup_id.clone());

        // Create backup directory
        create_dir_all(&metadata.backup_path).map_err(|e| {
            DbError::BackupError(format!("Failed to create backup path: {}", e))
        })?;

        // Initialize block change tracking for future incrementals
        if self.config.enable_change_tracking {
            let change_map = BlockChangeMap::new(metadata.scn, 10000); // Assume 10000 blocks
            self.change_maps.write().insert(backup_id.clone(), change_map);
        }

        // Update status
        metadata.status = BackupStatus::Running { progress_pct: 0.0 };

        // Simulate backup process
        self.perform_full_backup(&mut metadata)?;

        // Complete backup
        metadata.end_time = Some(SystemTime::now());
        metadata.status = BackupStatus::Completed {
            duration_secs: metadata.duration().unwrap_or_default().as_secs(),
        };

        // Store metadata
        self.backups.write().insert(backup_id.clone(), metadata);
        self.active_backups.write().remove(&backup_id);

        Ok(backup_id)
    }

    /// Create an incremental backup
    pub fn create_incremental_backup(&self, database_name: &str, parent_backup_id: &str) -> Result<String> {
        // Verify parent backup exists
        let parent = self.backups.read().get(parent_backup_id).cloned()
            .ok_or_else(|| DbError::BackupError("Parent backup not found".to_string()))?;

        let backup_id = self.generate_backup_id("INCR");
        let mut metadata = BackupMetadata::new(
            backup_id.clone(),
            BackupType::Incremental,
            database_name.to_string(),
        );

        metadata.scn = self.generate_scn();
        metadata.parent_backup_id = Some(parent_backup_id.to_string());
        metadata.backup_path = self.config.backup_dir.join(&backup_id);

        // Mark as active
        self.active_backups.write().insert(backup_id.clone());

        create_dir_all(&metadata.backup_path).map_err(|e| {
            DbError::BackupError(format!("Failed to create backup path: {}", e))
        })?;

        metadata.status = BackupStatus::Running { progress_pct: 0.0 };

        // Perform incremental backup using change tracking
        self.perform_incremental_backup(&mut metadata, &parent)?;

        metadata.end_time = Some(SystemTime::now());
        metadata.status = BackupStatus::Completed {
            duration_secs: metadata.duration().unwrap_or_default().as_secs(),
        };

        self.backups.write().insert(backup_id.clone(), metadata);
        self.active_backups.write().remove(&backup_id);

        Ok(backup_id)
    }

    /// Create a differential backup
    pub fn create_differential_backup(&self, database_name: &str, base_backup_id: &str) -> Result<String> {
        // Verify base backup exists and is a full backup
        let base = self.backups.read().get(base_backup_id).cloned()
            .ok_or_else(|| DbError::BackupError("Base backup not found".to_string()))?;

        if base.backup_type != BackupType::Full {
            return Err(DbError::BackupError("Base backup must be a full backup".to_string()));
        }

        let backup_id = self.generate_backup_id("DIFF");
        let mut metadata = BackupMetadata::new(
            backup_id.clone(),
            BackupType::Differential,
            database_name.to_string(),
        );

        metadata.scn = self.generate_scn();
        metadata.parent_backup_id = Some(base_backup_id.to_string());
        metadata.backup_path = self.config.backup_dir.join(&backup_id);

        self.active_backups.write().insert(backup_id.clone());

        create_dir_all(&metadata.backup_path).map_err(|e| {
            DbError::BackupError(format!("Failed to create backup path: {}", e))
        })?;

        metadata.status = BackupStatus::Running { progress_pct: 0.0 };

        // Perform differential backup
        self.perform_differential_backup(&mut metadata, &base)?;

        metadata.end_time = Some(SystemTime::now());
        metadata.status = BackupStatus::Completed {
            duration_secs: metadata.duration().unwrap_or_default().as_secs(),
        };

        self.backups.write().insert(backup_id.clone(), metadata);
        self.active_backups.write().remove(&backup_id);

        Ok(backup_id)
    }

    fn perform_full_backup(&self, metadata: &mut BackupMetadata) -> Result<()> {
        // Simulate backing up all data files
        let mut total_size = 0u64;
        let num_files = 10; // Simulate 10 data files

        for i in 0..num_files {
            let file_size = 1024 * 1024 * 100; // 100MB per file
            total_size += file_size;

            // Update progress
            metadata.status = BackupStatus::Running {
                progress_pct: ((i + 1) as f64 / num_files as f64) * 100.0,
            };

            // Simulate block-level tracking
            if let Some(change_map) = self.change_maps.write().get_mut(&metadata.backup_id) {
                for block_id in (i * 1000)..((i + 1) * 1000) {
                    change_map.mark_block_changed(
                        block_id as u64,
                        i as u32,
                        metadata.scn,
                        BlockOperation::Insert,
                    );
                }
            }
        }

        metadata.size_bytes = total_size;
        metadata.compressed_size_bytes = if self.config.compression_enabled {
            (total_size as f64 * 0.6) as u64 // Simulate 40% compression
        } else {
            total_size
        };
        metadata.compression_ratio = metadata.size_bytes as f64 / metadata.compressed_size_bytes as f64;
        metadata.num_files = num_files;
        metadata.encryption_enabled = self.config.encryption_enabled;
        metadata.compression_enabled = self.config.compression_enabled;

        Ok(())
    }

    fn perform_incremental_backup(&self, metadata: &mut BackupMetadata, parent: &BackupMetadata) -> Result<()> {
        // Get changed blocks since parent
        let changed_blocks = if let Some(parent_map) = parent.block_change_map.as_ref() {
            parent_map.get_changed_blocks()
        } else {
            vec![]
        };

        // Simulate backing up only changed blocks
        let num_changed = changed_blocks.len().max(100); // At least 100 blocks changed
        let block_size = self.config.block_size as u64;
        let total_size = num_changed as u64 * block_size;

        metadata.size_bytes = total_size;
        metadata.compressed_size_bytes = if self.config.compression_enabled {
            (total_size as f64 * 0.7) as u64 // Incremental compresses better
        } else {
            total_size
        };
        metadata.compression_ratio = metadata.size_bytes as f64 / metadata.compressed_size_bytes as f64;
        metadata.num_files = 1; // Single incremental file
        metadata.encryption_enabled = self.config.encryption_enabled;
        metadata.compression_enabled = self.config.compression_enabled;

        // Create change map for this backup
        let mut change_map = BlockChangeMap::new(parent.scn, 10000);
        for i in 0..num_changed {
            change_map.mark_block_changed(
                i as u64,
                0,
                metadata.scn,
                BlockOperation::Update,
            );
        }
        metadata.block_change_map = Some(change_map);

        Ok(())
    }

    fn perform_differential_backup(&self, metadata: &mut BackupMetadata, base: &BackupMetadata) -> Result<()> {
        // Differential backup includes all changes since base full backup
        // This is more data than incremental but fewer backups needed for restore

        let base_map = base.block_change_map.as_ref()
            .ok_or_else(|| DbError::BackupError("Base backup has no change map".to_string()))?;

        // Accumulate all changes since base
        let mut accumulated_changes = base_map.clone();

        // Find all incremental backups since base and merge their changes
        let backups = self.backups.read();
        for (_, backup) in backups.iter() {
            if let Some(parent_id) = &backup.parent_backup_id {
                if parent_id == &base.backup_id {
                    if let Some(change_map) = &backup.block_change_map {
                        accumulated_changes.merge(change_map);
                    }
                }
            }
        }

        let num_changed = accumulated_changes.changed_count as usize;
        let block_size = self.config.block_size as u64;
        let total_size = num_changed as u64 * block_size;

        metadata.size_bytes = total_size;
        metadata.compressed_size_bytes = if self.config.compression_enabled {
            (total_size as f64 * 0.65) as u64
        } else {
            total_size
        };
        metadata.compression_ratio = metadata.size_bytes as f64 / metadata.compressed_size_bytes as f64;
        metadata.num_files = 1;
        metadata.encryption_enabled = self.config.encryption_enabled;
        metadata.compression_enabled = self.config.compression_enabled;
        metadata.block_change_map = Some(accumulated_changes);

        Ok(())
    }

    /// Apply retention policy and remove obsolete backups
    pub fn apply_retention_policy(&self) -> Result<Vec<String>> {
        let mut backups = self.backups.write();
        let mut to_remove = Vec::new();

        let backup_list: Vec<BackupMetadata> = backups.values().cloned().collect();

        for (backup_id, backup) in backups.iter() {
            if !self.retention_policy.should_keep(backup, &backup_list) {
                to_remove.push(backup_id.clone());
            }
        }

        // Remove obsolete backups
        for backup_id in &to_remove {
            if let Some(backup) = backups.remove(backup_id) {
                // Delete backup files
                if backup.backup_path.exists() {
                    std::fs::remove_dir_all(&backup.backup_path).ok();
                }
            }
            // Remove change map
            self.change_maps.write().remove(backup_id);
        }

        Ok(to_remove)
    }

    /// Get backup metadata
    pub fn get_backup(&self, backup_id: &str) -> Option<BackupMetadata> {
        self.backups.read().get(backup_id).cloned()
    }

    /// List all backups
    pub fn list_backups(&self) -> Vec<BackupMetadata> {
        self.backups.read().values().cloned().collect()
    }

    /// List backups by type
    pub fn list_backups_by_type(&self, backup_type: BackupType) -> Vec<BackupMetadata> {
        self.backups.read()
            .values()
            .filter(|b| b.backup_type == backup_type)
            .cloned()
            .collect()
    }

    /// Cancel an active backup
    pub fn cancel_backup(&self, backup_id: &str) -> Result<()> {
        if !self.active_backups.read().contains(backup_id) {
            return Err(DbError::BackupError("Backup not active".to_string()));
        }

        // Update status
        if let Some(backup) = self.backups.write().get_mut(backup_id) {
            backup.status = BackupStatus::Cancelled;
            backup.end_time = Some(SystemTime::now());
        }

        self.active_backups.write().remove(backup_id);
        Ok(())
    }

    /// Get backup statistics
    pub fn get_statistics(&self) -> BackupStatistics {
        let backups = self.backups.read();
        let total_backups = backups.len();
        let mut total_size = 0u64;
        let mut total_compressed_size = 0u64;
        let mut by_type = HashMap::new();

        for backup in backups.values() {
            total_size += backup.size_bytes;
            total_compressed_size += backup.compressed_size_bytes;
            *by_type.entry(backup.backup_type.clone()).or_insert(0) += 1;
        }

        BackupStatistics {
            total_backups,
            total_size_bytes: total_size,
            total_compressed_size_bytes: total_compressed_size,
            compression_ratio: if total_compressed_size > 0 {
                total_size as f64 / total_compressed_size as f64
            } else {
                1.0
            },
            backups_by_type: by_type,
            active_backups: self.active_backups.read().len(),
        }
    }

    fn generate_backup_id(&self, prefix: &str) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("{}-{}", prefix, timestamp)
    }
}

/// Backup statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStatistics {
    pub total_backups: usize,
    pub total_size_bytes: u64,
    pub total_compressed_size_bytes: u64,
    pub compression_ratio: f64,
    pub backups_by_type: HashMap<BackupType, usize>,
    pub active_backups: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_manager_full_backup() {
        let config = BackupConfig::default();
        let retention = RetentionPolicy::default();
        let manager = BackupManager::new(config, retention).unwrap();

        let backup_id = manager.create_full_backup("testdb").unwrap();
        assert!(!backup_id.is_empty());

        let backup = manager.get_backup(&backup_id).unwrap();
        assert_eq!(backup.backup_type, BackupType::Full);
        assert!(backup.is_complete());
    }

    #[test]
    fn test_block_change_map() {
        let mut change_map = BlockChangeMap::new(100, 1000);
        change_map.mark_block_changed(1, 0, 101, BlockOperation::Insert);
        change_map.mark_block_changed(2, 0, 102, BlockOperation::Update);

        assert!(change_map.is_block_changed(1));
        assert!(change_map.is_block_changed(2));
        assert!(!change_map.is_block_changed(3));
        assert_eq!(change_map.changed_count, 2);
        assert_eq!(change_map.change_percentage(), 0.2);
    }

    #[test]
    fn test_retention_policy() {
        let mut policy = RetentionPolicy::default();
        policy.max_age_days = Some(1);

        let old_backup = BackupMetadata {
            backup_id: "old".to_string(),
            backup_type: BackupType::Full,
            start_time: SystemTime::now() - Duration::from_secs(2 * 86400),
            end_time: Some(SystemTime::now() - Duration::from_secs(2 * 86400)),
            status: BackupStatus::Completed { duration_secs: 100 },
            size_bytes: 1000,
            compressed_size_bytes: 600,
            num_files: 1,
            database_name: "test".to_string(),
            database_version: "1.0".to_string(),
            scn: 100,
            parent_backup_id: None,
            checkpoint_lsn: 100,
            backup_path: PathBuf::from("/tmp/old"),
            encryption_enabled: false,
            compression_enabled: true,
            compression_ratio: 1.67,
            tags: vec![],
            retention_until: None,
            block_change_map: None,
        };

        assert!(!policy.should_keep(&old_backup, &vec![]));
    }
}
