//! Enterprise Buffer Pool Management System
//!
//! Comprehensive multi-tier buffer pool implementation with advanced caching,
//! replacement policies, and dirty page management for high-performance database operations.

// Internal modules
pub(crate) mod common;
mod multi_tier;
mod arc;
mod two_q;
mod prefetcher;
mod eviction_policies;
mod checkpoint;
mod writer;
mod statistics;
mod manager;

// Re-export common types
pub use common::{
    PageId,
    BufferTier,
    PoolType,
    BufferFrame,
    BufferFrameGuard,
    BufferPoolConfig,
};

// Re-export from multi_tier
pub use multi_tier::{
    MultiTierBufferPool,
    BufferPoolStatsSnapshot,
};

// Re-export from arc
pub use arc::{
    AdaptiveReplacementCache,
    ArcStatsSnapshot,
};

// Re-export from two_q
pub use two_q::{
    TwoQCache,
    TwoQStatsSnapshot,
};

// Re-export from prefetcher
pub use prefetcher::{
    PagePrefetcher,
    PrefetchStatsSnapshot,
};

// Re-export from eviction_policies
pub use eviction_policies::{
    ClockSweepPolicy,
    LruKPolicy,
    TouchCountOptimizer,
    CostAwareReplacement,
    ClockStatsSnapshot,
    LruKStatsSnapshot,
    TouchOptimizerStatsSnapshot,
    CostAwareStatsSnapshot,
};

// Re-export from checkpoint
pub use checkpoint::{
    CheckpointQueue,
    CheckpointResult,
    CheckpointStatsSnapshot,
    IncrementalCheckpointer,
    IncrementalCheckpointStatsSnapshot,
};

// Re-export from writer
pub use writer::{
    BackgroundWriter,
    BackgroundWriterStatsSnapshot,
    WriteCoalescingBuffer,
    CoalescingStatsSnapshot,
    DoubleWriteBuffer,
    DoubleWriteStatsSnapshot,
    FlushListManager,
    FlushListStatsSnapshot,
};

// Re-export from statistics
pub use statistics::{
    BufferPoolStatisticsTracker,
    PageType,
    ComprehensiveBufferStats,
    MemoryPressureSnapshot,
};

// Re-export from manager
pub use manager::{
    BufferPoolManager,
};
