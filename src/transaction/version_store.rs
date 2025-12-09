// MVCC Version Store implementation.
//
// This module provides the version storage mechanism for Multi-Version
// Concurrency Control (MVCC), enabling non-blocking reads and consistent
// snapshots.
//
// # Key Concepts
//
// - Each data item can have multiple versions.
// - Readers see a consistent snapshot based on transaction timestamp.
// - Old versions are garbage collected when no longer needed.
//
// # Example
//
// ```rust,ignore
// let store = VersionStore::new();
// store.add_version("key1".to_string(), version);
// let visible = store.get_version("key1", txn_id, snapshot_time);
// ```

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration};

use parking_lot::{Mutex, RwLock};

use crate::common::TransactionId;

use super::types::Version;

/// Version store for MVCC.
///
/// Maintains multiple versions of data items for concurrent access.
/// Provides visibility checks based on transaction timestamps.
///
/// # Thread Safety
///
/// All operations are thread-safe via internal locking.
pub struct VersionStore {
    /// Map of key -> list of versions (newest first after sorting).
    versions: Arc<RwLock<HashMap<String, Vec<Version>>>>,
    /// Garbage collector for old versions.
    garbage_collector: Arc<Mutex<GarbageCollector>>,
}

impl VersionStore {
    /// Creates a new version store.
    pub fn new() -> Self {
        Self {
            versions: Arc::new(RwLock::new(HashMap::new())),
            garbage_collector: Arc::new(Mutex::new(GarbageCollector::new())),
        }
    }

    /// Creates a version store with custom garbage collection interval.
    pub fn with_gc_interval(interval: Duration) -> Self {
        Self {
            versions: Arc::new(RwLock::new(HashMap::new())),
            garbage_collector: Arc::new(Mutex::new(GarbageCollector::with_interval(interval))),
        }
    }

    /// Adds a new version for a key.
    ///
    /// # Arguments
    ///
    /// * `key` - The data item key.
    /// * `version` - The version to add.
    pub fn add_version(&self, key: String, version: Version) {
        let mut versions = self.versions.write();
        versions.entry(key).or_default().push(version);
    }

    /// Gets the visible version for a transaction.
    ///
    /// Returns the latest version that is visible to the given transaction
    /// based on the snapshot timestamp.
    ///
    /// # Arguments
    ///
    /// * `key` - The data item key.
    /// * `txn_id` - The reading transaction's ID.
    /// * `snapshot_ts` - The snapshot timestamp.
    ///
    /// # Returns
    ///
    /// The visible version, or `None` if no visible version exists.
    pub fn get_version(
        &self,
        key: &str,
        txn_id: TransactionId,
        snapshot_ts: SystemTime,
    ) -> Option<Version> {
        let versions = self.versions.read();

        if let Some(version_list) = versions.get(key) {
            // Find the latest version visible to this transaction.
            // Iterate in reverse to find the most recent visible version.
            for version in version_list.iter().rev() {
                if version.timestamp <= snapshot_ts && version.txn_id != txn_id {
                    if !version.is_deleted {
                        return Some(version.clone());
                    }
                }
            }
        }

        None
    }

    /// Gets the version created by a specific transaction.
    ///
    /// Useful for reading your own writes within a transaction.
    ///
    /// # Arguments
    ///
    /// * `key` - The data item key.
    /// * `txn_id` - The transaction ID to look for.
    ///
    /// # Returns
    ///
    /// The version created by this transaction, if any.
    pub fn get_version_by_txn(&self, key: &str, txn_id: TransactionId) -> Option<Version> {
        let versions = self.versions.read();

        if let Some(version_list) = versions.get(key) {
            for version in version_list.iter().rev() {
                if version.txn_id == txn_id {
                    return Some(version.clone());
                }
            }
        }

        None
    }

    /// Gets all versions for a key.
    ///
    /// Primarily useful for debugging and testing.
    pub fn get_all_versions(&self, key: &str) -> Vec<Version> {
        let versions = self.versions.read();
        versions.get(key).cloned().unwrap_or_default()
    }

    /// Returns the number of keys with versions.
    pub fn key_count(&self) -> usize {
        self.versions.read().len()
    }

    /// Returns the total number of versions across all keys.
    pub fn version_count(&self) -> usize {
        self.versions.read().values().map(|v| v.len()).sum()
    }

    /// Runs garbage collection to remove old versions.
    ///
    /// Removes versions that are no longer needed by any active transaction.
    ///
    /// # Arguments
    ///
    /// * `min_active_txn` - The minimum active transaction ID.
    ///   Versions from transactions older than this may be collected.
    pub fn cleanup(&self, min_active_txn: TransactionId) {
        let mut gc = self.garbage_collector.lock();
        gc.collect(&self.versions, min_active_txn);
    }

    /// Forces immediate garbage collection regardless of interval.
    pub fn force_cleanup(&self, min_active_txn: TransactionId) {
        let mut gc = self.garbage_collector.lock();
        gc.force_collect(&self.versions, min_active_txn);
    }

    /// Removes all versions for a key.
    pub fn remove_key(&self, key: &str) {
        let mut versions = self.versions.write();
        versions.remove(key);
    }

    /// Clears all versions from the store.
    pub fn clear(&self) {
        let mut versions = self.versions.write();
        versions.clear();
    }
}

impl Default for VersionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for VersionStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let versions = self.versions.read();
        f.debug_struct("VersionStore")
            .field("key_count", &versions.len())
            .field("version_count", &versions.values().map(|v| v.len()).sum::<usize>())
            .finish()
    }
}

/// Garbage collector for removing old versions.
///
/// Periodically removes versions that are no longer needed by any
/// active transaction, freeing memory.
///
/// # Collection Strategy
///
/// Versions are removed if:
/// 1. They were created by a transaction older than `min_active_txn`.
/// 2. There is a newer version for the same key.
pub struct GarbageCollector {
    /// Timestamp of last cleanup.
    last_cleanup: SystemTime,
    /// Minimum interval between cleanups.
    cleanup_interval: Duration,
    /// Statistics.
    stats: GCStats,
}

/// Garbage collection statistics.
#[derive(Debug, Default, Clone)]
pub struct GCStats {
    /// Total number of GC runs.
    pub runs: u64,
    /// Total versions removed.
    pub versions_removed: u64,
    /// Total keys compacted.
    pub keys_compacted: u64,
}

impl GarbageCollector {
    /// Creates a new garbage collector with default settings.
    pub fn new() -> Self {
        Self {
            last_cleanup: SystemTime::now(),
            cleanup_interval: Duration::from_secs(60),
            stats: GCStats::default(),
        }
    }

    /// Creates a garbage collector with custom interval.
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            last_cleanup: SystemTime::now(),
            cleanup_interval: interval,
            stats: GCStats::default(),
        }
    }

    /// Collects garbage if enough time has passed since last collection.
    ///
    /// # Arguments
    ///
    /// * `versions` - Reference to the version store.
    /// * `min_active_txn` - Minimum active transaction ID.
    pub fn collect(
        &mut self,
        versions: &Arc<RwLock<HashMap<String, Vec<Version>>>>,
        min_active_txn: TransactionId,
    ) {
        let now = SystemTime::now();
        let elapsed = now
            .duration_since(self.last_cleanup)
            .unwrap_or(Duration::ZERO);

        if elapsed < self.cleanup_interval {
            return;
        }

        self.force_collect(versions, min_active_txn);
    }

    /// Forces garbage collection regardless of interval.
    pub fn force_collect(
        &mut self,
        versions: &Arc<RwLock<HashMap<String, Vec<Version>>>>,
        min_active_txn: TransactionId,
    ) {
        let mut versions_map = versions.write();
        let mut removed = 0u64;

        for version_list in versions_map.values_mut() {
            let before_len = version_list.len();

            // Keep only versions from active or newer transactions.
            // Also keep the latest version even if old (for visibility).
            if version_list.len() > 1 {
                // Sort by timestamp and keep the newest + any from active transactions.
                let newest_ts = version_list
                    .iter()
                    .map(|v| v.timestamp)
                    .max()
                    .unwrap_or(SystemTime::UNIX_EPOCH);

                version_list.retain(|v| {
                    v.txn_id >= min_active_txn || v.timestamp == newest_ts
                });
            }

            removed += (before_len - version_list.len()) as u64;
        }

        // Remove empty entries.
        let before_keys = versions_map.len();
        versions_map.retain(|_, v| !v.is_empty());
        let keys_removed = before_keys - versions_map.len();

        // Update statistics.
        self.stats.runs += 1;
        self.stats.versions_removed += removed;
        self.stats.keys_compacted += keys_removed as u64;
        self.last_cleanup = SystemTime::now();
    }

    /// Returns garbage collection statistics.
    pub fn stats(&self) -> &GCStats {
        &self.stats
    }
}

impl Default for GarbageCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::UNIX_EPOCH;

    fn make_version(txn_id: TransactionId, data: &[u8]) -> Version {
        Version {
            txn_id,
            timestamp: SystemTime::now(),
            lsn: txn_id as u64,
            data: data.to_vec(),
            is_deleted: false,
        }
    }

    #[test]
    fn test_version_store_add_and_get() {
        let store = VersionStore::new();

        let v1 = make_version(1, b"value1");
        store.add_version("key1".to_string(), v1);

        let versions = store.get_all_versions("key1");
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].data, b"value1");
    }

    #[test]
    fn test_version_store_multiple_versions() {
        let store = VersionStore::new();

        store.add_version("key1".to_string(), make_version(1, b"v1"));
        store.add_version("key1".to_string(), make_version(2, b"v2"));
        store.add_version("key1".to_string(), make_version(3, b"v3"));

        let versions = store.get_all_versions("key1");
        assert_eq!(versions.len(), 3);
    }

    #[test]
    fn test_version_store_key_count() {
        let store = VersionStore::new();

        store.add_version("key1".to_string(), make_version(1, b"v1"));
        store.add_version("key2".to_string(), make_version(2, b"v2"));

        assert_eq!(store.key_count(), 2);
        assert_eq!(store.version_count(), 2);
    }

    #[test]
    fn test_version_store_clear() {
        let store = VersionStore::new();

        store.add_version("key1".to_string(), make_version(1, b"v1"));
        store.add_version("key2".to_string(), make_version(2, b"v2"));

        store.clear();
        assert_eq!(store.key_count(), 0);
    }

    #[test]
    fn test_garbage_collector_stats() {
        let gc = GarbageCollector::new();
        assert_eq!(gc.stats().runs, 0);
    }
}
