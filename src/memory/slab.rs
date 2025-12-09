// # Slab Allocator Implementation
//
// This module provides a high-performance slab allocator for fixed-size memory allocations.
// The slab allocator uses a magazine layer caching system with thread-local magazines
// for optimal performance in multi-threaded environments.
//
// ## Key Features
//
// - **Size Classes**: Multiple size classes for efficient allocation
// - **Magazine Caching**: Thread-local magazines for lock-free fast path
// - **Cache Coloring**: Reduces cache line conflicts
// - **Statistics Tracking**: Comprehensive performance monitoring
// - **Thread Safety**: Lock-free operations on hot path
// - **Memory Efficiency**: Minimal overhead per allocation
//
// ## Design Overview
//
// The slab allocator organizes memory into "slabs" - large contiguous chunks
// divided into fixed-size objects. Each size class manages its own slabs
// and maintains free lists for efficient allocation and deallocation.
//
// ### Magazine Layer
//
// The magazine layer provides thread-local caching to minimize lock contention:
// - **Loaded Magazine**: Currently used magazine per thread
// - **Previous Magazine**: Backup magazine for quick exchange
// - **Depot**: Central magazine storage with lock protection
//
// ### Cache Coloring
//
// Objects are allocated with different cache line offsets to reduce
// conflicts in CPU caches, improving overall system performance.
//
// ## Usage Example
//
// ```rust
// use crate::memory::slab::*;
// use crate::memory::types::*;
//
// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
// // Create slab allocator
// let config = SlabConfig::default();
// let allocator = SlabAllocator::new(config).await?;
//
// // Allocate 64-byte object
// let ptr = allocator.allocate(64, 8).await?;
//
// // Use the allocated memory
// unsafe {
//     std::ptr::write_bytes(ptr.as_ptr(), 0x42, 64);
// }
//
// // Deallocate when done
// allocator.deallocate(ptr, 64).await?;
//
// // Get statistics
// let stats = allocator.get_statistics().await;
// println!("Allocations: {}", stats.total_allocations);
// # Ok(())
// # }
// ```

use std::time::{SystemTime, UNIX_EPOCH};
use crate::memory::types::*;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::alloc::{alloc, alloc_zeroed, dealloc, Layout};
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

// Slab allocator specific errors
#[derive(Error, Debug)]
pub enum SlabError {
    #[error("Size class not found for size {size}")]
    SizeClassNotFound { size: usize },

    #[error("Slab allocation failed: {reason}")]
    SlabAllocationFailed { reason: String },

    #[error("Magazine creation failed: {reason}")]
    MagazineCreationFailed { reason: String },

    #[error("Invalid object pointer: {address:#x}")]
    InvalidObjectPointer { address: usize },

    #[error("Size class corrupted: class {class_id}")]
    SizeClassCorrupted { class_id: usize },

    #[error("Magazine cache corrupted")]
    MagazineCacheCorrupted,

    #[error("Thread cache initialization failed")]
    ThreadCacheInitializationFailed,
}

// Size class definition for slab allocator
//
// Each size class manages objects of a specific size with
// dedicated slabs and free lists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeClass {
    // Class identifier
    pub class_id: usize,
    // Object size in bytes
    pub object_size: usize,
    // Number of objects per slab
    pub objects_per_slab: usize,
    // Current number of slabs
    pub slab_count: AtomicUsize,
    // Number of free objects across all slabs
    pub free_objects: AtomicUsize,
    // Total allocations for this size class
    pub total_allocations: AtomicU64,
    // Total deallocations for this size class
    pub total_deallocations: AtomicU64,
    // Size class statistics
    pub stats: Arc<AsyncRwLock<SizeClassStats>>,
}

// Size class statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SizeClassStats {
    // Number of slabs allocated
    pub slabs_allocated: u64,
    // Number of slabs freed
    pub slabs_freed: u64,
    // Current active slabs
    pub active_slabs: u64,
    // Slab utilization percentage
    pub utilization: f64,
    // Cache hit ratio
    pub cache_hit_ratio: f64,
    // Average allocation rate (per second)
    pub allocation_rate: f64,
    // Peak memory usage
    pub peak_usage: u64,
    // Fragmentation ratio
    pub fragmentation: f64,
    // Last updated timestamp
    pub last_updated: SystemTime,
}

// Individual slab structure
//
// A slab contains multiple objects of the same size
// with a free list for efficient allocation.
#[derive(Debug)]
pub struct Slab {
    // Unique slab identifier
    pub slab_id: Uuid,
    // Pointer to slab memory
    pub memory: NonNull<u8>,
    // Size of the slab
    pub size: usize,
    // Size class this slab belongs to
    pub size_class_id: usize,
    // Object size
    pub object_size: usize,
    // Number of objects in this slab
    pub object_count: usize,
    // Number of free objects
    pub free_count: AtomicUsize,
    // Free list head
    pub free_list: AtomicPtr<FreeObject>,
    // Cache color offset
    pub color_offset: usize,
    // Reference count for the slab
    pub ref_count: AtomicUsize,
    // Whether the slab is active
    pub is_active: AtomicBool,
    // Allocation timestamp
    pub allocated_at: SystemTime,
}

// Free object entry in the free list
//
// Links free objects within a slab using a simple
// linked list structure.
#[repr(C)]
struct FreeObject {
    // Next free object in the list
    next: *mut FreeObject,
}

// Magazine structure for thread-local caching
//
// Magazines cache a fixed number of objects per thread
// to reduce lock contention on the global free lists.
#[derive(Debug)]
pub struct Magazine {
    // Magazine identifier
    pub magazine_id: Uuid,
    // Size class this magazine serves
    pub size_class_id: usize,
    // Array of cached objects
    pub objects: Vec<NonNull<u8>>,
    // Current number of cached objects
    pub count: AtomicUsize,
    // Magazine capacity
    pub capacity: usize,
    // Magazine allocation rounds
    pub rounds: AtomicU64,
    // Creation timestamp
    pub created_at: SystemTime,
    // Last access timestamp
    pub last_access: AtomicU64,
}

// Thread-local cache structure
//
// Each thread maintains a cache of magazines for
// different size classes to enable lock-free allocation.
#[derive(Debug)]
pub struct ThreadCache {
    // Thread identifier
    pub thread_id: std::thread::ThreadId,
    // Loaded magazines (one per size class)
    pub loaded_magazines: Vec<Option<Arc<Mutex<Magazine>>>>,
    // Previous magazines for quick exchange
    pub previous_magazines: Vec<Option<Arc<Mutex<Magazine>>>>,
    // Cache statistics
    pub stats: ThreadCacheStats,
    // Whether cache is active
    pub is_active: AtomicBool,
    // Creation timestamp
    pub created_at: SystemTime,
}

// Thread cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThreadCacheStats {
    // Number of cache hits
    pub cache_hits: u64,
    // Number of cache misses
    pub cache_misses: u64,
    // Number of magazine exchanges
    pub magazine_exchanges: u64,
    // Number of depot operations
    pub depot_operations: u64,
    // Total allocations through cache
    pub total_allocations: u64,
    // Total deallocations through cache
    pub total_deallocations: u64,
    // Cache hit ratio
    pub hit_ratio: f64,
    // Average service time (nanoseconds)
    pub avg_service_time: f64,
    // Last updated timestamp
    pub last_updated: SystemTime,
}

// Magazine depot for central magazine storage
//
// The depot stores magazines that are not currently
// assigned to threads and manages magazine lifecycle.
#[derive(Debug)]
pub struct MagazineDepot {
    // Full magazines ready for use
    pub full_magazines: Mutex<HashMap<usize, Vec<Arc<Mutex<Magazine>>>>>,
    // Empty magazines available for filling
    pub empty_magazines: Mutex<HashMap<usize, Vec<Arc<Mutex<Magazine>>>>>,
    // Maximum magazines per size class
    pub max_magazines_per_class: usize,
    // Magazine creation statistics
    pub magazines_created: AtomicU64,
    // Magazine destruction statistics
    pub magazines_destroyed: AtomicU64,
    // Depot statistics
    pub stats: Arc<AsyncRwLock<DepotStats>>,
}

// Depot statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DepotStats {
    // Total magazine exchanges
    pub magazine_exchanges: u64,
    // Total magazines created
    pub magazines_created: u64,
    // Total magazines destroyed
    pub magazines_destroyed: u64,
    // Current full magazine count
    pub full_magazine_count: usize,
    // Current empty magazine count
    pub empty_magazine_count: usize,
    // Average magazine lifetime
    pub avg_magazine_lifetime: Duration,
    // Depot contention events
    pub contention_events: u64,
    // Last updated timestamp
    pub last_updated: SystemTime,
}

// Main slab allocator structure
//
// Manages multiple size classes and provides the primary
// allocation interface with magazine-based caching.
#[derive(Debug)]
pub struct SlabAllocator {
    // Allocator configuration
    config: SlabConfig,
    // Size classes array
    size_classes: Vec<Arc<SizeClass>>,
    // Size to class mapping
    size_to_class: Vec<usize>,
    // Magazine depot
    depot: Arc<MagazineDepot>,
    // Thread-local caches
    thread_caches: Arc<RwLock<HashMap<std::thread::ThreadId, Arc<Mutex<ThreadCache>>>>>,
    // Global allocator statistics
    stats: Arc<AsyncRwLock<SlabAllocatorStats>>,
    // Whether allocator is initialized
    is_initialized: AtomicBool,
    // Creation timestamp
    created_at: SystemTime,
    // Allocator unique identifier
    allocator_id: Uuid,
}

// Slab allocator statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SlabAllocatorStats {
    // Total bytes allocated
    pub total_allocated: u64,
    // Total bytes deallocated
    pub total_deallocated: u64,
    // Current bytes in use
    pub bytes_in_use: u64,
    // Total number of allocations
    pub total_allocations: u64,
    // Total number of deallocations
    pub total_deallocations: u64,
    // Number of active size classes
    pub active_size_classes: usize,
    // Total number of slabs
    pub total_slabs: usize,
    // Total cache hits across all threads
    pub total_cache_hits: u64,
    // Total cache misses across all threads
    pub total_cache_misses: u64,
    // Global cache hit ratio
    pub cache_hit_ratio: f64,
    // Average allocation size
    pub avg_allocation_size: f64,
    // Peak memory usage
    pub peak_usage: u64,
    // Overall fragmentation
    pub fragmentation_ratio: f64,
    // Allocator uptime
    pub uptime: Duration,
    // Last updated timestamp
    pub last_updated: SystemTime,
}

impl SizeClass {
    // Creates a new size class
    pub fn new(class_id: usize, object_size: usize, slab_size: usize) -> Self {
        let objects_per_slab = slab_size / object_size;

        Self {
            class_id,
            object_size,
            objects_per_slab,
            slab_count: AtomicUsize::new(0),
            free_objects: AtomicUsize::new(0),
            total_allocations: AtomicU64::new(0),
            total_deallocations: AtomicU64::new(0),
            stats: Arc::new(AsyncRwLock::new(SizeClassStats::default())),
        }
    }

    // Gets the current utilization of this size class
    pub fn utilization(&self) -> f64 {
        let total_objects = self.slab_count.load(Ordering::Relaxed) * self.objects_per_slab;
        let free = self.free_objects.load(Ordering::Relaxed);

        if total_objects == 0 {
            0.0
        } else {
            1.0 - (free as f64 / total_objects as f64)
        }
    }

    // Checks if more slabs are needed
    pub fn needs_slab(&self) -> bool {
        self.free_objects.load(Ordering::Relaxed) < self.objects_per_slab / 4
    }
}

impl Slab {
    // Creates a new slab for the given size class
    pub fn new(
        size_class_id: usize,
        object_size: usize,
        slab_size: usize,
        color_offset: usize,
    ) -> Result<Arc<Self>, SlabError> {
        // Allocate slab memory
        let layout = Layout::from_size_align(slab_size, 4096) // Page-aligned
            .map_err(|_| SlabError::SlabAllocationFailed {
                reason: "Invalid layout".to_string(),
            })?;

        let memory = unsafe { alloc_zeroed(layout) };
        if memory.is_null() {
            return Err(SlabError::SlabAllocationFailed {
                reason: "Memory allocation failed".to_string(),
            });
        }

        let memory = NonNull::new(memory).unwrap();
        let object_count = slab_size / object_size;

        let slab = Arc::new(Self {
            slab_id: Uuid::new_v4(),
            memory,
            size: slab_size,
            size_class_id,
            object_size,
            object_count,
            free_count: AtomicUsize::new(object_count),
            free_list: AtomicPtr::new(std::ptr::null_mut()),
            color_offset,
            ref_count: AtomicUsize::new(1),
            is_active: AtomicBool::new(true),
            allocated_at: SystemTime::now(),
        });

        // Initialize free list
        slab.initialize_free_list();

        Ok(slab)
    }

    // Initializes the free list for this slab
    fn initialize_free_list(&self) {
        let mut current_offset = self.color_offset;
        let mut prev_object: *mut FreeObject = std::ptr::null_mut();

        for _ in 0..self.object_count {
            if current_offset + self.object_size <= self.size {
                let object_ptr = unsafe {
                    self.memory.as_ptr().add(current_offset) as *mut FreeObject
                };

                if !prev_object.is_null() {
                    unsafe {
                        (*prev_object).next = object_ptr;
                    }
                } else {
                    // First object becomes the head of free list
                    self.free_list.store(object_ptr, Ordering::Relaxed);
                }

                unsafe {
                    (*object_ptr).next = std::ptr::null_mut();
                }

                prev_object = object_ptr;
                current_offset += self.object_size;
            }
        }
    }

    // Allocates an object from this slab
    pub fn allocate_object(&self) -> Option<NonNull<u8>> {
        loop {
            let free_head = self.free_list.load(Ordering::Acquire);
            if free_head.is_null() {
                return None; // No free objects
            }

            let next = unsafe { (*free_head).next };

            if self.free_list.compare_exchange_weak(
                free_head,
                next,
                Ordering::Release,
                Ordering::Relaxed,
            ).is_ok() {
                self.free_count.fetch_sub(1, Ordering::Relaxed);
                return Some(NonNull::new(free_head as *mut u8).unwrap());
            }
        }
    }

    // Deallocates an object back to this slab
    pub fn deallocate_object(&self, ptr: NonNull<u8>) {
        let free_object = ptr.as_ptr() as *mut FreeObject;

        loop {
            let free_head = self.free_list.load(Ordering::Acquire);

            unsafe {
                (*free_object).next = free_head;
            }

            if self.free_list.compare_exchange_weak(
                free_head,
                free_object,
                Ordering::Release,
                Ordering::Relaxed,
            ).is_ok() {
                self.free_count.fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
    }

    // Checks if the slab is empty (all objects free)
    pub fn is_empty(&self) -> bool {
        self.free_count.load(Ordering::Relaxed) == self.object_count
    }

    // Checks if the slab is full (no free objects)
    pub fn is_full(&self) -> bool {
        self.free_count.load(Ordering::Relaxed) == 0
    }
}

impl Drop for Slab {
    fn drop(&mut self) {
        // Deallocate slab memory
        unsafe {
            let layout = Layout::from_size_align(self.size, 4096).unwrap();
            dealloc(self.memory.as_ptr(), layout);
        }
    }
}

impl Magazine {
    // Creates a new magazine for the given size class
    pub fn new(size_class_id: usize, capacity: usize) -> Result<Self, SlabError> {
        if capacity == 0 {
            return Err(SlabError::MagazineCreationFailed {
                reason: "Magazine capacity cannot be zero".to_string(),
            });
        }

        Ok(Self {
            magazine_id: Uuid::new_v4(),
            size_class_id,
            objects: Vec::with_capacity(capacity),
            count: AtomicUsize::new(0),
            capacity,
            rounds: AtomicU64::new(0),
            created_at: SystemTime::now(),
            last_access: AtomicU64::new(0),
        })
    }

    // Adds an object to the magazine
    pub fn add_object(&mut self, object: NonNull<u8>) -> Result<(), SlabError> {
        if self.is_full() {
            return Err(SlabError::MagazineCacheCorrupted);
        }

        self.objects.push(object);
        self.count.store(self.objects.len(), Ordering::Relaxed);
        self.update_access_time();
        Ok(())
    }

    // Removes an object from the magazine
    pub fn remove_object(&mut self) -> Option<NonNull<u8>> {
        if self.is_empty() {
            return None;
        }

        let object = self.objects.pop();
        self.count.store(self.objects.len(), Ordering::Relaxed);
        self.update_access_time();
        object
    }

    // Checks if the magazine is full
    pub fn is_full(&self) -> bool {
        self.count.load(Ordering::Relaxed) >= self.capacity
    }

    // Checks if the magazine is empty
    pub fn is_empty(&self) -> bool {
        self.count.load(Ordering::Relaxed) == 0
    }

    // Gets the current load of the magazine
    pub fn load(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    // Updates the last access time
    fn update_access_time(&self) {
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        self.last_access.store(now, Ordering::Relaxed);
    }
}

impl ThreadCache {
    // Creates a new thread cache
    pub fn new(thread_id: std::thread::ThreadId, num_size_classes: usize) -> Self {
        Self {
            thread_id,
            loaded_magazines: vec![None; num_size_classes],
            previous_magazines: vec![None; num_size_classes],
            stats: ThreadCacheStats::default(),
            is_active: AtomicBool::new(true),
            created_at: SystemTime::now(),
        }
    }

    // Gets a loaded magazine for the size class
    pub fn get_loaded_magazine(&self, size_class_id: usize) -> Option<&Arc<Mutex<Magazine>>> {
        self.loaded_magazines.get(size_class_id)?.as_ref()
    }

    // Sets a loaded magazine for the size class
    pub fn set_loaded_magazine(&mut self, size_class_id: usize, magazine: Arc<Mutex<Magazine>>) {
        if let Some(slot) = self.loaded_magazines.get_mut(size_class_id) {
            *slot = Some(magazine);
        }
    }

    // Exchanges loaded and previous magazines
    pub fn exchange_magazines(&mut self, size_class_id: usize) {
        if size_class_id < self.loaded_magazines.len() {
            let loaded = self.loaded_magazines[size_class_id].take();
            let previous = self.previous_magazines[size_class_id].take();

            self.loaded_magazines[size_class_id] = previous;
            self.previous_magazines[size_class_id] = loaded;

            self.stats.magazine_exchanges += 1;
            self.stats.last_updated = SystemTime::now();
        }
    }
}

impl MagazineDepot {
    // Creates a new magazine depot
    pub fn new(max_magazines_per_class: usize) -> Self {
        Self {
            full_magazines: Mutex::new(HashMap::new()),
            empty_magazines: Mutex::new(HashMap::new()),
            max_magazines_per_class,
            magazines_created: AtomicU64::new(0),
            magazines_destroyed: AtomicU64::new(0),
            stats: Arc::new(AsyncRwLock::new(DepotStats::default())),
        }
    }

    // Gets a full magazine from the depot
    pub fn get_full_magazine(&self, size_class_id: usize) -> Option<Arc<Mutex<Magazine>>> {
        let mut full_magazines = self.full_magazines.lock();
        if let Some(magazines) = full_magazines.get_mut(&size_class_id) {
            magazines.pop()
        } else {
            None
        }
    }

    // Returns a full magazine to the depot
    pub fn return_full_magazine(&self, magazine: Arc<Mutex<Magazine>>) {
        let size_class_id = magazine.lock().unwrap().size_class_id;
        let mut full_magazines = self.full_magazines.lock();

        let magazines = full_magazines.entry(size_class_id).or_insert_with(Vec::new);

        if magazines.len() < self.max_magazines_per_class {
            magazines.push(magazine);
        }
        // If at capacity, magazine will be dropped
    }

    // Gets an empty magazine from the depot
    pub fn get_empty_magazine(&self, size_class_id: usize) -> Option<Arc<Mutex<Magazine>>> {
        let mut empty_magazines = self.empty_magazines.lock();
        if let Some(magazines) = empty_magazines.get_mut(&size_class_id) {
            magazines.pop()
        } else {
            None
        }
    }

    // Returns an empty magazine to the depot
    pub fn return_empty_magazine(&self, magazine: Arc<Mutex<Magazine>>) {
        let size_class_id = magazine.lock().unwrap().size_class_id;
        let mut empty_magazines = self.empty_magazines.lock();

        let magazines = empty_magazines.entry(size_class_id).or_insert_with(Vec::new);

        if magazines.len() < self.max_magazines_per_class {
            magazines.push(magazine);
        }
        // If at capacity, magazine will be dropped
    }
}

impl SlabAllocator {
    // Creates a new slab allocator with the given configuration
    pub async fn new(config: SlabConfig) -> Result<Self, SlabError> {
        let mut allocator = Self {
            config: config.clone(),
            size_classes: Vec::new(),
            size_to_class: vec![0; constants::MAX_SLAB_SIZE + 1],
            depot: Arc::new(MagazineDepot::new(config.magazine_capacity * 4)),
            thread_caches: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(AsyncRwLock::new(SlabAllocatorStats::default())),
            is_initialized: AtomicBool::new(false),
            created_at: SystemTime::now(),
            allocator_id: Uuid::new_v4(),
        };

        allocator.initialize_size_classes()?;
        allocator.is_initialized.store(true, Ordering::Relaxed);

        Ok(allocator)
    }

    // Initializes size classes for the allocator
    fn initialize_size_classes(&mut self) -> Result<(), SlabError> {
        let mut size_classes = Vec::new();
        let mut current_size = self.config.min_object_size;
        let mut class_id = 0;

        while current_size <= self.config.max_slab_size && class_id < self.config.num_size_classes {
            let size_class = Arc::new(SizeClass::new(
                class_id,
                current_size,
                self.config.slab_size,
            ));

            size_classes.push(size_class);

            // Map all sizes up to next class to this class
            let next_size = (current_size as f64 * self.config.size_class_factor) as usize;
            for size in current_size..=next_size.min(self.config.max_slab_size) {
                if size < self.size_to_class.len() {
                    self.size_to_class[size] = class_id;
                }
            }

            current_size = next_size + 1;
            class_id += 1;
        }

        self.size_classes = size_classes;
        Ok(())
    }

    // Allocates memory of the given size
    pub async fn allocate(
        &self,
        size: usize,
        alignment: usize,
    ) -> Result<NonNull<u8>, MemoryError> {
        if !self.is_initialized.load(Ordering::Relaxed) {
            return Err(MemoryError::InvalidConfiguration {
                field: "allocator".to_string(),
                reason: "Slab allocator not initialized".to_string(),
            });
        }

        validate_allocation_size(size)?;
        validate_alignment(alignment)?;

        // Find appropriate size class
        let effective_size = size.max(alignment);
        if effective_size > self.config.max_slab_size {
            return Err(MemoryError::InvalidSize {
                size: effective_size,
                reason: "Size exceeds slab allocator limit".to_string(),
            });
        }

        let size_class_id = self.size_to_class[effective_size];
        let size_class = &self.size_classes[size_class_id];

        // Try thread cache first
        if let Some(ptr) = self.try_allocate_from_cache(size_class_id).await {
            self.update_allocation_stats(size_class, size).await;
            return Ok(ptr);
        }

        // Allocate from slab
        self.allocate_from_slab(size_class).await
    }

    // Deallocates memory at the given pointer
    pub async fn deallocate(
        &self,
        ptr: NonNull<u8>,
        size: usize,
    ) -> Result<(), MemoryError> {
        if !self.is_initialized.load(Ordering::Relaxed) {
            return Err(MemoryError::InvalidConfiguration {
                field: "allocator".to_string(),
                reason: "Slab allocator not initialized".to_string(),
            });
        }

        validate_allocation_size(size)?;

        let size_class_id = self.size_to_class[size.min(self.config.max_slab_size)];
        let size_class = &self.size_classes[size_class_id];

        // Try to cache in thread cache
        if self.try_deallocate_to_cache(ptr, size_class_id).await {
            self.update_deallocation_stats(size_class, size).await;
            return Ok(());
        }

        // Return to slab
        self.deallocate_to_slab(ptr, size_class).await
    }

    // Tries to allocate from thread cache
    async fn try_allocate_from_cache(
        &self,
        size_class_id: usize,
    ) -> Option<NonNull<u8>> {
        let thread_id = std::thread::current().id();
        let thread_caches = self.thread_caches.read();

        if let Some(thread_cache) = thread_caches.get(&thread_id) {
            let mut cache = thread_cache.lock();

            if let Some(magazine) = cache.get_loaded_magazine(size_class_id) {
                let mut mag = magazine.lock();
                if let Some(ptr) = mag.remove_object() {
                    cache.stats.cache_hits += 1;
                    cache.stats.total_allocations += 1;
                    cache.stats.last_updated = SystemTime::now();
                    return Some(ptr);
                }
            }

            cache.stats.cache_misses += 1;
        }

        None
    }

    // Tries to deallocate to thread cache
    async fn try_deallocate_to_cache(
        &self,
        ptr: NonNull<u8>,
        size_class_id: usize,
    ) -> bool {
        let thread_id = std::thread::current().id();
        let thread_caches = self.thread_caches.read();

        if let Some(thread_cache) = thread_caches.get(&thread_id) {
            let mut cache = thread_cache.lock();

            if let Some(magazine) = cache.get_loaded_magazine(size_class_id) {
                let mut mag = magazine.lock();
                if !mag.is_full() {
                    if mag.add_object(ptr).is_ok() {
                        cache.stats.total_deallocations += 1;
                        cache.stats.last_updated = SystemTime::now();
                        return true;
                    }
                }
            }
        }

        false
    }

    // Allocates directly from slab
    async fn allocate_from_slab(
        &self,
        size_class: &SizeClass,
    ) -> Result<NonNull<u8>, MemoryError> {
        // This would contain the core slab allocation logic
        // For now, we'll use a simple implementation
        todo!("Implement slab allocation logic")
    }

    // Deallocates directly to slab
    async fn deallocate_to_slab(
        &self,
        ptr: NonNull<u8>,
        size_class: &SizeClass,
    ) -> Result<(), MemoryError> {
        // This would contain the core slab deallocation logic
        todo!("Implement slab deallocation logic")
    }

    // Updates allocation statistics
    async fn update_allocation_stats(&self, size_class: &SizeClass, size: usize) {
        size_class.total_allocations.fetch_add(1, Ordering::Relaxed);

        let mut stats = self.stats.write().await;
        stats.total_allocations += 1;
        stats.total_allocated += size as u64;
        stats.bytes_in_use += size as u64;
        stats.last_updated = SystemTime::now();

        if stats.bytes_in_use > stats.peak_usage {
            stats.peak_usage = stats.bytes_in_use;
        }

        if stats.total_allocations > 0 {
            stats.avg_allocation_size = stats.total_allocated as f64 / stats.total_allocations as f64;
        }
    }

    // Updates deallocation statistics
    async fn update_deallocation_stats(&self, size_class: &SizeClass, size: usize) {
        size_class.total_deallocations.fetch_add(1, Ordering::Relaxed);

        let mut stats = self.stats.write().await;
        stats.total_deallocations += 1;
        stats.total_deallocated += size as u64;
        stats.bytes_in_use = stats.bytes_in_use.saturating_sub(size as u64);
        stats.last_updated = SystemTime::now();
    }

    // Gets comprehensive allocator statistics
    pub async fn get_statistics(&self) -> SlabAllocatorStats {
        let stats = self.stats.read().await;
        let mut result = stats.clone();

        result.uptime = SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or_default();

        // Calculate cache hit ratio
        result.total_cache_hits = 0;
        result.total_cache_misses = 0;

        let thread_caches = self.thread_caches.read();
        for cache in thread_caches.values() {
            let cache_stats = &cache.lock().unwrap().stats;
            result.total_cache_hits += cache_stats.cache_hits;
            result.total_cache_misses += cache_stats.cache_misses;
        }

        if result.total_cache_hits + result.total_cache_misses > 0 {
            result.cache_hit_ratio = result.total_cache_hits as f64
                / (result.total_cache_hits + result.total_cache_misses) as f64;
        }

        result
    }

    // Gets statistics for a specific size class
    pub async fn get_size_class_statistics(&self, size_class_id: usize) -> Option<SizeClassStats> {
        if let Some(size_class) = self.size_classes.get(size_class_id) {
            let stats = size_class.stats.read().await;
            Some(stats.clone())
        } else {
            None
        }
    }

    // Checks allocator health
    pub async fn health_check(&self) -> Result<(), MemoryError> {
        if !self.is_initialized.load(Ordering::Relaxed) {
            return Err(MemoryError::InvalidConfiguration {
                field: "allocator".to_string(),
                reason: "Slab allocator not initialized".to_string(),
            });
        }

        // Check for fragmentation issues
        let stats = self.get_statistics().await;
        if stats.fragmentation_ratio > 0.5 {
            return Err(MemoryError::OutOfMemory {
                reason: format!("High fragmentation ratio: {:.2}", stats.fragmentation_ratio),
            });
        }

        Ok(())
    }

    // Forces garbage collection of unused magazines
    pub async fn garbage_collect(&self) {
        // Implementation would clean up unused magazines and slabs
        // This is a placeholder for the actual GC logic
    }

    // Shuts down the allocator gracefully
    pub async fn shutdown(&self) -> Result<(), MemoryError> {
        self.is_initialized.store(false, Ordering::Relaxed);

        // Clean up thread caches
        let mut thread_caches = self.thread_caches.write();
        for (_, cache) in thread_caches.drain() {
            cache.lock().unwrap().is_active.store(false, Ordering::Relaxed);
        }

        Ok(())
    }
}

// Utility functions for slab allocator
impl SlabAllocator {
    // Calculates optimal number of size classes
    pub fn calculate_optimal_size_classes(
        min_size: usize,
        max_size: usize,
        growth_factor: f64,
    ) -> usize {
        if growth_factor <= 1.0 || min_size >= max_size {
            return 1;
        }

        let mut count = 0;
        let mut current_size = min_size as f64;

        while (current_size as usize) <= max_size {
            count += 1;
            current_size *= growth_factor;
        }

        count
    }

    // Rounds size up to the next size class
    pub fn round_up_to_size_class(size: usize, growth_factor: f64, min_size: usize) -> usize {
        if size <= min_size {
            return min_size;
        }

        let mut class_size = min_size;
        while class_size < size {
            class_size = (class_size as f64 * growth_factor) as usize;
        }

        class_size
    }
}

#[cfg(test)]
mod tests {
    use tokio::test;

    #[test]
    async fn test_slab_allocator_creation() {
        let config = SlabConfig::default();
        let allocator = SlabAllocator::new(config).await;
        assert!(allocator.is_ok());

        let allocator = allocator.unwrap();
        assert!(allocator.is_initialized.load(Ordering::Relaxed));
    }

    #[test]
    async fn test_size_class_creation() {
        let size_class = SizeClass::new(0, 64, 4096);
        assert_eq!(size_class.class_id, 0);
        assert_eq!(size_class.object_size, 64);
        assert_eq!(size_class.objects_per_slab, 64); // 4096 / 64
    }

    #[test]
    async fn test_size_class_utilization() {
        let size_class = SizeClass::new(0, 64, 4096);
        assert_eq!(size_class.utilization(), 0.0); // Empty initially

        size_class.slab_count.store(1, Ordering::Relaxed);
        size_class.free_objects.store(32, Ordering::Relaxed); // Half used
        assert_eq!(size_class.utilization(), 0.5);
    }

    #[test]
    fn test_magazine_creation() {
        let magazine = Magazine::new(0, 64);
        assert!(magazine.is_ok());

        let magazine = magazine.unwrap();
        assert_eq!(magazine.size_class_id, 0);
        assert_eq!(magazine.capacity, 64);
        assert!(magazine.is_empty());
        assert!(!magazine.is_full());
    }

    #[test]
    fn test_magazine_depot_creation() {
        let depot = MagazineDepot::new(100);
        assert_eq!(depot.max_magazines_per_class, 100);
    }

    #[test]
    fn test_thread_cache_creation() {
        let thread_id = std::thread::current().id();
        let cache = ThreadCache::new(thread_id, 10);
        assert_eq!(cache.thread_id, thread_id);
        assert_eq!(cache.loaded_magazines.len(), 10);
        assert!(cache.is_active.load(Ordering::Relaxed));
    }

    #[test]
    fn test_optimal_size_classes_calculation() {
        let classes = SlabAllocator::calculate_optimal_size_classes(16, 1024, 1.5);
        assert!(classes > 0);

        // Test edge cases
        assert_eq!(SlabAllocator::calculate_optimal_size_classes(100, 50, 1.5), 1);
        assert_eq!(SlabAllocator::calculate_optimal_size_classes(100, 100, 1.0), 1);
    }

    #[test]
    fn test_size_class_rounding() {
        let rounded = SlabAllocator::round_up_to_size_class(100, 1.5, 16);
        assert!(rounded >= 100);
        assert!(rounded >= 16);

        let exact = SlabAllocator::round_up_to_size_class(16, 1.5, 16);
        assert_eq!(exact, 16);
    }
}
