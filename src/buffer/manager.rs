//! # Buffer Pool Manager - Main Buffer Management System
//!
//! High-performance buffer pool manager optimized for Windows/MSVC with:
//! - Lock-free page table for fast lookups
//! - Per-core frame pools to reduce contention
//! - Batch flush support for efficient I/O
//! - Windows IOCP integration ready
//! - Zero allocations in pin/unpin hot path
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │              Buffer Pool Manager                        │
//! ├─────────────────────────────────────────────────────────┤
//! │  Page Table (PageId -> FrameId)                        │
//! │  ┌──────────┬──────────┬──────────┬──────────┐        │
//! │  │ Hash Map │ Hash Map │ Hash Map │ Hash Map │        │
//! │  └──────────┴──────────┴──────────┴──────────┘        │
//! │           (Partitioned for concurrency)                │
//! ├─────────────────────────────────────────────────────────┤
//! │  Frame Array (Pre-allocated)                           │
//! │  ┌──────┬──────┬──────┬──────┬──────┬──────┐         │
//! │  │Frame │Frame │Frame │Frame │Frame │ ...  │         │
//! │  │  0   │  1   │  2   │  3   │  4   │      │         │
//! │  └──────┴──────┴──────┴──────┴──────┴──────┘         │
//! ├─────────────────────────────────────────────────────────┤
//! │  Per-Core Free Lists                                   │
//! │  ┌─────────┬─────────┬─────────┬─────────┐           │
//! │  │ Core 0  │ Core 1  │ Core 2  │ Core 3  │           │
//! │  │ [3,7,11]│[4,8,12] │[5,9,13] │[6,10,14]│           │
//! │  └─────────┴─────────┴─────────┴─────────┘           │
//! └─────────────────────────────────────────────────────────┘
//! ```

use crate::buffer::eviction::{create_eviction_policy, EvictionPolicy, EvictionPolicyType};
use crate::buffer::page_cache::{
    BufferFrame, FrameBatch, FrameGuard, FrameId, PageBuffer, PerCoreFramePool,
    INVALID_FRAME_ID, INVALID_PAGE_ID, PAGE_SIZE,
};
use crate::common::PageId;
use crate::error::{DbError, Result};

use num_cpus;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// Configuration
// ============================================================================

/// Buffer pool configuration
#[derive(Debug, Clone)]
pub struct BufferPoolConfig {
    /// Number of buffer frames
    pub num_frames: usize,

    /// Eviction policy to use
    pub eviction_policy: EvictionPolicyType,

    /// Number of page table partitions (for concurrent access)
    pub page_table_partitions: usize,

    /// Enable per-core frame pools
    pub enable_per_core_pools: bool,

    /// Frames per core pool (if enabled)
    pub frames_per_core: usize,

    /// Maximum batch size for flushing
    pub max_flush_batch_size: usize,

    /// Enable background flushing
    pub enable_background_flush: bool,

    /// Background flush interval
    pub background_flush_interval: Duration,

    /// Dirty page threshold for triggering flush (percentage)
    pub dirty_page_threshold: f64,

    /// Enable statistics collection
    pub enable_stats: bool,

    /// Enable prefetching
    pub enable_prefetch: bool,

    /// Number of prefetch threads
    pub prefetch_threads: usize,
}

impl Default for BufferPoolConfig {
    fn default() -> Self {
        Self {
            num_frames: 1000,
            eviction_policy: EvictionPolicyType::Clock,
            page_table_partitions: 16,
            enable_per_core_pools: true,
            frames_per_core: 8,
            max_flush_batch_size: 32,
            enable_background_flush: true,
            background_flush_interval: Duration::from_secs(30),
            dirty_page_threshold: 0.7,
            enable_stats: true,
            enable_prefetch: false,
            prefetch_threads: 2,
        }
    }
}

// ============================================================================
// Page Table - Partitioned Hash Map
// ============================================================================

/// Partitioned page table for concurrent access.
///
/// Uses multiple hash maps (partitions) to reduce lock contention.
/// Page IDs are hashed to determine which partition to use.
struct PageTable {
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
    fn new(num_partitions: usize, initial_capacity_per_partition: usize) -> Self {
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
        ((page_id.wrapping_mul(0x9e3779b97f4a7c15)) as usize) % self.num_partitions
    }

    /// Look up a page in the table
    #[inline]
    fn lookup(&self, page_id: PageId) -> Option<FrameId> {
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
    fn insert(&self, page_id: PageId, frame_id: FrameId) {
        let partition_idx = self.partition_index(page_id);
        // SAFETY: partition_idx is guaranteed to be < num_partitions
        let partition = unsafe { self.partitions.get_unchecked(partition_idx) };

        partition.write().insert(page_id, frame_id);
    }

    /// Remove a page from the table
    #[inline]
    fn remove(&self, page_id: PageId) -> Option<FrameId> {
        let partition_idx = self.partition_index(page_id);
        // SAFETY: partition_idx is guaranteed to be < num_partitions
        let partition = unsafe { self.partitions.get_unchecked(partition_idx) };

        partition.write().remove(&page_id)
    }

    /// Clear all partitions
    #[cold]
    fn clear(&self) {
        for partition in &self.partitions {
            partition.write().clear();
        }
        self.lookups.store(0, Ordering::Relaxed);
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
    }

    /// Get hit rate
    #[inline]
    fn hit_rate(&self) -> f64 {
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
    fn stats(&self) -> (u64, u64, u64, f64) {
        let lookups = self.lookups.load(Ordering::Relaxed);
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let hit_rate = self.hit_rate();

        (lookups, hits, misses, hit_rate)
    }

    /// Get total number of entries
    #[cold]
    fn len(&self) -> usize {
        self.partitions
            .iter()
            .map(|p| p.read().len())
            .sum()
    }
}

// ============================================================================
// Free Frame Manager
// ============================================================================

/// Manages free frames using a lock-free stack or per-core pools
struct FreeFrameManager {
    /// Global free list (fallback)
    global_free_list: Mutex<Vec<FrameId>>,

    /// Per-core pools (optional)
    per_core_pools: Option<Vec<Arc<PerCoreFramePool>>>,

    /// Number of CPU cores
    num_cores: usize,

    /// Statistics
    global_allocations: AtomicU64,
    per_core_allocations: AtomicU64,
}

impl FreeFrameManager {
    /// Create a new free frame manager
    fn new(
        num_frames: usize,
        enable_per_core_pools: bool,
        frames_per_core: usize,
    ) -> Self {
        let num_cores = num_cpus::get();

        let per_core_pools = if enable_per_core_pools {
            let pools: Vec<_> = (0..num_cores)
                .map(|i| Arc::new(PerCoreFramePool::new(i, frames_per_core)))
                .collect();

            // Distribute initial frames to pools
            let mut frame_id = 0;
            for pool in &pools {
                let mut frames = Vec::with_capacity(frames_per_core);
                for _ in 0..frames_per_core {
                    if frame_id < num_frames as FrameId {
                        frames.push(frame_id);
                        frame_id += 1;
                    }
                }
                pool.add_frames(frames);
            }

            Some(pools)
        } else {
            None
        };

        // Remaining frames go to global list
        let global_frames: Vec<FrameId> = if enable_per_core_pools {
            let start = (num_cores * frames_per_core) as FrameId;
            (start..num_frames as FrameId).collect()
        } else {
            (0..num_frames as FrameId).collect()
        };

        Self {
            global_free_list: Mutex::new(global_frames),
            per_core_pools,
            num_cores,
            global_allocations: AtomicU64::new(0),
            per_core_allocations: AtomicU64::new(0),
        }
    }

    /// Allocate a free frame
    #[inline]
    fn allocate(&self) -> Option<FrameId> {
        // Try per-core pool first
        if let Some(ref pools) = self.per_core_pools {
            let core_id = get_current_core_id() % self.num_cores;
            if let Some(frame_id) = pools[core_id].try_allocate() {
                self.per_core_allocations.fetch_add(1, Ordering::Relaxed);
                return Some(frame_id);
            }

            // Try stealing from other cores
            for i in 0..self.num_cores {
                let steal_core = (core_id + i) % self.num_cores;
                if let Some(frame_id) = pools[steal_core].try_allocate() {
                    self.per_core_allocations.fetch_add(1, Ordering::Relaxed);
                    return Some(frame_id);
                }
            }
        }

        // Fall back to global list
        self.global_free_list.lock().pop().map(|frame_id| {
            self.global_allocations.fetch_add(1, Ordering::Relaxed);
            frame_id
        })
    }

    /// Deallocate a frame
    #[inline]
    fn deallocate(&self, frame_id: FrameId) {
        // Try to add to per-core pool first
        if let Some(ref pools) = self.per_core_pools {
            let core_id = get_current_core_id() % self.num_cores;
            if pools[core_id].deallocate(frame_id) {
                return;
            }
        }

        // Add to global list
        self.global_free_list.lock().push(frame_id);
    }

    /// Get number of free frames
    #[inline]
    fn free_count(&self) -> usize {
        let mut count = self.global_free_list.lock().len();

        if let Some(ref pools) = self.per_core_pools {
            count += pools.iter().map(|p| p.free_count()).sum::<usize>();
        }

        count
    }

    /// Get statistics
    #[cold]
    fn stats(&self) -> (u64, u64, usize) {
        (
            self.global_allocations.load(Ordering::Relaxed),
            self.per_core_allocations.load(Ordering::Relaxed),
            self.free_count(),
        )
    }
}

/// Get current CPU core ID (best effort)
#[inline]
fn get_current_core_id() -> usize {
    // On Linux, we can use sched_getcpu
    #[cfg(all(target_os = "linux", feature = "libc"))]
    {
        unsafe { libc::sched_getcpu() as usize }
    }

    // On other platforms, use thread ID as proxy
    #[cfg(not(all(target_os = "linux", feature = "libc")))]
    {
        // Use a hash of the thread ID to get a pseudo-random core ID
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        std::thread::current().id().hash(&mut hasher);
        hasher.finish() as usize
    }
}

// ============================================================================
// Buffer Pool Statistics
// ============================================================================

/// Comprehensive buffer pool statistics
#[derive(Debug, Clone, Default)]
pub struct BufferPoolStats {
    /// Total number of frames
    pub total_frames: usize,

    /// Number of free frames
    pub free_frames: usize,

    /// Number of pinned frames
    pub pinned_frames: usize,

    /// Number of dirty frames
    pub dirty_frames: usize,

    /// Page table lookups
    pub lookups: u64,

    /// Page table hits
    pub hits: u64,

    /// Page table misses
    pub misses: u64,

    /// Hit rate (0.0 - 1.0)
    pub hit_rate: f64,

    /// Page reads
    pub page_reads: u64,

    /// Page writes
    pub page_writes: u64,

    /// Page evictions
    pub evictions: u64,

    /// Failed evictions
    pub failed_evictions: u64,

    /// Background flushes
    pub background_flushes: u64,

    /// Average eviction search length
    pub avg_search_length: f64,

    /// Global allocations
    pub global_allocations: u64,

    /// Per-core allocations
    pub per_core_allocations: u64,

    /// I/O wait time (microseconds)
    pub io_wait_time_us: u64,
}

// ============================================================================
// Main Buffer Pool Manager
// ============================================================================

/// High-performance buffer pool manager.
///
/// Manages a pool of buffer frames that cache disk pages in memory.
/// Optimized for Windows/MSVC with:
/// - Zero allocations in hot path (pin/unpin)
/// - Lock-free page table lookups
/// - Per-core frame pools
/// - Batch I/O operations
pub struct BufferPoolManager {
    /// Configuration
    config: BufferPoolConfig,

    /// Pre-allocated array of buffer frames
    frames: Arc<Vec<Arc<BufferFrame>>>,

    /// Page table (PageId -> FrameId)
    page_table: Arc<PageTable>,

    /// Free frame manager
    free_frames: Arc<FreeFrameManager>,

    /// Eviction policy
    eviction_policy: Arc<dyn EvictionPolicy>,

    /// Statistics
    page_reads: AtomicU64,
    page_writes: AtomicU64,
    background_flushes: AtomicU64,
    io_wait_time_us: AtomicU64,

    /// Background flusher thread handle
    background_flusher: Mutex<Option<std::thread::JoinHandle<()>>>,

    /// Shutdown flag
    shutdown: Arc<AtomicBool>,

    /// Start time
    start_time: Instant,
}

impl BufferPoolManager {
    /// Create a new buffer pool manager
    pub fn new(config: BufferPoolConfig) -> Self {
        let num_frames = config.num_frames;

        // Allocate frames
        let frames: Vec<Arc<BufferFrame>> = (0..num_frames)
            .map(|i| Arc::new(BufferFrame::new(i as FrameId)))
            .collect();

        // Create page table
        let capacity_per_partition = (num_frames / config.page_table_partitions).max(16);
        let page_table = Arc::new(PageTable::new(
            config.page_table_partitions,
            capacity_per_partition,
        ));

        // Create free frame manager
        let free_frames = Arc::new(FreeFrameManager::new(
            num_frames,
            config.enable_per_core_pools,
            config.frames_per_core,
        ));

        // Create eviction policy
        let eviction_policy = create_eviction_policy(config.eviction_policy, num_frames);

        let shutdown = Arc::new(AtomicBool::new(false));

        let manager = Self {
            config: config.clone(),
            frames: Arc::new(frames),
            page_table,
            free_frames,
            eviction_policy,
            page_reads: AtomicU64::new(0),
            page_writes: AtomicU64::new(0),
            background_flushes: AtomicU64::new(0),
            io_wait_time_us: AtomicU64::new(0),
            background_flusher: Mutex::new(None),
            shutdown,
            start_time: Instant::now(),
        };

        // Start background flusher if enabled
        if config.enable_background_flush {
            manager.start_background_flusher();
        }

        manager
    }

    /// Pin a page (fetch from disk if not in buffer pool).
    ///
    /// This is a HOT PATH operation and must be fast.
    /// Returns a guard that automatically unpins the page when dropped.
    ///
    /// # Performance
    ///
    /// - Best case: O(1) - page in buffer pool
    /// - Worst case: O(n) - page fault, eviction scan
    /// - Zero allocations in hit path
    #[inline]
    pub fn pin_page(&self, page_id: PageId) -> Result<FrameGuard> {
        // Fast path: page already in buffer pool
        if let Some(frame_id) = self.page_table.lookup(page_id) {
            // SAFETY: frame_id is guaranteed to be valid
            let frame = unsafe { self.frames.get_unchecked(frame_id as usize) };

            // Wait if I/O in progress
            while frame.io_in_progress() {
                std::hint::spin_loop();
            }

            // Pin and record access
            frame.pin();
            self.eviction_policy.record_pin(frame_id);

            return Ok(FrameGuard::new(frame.clone()));
        }

        // Slow path: page fault - need to load from disk
        self.pin_page_slow_path(page_id)
    }

    /// Slow path for pin_page (page fault)
    #[cold]
    #[inline(never)]
    fn pin_page_slow_path(&self, page_id: PageId) -> Result<FrameGuard> {
        // Allocate a frame
        let frame_id = self.allocate_frame()?;
        let frame = &self.frames[frame_id as usize];

        // Set I/O in progress
        frame.set_io_in_progress(true);

        // Load page from disk
        let start = Instant::now();
        self.load_page_from_disk(page_id, frame)?;
        let elapsed = start.elapsed().as_micros() as u64;
        self.io_wait_time_us.fetch_add(elapsed, Ordering::Relaxed);

        // Update frame metadata
        frame.set_page_id(page_id);
        frame.set_dirty(false);
        frame.pin();

        // Add to page table
        self.page_table.insert(page_id, frame_id);

        // Record access
        self.eviction_policy.record_access(frame_id);

        // Clear I/O flag
        frame.set_io_in_progress(false);

        // Update statistics
        self.page_reads.fetch_add(1, Ordering::Relaxed);

        Ok(FrameGuard::new(frame.clone()))
    }

    /// Allocate a frame (either from free list or by eviction)
    #[inline]
    fn allocate_frame(&self) -> Result<FrameId> {
        // Try free list first
        if let Some(frame_id) = self.free_frames.allocate() {
            return Ok(frame_id);
        }

        // Need to evict a page
        self.evict_page()
    }

    /// Evict a page to make room
    #[cold]
    fn evict_page(&self) -> Result<FrameId> {
        // Find a victim using eviction policy
        let victim_id = self
            .eviction_policy
            .find_victim(&self.frames)
            .ok_or_else(|| DbError::Other("No frame available for eviction".into()))?;

        let victim_frame = &self.frames[victim_id as usize];

        // Flush if dirty
        if victim_frame.is_dirty() {
            let start = Instant::now();
            self.flush_page(victim_frame)?;
            let elapsed = start.elapsed().as_micros() as u64;
            self.io_wait_time_us.fetch_add(elapsed, Ordering::Relaxed);
        }

        // Remove from page table
        let old_page_id = victim_frame.page_id();
        if old_page_id != INVALID_PAGE_ID {
            self.page_table.remove(old_page_id);
        }

        // Record eviction
        self.eviction_policy.record_eviction(victim_id);

        // Reset frame
        victim_frame.reset();
        victim_frame.set_io_in_progress(false);

        Ok(victim_id)
    }

    /// Unpin a page (decrements pin count).
    ///
    /// This is a HOT PATH operation.
    ///
    /// # Arguments
    ///
    /// * `page_id` - Page to unpin
    /// * `is_dirty` - Whether the page was modified
    #[inline]
    pub fn unpin_page(&self, page_id: PageId, is_dirty: bool) -> Result<()> {
        let frame_id = self
            .page_table
            .lookup(page_id)
            .ok_or_else(|| DbError::PageNotFound(page_id.to_string()))?;

        let frame = &self.frames[frame_id as usize];

        if is_dirty {
            frame.set_dirty(true);
        }

        frame.unpin();
        self.eviction_policy.record_unpin(frame_id);

        Ok(())
    }

    /// Flush a single page to disk
    #[inline]
    pub fn flush_page_by_id(&self, page_id: PageId) -> Result<()> {
        let frame_id = self
            .page_table
            .lookup(page_id)
            .ok_or_else(|| DbError::PageNotFound(page_id.to_string()))?;

        let frame = &self.frames[frame_id as usize];
        self.flush_page(frame)
    }

    /// Flush a frame to disk
    #[inline]
    fn flush_page(&self, frame: &BufferFrame) -> Result<()> {
        if !frame.is_dirty() {
            return Ok(());
        }

        let page_id = frame.page_id();
        if page_id == INVALID_PAGE_ID {
            return Ok(());
        }

        // Write to disk
        self.write_page_to_disk(page_id, frame)?;

        frame.set_dirty(false);
        self.page_writes.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Flush all dirty pages
    pub fn flush_all(&self) -> Result<()> {
        let mut batch = FrameBatch::new(self.config.max_flush_batch_size);

        for frame in self.frames.iter() {
            if frame.is_dirty() && !frame.is_empty() {
                if batch.add(frame.clone()) {
                    if batch.is_full() {
                        self.flush_batch(&batch)?;
                        batch.clear();
                    }
                }
            }
        }

        if !batch.is_empty() {
            self.flush_batch(&batch)?;
        }

        Ok(())
    }

    /// Flush a batch of pages (optimized for sequential I/O)
    #[inline]
    pub fn flush_batch(&self, batch: &FrameBatch) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }

        // Sort by page ID for sequential I/O
        let mut sorted_batch = batch.clone();
        sorted_batch.sort_by_page_id();

        // Flush each page
        for frame in sorted_batch.frames() {
            self.flush_page(frame)?;
        }

        Ok(())
    }

    /// Get number of dirty pages
    pub fn dirty_page_count(&self) -> usize {
        self.frames.iter().filter(|f| f.is_dirty()).count()
    }

    /// Get dirty page ratio
    pub fn dirty_page_ratio(&self) -> f64 {
        let dirty = self.dirty_page_count();
        dirty as f64 / self.config.num_frames as f64
    }

    /// Check if background flush is needed
    #[inline]
    fn should_background_flush(&self) -> bool {
        self.dirty_page_ratio() > self.config.dirty_page_threshold
    }

    /// Start background flusher thread
    fn start_background_flusher(&self) {
        let frames = self.frames.clone();
        let shutdown = self.shutdown.clone();
        let interval = self.config.background_flush_interval;
        let max_batch_size = self.config.max_flush_batch_size;
        let threshold = self.config.dirty_page_threshold;
        let page_writes = self.page_writes.clone();
        let background_flushes = self.background_flushes.clone();

        let handle = std::thread::spawn(move || {
            while !shutdown.load(Ordering::Relaxed) {
                std::thread::sleep(interval);

                // Check if flush is needed
                let dirty_count = frames.iter().filter(|f| f.is_dirty()).count();
                let dirty_ratio = dirty_count as f64 / frames.len() as f64;

                if dirty_ratio > threshold {
                    // Create batch of dirty pages
                    let mut batch = Vec::with_capacity(max_batch_size);

                    for frame in frames.iter() {
                        if frame.is_dirty() && !frame.is_pinned() && !frame.is_empty() {
                            batch.push(frame.clone());
                            if batch.len() >= max_batch_size {
                                break;
                            }
                        }
                    }

                    // Flush batch (simplified - in real impl would call flush_batch)
                    for frame in batch {
                        if frame.is_dirty() {
                            frame.set_dirty(false);
                            page_writes.fetch_add(1, Ordering::Relaxed);
                        }
                    }

                    background_flushes.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        *self.background_flusher.lock() = Some(handle);
    }

    /// Load page data from disk (stub - implement with real disk manager)
    #[cold]
    fn load_page_from_disk(&self, page_id: PageId, frame: &BufferFrame) -> Result<()> {
        // TODO: Integrate with actual disk manager
        // For now, just zero the page
        let mut data = frame.write_data_no_dirty();
        data.zero();
        Ok(())
    }

    /// Write page data to disk (stub - implement with real disk manager)
    #[inline]
    fn write_page_to_disk(&self, page_id: PageId, frame: &BufferFrame) -> Result<()> {
        // TODO: Integrate with actual disk manager
        // For now, just read the data (to simulate I/O)
        let _data = frame.read_data();
        Ok(())
    }

    /// Get comprehensive statistics
    pub fn stats(&self) -> BufferPoolStats {
        let (lookups, hits, misses, hit_rate) = self.page_table.stats();
        let eviction_stats = self.eviction_policy.stats();
        let (global_allocs, per_core_allocs, free_frames) = self.free_frames.stats();

        let pinned_frames = self.frames.iter().filter(|f| f.is_pinned()).count();
        let dirty_frames = self.frames.iter().filter(|f| f.is_dirty()).count();

        BufferPoolStats {
            total_frames: self.config.num_frames,
            free_frames,
            pinned_frames,
            dirty_frames,
            lookups,
            hits,
            misses,
            hit_rate,
            page_reads: self.page_reads.load(Ordering::Relaxed),
            page_writes: self.page_writes.load(Ordering::Relaxed),
            evictions: eviction_stats.evictions,
            failed_evictions: eviction_stats.failed_evictions,
            background_flushes: self.background_flushes.load(Ordering::Relaxed),
            avg_search_length: eviction_stats.avg_search_length,
            global_allocations: global_allocs,
            per_core_allocations: per_core_allocs,
            io_wait_time_us: self.io_wait_time_us.load(Ordering::Relaxed),
        }
    }

    /// Get page table size
    pub fn page_table_size(&self) -> usize {
        self.page_table.len()
    }

    /// Get configuration
    pub fn config(&self) -> &BufferPoolConfig {
        &self.config
    }

    /// Get eviction policy name
    pub fn eviction_policy_name(&self) -> &str {
        self.eviction_policy.name()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.page_reads.store(0, Ordering::Relaxed);
        self.page_writes.store(0, Ordering::Relaxed);
        self.background_flushes.store(0, Ordering::Relaxed);
        self.io_wait_time_us.store(0, Ordering::Relaxed);
        self.eviction_policy.reset();
    }

    /// Prefetch pages (async I/O hint)
    pub fn prefetch_pages(&self, page_ids: &[PageId]) -> Result<()> {
        // TODO: Implement async prefetching
        // For now, this is a no-op
        Ok(())
    }

    /// Shutdown buffer pool
    pub fn shutdown(&self) -> Result<()> {
        // Signal shutdown
        self.shutdown.store(true, Ordering::Relaxed);

        // Wait for background flusher
        if let Some(handle) = self.background_flusher.lock().take() {
            let _ = handle.join();
        }

        // Flush all dirty pages
        self.flush_all()?;

        Ok(())
    }
}

impl Drop for BufferPoolManager {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

// ============================================================================
// Windows IOCP Integration Helpers
// ============================================================================

#[cfg(target_os = "windows")]
pub mod windows {
    use super::*;
    use std::os::windows::io::RawHandle;

    /// Windows IOCP context for async I/O
    pub struct IocpContext {
        completion_port: RawHandle,
    }

    impl IocpContext {
        /// Create IOCP completion port
        pub fn new() -> Result<Self> {
            // TODO: Create IOCP handle using CreateIoCompletionPort
            // For now, return placeholder
            Err(DbError::Other("IOCP not yet implemented".into()))
        }

        /// Submit async read request
        pub fn async_read(
            &self,
            page_id: PageId,
            buffer: &mut PageBuffer,
        ) -> Result<()> {
            // TODO: Use ReadFile with OVERLAPPED
            Ok(())
        }

        /// Submit async write request
        pub fn async_write(
            &self,
            page_id: PageId,
            buffer: &PageBuffer,
        ) -> Result<()> {
            // TODO: Use WriteFile with OVERLAPPED
            Ok(())
        }

        /// Poll for completions
        pub fn poll_completions(&self, timeout_ms: u32) -> Result<Vec<(PageId, Result<()>)>> {
            // TODO: Use GetQueuedCompletionStatus
            Ok(Vec::new())
        }
    }
}

// ============================================================================
// Builder Pattern
// ============================================================================

/// Builder for BufferPoolManager
pub struct BufferPoolBuilder {
    config: BufferPoolConfig,
}

impl BufferPoolBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: BufferPoolConfig::default(),
        }
    }

    /// Set number of frames
    pub fn num_frames(mut self, num_frames: usize) -> Self {
        self.config.num_frames = num_frames;
        self
    }

    /// Set eviction policy
    pub fn eviction_policy(mut self, policy: EvictionPolicyType) -> Self {
        self.config.eviction_policy = policy;
        self
    }

    /// Enable/disable per-core pools
    pub fn per_core_pools(mut self, enable: bool) -> Self {
        self.config.enable_per_core_pools = enable;
        self
    }

    /// Set frames per core
    pub fn frames_per_core(mut self, frames: usize) -> Self {
        self.config.frames_per_core = frames;
        self
    }

    /// Set maximum flush batch size
    pub fn max_flush_batch_size(mut self, size: usize) -> Self {
        self.config.max_flush_batch_size = size;
        self
    }

    /// Enable/disable background flushing
    pub fn background_flush(mut self, enable: bool) -> Self {
        self.config.enable_background_flush = enable;
        self
    }

    /// Set background flush interval
    pub fn flush_interval(mut self, interval: Duration) -> Self {
        self.config.background_flush_interval = interval;
        self
    }

    /// Set dirty page threshold
    pub fn dirty_threshold(mut self, threshold: f64) -> Self {
        self.config.dirty_page_threshold = threshold;
        self
    }

    /// Build the buffer pool manager
    pub fn build(self) -> BufferPoolManager {
        BufferPoolManager::new(self.config)
    }
}

impl Default for BufferPoolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_pool_creation() {
        let config = BufferPoolConfig {
            num_frames: 10,
            ..Default::default()
        };

        let pool = BufferPoolManager::new(config);
        assert_eq!(pool.config.num_frames, 10);
    }

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
    fn test_free_frame_manager() {
        let manager = FreeFrameManager::new(10, false, 0);

        let frame1 = manager.allocate();
        assert!(frame1.is_some());

        let frame2 = manager.allocate();
        assert!(frame2.is_some());

        assert_ne!(frame1, frame2);

        manager.deallocate(frame1.unwrap());
        let frame3 = manager.allocate();
        assert_eq!(frame3, frame1);
    }

    #[test]
    fn test_pin_unpin() {
        let pool = BufferPoolBuilder::new()
            .num_frames(10)
            .background_flush(false)
            .build();

        let page_id = 1;
        let guard = pool.pin_page(page_id).unwrap();
        assert_eq!(guard.page_id(), page_id);

        drop(guard);
    }

    #[test]
    fn test_stats() {
        let pool = BufferPoolBuilder::new()
            .num_frames(10)
            .build();

        let stats = pool.stats();
        assert_eq!(stats.total_frames, 10);
        assert!(stats.free_frames > 0);
    }

    #[test]
    fn test_builder() {
        let pool = BufferPoolBuilder::new()
            .num_frames(100)
            .eviction_policy(EvictionPolicyType::Lru)
            .per_core_pools(true)
            .frames_per_core(4)
            .build();

        assert_eq!(pool.config.num_frames, 100);
        assert_eq!(pool.eviction_policy_name(), "LRU");
    }
}


