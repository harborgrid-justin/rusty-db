//\! Large Object Allocator Implementation
//\!
//\! Direct mmap for huge allocations with huge page support.

use super::common::*;

// Large object metadata
struct LargeObject {
    // Base address
    base: NonNull<u8>,
    // Size of allocation
    size: usize,
    // Whether huge pages are used
    huge_pages: bool,
    // Huge page size used (if any)
    huge_page_size: usize,
    // Whether copy-on-write is enabled
    cow: bool,
    // Allocation timestamp
    allocated_at: Instant,
}

impl LargeObject {
    // Allocate a large object using mmap
    unsafe fn allocate(size: usize, use_huge_pages: bool, cow: bool) -> Result<Self> {
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

    // Enable lazy allocation (on-demand paging)
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

    // Prefault pages (force allocation)
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

    // Mark as sequential access
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

// Large object allocator
pub struct LargeObjectAllocator {
    // Active large objects
    objects: RwLock<HashMap<usize, LargeObject>>,
    // Statistics
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
    // Create a new large object allocator
    pub fn new() -> Self {
        Self {
            objects: RwLock::new(HashMap::new()),
            stats: LargeObjectStats::new(),
        }
    }

    // Allocate a large object
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

    // Deallocate a large object
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

    // Enable lazy allocation for an object
    pub fn enable_lazy_allocation(&self, ptr: NonNull<u8>) -> Result<()> {
        let addr = ptr.as_ptr() as usize;
        let objects = self.objects.read().unwrap();

        if let Some(obj) = objects.get(&addr) {
            unsafe { obj.enable_lazy_allocation() }
        } else {
            Err(DbError::InvalidArgument("Unknown large object pointer".to_string()))
        }
    }

    // Prefault pages for an object
    pub fn prefault(&self, ptr: NonNull<u8>) -> Result<()> {
        let addr = ptr.as_ptr() as usize;
        let objects = self.objects.read().unwrap();

        if let Some(obj) = objects.get(&addr) {
            unsafe { obj.prefault() }
        } else {
            Err(DbError::InvalidArgument("Unknown large object pointer".to_string()))
        }
    }

    // Set sequential access pattern
    pub fn set_sequential(&self, ptr: NonNull<u8>) -> Result<()> {
        let addr = ptr.as_ptr() as usize;
        let objects = self.objects.read().unwrap();

        if let Some(obj) = objects.get(&addr) {
            unsafe { obj.set_sequential() }
        } else {
            Err(DbError::InvalidArgument("Unknown large object pointer".to_string()))
        }
    }

    // Get statistics
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

// Public large object allocator statistics
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
