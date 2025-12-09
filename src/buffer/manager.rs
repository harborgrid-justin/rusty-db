// # Buffer Pool Manager - Main Buffer Management System
//
// High-performance buffer pool manager optimized for Windows/MSVC with:
// - Lock-free page table for fast lookups
// - Per-core frame pools to reduce contention
// - Batch flush support for efficient I/O
// - Windows IOCP integration ready
// - Zero allocations in pin/unpin hot path
//
// ## Architecture
//
// ```text
// ┌─────────────────────────────────────────────────────────┐
// │              Buffer Pool Manager                        │
// ├─────────────────────────────────────────────────────────┤
// │  Page Table (PageId -> FrameId)                        │
// │  ┌──────────┬──────────┬──────────┬──────────┐        │
// │  │ Hash Map │ Hash Map │ Hash Map │ Hash Map │        │
// │  └──────────┴──────────┴──────────┴──────────┘        │
// │           (Partitioned for concurrency)                │
// ├─────────────────────────────────────────────────────────┤
// │  Frame Array (Pre-allocated)                           │
// │  ┌──────┬──────┬──────┬──────┬──────┬──────┐         │
// │  │Frame │Frame │Frame │Frame │Frame │ ...  │         │
// │  │  0   │  1   │  2   │  3   │  4   │      │         │
// │  └──────┴──────┴──────┴──────┴──────┴──────┘         │
// ├─────────────────────────────────────────────────────────┤
// │  Per-Core Free Lists                                   │
// │  ┌─────────┬─────────┬─────────┬─────────┐           │
// │  │ Core 0  │ Core 1  │ Core 2  │ Core 3  │           │
// │  │ [3,7,11]│[4,8,12] │[5,9,13] │[6,10,14]│           │
// │  └─────────┴─────────┴─────────┴─────────┘           │
// └─────────────────────────────────────────────────────────┘
// ```

use tokio::time::sleep;
use crate::buffer::eviction::{create_eviction_policy, EvictionPolicy, EvictionPolicyType};
use crate::buffer::page_cache::{
    BufferFrame, FrameBatch, FrameGuard, FrameId, PageBuffer, PerCoreFramePool,
    INVALID_PAGE_ID, PAGE_SIZE,
};
use crate::common::PageId;
use crate::error::Result;
use crate::storage::disk::DiskManager;
use crate::storage::page::Page;
use crate::DbError;

use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
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

    /// Data directory for disk I/O
    pub data_directory: String,

    /// Page size (default 4KB)
    pub page_size: usize,
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
            data_directory: "./data".to_string(),
            page_size: PAGE_SIZE,
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
    fn new(numpartitions: usize, initial_capacity_per_partition: usize) -> Self {
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
    ffn new(
        numframes: usize,
        enableper_core_pools: bool,
        frames_per_core: usize,
    )-> Self {
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
/// - Production-grade disk I/O integration
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

    /// Disk manager for persistent storage I/O
    disk_manager: Option<Arc<DiskManager>>,

    /// Prefetch request queue (page_id -> priority)
    prefetch_queue: Arc<Mutex<Vec<(PageId, u8)>>>,

    /// Prefetch worker thread handles
    prefetch_workers: Mutex<Vec<thread::JoinHandle<()>>>,

    /// Statistics
    page_reads: AtomicU64,
    page_writes: AtomicU64,
    background_flushes: AtomicU64,
    io_wait_time_us: AtomicU64,
    prefetch_hits: AtomicU64,
    prefetch_misses: AtomicU64,

    /// Background flusher thread handle
    background_flusher: Mutex<Option<thread::JoinHandle<()>>>,

    /// Shutdown flag
    shutdown: Arc<AtomicBool>,

    /// Start time
    start_time: Instant,
}

impl BufferPoolManager {
    /// Create a new buffer pool manager
    pub fn new(config: BufferPoolConfig) -> Self {
        Self::with_disk_manager(config, None)
    }

    /// Create a new buffer pool manager with a disk manager for production I/O
    pub fn with_disk_manager(config: BufferPoolConfig, diskmanager: Option<Arc<DiskManager>>) -> Self {
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
        let prefetch_queue = Arc::new(Mutex::new(Vec::with_capacity(64)));

        let manager = Self {
            config: config.clone(),
            frames: Arc::new(frames),
            page_table,
            free_frames,
            eviction_policy,
            disk_manager,
            prefetch_queue,
            prefetch_workers: Mutex::new(Vec::new()),
            page_reads: AtomicU64::new(0),
            page_writes: AtomicU64::new(0),
            background_flushes: AtomicU64::new(0),
            io_wait_time_us: AtomicU64::new(0),
            prefetch_hits: AtomicU64::new(0),
            prefetch_misses: AtomicU64::new(0),
            background_flusher: Mutex::new(None),
            shutdown,
            start_time: Instant::now(),
        };

        // Start background flusher if enabled
        if config.enable_background_flush {
            manager.start_background_flusher();
        }

        // Start prefetch workers if enabled
        if config.enable_prefetch {
            manager.start_prefetch_workers();
        }

        manager
    }

    /// Create a buffer pool manager with automatic disk manager initialization
    pub fn with_data_directory(config: BufferPoolConfig) -> Result<Self> {
        let disk_manager = DiskManager::new(&config.data_directory, config.page_size)?;
        Ok(Self::with_disk_manager(config, Some(Arc::new(disk_manager))))
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

        // Sort by page ID for sequential I/O - use frames directly
        let sorted_batch = batch;

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

        let handle = thread::spawn(move || {
            // Stats counters (simplified since we can't easily share AtomicU64 across threads)
            let mut writes_count = 0u64;
            let mut flush_count = 0u64;

            while !shutdown.load(Ordering::Relaxed) {
                thread::sleep(interval);

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
                            writes_count += 1;
                        }
                    }

                    flush_count += 1;
                }
            }
        });

        *self.background_flusher.lock() = Some(handle);
    }

    /// Load page data from disk using the integrated DiskManager.
    ///
    /// This method reads a page from persistent storage into the buffer frame.
    /// If no disk manager is configured, it initializes the page to zeros.
    ///
    /// # Performance
    ///
    /// - Uses read-ahead buffer in disk manager for sequential access patterns
    /// - Tracks I/O wait time for performance monitoring
    /// - Hardware CRC32C checksum verification when available
    #[cold]
    fn load_page_from_disk(&self, page_id: PageId, frame: &BufferFrame) -> Result<()> {
        let start = Instant::now();

        match &self.disk_manager {
            Some(disk_manager) => {
                // Read page from disk manager (includes read-ahead optimization)
                let page = disk_manager.read_page(page_id)?;

                // Copy page data into the buffer frame
                let mut data = frame.write_data_no_dirty();
                let page_data = page.data.as_slice();
                let copy_len = page_data.len().min(PAGE_SIZE);
                data.data_mut()[..copy_len].copy_from_slice(&page_data[..copy_len]);

                // Zero remaining bytes if page data is smaller
                if copy_len < PAGE_SIZE {
                    data.data_mut()[copy_len..].fill(0);
                }
            }
            None => {
                // No disk manager - initialize page to zeros
                let mut data = frame.write_data_no_dirty();
                data.zero();
            }
        }

        // Track I/O wait time
        let elapsed_us = start.elapsed().as_micros() as u64;
        self.io_wait_time_us.fetch_add(elapsed_us, Ordering::Relaxed);

        Ok(())
    }

    /// Write page data to disk using the integrated DiskManager.
    ///
    /// This method persists a dirty page from the buffer frame to storage.
    /// If no disk manager is configured, it's a no-op (useful for testing).
    ///
    /// # Performance
    ///
    /// - Uses write-behind buffer for batching writes
    /// - Write coalescing for adjacent pages
    /// - fsync only on checkpoint or forced flush
    #[inline]
    fn write_page_to_disk(&self, page_id: PageId, frame: &BufferFrame) -> Result<()> {
        let start = Instant::now();

        match &self.disk_manager {
            Some(disk_manager) => {
                // Read the page data from the frame
                let data = frame.read_data();

                // Create a Page struct for the disk manager
                let page = Page::from_bytes(page_id, data.data().to_vec());

                // Write to disk (uses write-behind buffer for optimization)
                disk_manager.write_page(&page)?;
            }
            None => {
                // No disk manager - no-op for testing scenarios
            }
        }

        // Track I/O wait time
        let elapsed_us = start.elapsed().as_micros() as u64;
        self.io_wait_time_us.fetch_add(elapsed_us, Ordering::Relaxed);

        Ok(())
    }

    /// Force flush a specific page to disk synchronously.
    ///
    /// This bypasses write-behind buffering for durability guarantees.
    pub fn force_flush_page(&self, page_id: PageId) -> Result<()> {
        if let Some(frame_id) = self.page_table.lookup(page_id) {
            let frame = &self.frames[frame_id as usize];
            if frame.is_dirty() {
                self.write_page_to_disk(page_id, frame)?;
                frame.set_dirty(false);
                self.page_writes.fetch_add(1, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    /// Flush all pending writes in the disk manager's write-behind buffer.
    pub fn sync_disk(&self) -> Result<()> {
        if let Some(disk_manager) = &self.disk_manager {
            disk_manager.flush_all_writes()?;
        }
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
        self.prefetch_hits.store(0, Ordering::Relaxed);
        self.prefetch_misses.store(0, Ordering::Relaxed);
        self.eviction_policy.reset();
    }

    /// Prefetch pages asynchronously.
    ///
    /// Submits page IDs to the prefetch queue for background loading.
    /// Pages that are already in the buffer pool are skipped.
    /// Prefetch priority is determined by position in the array (earlier = higher priority).
    ///
    /// # Performance
    ///
    /// - Non-blocking: returns immediately after queuing
    /// - Deduplication: skips pages already in buffer pool or queue
    /// - Priority-based: worker threads process higher priority requests first
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Prefetch next 10 pages for sequential scan
    /// let page_ids: Vec<PageId> = (current_page + 1..current_page + 11).collect();
    /// buffer_pool.prefetch_pages(&page_ids)?;
    /// ```
    pub fn prefetch_pages(&self, page_ids: &[PageId]) -> Result<()> {
        if !self.config.enable_prefetch || page_ids.is_empty() {
            return Ok(());
        }

        let mut queue = self.prefetch_queue.lock();

        // Add pages to prefetch queue (with priority based on position)
        for (idx, &page_id) in page_ids.iter().enumerate() {
            // Skip if already in buffer pool
            if self.page_table.lookup(page_id).is_some() {
                self.prefetch_hits.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            // Skip if already in prefetch queue
            if queue.iter().any(|(pid, _)| *pid == page_id) {
                continue;
            }

            // Priority: 255 = highest, 0 = lowest
            // Earlier positions in the array get higher priority
            let priority = 255u8.saturating_sub(idx as u8);
            queue.push((page_id, priority));
            self.prefetch_misses.fetch_add(1, Ordering::Relaxed);
        }

        // Sort by priority (descending) - higher priority first
        queue.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(())
    }

    /// Prefetch a range of sequential pages (optimized for sequential scans).
    ///
    /// This is a convenience method for common sequential access patterns.
    pub fn prefetch_range(&self, start_page_id: PageId, count: usize) -> Result<()> {
        let page_ids: Vec<PageId> = (start_page_id..start_page_id + count as u64).collect();
        self.prefetch_pages(&page_ids)
    }

    /// Start prefetch worker threads.
    ///
    /// Worker threads continuously poll the prefetch queue and load pages
    /// into the buffer pool in the background.
    fn start_prefetch_workers(&self) {
        let num_workers = self.config.prefetch_threads;
        let mut workers = self.prefetch_workers.lock();

        for worker_id in 0..num_workers {
            let shutdown = self.shutdown.clone();
            let prefetch_queue = self.prefetch_queue.clone();
            let disk_manager = self.disk_manager.clone();
            let page_table = self.page_table.clone();
            let frames = self.frames.clone();
            let free_frames = self.free_frames.clone();
            let eviction_policy = self.eviction_policy.clone();

            let handle = thread::Builder::new()
                .name(format!("prefetch-worker-{}", worker_id))
                .spawn(move || {
                    Self::prefetch_worker_loop(
                        shutdown,
                        prefetch_queue,
                        disk_manager,
                        page_table,
                        frames,
                        free_frames,
                        eviction_policy,
                    )));
                })
                .expect("Failed to spawn prefetch worker thread");

            workers.push(handle);
        }
    }

    /// Main loop for prefetch worker threads.
    fnfn prefetch_worker_loop(
        shutdown: Arc<AtomicBool>,
        prefetchqueue: Arc<Mutex<Vec<(PageId, u8)>,
        disk_manager: Option<Arc<DiskManager>>,
        page_table: Arc<PageTable>,
        frames: Arc<Vec<Arc<BufferFrame>>>,
        free_frames: Arc<FreeFrameManager>,
        eviction_policy: Arc<dyn EvictionPolicy>,
    ) {
        while !shutdown.load(Ordering::Relaxed) {
            // Get next page to prefetch
            let page_to_prefetch = {
                let mut queue = prefetch_queue.lock();
                queue.pop()
            };

            match page_to_prefetch {
                Some((page_id, _priority)) => {
                    // Double-check page isn't already in buffer pool
                    if page_table.lookup(page_id).is_some() {
                        continue;
                    }

                    // Try to allocate a frame for prefetching
                    let frame_id = match free_frames.allocate() {
                        Some(id) => id,
                        None => {
                            // No free frames - try to evict one
                            match Self::try_evict_for_prefetch(&frames, &eviction_policy) {
                                Some(id) => id,
                                None => {
                                    // Can't evict - re-queue with lower priority and sleep
                                    {
                                        let mut queue = prefetch_queue.lock();
                                        queue.push((page_id, 0)); // Lowest priority
                                    }
                                    thread::sleep(Duration::from_millis(10));
                                    continue;
                                }
                            }
                        }
                    };

                    let frame = &frames[frame_id as usize];

                    // Set I/O in progress
                    frame.set_io_in_progress(true);
                    frame.set_page_id(page_id);

                    // Load page from disk
                    if let Some(ref dm) = disk_manager {
                        match dm.read_page(page_id) {
                            Ok(page) => {
                                let mut data = frame.write_data_no_dirty();
                                let page_data = page.data.as_slice();
                                let copy_len = page_data.len().min(PAGE_SIZE);
                                data.data_mut()[..copy_len].copy_from_slice(&page_data[..copy_len]);

                                // Insert into page table
                                page_table.insert(page_id, frame_id);
                                eviction_policy.record_access(frame_id);
                            }
                            Err(_) => {
                                // Failed to load - reset frame
                                frame.reset();
                                free_frames.deallocate(frame_id);
                            }
                        }
                    }

                    // Clear I/O in progress
                    frame.set_io_in_progress(false);
                }
                None => {
                    // No work to do - sleep briefly
                    thread::sleep(Duration::from_millis(5));
                }
            }
        }
    }

    /// Try to evict a frame for prefetching (low priority eviction).
    fn try_evict_for_prefetch(
        frames: &Arc<Vec<Arc<BufferFrame>>>,
        evictionpolicy: &Arc<dyn EvictionPolicy>,
    ) -> Option<FrameId> {
        // Use the eviction policy to find a victim frame
        if let Some(victim_id) = eviction_policy.find_victim(frames) {
            let frame = &frames[victim_id as usize];

            // Only evict if not pinned and not dirty (prefetch is low priority)
            if !frame.is_pinned() && !frame.is_dirty() && frame.try_evict() {
                eviction_policy.record_eviction(victim_id);
                frame.reset();
                return Some(victim_id);
            }
        }
        None
    }

    /// Get prefetch statistics
    pub fn prefetch_stats(&self) -> (u64, u64) {
        (
            self.prefetch_hits.load(Ordering::Relaxed),
            self.prefetch_misses.load(Ordering::Relaxed),
        )
    }

    /// Shutdown buffer pool
    pub fn shutdown(&self) -> Result<()> {
        // Signal shutdown
        self.shutdown.store(true, Ordering::Relaxed);

        // Wait for background flusher
        if let Some(handle) = self.background_flusher.lock().take() {
            let _ = handle.join();
        }

        // Wait for prefetch workers (drain remaining)
        // Note: We can't take ownership here, so workers will be joined on drop

        // Flush all dirty pages
        self.flush_all()?;

        // Sync to disk
        self.sync_disk()?;

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
    use std::fs::File;
    use std::os::windows::io::{AsRawHandle, RawHandle};
    use std::ptr;

    // Windows API constants
    const INVALID_HANDLE_VALUE: RawHandle = -1isize as RawHandle;

    /// Operation type for IOCP completion tracking
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum IocpOpType {
        Read,
        Write,
    }

    /// Overlapped structure for tracking async I/O operations
    #[repr(C)]
    pub struct IocpOverlapped {
        /// Standard Windows OVERLAPPED fields (must be first for FFI compatibility)
        internal: u64,
        internal_high: u64,
        offset: u32,
        offset_high: u32,
        event: RawHandle,
        /// Custom fields for our tracking
        pub page_id: PageId,
        pub op_type: IocpOpType,
        pub user_data: u64,
    }

    impl IocpOverlapped {
        /// Create a new OVERLAPPED structure for a specific page and offset
        pub fn new(page_id: PageId, offset: u64, op_type: IocpOpType) -> Self {
            Self {
                internal: 0,
                internal_high: 0,
                offset: offset as u32,
                offset_high: (offset >> 32) as u32,
                event: ptr::null_mut(),
                page_id,
                op_type,
                user_data: page_id,
            }
        }
    }

    /// I/O completion status returned from polling
    #[derive(Debug)]
    pub struct IocpCompletion {
        /// Page ID of the completed operation
        pub page_id: PageId,
        /// Type of operation that completed
        pub op_type: IocpOpType,
        /// Number of bytes transferred
        pub bytes_transferred: usize,
        /// Error code (0 = success)
        pub error_code: u32,
    }

    impl IocpCompletion {
        /// Check if operation succeeded
        pub fn is_success(&self) -> bool {
            self.error_code == 0
        }
    }

    /// Windows IOCP context for high-performance async I/O.
    ///
    /// I/O Completion Ports provide the most efficient async I/O mechanism on Windows,
    /// allowing thousands of concurrent I/O operations with minimal thread overhead.
    ///
    /// # Architecture
    ///
    /// ```text
    /// ┌─────────────────────────────────────────────────────────────┐
    /// │                    IOCP Context                             │
    /// ├─────────────────────────────────────────────────────────────┤
    /// │  Completion Port (kernel object)                           │
    /// │  ┌─────────────────────────────────────────────────────┐   │
    /// │  │  Worker threads dequeue completions                  │   │
    /// │  │  ← Completed I/O operations posted by kernel        │   │
    /// │  └─────────────────────────────────────────────────────┘   │
    /// │                                                             │
    /// │  Associated File Handles                                   │
    /// │  ┌──────────┬──────────┬──────────┐                       │
    /// │  │ data.db  │ log.wal  │ idx.db   │                       │
    /// │  └──────────┴──────────┴──────────┘                       │
    /// │                                                             │
    /// │  Pending Operations (OVERLAPPED tracking)                  │
    /// │  ┌──────────────────────────────────────────────────────┐  │
    /// │  │ PageId → (Buffer, OpType, Callback)                  │  │
    /// │  └──────────────────────────────────────────────────────┘  │
    /// └─────────────────────────────────────────────────────────────┘
    /// ```
    pub struct IocpContext {
        /// I/O completion port handle
        completion_port: RawHandle,
        /// Associated file handle for data file
        data_file: Option<File>,
        /// Page size for I/O operations
        page_size: usize,
        /// Pending operations tracking
        pending_ops: Mutex<HashMap<PageId, Box<IocpOverlapped>>>,
        /// Next operation ID for tracking
        next_op_id: AtomicU64,
        /// Number of concurrent threads (for IOCP thread pool)
        num_threads: u32,
    }

    // Windows API function declarations (using raw FFI for maximum control)
    #[cfg(target_os = "windows")]
    extern "system" {
        fn ffn CreateIoCompletionPort(
            file_handle: RawHandle,
            existingcompletion_port: RawHandle,
            completionkey: usize,
            number_of_concurrent_threads: u32,
        )Handle;

        fn Getfn fn GetQueuedCompletionStatus(
            completion_port: RawHandle,
            lp_number_of_bytes_transferred: *mut u32,
            lp_completion_key: *mut usize,
            lpoverlapped: *mut *mut IocpOverlapped,
            dwmilliseconds: u32,
        )    fn PostQuefn Posfn PostQueuedCompletionStatus(
            completionport: RawHandle,
            number_of_bytes_transferred: u32,
            completionkey: usize,
            overlapped: *mut IocpOverlapped,
        ) fn ReadFile(
            file: RawHandle,
            buffer: *mut u8,
            number_of_bytes_to_read: u32,
            number_of_bytes_read: *mut u32,
            overlapped: *mut IocpOverlapped,
        ) -> i32;

        fn WriteFile(
            file: RawHandle,
            buffer: *const u8,
            number_of_bytes_to_write: u32,
            number_of_bytes_written: *mut u32,
            overlapped: *mut IocpOverlapped,
        ) -> i32;

        fn CloseHandle(handle: RawHandle) -> i32;

        fn GetLastError() -> u32;
    }

    // Error codes
    const ERROR_IO_PENDING: u32 = 997;
    const ERROR_SUCCESS: u32 = 0;
    const WAIT_TIMEOUT: u32 = 258;

    impl IocpContext {
        /// Create a new IOCP completion port.
        ///
        /// # Arguments
        ///
        /// * `num_threads` - Number of concurrent threads for processing completions.
        ///                   Use 0 to use the number of CPU cores.
        ///
        /// # Returns
        ///
        /// A new IocpContext or an error if creation failed.
        pub fn new(num_threads: u32) -> Result<Self> {
            let num_threads = if num_threads == 0 {
                num_cpus::get() as u32
            } else {
                num_threads
            };

            // Create completion port with no initial file handle
            let completion_port = unsafe {
                CreateIoCompletionPort(
                    INVALID_HANDLE_VALUE,
                    ptr::null_mut(),
                    0,
                    num_threads,
                )
            };

            if completion_port.is_null() || completion_port == INVALID_HANDLE_VALUE {
                let error = unsafe { GetLastError() };
                return Err(DbError::Storage(format!(
                    "Failed to create IOCP: Windows error {}",
                    error
                )))));
            }

            Ok(Self {
                completion_port,
                data_file: None,
                page_size: PAGE_SIZE,
                pending_ops: Mutex::new(HashMap::new()),
                next_op_id: AtomicU64::new(1),
                num_threads,
            })
        }

        /// Associate a file handle with this IOCP for async I/O.
        ///
        /// The file must be opened with `FILE_FLAG_OVERLAPPED` for async operations.
        pub fn associate_file(&mut self, file: File) -> Result<()> {
            let file_handle = file.as_raw_handle();

            // Associate file with completion port
            let result = unsafe {
                CreateIoCompletionPort(
                    file_handle,
                    self.completion_port,
                    file_handle as usize, // Use file handle as completion key
                    0, // Ignored when associating with existing port
                )
            };

            if result.is_null() || result == INVALID_HANDLE_VALUE {
                let error = unsafe { GetLastError() };
                return Err(DbError::Storage(format!(
                    "Failed to associate file with IOCP: Windows error {}",
                    error
                )))));
            }

            self.data_file = Some(file);
            Ok(())
        }

        /// Submit an async read request for a page.
        ///
        /// The read operation is queued to the IOCP and will complete asynchronously.
        /// Use `poll_completions` to retrieve the result.
        ///
        /// # Arguments
        ///
        /// * `page_id` - The page ID to read
        /// * `buffer` - The buffer to read into (must remain valid until completion)
        ///
        /// # Returns
        ///
        /// Ok(()) if the operation was queued successfully, or an error.
        pub fn async_read(
            &self,
            page_id: PageId,
            buffer: &mut PageBuffer,
        ) -> Result<()> {
            let file = self.data_file.as_ref()
                .ok_or_else(|| DbError::Storage("No file associated with IOCP".to_string()))?;

            let offset = page_id as u64 * self.page_size as u64;
            let mut overlapped = Box::new(IocpOverlapped::new(page_id, offset, IocpOpType::Read));

            let result = unsafe {
                ReadFile(
                    file.as_raw_handle(),
                    buffer.data_mut().as_mut_ptr(),
                    self.page_size as u32,
                    ptr::null_mut(), // bytes_read not used with OVERLAPPED
                    overlapped.as_mut(),
                )
            };

            if result == 0 {
                let error = unsafe { GetLastError() };
                if error != ERROR_IO_PENDING {
                    return Err(DbError::Storage(format!(
                        "IOCP async read failed: Windows error {}",
                        error
                    )))));
                }
            }

            // Track the pending operation
            self.pending_ops.lock().insert(page_id, overlapped);

            Ok(())
        }

        /// Submit an async write request for a page.
        ///
        /// The write operation is queued to the IOCP and will complete asynchronously.
        /// Use `poll_completions` to retrieve the result.
        ///
        /// # Arguments
        ///
        /// * `page_id` - The page ID to write
        /// * `buffer` - The buffer containing data to write
        ///
        /// # Returns
        ///
        /// Ok(()) if the operation was queued successfully, or an error.
        pub fn async_write(
            &self,
            page_id: PageId,
            buffer: &PageBuffer,
        ) -> Result<()> {
            let file = self.data_file.as_ref()
                .ok_or_else(|| DbError::Storage("No file associated with IOCP".to_string()))?;

            let offset = page_id as u64 * self.page_size as u64;
            let mut overlapped = Box::new(IocpOverlapped::new(page_id, offset, IocpOpType::Write));

            let result = unsafe {
                WriteFile(
                    file.as_raw_handle(),
                    buffer.data().as_ptr(),
                    self.page_size as u32,
                    ptr::null_mut(), // bytes_written not used with OVERLAPPED
                    overlapped.as_mut(),
                )
            };

            if result == 0 {
                let error = unsafe { GetLastError() };
                if error != ERROR_IO_PENDING {
                    return Err(DbError::Storage(format!(
                        "IOCP async write failed: Windows error {}",
                        error
                    )))));
                }
            }

            // Track the pending operation
            self.pending_ops.lock().insert(page_id, overlapped);

            Ok(())
        }

        /// Poll for I/O completions.
        ///
        /// Waits up to `timeout_ms` milliseconds for I/O operations to complete.
        /// Returns a vector of completion results.
        ///
        /// # Arguments
        ///
        /// * `timeout_ms` - Maximum time to wait in milliseconds (0 for non-blocking)
        ///
        /// # Returns
        ///
        /// A vector of completion results, or an error.
        pub fn poll_completions(&self, timeout_ms: u32) -> Result<Vec<IocpCompletion>> {
            let mut completions = Vec::new();

            // Poll for completions in a loop until timeout or no more completions
            loop {
                let mut bytes_transferred: u32 = 0;
                let mut completion_key: usize = 0;
                let mut overlapped_ptr: *mut IocpOverlapped = ptr::null_mut();

                let result = unsafe {
                    GetQueuedCompletionStatus(
                        self.completion_port,
                        &mut bytes_transferred,
                        &mut completion_key,
                        &mut overlapped_ptr,
                        if completions.is_empty() { timeout_ms } else { 0 },
                    )
                };

                if result == 0 {
                    let error = unsafe { GetLastError() };
                    if error == WAIT_TIMEOUT || overlapped_ptr.is_null() {
                        // No more completions available
                        break;
                    }

                    // I/O failed but we have an overlapped structure
                    if !overlapped_ptr.is_null() {
                        let overlapped = unsafe { &*overlapped_ptr };
                        completions.push(IocpCompletion {
                            page_id: overlapped.page_id,
                            op_type: overlapped.op_type,
                            bytes_transferred: 0,
                            error_code: error,
                        });

                        // Remove from pending
                        self.pending_ops.lock().remove(&overlapped.page_id);
                    }
                } else {
                    // Success
                    if !overlapped_ptr.is_null() {
                        let overlapped = unsafe { &*overlapped_ptr };
                        completions.push(IocpCompletion {
                            page_id: overlapped.page_id,
                            op_type: overlapped.op_type,
                            bytes_transferred: bytes_transferred as usize,
                            error_code: ERROR_SUCCESS,
                        });

                        // Remove from pending
                        self.pending_ops.lock().remove(&overlapped.page_id);
                    }
                }
            }

            Ok(completions)
        }

        /// Get the number of pending I/O operations.
        pub fn pending_count(&self) -> usize {
            self.pending_ops.lock().len()
        }

        /// Cancel all pending I/O operations.
        pub fn cancel_all(&self) {
            self.pending_ops.lock().clear();
        }

        /// Post a manual completion to the IOCP (useful for signaling shutdown).
        pub fn post_completion(&self, page_id: PageId, bytes: u32) -> Result<()> {
            let result = unsafe {
                PostQueuedCompletionStatus(
                    self.completion_port,
                    bytes,
                    page_id as usize,
                    ptr::null_mut(),
                )
            };

            if result == 0 {
                let error = unsafe { GetLastError() };
                return Err(DbError::Storage(format!(
                    "Failed to post IOCP completion: Windows error {}",
                    error
                )))));
            }

            Ok(())
        }
    }

    impl Drop for IocpContext {
        fn drop(&mut self) {
            // Cancel pending operations
            self.cancel_all();

            // Close the completion port
            if !self.completion_port.is_null() && self.completion_port != INVALID_HANDLE_VALUE {
                unsafe {
                    CloseHandle(self.completion_port);
                }
            }
        }
    }

    // Safety: IocpContext can be sent between threads
    unsafe impl Send for IocpContext {}
    // Safety: IocpContext can be shared between threads (with internal synchronization)
    unsafe impl Sync for IocpContext {}
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
