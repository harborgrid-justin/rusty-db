//! # High-Performance Buffer Manager
//!
//! Enterprise-grade buffer pool management system optimized for Windows/MSVC with:
//!
//! - **Zero-allocation hot path**: Pin/unpin operations don't allocate
//! - **Lock-free page table**: Partitioned hash map for concurrent access
//! - **Per-core frame pools**: NUMA-aware allocation reduces contention
//! - **Batch flush support**: Optimized sequential I/O for dirty pages
//! - **Windows IOCP ready**: Async I/O integration points
//! - **Multiple eviction policies**: CLOCK, LRU, 2Q, LRU-K
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  Buffer Pool Manager                        │
//! │                                                              │
//! │  ┌────────────┐  ┌────────────┐  ┌────────────┐           │
//! │  │ Page Table │  │   Frames   │  │  Eviction  │           │
//! │  │ (Lock-free)│  │  (Aligned) │  │   Policy   │           │
//! │  └────────────┘  └────────────┘  └────────────┘           │
//! │                                                              │
//! │  ┌────────────────────────────────────────────┐            │
//! │  │         Per-Core Frame Pools               │            │
//! │  │  Core 0  │  Core 1  │  Core 2  │  Core 3  │            │
//! │  └────────────────────────────────────────────┘            │
//! └─────────────────────────────────────────────────────────────┘
//!          │                   │                   │
//!          ▼                   ▼                   ▼
//!    ┌──────────┐        ┌──────────┐       ┌──────────┐
//!    │   Disk   │        │   WAL    │       │  Network │
//!    │  Manager │        │ Manager  │       │   I/O    │
//!    └──────────┘        └──────────┘       └──────────┘
//! ```
//!
//! ## Performance Characteristics
//!
//! ### Hot Path (Page in Buffer Pool)
//! - **Page table lookup**: O(1) - lock-free hash map
//! - **Pin operation**: O(1) - atomic increment
//! - **Memory allocation**: 0 bytes
//! - **Latency**: ~50-100ns (L3 cache hit)
//!
//! ### Cold Path (Page Fault)
//! - **Frame allocation**: O(1) - per-core pool or O(1) global list
//! - **Eviction scan**: O(n) worst case, O(1) amortized for CLOCK
//! - **Disk I/O**: ~100µs for SSD, ~10ms for HDD
//!
//! ### Concurrent Access
//! - **Page table**: 16 partitions by default (configurable)
//! - **Per-core pools**: No contention for local allocations
//! - **Pin/unpin**: Lock-free atomics
//!
//! ## Usage Examples
//!
//! ### Basic Usage
//!
//! ```rust
//! use rusty_db::buffer::{BufferPoolBuilder, EvictionPolicyType};
//!
//! # fn example() -> rusty_db::Result<()> {
//! // Create buffer pool with 1000 frames
//! let buffer_pool = BufferPoolBuilder::new()
//!     .num_frames(1000)
//!     .eviction_policy(EvictionPolicyType::Clock)
//!     .build();
//!
//! // Pin a page
//! let page_id = 42;
//! let guard = buffer_pool.pin_page(page_id)?;
//!
//! // Access page data
//! let data = guard.read_data();
//! // ... use data ...
//!
//! // Page is automatically unpinned when guard is dropped
//! drop(guard);
//! # Ok(())
//! # }
//! ```
//!
//! ### Advanced Configuration
//!
//! ```rust
//! use rusty_db::buffer::{BufferPoolBuilder, EvictionPolicyType};
//! use std::time::Duration;
//!
//! # fn example() -> rusty_db::Result<()> {
//! let buffer_pool = BufferPoolBuilder::new()
//!     .num_frames(10000)
//!     .eviction_policy(EvictionPolicyType::TwoQ)
//!     .per_core_pools(true)
//!     .frames_per_core(8)
//!     .max_flush_batch_size(64)
//!     .background_flush(true)
//!     .flush_interval(Duration::from_secs(30))
//!     .dirty_threshold(0.7)
//!     .build();
//!
//! // Pin multiple pages
//! let guards: Vec<_> = (0..10)
//!     .map(|i| buffer_pool.pin_page(i))
//!     .collect::<Result<_, _>>()?;
//!
//! // Modify pages
//! for guard in &guards {
//!     let mut data = guard.write_data();
//!     // ... modify data ...
//! }
//!
//! // Get statistics
//! let stats = buffer_pool.stats();
//! println!("Hit rate: {:.2}%", stats.hit_rate * 100.0);
//! println!("Dirty pages: {}", stats.dirty_frames);
//! # Ok(())
//! # }
//! ```
//!
//! ### Batch Flush
//!
//! ```rust
//! use rusty_db::buffer::{BufferPoolBuilder, FrameBatch};
//!
//! # fn example() -> rusty_db::Result<()> {
//! let buffer_pool = BufferPoolBuilder::new()
//!     .num_frames(1000)
//!     .build();
//!
//! // Flush all dirty pages
//! buffer_pool.flush_all()?;
//!
//! // Check dirty ratio
//! let dirty_ratio = buffer_pool.dirty_page_ratio();
//! println!("Dirty pages: {:.1}%", dirty_ratio * 100.0);
//! # Ok(())
//! # }
//! ```
//!
//! ## Eviction Policies
//!
//! ### CLOCK (Default)
//! - Simple second-chance algorithm
//! - Good approximation of LRU
//! - Constant memory overhead
//! - Used by PostgreSQL, SQLite
//!
//! ### LRU
//! - True least-recently-used
//! - O(1) operations with intrusive list
//! - Higher memory overhead
//! - Best for predictable workloads
//!
//! ### 2Q
//! - Scan-resistant algorithm
//! - Three queues: A1in, A1out, Am
//! - Excellent for mixed workloads
//! - Used by Oracle (similar)
//!
//! ### LRU-K
//! - Tracks K-th access time
//! - K=2 provides good scan resistance
//! - Higher CPU overhead
//! - Best for analytical workloads
//!
//! ## Memory Layout
//!
//! All buffer structures use `#[repr(C)]` for predictable layout:
//!
//! ```text
//! PageBuffer (4096 bytes, aligned to 4096):
//! ┌────────────────────────────────┐
//! │  data: [u8; 4096]             │ ← 4KB aligned
//! └────────────────────────────────┘
//!
//! BufferFrame (~4200 bytes):
//! ┌────────────────────────────────┐
//! │  page_id: u64                  │
//! │  frame_id: u32                 │
//! │  pin_count: AtomicU32          │
//! │  dirty: AtomicBool             │
//! │  ...metadata...                │
//! │  data: PageBuffer (4096)       │ ← 4KB aligned
//! └────────────────────────────────┘
//! ```
//!
//! ## Windows IOCP Integration
//!
//! The buffer manager is designed for integration with Windows I/O Completion Ports:
//!
//! ```rust,no_run
//! #[cfg(target_os = "windows")]
//! use rusty_db::buffer::windows::IocpContext;
//!
//! # #[cfg(target_os = "windows")]
//! # fn example() -> rusty_db::Result<()> {
//! // Create IOCP context
//! let iocp = IocpContext::new()?;
//!
//! // Submit async read
//! // iocp.async_read(page_id, buffer)?;
//!
//! // Poll for completions
//! // let completions = iocp.poll_completions(timeout_ms)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Safety
//!
//! This module uses `unsafe` code in performance-critical paths:
//!
//! - `get_unchecked`: Array access with compile-time bounds checking
//! - `ptr::copy_nonoverlapping`: Page data copies (always valid)
//! - Raw pointers: Zero-copy I/O (lifetime guarantees via guards)
//!
//! All unsafe code is documented with safety invariants.
//!
//! ## Performance Tuning
//!
//! ### Buffer Pool Size
//! - **Too small**: High eviction rate, poor hit rate
//! - **Too large**: Wasted memory, longer eviction scans
//! - **Rule of thumb**: 25-50% of available RAM for OLTP
//! - **OLAP**: 50-75% of RAM (more read-heavy)
//!
//! ### Page Table Partitions
//! - **Default**: 16 partitions
//! - **High concurrency**: 32-64 partitions
//! - **Low concurrency**: 4-8 partitions
//!
//! ### Per-Core Pools
//! - **NUMA systems**: Enable for better locality
//! - **Non-NUMA**: May not improve performance
//! - **Frames per core**: 4-16 (tune based on workload)
//!
//! ### Eviction Policy
//! - **OLTP**: CLOCK (fast, low overhead)
//! - **OLAP**: LRU or 2Q (better scan resistance)
//! - **Mixed**: 2Q or LRU-2
//!
//! ## Monitoring
//!
//! ```rust
//! use rusty_db::buffer::BufferPoolBuilder;
//!
//! # fn example() -> rusty_db::Result<()> {
//! let buffer_pool = BufferPoolBuilder::new()
//!     .num_frames(1000)
//!     .build();
//!
//! // Get detailed statistics
//! let stats = buffer_pool.stats();
//!
//! println!("=== Buffer Pool Statistics ===");
//! println!("Total frames: {}", stats.total_frames);
//! println!("Free frames: {}", stats.free_frames);
//! println!("Pinned frames: {}", stats.pinned_frames);
//! println!("Dirty frames: {}", stats.dirty_frames);
//! println!("Hit rate: {:.2}%", stats.hit_rate * 100.0);
//! println!("Page reads: {}", stats.page_reads);
//! println!("Page writes: {}", stats.page_writes);
//! println!("Evictions: {}", stats.evictions);
//! println!("Avg search length: {:.2}", stats.avg_search_length);
//! println!("I/O wait time: {}µs", stats.io_wait_time_us);
//! # Ok(())
//! # }
//! ```

// ============================================================================
// Module Exports
// ============================================================================

pub mod eviction;
pub mod manager;
pub mod page_cache;

// New advanced buffer pool modules
pub mod arc;
pub mod lirs;
pub mod prefetch;
pub mod hugepages;
pub mod lockfree_latch;

// Re-export main types
pub use eviction::{
    create_eviction_policy, ClockEvictionPolicy, EvictionPolicy, EvictionPolicyType,
    EvictionStats, LruEvictionPolicy, TwoQEvictionPolicy,
};

pub use manager::{BufferPoolBuilder, BufferPoolConfig, BufferPoolManager, BufferPoolStats};

pub use page_cache::{
    BufferFrame, FrameBatch, FrameGuard, FrameId, PageBuffer, PerCoreFramePool, FrameStats,
    INVALID_FRAME_ID, INVALID_PAGE_ID, PAGE_SIZE,
};

// Re-export advanced eviction policies
pub use arc::ArcEvictionPolicy;
pub use lirs::LirsEvictionPolicy;

// Re-export prefetching
pub use prefetch::{
    AccessPattern, PatternDetector, PrefetchConfig, PrefetchEngine, PrefetchRequest, PrefetchStats,
};

// Re-export huge pages
pub use hugepages::{
    HugePageAllocator, HugePageAllocation, HugePageConfig, HugePageSize, HugePageStats,
    HugePageSystemInfo, query_huge_page_info,
};

// Re-export lock-free latching
pub use lockfree_latch::{
    HybridLatch, LatchStats, OptimisticLatch, ReadGuard, WriteGuard,
};

// ============================================================================
// Convenience Functions
// ============================================================================

/// Create a buffer pool with default configuration
///
/// # Example
///
/// ```rust
/// use rusty_db::buffer::create_default_buffer_pool;
///
/// # fn example() -> rusty_db::Result<()> {
/// let buffer_pool = create_default_buffer_pool(1000);
/// # Ok(())
/// # }
/// ```
pub fn create_default_buffer_pool(num_frames: usize) -> BufferPoolManager {
    BufferPoolBuilder::new().num_frames(num_frames).build()
}

/// Create a buffer pool optimized for OLTP workloads
///
/// Uses CLOCK eviction and enables per-core pools.
///
/// # Example
///
/// ```rust
/// use rusty_db::buffer::create_oltp_buffer_pool;
///
/// # fn example() -> rusty_db::Result<()> {
/// let buffer_pool = create_oltp_buffer_pool(10000);
/// # Ok(())
/// # }
/// ```
pub fn create_oltp_buffer_pool(num_frames: usize) -> BufferPoolManager {
    BufferPoolBuilder::new()
        .num_frames(num_frames)
        .eviction_policy(EvictionPolicyType::Clock)
        .per_core_pools(true)
        .frames_per_core(8)
        .background_flush(true)
        .build()
}

/// Create a buffer pool optimized for OLAP workloads
///
/// Uses 2Q eviction for scan resistance and larger batch sizes.
///
/// # Example
///
/// ```rust
/// use rusty_db::buffer::create_olap_buffer_pool;
///
/// # fn example() -> rusty_db::Result<()> {
/// let buffer_pool = create_olap_buffer_pool(50000);
/// # Ok(())
/// # }
/// ```
pub fn create_olap_buffer_pool(num_frames: usize) -> BufferPoolManager {
    BufferPoolBuilder::new()
        .num_frames(num_frames)
        .eviction_policy(EvictionPolicyType::TwoQ)
        .per_core_pools(false)
        .max_flush_batch_size(128)
        .background_flush(true)
        .dirty_threshold(0.5)
        .build()
}

// ============================================================================
// Module-level Documentation Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_buffer_pool() {
        let pool = create_default_buffer_pool(100);
        assert_eq!(pool.config().num_frames, 100);
    }

    #[test]
    fn test_create_oltp_buffer_pool() {
        let pool = create_oltp_buffer_pool(100);
        assert_eq!(pool.config().num_frames, 100);
        assert_eq!(pool.eviction_policy_name(), "CLOCK");
    }

    #[test]
    fn test_create_olap_buffer_pool() {
        let pool = create_olap_buffer_pool(100);
        assert_eq!(pool.config().num_frames, 100);
        assert_eq!(pool.eviction_policy_name(), "2Q");
    }

    #[test]
    fn test_buffer_pool_basic_operations() {
        let pool = create_default_buffer_pool(10);

        // Pin a page
        let guard = pool.pin_page(1).unwrap();
        assert_eq!(guard.page_id(), 1);

        // Read data
        let data = guard.read_data();
        assert_eq!(data.data().len(), PAGE_SIZE);

        drop(data);
        drop(guard);

        // Get stats
        let stats = pool.stats();
        assert_eq!(stats.total_frames, 10);
        assert!(stats.page_reads > 0);
    }

    #[test]
    fn test_buffer_pool_multi_pin() {
        let pool = create_default_buffer_pool(10);

        // Pin multiple pages
        let mut guards = Vec::new();
        for i in 0..5 {
            guards.push(pool.pin_page(i).unwrap());
        }

        let stats = pool.stats();
        assert!(stats.pinned_frames >= 5);
    }

    #[test]
    fn test_buffer_pool_flush() {
        let pool = create_default_buffer_pool(10);

        // Pin and modify a page
        {
            let guard = pool.pin_page(1).unwrap();
            let mut data = guard.write_data();
            data.data_mut()[0] = 42;
        }

        // Flush all pages
        pool.flush_all().unwrap();

        let stats = pool.stats();
        assert!(stats.page_writes > 0 || stats.dirty_frames == 0);
    }

    #[test]
    fn test_page_buffer_alignment() {
        let buffer = PageBuffer::new();
        let ptr = buffer.as_ptr();
        assert_eq!(ptr as usize % 4096, 0, "PageBuffer must be 4096-byte aligned");
    }

    #[test]
    fn test_page_buffer_operations() {
        let mut buffer = PageBuffer::new();

        // Zero the buffer
        buffer.zero();
        assert!(buffer.is_zeroed());

        // Write some data
        buffer.data_mut()[0] = 42;
        buffer.data_mut()[100] = 123;
        assert!(!buffer.is_zeroed());

        // Calculate checksum
        let checksum = buffer.checksum();
        assert!(checksum > 0);
        assert!(buffer.verify_checksum(checksum));

        // Modify and checksum should change
        buffer.data_mut()[0] = 43;
        assert!(!buffer.verify_checksum(checksum));
    }

    #[test]
    fn test_frame_guard_auto_unpin() {
        use std::sync::Arc;

        let frame = Arc::new(BufferFrame::new(0));
        assert_eq!(frame.pin_count(), 0);

        {
            let _guard = FrameGuard::new(frame.clone());
            assert_eq!(frame.pin_count(), 1);
        }

        // Guard dropped, frame should be unpinned
        assert_eq!(frame.pin_count(), 0);
    }

    #[test]
    fn test_eviction_policy_factory() {
        let clock = create_eviction_policy(EvictionPolicyType::Clock, 100);
        assert_eq!(clock.name(), "CLOCK");

        let lru = create_eviction_policy(EvictionPolicyType::Lru, 100);
        assert_eq!(lru.name(), "LRU");

        let twoq = create_eviction_policy(EvictionPolicyType::TwoQ, 100);
        assert_eq!(twoq.name(), "2Q");

        let lru_k = create_eviction_policy(EvictionPolicyType::LruK(2), 100);
        assert_eq!(lru_k.name(), "LRU-K");
    }

    #[test]
    fn test_buffer_pool_statistics() {
        let pool = create_default_buffer_pool(100);

        // Pin some pages
        let _g1 = pool.pin_page(1).unwrap();
        let _g2 = pool.pin_page(2).unwrap();
        let _g3 = pool.pin_page(3).unwrap();

        let stats = pool.stats();
        assert_eq!(stats.total_frames, 100);
        assert!(stats.pinned_frames >= 3);
        assert!(stats.lookups >= 3);
        assert_eq!(stats.page_reads, 3);
    }

    #[test]
    fn test_dirty_page_tracking() {
        let pool = create_default_buffer_pool(10);

        // Pin and modify a page
        {
            let guard = pool.pin_page(1).unwrap();
            let mut _data = guard.write_data();
            // Writing automatically marks as dirty
        }

        let stats = pool.stats();
        assert!(stats.dirty_frames > 0);

        let dirty_ratio = pool.dirty_page_ratio();
        assert!(dirty_ratio > 0.0);
        assert!(dirty_ratio <= 1.0);
    }
}


