//\! Advanced Features
//\!
//\! Memory pools, zones, buddy allocator, and utility functions.

use super::common::*;

// Memory pool for fixed-size allocations
pub struct MemoryPool {
    // Object size
    object_size: usize,
    // Pool capacity
    capacity: usize,
    // Free list
    free_list: Mutex<Vec<NonNull<u8>>>,
    // Allocated objects
    allocated: AtomicUsize,
    // Total allocations
    total_allocations: AtomicU64,
    // Total deallocations
    total_deallocations: AtomicU64,
}

impl MemoryPool {
    // Create a new memory pool
    pub fn new(object_size: usize, capacity: usize) -> Result<Self> {
        let mut free_list = Vec::with_capacity(capacity);

        // Pre-allocate all objects
        unsafe {
            let layout = Layout::from_size_align(object_size, 16)
                .map_err(|e| DbError::OutOfMemory(format!("Invalid layout: {}", e)))?;

            for _ in 0..capacity {
                let ptr = System.alloc(layout);
                if ptr.is_null() {
                    return Err(DbError::OutOfMemory(
                        "Failed to allocate pool object".to_string(),
                    ));
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

    // Allocate an object from the pool
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

    // Deallocate an object back to the pool
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

    // Get pool statistics
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

// Memory pool statistics
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
