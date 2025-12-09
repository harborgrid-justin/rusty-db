// # Memory Management Module
//
// Comprehensive memory and buffer pool management for RustyDB.
//
// This module provides enterprise-grade memory management including:
// - Multi-tier buffer pools (Hot/Warm/Cold)
// - Advanced caching algorithms (ARC, 2Q)
// - Intelligent replacement policies (Clock-Sweep, LRU-K)
// - Slab allocator with magazine-layer caching
// - Arena allocator for per-query memory contexts
// - Large object allocator with huge page support
// - Memory pressure management and OOM prevention
// - Memory debugging, leak detection, and profiling
//
// ## Buffer Pool Example
//
// ```rust,no_run
// use rusty_db::memory::buffer_pool::{BufferPoolManager, BufferPoolConfig};
//
// // Create buffer pool with custom configuration
// let config = BufferPoolConfig {
//     total_size: 2 * 1024 * 1024 * 1024, // 2GB
//     page_size: 8192,
//     hot_tier_ratio: 0.2,
//     warm_tier_ratio: 0.5,
//     ..Default::default()
// };
//
// let manager = BufferPoolManager::new(config);
//
// // Start background operations
// manager.api_start_background_operations();
//
// // Pin a page
// if let Some(frame) = manager.api_pin_page(0, 1) {
//     // Work with the page...
//     manager.api_unpin_page(0, 1, false);
// }
//
// // Get statistics
// let stats = manager.api_get_stats();
// println!("Buffer pool stats: {}", stats);
// ```
//
// ## Memory Allocator Example
//
// ```rust,no_run
// use rusty_db::memory::{MemoryManager, AllocationSource, ContextType};
//
// // Create memory manager with 8GB total
// let manager = MemoryManager::new(8 * 1024 * 1024 * 1024);
//
// // Allocate memory
// let ptr = manager.allocate(1024, AllocationSource::QueryExecution).unwrap();
//
// // Create query context
// let context = manager.create_context(
//     "query-123".to_string(),
//     ContextType::Query,
//     100 * 1024 * 1024  // 100MB limit
// ).unwrap();
//
// // Get comprehensive stats
// let stats = manager.get_comprehensive_stats();
// ```

pub mod buffer_pool;
pub mod allocator;
pub mod debug;
pub mod types;

// Re-export commonly used types from buffer pool
pub use buffer_pool::{
    BufferPoolManager,
    BufferPoolConfig,
    BufferPoolStatsSnapshot,
    PageId,
    BufferTier,
    PoolType,
    ComprehensiveBufferStats,
    MemoryPressureSnapshot,
    CheckpointResult,
};

// Re-export commonly used types from allocator
pub use allocator::{
    // Main manager
    MemoryManager,

    // Allocators
    SlabAllocator,
    ArenaAllocator,
    LargeObjectAllocator,
    MemoryPressureManager,
    MemoryDebugger,

    // Advanced allocators
    MemoryPool,
    MemoryZone,
    BuddyAllocator,

    // Memory contexts
    MemoryContext,
    ContextType,

    // Statistics and reports
    MemoryStats,
    SlabAllocatorStats,
    ArenaAllocatorStats,
    MemoryContextStats,
    LargeObjectAllocatorStats,
    MemoryPressureStats,
    MemoryDebuggerStats,
    ComprehensiveMemoryStats,
    MemoryReport,
    ComponentBreakdown,
    MemoryPoolStats,
    MemoryZoneStats,
    UsageSummary,

    // Performance monitoring
    PerformanceCounter,
    PerformanceStats,
    AccessPatternAnalyzer,
    AccessPatternStats,
    BandwidthMonitor,
    BandwidthStats,

    // Types
    MemoryPressureLevel,
    AllocationSource,
    MemoryUsage,
    MemoryPressureEvent,
    LeakReport,
    PressureCallback,
    ZoneType,
    AllocatorType,

    // Web API
    MemoryApi,

    // Utility functions
    format_memory_size,
    parse_memory_size,
    calculate_optimal_slab_size,
    classify_allocation_size,
};

// Re-export debug types
pub use debug::{
    DebugError,
    AllocationMetadata,
    CorruptionReport,
    CorruptionDetails,
    CorruptionType,
    CorruptionSeverity,
    MemoryLeakReport,
    LeakSummary,
    MemoryProfile,
    DebugStats,
};
