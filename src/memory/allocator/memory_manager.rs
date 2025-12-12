//\! Unified Memory Manager
//\!
//\! Integration layer for all allocators.

use super::common::*;
use super::slab_allocator::{SlabAllocator, SlabAllocatorStats};
use super::arena_allocator::{ArenaAllocator, ArenaAllocatorStats, MemoryContext};
use super::large_object_allocator::{LargeObjectAllocator, LargeObjectAllocatorStats};
use super::pressure_manager::{MemoryPressureManager, MemoryPressureStats, MemoryUsage};
use super::debugger::{MemoryDebugger, MemoryDebuggerStats};

pub struct MemoryManager {
    // Slab allocator for small objects
    slab_allocator: Arc<SlabAllocator>,
    // Arena allocator for query contexts
    arena_allocator: Arc<ArenaAllocator>,
    // Large object allocator
    large_object_allocator: Arc<LargeObjectAllocator>,
    // Memory pressure manager
    pressure_manager: Arc<MemoryPressureManager>,
    // Total memory capacity
    total_capacity: u64,
    // Memory debugger
    debugger: Arc<MemoryDebugger>,
}

impl MemoryManager {
    // Create a new unified memory manager
    pub fn new(total_memory: u64) -> Self {
        let manager = Self {
            slab_allocator: Arc::new(SlabAllocator::new()),
            arena_allocator: Arc::new(ArenaAllocator::new()),
            large_object_allocator: Arc::new(LargeObjectAllocator::new()),
            pressure_manager: Arc::new(MemoryPressureManager::new(total_memory)),
            debugger: Arc::new(MemoryDebugger::new()),
            total_capacity: total_memory,
        };

        // Register pressure callbacks
        manager.setup_pressure_callbacks();

        manager
    }

    // Setup pressure callbacks for memory release
    fn setup_pressure_callbacks(&self) {
        let arena = Arc::clone(&self.arena_allocator);

        self.pressure_manager.register_callback(Arc::new(move |level| {
            match level {
                MemoryPressureLevel::None | MemoryPressureLevel::Normal => {
                    // No cleanup needed
                    Ok(0)
                }
                MemoryPressureLevel::Low | MemoryPressureLevel::Warning => {
                    // Cleanup dead contexts
                    let freed = arena.cleanup_dead_contexts();
                    Ok(freed * 1024) // Estimate
                }
                MemoryPressureLevel::Medium | MemoryPressureLevel::High | MemoryPressureLevel::Critical | MemoryPressureLevel::Emergency => {
                    // Aggressive cleanup
                    let freed = arena.cleanup_dead_contexts();
                    Ok(freed * 1024)
                }
            }
        }));
    }

    // Allocate memory using the appropriate allocator
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

    // Deallocate memory
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

    // Create a memory context
    pub fn create_context(
        &self,
        name: String,
        _context_type: ContextType,
        limit: usize,
    ) -> Result<Arc<Mutex<MemoryContext>>> {
        self.arena_allocator.create_context(name, limit)
    }

    // Get comprehensive statistics
    pub fn get_comprehensive_stats(&self) -> ComprehensiveMemoryStats {
        let slab_stats = self.slab_allocator.get_stats();
        let arena_stats = self.arena_allocator.get_stats();
        let large_object_stats = self.large_object_allocator.get_stats();

        let total_allocated = slab_stats.total_allocated + arena_stats.total_allocated + large_object_stats.bytes_allocated;
        let total_capacity = self.total_capacity;

        ComprehensiveMemoryStats {
            slab_stats,
            arena_stats: arena_stats.clone(),
            large_object_stats,
            pressure_stats: self.pressure_manager.get_stats(),
            debugger_stats: self.debugger.get_stats(),
            total_usage: self.pressure_manager.get_usage(),
            total_capacity,
            total_allocated,
            context_stats: arena_stats.active_contexts,
        }
    }

    // Get memory debugger
    pub fn debugger(&self) -> &Arc<MemoryDebugger> {
        &self.debugger
    }

    // Get pressure manager
    pub fn pressure_manager(&self) -> &Arc<MemoryPressureManager> {
        &self.pressure_manager
    }
}

// Comprehensive memory statistics
#[derive(Debug, Clone)]
pub struct ComprehensiveMemoryStats {
    pub slab_stats: SlabAllocatorStats,
    pub arena_stats: ArenaAllocatorStats,
    pub large_object_stats: LargeObjectAllocatorStats,
    pub pressure_stats: MemoryPressureStats,
    pub debugger_stats: MemoryDebuggerStats,
    pub total_usage: MemoryUsage,
    pub total_capacity: u64,
    pub total_allocated: u64,
    pub context_stats: u64
}

// ============================================================================
// ADVANCED FEATURES: MEMORY POOLS & CUSTOM ALLOCATORS (500+ lines)
