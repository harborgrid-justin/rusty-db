// Multi-Version Concurrency Control (MVCC) Implementation
// Provides timestamp-based versioning with hybrid logical clocks,
// snapshot isolation, and write skew detection

use std::collections::{HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime};
use parking_lot::{RwLock};
use serde::{Deserialize, Serialize};
use crate::error::Result;
use super::{TransactionId, LogSequenceNumber};

/// Hybrid Logical Clock for distributed timestamp ordering
/// Combines physical time with logical counters for causality tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HybridTimestamp {
    /// Physical time component (milliseconds since epoch)
    pub physical: u64,
    /// Logical counter for events at the same physical time
    pub logical: u64,
    /// Node identifier for distributed systems
    pub node_id: u32,
}

impl HybridTimestamp {
    /// Create a new hybrid timestamp
    pub fn new(physical: u64, logical: u64, node_id: u32) -> Self {
        Self { physical, logical, node_id }
    }

    /// Get current wall clock time as hybrid timestamp
    pub fn now(node_id: u32) -> Self {
        let physical = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self::new(physical, 0, node_id)
    }

    /// Increment the logical component
    pub fn tick(&mut self) {
        self.logical += 1;
    }

    /// Update timestamp based on received timestamp (for causality)
    pub fn update(&mut self, other: &HybridTimestamp) {
        if other.physical > self.physical {
            self.physical = other.physical;
            self.logical = 0;
        } else if other.physical == self.physical {
            self.logical = self.logical.max(other.logical) + 1;
        } else {
            self.logical += 1;
        }
    }

    /// Check if this timestamp happens-before another
    pub fn happens_before(&self, other: &HybridTimestamp) -> bool {
        self.physical < other.physical ||
        (self.physical == other.physical && self.logical < other.logical)
    }

    /// Check if timestamps are concurrent (no causal relationship)
    pub fn is_concurrent(&self, other: &HybridTimestamp) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }
}

/// Hybrid Logical Clock manager for the database
pub struct HybridClock {
    node_id: u32,
    current: Arc<RwLock<HybridTimestamp>>,
    skew_tolerance: Duration,
}

impl HybridClock {
    /// Create a new hybrid clock
    pub fn new(node_id: u32) -> Self {
        let current = HybridTimestamp::now(node_id);
        Self {
            node_id,
            current: Arc::new(RwLock::new(current)),
            skew_tolerance: Duration::from_secs(5),
        }
    }

    /// Get current timestamp and increment
    pub fn now(&self) -> HybridTimestamp {
        let mut ts = self.current.write();
        let wall_clock = HybridTimestamp::now(self.node_id);
        ts.update(&wall_clock);
        ts.tick();
        *ts
    }

    /// Update clock based on received message timestamp
    pub fn update(&self, remote_ts: HybridTimestamp) -> std::result::Result<HybridTimestamp, DbError> {
        let mut ts = self.current.write();

        // Check for excessive clock skew
        let local_physical = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        if remote_ts.physical > local_physical {
            let skew = remote_ts.physical - local_physical;
            if skew > self.skew_tolerance.as_millis() as u64 {
                return Err(DbError::Transaction(
                    format!("Clock skew too large: {}ms", skew)
                ));
            }
        }

        ts.update(&remote_ts);
        ts.tick();
        Ok(*ts)
    }

    /// Get current timestamp without incrementing
    pub fn peek(&self) -> HybridTimestamp {
        *self.current.read()
    }
}

/// Version of a database record with MVCC metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedRecord<T: Clone> {
    /// Data payload
    pub data: T,
    /// Transaction that created this version
    pub created_by: TransactionId,
    /// Timestamp when this version was created
    pub created_at: HybridTimestamp,
    /// Transaction that deleted this version (None if active)
    pub deleted_by: Option<TransactionId>,
    /// Timestamp when this version was deleted (None if active)
    pub deleted_at: Option<HybridTimestamp>,
    /// Log sequence number for recovery
    pub lsn: LogSequenceNumber,
    /// Pointer to next version in chain
    pub next_version: Option<usize>,
    /// Pointer to previous version in chain
    pub prev_version: Option<usize>,
}

impl<T: Clone> VersionedRecord<T> {
    /// Create a new version
    pub fn new(
        data: T,
        txn_id: TransactionId,
        timestamp: HybridTimestamp,
        lsn: LogSequenceNumber,
    ) -> Self {
        Self {
            data,
            created_by: txn_id,
            created_at: timestamp,
            deleted_by: None,
            deleted_at: None,
            lsn,
            next_version: None,
            prev_version: None,
        }
    }

    /// Check if this version is visible to a transaction with given timestamp
    pub fn is_visible_to(&self, read_ts: &HybridTimestamp) -> bool {
        // Version must be created before the read timestamp
        if !self.created_at.happens_before(read_ts) && self.created_at != *read_ts {
            return false;
        }

        // Version must not be deleted, or deleted after read timestamp
        match &self.deleted_at {
            None => true,
            Some(deleted_ts) => read_ts.happens_before(deleted_ts),
        }
    }

    /// Mark this version as deleted
    pub fn mark_deleted(&mut self, txn_id: TransactionId, timestamp: HybridTimestamp) {
        self.deleted_by = Some(txn_id);
        self.deleted_at = Some(timestamp);
    }

    /// Check if this version is deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted_by.is_some()
    }
}

/// Version chain for a single record key
pub struct VersionChain<T: Clone> {
    /// Versions ordered from newest to oldest
    versions: VecDeque<VersionedRecord<T>>,
    /// Index of the latest version
    head: usize,
    /// Maximum number of versions to retain
    max_versions: usize,
}

impl<T: Clone> VersionChain<T> {
    /// Create a new version chain
    pub fn new(max_versions: usize) -> Self {
        Self {
            versions: VecDeque::new(),
            head: 0,
            max_versions,
        }
    }

    /// Add a new version to the chain
    pub fn add_version(&mut self, mut version: VersionedRecord<T>) {
        if !self.versions.is_empty() {
            // Link to previous head
            version.prev_version = Some(self.head);
            let old_head = &mut self.versions[self.head];
            old_head.next_version = Some(self.versions.len());
        }

        self.head = self.versions.len();
        self.versions.push_back(version);

        // Trim old versions if needed
        if self.versions.len() > self.max_versions {
            self.versions.pop_front();
            // Reindex after removal
            self.head = self.head.saturating_sub(1);
        }
    }

    /// Get the latest version
    pub fn get_latest(&self) -> Option<&VersionedRecord<T>> {
        self.versions.get(self.head)
    }

    /// Get version visible to a transaction with given timestamp
    pub fn get_version_at(&self, read_ts: &HybridTimestamp) -> Option<&VersionedRecord<T>> {
        // Start from newest and scan backwards
        for version in self.versions.iter().rev() {
            if version.is_visible_to(read_ts) {
                return Some(version);
            }
        }
        None
    }

    /// Get all versions (for garbage collection analysis)
    pub fn get_all_versions(&self) -> impl Iterator<Item = &VersionedRecord<T>> {
        self.versions.iter()
    }

    /// Remove versions older than the given timestamp
    pub fn gc_versions_before(&mut self, gc_ts: &HybridTimestamp) -> usize {
        let before_len = self.versions.len();

        // Keep only versions that are visible at or after gc_ts
        self.versions.retain(|v| {
            v.created_at >= *gc_ts ||
            v.deleted_at.map(|dt| dt >= *gc_ts).unwrap_or(true)
        });

        // Update head index
        if self.versions.is_empty() {
            self.head = 0;
        } else {
            self.head = self.versions.len() - 1;
        }

        before_len - self.versions.len()
    }

    /// Get the number of versions in the chain
    pub fn len(&self) -> usize {
        self.versions.len()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }
}

/// Multi-Version Concurrency Control Manager
pub struct MVCCManager<K: Clone + Eq + std::hash::Hash, V: Clone> {
    /// Version chains indexed by key
    versions: Arc<RwLock<HashMap<K, Arc<Mutex<VersionChain<V>>>>>>,
    /// Hybrid logical clock
    clock: Arc<HybridClock>,
    /// Active snapshots for garbage collection
    active_snapshots: Arc<RwLock<BTreeMap<TransactionId, HybridTimestamp>>>,
    /// Oldest active snapshot timestamp
    min_snapshot_ts: Arc<RwLock<Option<HybridTimestamp>>>,
    /// Statistics
    stats: Arc<RwLock<MVCCStats>>,
    /// Next LSN
    next_lsn: Arc<AtomicU64>,
    /// Configuration
    config: MVCCConfig,
}

#[derive(Debug, Clone)]
pub struct MVCCConfig {
    /// Maximum versions per key
    pub max_versions: usize,
    /// Enable automatic garbage collection
    pub auto_gc: bool,
    /// GC interval in seconds
    pub gc_interval_secs: u64,
    /// Node ID for hybrid clock
    pub node_id: u32,
}

impl Default for MVCCConfig {
    fn default() -> Self {
        Self {
            max_versions: 100,
            auto_gc: true,
            gc_interval_secs: 60,
            node_id: 0,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MVCCStats {
    pub total_versions: u64,
    pub active_versions: u64,
    pub deleted_versions: u64,
    pub gc_runs: u64,
    pub versions_collected: u64,
    pub read_requests: u64,
    pub write_requests: u64,
    pub snapshot_conflicts: u64,
}

impl<K: Clone + Eq + std::hash::Hash, V: Clone> MVCCManager<K, V> {
    /// Create a new MVCC manager
    pub fn new(config: MVCCConfig) -> Self {
        Self {
            versions: Arc::new(RwLock::new(HashMap::new())),
            clock: Arc::new(HybridClock::new(config.node_id)),
            active_snapshots: Arc::new(RwLock::new(BTreeMap::new())),
            min_snapshot_ts: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(MVCCStats::default())),
            next_lsn: Arc::new(AtomicU64::new(1)),
            config,
        }
    }

    /// Begin a new snapshot for a transaction
    pub fn begin_snapshot(&self, txn_id: TransactionId) -> HybridTimestamp {
        let timestamp = self.clock.now();

        // Register snapshot
        let mut snapshots = self.active_snapshots.write();
        snapshots.insert(txn_id, timestamp);

        // Update minimum snapshot timestamp
        if let Some((&_min_txn, &min_ts)) = snapshots.iter().next() {
            *self.min_snapshot_ts.write() = Some(min_ts);
        }

        timestamp
    }

    /// End a snapshot for a transaction
    pub fn end_snapshot(&self, txn_id: TransactionId) {
        let mut snapshots = self.active_snapshots.write();
        snapshots.remove(&txn_id);

        // Update minimum snapshot timestamp
        if let Some((&_min_txn, &min_ts)) = snapshots.iter().next() {
            *self.min_snapshot_ts.write() = Some(min_ts);
        } else {
            *self.min_snapshot_ts.write() = None;
        }
    }

    /// Read a value at a specific timestamp
    pub fn read(&self, key: &K, read_ts: &HybridTimestamp) -> std::result::Result<Option<V>, DbError> {
        self.stats.write().read_requests += 1;

        let versions = self.versions.read();
        if let Some(chain) = versions.get(key) {
            let chain = chain.lock();
            if let Some(version) = chain.get_version_at(read_ts) {
                return Ok(Some(version.data.clone()));
            }
        }
        Ok(None)
    }

    /// Write a new version
    pub fn write(
        &self,
        key: K,
        value: V,
        txn_id: TransactionId,
        timestamp: HybridTimestamp,
    ) -> std::result::Result<(), DbError> {
        self.stats.write().write_requests += 1;

        let lsn = self.next_lsn.fetch_add(1, Ordering::SeqCst);
        let version = VersionedRecord::new(value, txn_id, timestamp, lsn);

        let mut versions = self.versions.write();
        let chain = versions
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(VersionChain::new(self.config.max_versions))));

        chain.lock().add_version(version);

        self.stats.write().total_versions += 1;
        self.stats.write().active_versions += 1;

        Ok(())
    }

    /// Delete a value (creates a delete marker version)
    pub fn delete(
        &self,
        key: &K,
        txn_id: TransactionId,
        timestamp: HybridTimestamp,
    ) -> std::result::Result<bool, DbError> {
        let versions = self.versions.read();
        if let Some(chain) = versions.get(key) {
            let mut chain = chain.lock();
            if let Some(latest) = chain.get_latest() {
                if !latest.is_deleted() {
                    // Create a new version that's marked as deleted
                    let lsn = self.next_lsn.fetch_add(1, Ordering::SeqCst);
                    let mut new_version = VersionedRecord::new(
                        latest.data.clone(),
                        txn_id,
                        timestamp,
                        lsn,
                    );
                    new_version.mark_deleted(txn_id, timestamp);
                    chain.add_version(new_version);

                    self.stats.write().deleted_versions += 1;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Run garbage collection
    pub fn garbage_collect(&self) -> std::result::Result<usize, DbError> {
        let min_ts = match *self.min_snapshot_ts.read() {
            Some(ts) => ts,
            None => return Ok(0), // No active snapshots, can't GC
        };

        let mut total_collected = 0;
        let versions = self.versions.read();

        for (_key, chain) in versions.iter() {
            let mut chain = chain.lock();
            let collected = chain.gc_versions_before(&min_ts);
            total_collected += collected;
        }

        let mut stats = self.stats.write();
        stats.gc_runs += 1;
        stats.versions_collected += total_collected as u64;
        stats.active_versions = stats.active_versions.saturating_sub(total_collected as u64);

        Ok(total_collected)
    }

    /// Get current statistics
    pub fn get_stats(&self) -> MVCCStats {
        self.stats.read().clone()
    }

    /// Get the hybrid clock
    pub fn clock(&self) -> &Arc<HybridClock> {
        &self.clock
    }

    /// Get number of active snapshots
    pub fn active_snapshot_count(&self) -> usize {
        self.active_snapshots.read().len()
    }

    /// Get oldest active snapshot timestamp
    pub fn oldest_snapshot(&self) -> Option<HybridTimestamp> {
        *self.min_snapshot_ts.read()
    }
}

/// Snapshot Isolation Manager
pub struct SnapshotIsolationManager {
    /// Active transactions with their read timestamps
    active_txns: Arc<RwLock<HashMap<TransactionId, TransactionSnapshot>>>,
    /// Write sets for conflict detection
    write_sets: Arc<RwLock<HashMap<TransactionId<String>>>>,
    /// Committed write sets (for write-skew detection)
    committed_writes: Arc<RwLock<BTreeMap<HybridTimestamp<String>>>>,
    /// Hybrid clock
    clock: Arc<HybridClock>,
    /// Configuration
    config: SnapshotConfig,
}

#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    /// Enable serializable upgrade (SSI)
    pub serializable: bool,
    /// Enable write-skew detection
    pub detect_write_skew: bool,
    /// Retention period for committed writes
    pub retention_secs: u64,
    /// Node ID
    pub node_id: u32,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            serializable: false,
            detect_write_skew: true,
            retention_secs: 300,
            node_id: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransactionSnapshot {
    pub txn_id: TransactionId,
    pub start_ts: HybridTimestamp,
    pub read_set: HashSet<String>,
    pub write_set: HashSet<String>,
    pub read_only: bool,
}

impl SnapshotIsolationManager {
    /// Create a new snapshot isolation manager
    pub fn new(config: SnapshotConfig) -> Self {
        Self {
            active_txns: Arc::new(RwLock::new(HashMap::new())),
            write_sets: Arc::new(RwLock::new(HashMap::new())),
            committed_writes: Arc::new(RwLock::new(BTreeMap::new())),
            clock: Arc::new(HybridClock::new(config.node_id)),
            config,
        }
    }

    /// Begin a new transaction snapshot
    pub fn begin_transaction(&self, txn_id: TransactionId, read_only: bool) -> HybridTimestamp {
        let start_ts = self.clock.now();
        let snapshot = TransactionSnapshot {
            txn_id,
            start_ts,
            read_set: HashSet::new(),
            write_set: HashSet::new(),
            read_only,
        };

        self.active_txns.write().insert(txn_id, snapshot);
        start_ts
    }

    /// Record a read operation
    pub fn record_read(&self, txn_id: TransactionId, key: String) -> std::result::Result<(), DbError> {
        let mut txns = self.active_txns.write();
        if let Some(snapshot) = txns.get_mut(&txn_id) {
            snapshot.read_set.insert(key);
            Ok(())
        } else {
            Err(DbError::Transaction(format!(
                "Transaction {} not found",
                txn_id
            )))
        }
    }

    /// Record a write operation
    pub fn record_write(&self, txn_id: TransactionId, key: String) -> std::result::Result<(), DbError> {
        let mut txns = self.active_txns.write();
        if let Some(snapshot) = txns.get_mut(&txn_id) {
            snapshot.write_set.insert(key.clone());
            drop(txns);

            self.write_sets.write().entry(txn_id)
                .or_insert_with(HashSet::new)
                .insert(key);
            Ok(())
        } else {
            Err(DbError::Transaction(format!(
                "Transaction {} not found",
                txn_id
            )))
        }
    }

    /// Check for write-write conflicts
    pub fn check_write_conflicts(&self, txn_id: TransactionId) -> std::result::Result<(), DbError> {
        let txns = self.active_txns.read();
        let snapshot = txns.get(&txn_id).ok_or_else(|| {
            DbError::Transaction(format!("Transaction {} not found", txn_id))
        })?;

        if snapshot.read_only {
            return Ok(());
        }

        let write_sets = self.write_sets.read();
        let my_writes = write_sets.get(&txn_id);

        // Check for conflicts with other active transactions
        for (other_txn, other_snapshot) in txns.iter() {
            if *other_txn == txn_id {
                continue;
            }

            if let Some(my_writes) = my_writes {
                if let Some(other_writes) = write_sets.get(other_txn) {
                    // Check for overlapping writes
                    if my_writes.intersection(other_writes).next().is_some() {
                        return Err(DbError::Transaction(format!(
                            "Write-write conflict between txn {} and {}",
                            txn_id, other_txn
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Check for write-skew anomalies (requires serializable mode)
    pub fn check_write_skew(&self, txn_id: TransactionId) -> std::result::Result<(), DbError> {
        if !self.config.detect_write_skew {
            return Ok(());
        }

        let txns = self.active_txns.read();
        let snapshot = txns.get(&txn_id).ok_or_else(|| {
            DbError::Transaction(format!("Transaction {} not found", txn_id))
        })?;

        if snapshot.read_only {
            return Ok(());
        }

        // Check if any committed transaction wrote to our read set
        let committed = self.committed_writes.read();
        for (commit_ts, committed_keys) in committed.range(snapshot.start_ts..) {
            if snapshot.read_set.intersection(committed_keys).next().is_some() {
                return Err(DbError::Transaction(format!(
                    "Write-skew detected: transaction {} read data modified by commit at {:?}",
                    txn_id, commit_ts
                )));
            }
        }

        Ok(())
    }

    /// Commit a transaction
    pub fn commit_transaction(&self, txn_id: TransactionId) -> std::result::Result<HybridTimestamp, DbError> {
        // Check for conflicts
        self.check_write_conflicts(txn_id)?;

        if self.config.serializable {
            self.check_write_skew(txn_id)?;
        }

        let commit_ts = self.clock.now();

        // Move write set to committed writes
        if let Some(write_set) = self.write_sets.write().remove(&txn_id) {
            self.committed_writes.write().insert(commit_ts, write_set);
        }

        // Remove transaction
        self.active_txns.write().remove(&txn_id);

        // Clean up old committed writes
        self.cleanup_committed_writes();

        Ok(commit_ts)
    }

    /// Abort a transaction
    pub fn abort_transaction(&self, txn_id: TransactionId) {
        self.active_txns.write().remove(&txn_id);
        self.write_sets.write().remove(&txn_id);
    }

    /// Clean up old committed write records
    fn cleanup_committed_writes(&self) {
        let retention = Duration::from_secs(self.config.retention_secs);
        let cutoff_physical = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
            - retention.as_millis() as u64;

        let cutoff_ts = HybridTimestamp::new(cutoff_physical, 0, self.config.node_id);

        let mut committed = self.committed_writes.write();
        committed.retain(|ts, _| *ts >= cutoff_ts);
    }

    /// Get active transaction count
    pub fn active_transaction_count(&self) -> usize {
        self.active_txns.read().len()
    }

    /// Get clock
    pub fn clock(&self) -> &Arc<HybridClock> {
        &self.clock
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_timestamp_ordering() {
        let ts1 = HybridTimestamp::new(100, 0, 1);
        let ts2 = HybridTimestamp::new(100, 1, 1);
        let ts3 = HybridTimestamp::new(101, 0, 1);

        assert!(ts1.happens_before(&ts2));
        assert!(ts2.happens_before(&ts3));
        assert!(ts1.happens_before(&ts3));
    }

    #[test]
    fn test_hybrid_clock_monotonicity() {
        let clock = HybridClock::new(1);
        let ts1 = clock.now();
        let ts2 = clock.now();
        let ts3 = clock.now();

        assert!(ts1.happens_before(&ts2));
        assert!(ts2.happens_before(&ts3));
    }

    #[test]
    fn test_version_visibility() {
        let ts1 = HybridTimestamp::new(100, 0, 1);
        let ts2 = HybridTimestamp::new(200, 0, 1);
        let ts3 = HybridTimestamp::new(300, 0, 1);

        let version: VersionedRecord<String> = VersionedRecord::new(
            "data".to_string(),
            1,
            ts2,
            1,
        );

        assert!(!version.is_visible_to(&ts1)); // Created after read time
        assert!(version.is_visible_to(&ts2));  // Created at read time
        assert!(version.is_visible_to(&ts3));  // Created before read time
    }

    #[test]
    fn test_mvcc_read_write() {
        let config = MVCCConfig::default();
        let mvcc: MVCCManager<String, String> = MVCCManager::new(config);

        let ts1 = mvcc.begin_snapshot(1);
        mvcc.write("key1".to_string(), "value1".to_string(), 1, ts1).unwrap();

        let ts2 = mvcc.begin_snapshot(2);
        let _value = mvcc.read(&"key1".to_string(), &ts2).unwrap();
        assert_eq!(value, Some("value1".to_string()));

        mvcc.end_snapshot(1);
        mvcc.end_snapshot(2);
    }

    #[test]
    fn test_snapshot_isolation_conflict() {
        let config = SnapshotConfig::default();
        let si = SnapshotIsolationManager::new(config);

        let _ts1 = si.begin_transaction(1, false);
        si.record_write(1, "key1".to_string()).unwrap();

        let _ts2 = si.begin_transaction(2, false);
        si.record_write(2, "key1".to_string()).unwrap();

        // Should detect write-write conflict
        assert!(si.check_write_conflicts(1).is_err() || si.check_write_conflicts(2).is_err());
    }
}


