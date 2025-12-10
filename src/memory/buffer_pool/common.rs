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

use serde::{Serialize, Deserialize};

// Re-export commonly used types for other modules
pub use std::collections::{HashMap, VecDeque, BTreeMap};
pub use std::sync::atomic::{AtomicU64, AtomicUsize, AtomicBool, Ordering};
pub use std::sync::Arc;
pub use std::time::{Duration, Instant};
pub use parking_lot::{Mutex, RwLock as PRwLock};

// ============================================================================
// SECTION 1: MULTI-TIER BUFFER POOL (700+ lines)
// ============================================================================

// Page identifier combining tablespace and page number
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

// Buffer tier classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BufferTier {
    // Hottest pages - frequently accessed, pinned in memory
    Hot,
    // Moderately accessed pages
    Warm,
    // Rarely accessed pages - candidates for eviction
    Cold,
}

// Buffer pool type configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolType {
    // Default buffer pool for general operations
    Default,
    // Keep pool for pinned pages that should not be evicted
    Keep,
    // Recycle pool for sequential scans
    Recycle,
    // Per-tablespace dedicated pool
    Tablespace(u32),
}

// Buffer frame containing page data and metadata
#[derive(Debug)]
pub struct BufferFrame {
    // Page identifier
    pub(crate) page_id: Option<PageId>,
    // Actual page data (typically 8KB, 16KB, or 32KB)
    data: Vec<u8>,
    // Pin count - number of active references
    pin_count: AtomicUsize,
    // Dirty flag - has been modified
    pub(crate) dirty: AtomicBool,
    // Access count for replacement policy
    access_count: AtomicU64,
    // Last access timestamp
    last_access: Mutex<Instant>,
    // Current tier assignment
    tier: Mutex<BufferTier>,
    // LSN (Log Sequence Number) of last modification
    lsn: AtomicU64,
    // Lock for page content modifications
    _page_lock: PRwLock<()>,
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
            _page_lock: PRwLock::new(()),
        }
    }

    // Pin the buffer frame (increment reference count)
    pub fn pin(&self) -> usize {
        let count = self.pin_count.fetch_add(1, Ordering::AcqRel) + 1;
        self.access_count.fetch_add(1, Ordering::Relaxed);
        *self.last_access.lock() = Instant::now();
        count
    }

    // Unpin the buffer frame (decrement reference count)
    pub fn unpin(&self) -> usize {
        let prev = self.pin_count.fetch_sub(1, Ordering::AcqRel);
        if prev == 0 {
            panic!("Attempt to unpin a buffer frame with pin count 0");
        }
        prev - 1
    }

    // Get current pin count
    pub fn pin_count(&self) -> usize {
        self.pin_count.load(Ordering::Acquire)
    }

    // Mark page as dirty
    pub fn mark_dirty(&self, lsn: u64) {
        self.dirty.store(true, Ordering::Release);
        self.lsn.store(lsn, Ordering::Release);
    }

    // Check if page is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }

    // Get page data (read-only)
    pub fn read_data(&self) -> &[u8] {
        &self.data
    }

    // Get mutable page data
    pub fn write_data(&mut self) -> &mut [u8] {
        &mut self.data
    }

    // Get current tier
    pub fn tier(&self) -> BufferTier {
        *self.tier.lock()
    }

    // Set tier
    pub fn set_tier(&self, new_tier: BufferTier) {
        *self.tier.lock() = new_tier;
    }

    // Get access count
    pub fn access_count(&self) -> u64 {
        self.access_count.load(Ordering::Relaxed)
    }

    // Get time since last access
    pub fn idle_time(&self) -> Duration {
        self.last_access.lock().elapsed()
    }
}

// Guard type for automatic unpinning of buffer frames
pub struct BufferFrameGuard {
    frame: Arc<BufferFrame>,
}

impl BufferFrameGuard {
    pub fn new(frame: Arc<BufferFrame>) -> Self {
        frame.pin();
        Self { frame }
    }

    pub fn frame(&self) -> &Arc<BufferFrame> {
        &self.frame
    }
}

impl Drop for BufferFrameGuard {
    fn drop(&mut self) {
        self.frame.unpin();
    }
}

// NUMA node configuration
#[derive(Debug, Clone)]
pub struct NumaNode {
    pub node_id: u32,
    pub cpu_mask: Vec<usize>,
    pub memory_base: usize,
    pub memory_size: usize,
}

// Multi-tier buffer pool configuration
#[derive(Debug, Clone)]
pub struct BufferPoolConfig {
    // Total buffer pool size in bytes
    pub total_size: usize,
    // Page size in bytes (typically 8192, 16384, or 32768)
    pub page_size: usize,
    // Hot tier percentage (0.0 - 1.0)
    pub hot_tier_ratio: f64,
    // Warm tier percentage (0.0 - 1.0)
    pub warm_tier_ratio: f64,
    // NUMA-aware allocation enabled
    pub numa_aware: bool,
    // NUMA node configurations
    pub numa_nodes: Vec<NumaNode>,
    // Per-tablespace pool configurations
    pub tablespace_pools: HashMap<u32, usize>,
    // Keep pool size in bytes
    pub keep_pool_size: usize,
    // Recycle pool size in bytes
    pub recycle_pool_size: usize,
    // Promotion threshold (access count)
    pub promotion_threshold: u64,
    // Demotion threshold (idle time in seconds)
    pub demotion_threshold_secs: u64,
}

impl Default for BufferPoolConfig {
    fn default() -> Self {
        Self {
            total_size: 1024 * 1024 * 1024, // 1GB
            page_size: 8192,                 // 8KB pages
            hot_tier_ratio: 0.2,             // 20% hot
            warm_tier_ratio: 0.5,            // 50% warm
            numa_aware: false,
            numa_nodes: Vec::new(),
            tablespace_pools: HashMap::new(),
            keep_pool_size: 64 * 1024 * 1024,    // 64MB
            recycle_pool_size: 128 * 1024 * 1024, // 128MB
            promotion_threshold: 10,
            demotion_threshold_secs: 300,    // 5 minutes
        }
    }
}
