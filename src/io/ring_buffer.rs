// # Lock-Free Ring Buffer for I/O Queue
//
// High-performance, lock-free ring buffer for I/O request submission
// and completion queues.

use crate::error::Result;
use crate::io::{IoOpType, IoRequest, IoCompletion, IoHandle};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;

// ============================================================================
// Ring Buffer Configuration
// ============================================================================

/// Ring buffer configuration
#[derive(Debug, Clone)]
pub struct RingBufferConfig {
    /// Number of entries (must be power of 2)
    pub size: usize,

    /// Enable statistics collection
    pub enable_stats: bool,

    /// Enable memory prefetching
    pub enable_prefetch: bool,
}

impl Default for RingBufferConfig {
    fn default() -> Self {
        Self {
            size: 4096,
            enable_stats: true,
            enable_prefetch: true,
        }
    }
}

// ============================================================================
// Ring Buffer Error
// ============================================================================

/// Ring buffer error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum RingBufferError {
    #[error("Ring buffer is full")]
    Full,

    #[error("Ring buffer is empty")]
    Empty,

    #[error("Invalid size: must be power of 2")]
    InvalidSize,

    #[error("Allocation failed")]
    AllocationFailed,
}

// ============================================================================
// Submission Entry
// ============================================================================

/// Submission queue entry
#[repr(C, align(64))]
#[derive(Debug)]
pub struct SubmissionEntry {
    /// Operation type
    pub op_type: IoOpType,

    /// File handle
    pub file_handle: IoHandle,

    /// File offset
    pub offset: u64,

    /// Buffer pointer
    pub buffer: *mut u8,

    /// Length
    pub len: u32,

    /// Flags
    pub flags: u32,

    /// User data
    pub user_data: u64,

    /// Padding to 64 bytes
    _padding: [u8; 16],
}

impl SubmissionEntry {
    /// Create a new submission entry
    pub fn new(
        op_type: IoOpType,
        file_handle: IoHandle,
        offset: u64,
        buffer: *mut u8,
        len: u32,
    ) -> Self {
        Self {
            op_type,
            file_handle,
            offset,
            buffer,
            len,
            flags: 0,
            user_data: 0,
            _padding: [0; 16],
        }
    }

    /// Create from IoRequest
    pub fn from_request(request: &IoRequest) -> Self {
        Self {
            op_type: request.op_type,
            file_handle: request.file_handle,
            offset: request.offset,
            buffer: request.buffer,
            len: request.len,
            flags: 0,
            user_data: request.id,
            _padding: [0; 16],
        }
    }
}

// Safety: Can be sent between threads
unsafe impl Send for SubmissionEntry {}
unsafe impl Sync for SubmissionEntry {}

// ============================================================================
// Completion Entry
// ============================================================================

/// Completion queue entry
#[repr(C, align(64))]
#[derive(Debug, Clone)]
pub struct CompletionEntry {
    /// User data from submission
    pub user_data: u64,

    /// Result (bytes transferred or error code)
    pub result: i32,

    /// Flags
    pub flags: u32,

    /// Padding to 64 bytes
    _padding: [u8; 48],
}

impl CompletionEntry {
    /// Create a new completion entry
    pub fn new(user_data: u64, result: i32, flags: u32) -> Self {
        Self {
            user_data,
            result,
            flags,
            _padding: [0; 48],
        }
    }

    /// Create from IoCompletion
    pub fn from_completion(completion: &IoCompletion) -> Self {
        let _result = if completion.is_success() {
            completion.bytes_transferred as i32
        } else {
            -(completion.error_code as i32)
        };

        Self {
            user_data: completion.id,
            result,
            flags: 0,
            _padding: [0; 48],
        }
    }

    /// Check if operation was successful
    #[inline]
    pub fn is_success(&self) -> bool {
        self.result >= 0
    }

    /// Get bytes transferred (if successful)
    #[inline]
    pub fn bytes_transferred(&self) -> Option<usize> {
        if self.result >= 0 {
            Some(self.result as usize)
        } else {
            None
        }
    }

    /// Get error code (if failed)
    #[inline]
    pub fn error_code(&self) -> Option<i32> {
        if self.result < 0 {
            Some(-self.result)
        } else {
            None
        }
    }
}

// ============================================================================
// I/O Ring Buffer
// ============================================================================

/// Lock-free ring buffer for I/O operations
///
/// This ring buffer uses atomic operations for lock-free concurrent access.
/// The buffer size must be a power of 2 for efficient masking.
#[repr(C, align(64))]
pub struct IoRingBuffer<T> {
    /// Head index (consumer)
    head: AtomicU64,

    /// Tail index (producer)
    tail: AtomicU64,

    /// Mask for wrapping (size - 1)
    mask: u64,

    /// Buffer size
    size: usize,

    /// Array of entries
    entries: *mut T,

    /// Layout for deallocation
    layout: Layout,

    /// Statistics
    stats: Arc<parking_lot::Mutex<RingBufferStats>>,

    /// Configuration
    config: RingBufferConfig,
}

impl<T> IoRingBuffer<T> {
    /// Create a new ring buffer
    pub fn new(config: RingBufferConfig) -> Result<Self> {
        // Check that size is power of 2
        if !config.size.is_power_of_two() {
            return Err(DbError::Internal(
                RingBufferError::InvalidSize.to_string(),
            ));
        }

        // Allocate aligned memory
        let layout = Layout::from_size_align(
            size_of::<T>() * config.size,
            64, // Cache line alignment
        )
        .map_err(|_| DbError::Internal(RingBufferError::AllocationFailed.to_string()))?;

        let entries = unsafe { alloc(layout) as *mut T };
        if entries.is_null() {
            return Err(DbError::Internal(RingBufferError::AllocationFailed.to_string()));
        }

        // Initialize entries with zeros
        unsafe {
            ptr::write_bytes(entries, 0, config.size);
        }

        Ok(Self {
            head: AtomicU64::new(0),
            tail: AtomicU64::new(0),
            mask: (config.size - 1) as u64,
            size: config.size,
            entries,
            layout,
            stats: Arc::new(parking_lot::Mutex::new(RingBufferStats::default())),
            config,
        })
    }

    /// Get the number of entries in the buffer
    #[inline]
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        tail.wrapping_sub(head) as usize
    }

    /// Check if buffer is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Acquire) == self.tail.load(Ordering::Acquire)
    }

    /// Check if buffer is full
    #[inline]
    pub fn is_full(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        tail.wrapping_sub(head) >= self.size as u64
    }

    /// Get available space
    #[inline]
    pub fn available(&self) -> usize {
        self.size - self.len()
    }

    /// Get buffer capacity
    #[inline]
    pub fn capacity(&self) -> usize {
        self.size
    }

    /// Push an entry (returns false if full)
    #[inline]
    pub fn push(&self, entry: T) -> bool
    where
        T: Copy,
    {
        loop {
            let tail = self.tail.load(Ordering::Acquire);
            let head = self.head.load(Ordering::Acquire);

            // Check if full
            if tail.wrapping_sub(head) >= self.size as u64 {
                if self.config.enable_stats {
                    self.stats.lock().push_failures += 1;
                }
                return false;
            }

            // Try to claim this slot
            if self
                .tail
                .compare_exchange(tail, tail + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                // Write the entry
                let index = (tail & self.mask) as usize;
                unsafe {
                    ptr::write(self.entries.add(index), entry);
                }

                if self.config.enable_stats {
                    self.stats.lock().pushes += 1;
                }

                return true;
            }
        }
    }

    /// Pop an entry (returns None if empty)
    #[inline]
    pub fn pop(&self) -> Option<T>
    where
        T: Copy,
    {
        loop {
            let head = self.head.load(Ordering::Acquire);
            let tail = self.tail.load(Ordering::Acquire);

            // Check if empty
            if head == tail {
                if self.config.enable_stats {
                    self.stats.lock().pop_failures += 1;
                }
                return None;
            }

            // Try to claim this slot
            if self
                .head
                .compare_exchange(head, head + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                // Read the entry
                let index = (head & self.mask) as usize;
                let entry = unsafe { ptr::read(self.entries.add(index)) };

                if self.config.enable_stats {
                    self.stats.lock().pops += 1;
                }

                return Some(entry);
            }
        }
    }

    /// Batch push multiple entries
    pub fn push_batch(&self, entries: &[T]) -> usize
    where
        T: Copy,
    {
        let mut pushed = 0;

        for entry in entries {
            if !self.push(*entry) {
                break;
            }
            pushed += 1;
        }

        if self.config.enable_stats && pushed > 0 {
            self.stats.lock().batch_pushes += 1;
            self.stats.lock().batch_push_total += pushed as u64;
        }

        pushed
    }

    /// Batch pop multiple entries
    pub fn pop_batch(&self, max_entries: usize) -> Vec<T>
    where
        T: Copy,
    {
        let mut result = Vec::with_capacity(max_entries);

        for _ in 0..max_entries {
            if let Some(entry) = self.pop() {
                result.push(entry);
            } else {
                break;
            }
        }

        if self.config.enable_stats && !result.is_empty() {
            self.stats.lock().batch_pops += 1;
            self.stats.lock().batch_pop_total += result.len() as u64;
        }

        result
    }

    /// Peek at the next entry without removing it
    #[inline]
    pub fn peek(&self) -> Option<T>
    where
        T: Copy,
    {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);

        if head == tail {
            return None;
        }

        let index = (head & self.mask) as usize;
        Some(unsafe { ptr::read(self.entries.add(index)) })
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.head.store(0, Ordering::Release);
        self.tail.store(0, Ordering::Release);

        if self.config.enable_stats {
            self.stats.lock().clears += 1;
        }
    }

    /// Get statistics
    pub fn stats(&self) -> RingBufferStats {
        self.stats.lock().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = RingBufferStats::default();
    }
}

impl<T> Drop for IoRingBuffer<T> {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.entries as *mut u8, self.layout);
        }
    }
}

// Safety: Can be sent between threads if T is Send
unsafe impl<T: Send> Send for IoRingBuffer<T> {}
unsafe impl<T: Sync> Sync for IoRingBuffer<T> {}

// ============================================================================
// Statistics
// ============================================================================

/// Ring buffer statistics
#[derive(Debug, Clone, Default)]
pub struct RingBufferStats {
    /// Number of successful pushes
    pub pushes: u64,

    /// Number of failed pushes (buffer full)
    pub push_failures: u64,

    /// Number of successful pops
    pub pops: u64,

    /// Number of failed pops (buffer empty)
    pub pop_failures: u64,

    /// Number of batch pushes
    pub batch_pushes: u64,

    /// Total entries in batch pushes
    pub batch_push_total: u64,

    /// Number of batch pops
    pub batch_pops: u64,

    /// Total entries in batch pops
    pub batch_pop_total: u64,

    /// Number of clears
    pub clears: u64,
}

impl RingBufferStats {
    /// Get average batch push size
    pub fn avg_batch_push_size(&self) -> f64 {
        if self.batch_pushes == 0 {
            0.0
        } else {
            self.batch_push_total as f64 / self.batch_pushes as f64
        }
    }

    /// Get average batch pop size
    pub fn avg_batch_pop_size(&self) -> f64 {
        if self.batch_pops == 0 {
            0.0
        } else {
            self.batch_pop_total as f64 / self.batch_pops as f64
        }
    }

    /// Get push success rate
    pub fn push_success_rate(&self) -> f64 {
        let total = self.pushes + self.push_failures;
        if total == 0 {
            0.0
        } else {
            self.pushes as f64 / total as f64
        }
    }

    /// Get pop success rate
    pub fn pop_success_rate(&self) -> f64 {
        let total = self.pops + self.pop_failures;
        if total == 0 {
            0.0
        } else {
            self.pops as f64 / total as f64
        }
    }
}

// ============================================================================
// Specialized Ring Buffers
// ============================================================================

/// Submission queue ring buffer
pub type SubmissionQueue = IoRingBuffer<SubmissionEntry>;

/// Completion queue ring buffer
pub type CompletionQueue = IoRingBuffer<CompletionEntry>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_basic() {
        let rb = IoRingBuffer::<u64>::new(RingBufferConfig {
            size: 16,
            enable_stats: true,
            enable_prefetch: false,
        })
        .unwrap();

        assert!(rb.is_empty());
        assert_eq!(rb.len(), 0);
        assert_eq!(rb.capacity(), 16);

        // Push some entries
        assert!(rb.push(1));
        assert!(rb.push(2));
        assert!(rb.push(3));
        assert_eq!(rb.len(), 3);
        assert!(!rb.is_empty());

        // Pop entries
        assert_eq!(rb.pop(), Some(1));
        assert_eq!(rb.pop(), Some(2));
        assert_eq!(rb.pop(), Some(3));
        assert_eq!(rb.pop(), None);
        assert!(rb.is_empty());
    }

    #[test]
    fn test_ring_buffer_wrap() {
        let rb = IoRingBuffer::<u64>::new(RingBufferConfig {
            size: 4,
            enable_stats: false,
            enable_prefetch: false,
        })
        .unwrap();

        // Fill the buffer
        for _i in 0..4 {
            assert!(rb.push(i));
        }
        assert!(rb.is_full());
        assert!(!rb.push(100)); // Should fail

        // Drain half
        assert_eq!(rb.pop(), Some(0));
        assert_eq!(rb.pop(), Some(1));

        // Fill again (wrapping)
        assert!(rb.push(100));
        assert!(rb.push(101));
        assert!(rb.is_full());

        // Verify order
        assert_eq!(rb.pop(), Some(2));
        assert_eq!(rb.pop(), Some(3));
        assert_eq!(rb.pop(), Some(100));
        assert_eq!(rb.pop(), Some(101));
        assert!(rb.is_empty());
    }

    #[test]
    fn test_batch_operations() {
        let rb = IoRingBuffer::<u64>::new(RingBufferConfig {
            size: 16,
            enable_stats: true,
            enable_prefetch: false,
        })
        .unwrap();

        let entries = vec![1, 2, 3, 4, 5];
        let pushed = rb.push_batch(&entries);
        assert_eq!(pushed, 5);
        assert_eq!(rb.len(), 5);

        let popped = rb.pop_batch(3);
        assert_eq!(popped, vec![1, 2, 3]);
        assert_eq!(rb.len(), 2);

        let remaining = rb.pop_batch(10);
        assert_eq!(remaining, vec![4, 5]);
        assert!(rb.is_empty());
    }

    #[test]
    fn test_peek() {
        let rb = IoRingBuffer::<u64>::new(RingBufferConfig {
            size: 8,
            enable_stats: false,
            enable_prefetch: false,
        })
        .unwrap();

        assert_eq!(rb.peek(), None);

        rb.push(42);
        assert_eq!(rb.peek(), Some(42));
        assert_eq!(rb.len(), 1); // Peek doesn't remove

        assert_eq!(rb.pop(), Some(42));
        assert_eq!(rb.peek(), None);
    }

    #[test]
    fn test_clear() {
        let rb = IoRingBuffer::<u64>::new(RingBufferConfig {
            size: 8,
            enable_stats: true,
            enable_prefetch: false,
        })
        .unwrap();

        rb.push(1);
        rb.push(2);
        rb.push(3);
        assert_eq!(rb.len(), 3);

        rb.clear();
        assert!(rb.is_empty());
        assert_eq!(rb.len(), 0);
        assert_eq!(rb.stats().clears, 1);
    }

    #[test]
    fn test_stats() {
        let rb = IoRingBuffer::<u64>::new(RingBufferConfig {
            size: 4,
            enable_stats: true,
            enable_prefetch: false,
        })
        .unwrap();

        // Fill buffer
        for _i in 0..4 {
            rb.push(i);
        }
        assert!(!rb.push(100)); // Should fail

        // Pop all
        for _ in 0..4 {
            rb.pop();
        }
        assert!(rb.pop().is_none()); // Should fail

        let _stats = rb.stats();
        assert_eq!(stats.pushes, 4);
        assert_eq!(stats.push_failures, 1);
        assert_eq!(stats.pops, 4);
        assert_eq!(stats.pop_failures, 1);
        assert_eq!(stats.push_success_rate(), 0.8); // 4/5
        assert_eq!(stats.pop_success_rate(), 0.8); // 4/5
    }
}
