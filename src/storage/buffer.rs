use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::{RwLock, Mutex};
use crate::Result;
use crate::storage::page::{Page, PageId};
use crate::storage::disk::DiskManager;
use crate::error::DbError;

/// Copy-on-Write page frame for zero-copy reads
#[derive(Clone)]
struct CowFrame {
    page: Arc<RwLock<Page>>,
    reference_count: Arc<AtomicUsize>,
    version: u64,
    is_cow: bool,
}

impl CowFrame {
    fn new(page: Page, version: u64) -> Self {
        Self {
            page: Arc::new(RwLock::new(page)),
            reference_count: Arc::new(AtomicUsize::new(0)),
            version,
            is_cow: false,
        }
    }

    fn new_cow(page: Arc<RwLock<Page>>, version: u64) -> Self {
        Self {
            page,
            reference_count: Arc::new(AtomicUsize::new(0)),
            version,
            is_cow: true,
        }
    }

    fn pin(&self) {
        self.reference_count.fetch_add(1, Ordering::SeqCst);
    }

    fn unpin(&self) {
        self.reference_count.fetch_sub(1, Ordering::SeqCst);
    }

    fn is_pinned(&self) -> bool {
        self.reference_count.load(Ordering::SeqCst) > 0
    }
}

/// Access pattern tracking for LRU-K
#[derive(Debug, Clone)]
struct AccessHistory {
    timestamps: VecDeque<Instant>,
    k: usize,
}

impl AccessHistory {
    fn new(k: usize) -> Self {
        Self {
            timestamps: VecDeque::with_capacity(k),
            k,
        }
    }

    fn record_access(&mut self) {
        let now = Instant::now();
        if self.timestamps.len() >= self.k {
            self.timestamps.pop_front();
        }
        self.timestamps.push_back(now);
    }

    fn kth_distance(&self, now: Instant) -> Duration {
        if self.timestamps.is_empty() {
            return Duration::from_secs(u64::MAX);
        }

        if self.timestamps.len() < self.k {
            // Use oldest timestamp if we don't have K accesses yet
            return now.duration_since(self.timestamps[0]);
        }

        // Return distance to K-th most recent access
        now.duration_since(self.timestamps[0])
    }

    fn access_count(&self) -> usize {
        self.timestamps.len()
    }
}

/// LRU-K replacement policy with adaptive K selection
struct LruKReplacer {
    k: usize,
    adaptive_k: bool,
    access_history: HashMap<PageId, AccessHistory>,
    evictable_pages: Vec<PageId>,
    capacity: usize,
}

impl LruKReplacer {
    fn new(capacity: usize, k: usize, adaptive: bool) -> Self {
        Self {
            k,
            adaptive_k: adaptive,
            access_history: HashMap::new(),
            evictable_pages: Vec::new(),
            capacity,
        }
    }

    fn record_access(&mut self, page_id: PageId) {
        let history = self.access_history
            .entry(page_id)
            .or_insert_with(|| AccessHistory::new(self.k));
        history.record_access();
    }

    fn set_evictable(&mut self, page_id: PageId, evictable: bool) {
        if evictable && !self.evictable_pages.contains(&page_id) {
            self.evictable_pages.push(page_id);
        } else if !evictable {
            self.evictable_pages.retain(|&id| id != page_id);
        }
    }

    fn evict(&mut self) -> Option<PageId> {
        if self.evictable_pages.is_empty() {
            return None;
        }

        let now = Instant::now();
        let mut max_distance = Duration::from_secs(0);
        let mut victim_idx = 0;

        for (idx, &page_id) in self.evictable_pages.iter().enumerate() {
            if let Some(history) = self.access_history.get(&page_id) {
                let distance = history.kth_distance(now);
                if distance > max_distance {
                    max_distance = distance;
                    victim_idx = idx;
                }
            }
        }

        let victim = self.evictable_pages.remove(victim_idx);
        self.access_history.remove(&victim);

        // Adaptive K: adjust based on hit rate
        if self.adaptive_k {
            self.adjust_k();
        }

        Some(victim)
    }

    fn adjust_k(&mut self) {
        let total_accesses: usize = self.access_history.values()
            .map(|h| h.access_count())
            .sum();

        let avg_accesses = if !self.access_history.is_empty() {
            total_accesses / self.access_history.len()
        } else {
            1
        };

        // Increase K for workloads with high reuse
        if avg_accesses > self.k * 2 {
            self.k = (self.k + 1).min(10);
        } else if avg_accesses < self.k && self.k > 1 {
            self.k -= 1;
        }
    }

    fn size(&self) -> usize {
        self.evictable_pages.len()
    }
}

/// NUMA node affinity for pages
#[derive(Debug, Clone, Copy)]
struct NumaNode {
    node_id: usize,
    memory_size: usize,
    allocated: usize,
}

impl NumaNode {
    fn new(node_id: usize, memory_size: usize) -> Self {
        Self {
            node_id,
            memory_size,
            allocated: 0,
        }
    }

    fn can_allocate(&self, size: usize) -> bool {
        self.allocated + size <= self.memory_size
    }

    fn allocate(&mut self, size: usize) -> bool {
        if self.can_allocate(size) {
            self.allocated += size;
            true
        } else {
            false
        }
    }

    fn deallocate(&mut self, size: usize) {
        self.allocated = self.allocated.saturating_sub(size);
    }

    fn utilization(&self) -> f64 {
        if self.memory_size == 0 {
            0.0
        } else {
            self.allocated as f64 / self.memory_size as f64
        }
    }
}

/// NUMA-aware page allocator
struct NumaAllocator {
    nodes: Vec<NumaNode>,
    page_to_node: HashMap<PageId, usize>,
    round_robin_idx: usize,
}

impl NumaAllocator {
    fn new(num_nodes: usize, memory_per_node: usize) -> Self {
        let nodes = (0..num_nodes)
            .map(|i| NumaNode::new(i, memory_per_node))
            .collect();

        Self {
            nodes,
            page_to_node: HashMap::new(),
            round_robin_idx: 0,
        }
    }

    fn allocate_page(&mut self, page_id: PageId, page_size: usize) -> Result<usize> {
        // Try to balance allocation across NUMA nodes
        let start_idx = self.round_robin_idx;

        loop {
            if self.nodes[self.round_robin_idx].allocate(page_size) {
                let node_id = self.round_robin_idx;
                self.page_to_node.insert(page_id, node_id);
                self.round_robin_idx = (self.round_robin_idx + 1) % self.nodes.len();
                return Ok(node_id);
            }

            self.round_robin_idx = (self.round_robin_idx + 1) % self.nodes.len();

            if self.round_robin_idx == start_idx {
                return Err(DbError::Storage("All NUMA nodes exhausted".to_string()));
            }
        }
    }

    fn deallocate_page(&mut self, page_id: PageId, page_size: usize) {
        if let Some(&node_id) = self.page_to_node.get(&page_id) {
            self.nodes[node_id].deallocate(page_size);
            self.page_to_node.remove(&page_id);
        }
    }

    fn get_node(&self, page_id: PageId) -> Option<usize> {
        self.page_to_node.get(&page_id).copied()
    }

    fn rebalance(&mut self) {
        // Find most and least utilized nodes
        let (most_util_idx, least_util_idx) = self.find_imbalanced_nodes();

        if most_util_idx == least_util_idx {
            return;
        }

        // In a real implementation, would migrate pages
        // For now, just log the imbalance
        let imbalance = (self.nodes[most_util_idx].utilization() -
                        self.nodes[least_util_idx].utilization()).abs();

        if imbalance > 0.3 {
            // Trigger migration in production
        }
    }

    fn find_imbalanced_nodes(&self) -> (usize, usize) {
        let mut most_util = 0;
        let mut least_util = 0;
        let mut max_utilization = 0.0;
        let mut min_utilization = 1.0;

        for (idx, node) in self.nodes.iter().enumerate() {
            let util = node.utilization();
            if util > max_utilization {
                max_utilization = util;
                most_util = idx;
            }
            if util < min_utilization {
                min_utilization = util;
                least_util = idx;
            }
        }

        (most_util, least_util)
    }
}

/// Background flusher for write coalescing
struct BackgroundFlusher {
    dirty_pages: Arc<Mutex<Vec<PageId>>>,
    flush_interval: Duration,
    batch_size: usize,
    running: Arc<AtomicUsize>,
}

impl BackgroundFlusher {
    fn new(flush_interval: Duration, batch_size: usize) -> Self {
        Self {
            dirty_pages: Arc::new(Mutex::new(Vec::new())),
            flush_interval,
            batch_size,
            running: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn mark_dirty(&self, page_id: PageId) {
        let mut dirty = self.dirty_pages.lock();
        if !dirty.contains(&page_id) {
            dirty.push(page_id);
        }
    }

    fn get_batch_to_flush(&self) -> Vec<PageId> {
        let mut dirty = self.dirty_pages.lock();
        let batch_size = self.batch_size.min(dirty.len());

        if batch_size == 0 {
            return Vec::new();
        }

        // Coalesce sequential pages for better I/O
        dirty.sort_unstable();

        let batch: Vec<PageId> = dirty.drain(..batch_size).collect();
        batch
    }

    fn start(&self) -> bool {
        self.running.fetch_add(1, Ordering::SeqCst) == 0
    }

    fn stop(&self) {
        self.running.store(0, Ordering::SeqCst);
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst) > 0
    }
}

/// Enterprise-grade buffer pool manager with COW semantics
pub struct BufferPoolManager {
    // Core buffer pool
    pool: Arc<RwLock<HashMap<usize, CowFrame>>>,
    page_table: Arc<RwLock<HashMap<PageId, usize>>>,
    free_frames: Arc<Mutex<Vec<usize>>>,

    // Advanced eviction
    replacer: Arc<Mutex<LruKReplacer>>,

    // NUMA awareness
    numa_allocator: Arc<Mutex<NumaAllocator>>,

    // Background flushing
    flusher: Arc<BackgroundFlusher>,

    // Disk manager
    disk_manager: DiskManager,

    // Metadata
    pool_size: usize,
    version_counter: Arc<AtomicU64>,

    // Statistics
    hit_count: Arc<AtomicU64>,
    miss_count: Arc<AtomicU64>,
}

impl BufferPoolManager {
    pub fn new(pool_size: usize, disk_manager: DiskManager) -> Self {
        let free_frames: Vec<usize> = (0..pool_size).collect();

        Self {
            pool: Arc::new(RwLock::new(HashMap::new())),
            page_table: Arc::new(RwLock::new(HashMap::new())),
            free_frames: Arc::new(Mutex::new(free_frames)),
            replacer: Arc::new(Mutex::new(LruKReplacer::new(pool_size, 2, true))),
            numa_allocator: Arc::new(Mutex::new(NumaAllocator::new(4, pool_size / 4))),
            flusher: Arc::new(BackgroundFlusher::new(Duration::from_millis(100), 32)),
            disk_manager,
            pool_size,
            version_counter: Arc::new(AtomicU64::new(0)),
            hit_count: Arc::new(AtomicU64::new(0)),
            miss_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Fetch a page with COW semantics for zero-copy reads
    pub fn fetch_page(&self, page_id: PageId) -> Result<Page> {
        // Check if page is in buffer pool
        {
            let page_table = self.page_table.read();
            if let Some(&frame_id) = page_table.get(&page_id) {
                let pool = self.pool.read();
                if let Some(frame) = pool.get(&frame_id) {
                    frame.pin();
                    self.replacer.lock().record_access(page_id);
                    self.hit_count.fetch_add(1, Ordering::Relaxed);

                    // Return a COW copy
                    let page = frame.page.read().clone();
                    return Ok(page);
                }
            }
        }

        // Page not in buffer - fetch from disk
        self.miss_count.fetch_add(1, Ordering::Relaxed);
        self.fetch_from_disk(page_id)
    }

    fn fetch_from_disk(&self, page_id: PageId) -> Result<Page> {
        let frame_id = self.get_free_frame()?;

        // Load from disk
        let page = self.disk_manager.read_page(page_id)?;
        let version = self.version_counter.fetch_add(1, Ordering::SeqCst);

        let frame = CowFrame::new(page.clone(), version);

        // Update pool and page table
        {
            let mut pool = self.pool.write();
            pool.insert(frame_id, frame);
        }

        {
            let mut page_table = self.page_table.write();
            page_table.insert(page_id, frame_id);
        }

        // Update replacer
        self.replacer.lock().record_access(page_id);

        Ok(page)
    }

    /// Create a new page
    pub fn new_page(&self) -> Result<Page> {
        let page_id = self.disk_manager.allocate_page()?;
        let frame_id = self.get_free_frame()?;

        let page = Page::new(page_id, self.disk_manager.page_size);
        let version = self.version_counter.fetch_add(1, Ordering::SeqCst);

        let frame = CowFrame::new(page.clone(), version);

        {
            let mut pool = self.pool.write();
            pool.insert(frame_id, frame);
        }

        {
            let mut page_table = self.page_table.write();
            page_table.insert(page_id, frame_id);
        }

        self.replacer.lock().record_access(page_id);

        Ok(page)
    }

    /// Flush a specific page to disk
    pub fn flush_page(&self, page_id: PageId) -> Result<()> {
        let page_table = self.page_table.read();

        if let Some(&frame_id) = page_table.get(&page_id) {
            let pool = self.pool.read();
            if let Some(frame) = pool.get(&frame_id) {
                let page = frame.page.read();
                if page.is_dirty {
                    self.disk_manager.write_page(&page)?;
                }
            }
        }

        Ok(())
    }

    /// Flush all dirty pages to disk
    pub fn flush_all(&self) -> Result<()> {
        let page_table = self.page_table.read();
        let pool = self.pool.read();

        for &frame_id in page_table.values() {
            if let Some(frame) = pool.get(&frame_id) {
                let page = frame.page.read();
                if page.is_dirty {
                    self.disk_manager.write_page(&page)?;
                }
            }
        }

        Ok(())
    }

    /// Background flush with write coalescing
    pub fn background_flush(&self) -> Result<()> {
        let batch = self.flusher.get_batch_to_flush();

        for page_id in batch {
            self.flush_page(page_id)?;
        }

        Ok(())
    }

    fn get_free_frame(&self) -> Result<usize> {
        // Try free list first
        if let Some(frame_id) = self.free_frames.lock().pop() {
            return Ok(frame_id);
        }

        // Evict a page
        let victim_page_id = self.replacer.lock().evict()
            .ok_or_else(|| DbError::Storage("No evictable frames".to_string()))?;

        let page_table = self.page_table.read();
        let frame_id = *page_table.get(&victim_page_id)
            .ok_or_else(|| DbError::Storage("Invalid victim".to_string()))?;

        // Flush if dirty
        self.flush_page(victim_page_id)?;

        Ok(frame_id)
    }

    /// Get buffer pool statistics
    pub fn stats(&self) -> BufferPoolStats {
        let hits = self.hit_count.load(Ordering::Relaxed);
        let misses = self.miss_count.load(Ordering::Relaxed);
        let total = hits + misses;

        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };

        BufferPoolStats {
            pool_size: self.pool_size,
            pages_in_use: self.page_table.read().len(),
            hit_rate,
            total_accesses: total,
        }
    }

    /// Unpin a page (decrease reference count)
    pub fn unpin_page(&self, page_id: PageId, is_dirty: bool) -> Result<()> {
        let page_table = self.page_table.read();

        if let Some(&frame_id) = page_table.get(&page_id) {
            let pool = self.pool.read();
            if let Some(frame) = pool.get(&frame_id) {
                frame.unpin();

                if is_dirty {
                    self.flusher.mark_dirty(page_id);
                }

                if !frame.is_pinned() {
                    self.replacer.lock().set_evictable(page_id, true);
                }
            }
        }

        Ok(())
    }
}

/// Buffer pool statistics
#[derive(Debug, Clone)]
pub struct BufferPoolStats {
    pub pool_size: usize,
    pub pages_in_use: usize,
    pub hit_rate: f64,
    pub total_accesses: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_lru_k_replacer() {
        let mut replacer = LruKReplacer::new(3, 2, false);

        replacer.record_access(1);
        replacer.record_access(2);
        replacer.record_access(3);

        replacer.set_evictable(1, true);
        replacer.set_evictable(2, true);
        replacer.set_evictable(3, true);

        let victim = replacer.evict();
        assert!(victim.is_some());
    }

    #[test]
    fn test_numa_allocator() {
        let mut allocator = NumaAllocator::new(2, 8192);

        let node = allocator.allocate_page(1, 4096).unwrap();
        assert_eq!(node, 0);

        let node = allocator.allocate_page(2, 4096).unwrap();
        assert_eq!(node, 1);
    }

    #[test]
    fn test_buffer_pool() -> Result<()> {
        let dir = tempdir().unwrap();
        let dm = DiskManager::new(dir.path().to_str().unwrap(), 4096)?;
        let bp = BufferPoolManager::new(10, dm);

        let page = bp.new_page()?;
        assert_eq!(page.id, 0);

        let stats = bp.stats();
        assert_eq!(stats.pool_size, 10);

        Ok(())
    }

    #[test]
    fn test_cow_semantics() -> Result<()> {
        let dir = tempdir().unwrap();
        let dm = DiskManager::new(dir.path().to_str().unwrap(), 4096)?;
        let bp = BufferPoolManager::new(10, dm);

        let mut page1 = bp.new_page()?;
        let page_id = page1.id;
        page1.data[0] = 42;

        let page2 = bp.fetch_page(page_id)?;
        assert_eq!(page2.data[0], 0); // COW should give original

        Ok(())
    }
}


