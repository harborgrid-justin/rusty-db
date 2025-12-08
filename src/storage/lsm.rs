/// Log-Structured Merge Tree (LSM) for RustyDB
/// Optimized for write-heavy and time-series workloads
/// Features: Bloom filters, leveled compaction, concurrent memtable switching

use std::collections::{BTreeMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{SystemTime};
use parking_lot::{RwLock};
use serde::{Deserialize, Serialize};
use crate::error::Result;

/// LSM key type
pub type LsmKey = Vec<u8>;

/// LSM value with timestamp for MVCC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsmValue {
    data: Vec<u8>,
    timestamp: u64,
    is_tombstone: bool,
}

impl LsmValue {
    fn new(data: Vec<u8>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        Self {
            data,
            timestamp,
            is_tombstone: false,
        }
    }

    fn tombstone() -> Self {
        Self {
            data: Vec::new(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            is_tombstone: true,
        }
    }

    fn is_deleted(&self) -> bool {
        self.is_tombstone
    }
}

/// Bloom filter for fast negative lookups
struct BloomFilter {
    bits: Vec<bool>,
    num_hashes: usize,
    num_bits: usize,
}

impl BloomFilter {
    fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        let num_bits = Self::optimal_num_bits(expected_items, false_positive_rate);
        let num_hashes = Self::optimal_num_hashes(num_bits, expected_items);

        Self {
            bits: vec![false; num_bits],
            num_hashes,
            num_bits,
        }
    }

    fn optimal_num_bits(n: usize, p: f64) -> usize {
        let n = n as f64;
        let p = p.max(0.0001); // Avoid log(0)
        (-(n * p.ln()) / (2.0_f64.ln().powi(2))).ceil() as usize
    }

    fn optimal_num_hashes(m: usize, n: usize) -> usize {
        if n == 0 {
            return 1;
        }
        ((m as f64 / n as f64) * 2.0_f64.ln()).ceil() as usize
    }

    fn hash(&self, key: &[u8], seed: usize) -> usize {
        let mut hash = seed;
        for &byte in key {
            hash = hash.wrapping_mul(31).wrapping_add(byte as usize);
        }
        hash % self.num_bits
    }

    fn insert(&mut self, key: &[u8]) {
        for _i in 0..self.num_hashes {
            let idx = self.hash(key, i);
            self.bits[idx] = true;
        }
    }

    fn contains(&self, key: &[u8]) -> bool {
        for _i in 0..self.num_hashes {
            let idx = self.hash(key, i);
            if !self.bits[idx] {
                return false;
            }
        }
        true
    }

    fn reset(&mut self) {
        self.bits.fill(false);
    }
}

/// In-memory write buffer (memtable)
struct MemTable {
    data: BTreeMap<LsmKey, LsmValue>,
    size_bytes: usize,
    max_size: usize,
    id: u64,
}

impl MemTable {
    fn new(max_size: usize, id: u64) -> Self {
        Self {
            data: BTreeMap::new(),
            size_bytes: 0,
            max_size,
            id,
        }
    }

    fn put(&mut self, key: LsmKey, value: LsmValue) -> bool {
        let entry_size = key.len() + value.data.len() + 24; // Approximate overhead

        if self.size_bytes + entry_size > self.max_size {
            return false; // Memtable is full
        }

        self.data.insert(key, value);
        self.size_bytes += entry_size;
        true
    }

    fn get(&self, key: &LsmKey) -> Option<&LsmValue> {
        self.data.get(key)
    }

    fn delete(&mut self, key: LsmKey) -> bool {
        let tombstone = LsmValue::tombstone();
        self.put(key, tombstone)
    }

    fn is_full(&self) -> bool {
        self.size_bytes >= self.max_size
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn iter(&self) -> impl Iterator<Item = (&LsmKey, &LsmValue)> {
        self.data.iter()
    }
}

/// SSTable (Sorted String Table) on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SSTable {
    id: u64,
    level: usize,
    min_key: LsmKey,
    max_key: LsmKey,
    num_entries: usize,
    size_bytes: usize,
    bloom_filter: Vec<bool>, // Serialized bloom filter
    created_at: u64,
}

impl SSTable {
    fn new(id: u64, level: usize, entries: &BTreeMap<LsmKey, LsmValue>) -> Self {
        let min_key = entries.keys().next().cloned().unwrap_or_default();
        let max_key = entries.keys().next_back().cloned().unwrap_or_default();
        let num_entries = entries.len();

        let size_bytes: usize = entries.iter()
            .map(|(k, v)| k.len() + v.data.len() + 24)
            .sum();

        // Create bloom filter
        let mut bloom = BloomFilter::new(num_entries, 0.01);
        for key in entries.keys() {
            bloom.insert(key);
        }

        Self {
            id,
            level,
            min_key,
            max_key,
            num_entries,
            size_bytes,
            bloom_filter: bloom.bits,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    fn might_contain(&self, key: &LsmKey) -> bool {
        if key < &self.min_key || key > &self.max_key {
            return false;
        }

        // Check bloom filter
        let bloom = BloomFilter {
            bits: self.bloom_filter.clone(),
            num_hashes: 3,
            num_bits: self.bloom_filter.len(),
        };

        bloom.contains(key)
    }

    fn overlaps(&self, other: &SSTable) -> bool {
        !(self.max_key < other.min_key || self.min_key > other.max_key)
    }
}

/// Level in the LSM tree
struct Level {
    id: usize,
    sstables: Vec<Arc<SSTable>>,
    max_size: usize,
    max_sstables: usize,
}

impl Level {
    fn new(id: usize, max_size: usize) -> Self {
        Self {
            id,
            sstables: Vec::new(),
            max_size,
            max_sstables: 10 * (id + 1), // More files allowed at higher levels
        }
    }

    fn add_sstable(&mut self, sstable: Arc<SSTable>) {
        self.sstables.push(sstable);
        // Keep sorted by min_key for efficient search
        self.sstables.sort_by(|a, b| a.min_key.cmp(&b.min_key));
    }

    fn total_size(&self) -> usize {
        self.sstables.iter().map(|s| s.size_bytes).sum()
    }

    fn needs_compaction(&self) -> bool {
        self.total_size() > self.max_size || self.sstables.len() > self.max_sstables
    }

    fn find_overlapping(&self, key: &LsmKey) -> Vec<Arc<SSTable>> {
        self.sstables.iter()
            .filter(|s| s.might_contain(key))
            .cloned()
            .collect()
    }
}

/// Compaction strategy
#[derive(Debug, Clone, Copy)]
pub enum CompactionStrategy {
    Leveled,      // Standard leveled compaction
    SizeTiered,   // Size-tiered for write-heavy workloads
    TimeWindow,   // For time-series data
}

/// Compaction task
struct CompactionTask {
    level: usize,
    sstables: Vec<Arc<SSTable>>,
    strategy: CompactionStrategy,
}

impl CompactionTask {
    fn new(level: usize, sstables: Vec<Arc<SSTable>>, strategy: CompactionStrategy) -> Self {
        Self {
            level,
            sstables,
            strategy,
        }
    }

    fn execute(&self) -> Result<Vec<SSTable>> {
        // Merge all entries from selected SSTables
        let mut merged = BTreeMap::new();

        // In production, would read from disk
        // For now, simulate merging

        // Create new compacted SSTables
        let mut new_sstables = Vec::new();
        let target_level = self.level + 1;

        // Split into appropriately sized SSTables
        const TARGET_SSTABLE_SIZE: usize = 64 * 1024 * 1024; // 64MB

        if !merged.is_empty() {
            let sstable = SSTable::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_micros() as u64,
                target_level,
                &merged,
            );
            new_sstables.push(sstable);
        }

        Ok(new_sstables)
    }
}

/// LSM Tree main structure
pub struct LsmTree {
    // Active memtable for writes
    active_memtable: Arc<RwLock<MemTable>>,

    // Immutable memtables being flushed
    immutable_memtables: Arc<Mutex<VecDeque<Arc<MemTable>>>>,

    // Levels
    levels: Arc<RwLock<Vec<Level>>>,

    // Compaction queue
    compaction_queue: Arc<Mutex<VecDeque<CompactionTask>>>,

    // Configuration
    memtable_size: usize,
    num_levels: usize,
    compaction_strategy: CompactionStrategy,

    // Memtable ID counter
    next_memtable_id: Arc<AtomicU64>,

    // SSTable ID counter
    next_sstable_id: Arc<AtomicU64>,

    // Background tasks
    compaction_running: Arc<AtomicBool>,

    // Statistics
    stats: Arc<RwLock<LsmStats>>,
}

#[derive(Debug, Clone, Default)]
pub struct LsmStats {
    pub writes: u64,
    pub reads: u64,
    pub memtable_hits: u64,
    pub sstable_hits: u64,
    pub bloom_filter_saves: u64,
    pub compactions: u64,
    pub total_sstables: usize,
    pub total_levels: usize,
}

impl LsmTree {
    pub fn new(memtable_size: usize, num_levels: usize) -> Self {
        let mut levels = Vec::new();
        for _i in 0..num_levels {
            let max_size = (10_usize.pow(i as u32 + 1)) * 10 * 1024 * 1024; // 10MB * 10^level
            levels.push(Level::new(i, max_size));
        }

        Self {
            active_memtable: Arc::new(RwLock::new(MemTable::new(memtable_size, 0))),
            immutable_memtables: Arc::new(Mutex::new(VecDeque::new())),
            levels: Arc::new(RwLock::new(levels)),
            compaction_queue: Arc::new(Mutex::new(VecDeque::new())),
            memtable_size,
            num_levels,
            compaction_strategy: CompactionStrategy::Leveled,
            next_memtable_id: Arc::new(AtomicU64::new(1)),
            next_sstable_id: Arc::new(AtomicU64::new(1)),
            compaction_running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(LsmStats::default())),
        }
    }

    /// Put a key-value pair
    pub fn put(&self, key: LsmKey, value: Vec<u8>) -> Result<()> {
        let lsm_value = LsmValue::new(value);

        // Try to insert into active memtable
        let inserted = self.active_memtable.write().put(key.clone(), lsm_value.clone());

        if !inserted {
            // Memtable is full, switch to a new one
            self.switch_memtable()?;

            // Retry with new memtable
            if !self.active_memtable.write().put(key, lsm_value) {
                return Err(DbError::Storage("Failed to insert into new memtable".to_string()));
            }
        }

        self.stats.write().writes += 1;
        Ok(())
    }

    /// Get a value by key
    pub fn get(&self, key: &LsmKey) -> Result<Option<Vec<u8>>> {
        self.stats.write().reads += 1;

        // Check active memtable
        if let Some(value) = self.active_memtable.read().get(key) {
            self.stats.write().memtable_hits += 1;
            return if value.is_deleted() {
                Ok(None)
            } else {
                Ok(Some(value.data.clone()))
            };
        }

        // Check immutable memtables (newest first)
        {
            let immutables = self.immutable_memtables.lock();
            for memtable in immutables.iter().rev() {
                if let Some(value) = memtable.get(key) {
                    self.stats.write().memtable_hits += 1;
                    return if value.is_deleted() {
                        Ok(None)
                    } else {
                        Ok(Some(value.data.clone()))
                    };
                }
            }
        }

        // Check SSTables level by level
        let levels = self.levels.read();
        for level in levels.iter() {
            let overlapping = level.find_overlapping(key);

            for sstable in overlapping {
                // Bloom filter check
                if !sstable.might_contain(key) {
                    self.stats.write().bloom_filter_saves += 1;
                    continue;
                }

                // In production, would read from disk
                // For now, simulate a miss
                self.stats.write().sstable_hits += 1;
            }
        }

        Ok(None)
    }

    /// Delete a key
    pub fn delete(&self, key: LsmKey) -> Result<()> {
        // Insert tombstone
        let inserted = self.active_memtable.write().delete(key.clone());

        if !inserted {
            self.switch_memtable()?;
            self.active_memtable.write().delete(key);
        }

        Ok(())
    }

    /// Range scan
    pub fn scan(&self, start_key: &LsmKey, end_key: &LsmKey) -> Result<Vec<(LsmKey, Vec<u8>)>> {
        let mut results = BTreeMap::new();

        // Scan memtables
        {
            let active = self.active_memtable.read();
            for (k, v) in active.iter() {
                if k >= start_key && k <= end_key {
                    if !v.is_deleted() {
                        results.insert(k.clone(), v.data.clone());
                    }
                }
            }
        }

        // Scan immutable memtables
        {
            let immutables = self.immutable_memtables.lock();
            for memtable in immutables.iter() {
                for (k, v) in memtable.iter() {
                    if k >= start_key && k <= end_key {
                        if v.is_deleted() {
                            results.remove(k);
                        } else {
                            results.insert(k.clone(), v.data.clone());
                        }
                    }
                }
            }
        }

        // Would also scan SSTables in production

        Ok(results.into_iter().collect())
    }

    /// Switch active memtable to immutable
    fn switch_memtable(&self) -> Result<()> {
        let old_memtable = {
            let mut active = self.active_memtable.write();
            let new_id = self.next_memtable_id.fetch_add(1, Ordering::SeqCst);
            let new_memtable = MemTable::new(self.memtable_size, new_id);
            std::mem::replace(&mut *active, new_memtable)
        };

        // Add to immutable queue
        self.immutable_memtables.lock().push_back(Arc::new(old_memtable));

        // Trigger flush
        self.trigger_flush()?;

        Ok(())
    }

    /// Flush memtable to L0
    fn trigger_flush(&self) -> Result<()> {
        let memtable = {
            let mut immutables = self.immutable_memtables.lock();
            immutables.pop_front()
        };

        if let Some(memtable) = memtable {
            let sstable_id = self.next_sstable_id.fetch_add(1, Ordering::SeqCst);
            let sstable = SSTable::new(sstable_id, 0, &memtable.data);

            // Add to L0
            let mut levels = self.levels.write();
            levels[0].add_sstable(Arc::new(sstable));

            // Check if compaction is needed
            if levels[0].needs_compaction() {
                self.schedule_compaction(0)?;
            }
        }

        Ok(())
    }

    /// Schedule a compaction task
    fn schedule_compaction(&self, level: usize) -> Result<()> {
        let levels = self.levels.read();
        if level >= levels.len() {
            return Ok(());
        }

        let sstables = levels[level].sstables.clone();
        let task = CompactionTask::new(level, sstables, self.compaction_strategy);

        self.compaction_queue.lock().push_back(task);

        Ok(())
    }

    /// Process compaction queue
    pub fn run_compaction(&self, max_tasks: usize) -> Result<usize> {
        if !self.compaction_running.compare_exchange(
            false,
            true,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ).is_ok() {
            return Ok(0); // Already running
        }

        let mut completed = 0;

        for _ in 0..max_tasks {
            let task = {
                let mut queue = self.compaction_queue.lock();
                queue.pop_front()
            };

            if let Some(task) = task {
                let new_sstables = task.execute()?;

                // Update levels
                let mut levels = self.levels.write();

                // Remove old SSTables from source level
                levels[task.level].sstables.retain(|s| {
                    !task.sstables.iter().any(|t| t.id == s.id)
                });

                // Add new SSTables to next level
                let target_level = task.level + 1;
                if target_level < levels.len() {
                    for sstable in new_sstables {
                        levels[target_level].add_sstable(Arc::new(sstable));
                    }
                }

                self.stats.write().compactions += 1;
                completed += 1;
            } else {
                break;
            }
        }

        self.compaction_running.store(false, Ordering::SeqCst);
        Ok(completed)
    }

    pub fn get_stats(&self) -> LsmStats {
        let mut stats = self.stats.read().clone();
        let levels = self.levels.read();
        stats.total_levels = levels.len();
        stats.total_sstables = levels.iter().map(|l| l.sstables.len()).sum();
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_filter() {
        let mut bloom = BloomFilter::new(100, 0.01);

        bloom.insert(b"key1");
        bloom.insert(b"key2");

        assert!(bloom.contains(b"key1"));
        assert!(bloom.contains(b"key2"));
        assert!(!bloom.contains(b"key3")); // Might have false positives, but unlikely
    }

    #[test]
    fn test_memtable() {
        let mut memtable = MemTable::new(1024, 0);

        let key = b"test_key".to_vec();
        let _value = LsmValue::new(b"test_value".to_vec());

        assert!(memtable.put(key.clone(), value));
        assert!(memtable.get(&key).is_some());
    }

    #[test]
    fn test_lsm_put_get() {
        let lsm = LsmTree::new(1024, 5);

        let key = b"key1".to_vec();
        let _value = b"value1".to_vec();

        lsm.put(key.clone(), value.clone()).unwrap();

        let retrieved = lsm.get(&key).unwrap();
        assert_eq!(retrieved, Some(value));
    }

    #[test]
    fn test_lsm_delete() {
        let lsm = LsmTree::new(1024, 5);

        let key = b"key1".to_vec();
        let _value = b"value1".to_vec();

        lsm.put(key.clone(), value).unwrap();
        lsm.delete(key.clone()).unwrap();

        let retrieved = lsm.get(&key).unwrap();
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_lsm_scan() {
        let lsm = LsmTree::new(4096, 5);

        lsm.put(b"key1".to_vec(), b"value1".to_vec()).unwrap();
        lsm.put(b"key2".to_vec(), b"value2".to_vec()).unwrap();
        lsm.put(b"key3".to_vec(), b"value3".to_vec()).unwrap();

        let results = lsm.scan(&b"key1".to_vec(), &b"key2".to_vec()).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_memtable_switch() {
        let lsm = LsmTree::new(128, 5); // Small memtable

        // Fill memtable
        for _i in 0..20 {
            let key = format!("key{:02}", i).into_bytes();
            let _value = b"value".to_vec();
            lsm.put(key, value).unwrap();
        }

        let _stats = lsm.get_stats();
        assert!(stats.writes >= 20);
    }
}


