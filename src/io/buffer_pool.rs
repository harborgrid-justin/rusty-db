// # Buffer Pool for I/O Operations
//
// Pre-allocated, aligned buffer pool for Direct I/O operations.

use std::fmt;
use crate::error::Result;
use crate::io::{PAGE_SIZE, SECTOR_SIZE};
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;
use std::sync::Arc;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Buffer Pool Configuration
// ============================================================================

/// Buffer pool configuration
#[derive(Debug, Clone)]
pub struct BufferPoolConfig {
    /// Number of buffers in the pool
    pub pool_size: usize,

    /// Size of each buffer in bytes
    pub buffer_size: usize,

    /// Alignment requirement (must be power of 2)
    pub alignment: usize,

    /// Enable statistics collection
    pub enable_stats: bool,
}

impl Default for BufferPoolConfig {
    fn default() -> Self {
        Self {
            pool_size: 1024,
            buffer_size: PAGE_SIZE,
            alignment: SECTOR_SIZE,
            enable_stats: true,
        }
    }
}

// ============================================================================
// Buffer Allocation Strategy
// ============================================================================

/// Buffer allocation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferAllocationStrategy {
    /// Allocate from pool if available, otherwise allocate new
    PoolOrNew,

    /// Only allocate from pool (fail if pool is empty)
    PoolOnly,

    /// Always allocate new buffer (bypass pool)
    AlwaysNew,
}

// ============================================================================
// Aligned Buffer
// ============================================================================

/// Page-aligned buffer for Direct I/O
pub struct AlignedBuffer {
    /// Pointer to buffer data
    ptr: *mut u8,

    /// Buffer size
    size: usize,

    /// Layout for deallocation
    layout: Layout,

    /// Whether this buffer is from pool (for return)
    from_pool: bool,

    /// Pool reference (for returning)
    pool: Option<Arc<BufferPool>>,
}

impl AlignedBuffer {
    /// Create a new aligned buffer
    pub fn new(size: usize, alignment: usize) -> Result<Self> {
        let layout = Layout::from_size_align(size, alignment)
            .map_err(|_| DbError::Internal("Invalid buffer layout".to_string()))?;

        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            return Err(DbError::Internal("Buffer allocation failed".to_string()));
        }

        // Zero the buffer
        unsafe {
            ptr::write_bytes(ptr, 0, size);
        }

        Ok(Self {
            ptr,
            size,
            layout,
            from_pool: false,
            pool: None,
        })
    }

    /// Create from existing pointer (for pool)
    pub(crate) fn from_ptr(
        ptr: *mut u8,
        size: usize,
        layout: Layout,
        pool: Arc<BufferPool>,
    ) -> Self {
        Self {
            ptr,
            size,
            layout,
            from_pool: true,
            pool: Some(pool),
        }
    }

    /// Get buffer as slice
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.size) }
    }

    /// Get buffer as mutable slice
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size) }
    }

    /// Get raw pointer
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Get raw mutable pointer
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    /// Get buffer size
    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    /// Check if buffer is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Fill buffer with zeros
    pub fn zero(&mut self) {
        unsafe {
            ptr::write_bytes(self.ptr, 0, self.size);
        }
    }

    /// Fill buffer with a value
    pub fn fill(&mut self, value: u8) {
        unsafe {
            ptr::write_bytes(self.ptr, value, self.size);
        }
    }

    /// Copy data into buffer
    pub fn copy_from_slice(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > self.size {
            return Err(DbError::Internal(format!(
                "Data too large for buffer: {} > {}",
                data.len(),
                self.size
            )))));
        }

        self.as_mut_slice()[..data.len()].copy_from_slice(data);
        Ok(())
    }

    /// Leak the buffer (caller responsible for deallocation)
    pub fn leak(mut self) -> *mut u8 {
        let ptr = self.ptr;
        self.ptr = ptr::null_mut(); // Prevent drop
        ptr
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }

        if self.from_pool {
            // Return to pool
            if let Some(pool) = &self.pool {
                pool.return_buffer(self.ptr, self.size, self.layout);
                self.ptr = ptr::null_mut(); // Prevent deallocation
            }
        } else {
            // Deallocate
            unsafe {
                dealloc(self.ptr, self.layout);
            }
        }
    }
}

// Safety: Can be sent between threads
unsafe impl Send for AlignedBuffer {}
unsafe impl Sync for AlignedBuffer {}

impl std::fmt::Debug for AlignedBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlignedBuffer")
            .field("ptr", &self.ptr)
            .field("size", &self.size)
            .field("from_pool", &self.from_pool)
            .finish()
    }
}

// ============================================================================
// Pooled Buffer
// ============================================================================

/// Reference to a buffer in the pool
pub struct PooledBuffer {
    /// Pointer to buffer
    ptr: *mut u8,

    /// Buffer size
    size: usize,

    /// Layout
    layout: Layout,

    /// Whether buffer is currently in use
    in_use: bool,
}

// ============================================================================
// Buffer Pool
// ============================================================================

/// Pre-allocated buffer pool for I/O operations
pub struct BufferPool {
    /// Pool of buffers
    buffers: Arc<Mutex<Vec<PooledBuffer>>>,

    /// Configuration
    config: BufferPoolConfig,

    /// Statistics
    stats: Arc<Mutex<BufferPoolStats>>,

    /// Number of allocated buffers
    allocated: AtomicUsize,

    /// Number of available buffers
    available: AtomicUsize,
}

impl BufferPool {
    /// Create a new buffer pool
    pub fn new(config: BufferPoolConfig) -> Result<Self> {
        let mut buffers: Vec<PooledBuffer> = Vec::with_capacity(config.pool_size);

        // Pre-allocate all buffers
        for _ in 0..config.pool_size {
            let layout = Layout::from_size_align(config.buffer_size, config.alignment)
                .map_err(|_| DbError::Internal("Invalid buffer layout".to_string()))?;

            let ptr = unsafe { alloc(layout) };
            if ptr.is_null() {
                // Clean up already allocated buffers
                for buffer in &buffers {
                    unsafe {
                        dealloc(buffer.ptr, buffer.layout);
                    }
                }
                return Err(DbError::Internal("Buffer pool allocation failed".to_string()));
            }

            // Zero the buffer
            unsafe {
                ptr::write_bytes(ptr, 0, config.buffer_size);
            }

            buffers.push(PooledBuffer {
                ptr,
                size: config.buffer_size,
                layout,
                in_use: false,
            });
        }

        let pool_size = config.pool_size;

        Ok(Self {
            buffers: Arc::new(Mutex::new(buffers)),
            config,
            stats: Arc::new(Mutex::new(BufferPoolStats::default())),
            allocated: AtomicUsize::new(0),
            available: AtomicUsize::new(pool_size),
        })
    }

    /// Allocate a buffer from the pool
    pub fn allocate(&self, size: usize) -> Result<AlignedBuffer> {
        if size > self.config.buffer_size {
            // Size exceeds pool buffer size, allocate new
            if self.config.enable_stats {
                self.stats.lock().oversized_allocations += 1;
            }
            return AlignedBuffer::new(size, self.config.alignment);
        }

        let mut buffers = self.buffers.lock();

        // Find an available buffer
        for buffer in buffers.iter_mut() {
            if !buffer.in_use {
                buffer.in_use = true;

                self.allocated.fetch_add(1, Ordering::Relaxed);
                self.available.fetch_sub(1, Ordering::Relaxed);

                if self.config.enable_stats {
                    let mut stats = self.stats.lock();
                    stats.allocations += 1;
                    stats.pool_hits += 1;
                }

                return Ok(AlignedBuffer::from_ptr(
                    buffer.ptr,
                    buffer.size,
                    buffer.layout,
                    Arc::new(Self {
                        buffers: self.buffers.clone(),
                        config: self.config.clone(),
                        stats: self.stats.clone(),
                        allocated: AtomicUsize::new(self.allocated.load(Ordering::Relaxed)),
                        available: AtomicUsize::new(self.available.load(Ordering::Relaxed)),
                    }),
                ));
            }
        }

        // No available buffers, allocate new
        if self.config.enable_stats {
            let mut stats = self.stats.lock();
            stats.allocations += 1;
            stats.pool_misses += 1;
        }

        AlignedBuffer::new(size, self.config.alignment)
    }

    /// Allocate with specific strategy
    pub fn allocate_with_strategy(
        &self,
        size: usize,
        strategy: BufferAllocationStrategy,
    ) -> Result<AlignedBuffer> {
        match strategy {
            BufferAllocationStrategy::PoolOrNew => self.allocate(size),
            BufferAllocationStrategy::PoolOnly => {
                let mut buffers = self.buffers.lock();
                for buffer in buffers.iter_mut() {
                    if !buffer.in_use && buffer.size >= size {
                        buffer.in_use = true;
                        self.allocated.fetch_add(1, Ordering::Relaxed);
                        self.available.fetch_sub(1, Ordering::Relaxed);

                        if self.config.enable_stats {
                            let mut stats = self.stats.lock();
                            stats.allocations += 1;
                            stats.pool_hits += 1;
                        }

                        return Ok(AlignedBuffer::from_ptr(
                            buffer.ptr,
                            buffer.size,
                            buffer.layout,
                            Arc::new(Self {
                                buffers: self.buffers.clone(),
                                config: self.config.clone(),
                                stats: self.stats.clone(),
                                allocated: AtomicUsize::new(self.allocated.load(Ordering::Relaxed)),
                                available: AtomicUsize::new(self.available.load(Ordering::Relaxed)),
                            }),
                        ));
                    }
                }
                Err(DbError::Internal("No available buffers in pool".to_string()))
            }
            BufferAllocationStrategy::AlwaysNew => {
                if self.config.enable_stats {
                    self.stats.lock().direct_allocations += 1;
                }
                AlignedBuffer::new(size, self.config.alignment)
            }
        }
    }

    /// Return a buffer to the pool
    fn return_buffer(&self, ptr: *mut u8, size: usize, _layout: Layout) {
        let mut buffers = self.buffers.lock();

        for buffer in buffers.iter_mut() {
            if buffer.ptr == ptr && buffer.in_use {
                buffer.in_use = false;

                // Zero the buffer for security
                unsafe {
                    ptr::write_bytes(buffer.ptr, 0, buffer.size);
                }

                self.allocated.fetch_sub(1, Ordering::Relaxed);
                self.available.fetch_add(1, Ordering::Relaxed);

                if self.config.enable_stats {
                    self.stats.lock().deallocations += 1;
                }

                return;
            }
        }

        // Buffer not from this pool, ignore
    }

    /// Get number of allocated buffers
    #[inline]
    pub fn allocated_count(&self) -> usize {
        self.allocated.load(Ordering::Relaxed)
    }

    /// Get number of available buffers
    #[inline]
    pub fn available_count(&self) -> usize {
        self.available.load(Ordering::Relaxed)
    }

    /// Get total pool size
    #[inline]
    pub fn total_size(&self) -> usize {
        self.config.pool_size
    }

    /// Get utilization percentage
    #[inline]
    pub fn utilization(&self) -> f64 {
        let allocated = self.allocated.load(Ordering::Relaxed);
        (allocated as f64 / self.config.pool_size as f64) * 100.0
    }

    /// Get statistics
    pub fn stats(&self) -> BufferPoolStats {
        self.stats.lock().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = BufferPoolStats::default();
    }

    /// Clear all buffers (mark all as available)
    pub fn clear(&self) {
        let mut buffers = self.buffers.lock();
        for buffer in buffers.iter_mut() {
            buffer.in_use = false;

            // Zero the buffer
            unsafe {
                ptr::write_bytes(buffer.ptr, 0, buffer.size);
            }
        }

        self.allocated.store(0, Ordering::Relaxed);
        self.available.store(self.config.pool_size, Ordering::Relaxed);
    }
}

impl Drop for BufferPool {
    fn drop(&mut self) {
        let mut buffers = self.buffers.lock();
        for buffer in buffers.drain(..) {
            unsafe {
                dealloc(buffer.ptr, buffer.layout);
            }
        }
    }
}

// Safety: Can be shared between threads
unsafe impl Send for BufferPool {}
unsafe impl Sync for BufferPool {}

// ============================================================================
// Statistics
// ============================================================================

/// Buffer pool statistics
#[derive(Debug, Clone, Default)]
pub struct BufferPoolStats {
    /// Number of allocations
    pub allocations: u64,

    /// Number of deallocations
    pub deallocations: u64,

    /// Number of pool hits (allocated from pool)
    pub pool_hits: u64,

    /// Number of pool misses (allocated new)
    pub pool_misses: u64,

    /// Number of oversized allocations
    pub oversized_allocations: u64,

    /// Number of direct allocations (bypassing pool)
    pub direct_allocations: u64,
}

impl BufferPoolStats {
    /// Get pool hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.pool_hits + self.pool_misses;
        if total == 0 {
            0.0
        } else {
            (self.pool_hits as f64 / total as f64) * 100.0
        }
    }

    /// Get currently outstanding allocations
    pub fn outstanding(&self) -> i64 {
        self.allocations as i64 - self.deallocations as i64
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_buffer() {
        let mut buffer = AlignedBuffer::new(PAGE_SIZE, SECTOR_SIZE).unwrap();
        assert_eq!(buffer.len(), PAGE_SIZE);
        assert!(!buffer.is_empty());

        // Test zero
        buffer.fill(0xFF);
        buffer.zero();
        assert_eq!(buffer.as_slice()[0], 0);

        // Test fill
        buffer.fill(0xAA);
        assert_eq!(buffer.as_slice()[0], 0xAA);

        // Test copy
        let data = vec![1, 2, 3, 4];
        buffer.copy_from_slice(&data).unwrap();
        assert_eq!(&buffer.as_slice()[..4], &[1, 2, 3, 4]);
    }

    #[test]
    fn test_buffer_pool_allocation() {
        let pool = BufferPool::new(BufferPoolConfig {
            pool_size: 4,
            buffer_size: PAGE_SIZE,
            alignment: SECTOR_SIZE,
            enable_stats: true,
        })
        .unwrap();

        assert_eq!(pool.available_count(), 4);
        assert_eq!(pool.allocated_count(), 0);

        // Allocate buffers
        let buf1 = pool.allocate(PAGE_SIZE).unwrap();
        assert_eq!(pool.allocated_count(), 1);
        assert_eq!(pool.available_count(), 3);

        let buf2 = pool.allocate(PAGE_SIZE).unwrap();
        assert_eq!(pool.allocated_count(), 2);

        // Drop a buffer (should return to pool)
        drop(buf1);
        assert_eq!(pool.allocated_count(), 1);
        assert_eq!(pool.available_count(), 3);

        drop(buf2);
        assert_eq!(pool.allocated_count(), 0);
        assert_eq!(pool.available_count(), 4);
    }

    #[test]
    fn test_buffer_pool_exhaustion() {
        let pool = BufferPool::new(BufferPoolConfig {
            pool_size: 2,
            buffer_size: PAGE_SIZE,
            alignment: SECTOR_SIZE,
            enable_stats: true,
        })
        .unwrap();

        let buf1 = pool.allocate(PAGE_SIZE).unwrap();
        let buf2 = pool.allocate(PAGE_SIZE).unwrap();

        // Pool is exhausted, should allocate new
        let buf3 = pool.allocate(PAGE_SIZE).unwrap();
        assert!(buf3.len() > 0);

        let stats = pool.stats();
        assert_eq!(stats.pool_hits, 2);
        assert_eq!(stats.pool_misses, 1);
    }

    #[test]
    fn test_buffer_pool_strategies() {
        let pool = BufferPool::new(BufferPoolConfig {
            pool_size: 2,
            buffer_size: PAGE_SIZE,
            alignment: SECTOR_SIZE,
            enable_stats: false,
        })
        .unwrap();

        // PoolOrNew strategy
        let buf1 = pool
            .allocate_with_strategy(PAGE_SIZE, BufferAllocationStrategy::PoolOrNew)
            .unwrap();

        // AlwaysNew strategy
        let buf2 = pool
            .allocate_with_strategy(PAGE_SIZE, BufferAllocationStrategy::AlwaysNew)
            .unwrap();
        assert!(buf2.len() > 0);

        // Exhaust pool
        let buf3 = pool.allocate(PAGE_SIZE).unwrap();
        let _buf4 = pool.allocate(PAGE_SIZE).unwrap();

        // PoolOnly should fail when exhausted
        let result = pool.allocate_with_strategy(PAGE_SIZE, BufferAllocationStrategy::PoolOnly);
        assert!(result.is_err());
    }

    #[test]
    fn test_buffer_pool_stats() {
        let pool = BufferPool::new(BufferPoolConfig {
            pool_size: 4,
            buffer_size: PAGE_SIZE,
            alignment: SECTOR_SIZE,
            enable_stats: true,
        })
        .unwrap();

        let buf1 = pool.allocate(PAGE_SIZE).unwrap();
        let buf2 = pool.allocate(PAGE_SIZE).unwrap();

        let stats = pool.stats();
        assert_eq!(stats.allocations, 2);
        assert_eq!(stats.pool_hits, 2);
        assert_eq!(stats.hit_rate(), 100.0);

        drop(_buf1);
        drop(_buf2);

        let stats = pool.stats();
        assert_eq!(stats.deallocations, 2);
        assert_eq!(stats.outstanding(), 0);
    }

    #[test]
    fn test_buffer_pool_utilization() {
        let pool = BufferPool::new(BufferPoolConfig {
            pool_size: 10,
            buffer_size: PAGE_SIZE,
            alignment: SECTOR_SIZE,
            enable_stats: false,
        })
        .unwrap();

        assert_eq!(pool.utilization(), 0.0);

        let buf1 = pool.allocate(PAGE_SIZE).unwrap();
        let buf2 = pool.allocate(PAGE_SIZE).unwrap();
        let buf3 = pool.allocate(PAGE_SIZE).unwrap();

        assert_eq!(pool.utilization(), 30.0);
    }
}
