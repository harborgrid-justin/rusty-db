//! Memory Zones and Buddy Allocator

use super::common::*;

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