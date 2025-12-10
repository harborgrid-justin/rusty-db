// # Page Cache - High-Performance Buffer Frames
//
// Provides page-aligned buffers with explicit control optimized for Windows/MSVC.
// All structures use `#[repr(C)]` for predictable memory layout and direct I/O compatibility.
//
// ## Key Features
//
// - Page-aligned buffers (4KB alignment) for direct I/O
// - Lock-free pin counting with atomics
// - Zero-copy page access with unsafe optimizations
// - Explicit dirty tracking for write-back cache
// - MSVC-compatible memory layout

use crate::common::PageId;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Standard page size (4KB) - Windows default page size
pub const PAGE_SIZE: usize = 4096;

/// Frame ID type
pub type FrameId = u32;

/// Invalid frame ID sentinel
pub const INVALID_FRAME_ID: FrameId = u32::MAX;

/// Invalid page ID sentinel
pub const INVALID_PAGE_ID: PageId = u64::MAX;

// ============================================================================
// Page Buffer - Aligned Memory for Direct I/O
// ============================================================================

/// Page-aligned buffer for direct I/O operations.
///
/// Uses `#[repr(C, align(4096))]` to ensure:
/// - Compatible with Windows unbuffered I/O (requires sector alignment)
/// - Cache-line aligned for optimal CPU cache utilization
/// - Predictable memory layout for MSVC compiler
///
/// # Safety
///
/// This structure is designed for unsafe operations. The data array
/// can be accessed via raw pointers for zero-copy I/O.
#[repr(C, align(4096))]
#[derive(Clone)]
pub struct PageBuffer {
// Raw page data (4KB)
    data: [u8; PAGE_SIZE],
}

impl PageBuffer {
// Create a new zeroed page buffer
    #[inline]
    pub fn new() -> Self {
        Self {
            data: [0u8; PAGE_SIZE],
        }
    }

// Create a page buffer from existing data
//
// # Safety
//
// Caller must ensure data is exactly PAGE_SIZE bytes
    #[inline]
    pub fn from_bytes(data: &[u8]) -> Self {
        let mut buffer = Self::new();
        buffer.data[..data.len()].copy_from_slice(data);
        buffer
    }

// Get immutable reference to page data
    #[inline(always)]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

// Get mutable reference to page data
    #[inline(always)]
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

// Get raw pointer to page data (for zero-copy I/O)
//
// # Safety
//
// The returned pointer is valid for PAGE_SIZE bytes.
// Caller must ensure proper synchronization when using this pointer.
    #[inline(always)]
    pub unsafe fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

// Get mutable raw pointer to page data (for zero-copy I/O)
//
// # Safety
//
// The returned pointer is valid for PAGE_SIZE bytes.
// Caller must ensure exclusive access when using this pointer.
    #[inline(always)]
    pub unsafe fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_mut_ptr()
    }

// Zero out the entire page
    #[inline]
    pub fn zero(&mut self) {
// SAFETY: Writing zeros is always safe
        unsafe {
            std::ptr::write_bytes(self.data.as_mut_ptr(), 0, PAGE_SIZE);
        }
    }

// Copy data from another page buffer
    #[inline]
    pub fn copy_from(&mut self, other: &PageBuffer) {
// SAFETY: Both buffers are PAGE_SIZE, this is safe
        unsafe {
            std::ptr::copy_nonoverlapping(
                other.data.as_ptr(),
                self.data.as_mut_ptr(),
                PAGE_SIZE,
            );
        }
    }

// Check if page is zeroed (used for optimization)
    #[cold]
    pub fn is_zeroed(&self) -> bool {
        self.data.iter().all(|&b| b == 0)
    }

// Calculate checksum of page data (CRC32)
    #[inline]
    pub fn checksum(&self) -> u32 {
        crc32fast::hash(&self.data)
    }

// Verify page checksum
    #[inline]
    pub fn verify_checksum(&self, expected: u32) -> bool {
        self.checksum() == expected
    }
}

impl Default for PageBuffer {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Buffer Frame - Core Buffer Pool Unit
// ============================================================================

/// Buffer frame metadata and state tracking.
///
/// Stores both the page data and metadata about the frame state.
/// Uses atomic operations for lock-free pin counting.
///
/// # Memory Layout
///
/// ```text
/// +------------------+
/// | page_id (8)      |
/// | frame_id (4)     |
/// | pin_count (4)    |
/// | dirty (1)        |
/// | io_in_progress(1)|
/// | last_access (16) |
/// | data (4096)      | <- Page-aligned
/// +------------------+
/// ```
#[repr(C)]
pub struct BufferFrame {
// Page ID currently stored in this frame (INVALID_PAGE_ID if empty)
    page_id: AtomicU64,

// Frame ID (index in frame array)
    frame_id: FrameId,

// Pin count - number of concurrent users of this page
// 0 = unpinned (can be evicted)
// >0 = pinned (cannot be evicted)
    pin_count: AtomicU32,

// Dirty flag - has the page been modified?
    dirty: AtomicBool,

// I/O in progress flag - is this frame being read/written?
    io_in_progress: AtomicBool,

// Reference bit for CLOCK algorithm
    ref_bit: AtomicBool,

// Last access timestamp (for LRU)
    last_access: AtomicU64,

// Page data (aligned to 4096 bytes)
    data: RwLock<PageBuffer>,

// LSN of last modification (for WAL)
    page_lsn: AtomicU64,

// Access count for statistics
    access_count: AtomicU64,
}

impl BufferFrame {
// Create a new empty buffer frame
    #[inline]
    pub fn new(frame_id: FrameId) -> Self {
        Self {
            page_id: AtomicU64::new(INVALID_PAGE_ID),
            frame_id,
            pin_count: AtomicU32::new(0),
            dirty: AtomicBool::new(false),
            io_in_progress: AtomicBool::new(false),
            ref_bit: AtomicBool::new(false),
            last_access: AtomicU64::new(0),
            data: RwLock::new(PageBuffer::new()),
            page_lsn: AtomicU64::new(0),
            access_count: AtomicU64::new(0),
        }
    }

// Get frame ID
    #[inline(always)]
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

// Get page ID (lock-free)
    #[inline(always)]
    pub fn page_id(&self) -> PageId {
        self.page_id.load(Ordering::Acquire)
    }

// Set page ID
    #[inline]
    pub fn set_page_id(&self, page_id: PageId) {
        self.page_id.store(page_id, Ordering::Release);
    }

// Get pin count (lock-free)
    #[inline(always)]
    pub fn pin_count(&self) -> u32 {
        self.pin_count.load(Ordering::Acquire)
    }

// Check if frame is pinned (lock-free)
    #[inline(always)]
    pub fn is_pinned(&self) -> bool {
        self.pin_count() > 0
    }

// Increment pin count atomically
//
// Returns the old pin count.
// This is a hot-path operation, so it's marked `#[inline(always)]`.
    #[inline(always)]
    pub fn pin(&self) -> u32 {
        let old_count = self.pin_count.fetch_add(1, Ordering::AcqRel);
        self.update_access_time();
        self.access_count.fetch_add(1, Ordering::Relaxed);
        old_count
    }

// Decrement pin count atomically
//
// Returns the new pin count.
// Panics if pin count would underflow (indicates a bug).
    #[inline(always)]
    pub fn unpin(&self) -> u32 {
        let old_count = self.pin_count.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(old_count > 0, "Attempted to unpin unpinned frame");
        old_count - 1
    }

// Try to pin the frame (returns false if eviction in progress)
    #[inline]
    pub fn try_pin(&self) -> bool {
        if self.io_in_progress.load(Ordering::Acquire) {
            return false;
        }
        self.pin();
        true
    }

// Check if frame is dirty (lock-free)
    #[inline(always)]
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }

// Mark frame as dirty
    #[inline]
    pub fn set_dirty(&self, dirty: bool) {
        self.dirty.store(dirty, Ordering::Release);
    }

// Mark frame as dirty and update LSN
    #[inline]
    pub fn mark_dirty(&self, lsn: u64) {
        self.dirty.store(true, Ordering::Release);
        self.page_lsn.store(lsn, Ordering::Release);
    }

// Get page LSN
    #[inline]
    pub fn page_lsn(&self) -> u64 {
        self.page_lsn.load(Ordering::Acquire)
    }

// Check if I/O is in progress
    #[inline]
    pub fn io_in_progress(&self) -> bool {
        self.io_in_progress.load(Ordering::Acquire)
    }

// Set I/O in progress flag
    #[inline]
    pub fn set_io_in_progress(&self, in_progress: bool) {
        self.io_in_progress.store(in_progress, Ordering::Release);
    }

// Get reference bit (for CLOCK algorithm)
    #[inline(always)]
    pub fn ref_bit(&self) -> bool {
        self.ref_bit.load(Ordering::Acquire)
    }

// Set reference bit
    #[inline(always)]
    pub fn set_ref_bit(&self, bit: bool) {
        self.ref_bit.store(bit, Ordering::Release);
    }

// Clear reference bit and return old value (for CLOCK)
    #[inline(always)]
    pub fn clear_ref_bit(&self) -> bool {
        self.ref_bit.swap(false, Ordering::AcqRel)
    }

// Update access timestamp (for LRU)
    #[inline]
    fn update_access_time(&self) {
        let now = Instant::now().elapsed().as_millis() as u64;
        self.last_access.store(now, Ordering::Relaxed);
    }

// Get last access time
    #[inline]
    pub fn last_access_time(&self) -> u64 {
        self.last_access.load(Ordering::Relaxed)
    }

// Get access count for statistics
    #[inline]
    pub fn access_count(&self) -> u64 {
        self.access_count.load(Ordering::Relaxed)
    }

// Reset frame to empty state
    #[cold]
    pub fn reset(&self) {
        self.page_id.store(INVALID_PAGE_ID, Ordering::Release);
        self.pin_count.store(0, Ordering::Release);
        self.dirty.store(false, Ordering::Release);
        self.io_in_progress.store(false, Ordering::Release);
        self.ref_bit.store(false, Ordering::Release);
        self.page_lsn.store(0, Ordering::Release);
        self.access_count.store(0, Ordering::Release);
    }

// Check if frame is empty (not holding a page)
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.page_id() == INVALID_PAGE_ID
    }

// Get read access to page data
//
// This acquires a read lock, so multiple readers are allowed.
    #[inline]
    pub fn read_data(&self) -> parking_lot::RwLockReadGuard<'_, PageBuffer> {
        self.data.read()
    }

// Get write access to page data
//
// This acquires a write lock, so only one writer is allowed.
// Automatically marks the frame as dirty.
    #[inline]
    pub fn write_data(&self) -> parking_lot::RwLockWriteGuard<'_, PageBuffer> {
        self.dirty.store(true, Ordering::Release);
        self.data.write()
    }

// Get write access without marking dirty (for initial load)
    #[inline]
    pub fn write_data_no_dirty(&self) -> parking_lot::RwLockWriteGuard<'_, PageBuffer> {
        self.data.write()
    }

// Try to evict this frame (returns true if successful)
    #[cold]
    pub fn try_evict(&self) -> bool {
// Can only evict if not pinned and no I/O in progress
        if self.is_pinned() || self.io_in_progress() {
            return false;
        }

// Try to set I/O in progress
        if self.io_in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return false;
        }

// Double-check pin count after setting I/O flag
        if self.is_pinned() {
            self.io_in_progress.store(false, Ordering::Release);
            return false;
        }

        true
    }
}

// ============================================================================
// Page Guard - RAII Pin/Unpin
// ============================================================================

/// RAII guard that automatically unpins a frame when dropped.
///
/// This ensures that frames are always unpinned even if an error occurs.
///
/// # Example
///
/// ```ignore
/// let guard = frame.pin_guard();
/// // Use the page data
/// let data = guard.read_data();
/// // Frame is automatically unpinned when guard is dropped
/// ```
pub struct FrameGuard {
    frame: Arc<BufferFrame>,
}

impl FrameGuard {
// Create a new frame guard (pins the frame)
    #[inline]
    pub fn new(frame: Arc<BufferFrame>) -> Self {
        frame.pin();
        Self { frame }
    }

// Get the underlying frame
    #[inline]
    pub fn frame(&self) -> &BufferFrame {
        &self.frame
    }

// Get page ID
    #[inline]
    pub fn page_id(&self) -> PageId {
        self.frame.page_id()
    }

// Get frame ID
    #[inline]
    pub fn frame_id(&self) -> FrameId {
        self.frame.frame_id()
    }

// Read page data
    #[inline]
    pub fn read_data(&self) -> parking_lot::RwLockReadGuard<'_, PageBuffer> {
        self.frame.read_data()
    }

// Write page data
    #[inline]
    pub fn write_data(&self) -> parking_lot::RwLockWriteGuard<'_, PageBuffer> {
        self.frame.write_data()
    }
}

impl Drop for FrameGuard {
    #[inline(always)]
    fn drop(&mut self) {
        self.frame.unpin();
    }
}

// ============================================================================
// Frame Statistics
// ============================================================================

/// Statistics for a buffer frame
#[derive(Debug, Clone)]
pub struct FrameStats {
    pub frame_id: FrameId,
    pub page_id: PageId,
    pub pin_count: u32,
    pub is_dirty: bool,
    pub io_in_progress: bool,
    pub access_count: u64,
    pub last_access_ms: u64,
    pub page_lsn: u64,
}

impl BufferFrame {
// Get current frame statistics
    #[cold]
    pub fn get_stats(&self) -> FrameStats {
        FrameStats {
            frame_id: self.frame_id,
            page_id: self.page_id(),
            pin_count: self.pin_count(),
            is_dirty: self.is_dirty(),
            io_in_progress: self.io_in_progress(),
            access_count: self.access_count(),
            last_access_ms: self.last_access_time(),
            page_lsn: self.page_lsn(),
        }
    }
}

// ============================================================================
// Batch Operations
// ============================================================================

/// Batch of frames for efficient flushing
pub struct FrameBatch {
    frames: Vec<Arc<BufferFrame>>,
    max_size: usize,
}

impl FrameBatch {
// Create a new frame batch
    #[inline]
    pub fn new(max_size: usize) -> Self {
        Self {
            frames: Vec::with_capacity(max_size),
            max_size,
        }
    }

// Add a frame to the batch
//
// Returns false if batch is full.
    #[inline]
    pub fn add(&mut self, frame: Arc<BufferFrame>) -> bool {
        if self.frames.len() >= self.max_size {
            return false;
        }
        self.frames.push(frame);
        true
    }

// Check if batch is full
    #[inline]
    pub fn is_full(&self) -> bool {
        self.frames.len() >= self.max_size
    }

// Check if batch is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

// Get number of frames in batch
    #[inline]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

// Get frames in the batch
    #[inline]
    pub fn frames(&self) -> &[Arc<BufferFrame>] {
        &self.frames
    }

// Clear the batch
    #[inline]
    pub fn clear(&mut self) {
        self.frames.clear();
    }

// Sort frames by page ID (for sequential I/O)
    #[inline]
    pub fn sort_by_page_id(&mut self) {
        self.frames.sort_by_key(|f| f.page_id());
    }

// Get dirty frames in the batch
    #[inline]
    pub fn dirty_frames(&self) -> Vec<Arc<BufferFrame>> {
        self.frames
            .iter()
            .filter(|f| f.is_dirty())
            .cloned()
            .collect()
    }
}

// ============================================================================
// Per-Core Frame Pool
// ============================================================================

/// Per-core frame pool to reduce contention
///
/// Each CPU core gets its own pool of frames that it can allocate from
/// without contention. This is inspired by Linux's per-CPU page allocator.
pub struct PerCoreFramePool {
// Core ID this pool belongs to
    _core_id: usize,

// Free frames in this pool
    free_frames: Mutex<Vec<FrameId>>,

// Maximum frames in this pool
    max_frames: usize,

// Statistics
    allocations: AtomicU64,
    deallocations: AtomicU64,
}

impl PerCoreFramePool {
// Create a new per-core frame pool
    #[inline]
    pub fn new(core_id: usize, max_frames: usize) -> Self {
        Self {
            _core_id: core_id,
            free_frames: Mutex::new(Vec::with_capacity(max_frames)),
            max_frames,
            allocations: AtomicU64::new(0),
            deallocations: AtomicU64::new(0),
        }
    }

// Try to allocate a frame from this core's pool
    #[inline]
    pub fn try_allocate(&self) -> Option<FrameId> {
        let frame = self.free_frames.lock().unwrap().pop();
        if frame.is_some() {
            self.allocations.fetch_add(1, Ordering::Relaxed);
        }
        frame
    }

// Return a frame to this core's pool
    #[inline]
    pub fn deallocate(&self, frame_id: FrameId) -> bool {
        let mut frames = self.free_frames.lock().unwrap();
        if frames.len() >= self.max_frames {
            return false;
        }
        frames.push(frame_id);
        self.deallocations.fetch_add(1, Ordering::Relaxed);
        true
    }

// Add frames to this pool
    #[inline]
    pub fn add_frames(&self, new_frames: Vec<FrameId>) {
        let mut pool = self.free_frames.lock().unwrap();
        pool.extend(new_frames);
    }

// Get number of free frames
    #[inline]
    pub fn free_count(&self) -> usize {
        self.free_frames.lock().unwrap().len()
    }

// Get statistics
    #[cold]
    pub fn stats(&self) -> (u64, u64, usize) {
        (
            self.allocations.load(Ordering::Relaxed),
            self.deallocations.load(Ordering::Relaxed),
            self.free_count(),
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_buffer_alignment() {
        let buffer = PageBuffer::new();
        let ptr = buffer.data.as_ptr();
        assert_eq!(ptr as usize % 4096, 0, "PageBuffer must be 4096-byte aligned");
    }

    #[test]
    fn test_page_buffer_size() {
        assert_eq!(size_of::<PageBuffer>(), PAGE_SIZE);
    }

    #[test]
    fn test_buffer_frame_pin_unpin() {
        let frame = BufferFrame::new(0);
        assert_eq!(frame.pin_count(), 0);

        frame.pin();
        assert_eq!(frame.pin_count(), 1);
        assert!(frame.is_pinned());

        frame.pin();
        assert_eq!(frame.pin_count(), 2);

        frame.unpin();
        assert_eq!(frame.pin_count(), 1);

        frame.unpin();
        assert_eq!(frame.pin_count(), 0);
        assert!(!frame.is_pinned());
    }

    #[test]
    fn test_buffer_frame_dirty() {
        let frame = BufferFrame::new(0);
        assert!(!frame.is_dirty());

        frame.set_dirty(true);
        assert!(frame.is_dirty());

        frame.set_dirty(false);
        assert!(!frame.is_dirty());
    }

    #[test]
    fn test_frame_guard() {
        let frame = Arc::new(BufferFrame::new(0));
        assert_eq!(frame.pin_count(), 0);

        {
            let guard = FrameGuard::new(frame.clone());
            assert_eq!(frame.pin_count(), 1);
        }

        assert_eq!(frame.pin_count(), 0);
    }

    #[test]
    fn test_per_core_pool() {
        let pool = PerCoreFramePool::new(0, 10);

        assert_eq!(pool.free_count(), 0);
        assert!(pool.try_allocate().is_none());

        pool.deallocate(5);
        assert_eq!(pool.free_count(), 1);

        let frame = pool.try_allocate();
        assert_eq!(frame, Some(5));
        assert_eq!(pool.free_count(), 0);
    }

    #[test]
    fn test_frame_batch() {
        let mut batch = FrameBatch::new(3);
        assert!(batch.is_empty());

        let frame1 = Arc::new(BufferFrame::new(0));
        let frame2 = Arc::new(BufferFrame::new(1));
        let frame3 = Arc::new(BufferFrame::new(2));

        assert!(batch.add(frame1));
        assert!(batch.add(frame2));
        assert!(batch.add(frame3));
        assert!(batch.is_full());

        let frame4 = Arc::new(BufferFrame::new(3));
        assert!(!batch.add(frame4));
    }

    #[test]
    fn test_checksum() {
        let mut buffer = PageBuffer::new();
        buffer.data[0] = 42;
        buffer.data[100] = 123;

        let checksum = buffer.checksum();
        assert!(checksum > 0);
        assert!(buffer.verify_checksum(checksum));

        buffer.data[0] = 43;
        assert!(!buffer.verify_checksum(checksum));
    }
}
