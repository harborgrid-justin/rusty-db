// Snapshot Management - Copy-on-write snapshots with cloning and scheduling
// Provides space-efficient storage snapshots for testing and backup

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs::{create_dir_all};
use std::time::{SystemTime};
use std::collections::{HashMap};
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use crate::Result;
use crate::error::DbError;

/// Snapshot metadata
#[repr(C)]
#[repr(align(64))] // Cache-line aligned for hot-path performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub snapshot_id: String,
    pub snapshot_name: String,
    pub parent_snapshot_id: Option<String>,
    pub creation_time: SystemTime,
    pub database_name: String,
    pub scn: u64,
    pub snapshot_type: SnapshotType,
    pub status: SnapshotStatus,
    pub size_bytes: u64,
    pub referenced_bytes: u64,
    pub unique_bytes: u64,
    pub compression_enabled: bool,
    pub readonly: bool,
    pub clones: Vec<String>,
    pub retention_policy: SnapshotRetention,
    pub tags: HashMap<String, String>,
    pub metadata_path: PathBuf,
}

impl Snapshot {
    #[inline]
    pub fn new(snapshot_id: String, snapshot_name: String, database_name: String, scn: u64) -> Self {
        Self {
            snapshot_id,
            snapshot_name,
            parent_snapshot_id: None,
            creation_time: SystemTime::now(),
            database_name,
            scn,
            snapshot_type: SnapshotType::Manual,
            status: SnapshotStatus::Creating,
            size_bytes: 0,
            referenced_bytes: 0,
            unique_bytes: 0,
            compression_enabled: false,
            readonly: true,
            clones: Vec::new(),
            retention_policy: SnapshotRetention::default(),
            tags: HashMap::new(),
            metadata_path: PathBuf::new(),
        }
    }

    #[inline]
    pub fn deduplication_ratio(&self) -> f64 {
        if self.size_bytes == 0 {
            1.0
        } else {
            self.referenced_bytes as f64 / self.size_bytes as f64
        }
    }

    #[inline]
    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = self.retention_policy.expires_at {
            SystemTime::now() > expiry
        } else {
            false
        }
    }
}

/// Snapshot type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SnapshotType {
    Manual,
    Scheduled,
    BeforeOperation,
    Automatic,
}

/// Snapshot status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SnapshotStatus {
    Creating,
    Available,
    Deleting,
    Error { message: String },
}

/// Snapshot retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRetention {
    pub max_age_hours: Option<u64>,
    pub expires_at: Option<SystemTime>,
    pub keep_minimum: usize,
    pub auto_delete: bool,
}

impl Default for SnapshotRetention {
    fn default() -> Self {
        Self {
            max_age_hours: Some(24),
            expires_at: None,
            keep_minimum: 1,
            auto_delete: false,
        }
    }
}

/// Copy-on-write block tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CowBlock {
    pub block_id: u64,
    pub file_id: u32,
    pub original_data: Vec<u8>,
    pub snapshot_id: String,
    pub written_at: SystemTime,
}

/// Copy-on-write tracker for snapshots
pub struct CowTracker {
    blocks: Arc<RwLock<HashMap<(u32, u64), CowBlock>>>,
    snapshot_id: String,
}

impl CowTracker {
    pub fn new(snapshot_id: String) -> Self {
        Self {
            blocks: Arc::new(RwLock::new(HashMap::new())),
            snapshot_id,
        }
    }

    /// Record original block data before write (copy-on-write)
    pub fn record_block(&self, file_id: u32, block_id: u64, data: Vec<u8>) {
        let key = (file_id, block_id);
        let mut blocks = self.blocks.write();

        if !blocks.contains_key(&key) {
            let cow_block = CowBlock {
                block_id,
                file_id,
                original_data: data,
                snapshot_id: self.snapshot_id.clone(),
                written_at: SystemTime::now(),
            };
            blocks.insert(key, cow_block);
        }
    }

    /// Get original block data
    pub fn get_block(&self, file_id: u32, block_id: u64) -> Option<Vec<u8>> {
        let blocks = self.blocks.read();
        blocks.get(&(file_id, block_id)).map(|b| b.original_data.clone())
    }

    /// Get total size of COW blocks
    pub fn get_size(&self) -> u64 {
        let blocks = self.blocks.read();
        blocks.values().map(|b| b.original_data.len() as u64).sum()
    }

    /// Get number of COW blocks
    pub fn get_block_count(&self) -> usize {
        self.blocks.read().len()
    }
}

/// Snapshot clone for creating writable copies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotClone {
    pub clone_id: String,
    pub clone_name: String,
    pub parent_snapshot_id: String,
    pub creation_time: SystemTime,
    pub writable: bool,
    pub size_bytes: u64,
    pub cow_tracker: Option<String>,
    pub purpose: ClonePurpose,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClonePurpose {
    Testing,
    Development,
    Reporting,
    Backup,
    Custom(String),
}

/// Snapshot schedule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSchedule {
    pub schedule_id: String,
    pub name: String,
    pub frequency: SnapshotFrequency,
    pub retention: SnapshotRetention,
    pub enabled: bool,
    pub last_execution: Option<SystemTime>,
    pub next_execution: Option<SystemTime>,
    pub databases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotFrequency {
    Hourly,
    Daily { hour: u8 },
    Weekly { day: u8, hour: u8 },
    Monthly { day: u8, hour: u8 },
    Custom { cron_expression: String },
}

impl SnapshotSchedule {
    pub fn new(schedule_id: String, name: String, frequency: SnapshotFrequency) -> Self {
        Self {
            schedule_id,
            name,
            frequency,
            retention: SnapshotRetention::default(),
            enabled: true,
            last_execution: None,
            next_execution: None,
            databases: Vec::new(),
        }
    }

    pub fn is_due(&self) -> bool {
        if !self.enabled {
            return false;
        }

        if let Some(next_execution) = self.next_execution {
            SystemTime::now() >= next_execution
        } else {
            true // Never executed, so it's due
        }
    }

    pub fn calculate_next_execution(&mut self) {
        let now = SystemTime::now();

        self.next_execution = match &self.frequency {
            SnapshotFrequency::Hourly => {
                Some(now + Duration::from_secs(3600))
            }
            SnapshotFrequency::Daily { hour: _ } => {
                Some(now + Duration::from_secs(86400))
            }
            SnapshotFrequency::Weekly { day: _, hour: _ } => {
                Some(now + Duration::from_secs(7 * 86400))
            }
            SnapshotFrequency::Monthly { day: _, hour: _ } => {
                Some(now + Duration::from_secs(30 * 86400))
            }
            SnapshotFrequency::Custom { cron_expression: _ } => {
                // In a real implementation, parse cron expression
                Some(now + Duration::from_secs(3600))
            }
        };
    }
}

/// Snapshot manager
pub struct SnapshotManager {
    snapshots: Arc<RwLock<BTreeMap<String, Snapshot>>>,
    clones: Arc<RwLock<HashMap<String, SnapshotClone>>>,
    cow_trackers: Arc<RwLock<HashMap<String, CowTracker>>>,
    schedules: Arc<RwLock<HashMap<String, SnapshotSchedule>>>,
    snapshot_dir: PathBuf,
    scn_counter: Arc<Mutex<u64>>,
}

impl SnapshotManager {
    pub fn new(snapshot_dir: PathBuf) -> Result<Self> {
        create_dir_all(&snapshot_dir).map_err(|e| {
            DbError::BackupError(format!("Failed to create snapshot directory: {}", e))
        })?;

        Ok(Self {
            snapshots: Arc::new(RwLock::new(BTreeMap::new())),
            clones: Arc::new(RwLock::new(HashMap::new())),
            cow_trackers: Arc::new(RwLock::new(HashMap::new())),
            schedules: Arc::new(RwLock::new(HashMap::new())),
            snapshot_dir,
            scn_counter: Arc::new(Mutex::new(1000)),
        })
    }

    /// Create a new snapshot
    pub fn create_snapshot(
        &self,
        snapshot_name: String,
        database_name: String,
        snapshot_type: SnapshotType,
    ) -> Result<String> {
        let snapshot_id = self.generate_snapshot_id();
        let scn = self.generate_scn();

        let mut snapshot = Snapshot::new(
            snapshot_id.clone(),
            snapshot_name,
            database_name,
            scn,
        );

        snapshot.snapshot_type = snapshot_type;
        snapshot.metadata_path = self.snapshot_dir.join(&snapshot_id).join("metadata.json");

        // Create snapshot directory
        let snapshot_path = self.snapshot_dir.join(&snapshot_id);
        create_dir_all(&snapshot_path).map_err(|e| {
            DbError::BackupError(format!("Failed to create snapshot directory: {}", e))
        })?;

        // Initialize COW tracker
        let cow_tracker = CowTracker::new(snapshot_id.clone());
        self.cow_trackers.write().insert(snapshot_id.clone(), cow_tracker);

        // Simulate snapshot creation
        self.perform_snapshot(&mut snapshot)?;

        snapshot.status = SnapshotStatus::Available;

        // Store snapshot
        self.snapshots.write().insert(snapshot_id.clone(), snapshot);

        Ok(snapshot_id)
    }

    fn perform_snapshot(&self, snapshot: &mut Snapshot) -> Result<()> {
        // Simulate taking a snapshot by capturing metadata
        // In a real implementation, this would:
        // 1. Freeze database state
        // 2. Record current block locations
        // 3. Setup COW tracking for future writes

        let total_size = 1024 * 1024 * 1024 * 10; // 10GB database
        snapshot.size_bytes = total_size;
        snapshot.referenced_bytes = total_size;
        snapshot.unique_bytes = 0; // No unique data yet, all shared

        Ok(())
    }

    /// Clone a snapshot for testing or development
    pub fn clone_snapshot(
        &self,
        parent_snapshot_id: &str,
        clone_name: String,
        writable: bool,
        purpose: ClonePurpose,
    ) -> Result<String> {
        // Verify parent snapshot exists
        let _parent = self.snapshots.read().get(parent_snapshot_id).cloned()
            .ok_or_else(|| DbError::BackupError("Parent snapshot not found".to_string()))?;

        let clone_id = self.generate_clone_id();

        let clone = SnapshotClone {
            clone_id: clone_id.clone(),
            clone_name,
            parent_snapshot_id: parent_snapshot_id.to_string(),
            creation_time: SystemTime::now(),
            writable,
            size_bytes: 0, // Clone starts with 0 unique bytes
            cow_tracker: if writable {
                Some(clone_id.clone())
            } else {
                None
            },
            purpose,
        };

        // If writable, create COW tracker for the clone
        if writable {
            let cow_tracker = CowTracker::new(clone_id.clone());
            self.cow_trackers.write().insert(clone_id.clone(), cow_tracker);
        }

        // Add clone to parent snapshot
        if let Some(parent) = self.snapshots.write().get_mut(parent_snapshot_id) {
            parent.clones.push(clone_id.clone());
        }

        self.clones.write().insert(clone_id.clone(), clone);

        Ok(clone_id)
    }

    /// Delete a snapshot
    pub fn delete_snapshot(&self, snapshot_id: &str) -> Result<()> {
        // Check if snapshot has clones
        let snapshot = self.snapshots.read().get(snapshot_id).cloned()
            .ok_or_else(|| DbError::BackupError("Snapshot not found".to_string()))?;

        if !snapshot.clones.is_empty() {
            return Err(DbError::BackupError(
                format!("Cannot delete snapshot with {} active clones", snapshot.clones.len())
            ));
        }

        // Remove COW tracker
        self.cow_trackers.write().remove(snapshot_id);

        // Delete snapshot files
        let snapshot_path = self.snapshot_dir.join(snapshot_id);
        if snapshot_path.exists() {
            std::fs::remove_dir_all(&snapshot_path).map_err(|e| {
                DbError::BackupError(format!("Failed to delete snapshot: {}", e))
            })?;
        }

        // Remove from snapshots map
        self.snapshots.write().remove(snapshot_id);

        Ok(())
    }

    /// Delete a clone
    pub fn delete_clone(&self, clone_id: &str) -> Result<()> {
        let clone = self.clones.read().get(clone_id).cloned()
            .ok_or_else(|| DbError::BackupError("Clone not found".to_string()))?;

        // Remove from parent's clone list
        if let Some(parent) = self.snapshots.write().get_mut(&clone.parent_snapshot_id) {
            parent.clones.retain(|id| id != clone_id);
        }

        // Remove COW tracker
        if clone.writable {
            self.cow_trackers.write().remove(clone_id);
        }

        self.clones.write().remove(clone_id);

        Ok(())
    }

    /// Add a snapshot schedule
    pub fn add_schedule(&self, schedule: SnapshotSchedule) -> Result<()> {
        let schedule_id = schedule.schedule_id.clone();
        self.schedules.write().insert(schedule_id, schedule);
        Ok(())
    }

    /// Remove a snapshot schedule
    pub fn remove_schedule(&self, schedule_id: &str) -> Result<()> {
        self.schedules.write().remove(schedule_id)
            .ok_or_else(|| DbError::BackupError("Schedule not found".to_string()))?;
        Ok(())
    }

    /// Execute due scheduled snapshots
    pub fn execute_schedules(&self) -> Result<Vec<String>> {
        let mut created_snapshots = Vec::new();
        let mut schedules = self.schedules.write();

        for (_schedule_id, schedule) in schedules.iter_mut() {
            if schedule.is_due() {
                // Create snapshot for each database in the schedule
                for database_name in &schedule.databases {
                    let snapshot_name = format!("{}-{}", schedule.name,
                        SystemTime::now().duration_since(UNIX_EPOCH)
                            .unwrap_or_default().as_secs());

                    let snapshot_id = self.create_snapshot(
                        snapshot_name,
                        database_name.clone(),
                        SnapshotType::Scheduled,
                    )?;

                    created_snapshots.push(snapshot_id);
                }

                schedule.last_execution = Some(SystemTime::now());
                schedule.calculate_next_execution();
            }
        }

        Ok(created_snapshots)
    }

    /// Get snapshot by ID
    pub fn get_snapshot(&self, snapshot_id: &str) -> Option<Snapshot> {
        self.snapshots.read().get(snapshot_id).cloned()
    }

    /// List all snapshots
    pub fn list_snapshots(&self) -> Vec<Snapshot> {
        self.snapshots.read().values().cloned().collect()
    }

    /// List snapshots for a specific database
    pub fn list_snapshots_for_database(&self, database_name: &str) -> Vec<Snapshot> {
        self.snapshots.read()
            .values()
            .filter(|s| s.database_name == database_name)
            .cloned()
            .collect()
    }

    /// List all clones
    pub fn list_clones(&self) -> Vec<SnapshotClone> {
        self.clones.read().values().cloned().collect()
    }

    /// Get clone by ID
    pub fn get_clone(&self, clone_id: &str) -> Option<SnapshotClone> {
        self.clones.read().get(clone_id).cloned()
    }

    /// Apply retention policies and cleanup old snapshots
    pub fn apply_retention_policies(&self) -> Result<Vec<String>> {
        let mut deleted_snapshots = Vec::new();
        let snapshots: Vec<Snapshot> = self.snapshots.read().values().cloned().collect();

        for snapshot in snapshots {
            if snapshot.is_expired() && snapshot.clones.is_empty() {
                if snapshot.retention_policy.auto_delete {
                    self.delete_snapshot(&snapshot.snapshot_id)?;
                    deleted_snapshots.push(snapshot.snapshot_id);
                }
            }
        }

        Ok(deleted_snapshots)
    }

    /// Get snapshot storage statistics
    pub fn get_storage_statistics(&self) -> SnapshotStatistics {
        let snapshots = self.snapshots.read();
        let clones = self.clones.read();

        let total_snapshots = snapshots.len();
        let total_clones = clones.len();
        let mut total_size_bytes = 0u64;
        let mut total_unique_bytes = 0u64;
        let mut total_referenced_bytes = 0u64;

        for snapshot in snapshots.values() {
            total_size_bytes += snapshot.size_bytes;
            total_unique_bytes += snapshot.unique_bytes;
            total_referenced_bytes += snapshot.referenced_bytes;
        }

        SnapshotStatistics {
            total_snapshots,
            total_clones,
            total_size_bytes,
            total_unique_bytes,
            total_referenced_bytes,
            deduplication_ratio: if total_size_bytes > 0 {
                total_referenced_bytes as f64 / total_size_bytes as f64
            } else {
                1.0
            },
            space_savings_bytes: total_referenced_bytes.saturating_sub(total_unique_bytes),
        }
    }

    fn generate_snapshot_id(&self) -> String {
        format!("SNAP-{}", uuid::Uuid::new_v4())
    }

    fn generate_clone_id(&self) -> String {
        format!("CLONE-{}", uuid::Uuid::new_v4())
    }

    fn generate_scn(&self) -> u64 {
        let mut scn = self.scn_counter.lock();
        *scn += 1;
        *scn
    }
}

/// Snapshot storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotStatistics {
    pub total_snapshots: usize,
    pub total_clones: usize,
    pub total_size_bytes: u64,
    pub total_unique_bytes: u64,
    pub total_referenced_bytes: u64,
    pub deduplication_ratio: f64,
    pub space_savings_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_creation() {
        let manager = SnapshotManager::new(PathBuf::from("/tmp/snapshots")).unwrap();
        let snapshot_id = manager.create_snapshot(
            "test_snapshot".to_string(),
            "testdb".to_string(),
            SnapshotType::Manual,
        ).unwrap();

        let snapshot = manager.get_snapshot(&snapshot_id).unwrap();
        assert_eq!(snapshot.snapshot_name, "test_snapshot");
        assert_eq!(snapshot.status, SnapshotStatus::Available);
    }

    #[test]
    fn test_snapshot_clone() {
        let manager = SnapshotManager::new(PathBuf::from("/tmp/snapshots")).unwrap();
        let snapshot_id = manager.create_snapshot(
            "parent".to_string(),
            "testdb".to_string(),
            SnapshotType::Manual,
        ).unwrap();

        let clone_id = manager.clone_snapshot(
            &snapshot_id,
            "test_clone".to_string(),
            true,
            ClonePurpose::Testing,
        ).unwrap();

        let clone = manager.get_clone(&clone_id).unwrap();
        assert_eq!(clone.clone_name, "test_clone");
        assert!(clone.writable);
    }

    #[test]
    fn test_cow_tracker() {
        let tracker = CowTracker::new("test".to_string());
        let data = vec![1, 2, 3, 4, 5];

        tracker.record_block(1, 100, data.clone());
        let retrieved = tracker.get_block(1, 100).unwrap();

        assert_eq!(retrieved, data);
        assert_eq!(tracker.get_block_count(), 1);
    }

    #[test]
    fn test_snapshot_schedule() {
        let mut schedule = SnapshotSchedule::new(
            "daily".to_string(),
            "Daily Snapshot".to_string(),
            SnapshotFrequency::Daily { hour: 2 },
        );

        assert!(schedule.is_due()); // First time, should be due

        schedule.calculate_next_execution();
        assert!(schedule.next_execution.is_some());
    }
}


