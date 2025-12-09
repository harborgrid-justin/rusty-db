// # Arena Allocator Implementation
//
// This module provides a high-performance arena allocator for bump allocation patterns
// commonly used in query processing and transaction contexts. Arena allocators excel
// at scenarios where many small allocations are made and then freed all at once.
//
// ## Key Features
//
// - **Bump Allocation**: Extremely fast O(1) allocation with simple pointer increment
// - **Memory Contexts**: Hierarchical context system for isolated memory management
// - **Block Management**: Automatic block allocation and chaining for large contexts
// - **Memory Mapping**: Efficient large block allocation using mmap
// - **Zero-Cost Reset**: Instant memory reclamation by resetting arena pointers
// - **Context Hierarchy**: Parent-child relationships for scope-based memory management
//
// ## Design Overview
//
// The arena allocator organizes memory into contexts, each containing one or more
// memory blocks. Allocation is performed by advancing a pointer within the current
// block. When a block is exhausted, a new larger block is allocated automatically.
//
// ### Memory Contexts
//
// Memory contexts provide isolated allocation spaces that can be reset or destroyed
// independently. Common context types include:
// - **Query Context**: For entire query execution
// - **Statement Context**: For individual SQL statements
// - **Operator Context**: For specific query operators
// - **Temporary Context**: For short-lived allocations
//
// ### Block Chain
//
// Each context maintains a chain of memory blocks that grow in size as needed.
// This allows efficient handling of workloads with varying memory requirements.
//
// ## Usage Example
//
// ```rust
// use crate::memory::arena::*;
// use crate::memory::types::*;
//
// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
// // Create arena allocator
// let config = ArenaConfig::default();
// let allocator = ArenaAllocator::new(config).await?;
//
// // Create a query context
// let query_context_id = MemoryContextId::new("query_001")?;
// let _context = allocator.create_context(
//     query_context_id,
//     ContextType::Query,
//     None, // No parent
//     Some(1024 * 1024), // 1MB limit
// ).await?;
//
// // Allocate memory from the context
// let ptr1 = allocator.allocate_in_context(&context.id, 256, 8).await?;
// let ptr2 = allocator.allocate_in_context(&context.id, 1024, 16).await?;
//
// // Create child context for temporary data
// let temp_context_id = MemoryContextId::new("temp_001")?;
// let temp_context = allocator.create_context(
//     temp_context_id,
//     ContextType::Temporary,
//     Some(context.id.clone()),
//     None, // No limit
// ).await?;
//
// // Allocate in temporary context
// let temp_ptr = allocator.allocate_in_context(&temp_context.id, 512, 8).await?;
//
// // Reset temporary context (frees all temp allocations)
// allocator.reset_context(&temp_context.id).await?;
//
// // Destroy query context (frees all allocations including children)
// allocator.destroy_context(&context.id).await?;
// # Ok(())
// # }
// ```

use crate::memory::types::*;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::alloc::{alloc, alloc_zeroed, dealloc, Layout};
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration};
use thiserror::Error;
use tokio::sync::RwLock as AsyncRwLock;
use uuid::Uuid;

/// Arena allocator specific errors
#[derive(Error, Debug)]
pub enum ArenaError {
    #[error("Memory context not found: {context_id}")]
    ContextNotFound { context_id: String },

    #[error("Block allocation failed: {reason}")]
    BlockAllocationFailed { reason: String },

    #[error("Context limit exceeded: {limit} bytes")]
    ContextLimitExceeded { limit: usize },

    #[error("Invalid block size: {size}")]
    InvalidBlockSize { size: usize },

    #[error("Context hierarchy violation: {reason}")]
    ContextHierarchyViolation { reason: String },

    #[error("Arena corruption detected in context {context_id}")]
    ArenaCorruption { context_id: String },

    #[error("Block chain broken in context {context_id}")]
    BlockChainBroken { context_id: String },
}

/// Memory block within an arena
///
/// Each block represents a contiguous chunk of memory that can be
/// allocated from using bump allocation. Blocks are chained together
/// to form larger allocation spaces.
#[derive(Debug)]
pub struct MemoryBlock {
    /// Unique block identifier
    pub block_id: Uuid,
    /// Pointer to block memory
    pub memory: NonNull<u8>,
    /// Size of the block
    pub size: usize,
    /// Current allocation pointer within the block
    pub current: AtomicPtr<u8>,
    /// End pointer of the block
    pub end: *const u8,
    /// Remaining bytes in the block
    pub remaining: AtomicUsize,
    /// Next block in the chain
    pub next: Mutex<Option<Arc<MemoryBlock>>>,
    /// Block creation timestamp
    pub created_at: SystemTime,
    /// Whether block was allocated via mmap
    pub is_mmap: bool,
    /// Reference count
    pub ref_count: AtomicUsize,
}

/// Memory context for arena allocation
///
/// A context represents an isolated memory space with its own
/// block chain and allocation tracking. Contexts can have
/// parent-child relationships for hierarchical management.
#[derive(Debug)]
pub struct MemoryContext {
    /// Context identifier
    pub id: MemoryContextId,
    /// Context type
    pub context_type: ContextType,
    /// Parent context (if any)
    pub parent: Option<MemoryContextId>,
    /// Child contexts
    pub children: Arc<RwLock<Vec<MemoryContextId>>>,
    /// First block in the chain
    pub first_block: Arc<Mutex<Option<Arc<MemoryBlock>>>>,
    /// Current active block
    pub current_block: Arc<Mutex<Option<Arc<MemoryBlock>>>>,
    /// Total bytes allocated in this context
    pub bytes_allocated: AtomicUsize,
    /// Peak memory usage
    pub peak_usage: AtomicUsize,
    /// Number of allocations made
    pub allocation_count: AtomicU64,
    /// Number of context resets
    pub reset_count: AtomicU64,
    /// Memory limit for this context (None = unlimited)
    pub memory_limit: Option<usize>,
    /// Whether the context is active
    pub is_active: AtomicBool,
    /// Context creation timestamp
    pub created_at: SystemTime,
    /// Last allocation timestamp
    pub last_allocation: AtomicU64,
    /// Context statistics
    pub stats: Arc<AsyncRwLock<ContextStats>>,
}

/// Context statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextStats {
    /// Total bytes allocated
    pub total_allocated: u64,
    /// Peak memory usage
    pub peak_usage: u64,
    /// Number of allocations
    pub allocation_count: u64,
    /// Number of blocks allocated
    pub block_count: u64,
    /// Number of context resets
    pub reset_count: u64,
    /// Average allocation size
    pub avg_allocation_size: f64,
    /// Allocation rate (per second)
    pub allocation_rate: f64,
    /// Memory efficiency (allocated/committed ratio)
    pub efficiency_ratio: f64,
    /// Context lifetime
    pub lifetime: Duration,
    /// Last updated timestamp
    pub last_updated: SystemTime,
}

/// Arena allocator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaAllocatorConfig {
    /// Default initial block size
    pub initial_block_size: usize,
    /// Maximum block size
    pub max_block_size: usize,
    /// Block size growth factor
    pub growth_factor: f64,
    /// Enable memory mapping for large blocks
    pub enable_mmap: bool,
    /// Minimum size for mmap allocation
    pub mmap_threshold: usize,
    /// Enable huge pages
    pub enable_huge_pages: bool,
    /// Memory alignment for allocations
    pub default_alignment: usize,
    /// Enable memory zeroing
    pub enable_zeroing: bool,
    /// Maximum number of contexts
    pub max_contexts: usize,
    /// Enable statistics collection
    pub enable_statistics: bool,
    /// Enable debugging features
    pub enable_debugging: bool,
    /// Context cleanup threshold (unused contexts)
    pub cleanup_threshold: Duration,
}

impl Default for ArenaAllocatorConfig {
    fn default() -> Self {
        Self {
            initial_block_size: 64 * 1024, // 64KB
            max_block_size: 64 * 1024 * 1024, // 64MB
            growth_factor: 2.0,
            enable_mmap: true,
            mmap_threshold: 1024 * 1024, // 1MB
            enable_huge_pages: false,
            default_alignment: 8,
            enable_zeroing: false,
            max_contexts: 10000,
            enable_statistics: true,
            enable_debugging: false,
            cleanup_threshold: Duration::from_mins(30),
        }
    }
}

/// Main arena allocator structure
///
/// Manages multiple memory contexts and provides the primary
/// allocation interface for bump allocation patterns.
#[derive(Debug)]
pub struct ArenaAllocator {
    /// Allocator configuration
    config: ArenaAllocatorConfig,
    /// Active memory contexts
    contexts: Arc<RwLock<HashMap<MemoryContextId, Arc<MemoryContext>>>>,
    /// Global allocator statistics
    stats: Arc<AsyncRwLock<ArenaAllocatorStats>>,
    /// Whether allocator is active
    is_active: AtomicBool,
    /// Creation timestamp
    created_at: SystemTime,
    /// Allocator unique identifier
    allocator_id: Uuid,
    /// Background cleanup task handle
    cleanup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

/// Arena allocator statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArenaAllocatorStats {
    /// Total bytes allocated across all contexts
    pub total_allocated: u64,
    /// Current bytes in use
    pub bytes_in_use: u64,
    /// Number of active contexts
    pub active_contexts: usize,
    /// Total number of contexts created
    pub total_contexts_created: u64,
    /// Total number of contexts destroyed
    pub total_contexts_destroyed: u64,
    /// Number of active blocks
    pub active_blocks: usize,
    /// Total number of blocks allocated
    pub total_blocks_allocated: u64,
    /// Average allocation size
    pub avg_allocation_size: f64,
    /// Memory efficiency ratio
    pub efficiency_ratio: f64,
    /// Peak memory usage
    pub peak_usage: u64,
    /// Total allocations across all contexts
    pub total_allocations: u64,
    /// Context resets performed
    pub context_resets: u64,
    /// Blocks reused from reset contexts
    pub blocks_reused: u64,
    /// Allocator uptime
    pub uptime: Duration,
    /// Last updated timestamp
    pub last_updated: SystemTime,
}

impl MemoryBlock {
    /// Creates a new memory block with the specified size
    pub fn new(size: usize, use_mmap: bool) -> Result<Arc<Self>, ArenaError> {
        if size == 0 {
            return Err(ArenaError::InvalidBlockSize { size });
        }

        let memory = if use_mmap {
            // Use mmap for large allocations
            Self::allocate_mmap(size)?
        } else {
            // Use regular allocation
            Self::allocate_regular(size)?
        };

        let end = unsafe { memory.as_ptr().add(size) };

        Ok(Arc::new(Self {
            block_id: Uuid::new_v4(),
            memory,
            size,
            current: AtomicPtr::new(memory.as_ptr()),
            end,
            remaining: AtomicUsize::new(size),
            next: Mutex::new(None),
            created_at: SystemTime::now(),
            is_mmap: use_mmap,
            ref_count: AtomicUsize::new(1),
        }))
    }

    /// Allocates memory using regular system allocator
    fn allocate_regular(size: usize) -> Result<NonNull<u8>, ArenaError> {
        let layout = Layout::from_size_align(size, 4096)
            .map_err(|_| ArenaError::BlockAllocationFailed {
                reason: "Invalid layout".to_string(),
            })?;

        let memory = unsafe { alloc_zeroed(layout) };
        if memory.is_null() {
            return Err(ArenaError::BlockAllocationFailed {
                reason: "System allocation failed".to_string(),
            });
        }

        Ok(NonNull::new(memory).unwrap())
    }

    /// Allocates memory using mmap
    fn allocate_mmap(size: usize) -> Result<NonNull<u8>, ArenaError> {
        // Platform-specific mmap implementation would go here
        // For now, fall back to regular allocation
        Self::allocate_regular(size)
    }

    /// Attempts to allocate from this block
    pub fn allocate(&self, size: usize, alignment: usize) -> Option<NonNull<u8>> {
        loop {
            let current = self.current.load(Ordering::Acquire);

            // Calculate aligned address
            let aligned_addr = Self::align_up(current as usize, alignment);
            let aligned_ptr = aligned_addr as *mut u8;

            // Check if allocation fits
            let new_ptr = unsafe { aligned_ptr.add(size) };
            if new_ptr > self.end {
                return None; // Allocation doesn't fit
            }

            // Attempt to update current pointer
            if self.current.compare_exchange_weak(
                current,
                new_ptr,
                Ordering::Release,
                Ordering::Relaxed,
            ).is_ok() {
                let allocated_size = new_ptr as usize - current as usize;
                self.remaining.fetch_sub(allocated_size, Ordering::Relaxed);
                return Some(NonNull::new(aligned_ptr).unwrap());
            }
        }
    }

    /// Resets the block for reuse
    pub fn reset(&self) {
        self.current.store(self.memory.as_ptr(), Ordering::Release);
        self.remaining.store(self.size, Ordering::Relaxed);
    }

    /// Gets available space in the block
    pub fn available_space(&self) -> usize {
        self.remaining.load(Ordering::Relaxed)
    }

    /// Checks if the block is empty
    pub fn is_empty(&self) -> bool {
        self.current.load(Ordering::Relaxed) == self.memory.as_ptr()
    }

    /// Checks if the block is full
    pub fn is_full(&self) -> bool {
        self.remaining.load(Ordering::Relaxed) == 0
    }

    /// Aligns address up to the specified alignment
    fn align_up(addr: usize, alignment: usize) -> usize {
        (addr + alignment - 1) & !(alignment - 1)
    }
}

impl Drop for MemoryBlock {
    fn drop(&mut self) {
        if self.is_mmap {
            // Unmap memory (platform-specific)
            // For now, treat as regular deallocation
        }

        unsafe {
            let layout = Layout::from_size_align(self.size, 4096).unwrap();
            dealloc(self.memory.as_ptr(), layout);
        }
    }
}

impl MemoryContext {
    /// Creates a new memory context
    pub fn new(
        id: MemoryContextId,
        context_type: ContextType,
        parent: Option<MemoryContextId>,
        memory_limit: Option<usize>,
    ) -> Self {
        Self {
            id,
            context_type,
            parent,
            children: Arc::new(RwLock::new(Vec::new())),
            first_block: Arc::new(Mutex::new(None)),
            current_block: Arc::new(Mutex::new(None)),
            bytes_allocated: AtomicUsize::new(0),
            peak_usage: AtomicUsize::new(0),
            allocation_count: AtomicU64::new(0),
            reset_count: AtomicU64::new(0),
            memory_limit,
            is_active: AtomicBool::new(true),
            created_at: SystemTime::now(),
            last_allocation: AtomicU64::new(0),
            stats: Arc::new(AsyncRwLock::new(ContextStats::default())),
        }
    }

    /// Adds a child context
    pub fn add_child(&self, child_id: MemoryContextId) {
        let mut children = self.children.write();
        if !children.contains(&child_id) {
            children.push(child_id);
        }
    }

    /// Removes a child context
    pub fn remove_child(&self, child_id: &MemoryContextId) {
        let mut children = self.children.write();
        children.retain(|id| id != child_id);
    }

    /// Gets the current memory usage
    pub fn current_usage(&self) -> usize {
        self.bytes_allocated.load(Ordering::Relaxed)
    }

    /// Checks if allocation would exceed limit
    pub fn would_exceed_limit(&self, additional_size: usize) -> bool {
        if let Some(limit) = self.memory_limit {
            let current = self.current_usage();
            current.saturating_add(additional_size) > limit
        } else {
            false
        }
    }

    /// Updates allocation statistics
    pub fn record_allocation(&self, size: usize) {
        self.bytes_allocated.fetch_add(size, Ordering::Relaxed);
        self.allocation_count.fetch_add(1, Ordering::Relaxed);

        let current_usage = self.current_usage();
        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        while current_usage > peak {
            match self.peak_usage.compare_exchange_weak(
                peak,
                current_usage,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => peak = actual,
            }
        }

        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        self.last_allocation.store(now, Ordering::Relaxed);
    }

    /// Resets the context (clears all allocations)
    pub fn reset(&self) {
        // Reset all blocks in the chain
        if let Some(first_block) = self.first_block.lock().as_ref() {
            let mut current_block = Some(Arc::clone(first_block));
            while let Some(block) = current_block {
                block.reset();
                current_block = block.next.lock().clone();
            }
        }

        // Reset current block to first block
        if let Some(first_block) = self.first_block.lock().as_ref() {
            *self.current_block.lock() = Some(Arc::clone(first_block));
        }

        // Reset statistics
        self.bytes_allocated.store(0, Ordering::Relaxed);
        self.reset_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Gets the age of the context
    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or_default()
    }
}

impl ArenaAllocator {
    /// Creates a new arena allocator with the given configuration
    pub async fn new(config: ArenaAllocatorConfig) -> Result<Self, ArenaError> {
        let allocator = Self {
            config,
            contexts: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(AsyncRwLock::new(ArenaAllocatorStats::default())),
            is_active: AtomicBool::new(true),
            created_at: SystemTime::now(),
            allocator_id: Uuid::new_v4(),
            cleanup_handle: Arc::new(Mutex::new(None)),
        };

        // Start background cleanup task
        if allocator.config.enable_statistics {
            allocator.start_cleanup_task().await;
        }

        Ok(allocator)
    }

    /// Creates a new memory context
    pub async fn create_context(
        &self,
        id: MemoryContextId,
        context_type: ContextType,
        parent: Option<MemoryContextId>,
        memory_limit: Option<usize>,
    ) -> Result<Arc<MemoryContext>, ArenaError> {
        if !self.is_active.load(Ordering::Relaxed) {
            return Err(ArenaError::ContextHierarchyViolation {
                reason: "Allocator is not active".to_string(),
            });
        }

        let mut contexts = self.contexts.write();

        // Check context limit
        if contexts.len() >= self.config.max_contexts {
            return Err(ArenaError::ContextHierarchyViolation {
                reason: format!("Maximum context limit reached: {}", self.config.max_contexts),
            });
        }

        // Check if context already exists
        if contexts.contains_key(&id) {
            return Err(ArenaError::ContextHierarchyViolation {
                reason: format!("Context already exists: {}", id),
            });
        }

        // Validate parent relationship
        if let Some(parent_id) = &parent {
            if !contexts.contains_key(parent_id) {
                return Err(ArenaError::ContextNotFound {
                    context_id: parent_id.to_string(),
                });
            }
        }

        // Create the context
        let _context = Arc::new(MemoryContext::new(id.clone(), context_type, parent.clone(), memory_limit));

        // Add to parent's children list
        if let Some(parent_id) = &parent {
            if let Some(parent_context) = contexts.get(parent_id) {
                parent_context.add_child(id.clone());
            }
        }

        // Insert into contexts map
        contexts.insert(id, Arc::clone(&context));

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_contexts_created += 1;
        stats.active_contexts = contexts.len();
        stats.last_updated = SystemTime::now();

        Ok(context)
    }

    /// Destroys a memory context and all its children
    pub async fn destroy_context(&self, context_id: &MemoryContextId) -> Result<(), ArenaError> {
        let _context = {
            let mut contexts = self.contexts.write();
            contexts.remove(context_id)
        };

        let _context = context.ok_or_else(|| ArenaError::ContextNotFound {
            context_id: context_id.to_string(),
        })?;

        // Recursively destroy children
        let children = context.children.read().clone();
        for child_id in children {
            let _ = self.destroy_context(&child_id).await;
        }

        // Remove from parent's children list
        if let Some(parent_id) = &context.parent {
            let contexts = self.contexts.read();
            if let Some(parent_context) = contexts.get(parent_id) {
                parent_context.remove_child(context_id);
            }
        }

        // Mark context as inactive
        context.is_active.store(false, Ordering::Relaxed);

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_contexts_destroyed += 1;
        stats.active_contexts = self.contexts.read().len();
        stats.last_updated = SystemTime::now();

        Ok(())
    }

    /// Allocates memory within a specific context
    pub async fn allocate_in_context(
        &self,
        context_id: &MemoryContextId,
        size: usize,
        alignment: usize,
    ) -> Result<NonNull<u8>, MemoryError> {
        validate_allocation_size(size)?;
        validate_alignment(alignment)?;

        let contexts = self.contexts.read();
        let _context = contexts.get(context_id)
            .ok_or_else(|| MemoryError::ContextNotFound {
                context_id: context_id.to_string(),
            })?;

        // Check memory limit
        if context.would_exceed_limit(size) {
            return Err(MemoryError::PressureCritical {
                current_usage: context.current_usage() as u64,
                limit: context.memory_limit.unwrap_or(0) as u64,
            });
        }

        // Try to allocate from current block
        if let Some(ptr) = self.try_allocate_from_current_block(context, size, alignment).await? {
            context.record_allocation(size);
            self.update_global_stats(size).await;
            return Ok(ptr);
        }

        // Need to allocate a new block
        let ptr = self.allocate_new_block_and_allocate(context, size, alignment).await?;
        context.record_allocation(size);
        self.update_global_stats(size).await;

        Ok(ptr)
    }

    /// Tries to allocate from the current block
    async fn try_allocate_from_current_block(
        &self,
        context: &MemoryContext,
        size: usize,
        alignment: usize,
    ) -> Result<Option<NonNull<u8>>, ArenaError> {
        let current_block = context.current_block.lock();
        if let Some(block) = current_block.as_ref() {
            if let Some(ptr) = block.allocate(size, alignment) {
                return Ok(Some(ptr));
            }
        }
        Ok(None)
    }

    /// Allocates a new block and then allocates from it
    async fn allocate_new_block_and_allocate(
        &self,
        context: &MemoryContext,
        size: usize,
        alignment: usize,
    ) -> Result<NonNull<u8>, MemoryError> {
        // Calculate block size needed
        let block_size = self.calculate_block_size(context, size).await;
        let use_mmap = block_size >= self.config.mmap_threshold;

        // Create new block
        let new_block = MemoryBlock::new(block_size, use_mmap)
            .map_err(|e| MemoryError::OutOfMemory {
                reason: format!("Block allocation failed: {}", e),
            })?;

        // Link the new block to the chain
        self.link_block_to_context(context, Arc::clone(&new_block)).await;

        // Allocate from the new block
        new_block.allocate(size, alignment)
            .ok_or_else(|| MemoryError::OutOfMemory {
                reason: "Allocation failed in new block".to_string(),
            })
    }

    /// Calculates appropriate block size for allocation
    async fn calculate_block_size(&self, context: &MemoryContext, min_size: usize) -> usize {
        let _stats = context.stats.read().await;

        // Start with configured initial size or previous average
        let base_size = if stats.avg_allocation_size > 0.0 {
            (stats.avg_allocation_size * 100.0) as usize // Allocate for ~100 allocations
        } else {
            self.config.initial_block_size
        };

        // Ensure it can fit the requested allocation
        let required_size = min_size + 64; // Add some padding
        let block_size = base_size.max(required_size);

        // Cap at maximum block size
        block_size.min(self.config.max_block_size)
    }

    /// Links a new block to the context's block chain
    async fn link_block_to_context(
        &self,
        context: &MemoryContext,
        new_block: Arc<MemoryBlock>,
    ) {
        let mut first_block = context.first_block.lock();
        let mut current_block = context.current_block.lock();

        if first_block.is_none() {
            // First block in the context
            *first_block = Some(Arc::clone(&new_block));
            *current_block = Some(new_block);
        } else {
            // Link to the end of the chain
            if let Some(current) = current_block.as_ref() {
                *current.next.lock() = Some(Arc::clone(&new_block));
            }
            *current_block = Some(new_block);
        }
    }

    /// Resets a memory context (clears all allocations)
    pub async fn reset_context(&self, context_id: &MemoryContextId) -> Result<(), ArenaError> {
        let contexts = self.contexts.read();
        let _context = contexts.get(context_id)
            .ok_or_else(|| ArenaError::ContextNotFound {
                context_id: context_id.to_string(),
            })?;

        // Reset all child contexts first
        let children = context.children.read().clone();
        for child_id in children {
            let _ = self.reset_context(&child_id).await;
        }

        // Reset the context
        let bytes_before_reset = context.current_usage();
        context.reset();

        // Update global statistics
        let mut stats = self.stats.write().await;
        stats.context_resets += 1;
        stats.bytes_in_use = stats.bytes_in_use.saturating_sub(bytes_before_reset as u64);
        stats.last_updated = SystemTime::now();

        Ok(())
    }

    /// Updates global allocator statistics
    async fn update_global_stats(&self, size: usize) {
        let mut stats = self.stats.write().await;
        stats.total_allocated += size as u64;
        stats.bytes_in_use += size as u64;
        stats.total_allocations += 1;

        if stats.bytes_in_use > stats.peak_usage {
            stats.peak_usage = stats.bytes_in_use;
        }

        if stats.total_allocations > 0 {
            stats.avg_allocation_size = stats.total_allocated as f64 / stats.total_allocations as f64;
        }

        stats.last_updated = SystemTime::now();
    }

    /// Gets comprehensive allocator statistics
    pub async fn get_statistics(&self) -> ArenaAllocatorStats {
        let mut stats = self.stats.write().await;

        stats.active_contexts = self.contexts.read().len();
        stats.uptime = SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or_default();

        // Calculate efficiency ratio
        if stats.bytes_in_use > 0 {
            stats.efficiency_ratio = stats.total_allocated as f64 / stats.bytes_in_use as f64;
        }

        stats.clone()
    }

    /// Gets statistics for a specific context
    pub async fn get_context_statistics(
        &self,
        context_id: &MemoryContextId,
    ) -> Result<ContextStats, ArenaError> {
        let contexts = self.contexts.read();
        let _context = contexts.get(context_id)
            .ok_or_else(|| ArenaError::ContextNotFound {
                context_id: context_id.to_string(),
            })?;

        let mut stats = context.stats.write().await;
        stats.total_allocated = context.bytes_allocated.load(Ordering::Relaxed) as u64;
        stats.peak_usage = context.peak_usage.load(Ordering::Relaxed) as u64;
        stats.allocation_count = context.allocation_count.load(Ordering::Relaxed);
        stats.reset_count = context.reset_count.load(Ordering::Relaxed);
        stats.lifetime = context.age();

        if stats.allocation_count > 0 {
            stats.avg_allocation_size = stats.total_allocated as f64 / stats.allocation_count as f64;
        }

        stats.last_updated = SystemTime::now();

        Ok(stats.clone())
    }

    /// Lists all active contexts
    pub async fn list_contexts(&self) -> Vec<MemoryContextId> {
        self.contexts.read().keys().cloned().collect()
    }

    /// Starts the background cleanup task
    async fn start_cleanup_task(&self) {
        let contexts = Arc::clone(&self.contexts);
        let cleanup_threshold = self.config.cleanup_threshold;
        let is_active = Arc::new(AtomicBool::new(true));

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes

            while is_active.load(Ordering::Relaxed) {
                interval.tick().await;

                // Cleanup unused contexts
                let mut contexts_to_remove = Vec::new();
                {
                    let contexts_guard = contexts.read();
                    let now = SystemTime::now();

                    for (id, context) in contexts_guard.iter() {
                        if !context.is_active.load(Ordering::Relaxed) {
                            continue;
                        }

                        let last_allocation_ns = context.last_allocation.load(Ordering::Relaxed);
                        if last_allocation_ns > 0 {
                            let last_allocation_time = std::time::UNIX_EPOCH +
                                Duration::from_nanos(last_allocation_ns);

                            if let Ok(elapsed) = now.duration_since(last_allocation_time) {
                                if elapsed > cleanup_threshold {
                                    contexts_to_remove.push(id.clone());
                                }
                            }
                        }
                    }
                }

                // Remove unused contexts
                if !contexts_to_remove.is_empty() {
                    let mut contexts_guard = contexts.write();
                    for id in contexts_to_remove {
                        if let Some(context) = contexts_guard.get(&id) {
                            // Only remove if truly unused (no allocations and no children)
                            if context.current_usage() == 0 &&
                               context.children.read().is_empty() {
                                contexts_guard.remove(&id);
                            }
                        }
                    }
                }
            }
        });

        *self.cleanup_handle.lock() = Some(handle);
    }

    /// Shuts down the allocator gracefully
    pub async fn shutdown(&self) -> Result<(), ArenaError> {
        self.is_active.store(false, Ordering::Relaxed);

        // Stop cleanup task
        if let Some(handle) = self.cleanup_handle.lock().take() {
            handle.abort();
        }

        // Destroy all contexts
        let context_ids: Vec<_> = self.contexts.read().keys().cloned().collect();
        for context_id in context_ids {
            let _ = self.destroy_context(&context_id).await;
        }

        Ok(())
    }
}

/// Utility functions for arena allocator
impl ArenaAllocator {
    /// Calculates memory fragmentation for a context
    pub async fn calculate_fragmentation(
        &self,
        context_id: &MemoryContextId,
    ) -> Result<f64, ArenaError> {
        let contexts = self.contexts.read();
        let _context = contexts.get(context_id)
            .ok_or_else(|| ArenaError::ContextNotFound {
                context_id: context_id.to_string(),
            })?;

        let mut total_block_size = 0;
        let mut used_block_size = 0;

        // Walk the block chain
        if let Some(first_block) = context.first_block.lock().as_ref() {
            let mut current_block = Some(Arc::clone(first_block));

            while let Some(block) = current_block {
                total_block_size += block.size;
                used_block_size += block.size - block.available_space();
                current_block = block.next.lock().clone();
            }
        }

        if total_block_size > 0 {
            Ok(1.0 - (used_block_size as f64 / total_block_size as f64))
        } else {
            Ok(0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_arena_allocator_creation() {
        let config = ArenaAllocatorConfig::default();
        let allocator = ArenaAllocator::new(config).await;
        assert!(allocator.is_ok());

        let allocator = allocator.unwrap();
        assert!(allocator.is_active.load(Ordering::Relaxed));
    }

    #[test]
    async fn test_memory_context_creation() {
        let config = ArenaAllocatorConfig::default();
        let allocator = ArenaAllocator::new(config).await.unwrap();

        let context_id = MemoryContextId::new("test_context").unwrap();
        let _context = allocator.create_context(
            context_id.clone(),
            ContextType::Query,
            None,
            None,
        ).await;

        assert!(context.is_ok());
        let _context = context.unwrap();
        assert_eq!(context.id, context_id);
        assert!(context.is_active.load(Ordering::Relaxed));
    }

    #[test]
    async fn test_memory_block_creation() {
        let block = MemoryBlock::new(4096, false);
        assert!(block.is_ok());

        let block = block.unwrap();
        assert_eq!(block.size, 4096);
        assert!(block.is_empty());
        assert!(!block.is_full());
        assert_eq!(block.available_space(), 4096);
    }

    #[test]
    async fn test_block_allocation() {
        let block = MemoryBlock::new(4096, false).unwrap();

        let ptr1 = block.allocate(64, 8);
        assert!(ptr1.is_some());
        assert_eq!(block.available_space(), 4096 - 64);

        let ptr2 = block.allocate(128, 16);
        assert!(ptr2.is_some());

        // Verify alignment
        assert_eq!(ptr2.unwrap().as_ptr() as usize % 16, 0);
    }

    #[test]
    async fn test_context_hierarchy() {
        let config = ArenaAllocatorConfig::default();
        let allocator = ArenaAllocator::new(config).await.unwrap();

        // Create parent context
        let parent_id = MemoryContextId::new("parent").unwrap();
        let parent = allocator.create_context(
            parent_id.clone(),
            ContextType::Query,
            None,
            None,
        ).await.unwrap();

        // Create child context
        let child_id = MemoryContextId::new("child").unwrap();
        let child = allocator.create_context(
            child_id.clone(),
            ContextType::Operator,
            Some(parent_id.clone()),
            None,
        ).await.unwrap();

        // Verify hierarchy
        assert_eq!(child.parent, Some(parent_id));
        assert!(parent.children.read().contains(&child_id));
    }

    #[test]
    async fn test_context_memory_limit() {
        let config = ArenaAllocatorConfig::default();
        let allocator = ArenaAllocator::new(config).await.unwrap();

        let context_id = MemoryContextId::new("limited").unwrap();
        let _context = allocator.create_context(
            context_id.clone(),
            ContextType::Temporary,
            None,
            Some(1024), // 1KB limit
        ).await.unwrap();

        // Test limit checking
        assert!(!context.would_exceed_limit(500)); // OK
        assert!(context.would_exceed_limit(2000)); // Exceeds limit
    }

    #[test]
    async fn test_context_reset() {
        let config = ArenaAllocatorConfig::default();
        let allocator = ArenaAllocator::new(config).await.unwrap();

        let context_id = MemoryContextId::new("reset_test").unwrap();
        let _context = allocator.create_context(
            context_id.clone(),
            ContextType::Query,
            None,
            None,
        ).await.unwrap();

        // Allocate some memory
        let _ptr1 = allocator.allocate_in_context(&context_id, 256, 8).await.unwrap();
        let _ptr2 = allocator.allocate_in_context(&context_id, 512, 8).await.unwrap();

        // Check usage before reset
        let stats_before = allocator.get_context_statistics(&context_id).await.unwrap();
        assert_eq!(stats_before.total_allocated, 768);

        // Reset context
        allocator.reset_context(&context_id).await.unwrap();

        // Check usage after reset
        let contexts = allocator.contexts.read();
        let _context = contexts.get(&context_id).unwrap();
        assert_eq!(context.current_usage(), 0);
    }

    #[test]
    async fn test_allocator_statistics() {
        let config = ArenaAllocatorConfig::default();
        let allocator = ArenaAllocator::new(config).await.unwrap();

        let context_id = MemoryContextId::new("stats_test").unwrap();
        let _context = allocator.create_context(
            context_id.clone(),
            ContextType::Query,
            None,
            None,
        ).await.unwrap();

        // Make some allocations
        let _ptr1 = allocator.allocate_in_context(&context_id, 100, 8).await.unwrap();
        let _ptr2 = allocator.allocate_in_context(&context_id, 200, 8).await.unwrap();

        let _stats = allocator.get_statistics().await;
        assert_eq!(stats.total_allocations, 2);
        assert_eq!(stats.total_allocated, 300);
        assert_eq!(stats.active_contexts, 1);
        assert_eq!(stats.avg_allocation_size, 150.0);
    }

    #[test]
    fn test_memory_block_alignment() {
        let addr = 0x1001;
        assert_eq!(MemoryBlock::align_up(addr, 8), 0x1008);
        assert_eq!(MemoryBlock::align_up(addr, 16), 0x1010);
        assert_eq!(MemoryBlock::align_up(0x1000, 16), 0x1000); // Already aligned
    }
}
