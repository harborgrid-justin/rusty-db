// # Row Version Tracking and Management
//
// Implements Oracle-like VERSIONS BETWEEN queries and row version management.
// Tracks all historical versions of rows with efficient storage and retrieval.
//
// ## Features
//
// - VERSIONS BETWEEN SCN/TIMESTAMP queries
// - Version retention policies and TTL
// - Automatic version garbage collection
// - Undo-based versioning for rollback
// - Version metadata and change tracking
// - Cross-version joins and comparisons
// - Pseudocolumn support (VERSIONS_STARTSCN, VERSIONS_ENDSCN, etc.)
//
// ## Example
//
// ```sql
// SELECT versions_xid, versions_startscn, versions_endscn, salary
// FROM employees
// VERSIONS BETWEEN SCN 1000 AND 2000
// WHERE employee_id = 100;
// ```

use std::fmt;
use std::time::Duration;
use std::collections::VecDeque;
use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime};
use serde::{Deserialize, Serialize};

use crate::common::{TransactionId, TableId, RowId, Value};
use crate::error::{Result, DbError};
use super::time_travel::{SCN, Timestamp, RowVersion, current_timestamp};

// ============================================================================
// Arena Allocator for Version Data
// ============================================================================

/// Arena allocator for version data - avoids per-version heap allocations
/// Aligned to 4KB pages for optimal cache performance
#[repr(C, align(4096))]
pub struct VersionArena {
    /// Pre-allocated memory block
    data: Box<[u8]>,
    /// Current allocation offset
    offset: AtomicUsize,
    /// Arena capacity
    capacity: usize,
}

impl VersionArena {
    /// Create a new arena with specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0u8; capacity].into_boxed_slice(),
            offset: AtomicUsize::new(0),
            capacity,
        }
    }

    /// Allocate space in the arena, returns offset
    #[inline]
    pub fn allocate(&self, size: usize) -> Option<usize> {
        let current = self.offset.load(Ordering::Relaxed);
        let new_offset = current + size;

        if new_offset > self.capacity {
            return None;
        }

        self.offset.compare_exchange(
            current,
            new_offset,
            Ordering::Release,
            Ordering::Relaxed
        ).ok().map(|_| current)
    }

    /// Get remaining capacity
    #[inline]
    pub fn remaining(&self) -> usize {
        self.capacity.saturating_sub(self.offset.load(Ordering::Relaxed))
    }

    /// Reset arena (use with caution - only when all allocations are done)
    #[inline]
    pub fn reset(&self) {
        self.offset.store(0, Ordering::Release);
    }
}

// ============================================================================
// Version Manager
// ============================================================================

/// Manages all row versions across tables
pub struct VersionManager {
    /// Version store
    store: Arc<RwLock<VersionStore>>,

    /// Retention policies
    policies: Arc<RwLock<HashMap<TableId, VersionRetentionPolicy>>>,

    /// Garbage collector
    gc: Arc<RwLock<VersionGarbageCollector>>,

    /// Configuration
    #[allow(dead_code)]
    config: VersionConfig,

    /// Statistics
    stats: Arc<RwLock<VersionStats>>,
}

impl VersionManager {
    /// Create a new version manager
    pub fn new(config: VersionConfig) -> Self {
        Self {
            store: Arc::new(RwLock::new(VersionStore::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
            gc: Arc::new(RwLock::new(VersionGarbageCollector::new())),
            config,
            stats: Arc::new(RwLock::new(VersionStats::default())),
        }
    }

    /// Track a new row version
    pub fn track_version(
        &self,
        table_id: TableId,
        row_id: RowId,
        version: RowVersion,
    ) -> Result<()> {
        let mut store = self.store.write().unwrap();
        store.add_version(table_id, row_id, version)?;

        let mut stats = self.stats.write().unwrap();
        stats.total_versions += 1;
        stats.active_versions += 1;

        Ok(())
    }

    /// Execute VERSIONS BETWEEN query
    pub fn query_versions_between(
        &self,
        table_id: TableId,
        row_id: RowId,
        start: VersionBound,
        end: VersionBound,
    ) -> Result<Vec<VersionRow>> {
        let store = self.store.read().unwrap();
        let versions = store.get_versions_in_range(table_id, row_id, start, end)?;

        let mut stats = self.stats.write().unwrap();
        stats.version_queries += 1;

        Ok(versions)
    }

    /// Get all versions for a row
    pub fn get_row_versions(
        &self,
        table_id: TableId,
        row_id: RowId,
    ) -> Result<Vec<VersionRow>> {
        let store = self.store.read().unwrap();
        store.get_all_versions(table_id, row_id)
    }

    /// Set retention policy for a table
    pub fn set_retention_policy(
        &self,
        table_id: TableId,
        policy: VersionRetentionPolicy,
    ) -> Result<()> {
        let mut policies = self.policies.write().unwrap();
        policies.insert(table_id, policy);
        Ok(())
    }

    /// Run garbage collection
    pub fn run_garbage_collection(&self) -> Result<GarbageCollectionResult> {
        let policies = self.policies.read().unwrap();
        let mut gc = self.gc.write().unwrap();
        let mut store = self.store.write().unwrap();

        let result = gc.collect(&mut *store, &policies)?;

        let mut stats = self.stats.write().unwrap();
        stats.active_versions = stats.active_versions.saturating_sub(result.versions_removed as u64);
        stats.gc_runs += 1;
        stats.total_versions_removed += result.versions_removed as u64;

        Ok(result)
    }

    /// Create undo record for rollback
    pub fn create_undo_record(
        &self,
        table_id: TableId,
        row_id: RowId,
        old_version: RowVersion,
        new_version: RowVersion,
    ) -> Result<UndoRecord> {
        let undo = UndoRecord {
            table_id,
            row_id,
            old_values: old_version.values,
            new_values: new_version.values,
            scn: new_version.scn_created,
            txn_id: new_version.txn_id,
            timestamp: current_timestamp(),
        };

        Ok(undo)
    }

    /// Apply undo record
    pub fn apply_undo(&self, undo: &UndoRecord) -> Result<()> {
        // Restore old version by creating a new version with old values
        let version = RowVersion {
            values: undo.old_values.clone(),
            scn_created: undo.scn + 1, // Next SCN
            scn_deleted: None,
            txn_id: undo.txn_id,
            bitemporal: None,
        };

        self.track_version(undo.table_id, undo.row_id, version)
    }

    /// Get version metadata
    pub fn get_version_metadata(
        &self,
        table_id: TableId,
        row_id: RowId,
        scn: SCN,
    ) -> Result<VersionMetadata> {
        let store = self.store.read().unwrap();
        store.get_version_metadata(table_id, row_id, scn)
    }

    /// Compare two versions
    pub fn compare_versions(
        &self,
        table_id: TableId,
        row_id: RowId,
        scn1: SCN,
        scn2: SCN,
    ) -> Result<VersionComparison> {
        let store = self.store.read().unwrap();
        store.compare_versions(table_id, row_id, scn1, scn2)
    }

    /// Get statistics
    pub fn get_stats(&self) -> VersionStats {
        self.stats.read().unwrap().clone()
    }

    /// Compact version history for a table
    pub fn compact_versions(&self, table_id: TableId) -> Result<usize> {
        let mut store = self.store.write().unwrap();
        store.compact_table_versions(table_id)
    }
}

// ============================================================================
// Version Store
// ============================================================================

/// Storage for all row versions
struct VersionStore {
    /// Table -> Row -> Versions
    versions: HashMap<TableId, HashMap<RowId, VecDeque<RowVersion>>>,
}

impl VersionStore {
    fn new() -> Self {
        Self {
            versions: HashMap::new(),
        }
    }

    fn add_version(
        &mut self,
        table_id: TableId,
        row_id: RowId,
        version: RowVersion,
    ) -> Result<()> {
        let table_versions = self.versions.entry(table_id).or_insert_with(HashMap::new);
        let row_versions = table_versions.entry(row_id).or_insert_with(VecDeque::new);

        // Insert in chronological order
        row_versions.push_back(version);

        Ok(())
    }

    #[inline]
    fn get_versions_in_range(
        &self,
        table_id: TableId,
        row_id: RowId,
        start: VersionBound,
        end: VersionBound,
    ) -> Result<Vec<VersionRow>> {
        let versions = self.get_row_version_deque(table_id, row_id)?;
        let mut results = Vec::new();

        for version in versions.iter() {
            if self.is_version_in_range(version, &start, &end) {
                results.push(self.version_to_row(version));
            }
        }

        Ok(results)
    }

    fn get_all_versions(
        &self,
        table_id: TableId,
        row_id: RowId,
    ) -> Result<Vec<VersionRow>> {
        let versions = self.get_row_version_deque(table_id, row_id)?;
        Ok(versions.iter().map(|v| self.version_to_row(v)).collect())
    }

    #[inline]
    fn get_row_version_deque(
        &self,
        table_id: TableId,
        row_id: RowId,
    ) -> Result<&VecDeque<RowVersion>> {
        self.versions
            .get(&table_id)
            .and_then(|t| t.get(&row_id))
            .ok_or_else(|| Self::no_versions_error(table_id, row_id))
    }

    #[cold]
    #[inline(never)]
    fn no_versions_error(table_id: TableId, row_id: RowId) -> DbError {
        DbError::Validation(
            format!("No versions found for table {} row {}", table_id, row_id)
        )
    }

    fn is_version_in_range(
        &self,
        version: &RowVersion,
        start: &VersionBound,
        end: &VersionBound,
    ) -> bool {
        let after_start = match start {
            VersionBound::SCN(scn) => version.scn_created >= *scn,
            VersionBound::Timestamp(_ts) => true, // Would need timestamp mapping
            VersionBound::Minvalue => true,
            VersionBound::Maxvalue => false,
        };

        let before_end = match end {
            VersionBound::SCN(scn) => version.scn_created <= *scn,
            VersionBound::Timestamp(_ts) => true, // Would need timestamp mapping
            VersionBound::Maxvalue => true,
            VersionBound::Minvalue => false,
        };

        after_start && before_end
    }

    fn version_to_row(&self, version: &RowVersion) -> VersionRow {
        VersionRow {
            values: version.values.clone(),
            versions_startscn: version.scn_created,
            versions_endscn: version.scn_deleted,
            versions_xid: version.txn_id,
            versions_operation: self.determine_operation(version),
            versions_starttime: None, // Would need timestamp mapping
            versions_endtime: None,
        }
    }

    fn determine_operation(&self, version: &RowVersion) -> VersionOperation {
        if version.scn_deleted.is_some() {
            VersionOperation::Delete
        } else {
            VersionOperation::Update
        }
    }

    fn get_version_metadata(
        &self,
        table_id: TableId,
        row_id: RowId,
        scn: SCN,
    ) -> Result<VersionMetadata> {
        let versions = self.get_row_version_deque(table_id, row_id)?;

        let version = versions
            .iter()
            .find(|v| v.scn_created == scn)
            .ok_or_else(|| DbError::Validation(
                format!("Version not found at SCN {}", scn)
            ))?;

        Ok(VersionMetadata {
            scn_created: version.scn_created,
            scn_deleted: version.scn_deleted,
            txn_id: version.txn_id,
            size_bytes: self.estimate_version_size(version),
            column_count: version.values.len(),
        })
    }

    fn estimate_version_size(&self, version: &RowVersion) -> usize {
        version.values.iter().map(|v| self.estimate_value_size(v)).sum()
    }

    fn estimate_value_size(&self, value: &Value) -> usize {
        match value {
            Value::Null => 1,
            Value::Boolean(_) => 1,
            Value::Integer(_) => 8,
            Value::Float(_) => 8,
            Value::String(s) => s.len(),
            Value::Bytes(b) => b.len(),
            Value::Date(_) => 8,
            Value::Timestamp(_) => 8,
            Value::Json(j) => j.to_string().len(),
            Value::Array(a) => a.iter().map(|v| self.estimate_value_size(v)).sum(),
            Value::Text => 4,
        }
    }

    fn compare_versions(
        &self,
        table_id: TableId,
        row_id: RowId,
        scn1: SCN,
        scn2: SCN,
    ) -> Result<VersionComparison> {
        let versions = self.get_row_version_deque(table_id, row_id)?;

        let v1 = versions.iter().find(|v| v.scn_created == scn1)
            .ok_or_else(|| DbError::Validation(format!("Version at SCN {} not found", scn1)))?;

        let v2 = versions.iter().find(|v| v.scn_created == scn2)
            .ok_or_else(|| DbError::Validation(format!("Version at SCN {} not found", scn2)))?;

        let mut changed_columns = Vec::new();
        for (i, (val1, val2)) in v1.values.iter().zip(&v2.values).enumerate() {
            if val1 != val2 {
                changed_columns.push(ColumnChange {
                    column_index: i,
                    old_value: val1.clone(),
                    new_value: val2.clone(),
                });
            }
        }

        let identical = changed_columns.is_empty();

        Ok(VersionComparison {
            scn1,
            scn2,
            changed_columns,
            identical,
        })
    }

    fn compact_table_versions(&mut self, table_id: TableId) -> Result<usize> {
        let mut removed = 0;

        if let Some(table_versions) = self.versions.get_mut(&table_id) {
            for row_versions in table_versions.values_mut() {
                // Keep only active versions and the last deleted version
                let original_len = row_versions.len();

                let mut to_keep = Vec::new();
                let mut last_deleted = None;

                for version in row_versions.iter() {
                    if version.scn_deleted.is_none() {
                        to_keep.push(version.clone());
                    } else {
                        last_deleted = Some(version.clone());
                    }
                }

                if let Some(deleted) = last_deleted {
                    to_keep.push(deleted);
                }

                *row_versions = to_keep.into();
                removed += original_len - row_versions.len();
            }
        }

        Ok(removed)
    }
}

// ============================================================================
// Version Bounds
// ============================================================================

/// Boundary for version queries
#[derive(Debug, Clone)]
pub enum VersionBound {
    /// SCN boundary
    SCN(SCN),

    /// Timestamp boundary
    Timestamp(Timestamp),

    /// Minimum value (beginning of time)
    Minvalue,

    /// Maximum value (current time)
    Maxvalue,
}

// ============================================================================
// Version Row Result
// ============================================================================

/// Result row from VERSIONS BETWEEN query with pseudocolumns
#[repr(C)]
#[derive(Debug, Clone)]
pub struct VersionRow {
    /// Actual column values
    pub values: Vec<Value>,

    /// VERSIONS_STARTSCN pseudocolumn
    pub versions_startscn: SCN,

    /// VERSIONS_ENDSCN pseudocolumn
    pub versions_endscn: Option<SCN>,

    /// VERSIONS_XID pseudocolumn (transaction ID)
    pub versions_xid: TransactionId,

    /// VERSIONS_OPERATION pseudocolumn
    pub versions_operation: VersionOperation,

    /// VERSIONS_STARTTIME pseudocolumn
    pub versions_starttime: Option<Timestamp>,

    /// VERSIONS_ENDTIME pseudocolumn
    pub versions_endtime: Option<Timestamp>,
}

/// Type of operation that created this version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionOperation {
    Insert,
    Update,
    Delete,
}

impl fmt::Display for VersionOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionOperation::Insert => write!(f, "I"),
            VersionOperation::Update => write!(f, "U"),
            VersionOperation::Delete => write!(f, "D"),
        }
    }
}

// ============================================================================
// Version Retention Policy
// ============================================================================

/// Policy for retaining historical versions
#[derive(Debug, Clone)]
pub struct VersionRetentionPolicy {
    /// Maximum number of versions per row
    pub max_versions_per_row: Option<usize>,

    /// Maximum age of versions (in seconds)
    pub max_age_seconds: Option<u64>,

    /// Minimum number of versions to always keep
    pub min_versions_to_keep: usize,

    /// Keep all versions for important tables
    pub retain_all: bool,
}

impl Default for VersionRetentionPolicy {
    fn default() -> Self {
        Self {
            max_versions_per_row: Some(100),
            max_age_seconds: Some(30 * 24 * 3600), // 30 days
            min_versions_to_keep: 1,
            retain_all: false,
        }
    }
}

// ============================================================================
// Garbage Collector
// ============================================================================

/// Garbage collector for old versions
struct VersionGarbageCollector {
    /// Last GC run time
    last_run: Option<SystemTime>,
}

impl VersionGarbageCollector {
    fn new() -> Self {
        Self { last_run: None }
    }

    fn collect(
        &mut self,
        store: &mut VersionStore,
        policies: &HashMap<TableId, VersionRetentionPolicy>,
    ) -> Result<GarbageCollectionResult> {
        let start_time = SystemTime::now();
        let mut result = GarbageCollectionResult {
            versions_removed: 0,
            tables_processed: 0,
            duration: Duration::from_secs(0),
        };

        for (table_id, table_versions) in store.versions.iter_mut() {
            let policy = policies.get(table_id)
                .cloned()
                .unwrap_or_default();

            if policy.retain_all {
                continue;
            }

            for row_versions in table_versions.values_mut() {
                result.versions_removed += self.collect_row_versions(row_versions, &policy);
            }

            result.tables_processed += 1;
        }

        result.duration = start_time.elapsed().unwrap_or_default();
        self.last_run = Some(SystemTime::now());

        Ok(result)
    }

    fn collect_row_versions(
        &self,
        versions: &mut VecDeque<RowVersion>,
        policy: &VersionRetentionPolicy,
    ) -> usize {
        let original_count = versions.len();

        // Keep minimum versions
        if versions.len() <= policy.min_versions_to_keep {
            return 0;
        }

        let mut to_remove = Vec::new();

        for (i, version) in versions.iter().enumerate() {
            // Don't remove if it's within min_versions_to_keep from the end
            if i >= versions.len() - policy.min_versions_to_keep {
                break;
            }

            // Check max versions per row
            if let Some(max_versions) = policy.max_versions_per_row {
                if versions.len() > max_versions && i < versions.len() - max_versions {
                    to_remove.push(i);
                    continue;
                }
            }

            // Check max age
            if let Some(_max_age_secs) = policy.max_age_seconds {
                if version.scn_deleted.is_some() {
                    // Version is deleted, check if old enough
                    // Note: Would need proper timestamp mapping
                    to_remove.push(i);
                }
            }
        }

        // Remove in reverse order to maintain indices
        for &idx in to_remove.iter().rev() {
            versions.remove(idx);
        }

        original_count - versions.len()
    }
}

/// Result of garbage collection
#[derive(Debug, Clone)]
pub struct GarbageCollectionResult {
    pub versions_removed: usize,
    pub tables_processed: usize,
    pub duration: Duration,
}

// ============================================================================
// Undo Records
// ============================================================================

/// Undo record for transaction rollback
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoRecord {
    pub table_id: TableId,
    pub row_id: RowId,
    pub old_values: Vec<Value>,
    pub new_values: Vec<Value>,
    pub scn: SCN,
    pub txn_id: TransactionId,
    pub timestamp: Timestamp,
}

impl UndoRecord {
    /// Get size of undo record in bytes
    pub fn size_bytes(&self) -> usize {
        // Simplified size estimation
        self.old_values.len() * 8 + self.new_values.len() * 8 + 64
    }
}

// ============================================================================
// Version Metadata
// ============================================================================

/// Metadata about a specific version
#[repr(C)]
#[derive(Debug, Clone)]
pub struct VersionMetadata {
    pub scn_created: SCN,
    pub scn_deleted: Option<SCN>,
    pub txn_id: TransactionId,
    pub size_bytes: usize,
    pub column_count: usize,
}

// ============================================================================
// Version Comparison
// ============================================================================

/// Result of comparing two versions
#[derive(Debug, Clone)]
pub struct VersionComparison {
    pub scn1: SCN,
    pub scn2: SCN,
    pub changed_columns: Vec<ColumnChange>,
    pub identical: bool,
}

#[derive(Debug, Clone)]
pub struct ColumnChange {
    pub column_index: usize,
    pub old_value: Value,
    pub new_value: Value,
}

// ============================================================================
// Cross-Version Join Support
// ============================================================================

/// Support for joining versions across time
pub struct VersionJoinExecutor {
    #[allow(dead_code)]
    version_manager: Arc<VersionManager>,
}

impl VersionJoinExecutor {
    pub fn new(version_manager: Arc<VersionManager>) -> Self {
        Self { version_manager }
    }

    /// Execute temporal join between two table versions
    pub fn execute_temporal_join(
        &self,
        _left_table: TableId,
        _right_table: TableId,
        _left_scn: SCN,
        _right_scn: SCN,
        _join_condition: JoinCondition,
    ) -> Result<Vec<JoinedVersionRow>> {
        // Implementation would require integration with query executor
        // This is a placeholder for the structure
        Ok(Vec::new())
    }
}

#[derive(Debug, Clone)]
pub struct JoinCondition {
    pub left_column: usize,
    pub right_column: usize,
}

#[derive(Debug, Clone)]
pub struct JoinedVersionRow {
    pub left_values: Vec<Value>,
    pub right_values: Vec<Value>,
    pub left_scn: SCN,
    pub right_scn: SCN,
}

// ============================================================================
// Configuration
// ============================================================================

/// Version manager configuration
#[derive(Debug, Clone)]
pub struct VersionConfig {
    /// Enable undo-based versioning
    pub enable_undo: bool,

    /// Maximum undo records to keep
    pub max_undo_records: usize,

    /// Automatic garbage collection interval (seconds)
    pub gc_interval_seconds: u64,

    /// Enable version compression
    pub enable_compression: bool,
}

impl Default for VersionConfig {
    fn default() -> Self {
        Self {
            enable_undo: true,
            max_undo_records: 100000,
            gc_interval_seconds: 3600, // 1 hour
            enable_compression: false,
        }
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Statistics for version management
#[derive(Debug, Clone, Default)]
pub struct VersionStats {
    pub total_versions: u64,
    pub active_versions: u64,
    pub version_queries: u64,
    pub gc_runs: u64,
    pub total_versions_removed: u64,
    pub storage_bytes: ()
}

impl VersionStats {
    pub fn avg_versions_per_gc(&self) -> f64 {
        if self.gc_runs == 0 {
            0.0
        } else {
            (self.total_versions_removed as f64) / (self.gc_runs as f64)
        }
    }

    pub fn version_efficiency(&self) -> f64 {
        if self.total_versions == 0 {
            1.0
        } else {
            (self.active_versions as f64) / (self.total_versions as f64)
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_manager() {
        let config = VersionConfig::default();
        let manager = VersionManager::new(config);

        let version = RowVersion {
            values: vec![Value::Integer(100)],
            scn_created: 1000,
            scn_deleted: None,
            txn_id: 1,
            bitemporal: None,
        };

        manager.track_version(1, 1, version).unwrap();

        let versions = manager.get_row_versions(1, 1).unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].versions_startscn, 1000);
    }

    #[test]
    fn test_versions_between() {
        let config = VersionConfig::default();
        let manager = VersionManager::new(config);

        // Add multiple versions
        for i in 0..5 {
            let version = RowVersion {
                values: vec![Value::Integer(i as i64)],
                scn_created: 1000 + (i * 100),
                scn_deleted: None,
                txn_id: i as u64,
                bitemporal: None,
            };
            manager.track_version(1, 1, version).unwrap();
        }

        let versions = manager.query_versions_between(
            1, 1,
            VersionBound::SCN(1100),
            VersionBound::SCN(1300),
        ).unwrap();

        assert_eq!(versions.len(), 3); // SCN 1100, 1200, 1300
    }

    #[test]
    fn test_undo_record() {
        let config = VersionConfig::default();
        let manager = VersionManager::new(config);

        let old_version = RowVersion {
            values: vec![Value::Integer(10)],
            scn_created: 1000,
            scn_deleted: Some(1100),
            txn_id: 1,
            bitemporal: None,
        };

        let new_version = RowVersion {
            values: vec![Value::Integer(20)],
            scn_created: 1100,
            scn_deleted: None,
            txn_id: 2,
            bitemporal: None,
        };

        let undo = manager.create_undo_record(1, 1, old_version, new_version).unwrap();
        assert_eq!(undo.old_values[0], Value::Integer(10));
        assert_eq!(undo.new_values[0], Value::Integer(20));
    }

    #[test]
    fn test_version_comparison() {
        let config = VersionConfig::default();
        let manager = VersionManager::new(config);

        let v1 = RowVersion {
            values: vec![Value::Integer(10), Value::String("old".to_string())],
            scn_created: 1000,
            scn_deleted: None,
            txn_id: 1,
            bitemporal: None,
        };

        let v2 = RowVersion {
            values: vec![Value::Integer(20), Value::String("old".to_string())],
            scn_created: 1100,
            scn_deleted: None,
            txn_id: 2,
            bitemporal: None,
        };

        manager.track_version(1, 1, v1).unwrap();
        manager.track_version(1, 1, v2).unwrap();

        let comparison = manager.compare_versions(1, 1, 1000, 1100).unwrap();
        assert_eq!(comparison.changed_columns.len(), 1);
        assert_eq!(comparison.changed_columns[0].column_index, 0);
    }
}
