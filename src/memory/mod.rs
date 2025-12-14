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

pub mod allocator;
pub mod buffer_pool;
pub mod types;

// Re-export commonly used types from buffer pool
pub use buffer_pool::{
    BufferPoolConfig, BufferPoolManager, BufferPoolStatsSnapshot, BufferTier, CheckpointResult,
    ComprehensiveBufferStats, MemoryPressureSnapshot, PageId, PoolType,
};

// Re-export commonly used types from allocator
pub use allocator::{
    calculate_optimal_slab_size,
    classify_allocation_size,
    // Utility functions
    format_memory_size,
    parse_memory_size,
    AccessPatternAnalyzer,
    AccessPatternStats,
    AllocationSource,
    AllocatorType,

    ArenaAllocator,
    ArenaAllocatorStats,
    BandwidthMonitor,
    BandwidthStats,

    BuddyAllocator,

    ComponentBreakdown,
    ComprehensiveMemoryStats,
    ContextType,

    LargeObjectAllocator,
    LargeObjectAllocatorStats,
    LeakReport,
    // Web API
    MemoryApi,

    // Memory contexts
    MemoryContext,
    MemoryContextStats,
    MemoryDebugger,

    MemoryDebuggerStats,
    // Main manager
    MemoryManager,

    // Advanced allocators
    MemoryPool,
    MemoryPoolStats,
    MemoryPressureEvent,
    // Types
    MemoryPressureLevel,
    MemoryPressureManager,
    MemoryPressureStats,
    MemoryReport,
    // Statistics and reports
    MemoryStats,
    MemoryUsage,
    MemoryZone,
    MemoryZoneStats,
    // Performance monitoring
    PerformanceCounter,
    PerformanceStats,
    PressureCallback,
    // Allocators
    SlabAllocator,
    SlabAllocatorStats,
    UsageSummary,

    ZoneType,
};
