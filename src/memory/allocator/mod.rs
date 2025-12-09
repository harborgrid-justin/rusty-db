//! Enterprise Memory Allocator System
//!
//! This module provides a comprehensive memory allocation system for RustyDB,
//! implementing multiple allocation strategies optimized for different workload patterns.
//!
//! ## Architecture Overview
//!
//! The memory system consists of multiple components:
//!
//! 1. **Slab Allocator**: Fixed-size allocation with thread-local caching
//! 2. **Arena Allocator**: Bump allocation for per-query memory contexts
//! 3. **Large Object Allocator**: Direct mmap for huge allocations
//! 4. **Memory Pressure Manager**: Global memory monitoring and OOM prevention
//! 5. **Memory Debugger**: Leak detection and profiling
//!
//! ## Design Philosophy
//!
//! - **Performance**: Lock-free thread-local caches, magazine-layer optimization
//! - **Safety**: Rust memory safety with explicit unsafe boundaries
//! - **Observability**: Comprehensive metrics and debugging capabilities
//! - **Scalability**: Per-thread and per-query isolation

// Internal modules
pub(crate) mod common;
mod slab_allocator;
mod arena_allocator;
mod large_object_allocator;
mod pressure_manager;
mod debugger;
mod memory_manager;
mod pools;
mod zones;
mod monitoring;
mod api;
mod utils;

// Re-export public types from common
pub use common::{
    MemoryStats,
    MemoryPressureLevel,
    AllocationSource,
    ContextType,
    LeakReport,
};

// Re-export slab allocator
pub use slab_allocator::{
    SlabAllocator,
    SlabAllocatorStats,
};

// Re-export arena allocator
pub use arena_allocator::{
    MemoryContext,
    MemoryContextStats,
    ArenaAllocator,
    ArenaAllocatorStats,
};

// Re-export large object allocator
pub use large_object_allocator::{
    LargeObjectAllocator,
    LargeObjectAllocatorStats,
};

// Re-export pressure manager
pub use pressure_manager::{
    MemoryPressureManager,
    MemoryPressureEvent,
    MemoryUsage,
    MemoryPressureStats,
    PressureCallback,
};

// Re-export debugger
pub use debugger::{
    MemoryDebugger,
    MemoryReport,
    ComponentBreakdown,
    MemoryDebuggerStats,
};

// Re-export memory manager
pub use memory_manager::{
    MemoryManager,
    ComprehensiveMemoryStats,
};

// Re-export pools
pub use pools::{
    MemoryPool,
    MemoryPoolStats,
};

// Re-export zones
pub use zones::{
    MemoryZone,
    ZoneType,
    MemoryZoneStats,
    BuddyAllocator,
};

// Re-export monitoring
pub use monitoring::{
    PerformanceCounter,
    PerformanceStats,
    AccessPatternAnalyzer,
    AccessPatternStats,
    BandwidthMonitor,
    BandwidthStats,
};

// Re-export API
pub use api::{
    MemoryApi,
    UsageSummary,
};

// Re-export utils
pub use utils::{
    format_memory_size,
    parse_memory_size,
    calculate_optimal_slab_size,
    classify_allocation_size,
    AllocatorType,
};
