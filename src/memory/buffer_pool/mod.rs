// Enterprise Buffer Pool Management System
//
// Comprehensive multi-tier buffer pool implementation with advanced caching,
// replacement policies, and dirty page management for high-performance database operations.

// Internal modules
mod arc;
mod checkpoint;
pub(crate) mod common;
mod eviction_policies;
mod manager;
mod multi_tier;
mod prefetcher;
mod statistics;
mod two_q;
mod writer;

// Re-export common types
pub use common::{BufferFrame, BufferFrameGuard, BufferPoolConfig, BufferTier, PageId, PoolType};

// Re-export from multi_tier
pub use multi_tier::{BufferPoolStatsSnapshot, MultiTierBufferPool};

// Re-export from arc
pub use arc::{AdaptiveReplacementCache, ArcStatsSnapshot};

// Re-export from two_q
pub use two_q::{TwoQCache, TwoQStatsSnapshot};

// Re-export from prefetcher
pub use prefetcher::{PagePrefetcher, PrefetchStatsSnapshot};

// Re-export from eviction_policies
pub use eviction_policies::{
    ClockStatsSnapshot, ClockSweepPolicy, CostAwareReplacement, CostAwareStatsSnapshot, LruKPolicy,
    LruKStatsSnapshot, TouchCountOptimizer, TouchOptimizerStatsSnapshot,
};

// Re-export from checkpoint
pub use checkpoint::{
    CheckpointQueue, CheckpointResult, CheckpointStatsSnapshot, IncrementalCheckpointStatsSnapshot,
    IncrementalCheckpointer,
};

// Re-export from writer
pub use writer::{
    BackgroundWriter, BackgroundWriterStatsSnapshot, CoalescingStatsSnapshot, DoubleWriteBuffer,
    DoubleWriteStatsSnapshot, FlushListManager, FlushListStatsSnapshot, WriteCoalescingBuffer,
};

// Re-export from statistics
pub use statistics::{
    BufferPoolStatisticsTracker, ComprehensiveBufferStats, MemoryPressureSnapshot, PageType,
};

// Re-export from manager
pub use manager::BufferPoolManager;
