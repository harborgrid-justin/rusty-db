/// Page Table - Partitioned Hash Map for concurrent page lookups

use crate::common::PageId;
use crate::buffer::page_cache::FrameId;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Partitioned page table for concurrent access.
///
/// Uses multiple hash maps (partitions) to reduce lock contention.
/// Page IDs are hashed to determine which partition to use.
#[allow(dead_code)]
pub struct PageTable {
    /// Partitions (each is a separate hash map)
    partitions: Vec<RwLock<HashMap<PageId, FrameId>>>,

    /// Number of partitions
    num_partitions: usize,

    /// Lookup statistics
    lookups: AtomicU64,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl PageTable {
    /// Create a new partitioned page table
    #[allow(dead_code)]
    pub fn new(num_partitions: usize, initial_capacity_per_partition: usize) -> Self {
        let mut partitions = Vec::with_capacity(num_partitions);
        for _ in 0..num_partitions {
            partitions.push(RwLock::new(HashMap::with_capacity(
                initial_capacity_per_partition,
            )));
        }

        Self {
            partitions,
            num_partitions,
            lookups: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Get partition index for a page ID
    #[inline(always)]
    fn partition_index(&self, page_id: PageId) -> usize {
        // Fast hash: multiply by large prime and mask
        (page_id.wrapping_mul(0x9e3779b97f4a7c15) as usize) % self.num_partitions
    }

    /// Look up a page in the table
    #[inline]
    pub fn lookup(&self, page_id: PageId) -> Option<FrameId> {
        self.lookups.fetch_add(1, Ordering::Relaxed);

        let partition_idx = self.partition_index(page_id);
        // SAFETY: partition_idx is guaranteed to be < num_partitions
        let partition = unsafe { self.partitions.get_unchecked(partition_idx) };

        let result = partition.read().get(&page_id).copied();

        if result.is_some() {
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
        }

        result
    }

    /// Insert a page into the table
    #[inline]
    pub fn insert(&self, page_id: PageId, frame_id: FrameId) {
        let partition_idx = self.partition_index(page_id);
        // SAFETY: partition_idx is guaranteed to be < num_partitions
        let partition = unsafe { self.partitions.get_unchecked(partition_idx) };

        partition.write().insert(page_id, frame_id);
    }

    /// Remove a page from the table
    #[inline]
    pub fn remove(&self, page_id: PageId) -> Option<FrameId> {
        let partition_idx = self.partition_index(page_id);
        // SAFETY: partition_idx is guaranteed to be < num_partitions
        let partition = unsafe { self.partitions.get_unchecked(partition_idx) };

        partition.write().remove(&page_id)
    }

    /// Clear all partitions
    #[cold]
    pub fn clear(&self) {
        for partition in &self.partitions {
            partition.write().clear();
        }
        self.lookups.store(0, Ordering::Relaxed);
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
    }

    /// Get hit rate
    #[inline]
    pub fn hit_rate(&self) -> f64 {
        let lookups = self.lookups.load(Ordering::Relaxed);
        let hits = self.hits.load(Ordering::Relaxed);

        if lookups == 0 {
            0.0
        } else {
            hits as f64 / lookups as f64
        }
    }

    /// Get statistics
    #[cold]
    pub fn stats(&self) -> (u64, u64, u64, f64) {
        let lookups = self.lookups.load(Ordering::Relaxed);
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let hit_rate = self.hit_rate();

        (lookups, hits, misses, hit_rate)
    }

    /// Get total number of entries
    #[cold]
    pub fn len(&self) -> usize {
        self.partitions
            .iter()
            .map(|p| p.read().len())
            .sum()
    }

    /// Check if the page table is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_table() {
        let table = PageTable::new(4, 10);

        table.insert(1, 5);
        table.insert(2, 7);

        assert_eq!(table.lookup(1), Some(5));
        assert_eq!(table.lookup(2), Some(7));
        assert_eq!(table.lookup(3), None);

        table.remove(1);
        assert_eq!(table.lookup(1), None);
    }

    #[test]
    fn test_page_table_stats() {
        let table = PageTable::new(4, 10);

        table.insert(1, 5);

        assert_eq!(table.lookup(1), Some(5));
        assert_eq!(table.lookup(2), None);

        let (lookups, hits, misses, hit_rate) = table.stats();
        assert_eq!(lookups, 2);
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
        assert!((hit_rate - 0.5).abs() < 0.01);
    }
}
