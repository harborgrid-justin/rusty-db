// # Memory System Core Types
//
// This module provides fundamental types, identifiers, and data structures
// used throughout the enterprise memory allocation system. It defines
// strong typing patterns with newtypes and comprehensive validation.
//
// ## Key Features
//
// - **Strong Typing**: Extensive use of newtypes for type safety
// - **Memory Management Types**: Allocation tracking, pressure levels, and statistics
// - **Configuration Types**: Comprehensive configuration structures for all allocators
// - **Statistics Types**: Detailed metrics and performance monitoring structures
// - **Error Types**: Specific error types for memory allocation failures
// - **Validation**: Input validation for all public types
//
// ## Type Categories
//
// ### Identifiers
// - **MemoryContextId**: Unique identifier for memory contexts
// - **AllocationId**: Tracking ID for individual allocations
// - **SourceComponent**: Component that requested the allocation
//
// ### Configuration Types
// - **AllocatorConfig**: Configuration for different allocator types
// - **PressureConfig**: Memory pressure monitoring configuration
// - **DebugConfig**: Memory debugging and profiling configuration
//
// ### Statistics Types
// - **MemoryStats**: Comprehensive memory usage statistics
// - **AllocatorStats**: Per-allocator performance metrics
// - **ContextStats**: Memory context usage tracking
//
// ## Usage Example
//
// ```rust
// use crate::memory::types::*;
//
// # fn example() -> Result<(), Box<dyn std::error::Error>> {
// // Create a memory context identifier
// let context_id = MemoryContextId::new("query_executor_001")?;
//
// // Configure slab allocator
// let slab_config = SlabConfig {
//     num_size_classes: 64,
//     magazine_capacity: 64,
//     max_slab_size: 32 * 1024, // 32KB
//     enable_thread_caching: true,
//     enable_coloring: true,
//     color_count: 8,
//     ..Default::default()
// };
//
// // Set memory pressure thresholds
// let pressure_config = PressureConfig {
//     warning_threshold: 0.80, // 80% of total memory
//     critical_threshold: 0.90, // 90% of total memory
//     emergency_threshold: 0.95, // 95% of total memory
//     enable_monitoring: true,
//     check_interval: Duration::from_secs(5),
//     ..Default::default()
// };
//
// // Track allocation source
// let _source = AllocationSource::Query {
//     query_id: "SELECT_001".to_string(),
//     operation: "hash_join".to_string(),
// };
// # Ok(())
// # }
// ```

use std::time::{SystemTime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
use std::time::{Duration};
use thiserror::Error;
use uuid::Uuid;

/// Memory allocator specific errors
#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Out of memory: {reason}")]
    OutOfMemory { reason: String },

    #[error("Invalid allocation size: {size} bytes - {reason}")]
    InvalidSize { size: usize, reason: String },

    #[error("Invalid alignment: {alignment} - {reason}")]
    InvalidAlignment { alignment: usize, reason: String },

    #[error("Memory context not found: {context_id}")]
    ContextNotFound { context_id: String },

    #[error("Memory pressure critical: current {current_usage}, limit {limit}")]
    PressureCritical { current_usage: u64, limit: u64 },

    #[error("Allocation corruption detected at {address:#x}: {reason}")]
    CorruptionDetected { address: usize, reason: String },

    #[error("Memory leak detected: {leak_count} allocations totaling {total_bytes} bytes")]
    LeakDetected { leak_count: usize, total_bytes: u64 },

    #[error("Double free detected at {address:#x}")]
    DoubleFree { address: usize },

    #[error("Use after free detected at {address:#x}")]
    UseAfterFree { address: usize },

    #[error("Zone overflow: requested {requested}, available {available} in zone '{zone_name}'")]
    ZoneOverflow { zone_name: String, requested: usize, available: usize },

    #[error("Configuration invalid: {field} - {reason}")]
    InvalidConfiguration { field: String, reason: String },
}

/// Constants for memory management
pub mod constants {
    /// Minimum allocation size (16 bytes for alignment)
    pub const MIN_ALLOC_SIZE: usize = 16;

    /// Maximum size for slab allocation (anything larger goes to large object allocator)
    pub const MAX_SLAB_SIZE: usize = 32 * 1024; // 32KB

    /// Number of size classes in the slab allocator
    pub const NUM_SIZE_CLASSES: usize = 64;

    /// Slab size (typically 2MB for huge page alignment)
    pub const SLAB_SIZE: usize = 2 * 1024 * 1024;

    /// Magazine capacity (number of objects cached per CPU)
    pub const MAGAZINE_CAPACITY: usize = 64;

    /// Number of colors for cache line optimization
    pub const NUM_COLORS: usize = 8;

    /// Large object threshold (use mmap directly)
    pub const LARGE_OBJECT_THRESHOLD: usize = 256 * 1024; // 256KB

    /// Huge page size (2MB)
    pub const HUGE_PAGE_2MB: usize = 2 * 1024 * 1024;

    /// Huge page size (1GB)
    pub const HUGE_PAGE_1GB: usize = 1024 * 1024 * 1024;

    /// Memory pressure warning threshold (80% of total)
    pub const MEMORY_PRESSURE_WARNING: f64 = 0.80;

    /// Memory pressure critical threshold (90% of total)
    pub const MEMORY_PRESSURE_CRITICAL: f64 = 0.90;

    /// Memory pressure emergency threshold (95% of total)
    pub const MEMORY_PRESSURE_EMERGENCY: f64 = 0.95;

    /// Maximum number of stack frames to capture for leak detection
    pub const MAX_STACK_FRAMES: usize = 32;

    /// Memory guard pattern for corruption detection
    pub const GUARD_PATTERN: u64 = 0xDEADBEEFCAFEBABE;

    /// Default arena size
    pub const DEFAULT_ARENA_SIZE: usize = 64 * 1024; // 64KB

    /// Default memory limit
    pub const DEFAULT_MEMORY_LIMIT: u64 = 1024 * 1024 * 1024; // 1GB
}

/// Memory context identifier
///
/// Provides type-safe identification for memory contexts with validation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryContextId(String);

impl MemoryContextId {
    /// Creates a new memory context ID with validation
    pub fn new(id: impl Into<String>) -> Result<Self, MemoryError> {
        let id = id.into();

        if id.trim().is_empty() {
            return Err(MemoryError::InvalidConfiguration {
                field: "context_id".to_string(),
                reason: "Memory context ID cannot be empty".to_string(),
            });
        }

        if id.len() > 255 {
            return Err(MemoryError::InvalidConfiguration {
                field: "context_id".to_string(),
                reason: "Memory context ID too long (max 255 characters)".to_string(),
            });
        }

        // Check for valid characters
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || "_-:.".contains(c)) {
            return Err(MemoryError::InvalidConfiguration {
                field: "context_id".to_string(),
                reason: "Memory context ID contains invalid characters".to_string(),
            });
        }

        Ok(Self(id))
    }

    /// Generates a new unique memory context ID
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Returns the context ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MemoryContextId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Allocation identifier for tracking
///
/// Unique identifier for individual memory allocations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AllocationId(u64);

impl AllocationId {
    /// Creates a new allocation ID
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Generates a new unique allocation ID
    pub fn generate() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Returns the allocation ID value
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for AllocationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Memory allocation source component
///
/// Identifies which database component requested a memory allocation
/// for tracking and debugging purposes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AllocationSource {
    /// Storage engine allocation
    Storage {
        component: String,
        operation: String,
    },
    /// Query execution allocation
    Query {
        query_id: String,
        operation: String,
    },
    /// Index operation allocation
    Index {
        index_name: String,
        operation: String,
    },
    /// Transaction management allocation
    Transaction {
        txn_id: String,
        operation: String,
    },
    /// Buffer pool allocation
    BufferPool {
        pool_id: String,
        operation: String,
    },
    /// WAL (Write-Ahead Log) allocation
    Wal {
        segment_id: String,
        operation: String,
    },
    /// Replication system allocation
    Replication {
        replica_id: String,
        operation: String,
    },
    /// Network communication allocation
    Network {
        connection_id: String,
        operation: String,
    },
    /// Catalog system allocation
    Catalog {
        object_type: String,
        operation: String,
    },
    /// Administrative operation allocation
    Admin {
        admin_operation: String,
    },
    /// Unknown or system allocation
    Unknown,
}

impl Default for AllocationSource {
    fn default() -> Self {
        Self::Unknown
    }
}

impl fmt::Display for AllocationSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AllocationSource::Storage { component, operation } => {
                write!(f, "Storage[{}:{}]", component, operation)
            }
            AllocationSource::Query { query_id, operation } => {
                write!(f, "Query[{}:{}]", query_id, operation)
            }
            AllocationSource::Index { index_name, operation } => {
                write!(f, "Index[{}:{}]", index_name, operation)
            }
            AllocationSource::Transaction { txn_id, operation } => {
                write!(f, "Transaction[{}:{}]", txn_id, operation)
            }
            AllocationSource::BufferPool { pool_id, operation } => {
                write!(f, "BufferPool[{}:{}]", pool_id, operation)
            }
            AllocationSource::Wal { segment_id, operation } => {
                write!(f, "WAL[{}:{}]", segment_id, operation)
            }
            AllocationSource::Replication { replica_id, operation } => {
                write!(f, "Replication[{}:{}]", replica_id, operation)
            }
            AllocationSource::Network { connection_id, operation } => {
                write!(f, "Network[{}:{}]", connection_id, operation)
            }
            AllocationSource::Catalog { object_type, operation } => {
                write!(f, "Catalog[{}:{}]", object_type, operation)
            }
            AllocationSource::Admin { admin_operation } => {
                write!(f, "Admin[{}]", admin_operation)
            }
            AllocationSource::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Memory context types for different allocation patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextType {
    /// Global top-level context
    TopLevel,
    /// Per-connection context
    Connection,
    /// Per-transaction context
    Transaction,
    /// Per-query context
    Query,
    /// Per-statement context
    Statement,
    /// Per-operator context (join, sort, etc.)
    Operator,
    /// Temporary context for short-lived allocations
    Temporary,
    /// Cache context for cached data
    Cache,
    /// Index context for index operations
    Index,
    /// Buffer context for I/O buffers
    Buffer,
    /// Custom context type
    Custom(String),
}

impl Default for ContextType {
    fn default() -> Self {
        Self::TopLevel
    }
}

impl fmt::Display for ContextType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContextType::TopLevel => write!(f, "TopLevel"),
            ContextType::Connection => write!(f, "Connection"),
            ContextType::Transaction => write!(f, "Transaction"),
            ContextType::Query => write!(f, "Query"),
            ContextType::Statement => write!(f, "Statement"),
            ContextType::Operator => write!(f, "Operator"),
            ContextType::Temporary => write!(f, "Temporary"),
            ContextType::Cache => write!(f, "Cache"),
            ContextType::Index => write!(f, "Index"),
            ContextType::Buffer => write!(f, "Buffer"),
            ContextType::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

/// Memory pressure levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MemoryPressureLevel {
    /// Normal memory usage
    Normal,
    /// Memory usage approaching limits (warning level)
    Warning,
    /// High memory usage (critical level)
    Critical,
    /// Emergency memory usage (immediate action required)
    Emergency,
}

impl Default for MemoryPressureLevel {
    fn default() -> Self {
        Self::Normal
    }
}

impl fmt::Display for MemoryPressureLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryPressureLevel::Normal => write!(f, "Normal"),
            MemoryPressureLevel::Warning => write!(f, "Warning"),
            MemoryPressureLevel::Critical => write!(f, "Critical"),
            MemoryPressureLevel::Emergency => write!(f, "Emergency"),
        }
    }
}

/// Allocator type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllocatorType {
    /// Slab allocator for small fixed-size allocations
    Slab,
    /// Arena allocator for bump allocation
    Arena,
    /// Large object allocator for huge allocations
    LargeObject,
    /// System allocator fallback
    System,
}

impl fmt::Display for AllocatorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AllocatorType::Slab => write!(f, "Slab"),
            AllocatorType::Arena => write!(f, "Arena"),
            AllocatorType::LargeObject => write!(f, "LargeObject"),
            AllocatorType::System => write!(f, "System"),
        }
    }
}

/// Memory zone types for specialized allocation patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneType {
    /// Normal zone for general allocations
    Normal,
    /// DMA zone for device I/O
    Dma,
    /// High memory zone
    HighMem,
    /// Movable zone for compaction
    Movable,
    /// Custom zone type
    Custom,
}

impl Default for ZoneType {
    fn default() -> Self {
        Self::Normal
    }
}

/// Huge page type for large allocations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HugePageType {
    /// No huge pages
    None,
    /// 2MB huge pages
    Page2MB,
    /// 1GB huge pages
    Page1GB,
}

impl Default for HugePageType {
    fn default() -> Self {
        Self::None
    }
}

/// Memory allocation statistics
///
/// Comprehensive statistics for memory allocation tracking and monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Total bytes allocated since start
    pub total_allocated: u64,
    /// Total bytes freed since start
    pub total_freed: u64,
    /// Current bytes in use
    pub bytes_in_use: u64,
    /// Number of active allocations
    pub allocation_count: u64,
    /// Peak memory usage
    pub peak_usage: u64,
    /// Number of total allocations
    pub total_allocations: u64,
    /// Number of total deallocations
    pub total_deallocations: u64,
    /// Fragmentation ratio (0.0 to 1.0)
    pub fragmentation_ratio: f64,
    /// Allocations per second
    pub allocations_per_sec: f64,
    /// Average allocation size
    pub avg_allocation_size: u64,
    /// Largest allocation size
    pub largest_allocation: u64,
    /// Smallest allocation size
    pub smallest_allocation: u64,
    /// Memory overhead percentage
    pub overhead_percentage: f64,
    /// Cache hit ratio (for cached allocators)
    pub cache_hit_ratio: f64,
    /// Last updated timestamp
    pub last_updated: SystemTime,
}

impl Default for MemoryStats {
    fn default() -> Self {
        Self {
            total_allocated: 0,
            total_freed: 0,
            bytes_in_use: 0,
            allocation_count: 0,
            peak_usage: 0,
            total_allocations: 0,
            total_deallocations: 0,
            fragmentation_ratio: 0.0,
            allocations_per_sec: 0.0,
            avg_allocation_size: 0,
            largest_allocation: 0,
            smallest_allocation: u64::MAX,
            overhead_percentage: 0.0,
            cache_hit_ratio: 0.0,
            last_updated: SystemTime::now(),
        }
    }
}

/// Slab allocator configuration
///
/// Configuration parameters for the slab allocator including size classes,
/// caching behavior, and performance optimizations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlabConfig {
    /// Number of size classes
    pub num_size_classes: usize,
    /// Magazine capacity per thread
    pub magazine_capacity: usize,
    /// Maximum object size for slab allocation
    pub max_slab_size: usize,
    /// Size of each slab
    pub slab_size: usize,
    /// Enable thread-local caching
    pub enable_thread_caching: bool,
    /// Enable cache coloring for better performance
    pub enable_coloring: bool,
    /// Number of colors for cache optimization
    pub color_count: usize,
    /// Enable statistics collection
    pub enable_statistics: bool,
    /// Enable debugging features
    pub enable_debugging: bool,
    /// Minimum object size
    pub min_object_size: usize,
    /// Size class growth factor
    pub size_class_factor: f64,
    /// Magazine exchange threshold
    pub magazine_exchange_threshold: usize,
}

impl Default for SlabConfig {
    fn default() -> Self {
        Self {
            num_size_classes: constants::NUM_SIZE_CLASSES,
            magazine_capacity: constants::MAGAZINE_CAPACITY,
            max_slab_size: constants::MAX_SLAB_SIZE,
            slab_size: constants::SLAB_SIZE,
            enable_thread_caching: true,
            enable_coloring: true,
            color_count: constants::NUM_COLORS,
            enable_statistics: true,
            enable_debugging: false,
            min_object_size: constants::MIN_ALLOC_SIZE,
            size_class_factor: 1.25,
            magazine_exchange_threshold: 32,
        }
    }
}

/// Arena allocator configuration
///
/// Configuration for arena-based bump allocators used for
/// per-query and per-transaction memory contexts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaConfig {
    /// Default arena size
    pub default_arena_size: usize,
    /// Maximum arena size
    pub max_arena_size: usize,
    /// Arena size growth factor
    pub growth_factor: f64,
    /// Maximum number of arenas per context
    pub max_arenas_per_context: usize,
    /// Enable memory mapping for large arenas
    pub enable_mmap: bool,
    /// Enable huge pages
    pub enable_huge_pages: bool,
    /// Huge page type preference
    pub huge_page_type: HugePageType,
    /// Enable statistics collection
    pub enable_statistics: bool,
    /// Enable memory zeroing
    pub enable_zeroing: bool,
    /// Alignment for allocations
    pub default_alignment: usize,
}

impl Default for ArenaConfig {
    fn default() -> Self {
        Self {
            default_arena_size: constants::DEFAULT_ARENA_SIZE,
            max_arena_size: 64 * 1024 * 1024, // 64MB
            growth_factor: 2.0,
            max_arenas_per_context: 16,
            enable_mmap: true,
            enable_huge_pages: false,
            huge_page_type: HugePageType::None,
            enable_statistics: true,
            enable_zeroing: false,
            default_alignment: 8,
        }
    }
}

/// Large object allocator configuration
///
/// Configuration for allocations that exceed slab allocator limits
/// and require direct memory mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeObjectConfig {
    /// Threshold size for large objects
    pub threshold_size: usize,
    /// Enable memory mapping
    pub enable_mmap: bool,
    /// Enable huge pages for large objects
    pub enable_huge_pages: bool,
    /// Preferred huge page type
    pub huge_page_type: HugePageType,
    /// Enable memory prefaulting
    pub enable_prefault: bool,
    /// Enable memory advice (MADV_*)
    pub enable_memory_advice: bool,
    /// Memory advice flags
    pub memory_advice: Vec<String>,
    /// Enable statistics collection
    pub enable_statistics: bool,
    /// Enable object tracking
    pub enable_tracking: bool,
}

impl Default for LargeObjectConfig {
    fn default() -> Self {
        Self {
            threshold_size: constants::LARGE_OBJECT_THRESHOLD,
            enable_mmap: true,
            enable_huge_pages: false,
            huge_page_type: HugePageType::Page2MB,
            enable_prefault: false,
            enable_memory_advice: true,
            memory_advice: vec!["MADV_NORMAL".to_string()],
            enable_statistics: true,
            enable_tracking: true,
        }
    }
}

/// Memory pressure monitoring configuration
///
/// Configuration for monitoring memory usage and triggering
/// pressure callbacks when thresholds are exceeded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureConfig {
    /// Warning threshold (ratio of total memory)
    pub warning_threshold: f64,
    /// Critical threshold (ratio of total memory)
    pub critical_threshold: f64,
    /// Emergency threshold (ratio of total memory)
    pub emergency_threshold: f64,
    /// Enable pressure monitoring
    pub enable_monitoring: bool,
    /// Check interval for pressure monitoring
    pub check_interval: Duration,
    /// Maximum number of pressure events to retain
    pub max_pressure_events: usize,
    /// Enable callback invocation
    pub enable_callbacks: bool,
    /// Enable automatic memory release
    pub enable_auto_release: bool,
    /// Memory release percentage during pressure
    pub release_percentage: f64,
    /// Pressure calculation method
    pub calculation_method: PressureCalculationMethod,
}

impl Default for PressureConfig {
    fn default() -> Self {
        Self {
            warning_threshold: constants::MEMORY_PRESSURE_WARNING,
            critical_threshold: constants::MEMORY_PRESSURE_CRITICAL,
            emergency_threshold: constants::MEMORY_PRESSURE_EMERGENCY,
            enable_monitoring: true,
            check_interval: Duration::from_secs(5),
            max_pressure_events: 1000,
            enable_callbacks: true,
            enable_auto_release: true,
            release_percentage: 0.1, // Release 10% of memory during pressure
            calculation_method: PressureCalculationMethod::Rss,
        }
    }
}

/// Memory pressure calculation methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PressureCalculationMethod {
    /// Based on resident set size
    Rss,
    /// Based on virtual memory size
    Virtual,
    /// Based on committed memory
    Committed,
    /// Custom calculation
    Custom,
}

/// Memory debugging configuration
///
/// Configuration for memory debugging features including
/// leak detection, corruption detection, and profiling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    /// Enable memory leak detection
    pub enable_leak_detection: bool,
    /// Enable corruption detection with guard patterns
    pub enable_guards: bool,
    /// Enable use-after-free detection
    pub enable_uaf_detection: bool,
    /// Enable double-free detection
    pub enable_double_free_detection: bool,
    /// Enable stack trace capture
    pub enable_stack_traces: bool,
    /// Maximum stack frames to capture
    pub max_stack_frames: usize,
    /// Enable allocation tracking
    pub enable_tracking: bool,
    /// Enable profiling
    pub enable_profiling: bool,
    /// Profiling sample rate (0.0 to 1.0)
    pub profiling_sample_rate: f64,
    /// Enable component-wise statistics
    pub enable_component_stats: bool,
    /// Memory pattern to use for freed memory
    pub free_pattern: u8,
    /// Memory pattern to use for allocated memory
    pub alloc_pattern: u8,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            enable_leak_detection: false,
            enable_guards: false,
            enable_uaf_detection: false,
            enable_double_free_detection: false,
            enable_stack_traces: false,
            max_stack_frames: constants::MAX_STACK_FRAMES,
            enable_tracking: false,
            enable_profiling: false,
            profiling_sample_rate: 0.01, // 1% sampling
            enable_component_stats: true,
            free_pattern: 0xDD, // "Dead" pattern
            alloc_pattern: 0xCC, // "Clean" pattern
        }
    }
}

/// Memory zone configuration
///
/// Configuration for memory zones that provide specialized
/// allocation patterns and isolation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneConfig {
    /// Zone name
    pub name: String,
    /// Zone type
    pub zone_type: ZoneType,
    /// Zone size
    pub size: usize,
    /// Enable zone isolation
    pub enable_isolation: bool,
    /// Enable zone statistics
    pub enable_statistics: bool,
    /// Zone allocation policy
    pub allocation_policy: ZoneAllocationPolicy,
    /// Zone memory advice
    pub memory_advice: Vec<String>,
}

/// Zone allocation policies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneAllocationPolicy {
    /// First-fit allocation
    FirstFit,
    /// Best-fit allocation
    BestFit,
    /// Worst-fit allocation
    WorstFit,
    /// Buddy system allocation
    Buddy,
}

impl Default for ZoneAllocationPolicy {
    fn default() -> Self {
        Self::FirstFit
    }
}

/// Component memory statistics
///
/// Per-component memory usage statistics for tracking
/// which parts of the system are consuming memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMemoryStats {
    /// Total allocations for this component
    pub allocations: u64,
    /// Total deallocations for this component
    pub deallocations: u64,
    /// Total bytes allocated
    pub bytes_allocated: u64,
    /// Total bytes deallocated
    pub bytes_deallocated: u64,
    /// Current active allocations
    pub active_allocations: u64,
    /// Current active bytes
    pub active_bytes: u64,
    /// Peak allocations
    pub peak_allocations: u64,
    /// Peak bytes
    pub peak_bytes: u64,
    /// Average allocation size
    pub avg_allocation_size: f64,
    /// Allocation rate (per second)
    pub allocation_rate: f64,
    /// Last activity timestamp
    pub last_activity: SystemTime,
}

impl ComponentMemoryStats {
    /// Creates a new component memory statistics instance
    pub fn new() -> Self {
        Self {
            allocations: 0,
            deallocations: 0,
            bytes_allocated: 0,
            bytes_deallocated: 0,
            active_allocations: 0,
            active_bytes: 0,
            peak_allocations: 0,
            peak_bytes: 0,
            avg_allocation_size: 0.0,
            allocation_rate: 0.0,
            last_activity: SystemTime::now(),
        }
    }

    /// Updates statistics after allocation
    pub fn record_allocation(&mut self, size: u64) {
        self.allocations += 1;
        self.bytes_allocated += size;
        self.active_allocations += 1;
        self.active_bytes += size;

        if self.active_allocations > self.peak_allocations {
            self.peak_allocations = self.active_allocations;
        }
        if self.active_bytes > self.peak_bytes {
            self.peak_bytes = self.active_bytes;
        }

        self.avg_allocation_size = self.bytes_allocated as f64 / self.allocations as f64;
        self.last_activity = SystemTime::now();
    }

    /// Updates statistics after deallocation
    pub fn record_deallocation(&mut self, size: u64) {
        self.deallocations += 1;
        self.bytes_deallocated += size;
        self.active_allocations = self.active_allocations.saturating_sub(1);
        self.active_bytes = self.active_bytes.saturating_sub(size);
        self.last_activity = SystemTime::now();
    }
}

impl Default for ComponentMemoryStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory leak report
///
/// Information about detected memory leaks including
/// allocation details and stack traces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakReport {
    /// Memory address of the leak
    pub address: usize,
    /// Size of the leaked allocation
    pub size: usize,
    /// Source component that allocated the memory
    pub source: AllocationSource,
    /// When the allocation was made
    pub allocated_at: SystemTime,
    /// Stack trace at allocation time
    pub stack_trace: String,
    /// Age of the allocation
    pub age: Duration,
    /// Allocation ID
    pub allocation_id: AllocationId,
}

/// Memory pressure event information
///
/// Details about memory pressure events including trigger
/// conditions and response actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPressureEvent {
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Pressure level that triggered the event
    pub level: MemoryPressureLevel,
    /// Total memory available
    pub total_memory: u64,
    /// Memory currently in use
    pub used_memory: u64,
    /// Memory available for allocation
    pub available_memory: u64,
    /// Number of pressure callbacks invoked
    pub callbacks_invoked: usize,
    /// Total bytes freed by callbacks
    pub bytes_freed: u64,
    /// Duration of pressure event
    pub duration: Duration,
    /// Whether emergency actions were taken
    pub emergency_actions_taken: bool,
}

/// Memory context statistics
///
/// Statistics for individual memory contexts including
/// usage patterns and lifecycle information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContextStats {
    /// Context identifier
    pub context_id: MemoryContextId,
    /// Context type
    pub context_type: ContextType,
    /// Current bytes allocated
    pub bytes_allocated: usize,
    /// Peak memory usage
    pub peak_usage: usize,
    /// Number of allocations made
    pub allocations: u64,
    /// Number of context resets
    pub resets: u64,
    /// Age of the context
    pub age: Duration,
    /// Number of child contexts
    pub child_count: usize,
    /// Whether context is active
    pub is_active: bool,
    /// Parent context ID (if any)
    pub parent_id: Option<MemoryContextId>,
    /// Memory limit for this context
    pub memory_limit: Option<usize>,
}

/// Utility functions for type validation and conversion
impl MemoryContextId {
    /// Validates that a string can be used as a context ID
    pub fn validate(id: &str) -> Result<(), MemoryError> {
        if id.trim().is_empty() {
            return Err(MemoryError::InvalidConfiguration {
                field: "context_id".to_string(),
                reason: "Memory context ID cannot be empty".to_string(),
            });
        }

        if id.len() > 255 {
            return Err(MemoryError::InvalidConfiguration {
                field: "context_id".to_string(),
                reason: "Memory context ID too long (max 255 characters)".to_string(),
            });
        }

        if !id.chars().all(|c| c.is_ascii_alphanumeric() || "_-:.".contains(c)) {
            return Err(MemoryError::InvalidConfiguration {
                field: "context_id".to_string(),
                reason: "Memory context ID contains invalid characters".to_string(),
            });
        }

        Ok(())
    }
}

/// Allocation size classification
pub fn classify_allocation_size(size: usize) -> AllocatorType {
    if size <= constants::MAX_SLAB_SIZE {
        AllocatorType::Slab
    } else if size < constants::LARGE_OBJECT_THRESHOLD {
        AllocatorType::System
    } else {
        AllocatorType::LargeObject
    }
}

/// Check if size is valid for allocation
pub fn validate_allocation_size(size: usize) -> Result<(), MemoryError> {
    if size == 0 {
        return Err(MemoryError::InvalidSize {
            size,
            reason: "Allocation size cannot be zero".to_string(),
        });
    }

    if size > constants::HUGE_PAGE_1GB {
        return Err(MemoryError::InvalidSize {
            size,
            reason: "Allocation size exceeds maximum limit".to_string(),
        });
    }

    Ok(())
}

/// Check if alignment is valid
pub fn validate_alignment(alignment: usize) -> Result<(), MemoryError> {
    if alignment == 0 {
        return Err(MemoryError::InvalidAlignment {
            alignment,
            reason: "Alignment cannot be zero".to_string(),
        });
    }

    if !alignment.is_power_of_two() {
        return Err(MemoryError::InvalidAlignment {
            alignment,
            reason: "Alignment must be a power of two".to_string(),
        });
    }

    if alignment > 4096 {
        return Err(MemoryError::InvalidAlignment {
            alignment,
            reason: "Alignment exceeds maximum supported value".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_context_id_validation() {
        // Valid IDs
        assert!(MemoryContextId::new("valid_context").is_ok());
        assert!(MemoryContextId::new("context-123").is_ok());
        assert!(MemoryContextId::new("query:executor.001").is_ok());

        // Invalid IDs
        assert!(MemoryContextId::new("").is_err()); // Empty
        assert!(MemoryContextId::new("   ").is_err()); // Whitespace only
        assert!(MemoryContextId::new("context with spaces").is_err()); // Spaces
        assert!(MemoryContextId::new("context@invalid").is_err()); // Invalid chars

        // Too long
        let long_id = "a".repeat(256);
        assert!(MemoryContextId::new(long_id).is_err());
    }

    #[test]
    fn test_allocation_id_generation() {
        let id1 = AllocationId::generate();
        let id2 = AllocationId::generate();
        assert_ne!(id1, id2);
        assert!(id1.value() > 0);
        assert!(id2.value() > 0);
    }

    #[test]
    fn test_allocation_source_display() {
        let _source = AllocationSource::Query {
            query_id: "SELECT_001".to_string(),
            operation: "hash_join".to_string(),
        };
        assert_eq!(source.to_string(), "Query[SELECT_001:hash_join]");

        let unknown = AllocationSource::Unknown;
        assert_eq!(unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_memory_pressure_level_ordering() {
        assert!(MemoryPressureLevel::Normal < MemoryPressureLevel::Warning);
        assert!(MemoryPressureLevel::Warning < MemoryPressureLevel::Critical);
        assert!(MemoryPressureLevel::Critical < MemoryPressureLevel::Emergency);
    }

    #[test]
    fn test_allocation_size_classification() {
        assert_eq!(classify_allocation_size(1024), AllocatorType::Slab);
        assert_eq!(classify_allocation_size(constants::MAX_SLAB_SIZE), AllocatorType::Slab);
        assert_eq!(classify_allocation_size(constants::MAX_SLAB_SIZE + 1), AllocatorType::System);
        assert_eq!(classify_allocation_size(constants::LARGE_OBJECT_THRESHOLD), AllocatorType::LargeObject);
    }

    #[test]
    fn test_allocation_size_validation() {
        assert!(validate_allocation_size(1024).is_ok());
        assert!(validate_allocation_size(0).is_err()); // Zero size
        assert!(validate_allocation_size(constants::HUGE_PAGE_1GB + 1).is_err()); // Too large
    }

    #[test]
    fn test_alignment_validation() {
        assert!(validate_alignment(8).is_ok()); // Valid power of 2
        assert!(validate_alignment(16).is_ok()); // Valid power of 2
        assert!(validate_alignment(0).is_err()); // Zero alignment
        assert!(validate_alignment(3).is_err()); // Not power of 2
        assert!(validate_alignment(8192).is_err()); // Too large
    }

    #[test]
    fn test_component_memory_stats() {
        let mut stats = ComponentMemoryStats::new();
        assert_eq!(stats.allocations, 0);
        assert_eq!(stats.active_allocations, 0);

        stats.record_allocation(1024);
        assert_eq!(stats.allocations, 1);
        assert_eq!(stats.active_allocations, 1);
        assert_eq!(stats.bytes_allocated, 1024);
        assert_eq!(stats.active_bytes, 1024);
        assert_eq!(stats.peak_allocations, 1);
        assert_eq!(stats.peak_bytes, 1024);

        stats.record_deallocation(1024);
        assert_eq!(stats.deallocations, 1);
        assert_eq!(stats.active_allocations, 0);
        assert_eq!(stats.bytes_deallocated, 1024);
        assert_eq!(stats.active_bytes, 0);
    }

    #[test]
    fn test_context_type_display() {
        assert_eq!(ContextType::Query.to_string(), "Query");
        assert_eq!(ContextType::Transaction.to_string(), "Transaction");
        assert_eq!(ContextType::Custom("test".to_string()).to_string(), "Custom(test)");
    }

    #[test]
    fn test_memory_pressure_level_default() {
        let level: MemoryPressureLevel = Default::default();
        assert_eq!(level, MemoryPressureLevel::Normal);
    }

    #[test]
    fn test_slab_config_default() {
        let config = SlabConfig::default();
        assert_eq!(config.num_size_classes, constants::NUM_SIZE_CLASSES);
        assert_eq!(config.magazine_capacity, constants::MAGAZINE_CAPACITY);
        assert_eq!(config.max_slab_size, constants::MAX_SLAB_SIZE);
        assert!(config.enable_thread_caching);
        assert!(config.enable_coloring);
    }
}
