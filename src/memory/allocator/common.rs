// Common types, constants, and imports for memory allocator modules

// Note: std::backtrace::Backtrace is nightly-only, not used in stable builds
// use std::backtrace::Backtrace;

pub use crate::error::{DbError, Result};

// Re-export commonly used types
pub use std::alloc::{GlobalAlloc, Layout, System};
pub use std::cell::RefCell;
pub use std::sync::atomic::{Ordering, AtomicBool, AtomicU64, AtomicUsize};
pub use std::ptr::{self, NonNull};
pub use std::collections::{HashMap, VecDeque};
pub use std::sync::{Arc, Mutex, RwLock, Weak};
pub use std::time::{Duration, Instant, SystemTime};

// ============================================================================
// Constants and Configuration
// ============================================================================

// Minimum allocation size (16 bytes for alignment)
pub(super) const MIN_ALLOC_SIZE: usize = 16;

// Maximum size for slab allocation (anything larger goes to large object allocator)
pub(super) const MAX_SLAB_SIZE: usize = 32 * 1024; // 32KB

// Number of size classes in the slab allocator
pub(super) const NUM_SIZE_CLASSES: usize = 64;

// Slab size (typically 2MB for huge page alignment)
pub(super) const SLAB_SIZE: usize = 2 * 1024 * 1024;

// Magazine capacity (number of objects cached per CPU)
pub(super) const MAGAZINE_CAPACITY: usize = 64;

// Number of colors for cache line optimization
pub(super) const NUM_COLORS: usize = 8;

// Large object threshold (use mmap directly)
pub(super) const LARGE_OBJECT_THRESHOLD: usize = 256 * 1024; // 256KB

// Huge page size (2MB)
pub(super) const HUGE_PAGE_2MB: usize = 2 * 1024 * 1024;

// Huge page size (1GB)
pub(super) const HUGE_PAGE_1GB: usize = 1024 * 1024 * 1024;

// Memory pressure warning threshold (80% of total)
pub(super) const MEMORY_PRESSURE_WARNING: f64 = 0.80;

// Memory pressure critical threshold (90% of total)
pub(super) const MEMORY_PRESSURE_CRITICAL: f64 = 0.90;

// Maximum number of stack frames to capture for leak detection (for debug builds)
#[allow(dead_code)]
pub(super) const MAX_STACK_FRAMES: usize = 32;

// Memory guard pattern for corruption detection
pub(super) const GUARD_PATTERN: u64 = 0xDEADBEEFCAFEBABE;

// ============================================================================
// Public API Types and Enums
// ============================================================================

// Memory allocation statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    // Total bytes allocated
    pub total_allocated: u64,
    // Total bytes freed
    pub total_freed: u64,
    // Current bytes in use
    pub bytes_in_use: u64,
    // Number of active allocations
    pub allocation_count: u64,
    // Peak memory usage
    pub peak_usage: u64,
    // Fragmentation ratio (0.0 to 1.0)
    pub fragmentation_ratio: f64,
    // Allocations per second
    pub allocations_per_sec: f64,
    // Average allocation size
    pub avg_allocation_size: u64,
}

// Memory pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressureLevel {
    // Normal operation (<80% usage)
    Normal,
    // Warning level (80-90% usage)
    Warning,
    // Critical level (90-95% usage)
    Critical,
    // Emergency level (>95% usage)
    Emergency,
    None,
    Medium,
    High,
    Low,
}

// Memory allocation source/component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllocationSource {
    // Buffer pool allocations
    BufferPool,
    // Query execution allocations
    QueryExecution,
    // Index structures
    Index,
    // Transaction management
    Transaction,
    // Network buffers
    Network,
    // Catalog metadata
    Catalog,
    // Temporary workspace
    Temporary,
    // Unknown/other
    Other,
}

// Memory context type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextType {
    // Top-level context
    TopLevel,
    // Per-query context
    Query,
    // Per-transaction context
    Transaction,
    // Session-level context
    Session,
    // Temporary context
    Temporary,
    Cache,
}

// Memory leak report entry
#[derive(Debug, Clone)]
pub struct LeakReport {
    // Allocation address
    pub address: usize,
    // Allocation size
    pub size: usize,
    // Allocation source
    pub source: AllocationSource,
    // Time when allocated
    pub allocated_at: SystemTime,
    // Stack trace at allocation
    pub stack_trace: String,
}
