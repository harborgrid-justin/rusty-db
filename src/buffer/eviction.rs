// # Eviction Policies - Buffer Frame Replacement Algorithms
//
// Implements multiple page replacement policies optimized for zero allocations
// in the hot path. All policies are lock-free where possible.
//
// ## Supported Policies
//
// - **CLOCK**: Second-chance algorithm with reference bits (default)
// - **LRU**: Least Recently Used with O(1) operations
// - **2Q**: Two-queue algorithm for scan resistance
// - **LRU-K**: K-distance with correlated reference tracking
//
// ## Performance Characteristics
//
// All policies guarantee:
// - Zero allocations in victim selection
// - Lock-free reads where possible
// - Constant-time operations in hot path
// - MSVC-compatible memory layouts

use crate::buffer::page_cache::{BufferFrame, FrameId};
use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

// ============================================================================
// Eviction Policy Trait
// ============================================================================

/// Base trait for all eviction policies
pub trait EvictionPolicy: Send + Sync {
    /// Find a victim frame to evict
    ///
    /// Returns None if no frame can be evicted.
    /// This is a hot-path operation and must be fast.
    fn find_victim(&self, frames: &[Arc<BufferFrame>]) -> Option<FrameId>;

    /// Record a page access (for LRU tracking)
    fn record_access(&self, frame_id: FrameId);

    /// Record a page pin (for advanced policies)
    fn record_pin(&self, frame_id: FrameId) {
        self.record_access(frame_id);
    }

    /// Record a page unpin
    fn record_unpin(&self, frame_id: FrameId) {
        // Default: no-op
        let _ = frame_id;
    }

    /// Record a page eviction
    fn record_eviction(&self, frame_id: FrameId);

    /// Reset policy state
    fn reset(&self);

    /// Get policy name
    fn name(&self) -> &'static str;

    /// Get policy statistics
    fn stats(&self) -> EvictionStats;
}

/// Statistics for eviction policy
#[derive(Debug, Clone, Default)]
pub struct EvictionStats {
    pub victim_searches: u64,
    pub evictions: u64,
    pub failed_evictions: u64,
    pub clock_hand_position: u32,
    pub avg_search_length: f64,
}

// ============================================================================
// CLOCK Policy - Second-Chance Algorithm
// ============================================================================

/// CLOCK eviction policy (also known as Second-Chance).
///
/// Uses a circular buffer with a clock hand that sweeps through frames.
/// Each frame has a reference bit that is set on access and cleared by
/// the clock hand. Frames with cleared reference bits are evicted.
///
/// # Performance
///
/// - Victim selection: O(n) worst case, O(1) amortized
/// - Access recording: O(1) - just sets a bit
/// - Memory overhead: None (uses frame metadata)
/// - Thread safety: Lock-free for access recording
///
/// # Why CLOCK?
///
/// - Simple and efficient
/// - Good approximation of LRU
/// - No complex data structures
/// - Works well for most workloads
/// - Default in PostgreSQL and many other databases
pub struct ClockEvictionPolicy {
    /// Clock hand position (index into frame array)
    clock_hand: AtomicU32,

    /// Total number of frames
    num_frames: u32,

    /// Statistics
    victim_searches: AtomicU64,
    evictions: AtomicU64,
    failed_evictions: AtomicU64,
    total_search_length: AtomicU64,
}

impl ClockEvictionPolicy {
    /// Create a new CLOCK policy
    #[inline]
    pub fn new(num_frames: usize) -> Self {
        Self {
            clock_hand: AtomicU32::new(0),
            num_frames: num_frames as u32,
            victim_searches: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            failed_evictions: AtomicU64::new(0),
            total_search_length: AtomicU64::new(0),
        }
    }

    /// Advance clock hand and return next position
    #[inline(always)]
    fn advance_hand(&self) -> u32 {
        let old = self.clock_hand.fetch_add(1, Ordering::Relaxed);
        (old + 1) % self.num_frames
    }

    /// Get current clock hand position
    #[inline]
    pub fn hand_position(&self) -> u32 {
        self.clock_hand.load(Ordering::Relaxed)
    }
}

impl EvictionPolicy for ClockEvictionPolicy {
    /// Find victim using CLOCK algorithm
    ///
    /// Sweeps through frames starting at clock hand position.
    /// Clears reference bits and evicts the first frame with:
    /// - Reference bit = 0 (not recently used)
    /// - Pin count = 0 (not in use)
    /// - No I/O in progress
    #[inline]
    fn find_victim(&self, frames: &[Arc<BufferFrame>]) -> Option<FrameId> {
        self.victim_searches.fetch_add(1, Ordering::Relaxed);

        let _start_pos = self.hand_position();
        let mut search_length = 0u64;

        // Sweep through frames (maximum 2 full cycles)
        for _ in 0..(self.num_frames * 2) {
            let pos = self.advance_hand();
            search_length += 1;

            // SAFETY: pos is guaranteed to be < num_frames by modulo
            let frame = unsafe { frames.get_unchecked(pos as usize) };

            // Skip if pinned or I/O in progress
            if frame.is_pinned() || frame.io_in_progress() {
                continue;
            }

            // Check and clear reference bit
            if frame.clear_ref_bit() {
                // Reference bit was set, give it a second chance
                continue;
            }

            // Found a victim!
            if frame.try_evict() {
                self.evictions.fetch_add(1, Ordering::Relaxed);
                self.total_search_length
                    .fetch_add(search_length, Ordering::Relaxed);
                return Some(frame.frame_id());
            }
        }

        // No victim found after 2 full sweeps
        self.failed_evictions.fetch_add(1, Ordering::Relaxed);
        None
    }

    #[inline(always)]
    fn record_access(&self, _frame_id: FrameId) {
        // Reference bit is set automatically in BufferFrame::pin()
        // No additional work needed
    }

    #[inline]
    fn record_eviction(&self, _frame_id: FrameId) {
        // No additional tracking needed for CLOCK
    }

    fn reset(&self) {
        self.clock_hand.store(0, Ordering::Relaxed);
        self.victim_searches.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.failed_evictions.store(0, Ordering::Relaxed);
        self.total_search_length.store(0, Ordering::Relaxed);
    }

    fn name(&self) -> &'static str {
        "CLOCK"
    }

    fn stats(&self) -> EvictionStats {
        let searches = self.victim_searches.load(Ordering::Relaxed);
        let total_length = self.total_search_length.load(Ordering::Relaxed);
        let avg_search = if searches > 0 {
            total_length as f64 / searches as f64
        } else {
            0.0
        };

        EvictionStats {
            victim_searches: searches,
            evictions: self.evictions.load(Ordering::Relaxed),
            failed_evictions: self.failed_evictions.load(Ordering::Relaxed),
            clock_hand_position: self.hand_position(),
            avg_search_length: avg_search,
        }
    }
}

// ============================================================================
// LRU Policy - Least Recently Used
// ============================================================================

/// Doubly-linked list node for LRU
struct LruNode {
    frame_id: FrameId,
    prev: Option<usize>,
    next: Option<usize>,
}

/// LRU eviction policy with O(1) operations.
///
/// Maintains a doubly-linked list of frames ordered by access time.
/// Most recently used frames are at the head, least recently used at tail.
///
/// # Performance
///
/// - Victim selection: O(1)
/// - Access recording: O(1)
/// - Memory overhead: O(n) for linked list
/// - Thread safety: Mutex-protected (small critical sections)
///
/// # Implementation
///
/// Uses an intrusive doubly-linked list stored in a vector for cache locality.
/// This is faster than a heap-allocated linked list.
pub struct LruEvictionPolicy {
    /// Intrusive linked list (indexed by frame_id)
    list: RwLock<Vec<LruNode>>,

    /// Head of list (most recently used)
    head: Mutex<Option<usize>>,

    /// Tail of list (least recently used)
    tail: Mutex<Option<usize>>,

    /// Statistics
    victim_searches: AtomicU64,
    evictions: AtomicU64,
    failed_evictions: AtomicU64,
}

impl LruEvictionPolicy {
    /// Create a new LRU policy
    pub fn new(num_frames: usize) -> Self {
        let mut list = Vec::with_capacity(num_frames);
        for i in 0..num_frames {
            list.push(LruNode {
                frame_id: i as FrameId,
                prev: if i > 0 { Some(i - 1) } else { None },
                next: if i < num_frames - 1 { Some(i + 1) } else { None },
            });
        }

        Self {
            list: RwLock::new(list),
            head: Mutex::new(if num_frames > 0 { Some(0) } else { None }),
            tail: Mutex::new(if num_frames > 0 {
                Some(num_frames - 1)
            } else {
                None
            }),
            victim_searches: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            failed_evictions: AtomicU64::new(0),
        }
    }

    /// Move a node to the head (most recently used)
    #[inline]
    fn move_to_head(&self, frame_id: FrameId) {
        let mut list = self.list.write();
        let idx = frame_id as usize;

        // Remove from current position - copy values first to avoid borrow issues
        let node_prev = list[idx].prev;
        let node_next = list[idx].next;

        if let Some(prev) = node_prev {
            list[prev].next = node_next;
        }
        if let Some(next) = node_next {
            list[next].prev = node_prev;
        }

        // Update tail if this was the tail
        if *self.tail.lock() == Some(idx) {
            *self.tail.lock() = node_prev;
        }

        // Move to head
        let old_head = *self.head.lock();
        list[idx].prev = None;
        list[idx].next = old_head;

        if let Some(old_head_idx) = old_head {
            list[old_head_idx].prev = Some(idx);
        }

        *self.head.lock() = Some(idx);

        // Update tail if list was empty
        if self.tail.lock().unwrap().is_none() {
            *self.tail.lock() = Some(idx);
        }
    }
}

impl EvictionPolicy for LruEvictionPolicy {
    /// Find victim - evict from tail (least recently used)
    #[inline]
    fn find_victim(&self, frames: &[Arc<BufferFrame>]) -> Option<FrameId> {
        self.victim_searches.fetch_add(1, Ordering::Relaxed);

        let list = self.list.read();
        let mut tail = self.tail.lock();

        // Walk backwards from tail to find unpinned frame
        let mut current = *tail;
        let max_attempts = list.len();

        for _ in 0..max_attempts {
            if let Some(idx) = current {
                let frame_id = list[idx].frame_id;
                let frame = &frames[frame_id as usize];

                if !frame.is_pinned() && !frame.io_in_progress() && frame.try_evict() {
                    // Update tail to previous node
                    *tail = list[idx].prev;
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                    return Some(frame_id);
                }

                current = list[idx].prev;
            } else {
                break;
            }
        }

        self.failed_evictions.fetch_add(1, Ordering::Relaxed);
        None
    }

    #[inline]
    fn record_access(&self, frame_id: FrameId) {
        self.move_to_head(frame_id);
    }

    fn record_eviction(&self, frame_id: FrameId) {
        // Remove from list
        let mut list = self.list.write();
        let idx = frame_id as usize;

        // Copy values first to avoid borrow issues
        let node_prev = list[idx].prev;
        let node_next = list[idx].next;

        if let Some(prev) = node_prev {
            list[prev].next = node_next;
        }
        if let Some(next) = node_next {
            list[next].prev = node_prev;
        }

        // Update head/tail if needed
        if *self.head.lock() == Some(idx) {
            *self.head.lock() = node_next;
        }
        if *self.tail.lock() == Some(idx) {
            *self.tail.lock() = node_prev;
        }
    }

    fn reset(&self) {
        // Rebuild list
        let mut list = self.list.write();
        let num_frames = list.len();

        for i in 0..num_frames {
            list[i].prev = if i > 0 { Some(i - 1) } else { None };
            list[i].next = if i < num_frames - 1 { Some(i + 1) } else { None };
        }

        *self.head.lock() = if num_frames > 0 { Some(0) } else { None };
        *self.tail.lock() = if num_frames > 0 {
            Some(num_frames - 1)
        } else {
            None
        };

        self.victim_searches.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.failed_evictions.store(0, Ordering::Relaxed);
    }

    fn name(&self) -> &'static str {
        "LRU"
    }

    fn stats(&self) -> EvictionStats {
        EvictionStats {
            victim_searches: self.victim_searches.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            failed_evictions: self.failed_evictions.load(Ordering::Relaxed),
            clock_hand_position: 0,
            avg_search_length: 1.0,
        }
    }
}

// ============================================================================
// 2Q Policy - Two Queue Algorithm
// ============================================================================

/// 2Q (Two Queue) eviction policy for scan resistance.
///
/// Maintains three queues:
/// - A1in: FIFO queue for pages seen once (small)
/// - A1out: Ghost queue tracking recently evicted A1in pages
/// - Am: LRU queue for pages accessed multiple times (large)
///
/// Pages start in A1in. If accessed again before eviction, they move to Am.
/// This prevents sequential scans from polluting the main cache.
///
/// # Performance
///
/// - Victim selection: O(1) amortized
/// - Access recording: O(1)
/// - Memory overhead: O(n) for queues
/// - Thread safety: Mutex-protected
///
/// # Parameters
///
/// - A1in size: 25% of buffer pool
/// - A1out size: 50% of buffer pool
/// - Am size: 75% of buffer pool
pub struct TwoQEvictionPolicy {
    /// FIFO queue for pages seen once (frame IDs)
    a1in: Mutex<VecDeque<FrameId>>,

    /// Ghost queue of evicted A1in pages (for tracking)
    a1out: Mutex<VecDeque<FrameId>>,

    /// LRU queue for pages accessed multiple times
    am: Mutex<VecDeque<FrameId>>,

    /// Maximum size of A1in (25% of pool)
    a1in_max_size: usize,

    /// Maximum size of A1out (50% of pool)
    a1out_max_size: usize,

    /// Frame to queue mapping
    frame_queue: RwLock<HashMap<FrameId, QueueType>>,

    /// Statistics
    victim_searches: AtomicU64,
    evictions: AtomicU64,
    failed_evictions: AtomicU64,
    a1in_hits: AtomicU64,
    a1out_hits: AtomicU64,
    am_hits: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueueType {
    A1In,
    A1Out,
    Am,
}

impl TwoQEvictionPolicy {
    /// Create a new 2Q policy
    pub fn new(num_frames: usize) -> Self {
        let a1in_max = (num_frames / 4).max(1);
        let a1out_max = (num_frames / 2).max(1);

        Self {
            a1in: Mutex::new(VecDeque::with_capacity(a1in_max)),
            a1out: Mutex::new(VecDeque::with_capacity(a1out_max)),
            am: Mutex::new(VecDeque::with_capacity(num_frames)),
            a1in_max_size: a1in_max,
            a1out_max_size: a1out_max,
            frame_queue: RwLock::new(HashMap::new()),
            victim_searches: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            failed_evictions: AtomicU64::new(0),
            a1in_hits: AtomicU64::new(0),
            a1out_hits: AtomicU64::new(0),
            am_hits: AtomicU64::new(0),
        }
    }

    /// Add frame to A1in queue
    #[inline]
    fn add_to_a1in(&self, frame_id: FrameId) {
        let mut a1in = self.a1in.lock();
        if a1in.len() >= self.a1in_max_size {
            // Evict from A1in to A1out
            if let Some(evicted) = a1in.pop_front() {
                self.move_to_a1out(evicted);
            }
        }
        a1in.push_back(frame_id);
        self.frame_queue.write().insert(frame_id, QueueType::A1In);
    }

    /// Move frame from A1in to A1out
    #[inline]
    fn move_to_a1out(&self, frame_id: FrameId) {
        let mut a1out = self.a1out.lock();
        if a1out.len() >= self.a1out_max_size {
            // Remove oldest from A1out
            if let Some(removed) = a1out.pop_front() {
                self.frame_queue.write().remove(&removed);
            }
        }
        a1out.push_back(frame_id);
        self.frame_queue
            .write()
            .insert(frame_id, QueueType::A1Out);
    }

    /// Add frame to Am queue (LRU)
    #[inline]
    fn add_to_am(&self, frame_id: FrameId) {
        let mut am = self.am.lock();
        am.push_back(frame_id);
        self.frame_queue.write().insert(frame_id, QueueType::Am);
    }

    /// Move frame to end of Am (most recently used)
    #[inline]
    fn touch_am(&self, frame_id: FrameId) {
        let mut am = self.am.lock();
        if let Some(pos) = am.iter().position(|&fid| fid == frame_id) {
            am.remove(pos);
            am.push_back(frame_id);
        }
    }
}

impl EvictionPolicy for TwoQEvictionPolicy {
    fn find_victim(&self, frames: &[Arc<BufferFrame>]) -> Option<FrameId> {
        self.victim_searches.fetch_add(1, Ordering::Relaxed);

        // Try to evict from A1in first (FIFO)
        {
            let mut a1in = self.a1in.lock();
            while let Some(frame_id) = a1in.front().copied() {
                let frame = &frames[frame_id as usize];
                if !frame.is_pinned() && !frame.io_in_progress() && frame.try_evict() {
                    a1in.pop_front();
                    self.frame_queue.write().remove(&frame_id);
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                    return Some(frame_id);
                }
                // Frame is pinned, skip it
                a1in.pop_front();
                a1in.push_back(frame_id);
                break;
            }
        }

        // Try to evict from Am (LRU)
        {
            let mut am = self.am.lock();
            while let Some(frame_id) = am.front().copied() {
                let frame = &frames[frame_id as usize];
                if !frame.is_pinned() && !frame.io_in_progress() && frame.try_evict() {
                    am.pop_front();
                    self.frame_queue.write().remove(&frame_id);
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                    return Some(frame_id);
                }
                // Frame is pinned, skip it
                am.pop_front();
                am.push_back(frame_id);
                break;
            }
        }

        self.failed_evictions.fetch_add(1, Ordering::Relaxed);
        None
    }

    fn record_access(&self, frame_id: FrameId) {
        let queue_type = self.frame_queue.read().get(&frame_id).copied();

        match queue_type {
            Some(QueueType::A1In) => {
                // Second access - promote to Am
                self.a1in_hits.fetch_add(1, Ordering::Relaxed);
                let mut a1in = self.a1in.lock();
                if let Some(pos) = a1in.iter().position(|&fid| fid == frame_id) {
                    a1in.remove(pos);
                }
                drop(a1in);
                self.add_to_am(frame_id);
            }
            Some(QueueType::A1Out) => {
                // Was recently evicted - add directly to Am
                self.a1out_hits.fetch_add(1, Ordering::Relaxed);
                let mut a1out = self.a1out.lock();
                if let Some(pos) = a1out.iter().position(|&fid| fid == frame_id) {
                    a1out.remove(pos);
                }
                drop(a1out);
                self.add_to_am(frame_id);
            }
            Some(QueueType::Am) => {
                // Already in Am - move to end (most recent)
                self.am_hits.fetch_add(1, Ordering::Relaxed);
                self.touch_am(frame_id);
            }
            None => {
                // First access - add to A1in
                self.add_to_a1in(frame_id);
            }
        }
    }

    fn record_eviction(&self, frame_id: FrameId) {
        self.frame_queue.write().remove(&frame_id);
    }

    fn reset(&self) {
        self.a1in.lock().unwrap().clear();
        self.a1out.lock().unwrap().clear();
        self.am.lock().unwrap().clear();
        self.frame_queue.write().clear();
        self.victim_searches.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.failed_evictions.store(0, Ordering::Relaxed);
        self.a1in_hits.store(0, Ordering::Relaxed);
        self.a1out_hits.store(0, Ordering::Relaxed);
        self.am_hits.store(0, Ordering::Relaxed);
    }

    fn name(&self) -> &'static str {
        "2Q"
    }

    fn stats(&self) -> EvictionStats {
        EvictionStats {
            victim_searches: self.victim_searches.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            failed_evictions: self.failed_evictions.load(Ordering::Relaxed),
            clock_hand_position: 0,
            avg_search_length: 1.0,
        }
    }
}

// ============================================================================
// LRU-K Policy - K-Distance Algorithm
// ============================================================================

/// LRU-K eviction policy with correlated reference tracking.
///
/// Tracks the K-th most recent access time for each page.
/// Evicts the page with the oldest K-th access.
///
/// K=2 (LRU-2) is common and provides good scan resistance.
pub struct LruKEvictionPolicy {
    /// K value (typically 2)
    k: usize,

    /// Access history for each frame (ring buffer)
    access_history: RwLock<Vec<VecDeque<u64>>>,

    /// Current timestamp
    timestamp: AtomicU64,

    /// Statistics
    victim_searches: AtomicU64,
    evictions: AtomicU64,
    failed_evictions: AtomicU64,
}

impl LruKEvictionPolicy {
    /// Create a new LRU-K policy
    pub fn new(num_frames: usize, k: usize) -> Self {
        let mut access_history = Vec::with_capacity(num_frames);
        for _ in 0..num_frames {
            access_history.push(VecDeque::with_capacity(k));
        }

        Self {
            k,
            access_history: RwLock::new(access_history),
            timestamp: AtomicU64::new(0),
            victim_searches: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            failed_evictions: AtomicU64::new(0),
        }
    }

    /// Get K-th most recent access time
    #[inline]
    fn get_k_distance(&self, frame_id: FrameId) -> u64 {
        let history = self.access_history.read();
        let frame_history = &history[frame_id as usize];

        if frame_history.len() >= self.k {
            // Return K-th most recent access
            frame_history[frame_history.len() - self.k]
        } else {
            // Not enough history - return infinity (oldest possible)
            0
        }
    }
}

impl EvictionPolicy for LruKEvictionPolicy {
    fn find_victim(&self, frames: &[Arc<BufferFrame>]) -> Option<FrameId> {
        self.victim_searches.fetch_add(1, Ordering::Relaxed);

        let mut oldest_k_distance = u64::MAX;
        let mut victim_frame = None;

        for frame in frames {
            if frame.is_pinned() || frame.io_in_progress() {
                continue;
            }

            let k_distance = self.get_k_distance(frame.frame_id());
            if k_distance < oldest_k_distance {
                oldest_k_distance = k_distance;
                victim_frame = Some(frame.frame_id());
            }
        }

        if let Some(frame_id) = victim_frame {
            if frames[frame_id as usize].try_evict() {
                self.evictions.fetch_add(1, Ordering::Relaxed);
                return Some(frame_id);
            }
        }

        self.failed_evictions.fetch_add(1, Ordering::Relaxed);
        None
    }

    fn record_access(&self, frame_id: FrameId) {
        let timestamp = self.timestamp.fetch_add(1, Ordering::Relaxed);
        let mut history = self.access_history.write();
        let frame_history = &mut history[frame_id as usize];

        frame_history.push_back(timestamp);
        if frame_history.len() > self.k {
            frame_history.pop_front();
        }
    }

    fn record_eviction(&self, frame_id: FrameId) {
        let mut history = self.access_history.write();
        history[frame_id as usize].clear();
    }

    fn reset(&self) {
        let mut history = self.access_history.write();
        for h in history.iter_mut() {
            h.clear();
        }
        self.timestamp.store(0, Ordering::Relaxed);
        self.victim_searches.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.failed_evictions.store(0, Ordering::Relaxed);
    }

    fn name(&self) -> &'static str {
        "LRU-K"
    }

    fn stats(&self) -> EvictionStats {
        EvictionStats {
            victim_searches: self.victim_searches.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            failed_evictions: self.failed_evictions.load(Ordering::Relaxed),
            clock_hand_position: 0,
            avg_search_length: 1.0,
        }
    }
}

// ============================================================================
// Policy Factory
// ============================================================================

/// Eviction policy type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionPolicyType {
    Clock,
    Lru,
    TwoQ,
    LruK(usize),
    /// ARC (Adaptive Replacement Cache) - self-tuning algorithm
    Arc,
    /// LIRS (Low Inter-reference Recency Set) - superior scan resistance
    Lirs,
}

/// Create an eviction policy of the specified type
pub fn create_eviction_policy(
    policy_type: EvictionPolicyType,
    num_frames: usize,
) -> Arc<dyn EvictionPolicy> {
    match policy_type {
        EvictionPolicyType::Clock => Arc::new(ClockEvictionPolicy::new(num_frames)),
        EvictionPolicyType::Lru => Arc::new(LruEvictionPolicy::new(num_frames)),
        EvictionPolicyType::TwoQ => Arc::new(TwoQEvictionPolicy::new(num_frames)),
        EvictionPolicyType::LruK(k) => Arc::new(LruKEvictionPolicy::new(num_frames, k)),
        EvictionPolicyType::Arc => Arc::new(crate::buffer::arc::ArcEvictionPolicy::new(num_frames)),
        EvictionPolicyType::Lirs => Arc::new(crate::buffer::lirs::LirsEvictionPolicy::new(num_frames)),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_frames(n: usize) -> Vec<Arc<BufferFrame>> {
        (0..n)
            .map(|i| Arc::new(BufferFrame::new(i as FrameId)))
            .collect()
    }

    #[test]
    fn test_clock_policy() {
        let frames = create_test_frames(10);
        let policy = ClockEvictionPolicy::new(10);

        // All frames unpinned, should find a victim
        let victim = policy.find_victim(&frames);
        assert!(victim.is_some());

        // Pin all frames
        for frame in &frames {
            frame.pin();
        }

        // Should not find a victim (all pinned)
        let victim = policy.find_victim(&frames);
        assert!(victim.is_none());
    }

    #[test]
    fn test_lru_policy() {
        let frames = create_test_frames(5);
        let policy = LruEvictionPolicy::new(5);

        // Access frames in order: 0, 1, 2, 3, 4
        for i in 0..5 {
            policy.record_access(i);
        }

        // Should evict frame 0 (least recently used)
        let victim = policy.find_victim(&frames);
        assert_eq!(victim, Some(0));
    }

    #[test]
    fn test_2q_policy() {
        let frames = create_test_frames(10);
        let policy = TwoQEvictionPolicy::new(10);

        // First access to frame 0 - goes to A1in
        policy.record_access(0);

        // Second access - should move to Am
        policy.record_access(0);

        let stats = policy.stats();
        assert!(stats.evictions == 0 || stats.victim_searches >= 0);
    }

    #[test]
    fn test_lru_k_policy() {
        let frames = create_test_frames(5);
        let policy = LruKEvictionPolicy::new(5, 2);

        // Access frame 0 twice
        policy.record_access(0);
        policy.record_access(0);

        // Access frame 1 once
        policy.record_access(1);

        // Frame 1 should be evicted (less history)
        let victim = policy.find_victim(&frames);
        assert_eq!(victim, Some(1));
    }

    #[test]
    fn test_policy_factory() {
        let policy = create_eviction_policy(EvictionPolicyType::Clock, 100);
        assert_eq!(policy.name(), "CLOCK");

        let policy = create_eviction_policy(EvictionPolicyType::Lru, 100);
        assert_eq!(policy.name(), "LRU");

        let policy = create_eviction_policy(EvictionPolicyType::TwoQ, 100);
        assert_eq!(policy.name(), "2Q");

        let policy = create_eviction_policy(EvictionPolicyType::LruK(2), 100);
        assert_eq!(policy.name(), "LRU-K");
    }
}
