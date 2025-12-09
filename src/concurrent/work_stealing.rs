// Copyright (c) 2025 RustyDB Contributors
//
// Work-stealing deque implementation
//
// This module implements the Chase-Lev work-stealing deque, a highly efficient
// concurrent deque designed for work-stealing schedulers. The owner can push
// and pop from one end, while thieves can steal from the other end.
//
// Reference: "Dynamic Circular Work-Stealing Deque" by Chase and Lev (2005)

use super::Backoff;
use std::cell::UnsafeCell;
use std::mem::{self, MaybeUninit};
use std::ptr;
use std::sync::atomic::{fence, AtomicIsize, AtomicPtr, AtomicU64, Ordering};
use std::sync::Arc;

/// Minimum buffer size
const MIN_BUFFER_SIZE: usize = 32;

/// Maximum buffer size
const MAX_BUFFER_SIZE: usize = 1 << 30;

/// A growable circular buffer for the deque
struct Buffer<T> {
    /// The actual storage
    storage: Box<[UnsafeCell<MaybeUninit<T>>]>,
    /// Capacity (always power of 2)
    capacity: usize,
}

impl<T> Buffer<T> {
    /// Create a new buffer with the given capacity
    fn new(capacity: usize) -> Self {
        assert!(capacity.is_power_of_two());
        let mut storage = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            storage.push(UnsafeCell::new(MaybeUninit::uninit()));
        }

        Self {
            storage: storage.into_boxed_slice(),
            capacity,
        }
    }

    /// Get element at index
    ///
    /// # Safety
    ///
    /// The caller must ensure proper synchronization and that the element
    /// at this index has been initialized.
    unsafe fn get(&self, index: isize) -> &T {
        let idx = (index as usize) & (self.capacity - 1);
        (*self.storage[idx].get()).assume_init_ref()
    }

    /// Write element at index
    ///
    /// # Safety
    ///
    /// The caller must ensure proper synchronization.
    unsafe fn put(&self, index: isize, value: T) {
        let idx = (index as usize) & (self.capacity - 1);
        (*self.storage[idx].get()).write(value);
    }

    /// Take element at index
    ///
    /// # Safety
    ///
    /// The caller must ensure proper synchronization and that the element
    /// at this index has been initialized.
    unsafe fn take(&self, index: isize) -> T {
        let idx = (index as usize) & (self.capacity - 1);
        (*self.storage[idx].get()).assume_init_read()
    }

    /// Grow the buffer to double the capacity
    fn grow(&self, old_bottom: isize, old_top: isize) -> Buffer<T> {
        let new_capacity = (self.capacity * 2).min(MAX_BUFFER_SIZE);
        let new_buffer = Buffer::new(new_capacity);

        // Copy elements from old buffer to new buffer
        for i in old_top..old_bottom {
            // Safety: These elements are initialized and we have exclusive access
            unsafe {
                let value = self.get(i);
                let ptr = value as *const T;
                new_buffer.put(i, ptr::read(ptr));
            }
        }

        new_buffer
    }
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self) {
        // Elements are dropped by the deque, not the buffer
    }
}

// Safety: Buffer is only accessed through proper synchronization
unsafe impl<T: Send> Send for Buffer<T> {}
unsafe impl<T: Send> Sync for Buffer<T> {}

/// Chase-Lev work-stealing deque
///
/// This deque is designed for a single owner thread that can push and pop
/// from the bottom, while multiple thief threads can steal from the top.
///
/// # Lock-free guarantees
///
/// - Owner operations (push/pop) are wait-free when no stealing occurs
/// - Stealer operations are lock-free
/// - The implementation uses minimal atomic operations for best performance
#[repr(C, align(64))]
pub struct WorkStealingDeque<T> {
    /// Bottom index (owner only)
    bottom: AtomicIsize,
    /// Padding to separate bottom and top into different cache lines
    _pad1: [u8; 64 - size_of::<AtomicIsize>()],
    /// Top index (shared between owner and stealers)
    top: AtomicIsize,
    /// Padding
    _pad2: [u8; 64 - size_of::<AtomicIsize>()],
    /// Current buffer
    buffer: AtomicPtr<Buffer<T>>,
    /// Statistics
    push_count: AtomicU64,
    pop_count: AtomicU64,
    steal_count: AtomicU64,
    steal_attempt_count: AtomicU64,
    grow_count: AtomicU64,
}

impl<T> WorkStealingDeque<T> {
    /// Create a new work-stealing deque
    pub fn new() -> Self {
        let buffer = Box::into_raw(Box::new(Buffer::new(MIN_BUFFER_SIZE)));

        Self {
            bottom: AtomicIsize::new(0),
            _pad1: [0; 64 - size_of::<AtomicIsize>()],
            top: AtomicIsize::new(0),
            _pad2: [0; 64 - size_of::<AtomicIsize>()],
            buffer: AtomicPtr::new(buffer),
            push_count: AtomicU64::new(0),
            pop_count: AtomicU64::new(0),
            steal_count: AtomicU64::new(0),
            steal_attempt_count: AtomicU64::new(0),
            grow_count: AtomicU64::new(0),
        }
    }

    /// Push an item to the bottom of the deque (owner only)
    ///
    /// # Safety
    ///
    /// Only the owner thread may call this method.
    pub fn push(&self, value: T) {
        let bottom = self.bottom.load(Ordering::Relaxed);
        let top = self.top.load(Ordering::Acquire);
        let buffer = unsafe { &*self.buffer.load(Ordering::Relaxed) };

        let size = bottom - top;
        if size >= buffer.capacity as isize {
            // Buffer is full, need to grow
            let new_buffer = buffer.grow(bottom, top);
            let new_buffer_ptr = Box::into_raw(Box::new(new_buffer));

            self.buffer.store(new_buffer_ptr, Ordering::Release);
            self.grow_count.fetch_add(1, Ordering::Relaxed);

            // Note: We're leaking the old buffer here. In a production implementation,
            // we would use epoch-based reclamation to safely free it.
        }

        let buffer = unsafe { &*self.buffer.load(Ordering::Relaxed) };

        // Safety: We're the owner, and we have exclusive access to bottom
        unsafe {
            buffer.put(bottom, value);
        }

        fence(Ordering::Release);
        self.bottom.store(bottom + 1, Ordering::Relaxed);
        self.push_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Pop an item from the bottom of the deque (owner only)
    ///
    /// Returns `None` if the deque is empty.
    ///
    /// # Safety
    ///
    /// Only the owner thread may call this method.
    pub fn pop(&self) -> Option<T> {
        let bottom = self.bottom.load(Ordering::Relaxed);
        let buffer = unsafe { &*self.buffer.load(Ordering::Relaxed) };

        let new_bottom = bottom - 1;
        self.bottom.store(new_bottom, Ordering::Relaxed);
        fence(Ordering::SeqCst);

        let top = self.top.load(Ordering::Relaxed);

        if top <= new_bottom {
            // Non-empty deque
            let value = unsafe { buffer.take(new_bottom) };

            if top == new_bottom {
                // Last element, race with stealers
                if self
                    .top
                    .compare_exchange(top, top + 1, Ordering::SeqCst, Ordering::Relaxed)
                    .is_err()
                {
                    // Lost the race, a stealer got it
                    // Safety: We need to put the value back or forget it
                    mem::forget(value);
                    self.bottom.store(bottom, Ordering::Relaxed);
                    return None;
                }

                self.bottom.store(bottom, Ordering::Relaxed);
                self.pop_count.fetch_add(1, Ordering::Relaxed);
                return Some(value);
            }

            self.pop_count.fetch_add(1, Ordering::Relaxed);
            Some(value)
        } else {
            // Empty deque
            self.bottom.store(bottom, Ordering::Relaxed);
            None
        }
    }

    /// Steal an item from the top of the deque (stealers only)
    ///
    /// Returns `None` if the deque is empty or the steal failed due to
    /// contention.
    pub fn steal(&self) -> Steal<T> {
        self.steal_attempt_count.fetch_add(1, Ordering::Relaxed);

        let top = self.top.load(Ordering::Acquire);
        fence(Ordering::SeqCst);
        let bottom = self.bottom.load(Ordering::Acquire);

        if top >= bottom {
            // Empty deque
            return Steal::Empty;
        }

        let buffer = unsafe { &*self.buffer.load(Ordering::Acquire) };

        // Safety: Protected by the CAS below
        let value = unsafe { buffer.get(top) };
        let value_ptr = value as *const T;

        // Try to increment top
        if self
            .top
            .compare_exchange(top, top + 1, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok()
        {
            // Successfully stole
            self.steal_count.fetch_add(1, Ordering::Relaxed);
            // Safety: We won the CAS, so we own this value
            Steal::Success(unsafe { ptr::read(value_ptr) })
        } else {
            // Failed to steal due to contention
            Steal::Retry
        }
    }

    /// Check if the deque is empty
    pub fn is_empty(&self) -> bool {
        let bottom = self.bottom.load(Ordering::Relaxed);
        let top = self.top.load(Ordering::Relaxed);
        bottom <= top
    }

    /// Get approximate size
    pub fn len(&self) -> usize {
        let bottom = self.bottom.load(Ordering::Relaxed);
        let top = self.top.load(Ordering::Relaxed);
        (bottom - top).max(0) as usize
    }

    /// Get statistics
    pub fn stats(&self) -> DequeStats {
        DequeStats {
            push_count: self.push_count.load(Ordering::Relaxed),
            pop_count: self.pop_count.load(Ordering::Relaxed),
            steal_count: self.steal_count.load(Ordering::Relaxed),
            steal_attempt_count: self.steal_attempt_count.load(Ordering::Relaxed),
            grow_count: self.grow_count.load(Ordering::Relaxed),
            current_size: self.len(),
        }
    }
}

impl<T> Default for WorkStealingDeque<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for WorkStealingDeque<T> {
    fn drop(&mut self) {
        // Pop all remaining items to drop them
        while self.pop().is_some() {}

        // Drop the buffer
        let buffer = self.buffer.load(Ordering::Relaxed);
        unsafe {
            drop(Box::from_raw(buffer));
        }
    }
}

// Safety: The deque is thread-safe with proper usage (owner vs stealer)
unsafe impl<T: Send> Send for WorkStealingDeque<T> {}
unsafe impl<T: Send> Sync for WorkStealingDeque<T> {}

/// Result of a steal operation
#[derive(Debug)]
pub enum Steal<T> {
    /// Successfully stole a value
    Success(T),
    /// Deque was empty
    Empty,
    /// Steal failed due to contention, retry
    Retry,
}

impl<T> Steal<T> {
    /// Check if steal was successful
    pub fn is_success(&self) -> bool {
        matches!(self, Steal::Success(_))
    }

    /// Check if deque was empty
    pub fn is_empty(&self) -> bool {
        matches!(self, Steal::Empty)
    }

    /// Check if should retry
    pub fn is_retry(&self) -> bool {
        matches!(self, Steal::Retry)
    }

    /// Convert to Option
    pub fn into_option(self) -> Option<T> {
        match self {
            Steal::Success(v) => Some(v),
            _ => None,
        }
    }
}

/// Statistics for the deque
#[derive(Debug, Clone, Copy)]
pub struct DequeStats {
    pub push_count: u64,
    pub pop_count: u64,
    pub steal_count: u64,
    pub steal_attempt_count: u64,
    pub grow_count: u64,
    pub current_size: usize,
}

/// A worker in a work-stealing thread pool
pub struct Worker<T> {
    deque: Arc<WorkStealingDeque<T>>,
    stealers: Vec<Stealer<T>>,
}

impl<T> Worker<T> {
    /// Create a new worker with connections to other workers
    pub fn new(deque: Arc<WorkStealingDeque<T>>, stealers: Vec<Stealer<T>>) -> Self {
        Self { deque, stealers }
    }

    /// Push a task
    pub fn push(&self, value: T) {
        self.deque.push(value);
    }

    /// Pop a task (local)
    pub fn pop(&self) -> Option<T> {
        self.deque.pop()
    }

    /// Try to get work (pop local, then steal from others)
    pub fn get_work(&self) -> Option<T> {
        // Try local work first
        if let Some(work) = self.pop() {
            return Some(work);
        }

        // Try stealing from others
        let mut backoff = Backoff::new();
        for _ in 0..3 {
            // Try a few rounds
            for stealer in &self.stealers {
                loop {
                    match stealer.steal() {
                        Steal::Success(v) => return Some(v),
                        Steal::Empty => break,
                        Steal::Retry => {
                            backoff.spin();
                        }
                    }
                }
            }
            backoff.snooze();
        }

        None
    }
}

/// A stealer that can steal from a deque
#[derive(Clone)]
pub struct Stealer<T> {
    deque: Arc<WorkStealingDeque<T>>,
}

impl<T> Stealer<T> {
    /// Create a new stealer for a deque
    pub fn new(deque: Arc<WorkStealingDeque<T>>) -> Self {
        Self { deque }
    }

    /// Steal a task
    pub fn steal(&self) -> Steal<T> {
        self.deque.steal()
    }

    /// Check if the deque is empty
    pub fn is_empty(&self) -> bool {
        self.deque.is_empty()
    }
}

/// A work-stealing thread pool
pub struct WorkStealingPool<T> {
    workers: Vec<Arc<WorkStealingDeque<T>>>,
}

impl<T> WorkStealingPool<T> {
    /// Create a new work-stealing pool with the given number of workers
    pub fn new(num_workers: usize) -> (Self, Vec<Worker<T>>) {
        let mut workers = Vec::with_capacity(num_workers);
        for _ in 0..num_workers {
            workers.push(Arc::new(WorkStealingDeque::new()));
        }

        let pool = Self {
            workers: workers.clone(),
        };

        // Create worker handles
        let mut worker_handles = Vec::with_capacity(num_workers);
        for (i, deque) in workers.iter().enumerate() {
            let mut stealers = Vec::new();
            for (j, other) in workers.iter().enumerate() {
                if i != j {
                    stealers.push(Stealer::new(other.clone()));
                }
            }
            worker_handles.push(Worker::new(deque.clone(), stealers));
        }

        (pool, worker_handles)
    }

    /// Get a stealer for a specific worker
    pub fn stealer(&self, worker_id: usize) -> Option<Stealer<T>> {
        self.workers.get(worker_id).map(|d| Stealer::new(d.clone()))
    }

    /// Get all stealers
    pub fn stealers(&self) -> Vec<Stealer<T>> {
        self.workers.iter().map(|d| Stealer::new(d.clone())).collect()
    }

    /// Get aggregate statistics
    pub fn stats(&self) -> PoolStats {
        let mut total_push = 0;
        let mut total_pop = 0;
        let mut total_steal = 0;
        let mut total_steal_attempt = 0;
        let mut total_grow = 0;
        let mut total_size = 0;

        for worker in &self.workers {
            let stats = worker.stats();
            total_push += stats.push_count;
            total_pop += stats.pop_count;
            total_steal += stats.steal_count;
            total_steal_attempt += stats.steal_attempt_count;
            total_grow += stats.grow_count;
            total_size += stats.current_size;
        }

        PoolStats {
            num_workers: self.workers.len(),
            total_push,
            total_pop,
            total_steal,
            total_steal_attempt,
            total_grow,
            total_size,
            steal_success_rate: if total_steal_attempt > 0 {
                total_steal as f64 / total_steal_attempt as f64
            } else {
                0.0
            },
        }
    }
}

/// Statistics for the work-stealing pool
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub num_workers: usize,
    pub total_push: u64,
    pub total_pop: u64,
    pub total_steal: u64,
    pub total_steal_attempt: u64,
    pub total_grow: u64,
    pub total_size: usize,
    pub steal_success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::thread;

    #[test]
    fn test_basic_push_pop() {
        let deque = WorkStealingDeque::new();

        deque.push(1);
        deque.push(2);
        deque.push(3);

        assert_eq!(deque.pop(), Some(3));
        assert_eq!(deque.pop(), Some(2));
        assert_eq!(deque.pop(), Some(1));
        assert_eq!(deque.pop(), None);
    }

    #[test]
    fn test_steal() {
        let deque = Arc::new(WorkStealingDeque::new());

        deque.push(1);
        deque.push(2);
        deque.push(3);

        let stealer = Stealer::new(deque.clone());

        match stealer.steal() {
            Steal::Success(v) => assert_eq!(v, 1),
            _ => panic!("Expected success"),
        }

        assert_eq!(deque.pop(), Some(3));
        assert_eq!(deque.pop(), Some(2));
    }

    #[test]
    fn test_concurrent_steal() {
        let deque = Arc::new(WorkStealingDeque::new());

        // Push many items
        for i in 0..1000 {
            deque.push(i);
        }

        let barrier = Arc::new(Barrier::new(11));
        let mut handles = vec![];

        // Owner pops
        {
            let d = deque.clone();
            let b = barrier.clone();
            handles.push(thread::spawn(move || {
                b.wait();
                let mut count = 0;
                while d.pop().is_some() {
                    count += 1;
                }
                count
            }));
        }

        // Stealers
        for _ in 0..10 {
            let stealer = Stealer::new(deque.clone());
            let b = barrier.clone();
            handles.push(thread::spawn(move || {
                b.wait();
                let mut count = 0;
                loop {
                    match stealer.steal() {
                        Steal::Success(_) => count += 1,
                        Steal::Empty => break,
                        Steal::Retry => {}
                    }
                }
                count
            }));
        }

        let mut total = 0;
        for handle in handles {
            total += handle.join().unwrap();
        }

        assert_eq!(total, 1000);
    }

    #[test]
    fn test_worker_pool() {
        let (pool, mut workers) = WorkStealingPool::new(4);

        // Take one worker out
        let worker = workers.pop().unwrap();

        // Push work
        for i in 0..100 {
            worker.push(i);
        }

        // Other workers steal
        let mut handles = vec![];
        for worker in workers {
            handles.push(thread::spawn(move || {
                let mut count = 0;
                while worker.get_work().is_some() {
                    count += 1;
                }
                count
            }));
        }

        let mut total = 0;
        for handle in handles {
            total += handle.join().unwrap();
        }

        // Should have stolen most/all items
        assert!(total > 0);
    }

    #[test]
    fn test_grow() {
        let deque = WorkStealingDeque::new();

        // Push more than initial capacity
        for i in 0..100 {
            deque.push(i);
        }

        let stats = deque.stats();
        assert!(stats.grow_count > 0);

        // Pop all
        for i in (0..100).rev() {
            assert_eq!(deque.pop(), Some(i));
        }
    }
}


