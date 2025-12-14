// Enterprise Memory Allocator System
//
// This module provides a comprehensive memory allocation system for RustyDB,
// implementing multiple allocation strategies optimized for different workload patterns.
//
// ## Architecture Overview
//
// The memory system consists of multiple components:
//
// 1. **Slab Allocator**: Fixed-size allocation with thread-local caching
// 2. **Arena Allocator**: Bump allocation for per-query memory contexts
// 3. **Large Object Allocator**: Direct mmap for huge allocations
// 4. **Memory Pressure Manager**: Global memory monitoring and OOM prevention
// 5. **Memory Debugger**: Leak detection and profiling
//
// ## Design Philosophy
//
// - **Performance**: Lock-free thread-local caches, magazine-layer optimization
// - **Safety**: Rust memory safety with explicit unsafe boundaries
// - **Observability**: Comprehensive metrics and debugging capabilities
// - **Scalability**: Per-thread and per-query isolation

// Internal modules
mod api;
mod arena_allocator;
pub(crate) mod common;
mod debugger;
mod large_object_allocator;
mod memory_manager;
mod monitoring;
mod pools;
mod pressure_manager;
mod slab_allocator;
mod utils;
mod zones;

// Re-export public types from common
pub use common::{AllocationSource, ContextType, LeakReport, MemoryPressureLevel, MemoryStats};

// Re-export slab allocator
pub use slab_allocator::{SlabAllocator, SlabAllocatorStats};

// Re-export arena allocator
pub use arena_allocator::{ArenaAllocator, ArenaAllocatorStats, MemoryContext, MemoryContextStats};

// Re-export large object allocator
pub use large_object_allocator::{LargeObjectAllocator, LargeObjectAllocatorStats};

// Re-export pressure manager
pub use pressure_manager::{
    MemoryPressureEvent, MemoryPressureManager, MemoryPressureStats, MemoryUsage, PressureCallback,
};

// Re-export debugger
pub use debugger::{ComponentBreakdown, MemoryDebugger, MemoryDebuggerStats, MemoryReport};

// Re-export memory manager
pub use memory_manager::{ComprehensiveMemoryStats, MemoryManager};

// Re-export pools
pub use pools::{MemoryPool, MemoryPoolStats};

// Re-export zones
pub use zones::{BuddyAllocator, MemoryZone, MemoryZoneStats, ZoneType};

// Re-export monitoring
pub use monitoring::{
    AccessPatternAnalyzer, AccessPatternStats, BandwidthMonitor, BandwidthStats,
    PerformanceCounter, PerformanceStats,
};

// Re-export API
pub use api::{MemoryApi, UsageSummary};

// Re-export utils
pub use utils::{
    calculate_optimal_slab_size, classify_allocation_size, format_memory_size, parse_memory_size,
    AllocatorType,
};
