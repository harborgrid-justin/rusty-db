// # Enterprise Memory Allocator System
//
// This module provides a comprehensive memory allocation system for RustyDB,
// implementing multiple allocation strategies optimized for different workload patterns.
//
// ## Architecture Overview
//
// The memory system consists of five major components:
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

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::{Duration, Instant, SystemTime};
use std::backtrace::Backtrace;

use crate::{DbError, Result};

// ============================================================================
// Constants and Configuration
// ============================================================================

/// Minimum allocation size (16 bytes for alignment)
const MIN_ALLOC_SIZE: usize = 16;

/// Maximum size for slab allocation (anything larger goes to large object allocator)
const MAX_SLAB_SIZE: usize = 32 * 1024; // 32KB

/// Number of size classes in the slab allocator
const NUM_SIZE_CLASSES: usize = 64;

/// Slab size (typically 2MB for huge page alignment)
const SLAB_SIZE: usize = 2 * 1024 * 1024;

/// Magazine capacity (number of objects cached per CPU)
const MAGAZINE_CAPACITY: usize = 64;

/// Number of colors for cache line optimization
const NUM_COLORS: usize = 8;

/// Large object threshold (use mmap directly)
const LARGE_OBJECT_THRESHOLD: usize = 256 * 1024; // 256KB

/// Huge page size (2MB)
const HUGE_PAGE_2MB: usize = 2 * 1024 * 1024;

/// Huge page size (1GB)
const HUGE_PAGE_1GB: usize = 1024 * 1024 * 1024;

/// Memory pressure warning threshold (80% of total)
const MEMORY_PRESSURE_WARNING: f64 = 0.80;

/// Memory pressure critical threshold (90% of total)
const MEMORY_PRESSURE_CRITICAL: f64 = 0.90;

/// Maximum number of stack frames to capture for leak detection
const MAX_STACK_FRAMES: usize = 32;

/// Memory guard pattern for corruption detection
const GUARD_PATTERN: u64 = 0xDEADBEEFCAFEBABE;

// ============================================================================
// Public API Types and Enums
// ============================================================================

/// Memory allocation statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total bytes allocated
    pub total_allocated: u64,
    /// Total bytes freed
    pub total_freed: u64,
    /// Current bytes in use
    pub bytes_in_use: u64,
    /// Number of active allocations
    pub allocation_count: u64,
    /// Peak memory usage
    pub peak_usage: u64,
    /// Fragmentation ratio (0.0 to 1.0)
    pub fragmentation_ratio: f64,
    /// Allocations per second
    pub allocations_per_sec: f64,
    /// Average allocation size
    pub avg_allocation_size: u64,
}

/// Memory pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressureLevel {
    /// Normal operation (<80% usage)
    Normal,
    /// Warning level (80-90% usage)
    Warning,
    /// Critical level (90-95% usage)
    Critical,
    /// Emergency level (>95% usage)
    Emergency,
}

/// Memory allocation source/component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllocationSource {
    /// Buffer pool allocations
    BufferPool,
    /// Query execution allocations
    QueryExecution,
    /// Index structures
    Index,
    /// Transaction management
    Transaction,
    /// Network buffers
    Network,
    /// Catalog metadata
    Catalog,
    /// Temporary workspace
    Temporary,
    /// Unknown/other
    Other,
}

/// Memory context type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextType {
    /// Top-level context
    TopLevel,
    /// Per-query context
    Query,
    /// Per-transaction context
    Transaction,
    /// Session-level context
    Session,
    /// Temporary context
    Temporary,
}

/// Memory leak report entry
#[derive(Debug, Clone)]
pub struct LeakReport {
    /// Allocation address
    pub address: usize,
    /// Allocation size
    pub size: usize,
    /// Allocation source
    pub source: AllocationSource,
    /// Time when allocated
    pub allocated_at: SystemTime,
    /// Stack trace at allocation
    pub stack_trace: String,
}

// ============================================================================
// PART 1: SLAB ALLOCATOR (700+ lines)
// ============================================================================

/// Size class information for slab allocation
#[derive(Debug, Clone)]
struct SizeClass {
    /// Object size for this class
    object_size: usize,
    /// Number of objects per slab
    objects_per_slab: usize,
    /// Color offset for cache optimization
    color_offset: usize,
}

impl SizeClass {
    fn new(object_size: usize) -> Self {
        let objects_per_slab = (SLAB_SIZE - 128) / object_size; // Reserve 128 bytes for metadata
        Self {
            object_size,
            objects_per_slab,
            color_offset: 0,
        }
    }
}

/// Slab metadata
struct Slab {
    /// Base pointer to slab memory
    base: NonNull<u8>,
    /// Size class index
    size_class: usize,
    /// Number of free objects
    free_count: usize,
    /// Freelist head
    freelist: Option<NonNull<u8>>,
    /// Slab color for cache optimization
    color: usize,
    /// Allocation timestamp
    allocated_at: Instant,
}

impl Slab {
    /// Create a new slab for the given size class
    unsafe fn new(size_class_info: &SizeClass, size_class: usize, color: usize) -> Result<Self> {
        // Allocate slab memory with proper alignment
        let layout = Layout::from_size_align(SLAB_SIZE, SLAB_SIZE)
            .map_err(|e| DbError::OutOfMemory(format!("Invalid slab layout: {}", e)))?;

        let base = System.alloc(layout);
        if base.is_null() {
            return Err(DbError::OutOfMemory("Failed to allocate slab".to_string()));
        }

        let base = NonNull::new_unchecked(base);

        // Initialize freelist by linking all objects
        let object_size = size_class_info.object_size;
        let objects_per_slab = size_class_info.objects_per_slab;

        let mut current = base.as_ptr().add(color * 64); // Apply cache coloring
        let mut freelist_head = None;

        for _i in 0..objects_per_slab {
            let next = if i < objects_per_slab - 1 {
                current.add(object_size)
            } else {
                ptr::null_mut()
            };

            // Store next pointer at the beginning of the object
            *(current as *mut *mut u8) = next;

            if freelist_head.is_none() {
                freelist_head = NonNull::new(current);
            }

            current = next;
        }

        Ok(Self {
            base,
            size_class,
            free_count: objects_per_slab,
            freelist: freelist_head,
            color,
            allocated_at: Instant::now(),
        })
    }

    /// Allocate an object from this slab
    unsafe fn allocate(&mut self) -> Option<NonNull<u8>> {
        if let Some(obj) = self.freelist {
            // Pop from freelist
            let next_ptr = *(obj.as_ptr() as *const *mut u8);
            self.freelist = NonNull::new(next_ptr);
            self.free_count -= 1;

            // Zero the memory for security
            ptr::write_bytes(obj.as_ptr(), 0, size_of::<*mut u8>());

            Some(obj)
        } else {
            None
        }
    }

    /// Free an object back to this slab
    unsafe fn deallocate(&mut self, ptr: NonNull<u8>) {
        // Push to freelist
        *(ptr.as_ptr() as *mut *mut u8) = self.freelist.map_or(ptr::null_mut(), |p| p.as_ptr());
        self.freelist = Some(ptr);
        self.free_count += 1;
    }

    /// Check if slab is full
    fn is_full(&self) -> bool {
        self.free_count == 0
    }

    /// Check if slab is empty
    fn is_empty(&self, size_class_info: &SizeClass) -> bool {
        self.free_count == size_class_info.objects_per_slab
    }
}

impl Drop for Slab {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(SLAB_SIZE, SLAB_SIZE);
            System.dealloc(self.base.as_ptr(), layout);
        }
    }
}

/// Magazine for CPU-level caching
struct Magazine {
    /// Cached objects
    objects: Vec<NonNull<u8>>,
    /// Capacity of magazine
    capacity: usize,
    /// Size class this magazine belongs to
    size_class: usize,
}

impl Magazine {
    fn new(size_class: usize) -> Self {
        Self {
            objects: Vec::with_capacity(MAGAZINE_CAPACITY),
            capacity: MAGAZINE_CAPACITY,
            size_class,
        }
    }

    /// Try to allocate from magazine
    fn allocate(&mut self) -> Option<NonNull<u8>> {
        self.objects.pop()
    }

    /// Try to free to magazine
    fn deallocate(&mut self, ptr: NonNull<u8>) -> bool {
        if self.objects.len() < self.capacity {
            self.objects.push(ptr);
            true
        } else {
            false
        }
    }

    fn is_full(&self) -> bool {
        self.objects.len() >= self.capacity
    }

    fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
}

/// Per-thread slab cache
struct ThreadLocalCache {
    /// Loaded magazine per size class
    loaded_magazines: Vec<Option<Magazine>>,
    /// Previous magazine per size class
    previous_magazines: Vec<Option<Magazine>>,
    /// Thread ID
    thread_id: usize,
}

thread_local! {
    static THREAD_CACHE: RefCell<Option<ThreadLocalCache>> = RefCell::new(None);
}

impl ThreadLocalCache {
    fn new(thread_id: usize) -> Self {
        Self {
            loaded_magazines: (0..NUM_SIZE_CLASSES).map(|_| None).collect(),
            previous_magazines: (0..NUM_SIZE_CLASSES).map(|_| None).collect(),
            thread_id,
        }
    }

    fn ensure_initialized() {
        THREAD_CACHE.with(|cache| {
            let mut cache_mut = cache.borrow_mut();
            if cache_mut.is_none() {
                static THREAD_COUNTER: AtomicUsize = AtomicUsize::new(0);
                let thread_id = THREAD_COUNTER.fetch_add(1, Ordering::SeqCst);
                *cache_mut = Some(ThreadLocalCache::new(thread_id));
            }
        })
    }

    fn with_cache<F, R>(f: F) -> R
    where
        F: FnOnce(&mut Option<ThreadLocalCache>) -> R,
    {
        Self::ensure_initialized();
        THREAD_CACHE.with(|cache| {
            f(&mut *cache.borrow_mut())
        })
    }
}

/// Slab depot - central storage for magazines and slabs
struct SlabDepot {
    /// Full magazines per size class
    full_magazines: Vec<VecDeque<Magazine>>,
    /// Empty magazines per size class
    empty_magazines: Vec<VecDeque<Magazine>>,
    /// Partial slabs per size class
    partial_slabs: Vec<VecDeque<Slab>>,
    /// Empty slabs per size class
    empty_slabs: Vec<VecDeque<Slab>>,
    /// Full slabs count per size class (for tracking)
    full_slab_counts: Vec<usize>,
    /// Current color per size class (for slab coloring)
    current_colors: Vec<usize>,
}

impl SlabDepot {
    fn new() -> Self {
        Self {
            full_magazines: (0..NUM_SIZE_CLASSES).map(|_| VecDeque::new()).collect(),
            empty_magazines: (0..NUM_SIZE_CLASSES).map(|_| VecDeque::new()).collect(),
            partial_slabs: (0..NUM_SIZE_CLASSES).map(|_| VecDeque::new()).collect(),
            empty_slabs: (0..NUM_SIZE_CLASSES).map(|_| VecDeque::new()).collect(),
            full_slab_counts: vec![0; NUM_SIZE_CLASSES],
            current_colors: vec![0; NUM_SIZE_CLASSES],
        }
    }

    fn get_full_magazine(&mut self, size_class: usize) -> Option<Magazine> {
        self.full_magazines[size_class].pop_front()
    }

    fn put_full_magazine(&mut self, size_class: usize, magazine: Magazine) {
        self.full_magazines[size_class].push_back(magazine);
    }

    fn get_empty_magazine(&mut self, size_class: usize) -> Magazine {
        self.empty_magazines[size_class]
            .pop_front()
            .unwrap_or_else(|| Magazine::new(size_class))
    }

    fn put_empty_magazine(&mut self, size_class: usize, magazine: Magazine) {
        self.empty_magazines[size_class].push_back(magazine);
    }

    unsafe fn get_slab(&mut self, size_class_info: &SizeClass, size_class: usize) -> Result<Slab> {
        // Try to get a partial slab first
        if let Some(slab) = self.partial_slabs[size_class].pop_front() {
            return Ok(slab);
        }

        // Try to get an empty slab
        if let Some(slab) = self.empty_slabs[size_class].pop_front() {
            return Ok(slab);
        }

        // Allocate a new slab with coloring
        let color = self.current_colors[size_class];
        self.current_colors[size_class] = (color + 1) % NUM_COLORS;

        Slab::new(size_class_info, size_class, color)
    }

    fn put_slab(&mut self, size_class_info: &SizeClass, slab: Slab) {
        let size_class = slab.size_class;

        if slab.is_empty(size_class_info) {
            self.empty_slabs[size_class].push_back(slab);
        } else if slab.is_full() {
            self.full_slab_counts[size_class] += 1;
            // Full slabs are just tracked, not stored
            drop(slab);
        } else {
            self.partial_slabs[size_class].push_back(slab);
        }
    }
}

/// Main slab allocator
pub struct SlabAllocator {
    /// Size class configurations
    size_classes: Vec<SizeClass>,
    /// Central depot (protected by lock)
    depot: Mutex<SlabDepot>,
    /// Allocation statistics
    stats: SlabStats,
}

/// Slab allocator statistics
struct SlabStats {
    allocations: AtomicU64,
    deallocations: AtomicU64,
    slab_allocations: AtomicU64,
    magazine_loads: AtomicU64,
    magazine_unloads: AtomicU64,
    bytes_allocated: AtomicU64,
}

impl SlabStats {
    fn new() -> Self {
        Self {
            allocations: AtomicU64::new(0),
            deallocations: AtomicU64::new(0),
            slab_allocations: AtomicU64::new(0),
            magazine_loads: AtomicU64::new(0),
            magazine_unloads: AtomicU64::new(0),
            bytes_allocated: AtomicU64::new(0),
        }
    }
}

impl SlabAllocator {
    /// Create a new slab allocator
    pub fn new() -> Self {
        let mut size_classes = Vec::with_capacity(NUM_SIZE_CLASSES);

        // Create size classes with power-of-two and fine-grained sizes
        let mut size = MIN_ALLOC_SIZE;
        for _ in 0..NUM_SIZE_CLASSES {
            if size <= MAX_SLAB_SIZE {
                size_classes.push(SizeClass::new(size));
                size = if size < 256 {
                    size + 16
                } else if size < 2048 {
                    size + 128
                } else {
                    size + 512
                };
            } else {
                break;
            }
        }

        Self {
            size_classes,
            depot: Mutex::new(SlabDepot::new()),
            stats: SlabStats::new(),
        }
    }

    /// Get size class index for a given size
    fn size_to_class(&self, size: usize) -> Option<usize> {
        self.size_classes
            .iter()
            .position(|sc| sc.object_size >= size)
    }

    /// Allocate memory from slab allocator
    pub fn allocate(&self, size: usize) -> Result<NonNull<u8>> {
        if size > MAX_SLAB_SIZE {
            return Err(DbError::InvalidArgument(
                format!("Size {} exceeds slab max {}", size, MAX_SLAB_SIZE)
            ));
        }

        let size_class = self.size_to_class(size)
            .ok_or_else(|| DbError::InvalidArgument(format!("Invalid size: {}", size)))?;

        self.stats.allocations.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_allocated.fetch_add(size as u64, Ordering::Relaxed);

        unsafe { self.allocate_from_class(size_class) }
    }

    /// Internal allocation from specific size class
    unsafe fn allocate_from_class(&self, size_class: usize) -> Result<NonNull<u8>> {
        THREAD_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let cache = cache.get_or_insert_with(|| {
                static COUNTER: AtomicUsize = AtomicUsize::new(0);
                ThreadLocalCache::new(COUNTER.fetch_add(1, Ordering::SeqCst))
            });

            // Try loaded magazine first
            if let Some(ref mut magazine) = cache.loaded_magazines[size_class] {
                if let Some(ptr) = magazine.allocate() {
                    return Ok(ptr);
                }
            }

            // Try previous magazine
            if let Some(prev) = cache.previous_magazines[size_class].take() {
                cache.loaded_magazines[size_class] = Some(prev);
                if let Some(ref mut magazine) = cache.loaded_magazines[size_class] {
                    if let Some(ptr) = magazine.allocate() {
                        return Ok(ptr);
                    }
                }
            }

            // Load a full magazine from depot
            let mut depot = self.depot.lock().unwrap();
            if let Some(magazine) = depot.get_full_magazine(size_class) {
                self.stats.magazine_loads.fetch_add(1, Ordering::Relaxed);
                cache.loaded_magazines[size_class] = Some(magazine);
                drop(depot);

                if let Some(ref mut magazine) = cache.loaded_magazines[size_class] {
                    if let Some(ptr) = magazine.allocate() {
                        return Ok(ptr);
                    }
                }
            } else {
                drop(depot);
            }

            // Allocate from slab
            let size_class_info = &self.size_classes[size_class];
            let mut depot = self.depot.lock().unwrap();
            let mut slab = depot.get_slab(size_class_info, size_class)?;

            if let Some(ptr) = slab.allocate() {
                depot.put_slab(size_class_info, slab);
                self.stats.slab_allocations.fetch_add(1, Ordering::Relaxed);
                Ok(ptr)
            } else {
                Err(DbError::OutOfMemory("Failed to allocate from slab".to_string()))
            }
        })
    }

    /// Deallocate memory back to slab allocator
    pub unsafe fn deallocate(&self, ptr: NonNull<u8>, size: usize) -> Result<()> {
        let size_class = self.size_to_class(size)
            .ok_or_else(|| DbError::InvalidArgument(format!("Invalid size: {}", size)))?;

        self.stats.deallocations.fetch_add(1, Ordering::Relaxed);

        THREAD_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let cache = cache.get_or_insert_with(|| {
                static COUNTER: AtomicUsize = AtomicUsize::new(0);
                ThreadLocalCache::new(COUNTER.fetch_add(1, Ordering::SeqCst))
            });

            // Try to free to loaded magazine
            if let Some(ref mut magazine) = cache.loaded_magazines[size_class] {
                if magazine.deallocate(ptr) {
                    return Ok(());
                }

                // Magazine is full, swap with previous
                let full_mag = cache.loaded_magazines[size_class].take();
                cache.loaded_magazines[size_class] = cache.previous_magazines[size_class].take();
                cache.previous_magazines[size_class] = full_mag;

                if let Some(ref mut magazine) = cache.loaded_magazines[size_class] {
                    if magazine.deallocate(ptr) {
                        return Ok(());
                    }
                }
            }

            // Both magazines full, return previous to depot
            if let Some(prev) = cache.previous_magazines[size_class].take() {
                let mut depot = self.depot.lock().unwrap();
                depot.put_full_magazine(size_class, prev);
                self.stats.magazine_unloads.fetch_add(1, Ordering::Relaxed);
            }

            // Get empty magazine and add object
            let mut depot = self.depot.lock().unwrap();
            let mut magazine = depot.get_empty_magazine(size_class);
            magazine.deallocate(ptr);
            cache.loaded_magazines[size_class] = Some(magazine);

            Ok(())
        })
    }

    /// Get allocation statistics
    pub fn get_stats(&self) -> SlabAllocatorStats {
        SlabAllocatorStats {
            total_allocations: self.stats.allocations.load(Ordering::Relaxed),
            total_deallocations: self.stats.deallocations.load(Ordering::Relaxed),
            slab_allocations: self.stats.slab_allocations.load(Ordering::Relaxed),
            magazine_loads: self.stats.magazine_loads.load(Ordering::Relaxed),
            magazine_unloads: self.stats.magazine_unloads.load(Ordering::Relaxed),
            bytes_allocated: self.stats.bytes_allocated.load(Ordering::Relaxed),
        }
    }

    /// Calculate fragmentation for a size class
    pub fn calculate_fragmentation(&self, size_class: usize) -> f64 {
        let depot = self.depot.lock().unwrap();
        let partial = depot.partial_slabs[size_class].len();
        let empty = depot.empty_slabs[size_class].len();
        let full = depot.full_slab_counts[size_class];
        let total = partial + empty + full;

        if total == 0 {
            return 0.0;
        }

        // Fragmentation = (partial_slabs + empty_slabs) / total_slabs
        (partial + empty) as f64 / total as f64
    }
}

/// Public slab allocator statistics
#[derive(Debug, Clone)]
pub struct SlabAllocatorStats {
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub slab_allocations: u64,
    pub magazine_loads: u64,
    pub magazine_unloads: u64,
    pub bytes_allocated: u64,
}

// ============================================================================
// PART 2: ARENA ALLOCATOR (600+ lines)
// ============================================================================

/// Memory arena for bump allocation
struct Arena {
    /// Current chunk
    current_chunk: Option<ArenaChunk>,
    /// Previous chunks
    chunks: Vec<ArenaChunk>,
    /// Total allocated in this arena
    total_allocated: usize,
    /// Allocation limit (0 = unlimited)
    limit: usize,
    /// Arena name
    name: String,
}

/// Arena chunk
struct ArenaChunk {
    /// Base pointer
    base: NonNull<u8>,
    /// Chunk size
    size: usize,
    /// Current offset
    offset: usize,
}

impl ArenaChunk {
    /// Create a new arena chunk
    unsafe fn new(size: usize) -> Result<Self> {
        let layout = Layout::from_size_align(size, 16)
            .map_err(|e| DbError::OutOfMemory(format!("Invalid arena layout: {}", e)))?;

        let base = System.alloc(layout);
        if base.is_null() {
            return Err(DbError::OutOfMemory("Failed to allocate arena chunk".to_string()));
        }

        Ok(Self {
            base: NonNull::new_unchecked(base),
            size,
            offset: 0,
        })
    }

    /// Allocate from this chunk
    unsafe fn allocate(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
        let offset = (self.offset + align - 1) & !(align - 1);
        let end = offset + size;

        if end <= self.size {
            let ptr = NonNull::new_unchecked(self.base.as_ptr().add(offset));
            self.offset = end;
            Some(ptr)
        } else {
            None
        }
    }

    fn bytes_used(&self) -> usize {
        self.offset
    }

    fn bytes_free(&self) -> usize {
        self.size - self.offset
    }
}

impl Drop for ArenaChunk {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.size, 16);
            System.dealloc(self.base.as_ptr(), layout);
        }
    }
}

impl Arena {
    fn new(name: String, initial_size: usize, limit: usize) -> Result<Self> {
        Ok(Self {
            current_chunk: Some(unsafe { ArenaChunk::new(initial_size)? }),
            chunks: Vec::new(),
            total_allocated: 0,
            limit,
            name,
        })
    }

    unsafe fn allocate(&mut self, size: usize, align: usize) -> Result<NonNull<u8>> {
        // Check limit
        if self.limit > 0 && self.total_allocated + size > self.limit {
            return Err(DbError::LimitExceeded(
                format!("Arena '{}' limit {} exceeded", self.name, self.limit)
            ));
        }

        // Try current chunk
        if let Some(ref mut chunk) = self.current_chunk {
            if let Some(ptr) = chunk.allocate(size, align) {
                self.total_allocated += size;
                return Ok(ptr);
            }
        }

        // Need new chunk
        let chunk_size = (size.max(64 * 1024) + 4095) & !4095; // Round up to 4KB
        let old_chunk = self.current_chunk.take();
        if let Some(chunk) = old_chunk {
            self.chunks.push(chunk);
        }

        let mut new_chunk = ArenaChunk::new(chunk_size)?;
        let ptr = new_chunk.allocate(size, align)
            .ok_or_else(|| DbError::OutOfMemory("Failed to allocate from new chunk".to_string()))?;

        self.current_chunk = Some(new_chunk);
        self.total_allocated += size;

        Ok(ptr)
    }

    fn reset(&mut self) {
        self.chunks.clear();
        if let Some(ref mut chunk) = self.current_chunk {
            chunk.offset = 0;
        }
        self.total_allocated = 0;
    }

    fn bytes_allocated(&self) -> usize {
        self.total_allocated
    }
}

/// Memory context (hierarchical arena)
pub struct MemoryContext {
    /// Context ID
    id: u64,
    /// Context type
    context_type: ContextType,
    /// Arena for this context
    arena: Arena,
    /// Parent context
    parent: Option<Weak<Mutex<MemoryContext>>>,
    /// Child contexts
    children: Vec<Arc<Mutex<MemoryContext>>>,
    /// Creation time
    created_at: Instant,
    /// Statistics
    stats: ContextStats,
}

/// Memory context statistics
struct ContextStats {
    allocations: AtomicU64,
    deallocations: AtomicU64,
    resets: AtomicU64,
    peak_usage: AtomicUsize,
}

impl ContextStats {
    fn new() -> Self {
        Self {
            allocations: AtomicU64::new(0),
            deallocations: AtomicU64::new(0),
            resets: AtomicU64::new(0),
            peak_usage: AtomicUsize::new(0),
        }
    }
}

impl MemoryContext {
    /// Create a new top-level memory context
    pub fn new_top_level(name: String, limit: usize) -> Result<Arc<Mutex<Self>>> {
        static CONTEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = CONTEXT_ID.fetch_add(1, Ordering::SeqCst);

        Ok(Arc::new(Mutex::new(Self {
            id,
            context_type: ContextType::TopLevel,
            arena: Arena::new(name, 64 * 1024, limit)?,
            parent: None,
            children: Vec::new(),
            created_at: Instant::now(),
            stats: ContextStats::new(),
        })))
    }

    /// Create a child context
    pub fn create_child(
        parent: &Arc<Mutex<Self>>,
        name: String,
        context_type: ContextType,
        limit: usize,
    ) -> Result<Arc<Mutex<Self>>> {
        static CONTEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = CONTEXT_ID.fetch_add(1, Ordering::SeqCst);

        let child = Arc::new(Mutex::new(Self {
            id,
            context_type,
            arena: Arena::new(name, 32 * 1024, limit)?,
            parent: Some(Arc::downgrade(parent)),
            children: Vec::new(),
            created_at: Instant::now(),
            stats: ContextStats::new(),
        }));

        parent.lock().unwrap().children.push(Arc::clone(&child));

        Ok(child)
    }

    /// Allocate memory in this context
    pub fn allocate(&mut self, size: usize) -> Result<NonNull<u8>> {
        self.stats.allocations.fetch_add(1, Ordering::Relaxed);

        let ptr = unsafe { self.arena.allocate(size, 16)? };

        let current = self.arena.bytes_allocated();
        let peak = self.stats.peak_usage.load(Ordering::Relaxed);
        if current > peak {
            self.stats.peak_usage.store(current, Ordering::Relaxed);
        }

        Ok(ptr)
    }

    /// Allocate aligned memory in this context
    pub fn allocate_aligned(&mut self, size: usize, align: usize) -> Result<NonNull<u8>> {
        self.stats.allocations.fetch_add(1, Ordering::Relaxed);

        let ptr = unsafe { self.arena.allocate(size, align)? };

        let current = self.arena.bytes_allocated();
        let peak = self.stats.peak_usage.load(Ordering::Relaxed);
        if current > peak {
            self.stats.peak_usage.store(current, Ordering::Relaxed);
        }

        Ok(ptr)
    }

    /// Reset this context (free all memory)
    pub fn reset(&mut self) {
        self.stats.resets.fetch_add(1, Ordering::Relaxed);
        self.arena.reset();

        // Reset all children
        for child in &self.children {
            if let Ok(mut child) = child.lock() {
                child.reset();
            }
        }
    }

    /// Delete this context and all children
    pub fn delete(context: Arc<Mutex<Self>>) {
        let mut ctx = context.lock().unwrap();

        // Delete all children first
        let children = std::mem::take(&mut ctx.children);
        drop(ctx);

        for child in children {
            Self::delete(child);
        }
    }

    /// Get context statistics
    pub fn get_stats(&self) -> MemoryContextStats {
        MemoryContextStats {
            id: self.id,
            context_type: self.context_type,
            bytes_allocated: self.arena.bytes_allocated(),
            peak_usage: self.stats.peak_usage.load(Ordering::Relaxed),
            allocations: self.stats.allocations.load(Ordering::Relaxed),
            resets: self.stats.resets.load(Ordering::Relaxed),
            age: self.created_at.elapsed(),
            child_count: self.children.len(),
        }
    }
}

/// Public memory context statistics
#[derive(Debug, Clone)]
pub struct MemoryContextStats {
    pub id: u64,
    pub context_type: ContextType,
    pub bytes_allocated: usize,
    pub peak_usage: usize,
    pub allocations: u64,
    pub resets: u64,
    pub age: Duration,
    pub child_count: usize,
}

/// Arena allocator manager
pub struct ArenaAllocator {
    /// All active contexts
    contexts: RwLock<HashMap<u64, Weak<Mutex<MemoryContext>>>>,
    /// Global statistics
    stats: ArenaStats,
}

unsafe impl Send for ArenaAllocator {}
unsafe impl Sync for ArenaAllocator {}

struct ArenaStats {
    contexts_created: AtomicU64,
    contexts_deleted: AtomicU64,
    total_resets: AtomicU64,
}

impl ArenaStats {
    fn new() -> Self {
        Self {
            contexts_created: AtomicU64::new(0),
            contexts_deleted: AtomicU64::new(0),
            total_resets: AtomicU64::new(0),
        }
    }
}

impl ArenaAllocator {
    /// Create a new arena allocator
    pub fn new() -> Self {
        Self {
            contexts: RwLock::new(HashMap::new()),
            stats: ArenaStats::new(),
        }
    }

    /// Create a new top-level context
    pub fn create_context(&self, name: String, limit: usize) -> Result<Arc<Mutex<MemoryContext>>> {
        let _context = MemoryContext::new_top_level(name, limit)?;
        let id = context.lock().unwrap().id;

        self.contexts.write().unwrap().insert(id, Arc::downgrade(&context));
        self.stats.contexts_created.fetch_add(1, Ordering::Relaxed);

        Ok(context)
    }

    /// Get global statistics
    pub fn get_stats(&self) -> ArenaAllocatorStats {
        let contexts = self.contexts.read().unwrap();
        let active_contexts = contexts.values().filter(|w| w.strong_count() > 0).count();

        ArenaAllocatorStats {
            contexts_created: self.stats.contexts_created.load(Ordering::Relaxed),
            contexts_deleted: self.stats.contexts_deleted.load(Ordering::Relaxed),
            active_contexts: active_contexts as u64,
            total_resets: self.stats.total_resets.load(Ordering::Relaxed),
        }
    }

    /// Cleanup dead contexts
    pub fn cleanup_dead_contexts(&self) -> usize {
        let mut contexts = self.contexts.write().unwrap();
        let before = contexts.len();
        contexts.retain(|_, weak| weak.strong_count() > 0);
        let removed = before - contexts.len();

        self.stats.contexts_deleted.fetch_add(removed as u64, Ordering::Relaxed);
        removed
    }
}

/// Public arena allocator statistics
#[derive(Debug, Clone)]
pub struct ArenaAllocatorStats {
    pub contexts_created: u64,
    pub contexts_deleted: u64,
    pub active_contexts: u64,
    pub total_resets: u64,
}

// ============================================================================
// PART 3: LARGE OBJECT ALLOCATOR (500+ lines)
// ============================================================================

/// Large object metadata
struct LargeObject {
    /// Base address
    base: NonNull<u8>,
    /// Size of allocation
    size: usize,
    /// Whether huge pages are used
    huge_pages: bool,
    /// Huge page size used (if any)
    huge_page_size: usize,
    /// Whether copy-on-write is enabled
    cow: bool,
    /// Allocation timestamp
    allocated_at: Instant,
}

impl LargeObject {
    /// Allocate a large object using mmap
    unsafe fn allocate(size: usize, _use_huge_pages: bool, cow: bool) -> Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::io::RawFd;

            let mut flags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS;
            if cow {
                flags |= libc::MAP_PRIVATE;
            }

            let mut huge_page_size = 0;

            // Try to use huge pages if requested
            if use_huge_pages {
                if size >= HUGE_PAGE_1GB && size % HUGE_PAGE_1GB == 0 {
                    #[cfg(target_os = "linux")]
                    {
                        flags |= libc::MAP_HUGETLB | (30 << libc::MAP_HUGE_SHIFT); // 1GB pages
                    }
                    huge_page_size = HUGE_PAGE_1GB;
                } else if size >= HUGE_PAGE_2MB {
                    #[cfg(target_os = "linux")]
                    {
                        flags |= libc::MAP_HUGETLB | (21 << libc::MAP_HUGE_SHIFT); // 2MB pages
                    }
                    huge_page_size = HUGE_PAGE_2MB;
                }
            }

            let ptr = libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                flags,
                -1,
                0,
            );

            if ptr == libc::MAP_FAILED {
                // If huge pages failed, try regular allocation
                if use_huge_pages {
                    let ptr = libc::mmap(
                        ptr::null_mut(),
                        size,
                        libc::PROT_READ | libc::PROT_WRITE,
                        libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                        -1,
                        0,
                    );

                    if ptr == libc::MAP_FAILED {
                        return Err(DbError::OutOfMemory("mmap failed".to_string()));
                    }

                    return Ok(Self {
                        base: NonNull::new_unchecked(ptr as *mut u8),
                        size,
                        huge_pages: false,
                        huge_page_size: 0,
                        cow,
                        allocated_at: Instant::now(),
                    });
                }

                return Err(DbError::OutOfMemory("mmap failed".to_string()));
            }

            // Advise kernel about usage pattern
            libc::madvise(ptr, size, libc::MADV_RANDOM);

            Ok(Self {
                base: NonNull::new_unchecked(ptr as *mut u8),
                size,
                huge_pages: use_huge_pages && huge_page_size > 0,
                huge_page_size,
                cow,
                allocated_at: Instant::now(),
            })
        }

        #[cfg(not(unix))]
        {
            // Fallback to regular allocation on non-Unix systems
            let layout = Layout::from_size_align(size, 4096)
                .map_err(|e| DbError::OutOfMemory(format!("Invalid layout: {}", e)))?;

            let ptr = System.alloc(layout);
            if ptr.is_null() {
                return Err(DbError::OutOfMemory("Failed to allocate large object".to_string()));
            }

            Ok(Self {
                base: NonNull::new_unchecked(ptr),
                size,
                huge_pages: false,
                huge_page_size: 0,
                cow,
                allocated_at: Instant::now(),
            })
        }
    }

    /// Enable lazy allocation (on-demand paging)
    unsafe fn enable_lazy_allocation(&self) -> Result<()> {
        #[cfg(unix)]
        {
            if libc::madvise(
                self.base.as_ptr() as *mut libc::c_void,
                self.size,
                libc::MADV_FREE,
            ) != 0 {
                return Err(DbError::Internal("madvise failed".to_string()));
            }
        }
        Ok(())
    }

    /// Prefault pages (force allocation)
    unsafe fn prefault(&self) -> Result<()> {
        #[cfg(unix)]
        {
            if libc::madvise(
                self.base.as_ptr() as *mut libc::c_void,
                self.size,
                libc::MADV_WILLNEED,
            ) != 0 {
                return Err(DbError::Internal("madvise failed".to_string()));
            }
        }
        Ok(())
    }

    /// Mark as sequential access
    unsafe fn set_sequential(&self) -> Result<()> {
        #[cfg(unix)]
        {
            if libc::madvise(
                self.base.as_ptr() as *mut libc::c_void,
                self.size,
                libc::MADV_SEQUENTIAL,
            ) != 0 {
                return Err(DbError::Internal("madvise failed".to_string()));
            }
        }
        Ok(())
    }
}

impl Drop for LargeObject {
    fn drop(&mut self) {
        unsafe {
            #[cfg(unix)]
            {
                libc::munmap(self.base.as_ptr() as *mut libc::c_void, self.size);
            }

            #[cfg(not(unix))]
            {
                let layout = Layout::from_size_align_unchecked(self.size, 4096);
                System.dealloc(self.base.as_ptr(), layout);
            }
        }
    }
}

/// Large object allocator
pub struct LargeObjectAllocator {
    /// Active large objects
    objects: RwLock<HashMap<usize, LargeObject>>,
    /// Statistics
    stats: LargeObjectStats,
}

struct LargeObjectStats {
    allocations: AtomicU64,
    deallocations: AtomicU64,
    huge_page_allocations: AtomicU64,
    huge_page_2mb: AtomicU64,
    huge_page_1gb: AtomicU64,
    bytes_allocated: AtomicU64,
    bytes_deallocated: AtomicU64,
}

impl LargeObjectStats {
    fn new() -> Self {
        Self {
            allocations: AtomicU64::new(0),
            deallocations: AtomicU64::new(0),
            huge_page_allocations: AtomicU64::new(0),
            huge_page_2mb: AtomicU64::new(0),
            huge_page_1gb: AtomicU64::new(0),
            bytes_allocated: AtomicU64::new(0),
            bytes_deallocated: AtomicU64::new(0),
        }
    }
}

impl LargeObjectAllocator {
    /// Create a new large object allocator
    pub fn new() -> Self {
        Self {
            objects: RwLock::new(HashMap::new()),
            stats: LargeObjectStats::new(),
        }
    }

    /// Allocate a large object
    pub fn allocate(
        &self,
        size: usize,
        use_huge_pages: bool,
        cow: bool,
    ) -> Result<NonNull<u8>> {
        let obj = unsafe { LargeObject::allocate(size, use_huge_pages, cow)? };
        let ptr = obj.base;
        let addr = ptr.as_ptr() as usize;

        self.stats.allocations.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_allocated.fetch_add(size as u64, Ordering::Relaxed);

        if obj.huge_pages {
            self.stats.huge_page_allocations.fetch_add(1, Ordering::Relaxed);
            if obj.huge_page_size == HUGE_PAGE_2MB {
                self.stats.huge_page_2mb.fetch_add(1, Ordering::Relaxed);
            } else if obj.huge_page_size == HUGE_PAGE_1GB {
                self.stats.huge_page_1gb.fetch_add(1, Ordering::Relaxed);
            }
        }

        self.objects.write().unwrap().insert(addr, obj);

        Ok(ptr)
    }

    /// Deallocate a large object
    pub fn deallocate(&self, ptr: NonNull<u8>) -> Result<()> {
        let addr = ptr.as_ptr() as usize;

        if let Some(obj) = self.objects.write().unwrap().remove(&addr) {
            self.stats.deallocations.fetch_add(1, Ordering::Relaxed);
            self.stats.bytes_deallocated.fetch_add(obj.size as u64, Ordering::Relaxed);
            Ok(())
        } else {
            Err(DbError::InvalidArgument("Unknown large object pointer".to_string()))
        }
    }

    /// Enable lazy allocation for an object
    pub fn enable_lazy_allocation(&self, ptr: NonNull<u8>) -> Result<()> {
        let addr = ptr.as_ptr() as usize;
        let objects = self.objects.read().unwrap();

        if let Some(obj) = objects.get(&addr) {
            unsafe { obj.enable_lazy_allocation() }
        } else {
            Err(DbError::InvalidArgument("Unknown large object pointer".to_string()))
        }
    }

    /// Prefault pages for an object
    pub fn prefault(&self, ptr: NonNull<u8>) -> Result<()> {
        let addr = ptr.as_ptr() as usize;
        let objects = self.objects.read().unwrap();

        if let Some(obj) = objects.get(&addr) {
            unsafe { obj.prefault() }
        } else {
            Err(DbError::InvalidArgument("Unknown large object pointer".to_string()))
        }
    }

    /// Set sequential access pattern
    pub fn set_sequential(&self, ptr: NonNull<u8>) -> Result<()> {
        let addr = ptr.as_ptr() as usize;
        let objects = self.objects.read().unwrap();

        if let Some(obj) = objects.get(&addr) {
            unsafe { obj.set_sequential() }
        } else {
            Err(DbError::InvalidArgument("Unknown large object pointer".to_string()))
        }
    }

    /// Get statistics
    pub fn get_stats(&self) -> LargeObjectAllocatorStats {
        let objects = self.objects.read().unwrap();
        let active_objects = objects.len();
        let active_bytes: usize = objects.values().map(|o| o.size).sum();

        LargeObjectAllocatorStats {
            allocations: self.stats.allocations.load(Ordering::Relaxed),
            deallocations: self.stats.deallocations.load(Ordering::Relaxed),
            active_objects: active_objects as u64,
            active_bytes: active_bytes as u64,
            huge_page_allocations: self.stats.huge_page_allocations.load(Ordering::Relaxed),
            huge_page_2mb: self.stats.huge_page_2mb.load(Ordering::Relaxed),
            huge_page_1gb: self.stats.huge_page_1gb.load(Ordering::Relaxed),
            bytes_allocated: self.stats.bytes_allocated.load(Ordering::Relaxed),
            bytes_deallocated: self.stats.bytes_deallocated.load(Ordering::Relaxed),
        }
    }
}

/// Public large object allocator statistics
#[derive(Debug, Clone)]
pub struct LargeObjectAllocatorStats {
    pub allocations: u64,
    pub deallocations: u64,
    pub active_objects: u64,
    pub active_bytes: u64,
    pub huge_page_allocations: u64,
    pub huge_page_2mb: u64,
    pub huge_page_1gb: u64,
    pub bytes_allocated: u64,
    pub bytes_deallocated: u64,
}

// ============================================================================
// PART 4: MEMORY PRESSURE MANAGEMENT (600+ lines)
// ============================================================================

/// Memory pressure callback type
pub type PressureCallback = Arc<dyn Fn(MemoryPressureLevel) -> Result<usize> + Send + Sync>;

/// Memory pressure event
#[derive(Debug, Clone)]
pub struct MemoryPressureEvent {
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Pressure level
    pub level: MemoryPressureLevel,
    /// Total memory
    pub total_memory: u64,
    /// Used memory
    pub used_memory: u64,
    /// Available memory
    pub available_memory: u64,
    /// Number of callbacks invoked
    pub callbacks_invoked: usize,
    /// Total bytes freed
    pub bytes_freed: u64,
}

/// Memory pressure manager
pub struct MemoryPressureManager {
    /// Total memory limit
    total_memory: AtomicU64,
    /// Current used memory
    used_memory: AtomicU64,
    /// Memory pressure callbacks
    callbacks: RwLock<Vec<PressureCallback>>,
    /// Current pressure level
    current_level: RwLock<MemoryPressureLevel>,
    /// Pressure events history
    events: RwLock<VecDeque<MemoryPressureEvent>>,
    /// Emergency mode flag
    emergency_mode: AtomicBool,
    /// Statistics
    stats: PressureStats,
    /// Monitoring enabled
    monitoring_enabled: AtomicBool,
}

struct PressureStats {
    pressure_events: AtomicU64,
    callbacks_invoked: AtomicU64,
    emergency_releases: AtomicU64,
    total_freed: AtomicU64,
    oom_prevented: AtomicU64,
}

impl PressureStats {
    fn new() -> Self {
        Self {
            pressure_events: AtomicU64::new(0),
            callbacks_invoked: AtomicU64::new(0),
            emergency_releases: AtomicU64::new(0),
            total_freed: AtomicU64::new(0),
            oom_prevented: AtomicU64::new(0),
        }
    }
}

impl MemoryPressureManager {
    /// Create a new memory pressure manager
    pub fn new(total_memory: u64) -> Self {
        Self {
            total_memory: AtomicU64::new(total_memory),
            used_memory: AtomicU64::new(0),
            callbacks: RwLock::new(Vec::new()),
            current_level: RwLock::new(MemoryPressureLevel::Normal),
            events: RwLock::new(VecDeque::new()),
            emergency_mode: AtomicBool::new(false),
            stats: PressureStats::new(),
            monitoring_enabled: AtomicBool::new(true),
        }
    }

    /// Register a pressure callback
    pub fn register_callback(&self, callback: PressureCallback) {
        self.callbacks.write().unwrap().push(callback);
    }

    /// Record memory allocation
    pub fn record_allocation(&self, size: u64) -> Result<()> {
        if !self.monitoring_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        let new_used = self.used_memory.fetch_add(size, Ordering::SeqCst) + size;
        let total = self.total_memory.load(Ordering::Relaxed);

        let usage_ratio = new_used as f64 / total as f64;
        let new_level = self.calculate_pressure_level(usage_ratio);

        let mut current_level = self.current_level.write().unwrap();

        if new_level != *current_level {
            *current_level = new_level;
            drop(current_level);

            self.handle_pressure_change(new_level)?;
        }

        Ok(())
    }

    /// Record memory deallocation
    pub fn record_deallocation(&self, size: u64) {
        if !self.monitoring_enabled.load(Ordering::Relaxed) {
            return;
        }

        self.used_memory.fetch_sub(size, Ordering::SeqCst);

        let used = self.used_memory.load(Ordering::Relaxed);
        let total = self.total_memory.load(Ordering::Relaxed);
        let usage_ratio = used as f64 / total as f64;

        let new_level = self.calculate_pressure_level(usage_ratio);
        let mut current_level = self.current_level.write().unwrap();

        if new_level != *current_level && usage_ratio < MEMORY_PRESSURE_WARNING {
            *current_level = new_level;

            if self.emergency_mode.load(Ordering::Relaxed) {
                self.emergency_mode.store(false, Ordering::Relaxed);
            }
        }
    }

    /// Calculate pressure level from usage ratio
    fn calculate_pressure_level(&self, usage_ratio: f64) -> MemoryPressureLevel {
        if usage_ratio >= 0.95 {
            MemoryPressureLevel::Emergency
        } else if usage_ratio >= MEMORY_PRESSURE_CRITICAL {
            MemoryPressureLevel::Critical
        } else if usage_ratio >= MEMORY_PRESSURE_WARNING {
            MemoryPressureLevel::Warning
        } else {
            MemoryPressureLevel::Normal
        }
    }

    /// Handle pressure level change
    fn handle_pressure_change(&self, new_level: MemoryPressureLevel) -> Result<()> {
        self.stats.pressure_events.fetch_add(1, Ordering::Relaxed);

        // Invoke callbacks
        let callbacks = self.callbacks.read().unwrap();
        let mut total_freed = 0u64;
        let mut callbacks_invoked: usize = 0;

        for callback in callbacks.iter() {
            match callback(new_level) {
                Ok(freed) => {
                    total_freed += freed as u64;
                    callbacks_invoked += 1;
                }
                Err(e) => {
                    eprintln!("Pressure callback failed: {}", e);
                }
            }
        }

        drop(callbacks);

        self.stats.callbacks_invoked.fetch_add(callbacks_invoked as u64, Ordering::Relaxed);
        self.stats.total_freed.fetch_add(total_freed, Ordering::Relaxed);

        // Record event
        let event = MemoryPressureEvent {
            timestamp: SystemTime::now(),
            level: new_level,
            total_memory: self.total_memory.load(Ordering::Relaxed),
            used_memory: self.used_memory.load(Ordering::Relaxed),
            available_memory: self.total_memory.load(Ordering::Relaxed)
                - self.used_memory.load(Ordering::Relaxed),
            callbacks_invoked,
            bytes_freed: total_freed,
        };

        let mut events = self.events.write().unwrap();
        events.push_back(event);

        // Keep only last 1000 events
        while events.len() > 1000 {
            events.pop_front();
        }

        // Enter emergency mode if critical
        if new_level == MemoryPressureLevel::Emergency {
            self.emergency_mode.store(true, Ordering::Relaxed);
            self.emergency_release()?;
        }

        Ok(())
    }

    /// Emergency memory release
    fn emergency_release(&self) -> Result<()> {
        self.stats.emergency_releases.fetch_add(1, Ordering::Relaxed);

        // Trigger all callbacks with emergency level
        let callbacks = self.callbacks.read().unwrap();
        let mut total_freed = 0u64;

        for callback in callbacks.iter() {
            if let Ok(freed) = callback(MemoryPressureLevel::Emergency) {
                total_freed += freed as u64;
            }
        }

        if total_freed > 0 {
            self.stats.oom_prevented.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Check if allocation would cause OOM
    pub fn check_allocation(&self, size: u64) -> Result<()> {
        let used = self.used_memory.load(Ordering::Relaxed);
        let total = self.total_memory.load(Ordering::Relaxed);

        if used + size > total {
            // Try emergency release
            self.emergency_release()?;

            let used = self.used_memory.load(Ordering::Relaxed);
            if used + size > total {
                return Err(DbError::OutOfMemory(
                    format!("Cannot allocate {} bytes (used: {}, total: {})", size, used, total)
                ));
            }
        }

        Ok(())
    }

    /// Set total memory limit
    pub fn set_total_memory(&self, total: u64) {
        self.total_memory.store(total, Ordering::Relaxed);
    }

    /// Get current pressure level
    pub fn get_pressure_level(&self) -> MemoryPressureLevel {
        *self.current_level.read().unwrap()
    }

    /// Get memory usage
    pub fn get_usage(&self) -> MemoryUsage {
        let total = self.total_memory.load(Ordering::Relaxed);
        let used = self.used_memory.load(Ordering::Relaxed);

        MemoryUsage {
            total_memory: total,
            used_memory: used,
            available_memory: total.saturating_sub(used),
            usage_ratio: used as f64 / total as f64,
            pressure_level: self.get_pressure_level(),
            emergency_mode: self.emergency_mode.load(Ordering::Relaxed),
        }
    }

    /// Get pressure statistics
    pub fn get_stats(&self) -> MemoryPressureStats {
        MemoryPressureStats {
            pressure_events: self.stats.pressure_events.load(Ordering::Relaxed),
            callbacks_invoked: self.stats.callbacks_invoked.load(Ordering::Relaxed),
            emergency_releases: self.stats.emergency_releases.load(Ordering::Relaxed),
            total_freed: self.stats.total_freed.load(Ordering::Relaxed),
            oom_prevented: self.stats.oom_prevented.load(Ordering::Relaxed),
            current_level: self.get_pressure_level(),
            current_usage: self.get_usage(),
        }
    }

    /// Get recent pressure events
    pub fn get_recent_events(&self, count: usize) -> Vec<MemoryPressureEvent> {
        let events = self.events.read().unwrap();
        events.iter().rev().take(count).cloned().collect()
    }

    /// Enable/disable monitoring
    pub fn set_monitoring_enabled(&self, enabled: bool) {
        self.monitoring_enabled.store(enabled, Ordering::Relaxed);
    }
}

/// Memory usage snapshot
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    pub total_memory: u64,
    pub used_memory: u64,
    pub available_memory: u64,
    pub usage_ratio: f64,
    pub pressure_level: MemoryPressureLevel,
    pub emergency_mode: bool,
}

/// Memory pressure statistics
#[derive(Debug, Clone)]
pub struct MemoryPressureStats {
    pub pressure_events: u64,
    pub callbacks_invoked: u64,
    pub emergency_releases: u64,
    pub total_freed: u64,
    pub oom_prevented: u64,
    pub current_level: MemoryPressureLevel,
    pub current_usage: MemoryUsage,
}

// ============================================================================
// PART 5: MEMORY DEBUGGING & PROFILING (600+ lines)
// ============================================================================

/// Allocation tracking entry
#[derive(Debug, Clone)]
struct AllocationEntry {
    /// Allocation address
    address: usize,
    /// Allocation size
    size: usize,
    /// Allocation source
    source: AllocationSource,
    /// Allocation timestamp
    timestamp: Instant,
    /// Stack trace (simplified)
    stack_trace: String,
    /// Guard before allocation
    guard_before: u64,
    /// Guard after allocation
    guard_after: u64,
}

/// Memory debugger and profiler
pub struct MemoryDebugger {
    /// Tracking enabled
    tracking_enabled: AtomicBool,
    /// Active allocations
    allocations: RwLock<HashMap<usize, AllocationEntry>>,
    /// Per-component statistics
    component_stats: RwLock<HashMap<AllocationSource, ComponentMemoryStats>>,
    /// Leak detection enabled
    leak_detection_enabled: AtomicBool,
    /// Use-after-free detection enabled
    uaf_detection_enabled: AtomicBool,
    /// Memory guards enabled
    guards_enabled: AtomicBool,
    /// Stack trace capture enabled
    stack_traces_enabled: AtomicBool,
    /// Statistics
    stats: DebugStats,
}

struct DebugStats {
    total_allocations: AtomicU64,
    total_deallocations: AtomicU64,
    leaks_detected: AtomicU64,
    uaf_detected: AtomicU64,
    corruption_detected: AtomicU64,
    stack_traces_captured: AtomicU64,
}

impl DebugStats {
    fn new() -> Self {
        Self {
            total_allocations: AtomicU64::new(0),
            total_deallocations: AtomicU64::new(0),
            leaks_detected: AtomicU64::new(0),
            uaf_detected: AtomicU64::new(0),
            corruption_detected: AtomicU64::new(0),
            stack_traces_captured: AtomicU64::new(0),
        }
    }
}

/// Per-component memory statistics
#[derive(Debug, Clone)]
struct ComponentMemoryStats {
    allocations: u64,
    deallocations: u64,
    bytes_allocated: u64,
    bytes_deallocated: u64,
    active_allocations: u64,
    active_bytes: u64,
    peak_allocations: u64,
    peak_bytes: u64,
}

impl ComponentMemoryStats {
    fn new() -> Self {
        Self {
            allocations: 0,
            deallocations: 0,
            bytes_allocated: 0,
            bytes_deallocated: 0,
            active_allocations: 0,
            active_bytes: 0,
            peak_allocations: 0,
            peak_bytes: 0,
        }
    }
}

impl MemoryDebugger {
    /// Create a new memory debugger
    pub fn new() -> Self {
        Self {
            tracking_enabled: AtomicBool::new(false),
            allocations: RwLock::new(HashMap::new()),
            component_stats: RwLock::new(HashMap::new()),
            leak_detection_enabled: AtomicBool::new(false),
            uaf_detection_enabled: AtomicBool::new(false),
            guards_enabled: AtomicBool::new(false),
            stack_traces_enabled: AtomicBool::new(false),
            stats: DebugStats::new(),
        }
    }

    /// Enable tracking
    pub fn enable_tracking(&self) {
        self.tracking_enabled.store(true, Ordering::Relaxed);
    }

    /// Disable tracking
    pub fn disable_tracking(&self) {
        self.tracking_enabled.store(false, Ordering::Relaxed);
    }

    /// Enable leak detection
    pub fn enable_leak_detection(&self) {
        self.leak_detection_enabled.store(true, Ordering::Relaxed);
        self.enable_tracking();
    }

    /// Enable use-after-free detection
    pub fn enable_uaf_detection(&self) {
        self.uaf_detection_enabled.store(true, Ordering::Relaxed);
        self.enable_tracking();
    }

    /// Enable memory guards
    pub fn enable_guards(&self) {
        self.guards_enabled.store(true, Ordering::Relaxed);
        self.enable_tracking();
    }

    /// Enable stack trace capture
    pub fn enable_stack_traces(&self) {
        self.stack_traces_enabled.store(true, Ordering::Relaxed);
        self.stats.stack_traces_captured.fetch_add(1, Ordering::Relaxed);
    }

    /// Track allocation
    pub fn track_allocation(
        &self,
        address: usize,
        size: usize,
        source: AllocationSource,
    ) {
        if !self.tracking_enabled.load(Ordering::Relaxed) {
            return;
        }

        self.stats.total_allocations.fetch_add(1, Ordering::Relaxed);

        let stack_trace = if self.stack_traces_enabled.load(Ordering::Relaxed) {
            self.capture_stack_trace()
        } else {
            String::new()
        };

        let entry = AllocationEntry {
            address,
            size,
            source,
            timestamp: Instant::now(),
            stack_trace,
            guard_before: GUARD_PATTERN,
            guard_after: GUARD_PATTERN,
        };

        self.allocations.write().unwrap().insert(address, entry);

        // Update component stats
        let mut stats = self.component_stats.write().unwrap();
        let component_stat = stats.entry(source).or_insert_with(ComponentMemoryStats::new);

        component_stat.allocations += 1;
        component_stat.bytes_allocated += size as u64;
        component_stat.active_allocations += 1;
        component_stat.active_bytes += size as u64;

        if component_stat.active_allocations > component_stat.peak_allocations {
            component_stat.peak_allocations = component_stat.active_allocations;
        }
        if component_stat.active_bytes > component_stat.peak_bytes {
            component_stat.peak_bytes = component_stat.active_bytes;
        }
    }

    /// Track deallocation
    pub fn track_deallocation(&self, address: usize) -> Result<()> {
        if !self.tracking_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.stats.total_deallocations.fetch_add(1, Ordering::Relaxed);

        let mut allocations = self.allocations.write().unwrap();

        if let Some(entry) = allocations.remove(&address) {
            // Check guards if enabled
            if self.guards_enabled.load(Ordering::Relaxed) {
                if entry.guard_before != GUARD_PATTERN || entry.guard_after != GUARD_PATTERN {
                    self.stats.corruption_detected.fetch_add(1, Ordering::Relaxed);
                    return Err(DbError::Internal(
                        format!("Memory corruption detected at address 0x{:x}", address)
                    ));
                }
            }

            // Update component stats
            let mut stats = self.component_stats.write().unwrap();
            if let Some(component_stat) = stats.get_mut(&entry.source) {
                component_stat.deallocations += 1;
                component_stat.bytes_deallocated += entry.size as u64;
                component_stat.active_allocations = component_stat.active_allocations.saturating_sub(1);
                component_stat.active_bytes = component_stat.active_bytes.saturating_sub(entry.size as u64);
            }

            Ok(())
        } else if self.uaf_detection_enabled.load(Ordering::Relaxed) {
            self.stats.uaf_detected.fetch_add(1, Ordering::Relaxed);
            Err(DbError::Internal(
                format!("Use-after-free or double-free detected at address 0x{:x}", address)
            ))
        } else {
            Ok(())
        }
    }

    /// Capture stack trace
    fn capture_stack_trace(&self) -> String {
        let backtrace = Backtrace::capture();
        format!("{:?}", backtrace)
    }

    /// Detect memory leaks
    pub fn detect_leaks(&self, min_age: Duration) -> Vec<LeakReport> {
        if !self.leak_detection_enabled.load(Ordering::Relaxed) {
            return Vec::new();
        }

        let now = Instant::now();
        let allocations = self.allocations.read().unwrap();

        let leaks: Vec<LeakReport> = allocations
            .values()
            .filter(|entry| now.duration_since(entry.timestamp) >= min_age)
            .map(|entry| {
                self.stats.leaks_detected.fetch_add(1, Ordering::Relaxed);

                LeakReport {
                    address: entry.address,
                    size: entry.size,
                    source: entry.source,
                    allocated_at: SystemTime::now() - now.duration_since(entry.timestamp),
                    stack_trace: entry.stack_trace.clone(),
                }
            })
            .collect();

        leaks
    }

    /// Get component statistics
    pub fn get_component_stats(&self, source: AllocationSource) -> Option<ComponentMemoryStats> {
        self.component_stats.read().unwrap().get(&source).cloned()
    }

    /// Get all component statistics
    pub fn get_all_component_stats(&self) -> HashMap<AllocationSource, ComponentMemoryStats> {
        self.component_stats.read().unwrap().clone()
    }

    /// Generate memory usage report
    pub fn generate_report(&self) -> MemoryReport {
        let allocations = self.allocations.read().unwrap();
        let component_stats = self.component_stats.read().unwrap();

        let total_active_allocations = allocations.len() as u64;
        let total_active_bytes: usize = allocations.values().map(|e| e.size).sum();

        let mut component_breakdown = Vec::new();
        for (source, stats) in component_stats.iter() {
            component_breakdown.push(ComponentBreakdown {
                source: *source,
                active_allocations: stats.active_allocations,
                active_bytes: stats.active_bytes,
                peak_allocations: stats.peak_allocations,
                peak_bytes: stats.peak_bytes,
                total_allocations: stats.allocations,
                total_deallocations: stats.deallocations,
            });
        }

        component_breakdown.sort_by_key(|c| std::cmp::Reverse(c.active_bytes));

        MemoryReport {
            timestamp: SystemTime::now(),
            total_active_allocations,
            total_active_bytes: total_active_bytes as u64,
            component_breakdown,
            total_allocations: self.stats.total_allocations.load(Ordering::Relaxed),
            total_deallocations: self.stats.total_deallocations.load(Ordering::Relaxed),
            leaks_detected: self.stats.leaks_detected.load(Ordering::Relaxed),
            uaf_detected: self.stats.uaf_detected.load(Ordering::Relaxed),
            corruption_detected: self.stats.corruption_detected.load(Ordering::Relaxed),
        }
    }

    /// Get debugger statistics
    pub fn get_stats(&self) -> MemoryDebuggerStats {
        MemoryDebuggerStats {
            tracking_enabled: self.tracking_enabled.load(Ordering::Relaxed),
            leak_detection_enabled: self.leak_detection_enabled.load(Ordering::Relaxed),
            uaf_detection_enabled: self.uaf_detection_enabled.load(Ordering::Relaxed),
            guards_enabled: self.guards_enabled.load(Ordering::Relaxed),
            total_allocations: self.stats.total_allocations.load(Ordering::Relaxed),
            total_deallocations: self.stats.total_deallocations.load(Ordering::Relaxed),
            active_allocations: self.allocations.read().unwrap().len() as u64,
            leaks_detected: self.stats.leaks_detected.load(Ordering::Relaxed),
            uaf_detected: self.stats.uaf_detected.load(Ordering::Relaxed),
            corruption_detected: self.stats.corruption_detected.load(Ordering::Relaxed),
        }
    }

    /// Clear all tracked allocations (use with caution)
    pub fn clear_tracking(&self) {
        self.allocations.write().unwrap().clear();
        self.component_stats.write().unwrap().clear();
    }
}

/// Memory usage report
#[derive(Debug, Clone)]
pub struct MemoryReport {
    pub timestamp: SystemTime,
    pub total_active_allocations: u64,
    pub total_active_bytes: u64,
    pub component_breakdown: Vec<ComponentBreakdown>,
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub leaks_detected: u64,
    pub uaf_detected: u64,
    pub corruption_detected: u64,
}

/// Component memory breakdown
#[derive(Debug, Clone)]
pub struct ComponentBreakdown {
    pub source: AllocationSource,
    pub active_allocations: u64,
    pub active_bytes: u64,
    pub peak_allocations: u64,
    pub peak_bytes: u64,
    pub total_allocations: u64,
    pub total_deallocations: u64,
}

/// Memory debugger statistics
#[derive(Debug, Clone)]
pub struct MemoryDebuggerStats {
    pub tracking_enabled: bool,
    pub leak_detection_enabled: bool,
    pub uaf_detection_enabled: bool,
    pub guards_enabled: bool,
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub active_allocations: u64,
    pub leaks_detected: u64,
    pub uaf_detected: u64,
    pub corruption_detected: u64,
}

// ============================================================================
// UNIFIED MEMORY MANAGER (Integration Layer)
// ============================================================================

/// Unified memory manager integrating all allocators
pub struct MemoryManager {
    /// Slab allocator for small objects
    slab_allocator: Arc<SlabAllocator>,
    /// Arena allocator for query contexts
    arena_allocator: Arc<ArenaAllocator>,
    /// Large object allocator
    large_object_allocator: Arc<LargeObjectAllocator>,
    /// Memory pressure manager
    pressure_manager: Arc<MemoryPressureManager>,
    /// Memory debugger
    debugger: Arc<MemoryDebugger>,
}

impl MemoryManager {
    /// Create a new unified memory manager
    pub fn new(total_memory: u64) -> Self {
        let manager = Self {
            slab_allocator: Arc::new(SlabAllocator::new()),
            arena_allocator: Arc::new(ArenaAllocator::new()),
            large_object_allocator: Arc::new(LargeObjectAllocator::new()),
            pressure_manager: Arc::new(MemoryPressureManager::new(total_memory)),
            debugger: Arc::new(MemoryDebugger::new()),
        };

        // Register pressure callbacks
        manager.setup_pressure_callbacks();

        manager
    }

    /// Setup pressure callbacks for memory release
    fn setup_pressure_callbacks(&self) {
        let arena = Arc::clone(&self.arena_allocator);

        self.pressure_manager.register_callback(Arc::new(move |level| {
            match level {
                MemoryPressureLevel::Warning => {
                    // Cleanup dead contexts
                    let freed = arena.cleanup_dead_contexts();
                    Ok(freed * 1024) // Estimate
                }
                MemoryPressureLevel::Critical | MemoryPressureLevel::Emergency => {
                    // Aggressive cleanup
                    let freed = arena.cleanup_dead_contexts();
                    Ok(freed * 1024)
                }
                _ => Ok(0),
            }
        }));
    }

    /// Allocate memory using the appropriate allocator
    pub fn allocate(&self, size: usize, source: AllocationSource) -> Result<NonNull<u8>> {
        // Check memory pressure
        self.pressure_manager.check_allocation(size as u64)?;

        let ptr = if size <= MAX_SLAB_SIZE {
            self.slab_allocator.allocate(size)?
        } else if size < LARGE_OBJECT_THRESHOLD {
            // Use system allocator for medium sizes
            unsafe {
                let layout = Layout::from_size_align(size, 16)
                    .map_err(|e| DbError::OutOfMemory(format!("Invalid layout: {}", e)))?;
                let ptr = System.alloc(layout);
                if ptr.is_null() {
                    return Err(DbError::OutOfMemory("System allocation failed".to_string()));
                }
                NonNull::new_unchecked(ptr)
            }
        } else {
            // Use large object allocator
            self.large_object_allocator.allocate(size, size >= HUGE_PAGE_2MB, false)?
        };

        // Record allocation
        self.pressure_manager.record_allocation(size as u64)?;
        self.debugger.track_allocation(ptr.as_ptr() as usize, size, source);

        Ok(ptr)
    }

    /// Deallocate memory
    pub fn deallocate(&self, ptr: NonNull<u8>, size: usize) -> Result<()> {
        // Track deallocation
        self.debugger.track_deallocation(ptr.as_ptr() as usize)?;
        self.pressure_manager.record_deallocation(size as u64);

        if size <= MAX_SLAB_SIZE {
            unsafe { self.slab_allocator.deallocate(ptr, size)? };
        } else if size < LARGE_OBJECT_THRESHOLD {
            unsafe {
                let layout = Layout::from_size_align_unchecked(size, 16);
                System.dealloc(ptr.as_ptr(), layout);
            }
        } else {
            self.large_object_allocator.deallocate(ptr)?;
        }

        Ok(())
    }

    /// Create a memory context
    pub fn create_context(
        &self,
        name: String,
        _context_type: ContextType,
        limit: usize,
    ) -> Result<Arc<Mutex<MemoryContext>>> {
        self.arena_allocator.create_context(name, limit)
    }

    /// Get comprehensive statistics
    pub fn get_comprehensive_stats(&self) -> ComprehensiveMemoryStats {
        ComprehensiveMemoryStats {
            slab_stats: self.slab_allocator.get_stats(),
            arena_stats: self.arena_allocator.get_stats(),
            large_object_stats: self.large_object_allocator.get_stats(),
            pressure_stats: self.pressure_manager.get_stats(),
            debugger_stats: self.debugger.get_stats(),
            total_usage: self.pressure_manager.get_usage(),
        }
    }

    /// Get memory debugger
    pub fn debugger(&self) -> &Arc<MemoryDebugger> {
        &self.debugger
    }

    /// Get pressure manager
    pub fn pressure_manager(&self) -> &Arc<MemoryPressureManager> {
        &self.pressure_manager
    }
}

/// Comprehensive memory statistics
#[derive(Debug, Clone)]
pub struct ComprehensiveMemoryStats {
    pub slab_stats: SlabAllocatorStats,
    pub arena_stats: ArenaAllocatorStats,
    pub large_object_stats: LargeObjectAllocatorStats,
    pub pressure_stats: MemoryPressureStats,
    pub debugger_stats: MemoryDebuggerStats,
    pub total_usage: MemoryUsage,
}

// ============================================================================
// ADVANCED FEATURES: MEMORY POOLS & CUSTOM ALLOCATORS (500+ lines)
// ============================================================================

/// Memory pool for fixed-size allocations
pub struct MemoryPool {
    /// Object size
    object_size: usize,
    /// Pool capacity
    capacity: usize,
    /// Free list
    free_list: Mutex<Vec<NonNull<u8>>>,
    /// Allocated objects
    allocated: AtomicUsize,
    /// Total allocations
    total_allocations: AtomicU64,
    /// Total deallocations
    total_deallocations: AtomicU64,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(object_size: usize, capacity: usize) -> Result<Self> {
        let mut free_list = Vec::with_capacity(capacity);

        // Pre-allocate all objects
        unsafe {
            let layout = Layout::from_size_align(object_size, 16)
                .map_err(|e| DbError::OutOfMemory(format!("Invalid layout: {}", e)))?;

            for _ in 0..capacity {
                let ptr = System.alloc(layout);
                if ptr.is_null() {
                    return Err(DbError::OutOfMemory("Failed to allocate pool object".to_string()));
                }
                free_list.push(NonNull::new_unchecked(ptr));
            }
        }

        Ok(Self {
            object_size,
            capacity,
            free_list: Mutex::new(free_list),
            allocated: AtomicUsize::new(0),
            total_allocations: AtomicU64::new(0),
            total_deallocations: AtomicU64::new(0),
        })
    }

    /// Allocate an object from the pool
    pub fn allocate(&self) -> Option<NonNull<u8>> {
        let mut free_list = self.free_list.lock().unwrap();

        if let Some(ptr) = free_list.pop() {
            self.allocated.fetch_add(1, Ordering::Relaxed);
            self.total_allocations.fetch_add(1, Ordering::Relaxed);
            Some(ptr)
        } else {
            None
        }
    }

    /// Deallocate an object back to the pool
    pub fn deallocate(&self, ptr: NonNull<u8>) -> Result<()> {
        let mut free_list = self.free_list.lock().unwrap();

        if free_list.len() >= self.capacity {
            return Err(DbError::Internal("Pool overflow".to_string()));
        }

        free_list.push(ptr);
        self.allocated.fetch_sub(1, Ordering::Relaxed);
        self.total_deallocations.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> MemoryPoolStats {
        MemoryPoolStats {
            object_size: self.object_size,
            capacity: self.capacity,
            allocated: self.allocated.load(Ordering::Relaxed),
            available: self.free_list.lock().unwrap().len(),
            total_allocations: self.total_allocations.load(Ordering::Relaxed),
            total_deallocations: self.total_deallocations.load(Ordering::Relaxed),
            utilization: self.allocated.load(Ordering::Relaxed) as f64 / self.capacity as f64,
        }
    }
}

impl Drop for MemoryPool {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.object_size, 16);
            let free_list = self.free_list.lock().unwrap();

            for ptr in free_list.iter() {
                System.dealloc(ptr.as_ptr(), layout);
            }
        }
    }
}

/// Memory pool statistics
#[derive(Debug, Clone)]
pub struct MemoryPoolStats {
    pub object_size: usize,
    pub capacity: usize,
    pub allocated: usize,
    pub available: usize,
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub utilization: f64,
}

/// Memory zone allocator for segregated storage
pub struct MemoryZone {
    /// Zone name
    name: String,
    /// Base address
    base: NonNull<u8>,
    /// Zone size
    size: usize,
    /// Current offset
    offset: AtomicUsize,
    /// Zone type
    zone_type: ZoneType,
}

/// Zone type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneType {
    /// Normal zone
    Normal,
    /// DMA zone (for direct memory access)
    Dma,
    /// High memory zone
    HighMem,
}

impl MemoryZone {
    /// Create a new memory zone
    pub fn new(name: String, size: usize, zone_type: ZoneType) -> Result<Self> {
        unsafe {
            let layout = Layout::from_size_align(size, 4096)
                .map_err(|e| DbError::OutOfMemory(format!("Invalid zone layout: {}", e)))?;

            let base = System.alloc(layout);
            if base.is_null() {
                return Err(DbError::OutOfMemory("Failed to allocate zone".to_string()));
            }

            Ok(Self {
                name,
                base: NonNull::new_unchecked(base),
                size,
                offset: AtomicUsize::new(0),
                zone_type,
            })
        }
    }

    /// Allocate from zone
    pub fn allocate(&self, size: usize, align: usize) -> Option<NonNull<u8>> {
        loop {
            let current_offset = self.offset.load(Ordering::Acquire);
            let aligned_offset = (current_offset + align - 1) & !(align - 1);
            let new_offset = aligned_offset + size;

            if new_offset > self.size {
                return None;
            }

            if self.offset.compare_exchange(
                current_offset,
                new_offset,
                Ordering::Release,
                Ordering::Acquire,
            ).is_ok() {
                unsafe {
                    return Some(NonNull::new_unchecked(self.base.as_ptr().add(aligned_offset)));
                }
            }
        }
    }

    /// Reset zone
    pub fn reset(&self) {
        self.offset.store(0, Ordering::Release);
    }

    /// Get zone statistics
    pub fn get_stats(&self) -> MemoryZoneStats {
        let used = self.offset.load(Ordering::Relaxed);
        MemoryZoneStats {
            name: self.name.clone(),
            zone_type: self.zone_type,
            size: self.size,
            used,
            available: self.size - used,
            utilization: used as f64 / self.size as f64,
        }
    }
}

impl Drop for MemoryZone {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.size, 4096);
            System.dealloc(self.base.as_ptr(), layout);
        }
    }
}

/// Memory zone statistics
#[derive(Debug, Clone)]
pub struct MemoryZoneStats {
    pub name: String,
    pub zone_type: ZoneType,
    pub size: usize,
    pub used: usize,
    pub available: usize,
    pub utilization: f64,
}

/// Buddy allocator for power-of-two allocations
pub struct BuddyAllocator {
    /// Base address
    base: NonNull<u8>,
    /// Total size (must be power of 2)
    size: usize,
    /// Minimum block size
    min_block_size: usize,
    /// Free lists per order
    free_lists: Vec<Mutex<Vec<usize>>>,
    /// Block states
    block_states: Mutex<HashMap<usize, BlockState>>,
}

#[derive(Debug, Clone, Copy)]
enum BlockState {
    Free,
    Allocated,
    Split,
}

impl BuddyAllocator {
    /// Create a new buddy allocator
    pub fn new(size: usize, min_block_size: usize) -> Result<Self> {
        if !size.is_power_of_two() || !min_block_size.is_power_of_two() {
            return Err(DbError::InvalidArgument("Size must be power of 2".to_string()));
        }

        unsafe {
            let layout = Layout::from_size_align(size, size)
                .map_err(|e| DbError::OutOfMemory(format!("Invalid layout: {}", e)))?;

            let base = System.alloc(layout);
            if base.is_null() {
                return Err(DbError::OutOfMemory("Failed to allocate buddy memory".to_string()));
            }

            let num_orders = (size / min_block_size).trailing_zeros() as usize + 1;
            let mut free_lists = Vec::with_capacity(num_orders);

            for _ in 0..num_orders {
                free_lists.push(Mutex::new(Vec::new()));
            }

            // Add the entire memory as one free block
            free_lists[num_orders - 1].lock().unwrap().push(0);

            let mut block_states = HashMap::new();
            block_states.insert(0, BlockState::Free);

            Ok(Self {
                base: NonNull::new_unchecked(base),
                size,
                min_block_size,
                free_lists,
                block_states: Mutex::new(block_states),
            })
        }
    }

    /// Calculate order for size
    fn size_to_order(&self, size: usize) -> Option<usize> {
        let block_size = size.next_power_of_two().max(self.min_block_size);
        if block_size > self.size {
            return None;
        }
        Some((block_size / self.min_block_size).trailing_zeros() as usize)
    }

    /// Allocate memory
    pub fn allocate(&self, size: usize) -> Option<NonNull<u8>> {
        let order = self.size_to_order(size)?;

        // Find a free block of the right size or larger
        for current_order in order..self.free_lists.len() {
            let mut free_list = self.free_lists[current_order].lock().unwrap();

            if let Some(offset) = free_list.pop() {
                drop(free_list);

                // Split larger blocks if necessary
                for split_order in (order..current_order).rev() {
                    let block_size = self.min_block_size << split_order;
                    let buddy_offset = offset ^ block_size;

                    self.free_lists[split_order].lock().unwrap().push(buddy_offset);

                    let mut states = self.block_states.lock().unwrap();
                    states.insert(offset, BlockState::Split);
                    states.insert(buddy_offset, BlockState::Free);
                }

                self.block_states.lock().unwrap().insert(offset, BlockState::Allocated);

                unsafe {
                    return Some(NonNull::new_unchecked(self.base.as_ptr().add(offset)));
                }
            }
        }

        None
    }

    /// Deallocate memory
    pub fn deallocate(&self, ptr: NonNull<u8>, size: usize) -> Result<()> {
        let offset = unsafe { ptr.as_ptr().offset_from(self.base.as_ptr()) as usize };
        let order = self.size_to_order(size)
            .ok_or_else(|| DbError::InvalidArgument("Invalid size".to_string()))?;

        let mut current_offset = offset;
        let mut current_order = order;

        // Try to merge with buddy
        while current_order < self.free_lists.len() - 1 {
            let block_size = self.min_block_size << current_order;
            let buddy_offset = current_offset ^ block_size;

            let mut free_list = self.free_lists[current_order].lock().unwrap();

            if let Some(pos) = free_list.iter().position(|&off| off == buddy_offset) {
                free_list.swap_remove(pos);
                drop(free_list);

                current_offset = current_offset.min(buddy_offset);
                current_order += 1;
            } else {
                break;
            }
        }

        self.free_lists[current_order].lock().unwrap().push(current_offset);
        self.block_states.lock().unwrap().insert(current_offset, BlockState::Free);

        Ok(())
    }
}

impl Drop for BuddyAllocator {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.size, self.size);
            System.dealloc(self.base.as_ptr(), layout);
        }
    }
}

// ============================================================================
// PERFORMANCE MONITORING & OPTIMIZATION (400+ lines)
// ============================================================================

/// Performance counter for memory operations
pub struct PerformanceCounter {
    /// Fast path allocations (thread-local cache hits)
    fast_path: AtomicU64,
    /// Medium path allocations (depot hits)
    medium_path: AtomicU64,
    /// Slow path allocations (new slab/chunk)
    slow_path: AtomicU64,
    /// Cache line conflicts
    cache_conflicts: AtomicU64,
    /// TLB misses
    tlb_misses: AtomicU64,
    /// Page faults
    page_faults: AtomicU64,
}

impl PerformanceCounter {
    pub fn new() -> Self {
        Self {
            fast_path: AtomicU64::new(0),
            medium_path: AtomicU64::new(0),
            slow_path: AtomicU64::new(0),
            cache_conflicts: AtomicU64::new(0),
            tlb_misses: AtomicU64::new(0),
            page_faults: AtomicU64::new(0),
        }
    }

    pub fn record_fast_path(&self) {
        self.fast_path.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_medium_path(&self) {
        self.medium_path.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_slow_path(&self) {
        self.slow_path.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> PerformanceStats {
        let fast = self.fast_path.load(Ordering::Relaxed);
        let medium = self.medium_path.load(Ordering::Relaxed);
        let slow = self.slow_path.load(Ordering::Relaxed);
        let total = fast + medium + slow;

        PerformanceStats {
            fast_path_count: fast,
            medium_path_count: medium,
            slow_path_count: slow,
            fast_path_ratio: if total > 0 { fast as f64 / total as f64 } else { 0.0 },
            cache_conflicts: self.cache_conflicts.load(Ordering::Relaxed),
            tlb_misses: self.tlb_misses.load(Ordering::Relaxed),
            page_faults: self.page_faults.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub fast_path_count: u64,
    pub medium_path_count: u64,
    pub slow_path_count: u64,
    pub fast_path_ratio: f64,
    pub cache_conflicts: u64,
    pub tlb_misses: u64,
    pub page_faults: u64,
}

/// Memory access pattern analyzer
pub struct AccessPatternAnalyzer {
    /// Recent allocations
    recent_allocations: RwLock<VecDeque<AllocationRecord>>,
    /// Temporal locality score
    temporal_locality: AtomicU64,
    /// Spatial locality score
    spatial_locality: AtomicU64,
    /// Sequential access count
    sequential_access: AtomicU64,
    /// Random access count
    random_access: AtomicU64,
}

#[derive(Debug, Clone)]
struct AllocationRecord {
    address: usize,
    size: usize,
    timestamp: Instant,
}

impl AccessPatternAnalyzer {
    pub fn new() -> Self {
        Self {
            recent_allocations: RwLock::new(VecDeque::with_capacity(1000)),
            temporal_locality: AtomicU64::new(0),
            spatial_locality: AtomicU64::new(0),
            sequential_access: AtomicU64::new(0),
            random_access: AtomicU64::new(0),
        }
    }

    pub fn record_allocation(&self, address: usize, size: usize) {
        let record = AllocationRecord {
            address,
            size,
            timestamp: Instant::now(),
        };

        let mut recent = self.recent_allocations.write().unwrap();

        // Analyze pattern
        if let Some(last) = recent.back() {
            let time_diff = record.timestamp.duration_since(last.timestamp);
            let addr_diff = (record.address as i64 - last.address as i64).abs() as usize;

            // Check temporal locality (reuse within 1ms)
            if time_diff < Duration::from_millis(1) {
                self.temporal_locality.fetch_add(1, Ordering::Relaxed);
            }

            // Check spatial locality (nearby addresses)
            if addr_diff < 4096 {
                self.spatial_locality.fetch_add(1, Ordering::Relaxed);
            }

            // Check sequential vs random
            if addr_diff == last.size {
                self.sequential_access.fetch_add(1, Ordering::Relaxed);
            } else {
                self.random_access.fetch_add(1, Ordering::Relaxed);
            }
        }

        recent.push_back(record);
        if recent.len() > 1000 {
            recent.pop_front();
        }
    }

    pub fn get_pattern_stats(&self) -> AccessPatternStats {
        let temporal = self.temporal_locality.load(Ordering::Relaxed);
        let spatial = self.spatial_locality.load(Ordering::Relaxed);
        let sequential = self.sequential_access.load(Ordering::Relaxed);
        let random = self.random_access.load(Ordering::Relaxed);
        let total_access = sequential + random;

        AccessPatternStats {
            temporal_locality_score: temporal,
            spatial_locality_score: spatial,
            sequential_access_ratio: if total_access > 0 {
                sequential as f64 / total_access as f64
            } else {
                0.0
            },
            random_access_ratio: if total_access > 0 {
                random as f64 / total_access as f64
            } else {
                0.0
            },
            recent_allocation_count: self.recent_allocations.read().unwrap().len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccessPatternStats {
    pub temporal_locality_score: u64,
    pub spatial_locality_score: u64,
    pub sequential_access_ratio: f64,
    pub random_access_ratio: f64,
    pub recent_allocation_count: usize,
}

/// Memory bandwidth monitor
pub struct BandwidthMonitor {
    /// Bytes allocated per second
    alloc_bandwidth: AtomicU64,
    /// Bytes deallocated per second
    dealloc_bandwidth: AtomicU64,
    /// Last measurement time
    last_measurement: RwLock<Instant>,
    /// Total bytes allocated in current window
    window_allocated: AtomicU64,
    /// Total bytes deallocated in current window
    window_deallocated: AtomicU64,
}

impl BandwidthMonitor {
    pub fn new() -> Self {
        Self {
            alloc_bandwidth: AtomicU64::new(0),
            dealloc_bandwidth: AtomicU64::new(0),
            last_measurement: RwLock::new(Instant::now()),
            window_allocated: AtomicU64::new(0),
            window_deallocated: AtomicU64::new(0),
        }
    }

    pub fn record_allocation(&self, size: u64) {
        self.window_allocated.fetch_add(size, Ordering::Relaxed);
        self.update_bandwidth();
    }

    pub fn record_deallocation(&self, size: u64) {
        self.window_deallocated.fetch_add(size, Ordering::Relaxed);
        self.update_bandwidth();
    }

    fn update_bandwidth(&self) {
        let mut last = self.last_measurement.write().unwrap();
        let now = Instant::now();
        let elapsed = now.duration_since(*last);

        if elapsed >= Duration::from_secs(1) {
            let alloc_bytes = self.window_allocated.swap(0, Ordering::Relaxed);
            let dealloc_bytes = self.window_deallocated.swap(0, Ordering::Relaxed);

            let elapsed_secs = elapsed.as_secs_f64();
            self.alloc_bandwidth.store(
                (alloc_bytes as f64 / elapsed_secs) as u64,
                Ordering::Relaxed
            );
            self.dealloc_bandwidth.store(
                (dealloc_bytes as f64 / elapsed_secs) as u64,
                Ordering::Relaxed
            );

            *last = now;
        }
    }

    pub fn get_bandwidth(&self) -> BandwidthStats {
        BandwidthStats {
            alloc_bandwidth_bytes_per_sec: self.alloc_bandwidth.load(Ordering::Relaxed),
            dealloc_bandwidth_bytes_per_sec: self.dealloc_bandwidth.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BandwidthStats {
    pub alloc_bandwidth_bytes_per_sec: u64,
    pub dealloc_bandwidth_bytes_per_sec: u64,
}

// ============================================================================
// WEB API INTEGRATION (200+ lines)
// ============================================================================

/// Web API for memory management
pub struct MemoryApi {
    manager: Arc<MemoryManager>,
}

impl MemoryApi {
    pub fn new(manager: Arc<MemoryManager>) -> Self {
        Self { manager }
    }

    /// Get comprehensive statistics as JSON-compatible structure
    pub fn api_get_stats(&self) -> ComprehensiveMemoryStats {
        self.manager.get_comprehensive_stats()
    }

    /// Get memory usage summary
    pub fn api_get_usage_summary(&self) -> UsageSummary {
        let _stats = self.manager.get_comprehensive_stats();

        UsageSummary {
            total_memory: stats.total_usage.total_memory,
            used_memory: stats.total_usage.used_memory,
            available_memory: stats.total_usage.available_memory,
            usage_percentage: stats.total_usage.usage_ratio * 100.0,
            pressure_level: format!("{:?}", stats.total_usage.pressure_level),
            slab_usage: stats.slab_stats.bytes_allocated,
            arena_active_contexts: stats.arena_stats.active_contexts,
            large_object_count: stats.large_object_stats.active_objects,
        }
    }

    /// Get component breakdown
    pub fn api_get_component_breakdown(&self) -> Vec<ComponentBreakdown> {
        self.manager.debugger()
            .generate_report()
            .component_breakdown
    }

    /// Detect memory leaks
    pub fn api_detect_leaks(&self, min_age_seconds: u64) -> Vec<LeakReport> {
        self.manager.debugger()
            .detect_leaks(Duration::from_secs(min_age_seconds))
    }

    /// Enable debugging features
    pub fn api_enable_debugging(&self, feature: &str) -> Result<()> {
        match feature {
            "tracking" => {
                self.manager.debugger().enable_tracking();
                Ok(())
            }
            "leak_detection" => {
                self.manager.debugger().enable_leak_detection();
                Ok(())
            }
            "uaf_detection" => {
                self.manager.debugger().enable_uaf_detection();
                Ok(())
            }
            "guards" => {
                self.manager.debugger().enable_guards();
                Ok(())
            }
            "stack_traces" => {
                self.manager.debugger().enable_stack_traces();
                Ok(())
            }
            _ => Err(DbError::InvalidArgument(format!("Unknown feature: {}", feature)))
        }
    }

    /// Disable debugging features
    pub fn api_disable_debugging(&self, feature: &str) -> Result<()> {
        match feature {
            "tracking" => {
                self.manager.debugger().disable_tracking();
                Ok(())
            }
            _ => Err(DbError::InvalidArgument(format!("Cannot disable: {}", feature)))
        }
    }

    /// Get pressure events
    pub fn api_get_pressure_events(&self, count: usize) -> Vec<MemoryPressureEvent> {
        self.manager.pressure_manager()
            .get_recent_events(count)
    }

    /// Force emergency memory release
    pub fn api_force_emergency_release(&self) -> Result<()> {
        self.manager.pressure_manager()
            .emergency_release()
    }

    /// Set memory limit
    pub fn api_set_memory_limit(&self, limit_bytes: u64) {
        self.manager.pressure_manager()
            .set_total_memory(limit_bytes);
    }

    /// Generate full memory report
    pub fn api_generate_report(&self) -> MemoryReport {
        self.manager.debugger().generate_report()
    }
}

/// Usage summary for web display
#[derive(Debug, Clone)]
pub struct UsageSummary {
    pub total_memory: u64,
    pub used_memory: u64,
    pub available_memory: u64,
    pub usage_percentage: f64,
    pub pressure_level: String,
    pub slab_usage: u64,
    pub arena_active_contexts: u64,
    pub large_object_count: u64,
}

// ============================================================================
// UTILITY FUNCTIONS & HELPERS (50+ lines)
// ============================================================================

/// Helper to format memory size
pub fn format_memory_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Helper to parse memory size string
pub fn parse_memory_size(s: &str) -> Result<u64> {
    let s = s.trim().to_uppercase();
    let (num_str, multiplier) = if s.ends_with("TB") {
        (&s[..s.len()-2], 1024u64 * 1024 * 1024 * 1024)
    } else if s.ends_with("GB") {
        (&s[..s.len()-2], 1024u64 * 1024 * 1024)
    } else if s.ends_with("MB") {
        (&s[..s.len()-2], 1024u64 * 1024)
    } else if s.ends_with("KB") {
        (&s[..s.len()-2], 1024u64)
    } else if s.ends_with("B") {
        (&s[..s.len()-1], 1)
    } else {
        (s.as_str(), 1)
    };

    let num: f64 = num_str.trim().parse()
        .map_err(|e| DbError::InvalidArgument(format!("Invalid memory size: {}", e)))?;

    Ok((num * multiplier as f64) as u64)
}

/// Calculate optimal slab size for object size
pub fn calculate_optimal_slab_size(object_size: usize) -> usize {
    // Aim for ~2MB slabs aligned to huge pages
    let objects_per_slab = SLAB_SIZE / object_size.max(1);
    if objects_per_slab < 64 {
        // Too few objects, use smaller slab
        (object_size * 64).next_power_of_two()
    } else {
        SLAB_SIZE
    }
}

/// Check if size should use slab, system, or large object allocator
pub fn classify_allocation_size(size: usize) -> AllocatorType {
    if size <= MAX_SLAB_SIZE {
        AllocatorType::Slab
    } else if size < LARGE_OBJECT_THRESHOLD {
        AllocatorType::System
    } else {
        AllocatorType::LargeObject
    }
}

/// Allocator type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocatorType {
    Slab,
    System,
    LargeObject,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_memory_size() {
        assert_eq!(format_memory_size(1024), "1.00 KB");
        assert_eq!(format_memory_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_memory_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_parse_memory_size() {
        assert_eq!(parse_memory_size("1KB").unwrap(), 1024);
        assert_eq!(parse_memory_size("1MB").unwrap(), 1024 * 1024);
        assert_eq!(parse_memory_size("1GB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_memory_size("1.5GB").unwrap(), (1.5 * 1024.0 * 1024.0 * 1024.0) as u64);
    }

    #[test]
    fn test_classify_allocation_size() {
        assert_eq!(classify_allocation_size(1024), AllocatorType::Slab);
        assert_eq!(classify_allocation_size(MAX_SLAB_SIZE), AllocatorType::Slab);
        assert_eq!(classify_allocation_size(MAX_SLAB_SIZE + 1), AllocatorType::System);
        assert_eq!(classify_allocation_size(LARGE_OBJECT_THRESHOLD), AllocatorType::LargeObject);
    }
}
