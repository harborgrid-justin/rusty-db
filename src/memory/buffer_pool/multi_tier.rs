// Multi-Tier Buffer Pool
//
// Hot/Warm/Cold tier management.

use super::common::*;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::collections::{HashMap, VecDeque};
use parking_lot::{Mutex, RwLock as PRwLock};

// Note: BufferFrame, NumaNode, and BufferPoolConfig are now in common.rs

// Multi-tier buffer pool implementation
pub struct MultiTierBufferPool {
    config: BufferPoolConfig,
    // Hot tier frames
    hot_frames: Arc<Mutex<Vec<Arc<BufferFrame>>>>,
    // Warm tier frames
    warm_frames: Arc<Mutex<Vec<Arc<BufferFrame>>>>,
    // Cold tier frames
    cold_frames: Arc<Mutex<Vec<Arc<BufferFrame>>>>,
    // Keep pool frames (pinned pages)
    keep_frames: Arc<Mutex<Vec<Arc<BufferFrame>>>>,
    // Recycle pool frames (sequential access)
    recycle_frames: Arc<Mutex<Vec<Arc<BufferFrame>>>>,
    // Per-tablespace pools
    tablespace_pools: Arc<Mutex<HashMap<u32, Vec<Arc<BufferFrame>>>>>,
    // Page table mapping PageId to BufferFrame
    page_table: Arc<PRwLock<HashMap<PageId, Arc<BufferFrame>>>>,
    // Free frames list
    free_frames: Arc<Mutex<VecDeque<Arc<BufferFrame>>>>,
    // Background tier management thread handle
    tier_manager_running: Arc<AtomicBool>,
    // Statistics
    stats: Arc<BufferPoolStats>,
}

impl MultiTierBufferPool {
    // Create a new multi-tier buffer pool
    pub fn new(config: BufferPoolConfig) -> Self {
        let total_frames = config.total_size / config.page_size;
        let hot_frames_count = (total_frames as f64 * config.hot_tier_ratio) as usize;
        let warm_frames_count = (total_frames as f64 * config.warm_tier_ratio) as usize;
        let cold_frames_count = total_frames - hot_frames_count - warm_frames_count;

        let mut all_frames = Vec::new();

        // Create frames for hot tier
        let mut hot_frames = Vec::new();
        for _ in 0..hot_frames_count {
            let frame = Arc::new(BufferFrame::new(config.page_size));
            frame.set_tier(BufferTier::Hot);
            hot_frames.push(frame.clone());
            all_frames.push(frame);
        }

        // Create frames for warm tier
        let mut warm_frames = Vec::new();
        for _ in 0..warm_frames_count {
            let frame = Arc::new(BufferFrame::new(config.page_size));
            frame.set_tier(BufferTier::Warm);
            warm_frames.push(frame.clone());
            all_frames.push(frame);
        }

        // Create frames for cold tier
        let mut cold_frames = Vec::new();
        for _ in 0..cold_frames_count {
            let frame = Arc::new(BufferFrame::new(config.page_size));
            cold_frames.push(frame.clone());
            all_frames.push(frame);
        }

        // Create keep pool frames
        let keep_frames_count = config.keep_pool_size / config.page_size;
        let mut keep_frames = Vec::new();
        for _ in 0..keep_frames_count {
            let frame = Arc::new(BufferFrame::new(config.page_size));
            keep_frames.push(frame.clone());
            all_frames.push(frame);
        }

        // Create recycle pool frames
        let recycle_frames_count = config.recycle_pool_size / config.page_size;
        let mut recycle_frames = Vec::new();
        for _ in 0..recycle_frames_count {
            let frame = Arc::new(BufferFrame::new(config.page_size));
            recycle_frames.push(frame.clone());
            all_frames.push(frame);
        }

        // Initialize free frames list
        let mut free_frames = VecDeque::new();
        for frame in all_frames.iter() {
            free_frames.push_back(frame.clone());
        }

        Self {
            config,
            hot_frames: Arc::new(Mutex::new(hot_frames)),
            warm_frames: Arc::new(Mutex::new(warm_frames)),
            cold_frames: Arc::new(Mutex::new(cold_frames)),
            keep_frames: Arc::new(Mutex::new(keep_frames)),
            recycle_frames: Arc::new(Mutex::new(recycle_frames)),
            tablespace_pools: Arc::new(Mutex::new(HashMap::new())),
            page_table: Arc::new(PRwLock::new(HashMap::new())),
            free_frames: Arc::new(Mutex::new(free_frames)),
            tier_manager_running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(BufferPoolStats::new()),
        }
    }

    // Allocate a frame from the appropriate pool
    pub fn allocate_frame(&self, pool_type: PoolType) -> Option<Arc<BufferFrame>> {
        let mut free_frames = self.free_frames.lock();
        if let Some(frame) = free_frames.pop_front() {
            self.stats.frames_allocated.fetch_add(1, Ordering::Relaxed);
            return Some(frame);
        }

        // No free frames, need to evict
        self.stats.allocation_failures.fetch_add(1, Ordering::Relaxed);
        None
    }

    // Promote page to higher tier based on access patterns
    pub fn promote_page(&self, page_id: PageId) -> bool {
        let page_table = self.page_table.read();
        if let Some(frame) = page_table.get(&page_id) {
            let current_tier = frame.tier();
            let access_count = frame.access_count();

            if access_count >= self.config.promotion_threshold {
                match current_tier {
                    BufferTier::Cold => {
                        frame.set_tier(BufferTier::Warm);
                        self.stats.promotions_cold_to_warm.fetch_add(1, Ordering::Relaxed);
                        return true;
                    }
                    BufferTier::Warm => {
                        frame.set_tier(BufferTier::Hot);
                        self.stats.promotions_warm_to_hot.fetch_add(1, Ordering::Relaxed);
                        return true;
                    }
                    BufferTier::Hot => {
                        // Already at highest tier
                    }
                }
            }
        }
        false
    }

    // Demote page to lower tier based on idle time
    pub fn demote_page(&self, page_id: PageId) -> bool {
        let page_table = self.page_table.read();
        if let Some(frame) = page_table.get(&page_id) {
            let current_tier = frame.tier();
            let idle_time = frame.idle_time();

            if idle_time.as_secs() >= self.config.demotion_threshold_secs {
                match current_tier {
                    BufferTier::Hot => {
                        frame.set_tier(BufferTier::Warm);
                        self.stats.demotions_hot_to_warm.fetch_add(1, Ordering::Relaxed);
                        return true;
                    }
                    BufferTier::Warm => {
                        frame.set_tier(BufferTier::Cold);
                        self.stats.demotions_warm_to_cold.fetch_add(1, Ordering::Relaxed);
                        return true;
                    }
                    BufferTier::Cold => {
                        // Already at lowest tier
                    }
                }
            }
        }
        false
    }

    // Pin a page in the buffer pool
    pub fn pin_page(&self, page_id: PageId, pool_type: PoolType) -> Option<Arc<BufferFrame>> {
        // Try to find in page table first
        {
            let page_table = self.page_table.read();
            if let Some(frame) = page_table.get(&page_id) {
                frame.pin();
                self.stats.page_hits.fetch_add(1, Ordering::Relaxed);
                self.promote_page(page_id);
                return Some(frame.clone());
            }
        }

        // Page miss - need to allocate a new frame
        self.stats.page_misses.fetch_add(1, Ordering::Relaxed);

        if let Some(frame) = self.allocate_frame(pool_type) {
            frame.pin();
            let mut page_table = self.page_table.write();
            page_table.insert(page_id, frame.clone());
            Some(frame)
        } else {
            None
        }
    }

    // Unpin a page in the buffer pool
    pub fn unpin_page(&self, page_id: PageId, dirty: bool) -> bool {
        let page_table = self.page_table.read();
        if let Some(frame) = page_table.get(&page_id) {
            if dirty {
                frame.mark_dirty(0); // LSN would come from transaction log
            }
            frame.unpin();
            return true;
        }
        false
    }

    // Get frame from keep pool (for pinned pages)
    pub fn allocate_keep_frame(&self) -> Option<Arc<BufferFrame>> {
        let keep_frames = self.keep_frames.lock();
        for frame in keep_frames.iter() {
            if frame.pin_count() == 0 && frame.page_id.is_none() {
                return Some(frame.clone());
            }
        }
        None
    }

    // Get frame from recycle pool (for sequential scans)
    pub fn allocate_recycle_frame(&self) -> Option<Arc<BufferFrame>> {
        let recycle_frames = self.recycle_frames.lock();
        for frame in recycle_frames.iter() {
            if frame.pin_count() == 0 && frame.page_id.is_none() {
                return Some(frame.clone());
            }
        }
        None
    }

    // Create or get per-tablespace buffer pool
    pub fn get_tablespace_pool(&self, tablespace_id: u32) -> Vec<Arc<BufferFrame>> {
        let mut pools = self.tablespace_pools.lock();
        if let Some(pool) = pools.get(&tablespace_id) {
            return pool.clone();
        }

        // Create new tablespace pool
        let pool_size = self.config.tablespace_pools
            .get(&tablespace_id)
            .copied()
            .unwrap_or(64 * 1024 * 1024); // Default 64MB

        let frames_count = pool_size / self.config.page_size;
        let mut frames = Vec::new();

        for _ in 0..frames_count {
            let frame = Arc::new(BufferFrame::new(self.config.page_size));
            frames.push(frame);
        }

        pools.insert(tablespace_id, frames.clone());
        frames
    }

    // NUMA-aware frame allocation
    pub fn allocate_numa_frame(&self, numa_node: u32) -> Option<Arc<BufferFrame>> {
        if !self.config.numa_aware {
            return self.allocate_frame(PoolType::Default);
        }

        // In a real implementation, this would use NUMA-specific allocation
        // For now, we'll use the default allocation
        self.allocate_frame(PoolType::Default)
    }

    // Start background tier management thread
    pub fn start_tier_manager(&self) {
        if self.tier_manager_running.swap(true, Ordering::Acquire) {
            return; // Already running
        }

        let page_table = self.page_table.clone();
        let running = self.tier_manager_running.clone();
        let _config = self.config.clone();
        let _pool_ref = Arc::new(self.stats.clone());

        std::thread::spawn(move || {
            while running.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_secs(60));

                // Scan all pages and adjust tiers
                let table = page_table.read();
                for (page_id, _frame) in table.iter() {
                    // Would call promote/demote logic here
                    // Simplified for this implementation
                }
            }
        });
    }

    // Stop background tier management
    pub fn stop_tier_manager(&self) {
        self.tier_manager_running.store(false, Ordering::Release);
    }

    // Get buffer pool statistics
    pub fn get_stats(&self) -> BufferPoolStatsSnapshot {
        self.stats.snapshot()
    }

    // Flush all dirty pages
    pub fn flush_all(&self) -> usize {
        let mut flushed = 0;
        let page_table = self.page_table.read();

        for (_page_id, frame) in page_table.iter() {
            if frame.is_dirty() {
                // Would actually write to disk here
                frame.dirty.store(false, Ordering::Release);
                flushed += 1;
            }
        }

        flushed
    }

    // Get total buffer pool capacity
    pub fn capacity(&self) -> usize {
        self.config.total_size
    }

    // Get number of frames in use
    pub fn frames_in_use(&self) -> usize {
        let page_table = self.page_table.read();
        page_table.len()
    }
}

// Buffer pool statistics
#[derive(Debug)]
pub struct BufferPoolStats {
    pub page_hits: AtomicU64,
    pub page_misses: AtomicU64,
    pub frames_allocated: AtomicU64,
    pub allocation_failures: AtomicU64,
    pub promotions_cold_to_warm: AtomicU64,
    pub promotions_warm_to_hot: AtomicU64,
    pub demotions_hot_to_warm: AtomicU64,
    pub demotions_warm_to_cold: AtomicU64,
}

impl BufferPoolStats {
    pub fn new() -> Self {
        Self {
            page_hits: AtomicU64::new(0),
            page_misses: AtomicU64::new(0),
            frames_allocated: AtomicU64::new(0),
            allocation_failures: AtomicU64::new(0),
            promotions_cold_to_warm: AtomicU64::new(0),
            promotions_warm_to_hot: AtomicU64::new(0),
            demotions_hot_to_warm: AtomicU64::new(0),
            demotions_warm_to_cold: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> BufferPoolStatsSnapshot {
        BufferPoolStatsSnapshot {
            page_hits: self.page_hits.load(Ordering::Relaxed),
            page_misses: self.page_misses.load(Ordering::Relaxed),
            frames_allocated: self.frames_allocated.load(Ordering::Relaxed),
            allocation_failures: self.allocation_failures.load(Ordering::Relaxed),
            promotions_cold_to_warm: self.promotions_cold_to_warm.load(Ordering::Relaxed),
            promotions_warm_to_hot: self.promotions_warm_to_hot.load(Ordering::Relaxed),
            demotions_hot_to_warm: self.demotions_hot_to_warm.load(Ordering::Relaxed),
            demotions_warm_to_cold: self.demotions_warm_to_cold.load(Ordering::Relaxed),
            hit_ratio: self.hit_ratio(),
        }
    }

    pub fn hit_ratio(&self) -> f64 {
        let hits = self.page_hits.load(Ordering::Relaxed) as f64;
        let misses = self.page_misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;
        if total == 0.0 {
            0.0
        } else {
            hits / total
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferPoolStatsSnapshot {
    pub page_hits: u64,
    pub page_misses: u64,
    pub frames_allocated: u64,
    pub allocation_failures: u64,
    pub promotions_cold_to_warm: u64,
    pub promotions_warm_to_hot: u64,
    pub demotions_hot_to_warm: u64,
    pub demotions_warm_to_cold: u64,
    pub hit_ratio: f64,
}
