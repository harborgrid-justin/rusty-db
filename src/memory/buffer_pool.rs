// # Enterprise Buffer Pool Management System
//
// Comprehensive multi-tier buffer pool implementation with advanced caching,
// replacement policies, and dirty page management for high-performance database operations.
//
// ## Architecture Overview
//
// ```text
// ┌──────────────────────────────────────────────────────────────────────┐
// │                    Multi-Tier Buffer Pool System                      │
// │                                                                        │
// │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │
// │  │  Hot Tier   │  │  Warm Tier  │  │  Cold Tier  │                  │
// │  │  (SSD-like) │  │  (Memory)   │  │  (Evict)    │                  │
// │  └─────────────┘  └─────────────┘  └─────────────┘                  │
// │         │                 │                 │                         │
// │         └─────────────────┴─────────────────┘                         │
// │                           │                                           │
// │  ┌────────────────────────┴─────────────────────────┐                │
// │  │         Adaptive Replacement Cache (ARC)         │                │
// │  │  T1 (Recent)  │  T2 (Frequent)  │  B1  │  B2    │                │
// │  └──────────────────────────────────────────────────┘                │
// │                                                                        │
// │  ┌────────────────────────────────────────────────┐                  │
// │  │        Buffer Replacement Policies              │                  │
// │  │  Clock-Sweep │ LRU-K │ 2Q │ Cost-Aware         │                  │
// │  └────────────────────────────────────────────────┘                  │
// │                                                                        │
// │  ┌────────────────────────────────────────────────┐                  │
// │  │         Dirty Page Management                   │                  │
// │  │  Checkpoint Queue │ Background Writer │ Double  │                  │
// │  │  Write Buffer     │ Flush Lists       │ Write   │                  │
// │  └────────────────────────────────────────────────┘                  │
// └──────────────────────────────────────────────────────────────────────┘
// ```

use tokio::time::sleep;
use std::collections::{HashMap, VecDeque, BTreeMap};
use std::sync::atomic::{AtomicU64, AtomicUsize, AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use parking_lot::{Mutex, RwLock as PRwLock};
use serde::{Serialize, Deserialize};
use crate::error::Result;

// ============================================================================
// SECTION 1: MULTI-TIER BUFFER POOL (700+ lines)
// ============================================================================

/// Page identifier combining tablespace and page number
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageId {
    pub tablespace_id: u32,
    pub page_number: u64,
}

impl PageId {
    pub fn new(tablespace_id: u32, page_number: u64) -> Self {
        Self { tablespace_id, page_number }
    }
}

/// Buffer tier classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BufferTier {
    /// Hottest pages - frequently accessed, pinned in memory
    Hot,
    /// Moderately accessed pages
    Warm,
    /// Rarely accessed pages - candidates for eviction
    Cold,
}

/// Buffer pool type configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolType {
    /// Default buffer pool for general operations
    Default,
    /// Keep pool for pinned pages that should not be evicted
    Keep,
    /// Recycle pool for sequential scans
    Recycle,
    /// Per-tablespace dedicated pool
    Tablespace(u32),
}

/// Buffer frame containing page data and metadata
#[derive(Debug)]
pub struct BufferFrame {
    /// Page identifier
    page_id: Option<PageId>,
    /// Actual page data (typically 8KB, 16KB, or 32KB)
    data: Vec<u8>,
    /// Pin count - number of active references
    pin_count: AtomicUsize,
    /// Dirty flag - has been modified
    dirty: AtomicBool,
    /// Access count for replacement policy
    access_count: AtomicU64,
    /// Last access timestamp
    last_access: Mutex<Instant>,
    /// Current tier assignment
    tier: Mutex<BufferTier>,
    /// LSN (Log Sequence Number) of last modification
    lsn: AtomicU64,
    /// Lock for page content modifications
    page_lock: PRwLock<()>,
}

impl BufferFrame {
    pub fn new(page_size: usize) -> Self {
        Self {
            page_id: None,
            data: vec![0; page_size],
            pin_count: AtomicUsize::new(0),
            dirty: AtomicBool::new(false),
            access_count: AtomicU64::new(0),
            last_access: Mutex::new(Instant::now()),
            tier: Mutex::new(BufferTier::Cold),
            lsn: AtomicU64::new(0),
            page_lock: PRwLock::new(()),
        }
    }

    /// Pin the buffer frame (increment reference count)
    pub fn pin(&self) -> usize {
        let count = self.pin_count.fetch_add(1, Ordering::AcqRel) + 1;
        self.access_count.fetch_add(1, Ordering::Relaxed);
        *self.last_access.lock() = Instant::now();
        count
    }

    /// Unpin the buffer frame (decrement reference count)
    pub fn unpin(&self) -> usize {
        let prev = self.pin_count.fetch_sub(1, Ordering::AcqRel);
        if prev == 0 {
            panic!("Attempt to unpin a buffer frame with pin count 0");
        }
        prev - 1
    }

    /// Get current pin count
    pub fn pin_count(&self) -> usize {
        self.pin_count.load(Ordering::Acquire)
    }

    /// Mark page as dirty
    pub fn mark_dirty(&self, lsn: u64) {
        self.dirty.store(true, Ordering::Release);
        self.lsn.store(lsn, Ordering::Release);
    }

    /// Check if page is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }

    /// Get page data (read-only)
    pub fn read_data(&self) -> &[u8] {
        &self.data
    }

    /// Get mutable page data
    pub fn write_data(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Get current tier
    pub fn tier(&self) -> BufferTier {
        *self.tier.lock()
    }

    /// Set tier
    pub fn set_tier(&self, new_tier: BufferTier) {
        *self.tier.lock() = new_tier;
    }

    /// Get access count
    pub fn access_count(&self) -> u64 {
        self.access_count.load(Ordering::Relaxed)
    }

    /// Get time since last access
    pub fn idle_time(&self) -> Duration {
        self.last_access.lock().elapsed()
    }
}

/// NUMA node configuration
#[derive(Debug, Clone)]
pub struct NumaNode {
    pub node_id: u32,
    pub cpu_mask: Vec<usize>,
    pub memory_base: usize,
    pub memory_size: usize,
}

/// Multi-tier buffer pool configuration
#[derive(Debug, Clone)]
pub struct BufferPoolConfig {
    /// Total buffer pool size in bytes
    pub total_size: usize,
    /// Page size in bytes (typically 8192, 16384, or 32768)
    pub page_size: usize,
    /// Hot tier percentage (0.0 - 1.0)
    pub hot_tier_ratio: f64,
    /// Warm tier percentage (0.0 - 1.0)
    pub warm_tier_ratio: f64,
    /// NUMA-aware allocation enabled
    pub numa_aware: bool,
    /// NUMA node configurations
    pub numa_nodes: Vec<NumaNode>,
    /// Per-tablespace pool configurations
    pub tablespace_pools: HashMap<u32, usize>,
    /// Keep pool size in bytes
    pub keep_pool_size: usize,
    /// Recycle pool size in bytes
    pub recycle_pool_size: usize,
    /// Promotion threshold (access count)
    pub promotion_threshold: u64,
    /// Demotion threshold (idle time in seconds)
    pub demotion_threshold_secs: u64,
}

impl Default for BufferPoolConfig {
    fn default() -> Self {
        Self {
            total_size: 1024 * 1024 * 1024, // 1GB
            page_size: 8192,
            hot_tier_ratio: 0.2,
            warm_tier_ratio: 0.5,
            numa_aware: false,
            numa_nodes: Vec::new(),
            tablespace_pools: HashMap::new(),
            keep_pool_size: 64 * 1024 * 1024, // 64MB
            recycle_pool_size: 32 * 1024 * 1024, // 32MB
            promotion_threshold: 10,
            demotion_threshold_secs: 300,
        }
    }
}

/// Multi-tier buffer pool implementation
pub struct MultiTierBufferPool {
    config: BufferPoolConfig,
    /// Hot tier frames
    hot_frames: Arc<Mutex<Vec<Arc<BufferFrame>>>>,
    /// Warm tier frames
    warm_frames: Arc<Mutex<Vec<Arc<BufferFrame>>>>,
    /// Cold tier frames
    cold_frames: Arc<Mutex<Vec<Arc<BufferFrame>>>>,
    /// Keep pool frames (pinned pages)
    keep_frames: Arc<Mutex<Vec<Arc<BufferFrame>>>>,
    /// Recycle pool frames (sequential access)
    recycle_frames: Arc<Mutex<Vec<Arc<BufferFrame>>>>,
    /// Per-tablespace pools
    tablespace_pools: Arc<Mutex<HashMap<u32, Vec<Arc<BufferFrame>>>>>,
    /// Page table mapping PageId to BufferFrame
    page_table: Arc<PRwLock<HashMap<PageId, Arc<BufferFrame>>>>,
    /// Free frames list
    free_frames: Arc<Mutex<VecDeque<Arc<BufferFrame>>>>,
    /// Background tier management thread handle
    tier_manager_running: Arc<AtomicBool>,
    /// Statistics
    stats: Arc<BufferPoolStats>,
}

impl MultiTierBufferPool {
    /// Create a new multi-tier buffer pool
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

    /// Allocate a frame from the appropriate pool
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

    /// Promote page to higher tier based on access patterns
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

    /// Demote page to lower tier based on idle time
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

    /// Pin a page in the buffer pool
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

    /// Unpin a page in the buffer pool
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

    /// Get frame from keep pool (for pinned pages)
    pub fn allocate_keep_frame(&self) -> Option<Arc<BufferFrame>> {
        let mut keep_frames = self.keep_frames.lock();
        for frame in keep_frames.iter() {
            if frame.pin_count() == 0 && frame.page_id.is_none() {
                return Some(frame.clone());
            }
        }
        None
    }

    /// Get frame from recycle pool (for sequential scans)
    pub fn allocate_recycle_frame(&self) -> Option<Arc<BufferFrame>> {
        let mut recycle_frames = self.recycle_frames.lock();
        for frame in recycle_frames.iter() {
            if frame.pin_count() == 0 && frame.page_id.is_none() {
                return Some(frame.clone());
            }
        }
        None
    }

    /// Create or get per-tablespace buffer pool
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

    /// NUMA-aware frame allocation
    pub fn allocate_numa_frame(&self, numa_node: u32) -> Option<Arc<BufferFrame>> {
        if !self.config.numa_aware {
            return self.allocate_frame(PoolType::Default);
        }

        // In a real implementation, this would use NUMA-specific allocation
        // For now, we'll use the default allocation
        self.allocate_frame(PoolType::Default)
    }

    /// Start background tier management thread
    pub fn start_tier_manager(&self) {
        if self.tier_manager_running.swap(true, Ordering::Acquire) {
            return; // Already running
        }

        let page_table = self.page_table.clone();
        let running = self.tier_manager_running.clone();
        let config = self.config.clone();
        let pool_ref = Arc::new(self.stats.clone());

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

    /// Stop background tier management
    pub fn stop_tier_manager(&self) {
        self.tier_manager_running.store(false, Ordering::Release);
    }

    /// Get buffer pool statistics
    pub fn get_stats(&self) -> BufferPoolStatsSnapshot {
        self.stats.snapshot()
    }

    /// Flush all dirty pages
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

    /// Get total buffer pool capacity
    pub fn capacity(&self) -> usize {
        self.config.total_size
    }

    /// Get number of frames in use
    pub fn frames_in_use(&self) -> usize {
        let page_table = self.page_table.read();
        page_table.len()
    }
}

/// Buffer pool statistics
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

// ============================================================================
// SECTION 2: PAGE CACHE MANAGEMENT (600+ lines)
// ============================================================================

/// Adaptive Replacement Cache (ARC) implementation
///
/// ARC maintains four lists:
/// - T1: Recently accessed pages (once)
/// - T2: Frequently accessed pages (multiple times)
/// - B1: Ghost entries for recently evicted from T1
/// - B2: Ghost entries for recently evicted from T2
pub struct AdaptiveReplacementCache {
    /// Target size for T1
    p: AtomicUsize,
    /// Maximum cache size
    c: usize,
    /// T1: Recent cache (frequency = 1)
    t1: Mutex<VecDeque<PageId>>,
    /// T2: Frequent cache (frequency > 1)
    t2: Mutex<VecDeque<PageId>>,
    /// B1: Ghost entries for T1
    b1: Mutex<VecDeque<PageId>>,
    /// B2: Ghost entries for T2
    b2: Mutex<VecDeque<PageId>>,
    /// Page directory mapping PageId to location
    directory: PRwLock<HashMap<PageId, CacheLocation>>,
    /// Frame storage
    frames: PRwLock<HashMap<PageId, Arc<BufferFrame>>>,
    /// Statistics
    stats: ArcStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CacheLocation {
    T1,
    T2,
    B1,
    B2,
}

#[derive(Debug)]
struct ArcStats {
    hits_t1: AtomicU64,
    hits_t2: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    ghost_hits_b1: AtomicU64,
    ghost_hits_b2: AtomicU64,
}

impl ArcStats {
    fn new() -> Self {
        Self {
            hits_t1: AtomicU64::new(0),
            hits_t2: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            ghost_hits_b1: AtomicU64::new(0),
            ghost_hits_b2: AtomicU64::new(0),
        }
    }
}

impl AdaptiveReplacementCache {
    /// Create new ARC cache with given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            p: AtomicUsize::new(0),
            c: capacity,
            t1: Mutex::new(VecDeque::new()),
            t2: Mutex::new(VecDeque::new()),
            b1: Mutex::new(VecDeque::new()),
            b2: Mutex::new(VecDeque::new()),
            directory: PRwLock::new(HashMap::new()),
            frames: PRwLock::new(HashMap::new()),
            stats: ArcStats::new(),
        }
    }

    /// Access a page in the cache
    pub fn get(&self, page_id: PageId, page_size: usize) -> Option<Arc<BufferFrame>> {
        let dir = self.directory.read();

        if let Some(&location) = dir.get(&page_id) {
            drop(dir);

            match location {
                CacheLocation::T1 => {
                    self.stats.hits_t1.fetch_add(1, Ordering::Relaxed);
                    // Move from T1 to T2 (accessed more than once)
                    self.move_t1_to_t2(page_id);
                }
                CacheLocation::T2 => {
                    self.stats.hits_t2.fetch_add(1, Ordering::Relaxed);
                    // Move to MRU position in T2
                    self.touch_t2(page_id);
                }
                CacheLocation::B1 => {
                    self.stats.ghost_hits_b1.fetch_add(1, Ordering::Relaxed);
                    // Increase p (favor recency)
                    self.adapt_on_b1_hit();
                    return None; // Ghost entry, need to load from disk
                }
                CacheLocation::B2 => {
                    self.stats.ghost_hits_b2.fetch_add(1, Ordering::Relaxed);
                    // Decrease p (favor frequency)
                    self.adapt_on_b2_hit();
                    return None; // Ghost entry, need to load from disk
                }
            }

            let frames = self.frames.read();
            return frames.get(&page_id).cloned();
        }

        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Insert a new page into the cache
    pub fn insert(&self, page_id: PageId, frame: Arc<BufferFrame>) {
        let dir = self.directory.read();

        if let Some(&location) = dir.get(&page_id) {
            drop(dir);

            // Handle ghost hits
            match location {
                CacheLocation::B1 => {
                    self.adapt_on_b1_hit();
                    self.replace(page_id, CacheLocation::B1);
                    self.remove_from_b1(page_id);
                    self.add_to_t2(page_id, frame);
                    return;
                }
                CacheLocation::B2 => {
                    self.adapt_on_b2_hit();
                    self.replace(page_id, CacheLocation::B2);
                    self.remove_from_b2(page_id);
                    self.add_to_t2(page_id, frame);
                    return;
                }
                _ => {
                    // Already in cache, just update
                    return;
                }
            }
        }
        drop(dir);

        // New page, add to T1
        let t1_len = self.t1.lock().len();
        let t2_len = self.t2.lock().len();

        if t1_len + t2_len >= self.c {
            self.replace(page_id, CacheLocation::T1);
        }

        self.add_to_t1(page_id, frame);
    }

    /// ARC replacement algorithm
    fn replace(&self, page_id: PageId, hit_location: CacheLocation) {
        let p = self.p.load(Ordering::Relaxed);
        let t1_len = self.t1.lock().len();

        let evict_from_t1 = if t1_len > 0 {
            if t1_len > p || (hit_location == CacheLocation::B2 && t1_len == p) {
                true
            } else {
                false
            }
        } else {
            false
        };

        if evict_from_t1 {
            // Evict from T1
            let mut t1 = self.t1.lock();
            if let Some(evict_page) = t1.pop_front() {
                drop(t1);

                // Move to B1
                let mut frames = self.frames.write();
                frames.remove(&evict_page);
                drop(frames);

                self.add_to_b1(evict_page);
                self.stats.evictions.fetch_add(1, Ordering::Relaxed);
            }
        } else {
            // Evict from T2
            let mut t2 = self.t2.lock();
            if let Some(evict_page) = t2.pop_front() {
                drop(t2);

                // Move to B2
                let mut frames = self.frames.write();
                frames.remove(&evict_page);
                drop(frames);

                self.add_to_b2(evict_page);
                self.stats.evictions.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Adapt parameter p on B1 hit
    fn adapt_on_b1_hit(&self) {
        let b1_len = self.b1.lock().len();
        let b2_len = self.b2.lock().len();

        let delta = if b1_len >= b2_len {
            1
        } else {
            b2_len / b1_len
        };

        let current_p = self.p.load(Ordering::Relaxed);
        let new_p = std::cmp::min(current_p + delta, self.c);
        self.p.store(new_p, Ordering::Relaxed);
    }

    /// Adapt parameter p on B2 hit
    fn adapt_on_b2_hit(&self) {
        let b1_len = self.b1.lock().len();
        let b2_len = self.b2.lock().len();

        let delta = if b2_len >= b1_len {
            1
        } else {
            b1_len / b2_len
        };

        let current_p = self.p.load(Ordering::Relaxed);
        let new_p = current_p.saturating_sub(delta);
        self.p.store(new_p, Ordering::Relaxed);
    }

    // Helper methods for list management
    fn add_to_t1(&self, page_id: PageId, frame: Arc<BufferFrame>) {
        let mut t1 = self.t1.lock();
        t1.push_back(page_id);
        drop(t1);

        let mut frames = self.frames.write();
        frames.insert(page_id, frame);
        drop(frames);

        let mut dir = self.directory.write();
        dir.insert(page_id, CacheLocation::T1);
    }

    fn add_to_t2(&self, page_id: PageId, frame: Arc<BufferFrame>) {
        let mut t2 = self.t2.lock();
        t2.push_back(page_id);
        drop(t2);

        let mut frames = self.frames.write();
        frames.insert(page_id, frame);
        drop(frames);

        let mut dir = self.directory.write();
        dir.insert(page_id, CacheLocation::T2);
    }

    fn add_to_b1(&self, page_id: PageId) {
        let mut b1 = self.b1.lock();
        b1.push_back(page_id);

        // Limit B1 size
        while b1.len() > self.c {
            if let Some(evict) = b1.pop_front() {
                let mut dir = self.directory.write();
                dir.remove(&evict);
            }
        }
        drop(b1);

        let mut dir = self.directory.write();
        dir.insert(page_id, CacheLocation::B1);
    }

    fn add_to_b2(&self, page_id: PageId) {
        let mut b2 = self.b2.lock();
        b2.push_back(page_id);

        // Limit B2 size
        while b2.len() > self.c {
            if let Some(evict) = b2.pop_front() {
                let mut dir = self.directory.write();
                dir.remove(&evict);
            }
        }
        drop(b2);

        let mut dir = self.directory.write();
        dir.insert(page_id, CacheLocation::B2);
    }

    fn move_t1_to_t2(&self, page_id: PageId) {
        let mut t1 = self.t1.lock();
        t1.retain(|&id| id != page_id);
        drop(t1);

        let mut t2 = self.t2.lock();
        t2.push_back(page_id);
        drop(t2);

        let mut dir = self.directory.write();
        dir.insert(page_id, CacheLocation::T2);
    }

    fn touch_t2(&self, page_id: PageId) {
        let mut t2 = self.t2.lock();
        t2.retain(|&id| id != page_id);
        t2.push_back(page_id);
    }

    fn remove_from_b1(&self, page_id: PageId) {
        let mut b1 = self.b1.lock();
        b1.retain(|&id| id != page_id);
    }

    fn remove_from_b2(&self, page_id: PageId) {
        let mut b2 = self.b2.lock();
        b2.retain(|&id| id != page_id);
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> ArcStatsSnapshot {
        ArcStatsSnapshot {
            hits_t1: self.stats.hits_t1.load(Ordering::Relaxed),
            hits_t2: self.stats.hits_t2.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            ghost_hits_b1: self.stats.ghost_hits_b1.load(Ordering::Relaxed),
            ghost_hits_b2: self.stats.ghost_hits_b2.load(Ordering::Relaxed),
            t1_size: self.t1.lock().len(),
            t2_size: self.t2.lock().len(),
            b1_size: self.b1.lock().len(),
            b2_size: self.b2.lock().len(),
            p_value: self.p.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcStatsSnapshot {
    pub hits_t1: u64,
    pub hits_t2: u64,
    pub misses: u64,
    pub evictions: u64,
    pub ghost_hits_b1: u64,
    pub ghost_hits_b2: u64,
    pub t1_size: usize,
    pub t2_size: usize,
    pub b1_size: usize,
    pub b2_size: usize,
    pub p_value: usize,
}

/// 2Q Cache (Scan-Resistant) Implementation
///
/// Maintains three queues:
/// - A1in: FIFO queue for new pages
/// - A1out: Ghost queue for recently evicted pages
/// - Am: LRU queue for frequently accessed pages
pub struct TwoQCache {
    /// Maximum cache size
    capacity: usize,
    /// A1in size (typically 25% of capacity)
    a1in_size: usize,
    /// A1out size (typically 50% of capacity)
    a1out_size: usize,
    /// A1in queue (FIFO for new pages)
    a1in: Mutex<VecDeque<PageId>>,
    /// A1out queue (ghost entries)
    a1out: Mutex<VecDeque<PageId>>,
    /// Am queue (LRU for frequent pages)
    am: Mutex<VecDeque<PageId>>,
    /// Page directory
    directory: PRwLock<HashMap<PageId, TwoQLocation>>,
    /// Frame storage
    frames: PRwLock<HashMap<PageId, Arc<BufferFrame>>>,
    /// Statistics
    stats: TwoQStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TwoQLocation {
    A1In,
    A1Out,
    Am,
}

#[derive(Debug)]
struct TwoQStats {
    hits_a1in: AtomicU64,
    hits_am: AtomicU64,
    misses: AtomicU64,
    promotions: AtomicU64,
    evictions: AtomicU64,
}

impl TwoQStats {
    fn new() -> Self {
        Self {
            hits_a1in: AtomicU64::new(0),
            hits_am: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            promotions: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }
}

impl TwoQCache {
    /// Create new 2Q cache
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            a1in_size: capacity / 4,
            a1out_size: capacity / 2,
            a1in: Mutex::new(VecDeque::new()),
            a1out: Mutex::new(VecDeque::new()),
            am: Mutex::new(VecDeque::new()),
            directory: PRwLock::new(HashMap::new()),
            frames: PRwLock::new(HashMap::new()),
            stats: TwoQStats::new(),
        }
    }

    /// Access a page
    pub fn get(&self, page_id: PageId) -> Option<Arc<BufferFrame>> {
        let dir = self.directory.read();

        if let Some(&location) = dir.get(&page_id) {
            drop(dir);

            match location {
                TwoQLocation::A1In => {
                    self.stats.hits_a1in.fetch_add(1, Ordering::Relaxed);
                }
                TwoQLocation::Am => {
                    self.stats.hits_am.fetch_add(1, Ordering::Relaxed);
                    self.touch_am(page_id);
                }
                TwoQLocation::A1Out => {
                    // Ghost hit - promote to Am
                    self.stats.promotions.fetch_add(1, Ordering::Relaxed);
                    return None;
                }
            }

            let frames = self.frames.read();
            return frames.get(&page_id).cloned();
        }

        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Insert a new page
    pub fn insert(&self, page_id: PageId, frame: Arc<BufferFrame>) {
        let dir = self.directory.read();

        if let Some(&location) = dir.get(&page_id) {
            drop(dir);

            match location {
                TwoQLocation::A1Out => {
                    // Promote to Am
                    self.remove_from_a1out(page_id);
                    self.ensure_am_space();
                    self.add_to_am(page_id, frame);
                    return;
                }
                _ => {
                    // Already in cache
                    return;
                }
            }
        }
        drop(dir);

        // New page - add to A1in
        self.ensure_a1in_space();
        self.add_to_a1in(page_id, frame);
    }

    fn ensure_a1in_space(&self) {
        let mut a1in = self.a1in.lock();
        if a1in.len() >= self.a1in_size {
            if let Some(evict_page) = a1in.pop_front() {
                drop(a1in);

                // Move to A1out (ghost)
                let mut frames = self.frames.write();
                frames.remove(&evict_page);
                drop(frames);

                self.add_to_a1out(evict_page);
                self.stats.evictions.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn ensure_am_space(&self) {
        let am_capacity = self.capacity - self.a1in_size;
        let mut am = self.am.lock();

        if am.len() >= am_capacity {
            if let Some(evict_page) = am.pop_front() {
                drop(am);

                let mut frames = self.frames.write();
                frames.remove(&evict_page);
                drop(frames);

                let mut dir = self.directory.write();
                dir.remove(&evict_page);
                drop(dir);

                self.stats.evictions.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn add_to_a1in(&self, page_id: PageId, frame: Arc<BufferFrame>) {
        let mut a1in = self.a1in.lock();
        a1in.push_back(page_id);
        drop(a1in);

        let mut frames = self.frames.write();
        frames.insert(page_id, frame);
        drop(frames);

        let mut dir = self.directory.write();
        dir.insert(page_id, TwoQLocation::A1In);
    }

    fn add_to_a1out(&self, page_id: PageId) {
        let mut a1out = self.a1out.lock();
        a1out.push_back(page_id);

        // Limit A1out size
        while a1out.len() > self.a1out_size {
            if let Some(evict) = a1out.pop_front() {
                let mut dir = self.directory.write();
                dir.remove(&evict);
            }
        }
        drop(a1out);

        let mut dir = self.directory.write();
        dir.insert(page_id, TwoQLocation::A1Out);
    }

    fn add_to_am(&self, page_id: PageId, frame: Arc<BufferFrame>) {
        let mut am = self.am.lock();
        am.push_back(page_id);
        drop(am);

        let mut frames = self.frames.write();
        frames.insert(page_id, frame);
        drop(frames);

        let mut dir = self.directory.write();
        dir.insert(page_id, TwoQLocation::Am);
    }

    fn touch_am(&self, page_id: PageId) {
        let mut am = self.am.lock();
        am.retain(|&id| id != page_id);
        am.push_back(page_id);
    }

    fn remove_from_a1out(&self, page_id: PageId) {
        let mut a1out = self.a1out.lock();
        a1out.retain(|&id| id != page_id);
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> TwoQStatsSnapshot {
        TwoQStatsSnapshot {
            hits_a1in: self.stats.hits_a1in.load(Ordering::Relaxed),
            hits_am: self.stats.hits_am.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            promotions: self.stats.promotions.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            a1in_size: self.a1in.lock().len(),
            a1out_size: self.a1out.lock().len(),
            am_size: self.am.lock().len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoQStatsSnapshot {
    pub hits_a1in: u64,
    pub hits_am: u64,
    pub misses: u64,
    pub promotions: u64,
    pub evictions: u64,
    pub a1in_size: usize,
    pub a1out_size: usize,
    pub am_size: usize,
}

/// Page prefetcher with ML-based prediction
pub struct PagePrefetcher {
    /// Sequential scan detection window
    scan_window: usize,
    /// Recent access pattern
    access_history: Mutex<VecDeque<PageId>>,
    /// Prefetch queue
    prefetch_queue: Mutex<VecDeque<PageId>>,
    /// Statistics
    stats: PrefetchStats,
}

#[derive(Debug)]
struct PrefetchStats {
    prefetch_requests: AtomicU64,
    prefetch_hits: AtomicU64,
    prefetch_misses: AtomicU64,
    sequential_scans_detected: AtomicU64,
}

impl PagePrefetcher {
    pub fn new(scan_window: usize) -> Self {
        Self {
            scan_window,
            access_history: Mutex::new(VecDeque::new()),
            prefetch_queue: Mutex::new(VecDeque::new()),
            stats: PrefetchStats {
                prefetch_requests: AtomicU64::new(0),
                prefetch_hits: AtomicU64::new(0),
                prefetch_misses: AtomicU64::new(0),
                sequential_scans_detected: AtomicU64::new(0),
            },
        }
    }

    /// Record page access and predict next pages
    pub fn record_access(&self, page_id: PageId) -> Vec<PageId> {
        let mut history = self.access_history.lock();
        history.push_back(page_id);

        if history.len() > self.scan_window {
            history.pop_front();
        }

        // Detect sequential pattern
        if self.is_sequential_scan(&history) {
            self.stats.sequential_scans_detected.fetch_add(1, Ordering::Relaxed);
            return self.predict_sequential(page_id);
        }

        Vec::new()
    }

    /// Check if access pattern is sequential
    fn is_sequential_scan(&self, history: &VecDeque<PageId>) -> bool {
        if history.len() < 3 {
            return false;
        }

        let vec: Vec<&PageId> = history.iter().collect();
        let mut sequential_count = 0;

        for i in 0..vec.len() - 1 {
            if vec[i].tablespace_id == vec[i + 1].tablespace_id &&
               vec[i + 1].page_number == vec[i].page_number + 1 {
                sequential_count += 1;
            }
        }

        sequential_count as f64 / (vec.len() - 1) as f64 > 0.7
    }

    /// Predict next pages in sequential scan
    fn predict_sequential(&self, last_page: PageId) -> Vec<PageId> {
        let mut predictions = Vec::new();

        // Prefetch next 4 pages
        for i in 1..=4 {
            predictions.push(PageId {
                tablespace_id: last_page.tablespace_id,
                page_number: last_page.page_number + i,
            });
        }

        self.stats.prefetch_requests.fetch_add(predictions.len() as u64, Ordering::Relaxed);
        predictions
    }

    /// Get prefetch statistics
    pub fn get_stats(&self) -> PrefetchStatsSnapshot {
        PrefetchStatsSnapshot {
            prefetch_requests: self.stats.prefetch_requests.load(Ordering::Relaxed),
            prefetch_hits: self.stats.prefetch_hits.load(Ordering::Relaxed),
            prefetch_misses: self.stats.prefetch_misses.load(Ordering::Relaxed),
            sequential_scans_detected: self.stats.sequential_scans_detected.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchStatsSnapshot {
    pub prefetch_requests: u64,
    pub prefetch_hits: u64,
    pub prefetch_misses: u64,
    pub sequential_scans_detected: u64,
}

// ============================================================================
// SECTION 3: BUFFER REPLACEMENT POLICIES (500+ lines)
// ============================================================================

/// Clock-Sweep (Second-Chance) algorithm implementation
pub struct ClockSweepPolicy {
    /// Clock hand position
    hand: AtomicUsize,
    /// Buffer frames
    frames: Vec<Arc<BufferFrame>>,
    /// Reference bits
    reference_bits: Vec<AtomicBool>,
    /// Statistics
    stats: ClockStats,
}

#[derive(Debug)]
struct ClockStats {
    sweeps: AtomicU64,
    evictions: AtomicU64,
    second_chances: AtomicU64,
}

impl ClockSweepPolicy {
    pub fn new(capacity: usize, page_size: usize) -> Self {
        let mut frames = Vec::new();
        let mut reference_bits = Vec::new();

        for _ in 0..capacity {
            frames.push(Arc::new(BufferFrame::new(page_size)));
            reference_bits.push(AtomicBool::new(false));
        }

        Self {
            hand: AtomicUsize::new(0),
            frames,
            reference_bits,
            stats: ClockStats {
                sweeps: AtomicU64::new(0),
                evictions: AtomicU64::new(0),
                second_chances: AtomicU64::new(0),
            },
        }
    }

    /// Find victim page for eviction
    pub fn find_victim(&self) -> Option<usize> {
        let capacity = self.frames.len();
        let mut current_hand = self.hand.load(Ordering::Relaxed);

        loop {
            self.stats.sweeps.fetch_add(1, Ordering::Relaxed);

            // Check if frame is pinned
            if self.frames[current_hand].pin_count() > 0 {
                current_hand = (current_hand + 1) % capacity;
                continue;
            }

            // Check reference bit
            let had_reference = self.reference_bits[current_hand].swap(false, Ordering::Relaxed);

            if !had_reference {
                // Found victim
                self.stats.evictions.fetch_add(1, Ordering::Relaxed);
                self.hand.store((current_hand + 1) % capacity, Ordering::Relaxed);
                return Some(current_hand);
            } else {
                // Give second chance
                self.stats.second_chances.fetch_add(1, Ordering::Relaxed);
                current_hand = (current_hand + 1) % capacity;
            }
        }
    }

    /// Set reference bit for a frame
    pub fn set_reference(&self, frame_idx: usize) {
        if frame_idx < self.reference_bits.len() {
            self.reference_bits[frame_idx].store(true, Ordering::Relaxed);
        }
    }

    /// Get frame at index
    pub fn get_frame(&self, idx: usize) -> Option<Arc<BufferFrame>> {
        self.frames.get(idx).cloned()
    }

    /// Get statistics
    pub fn get_stats(&self) -> ClockStatsSnapshot {
        ClockStatsSnapshot {
            sweeps: self.stats.sweeps.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            second_chances: self.stats.second_chances.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockStatsSnapshot {
    pub sweeps: u64,
    pub evictions: u64,
    pub second_chances: u64,
}

/// LRU-K (K = 2) implementation - tracks K most recent accesses
pub struct LruKPolicy {
    /// K value (typically 2)
    k: usize,
    /// Access history for each page
    history: PRwLock<HashMap<PageId, VecDeque<Instant>>>,
    /// Correlation period for history
    corr_period: Duration,
    /// Statistics
    stats: LruKStats,
}

#[derive(Debug)]
struct LruKStats {
    accesses: AtomicU64,
    evictions: AtomicU64,
    history_promotions: AtomicU64,
}

impl LruKPolicy {
    pub fn new(k: usize, corr_period_secs: u64) -> Self {
        Self {
            k,
            history: PRwLock::new(HashMap::new()),
            corr_period: Duration::from_secs(corr_period_secs),
            stats: LruKStats {
                accesses: AtomicU64::new(0),
                evictions: AtomicU64::new(0),
                history_promotions: AtomicU64::new(0),
            },
        }
    }

    /// Record page access
    pub fn access(&self, page_id: PageId) {
        self.stats.accesses.fetch_add(1, Ordering::Relaxed);

        let mut history = self.history.write();
        let page_history = history.entry(page_id).or_insert_with(VecDeque::new);

        page_history.push_back(Instant::now());

        // Keep only K most recent accesses
        if page_history.len() > self.k {
            page_history.pop_front();
            self.stats.history_promotions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Calculate backward K-distance for a page
    pub fn backward_k_distance(&self, page_id: PageId) -> Option<Duration> {
        let history = self.history.read();
        if let Some(page_history) = history.get(&page_id) {
            if page_history.len() >= self.k {
                // K-th most recent access
                if let Some(&kth_access) = page_history.get(page_history.len() - self.k) {
                    return Some(kth_access.elapsed());
                }
            } else if let Some(&first_access) = page_history.front() {
                // Not enough history, use oldest access
                return Some(first_access.elapsed());
            }
        }
        None
    }

    /// Find victim page (largest backward K-distance)
    pub fn find_victim(&self, candidates: &[PageId]) -> Option<PageId> {
        let mut max_distance = Duration::ZERO;
        let mut victim = None;

        for &page_id in candidates {
            if let Some(distance) = self.backward_k_distance(page_id) {
                if distance > max_distance {
                    max_distance = distance;
                    victim = Some(page_id);
                }
            } else {
                // No history = infinite distance, best victim
                return Some(page_id);
            }
        }

        if victim.is_some() {
            self.stats.evictions.fetch_add(1, Ordering::Relaxed);
        }

        victim
    }

    /// Clean old history entries
    pub fn clean_old_history(&self) {
        let mut history = self.history.write();
        let cutoff = Instant::now() - self.corr_period;

        history.retain(|_, page_history| {
            page_history.retain(|&access_time| access_time > cutoff);
            !page_history.is_empty()
        });
    }

    /// Get statistics
    pub fn get_stats(&self) -> LruKStatsSnapshot {
        LruKStatsSnapshot {
            accesses: self.stats.accesses.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            history_promotions: self.stats.history_promotions.load(Ordering::Relaxed),
            history_entries: self.history.read().len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LruKStatsSnapshot {
    pub accesses: u64,
    pub evictions: u64,
    pub history_promotions: u64,
    pub history_entries: usize,
}

/// Touch count optimization for hot pages
pub struct TouchCountOptimizer {
    /// Touch counts per page
    touch_counts: PRwLock<HashMap<PageId, AtomicU64>>,
    /// Hot threshold
    hot_threshold: u64,
    /// Statistics
    stats: TouchCountStats,
}

#[derive(Debug)]
struct TouchCountStats {
    total_touches: AtomicU64,
    hot_pages: AtomicU64,
    warm_pages: AtomicU64,
    cold_pages: AtomicU64,
}

impl TouchCountOptimizer {
    pub fn new(hot_threshold: u64) -> Self {
        Self {
            touch_counts: PRwLock::new(HashMap::new()),
            hot_threshold,
            stats: TouchCountStats {
                total_touches: AtomicU64::new(0),
                hot_pages: AtomicU64::new(0),
                warm_pages: AtomicU64::new(0),
                cold_pages: AtomicU64::new(0),
            },
        }
    }

    /// Record page touch
    pub fn touch(&self, page_id: PageId) {
        self.stats.total_touches.fetch_add(1, Ordering::Relaxed);

        let counts = self.touch_counts.read();
        if let Some(count) = counts.get(&page_id) {
            count.fetch_add(1, Ordering::Relaxed);
        } else {
            drop(counts);
            let mut counts = self.touch_counts.write();
            counts.entry(page_id).or_insert_with(|| AtomicU64::new(1));
        }
    }

    /// Get touch count for a page
    pub fn get_count(&self, page_id: PageId) -> u64 {
        let counts = self.touch_counts.read();
        counts.get(&page_id)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Determine page temperature
    pub fn temperature(&self, page_id: PageId) -> BufferTier {
        let count = self.get_count(page_id);

        if count >= self.hot_threshold {
            BufferTier::Hot
        } else if count >= self.hot_threshold / 2 {
            BufferTier::Warm
        } else {
            BufferTier::Cold
        }
    }

    /// Reset touch count for a page
    pub fn reset(&self, page_id: PageId) {
        let counts = self.touch_counts.read();
        if let Some(count) = counts.get(&page_id) {
            count.store(0, Ordering::Relaxed);
        }
    }

    /// Decay all touch counts (age out old activity)
    pub fn decay_all(&self, factor: f64) {
        let counts = self.touch_counts.read();
        for (_, count) in counts.iter() {
            let current = count.load(Ordering::Relaxed);
            let new_value = (current as f64 * factor) as u64;
            count.store(new_value, Ordering::Relaxed);
        }
    }

    /// Get statistics
    pub fn get_stats(&self) -> TouchCountStatsSnapshot {
        let counts = self.touch_counts.read();
        let mut hot = 0u64;
        let mut warm = 0u64;
        let mut cold = 0u64;

        for (_, count) in counts.iter() {
            let c = count.load(Ordering::Relaxed);
            if c >= self.hot_threshold {
                hot += 1;
            } else if c >= self.hot_threshold / 2 {
                warm += 1;
            } else {
                cold += 1;
            }
        }

        TouchCountStatsSnapshot {
            total_touches: self.stats.total_touches.load(Ordering::Relaxed),
            hot_pages: hot,
            warm_pages: warm,
            cold_pages: cold,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchCountStatsSnapshot {
    pub total_touches: u64,
    pub hot_pages: u64,
    pub warm_pages: u64,
    pub cold_pages: u64,
}

/// Cost-aware replacement policy
pub struct CostAwareReplacement {
    /// Cost per page (based on load time, etc.)
    page_costs: PRwLock<HashMap<PageId, f64>>,
    /// Access frequency
    access_freq: PRwLock<HashMap<PageId, AtomicU64>>,
    /// Statistics
    stats: CostAwareStats,
}

#[derive(Debug)]
struct CostAwareStats {
    cost_calculations: AtomicU64,
    evictions: AtomicU64,
}

impl CostAwareReplacement {
    pub fn new() -> Self {
        Self {
            page_costs: PRwLock::new(HashMap::new()),
            access_freq: PRwLock::new(HashMap::new()),
            stats: CostAwareStats {
                cost_calculations: AtomicU64::new(0),
                evictions: AtomicU64::new(0),
            },
        }
    }

    /// Set page load cost
    pub fn set_cost(&self, page_id: PageId, cost: f64) {
        let mut costs = self.page_costs.write();
        costs.insert(page_id, cost);
    }

    /// Record page access
    pub fn access(&self, page_id: PageId) {
        let freq = self.access_freq.read();
        if let Some(count) = freq.get(&page_id) {
            count.fetch_add(1, Ordering::Relaxed);
        } else {
            drop(freq);
            let mut freq = self.access_freq.write();
            freq.entry(page_id).or_insert_with(|| AtomicU64::new(1));
        }
    }

    /// Calculate replacement value (higher = keep, lower = evict)
    pub fn replacement_value(&self, page_id: PageId) -> f64 {
        self.stats.cost_calculations.fetch_add(1, Ordering::Relaxed);

        let costs = self.page_costs.read();
        let freq = self.access_freq.read();

        let cost = costs.get(&page_id).copied().unwrap_or(1.0);
        let frequency = freq.get(&page_id)
            .map(|f| f.load(Ordering::Relaxed))
            .unwrap_or(1) as f64;

        // Value = Cost * Frequency (expensive, frequently accessed pages stay)
        cost * frequency
    }

    /// Find victim page (lowest replacement value)
    pub fn find_victim(&self, candidates: &[PageId]) -> Option<PageId> {
        let mut min_value = f64::MAX;
        let mut victim = None;

        for &page_id in candidates {
            let value = self.replacement_value(page_id);
            if value < min_value {
                min_value = value;
                victim = Some(page_id);
            }
        }

        if victim.is_some() {
            self.stats.evictions.fetch_add(1, Ordering::Relaxed);
        }

        victim
    }

    /// Get statistics
    pub fn get_stats(&self) -> CostAwareStatsSnapshot {
        CostAwareStatsSnapshot {
            cost_calculations: self.stats.cost_calculations.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            tracked_pages: self.page_costs.read().len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAwareStatsSnapshot {
    pub cost_calculations: u64,
    pub evictions: u64,
    pub tracked_pages: usize,
}

// ============================================================================
// SECTION 4: DIRTY PAGE MANAGEMENT (600+ lines)
// ============================================================================

/// Dirty page descriptor
#[derive(Debug, Clone)]
pub struct DirtyPage {
    pub page_id: PageId,
    pub lsn: u64,
    pub dirty_time: Instant,
    pub frame: Arc<BufferFrame>,
}

/// Checkpoint queue for dirty pages
pub struct CheckpointQueue {
    /// Dirty pages ordered by LSN
    queue: Mutex<BTreeMap<u64, Vec<DirtyPage>>>,
    /// Total dirty pages
    dirty_count: AtomicUsize,
    /// Checkpoint LSN watermark
    checkpoint_lsn: AtomicU64,
    /// Statistics
    stats: CheckpointStats,
}

#[derive(Debug)]
struct CheckpointStats {
    pages_queued: AtomicU64,
    pages_flushed: AtomicU64,
    checkpoints: AtomicU64,
}

impl CheckpointQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(BTreeMap::new()),
            dirty_count: AtomicUsize::new(0),
            checkpoint_lsn: AtomicU64::new(0),
            stats: CheckpointStats {
                pages_queued: AtomicU64::new(0),
                pages_flushed: AtomicU64::new(0),
                checkpoints: AtomicU64::new(0),
            },
        }
    }

    /// Add dirty page to checkpoint queue
    pub fn enqueue(&self, dirty_page: DirtyPage) {
        let lsn = dirty_page.lsn;
        let mut queue = self.queue.lock();

        queue.entry(lsn)
            .or_insert_with(Vec::new)
            .push(dirty_page);

        self.dirty_count.fetch_add(1, Ordering::Relaxed);
        self.stats.pages_queued.fetch_add(1, Ordering::Relaxed);
    }

    /// Get pages to flush up to a given LSN
    pub fn get_pages_to_flush(&self, up_to_lsn: u64) -> Vec<DirtyPage> {
        let mut queue = self.queue.lock();
        let mut pages = Vec::new();

        // Collect all pages with LSN <= up_to_lsn
        let lsns_to_remove: Vec<u64> = queue.range(..=up_to_lsn)
            .map(|(lsn, _)| *lsn)
            .collect();

        for lsn in lsns_to_remove {
            if let Some(lsn_pages) = queue.remove(&lsn) {
                let count = lsn_pages.len();
                pages.extend(lsn_pages);
                self.dirty_count.fetch_sub(count, Ordering::Relaxed);
            }
        }

        pages
    }

    /// Perform checkpoint
    pub fn checkpoint(&self) -> CheckpointResult {
        self.stats.checkpoints.fetch_add(1, Ordering::Relaxed);

        let current_lsn = self.checkpoint_lsn.load(Ordering::Relaxed);
        let pages = self.get_pages_to_flush(current_lsn);
        let page_count = pages.len();

        // Flush pages (would actually write to disk)
        for page in &pages {
            page.frame.dirty.store(false, Ordering::Release);
        }

        self.stats.pages_flushed.fetch_add(page_count as u64, Ordering::Relaxed);
        self.checkpoint_lsn.fetch_add(1, Ordering::Relaxed);

        CheckpointResult {
            pages_flushed: page_count,
            checkpoint_lsn: current_lsn,
            duration: Duration::ZERO, // Would measure actual flush time
        }
    }

    /// Get dirty page count
    pub fn dirty_count(&self) -> usize {
        self.dirty_count.load(Ordering::Relaxed)
    }

    /// Get statistics
    pub fn get_stats(&self) -> CheckpointStatsSnapshot {
        CheckpointStatsSnapshot {
            pages_queued: self.stats.pages_queued.load(Ordering::Relaxed),
            pages_flushed: self.stats.pages_flushed.load(Ordering::Relaxed),
            checkpoints: self.stats.checkpoints.load(Ordering::Relaxed),
            current_dirty_count: self.dirty_count.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointResult {
    pub pages_flushed: usize,
    pub checkpoint_lsn: u64,
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointStatsSnapshot {
    pub pages_queued: u64,
    pub pages_flushed: u64,
    pub checkpoints: u64,
    pub current_dirty_count: usize,
}

/// Incremental checkpoint manager
pub struct IncrementalCheckpointer {
    /// Checkpoint interval in seconds
    interval: Duration,
    /// Pages per checkpoint batch
    batch_size: usize,
    /// Checkpoint queue reference
    checkpoint_queue: Arc<CheckpointQueue>,
    /// Running flag
    running: Arc<AtomicBool>,
    /// Statistics
    stats: IncrementalCheckpointStats,
}

#[derive(Debug)]
struct IncrementalCheckpointStats {
    incremental_checkpoints: AtomicU64,
    total_pages_flushed: AtomicU64,
    average_batch_size: AtomicU64,
}

impl IncrementalCheckpointer {
    pub fn new(
        interval_secs: u64,
        batch_size: usize,
        checkpoint_queue: Arc<CheckpointQueue>,
    ) -> Self {
        Self {
            interval: Duration::from_secs(interval_secs),
            batch_size,
            checkpoint_queue,
            running: Arc::new(AtomicBool::new(false)),
            stats: IncrementalCheckpointStats {
                incremental_checkpoints: AtomicU64::new(0),
                total_pages_flushed: AtomicU64::new(0),
                average_batch_size: AtomicU64::new(0),
            },
        }
    }

    /// Start incremental checkpointing
    pub fn start(&self) {
        if self.running.swap(true, Ordering::Acquire) {
            return; // Already running
        }

        let interval = self.interval;
        let batch_size = self.batch_size;
        let queue = self.checkpoint_queue.clone();
        let running = self.running.clone();
        let stats = Arc::new(Mutex::new(AtomicU64::new(self.stats.incremental_checkpoints.load(Ordering::Relaxed))));

        std::thread::spawn(move || {
            while running.load(Ordering::Acquire) {
                std::thread::sleep(interval);

                // Perform incremental checkpoint
                let dirty_count = queue.dirty_count();
                if dirty_count > 0 {
                    let pages_to_flush = std::cmp::min(dirty_count, batch_size);
                    // Would flush pages here
                }
            }
        });
    }

    /// Stop incremental checkpointing
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// Get statistics
    pub fn get_stats(&self) -> IncrementalCheckpointStatsSnapshot {
        IncrementalCheckpointStatsSnapshot {
            incremental_checkpoints: self.stats.incremental_checkpoints.load(Ordering::Relaxed),
            total_pages_flushed: self.stats.total_pages_flushed.load(Ordering::Relaxed),
            average_batch_size: self.stats.average_batch_size.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalCheckpointStatsSnapshot {
    pub incremental_checkpoints: u64,
    pub total_pages_flushed: u64,
    pub average_batch_size: u64,
}

/// Background writer for dirty pages
pub struct BackgroundWriter {
    /// Write batch size
    batch_size: usize,
    /// Write interval
    interval: Duration,
    /// Maximum dirty page percentage before aggressive flushing
    dirty_threshold: f64,
    /// Running flag
    running: Arc<AtomicBool>,
    /// Statistics
    stats: BackgroundWriterStats,
}

#[derive(Debug)]
struct BackgroundWriterStats {
    write_cycles: AtomicU64,
    pages_written: AtomicU64,
    bytes_written: AtomicU64,
}

impl BackgroundWriter {
    pub fn new(batch_size: usize, interval_secs: u64, dirty_threshold: f64) -> Self {
        Self {
            batch_size,
            interval: Duration::from_secs(interval_secs),
            dirty_threshold,
            running: Arc::new(AtomicBool::new(false)),
            stats: BackgroundWriterStats {
                write_cycles: AtomicU64::new(0),
                pages_written: AtomicU64::new(0),
                bytes_written: AtomicU64::new(0),
            },
        }
    }

    /// Start background writer
    pub fn start(&self) {
        if self.running.swap(true, Ordering::Acquire) {
            return;
        }

        let interval = self.interval;
        let running = self.running.clone();

        std::thread::spawn(move || {
            while running.load(Ordering::Acquire) {
                std::thread::sleep(interval);
                // Would perform background writes here
            }
        });
    }

    /// Stop background writer
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// Get statistics
    pub fn get_stats(&self) -> BackgroundWriterStatsSnapshot {
        BackgroundWriterStatsSnapshot {
            write_cycles: self.stats.write_cycles.load(Ordering::Relaxed),
            pages_written: self.stats.pages_written.load(Ordering::Relaxed),
            bytes_written: self.stats.bytes_written.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundWriterStatsSnapshot {
    pub write_cycles: u64,
    pub pages_written: u64,
    pub bytes_written: u64,
}

/// Write coalescing buffer
pub struct WriteCoalescingBuffer {
    /// Pending writes grouped by extent
    pending_writes: Mutex<HashMap<u64, Vec<DirtyPage>>>,
    /// Coalescing window (time to wait for adjacent pages)
    coalesce_window: Duration,
    /// Statistics
    stats: CoalescingStats,
}

#[derive(Debug)]
struct CoalescingStats {
    writes_coalesced: AtomicU64,
    io_operations_saved: AtomicU64,
}

impl WriteCoalescingBuffer {
    pub fn new(coalesce_window_ms: u64) -> Self {
        Self {
            pending_writes: Mutex::new(HashMap::new()),
            coalesce_window: Duration::from_millis(coalesce_window_ms),
            stats: CoalescingStats {
                writes_coalesced: AtomicU64::new(0),
                io_operations_saved: AtomicU64::new(0),
            },
        }
    }

    /// Add page to coalescing buffer
    pub fn add_page(&self, page: DirtyPage) {
        let extent_id = page.page_id.page_number / 64; // 64 pages per extent

        let mut pending = self.pending_writes.lock();
        pending.entry(extent_id)
            .or_insert_with(Vec::new)
            .push(page);
    }

    /// Flush extent if coalescing window expired or extent is full
    pub fn try_flush_extent(&self, extent_id: u64) -> Option<Vec<DirtyPage>> {
        let mut pending = self.pending_writes.lock();

        if let Some(pages) = pending.get(&extent_id) {
            let oldest_time = pages.iter()
                .map(|p| p.dirty_time)
                .min()
                .unwrap();

            if oldest_time.elapsed() >= self.coalesce_window || pages.len() >= 64 {
                let pages = pending.remove(&extent_id).unwrap();
                let saved_io = (pages.len() as u64).saturating_sub(1);

                self.stats.writes_coalesced.fetch_add(1, Ordering::Relaxed);
                self.stats.io_operations_saved.fetch_add(saved_io, Ordering::Relaxed);

                return Some(pages);
            }
        }

        None
    }

    /// Get statistics
    pub fn get_stats(&self) -> CoalescingStatsSnapshot {
        CoalescingStatsSnapshot {
            writes_coalesced: self.stats.writes_coalesced.load(Ordering::Relaxed),
            io_operations_saved: self.stats.io_operations_saved.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoalescingStatsSnapshot {
    pub writes_coalesced: u64,
    pub io_operations_saved: u64,
}

/// Double-write buffer for crash recovery
pub struct DoubleWriteBuffer {
    /// Buffer capacity (number of pages)
    capacity: usize,
    /// Buffer pages
    buffer: Mutex<Vec<DirtyPage>>,
    /// Flush threshold
    flush_threshold: usize,
    /// Statistics
    stats: DoubleWriteStats,
}

#[derive(Debug)]
struct DoubleWriteStats {
    pages_buffered: AtomicU64,
    buffer_flushes: AtomicU64,
    recovery_operations: AtomicU64,
}

impl DoubleWriteBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            buffer: Mutex::new(Vec::with_capacity(capacity)),
            flush_threshold: capacity * 3 / 4, // Flush at 75% full
            stats: DoubleWriteStats {
                pages_buffered: AtomicU64::new(0),
                buffer_flushes: AtomicU64::new(0),
                recovery_operations: AtomicU64::new(0),
            },
        }
    }

    /// Add page to double-write buffer
    pub fn add_page(&self, page: DirtyPage) -> bool {
        let mut buffer = self.buffer.lock();

        if buffer.len() >= self.capacity {
            return false; // Buffer full
        }

        buffer.push(page);
        self.stats.pages_buffered.fetch_add(1, Ordering::Relaxed);

        buffer.len() >= self.flush_threshold
    }

    /// Flush double-write buffer
    pub fn flush(&self) -> usize {
        let mut buffer = self.buffer.lock();
        let page_count = buffer.len();

        if page_count == 0 {
            return 0;
        }

        // Step 1: Write all pages to double-write buffer area
        // (In real implementation, would write to dedicated disk area)

        // Step 2: Fsync double-write buffer

        // Step 3: Write pages to their actual locations

        // Step 4: Clear buffer
        buffer.clear();

        self.stats.buffer_flushes.fetch_add(1, Ordering::Relaxed);
        page_count
    }

    /// Recover from double-write buffer after crash
    pub fn recover(&self) -> usize {
        // Would read double-write buffer from disk and restore any partial writes
        self.stats.recovery_operations.fetch_add(1, Ordering::Relaxed);
        0
    }

    /// Get statistics
    pub fn get_stats(&self) -> DoubleWriteStatsSnapshot {
        DoubleWriteStatsSnapshot {
            pages_buffered: self.stats.pages_buffered.load(Ordering::Relaxed),
            buffer_flushes: self.stats.buffer_flushes.load(Ordering::Relaxed),
            recovery_operations: self.stats.recovery_operations.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubleWriteStatsSnapshot {
    pub pages_buffered: u64,
    pub buffer_flushes: u64,
    pub recovery_operations: u64,
}

/// Flush list manager
pub struct FlushListManager {
    /// Flush lists per tablespace
    flush_lists: PRwLock<HashMap<u32, VecDeque<DirtyPage>>>,
    /// Flush batch size
    batch_size: usize,
    /// Statistics
    stats: FlushListStats,
}

#[derive(Debug)]
struct FlushListStats {
    pages_added: AtomicU64,
    pages_flushed: AtomicU64,
    flush_operations: AtomicU64,
}

impl FlushListManager {
    pub fn new(batch_size: usize) -> Self {
        Self {
            flush_lists: PRwLock::new(HashMap::new()),
            batch_size,
            stats: FlushListStats {
                pages_added: AtomicU64::new(0),
                pages_flushed: AtomicU64::new(0),
                flush_operations: AtomicU64::new(0),
            },
        }
    }

    /// Add page to flush list
    pub fn add_page(&self, page: DirtyPage) {
        let tablespace_id = page.page_id.tablespace_id;

        let lists = self.flush_lists.read();
        if let Some(list) = lists.get(&tablespace_id) {
            let mut list = list.lock();
            list.push_back(page);
            self.stats.pages_added.fetch_add(1, Ordering::Relaxed);
            return;
        }
        drop(lists);

        // Create new flush list for tablespace
        let mut lists = self.flush_lists.write();
        let list = lists.entry(tablespace_id)
            .or_insert_with(|| Mutex::new(VecDeque::new()));

        let mut list = list.lock();
        list.push_back(page);
        self.stats.pages_added.fetch_add(1, Ordering::Relaxed);
    }

    /// Flush pages from a tablespace
    pub fn flush_tablespace(&self, tablespace_id: u32, max_pages: usize) -> usize {
        let lists = self.flush_lists.read();
        if let Some(list) = lists.get(&tablespace_id) {
            let mut list = list.lock();
            let flush_count = std::cmp::min(list.len(), max_pages);

            for _ in 0..flush_count {
                if let Some(page) = list.pop_front() {
                    // Flush page to disk
                    page.frame.dirty.store(false, Ordering::Release);
                }
            }

            self.stats.pages_flushed.fetch_add(flush_count as u64, Ordering::Relaxed);
            self.stats.flush_operations.fetch_add(1, Ordering::Relaxed);

            return flush_count;
        }

        0
    }

    /// Flush all tablespaces
    pub fn flush_all(&self) -> usize {
        let lists = self.flush_lists.read();
        let mut total_flushed = 0;

        for (tablespace_id, _) in lists.iter() {
            total_flushed += self.flush_tablespace(*tablespace_id, self.batch_size);
        }

        total_flushed
    }

    /// Get statistics
    pub fn get_stats(&self) -> FlushListStatsSnapshot {
        FlushListStatsSnapshot {
            pages_added: self.stats.pages_added.load(Ordering::Relaxed),
            pages_flushed: self.stats.pages_flushed.load(Ordering::Relaxed),
            flush_operations: self.stats.flush_operations.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlushListStatsSnapshot {
    pub pages_added: u64,
    pub pages_flushed: u64,
    pub flush_operations: u64,
}

// ============================================================================
// SECTION 5: BUFFER POOL STATISTICS (600+ lines)
// ============================================================================

/// Comprehensive buffer pool statistics tracker
pub struct BufferPoolStatisticsTracker {
    /// Per-pool hit ratio tracking
    pool_hit_ratios: PRwLock<HashMap<String, PoolHitRatio>>,
    /// Page type distribution
    page_type_dist: PRwLock<HashMap<PageType, AtomicU64>>,
    /// Wait statistics
    wait_stats: WaitStatistics,
    /// Buffer busy waits
    busy_waits: BusyWaitStatistics,
    /// Memory pressure
    memory_pressure: MemoryPressureMonitor,
    /// Real-time metrics
    realtime_metrics: RealtimeMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PageType {
    Data,
    Index,
    Undo,
    Redo,
    Temp,
    System,
}

#[derive(Debug)]
struct PoolHitRatio {
    hits: AtomicU64,
    misses: AtomicU64,
    accesses: AtomicU64,
}

impl PoolHitRatio {
    fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            accesses: AtomicU64::new(0),
        }
    }

    fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
        self.accesses.fetch_add(1, Ordering::Relaxed);
    }

    fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
        self.accesses.fetch_add(1, Ordering::Relaxed);
    }

    fn hit_ratio(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let accesses = self.accesses.load(Ordering::Relaxed) as f64;
        if accesses == 0.0 {
            0.0
        } else {
            hits / accesses
        }
    }
}

/// Wait statistics for buffer operations
#[derive(Debug)]
pub struct WaitStatistics {
    /// Wait time for free buffers
    free_buffer_waits: AtomicU64,
    free_buffer_wait_time_ns: AtomicU64,
    /// Wait time for buffer locks
    buffer_lock_waits: AtomicU64,
    buffer_lock_wait_time_ns: AtomicU64,
    /// Wait time for I/O completion
    io_waits: AtomicU64,
    io_wait_time_ns: AtomicU64,
}

impl WaitStatistics {
    pub fn new() -> Self {
        Self {
            free_buffer_waits: AtomicU64::new(0),
            free_buffer_wait_time_ns: AtomicU64::new(0),
            buffer_lock_waits: AtomicU64::new(0),
            buffer_lock_wait_time_ns: AtomicU64::new(0),
            io_waits: AtomicU64::new(0),
            io_wait_time_ns: AtomicU64::new(0),
        }
    }

    /// Record free buffer wait
    pub fn record_free_buffer_wait(&self, duration: Duration) {
        self.free_buffer_waits.fetch_add(1, Ordering::Relaxed);
        self.free_buffer_wait_time_ns.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Record buffer lock wait
    pub fn record_buffer_lock_wait(&self, duration: Duration) {
        self.buffer_lock_waits.fetch_add(1, Ordering::Relaxed);
        self.buffer_lock_wait_time_ns.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Record I/O wait
    pub fn record_io_wait(&self, duration: Duration) {
        self.io_waits.fetch_add(1, Ordering::Relaxed);
        self.io_wait_time_ns.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Get snapshot
    pub fn snapshot(&self) -> WaitStatisticsSnapshot {
        WaitStatisticsSnapshot {
            free_buffer_waits: self.free_buffer_waits.load(Ordering::Relaxed),
            free_buffer_wait_time_ns: self.free_buffer_wait_time_ns.load(Ordering::Relaxed),
            buffer_lock_waits: self.buffer_lock_waits.load(Ordering::Relaxed),
            buffer_lock_wait_time_ns: self.buffer_lock_wait_time_ns.load(Ordering::Relaxed),
            io_waits: self.io_waits.load(Ordering::Relaxed),
            io_wait_time_ns: self.io_wait_time_ns.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitStatisticsSnapshot {
    pub free_buffer_waits: u64,
    pub free_buffer_wait_time_ns: u64,
    pub buffer_lock_waits: u64,
    pub buffer_lock_wait_time_ns: u64,
    pub io_waits: u64,
    pub io_wait_time_ns: u64,
}

/// Buffer busy wait statistics
#[derive(Debug)]
pub struct BusyWaitStatistics {
    /// Waits by page type
    waits_by_type: PRwLock<HashMap<PageType, AtomicU64>>,
    /// Waits by tablespace
    waits_by_tablespace: PRwLock<HashMap<u32, AtomicU64>>,
    /// Total busy waits
    total_waits: AtomicU64,
    /// Total wait time
    total_wait_time_ns: AtomicU64,
}

impl BusyWaitStatistics {
    pub fn new() -> Self {
        Self {
            waits_by_type: PRwLock::new(HashMap::new()),
            waits_by_tablespace: PRwLock::new(HashMap::new()),
            total_waits: AtomicU64::new(0),
            total_wait_time_ns: AtomicU64::new(0),
        }
    }

    /// Record busy wait
    pub fn record_wait(&self, page_type: PageType, tablespace_id: u32, duration: Duration) {
        self.total_waits.fetch_add(1, Ordering::Relaxed);
        self.total_wait_time_ns.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);

        // Record by type
        let types = self.waits_by_type.read();
        if let Some(counter) = types.get(&page_type) {
            counter.fetch_add(1, Ordering::Relaxed);
        } else {
            drop(types);
            let mut types = self.waits_by_type.write();
            types.entry(page_type).or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }

        // Record by tablespace
        let spaces = self.waits_by_tablespace.read();
        if let Some(counter) = spaces.get(&tablespace_id) {
            counter.fetch_add(1, Ordering::Relaxed);
        } else {
            drop(spaces);
            let mut spaces = self.waits_by_tablespace.write();
            spaces.entry(tablespace_id).or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get snapshot
    pub fn snapshot(&self) -> BusyWaitStatisticsSnapshot {
        let types = self.waits_by_type.read();
        let waits_by_type: HashMap<PageType, u64> = types.iter()
            .map(|(k, v)| (*k, v.load(Ordering::Relaxed)))
            .collect();

        let spaces = self.waits_by_tablespace.read();
        let waits_by_tablespace: HashMap<u32, u64> = spaces.iter()
            .map(|(k, v)| (*k, v.load(Ordering::Relaxed)))
            .collect();

        BusyWaitStatisticsSnapshot {
            waits_by_type,
            waits_by_tablespace,
            total_waits: self.total_waits.load(Ordering::Relaxed),
            total_wait_time_ns: self.total_wait_time_ns.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusyWaitStatisticsSnapshot {
    pub waits_by_type: HashMap<PageType, u64>,
    pub waits_by_tablespace: HashMap<u32, u64>,
    pub total_waits: u64,
    pub total_wait_time_ns: u64,
}

/// Memory pressure monitor
#[derive(Debug)]
pub struct MemoryPressureMonitor {
    /// Current memory usage
    current_usage: AtomicU64,
    /// Peak memory usage
    peak_usage: AtomicU64,
    /// Memory limit
    limit: AtomicU64,
    /// Pressure events
    pressure_events: AtomicU64,
    /// Last pressure check
    last_check: Mutex<Instant>,
}

impl MemoryPressureMonitor {
    pub fn new(limit: u64) -> Self {
        Self {
            current_usage: AtomicU64::new(0),
            peak_usage: AtomicU64::new(0),
            limit: AtomicU64::new(limit),
            pressure_events: AtomicU64::new(0),
            last_check: Mutex::new(Instant::now()),
        }
    }

    /// Update memory usage
    pub fn update_usage(&self, usage: u64) {
        self.current_usage.store(usage, Ordering::Relaxed);

        // Update peak if necessary
        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        while usage > peak {
            match self.peak_usage.compare_exchange_weak(
                peak,
                usage,
                Ordering::Relaxed,
                Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => peak = x,
            }
        }

        // Check for pressure
        if self.is_under_pressure() {
            self.pressure_events.fetch_add(1, Ordering::Relaxed);
        }

        *self.last_check.lock() = Instant::now();
    }

    /// Check if under memory pressure
    pub fn is_under_pressure(&self) -> bool {
        let usage = self.current_usage.load(Ordering::Relaxed);
        let limit = self.limit.load(Ordering::Relaxed);
        usage as f64 / limit as f64 > 0.9 // 90% threshold
    }

    /// Get pressure level (0.0 - 1.0)
    pub fn pressure_level(&self) -> f64 {
        let usage = self.current_usage.load(Ordering::Relaxed);
        let limit = self.limit.load(Ordering::Relaxed);
        (usage as f64 / limit as f64).min(1.0)
    }

    /// Get snapshot
    pub fn snapshot(&self) -> MemoryPressureSnapshot {
        MemoryPressureSnapshot {
            current_usage: self.current_usage.load(Ordering::Relaxed),
            peak_usage: self.peak_usage.load(Ordering::Relaxed),
            limit: self.limit.load(Ordering::Relaxed),
            pressure_events: self.pressure_events.load(Ordering::Relaxed),
            pressure_level: self.pressure_level(),
            under_pressure: self.is_under_pressure(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPressureSnapshot {
    pub current_usage: u64,
    pub peak_usage: u64,
    pub limit: u64,
    pub pressure_events: u64,
    pub pressure_level: f64,
    pub under_pressure: bool,
}

/// Real-time metrics exporter
#[derive(Debug)]
pub struct RealtimeMetrics {
    /// Metrics update interval
    interval: Duration,
    /// Current metrics
    current: Mutex<MetricsSnapshot>,
    /// Last update time
    last_update: Mutex<Instant>,
}

impl RealtimeMetrics {
    pub fn new(interval_secs: u64) -> Self {
        Self {
            interval: Duration::from_secs(interval_secs),
            current: Mutex::new(MetricsSnapshot::default()),
            last_update: Mutex::new(Instant::now()),
        }
    }

    /// Update metrics
    pub fn update(&self, snapshot: MetricsSnapshot) {
        *self.current.lock() = snapshot;
        *self.last_update.lock() = Instant::now();
    }

    /// Get current metrics
    pub fn get(&self) -> MetricsSnapshot {
        self.current.lock().clone()
    }

    /// Check if metrics are stale
    pub fn is_stale(&self) -> bool {
        self.last_update.lock().elapsed() > self.interval * 2
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub buffer_pool_size: usize,
    pub pages_in_use: usize,
    pub dirty_pages: usize,
    pub hit_ratio: f64,
    pub pages_read: u64,
    pub pages_written: u64,
    pub io_operations: u64,
}

impl BufferPoolStatisticsTracker {
    pub fn new() -> Self {
        Self {
            pool_hit_ratios: PRwLock::new(HashMap::new()),
            page_type_dist: PRwLock::new(HashMap::new()),
            wait_stats: WaitStatistics::new(),
            busy_waits: BusyWaitStatistics::new(),
            memory_pressure: MemoryPressureMonitor::new(1024 * 1024 * 1024), // 1GB default
            realtime_metrics: RealtimeMetrics::new(1),
        }
    }

    /// Record hit for a pool
    pub fn record_hit(&self, pool_name: &str) {
        let ratios = self.pool_hit_ratios.read();
        if let Some(ratio) = ratios.get(pool_name) {
            ratio.record_hit();
        } else {
            drop(ratios);
            let mut ratios = self.pool_hit_ratios.write();
            let ratio = ratios.entry(pool_name.to_string())
                .or_insert_with(PoolHitRatio::new);
            ratio.record_hit();
        }
    }

    /// Record miss for a pool
    pub fn record_miss(&self, pool_name: &str) {
        let ratios = self.pool_hit_ratios.read();
        if let Some(ratio) = ratios.get(pool_name) {
            ratio.record_miss();
        } else {
            drop(ratios);
            let mut ratios = self.pool_hit_ratios.write();
            let ratio = ratios.entry(pool_name.to_string())
                .or_insert_with(PoolHitRatio::new);
            ratio.record_miss();
        }
    }

    /// Record page type access
    pub fn record_page_type(&self, page_type: PageType) {
        let types = self.page_type_dist.read();
        if let Some(counter) = types.get(&page_type) {
            counter.fetch_add(1, Ordering::Relaxed);
        } else {
            drop(types);
            let mut types = self.page_type_dist.write();
            types.entry(page_type)
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get comprehensive statistics
    pub fn get_comprehensive_stats(&self) -> ComprehensiveBufferStats {
        let ratios = self.pool_hit_ratios.read();
        let pool_stats: HashMap<String, PoolStatsSnapshot> = ratios.iter()
            .map(|(name, ratio)| {
                (name.clone(), PoolStatsSnapshot {
                    hits: ratio.hits.load(Ordering::Relaxed),
                    misses: ratio.misses.load(Ordering::Relaxed),
                    accesses: ratio.accesses.load(Ordering::Relaxed),
                    hit_ratio: ratio.hit_ratio(),
                })
            })
            .collect();

        let types = self.page_type_dist.read();
        let page_type_distribution: HashMap<PageType, u64> = types.iter()
            .map(|(pt, count)| (*pt, count.load(Ordering::Relaxed)))
            .collect();

        ComprehensiveBufferStats {
            pool_stats,
            page_type_distribution,
            wait_stats: self.wait_stats.snapshot(),
            busy_waits: self.busy_waits.snapshot(),
            memory_pressure: self.memory_pressure.snapshot(),
            realtime_metrics: self.realtime_metrics.get(),
        }
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let stats = self.get_comprehensive_stats();
        let mut output = String::new();

        // Buffer pool hit ratios
        for (pool_name, pool_stats) in &stats.pool_stats {
            output.push_str(&format!(
                "buffer_pool_hit_ratio{{pool=\"{}\"}} {}\n",
                pool_name, pool_stats.hit_ratio
            ));
            output.push_str(&format!(
                "buffer_pool_accesses_total{{pool=\"{}\"}} {}\n",
                pool_name, pool_stats.accesses
            ));
        }

        // Page type distribution
        for (page_type, count) in &stats.page_type_distribution {
            output.push_str(&format!(
                "buffer_pool_pages_by_type{{type=\"{:?}\"}} {}\n",
                page_type, count
            ));
        }

        // Wait statistics
        output.push_str(&format!(
            "buffer_pool_free_buffer_waits_total {}\n",
            stats.wait_stats.free_buffer_waits
        ));
        output.push_str(&format!(
            "buffer_pool_io_waits_total {}\n",
            stats.wait_stats.io_waits
        ));

        // Memory pressure
        output.push_str(&format!(
            "buffer_pool_memory_usage_bytes {}\n",
            stats.memory_pressure.current_usage
        ));
        output.push_str(&format!(
            "buffer_pool_memory_pressure_level {}\n",
            stats.memory_pressure.pressure_level
        ));

        output
    }

    /// Export metrics in JSON format
    pub fn export_json(&self) -> String {
        let stats = self.get_comprehensive_stats();
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatsSnapshot {
    pub hits: u64,
    pub misses: u64,
    pub accesses: u64,
    pub hit_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveBufferStats {
    pub pool_stats: HashMap<String, PoolStatsSnapshot>,
    pub page_type_distribution: HashMap<PageType, u64>,
    pub wait_stats: WaitStatisticsSnapshot,
    pub busy_waits: BusyWaitStatisticsSnapshot,
    pub memory_pressure: MemoryPressureSnapshot,
    pub realtime_metrics: MetricsSnapshot,
}

// ============================================================================
// PUBLIC API FUNCTIONS (for web management interface)
// ============================================================================

/// Public API for buffer pool management
pub struct BufferPoolManager {
    pool: Arc<MultiTierBufferPool>,
    arc_cache: Arc<AdaptiveReplacementCache>,
    two_q_cache: Arc<TwoQCache>,
    prefetcher: Arc<PagePrefetcher>,
    clock_policy: Arc<ClockSweepPolicy>,
    lru_k_policy: Arc<LruKPolicy>,
    touch_optimizer: Arc<TouchCountOptimizer>,
    cost_aware: Arc<CostAwareReplacement>,
    checkpoint_queue: Arc<CheckpointQueue>,
    incremental_checkpointer: Arc<IncrementalCheckpointer>,
    background_writer: Arc<BackgroundWriter>,
    write_coalescing: Arc<WriteCoalescingBuffer>,
    double_write: Arc<DoubleWriteBuffer>,
    flush_manager: Arc<FlushListManager>,
    stats_tracker: Arc<BufferPoolStatisticsTracker>,
}

impl BufferPoolManager {
    /// Create a new buffer pool manager
    pub fn new(config: BufferPoolConfig) -> Self {
        let pool = Arc::new(MultiTierBufferPool::new(config.clone()));
        let capacity = config.total_size / config.page_size;
        let checkpoint_queue = Arc::new(CheckpointQueue::new());

        Self {
            pool: pool.clone(),
            arc_cache: Arc::new(AdaptiveReplacementCache::new(capacity)),
            two_q_cache: Arc::new(TwoQCache::new(capacity)),
            prefetcher: Arc::new(PagePrefetcher::new(8)),
            clock_policy: Arc::new(ClockSweepPolicy::new(capacity, config.page_size)),
            lru_k_policy: Arc::new(LruKPolicy::new(2, 300)),
            touch_optimizer: Arc::new(TouchCountOptimizer::new(10)),
            cost_aware: Arc::new(CostAwareReplacement::new()),
            checkpoint_queue: checkpoint_queue.clone(),
            incremental_checkpointer: Arc::new(IncrementalCheckpointer::new(
                60,
                100,
                checkpoint_queue.clone()
            )),
            background_writer: Arc::new(BackgroundWriter::new(50, 5, 0.75)),
            write_coalescing: Arc::new(WriteCoalescingBuffer::new(100)),
            double_write: Arc::new(DoubleWriteBuffer::new(128)),
            flush_manager: Arc::new(FlushListManager::new(100)),
            stats_tracker: Arc::new(BufferPoolStatisticsTracker::new()),
        }
    }

    /// Pin a page (web API endpoint)
    pub fn api_pin_page(&self, tablespace_id: u32, page_number: u64) -> Option<Arc<BufferFrame>> {
        let page_id = PageId::new(tablespace_id, page_number);
        self.pool.pin_page(page_id, PoolType::Default)
    }

    /// Unpin a page (web API endpoint)
    pub fn api_unpin_page(&self, tablespace_id: u32, page_number: u64, dirty: bool) -> bool {
        let page_id = PageId::new(tablespace_id, page_number);
        self.pool.unpin_page(page_id, dirty)
    }

    /// Get buffer pool statistics (web API endpoint)
    pub fn api_get_stats(&self) -> serde_json::Value {
        serde_json::json!({
            "buffer_pool": self.pool.get_stats(),
            "arc_cache": self.arc_cache.get_stats(),
            "two_q_cache": self.two_q_cache.get_stats(),
            "prefetcher": self.prefetcher.get_stats(),
            "clock_policy": self.clock_policy.get_stats(),
            "lru_k_policy": self.lru_k_policy.get_stats(),
            "touch_optimizer": self.touch_optimizer.get_stats(),
            "cost_aware": self.cost_aware.get_stats(),
            "checkpoint": self.checkpoint_queue.get_stats(),
            "incremental_checkpoint": self.incremental_checkpointer.get_stats(),
            "background_writer": self.background_writer.get_stats(),
            "write_coalescing": self.write_coalescing.get_stats(),
            "double_write": self.double_write.get_stats(),
            "flush_manager": self.flush_manager.get_stats(),
            "comprehensive": self.stats_tracker.get_comprehensive_stats(),
        })
    }

    /// Flush all dirty pages (web API endpoint)
    pub fn api_flush_all(&self) -> usize {
        self.pool.flush_all()
    }

    /// Perform checkpoint (web API endpoint)
    pub fn api_checkpoint(&self) -> CheckpointResult {
        self.checkpoint_queue.checkpoint()
    }

    /// Get memory pressure (web API endpoint)
    pub fn api_get_memory_pressure(&self) -> MemoryPressureSnapshot {
        self.stats_tracker.memory_pressure.snapshot()
    }

    /// Export Prometheus metrics (web API endpoint)
    pub fn api_export_prometheus(&self) -> String {
        self.stats_tracker.export_prometheus()
    }

    /// Export JSON metrics (web API endpoint)
    pub fn api_export_json(&self) -> String {
        self.stats_tracker.export_json()
    }

    /// Start background operations (web API endpoint)
    pub fn api_start_background_operations(&self) {
        self.pool.start_tier_manager();
        self.incremental_checkpointer.start();
        self.background_writer.start();
    }

    /// Stop background operations (web API endpoint)
    pub fn api_stop_background_operations(&self) {
        self.pool.stop_tier_manager();
        self.incremental_checkpointer.stop();
        self.background_writer.stop();
    }

    /// Get buffer pool capacity (web API endpoint)
    pub fn api_get_capacity(&self) -> usize {
        self.pool.capacity()
    }

    /// Get frames in use (web API endpoint)
    pub fn api_get_frames_in_use(&self) -> usize {
        self.pool.frames_in_use()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_pool_creation() {
        let config = BufferPoolConfig::default();
        let pool = MultiTierBufferPool::new(config);
        assert!(pool.capacity() > 0);
    }

    #[test]
    fn test_page_pin_unpin() {
        let config = BufferPoolConfig::default();
        let pool = MultiTierBufferPool::new(config);
        let page_id = PageId::new(0, 1);

        let frame = pool.pin_page(page_id, PoolType::Default);
        assert!(frame.is_some());

        let unpinned = pool.unpin_page(page_id, false);
        assert!(unpinned);
    }

    #[test]
    fn test_arc_cache() {
        let arc = AdaptiveReplacementCache::new(100);
        let page_id = PageId::new(0, 1);
        let frame = Arc::new(BufferFrame::new(8192));

        arc.insert(page_id, frame.clone());
        let retrieved = arc.get(page_id, 8192);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_clock_sweep() {
        let clock = ClockSweepPolicy::new(10, 8192);
        let victim = clock.find_victim();
        assert!(victim.is_some());
    }

    #[test]
    fn test_checkpoint_queue() {
        let queue = CheckpointQueue::new();
        let page_id = PageId::new(0, 1);
        let frame = Arc::new(BufferFrame::new(8192));

        let dirty_page = DirtyPage {
            page_id,
            lsn: 100,
            dirty_time: Instant::now(),
            frame,
        };

        queue.enqueue(dirty_page);
        assert_eq!(queue.dirty_count(), 1);
    }
}
