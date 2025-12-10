//\! Slab Allocator Implementation
//\!
//\! Fixed-size allocation with thread-local caching and magazine-layer optimization.

use super::common::*;


// Size class information for slab allocation
#[derive(Debug, Clone)]
struct SizeClass {
    // Object size for this class
    object_size: usize,
    // Number of objects per slab
    objects_per_slab: usize,
    // Color offset for cache optimization
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

// Slab metadata
struct Slab {
    // Base pointer to slab memory
    base: NonNull<u8>,
    // Size class index
    size_class: usize,
    // Number of free objects
    free_count: usize,
    // Freelist head
    freelist: Option<NonNull<u8>>,
    // Slab color for cache optimization
    color: usize,
    // Allocation timestamp
    allocated_at: Instant,
}

impl Slab {
    // Create a new slab for the given size class
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

        for i in 0..objects_per_slab {
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

    // Allocate an object from this slab
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

    // Free an object back to this slab (part of slab allocator API)
    #[allow(dead_code)]
    unsafe fn deallocate(&mut self, ptr: NonNull<u8>) {
        // Push to freelist
        *(ptr.as_ptr() as *mut *mut u8) = self.freelist.map_or(ptr::null_mut(), |p| p.as_ptr());
        self.freelist = Some(ptr);
        self.free_count += 1;
    }

    // Check if slab is full (for allocation decisions)
    #[allow(dead_code)]
    fn is_full(&self) -> bool {
        self.free_count == 0
    }

    // Check if slab is empty (for deallocation decisions)
    #[allow(dead_code)]
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

// Magazine for CPU-level caching
struct Magazine {
    // Cached objects
    objects: Vec<NonNull<u8>>,
    // Capacity of magazine
    capacity: usize,
    // Size class this magazine belongs to
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

    // Try to allocate from magazine
    fn allocate(&mut self) -> Option<NonNull<u8>> {
        self.objects.pop()
    }

    // Try to free to magazine
    fn deallocate(&mut self, ptr: NonNull<u8>) -> bool {
        if self.objects.len() < self.capacity {
            self.objects.push(ptr);
            true
        } else {
            false
        }
    }

    /// Check if magazine is full (for magazine layer)
    #[allow(dead_code)]
    fn is_full(&self) -> bool {
        self.objects.len() >= self.capacity
    }

    /// Check if magazine is empty (for magazine layer)
    #[allow(dead_code)]
    fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
}

// Per-thread slab cache
struct ThreadLocalCache {
    // Loaded magazine per size class
    loaded_magazines: Vec<Option<Magazine>>,
    // Previous magazine per size class
    previous_magazines: Vec<Option<Magazine>>,
    // Thread ID
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

    /// Ensure thread-local cache is initialized (for thread-local allocation)
    #[allow(dead_code)]
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

    /// Execute function with thread-local cache (for magazine layer)
    #[allow(dead_code)]
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

// Slab depot - central storage for magazines and slabs
struct SlabDepot {
    // Full magazines per size class
    full_magazines: Vec<VecDeque<Magazine>>,
    // Empty magazines per size class
    empty_magazines: Vec<VecDeque<Magazine>>,
    // Partial slabs per size class
    partial_slabs: Vec<VecDeque<Slab>>,
    // Empty slabs per size class
    empty_slabs: Vec<VecDeque<Slab>>,
    // Full slabs count per size class (for tracking)
    full_slab_counts: Vec<usize>,
    // Current color per size class (for slab coloring)
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

    /// Return empty magazine to depot (for magazine recycling)
    #[allow(dead_code)]
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

// Main slab allocator
pub struct SlabAllocator {
    // Size class configurations
    size_classes: Vec<SizeClass>,
    // Central depot (protected by lock)
    depot: Mutex<SlabDepot>,
    // Allocation statistics
    stats: SlabStats,
}

// Slab allocator statistics
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
    // Create a new slab allocator
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

    // Get size class index for a given size
    fn size_to_class(&self, size: usize) -> Option<usize> {
        self.size_classes
            .iter()
            .position(|sc| sc.object_size >= size)
    }

    // Allocate memory from slab allocator
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

    // Internal allocation from specific size class
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

    // Deallocate memory back to slab allocator
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

    // Get allocation statistics
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

    // Calculate fragmentation for a size class
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

// Public slab allocator statistics
#[derive(Debug, Clone)]
pub struct SlabAllocatorStats {
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub slab_allocations: u64,
    pub magazine_loads: u64,
    pub magazine_unloads: u64,
    pub bytes_allocated: u64,
}
