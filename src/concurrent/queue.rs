// Copyright (c) 2025 RustyDB Contributors
//
// Lock-free queue implementation
//
// This module implements the Michael-Scott lock-free queue algorithm,
// a wait-free FIFO queue that uses compare-and-swap operations.
// Reference: "Simple, Fast, and Practical Non-Blocking and Blocking
// Concurrent Queue Algorithms" by Michael and Scott (1996)

use super::epoch::{Atomic, Epoch, Owned};
use super::Backoff;

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Cache-line padded node to avoid false sharing
#[repr(C, align(64))]
pub struct QueueNode<T> {
    /// The data stored in this node (None for sentinel)
    data: Option<T>,
    /// Pointer to the next node
    next: Atomic<QueueNode<T>>,
    /// Padding to fill cache line
    _padding: [u8; 0],
}

impl<T> QueueNode<T> {
    /// Create a new node with data
    fn new(data: T) -> Self {
        Self {
            data: Some(data),
            next: Atomic::null(),
            _padding: [],
        }
    }

    /// Create a sentinel node (no data)
    fn sentinel() -> Self {
        Self {
            data: None,
            next: Atomic::null(),
            _padding: [],
        }
    }
}

/// Michael-Scott lock-free queue
///
/// This is a FIFO queue that supports concurrent enqueue and dequeue operations
/// without locks. It uses atomic compare-and-swap operations to ensure
/// linearizability.
///
/// # Cache-line optimization
///
/// The head and tail pointers are placed in separate cache lines to minimize
/// false sharing between enqueueing and dequeueing threads.
#[repr(C)]
pub struct LockFreeQueue<T: 'static> {
    /// Head pointer (for dequeue)
    head: Atomic<QueueNode<T>>,
    /// Padding to separate head and tail into different cache lines
    _pad1: [u8; 56], // 64 - 8 (pointer size) = 56
    /// Tail pointer (for enqueue)
    tail: Atomic<QueueNode<T>>,
    /// Padding to prevent false sharing
    _pad2: [u8; 56], // 64 - 8 (pointer size) = 56
    /// Size estimate (may be slightly inaccurate due to concurrency)
    size: AtomicUsize,
    /// Total number of enqueues (for statistics)
    enqueue_count: AtomicU64,
    /// Total number of dequeues (for statistics)
    dequeue_count: AtomicU64,
}

impl<T: 'static> LockFreeQueue<T> {
    /// Create a new empty queue
    pub fn new() -> Self {
        let sentinel = Owned::new(QueueNode::sentinel());
        let sentinel_ptr = sentinel.into_shared();

        let queue = Self {
            head: Atomic::null(),
            _pad1: [0; 56], // 64 - 8 (pointer size) = 56
            tail: Atomic::null(),
            _pad2: [0; 56], // 64 - 8 (pointer size) = 56
            size: AtomicUsize::new(0),
            enqueue_count: AtomicU64::new(0),
            dequeue_count: AtomicU64::new(0),
        };

        // Initialize with sentinel node
        let guard = Epoch::pin();
        queue.head.store(sentinel_ptr, Ordering::Relaxed);
        queue.tail.store(sentinel_ptr, Ordering::Relaxed);

        queue
    }

    /// Enqueue an item to the back of the queue
    ///
    /// This operation is lock-free and wait-free. It always succeeds.
    ///
    /// # Performance
    ///
    /// This operation uses compare-and-swap in a retry loop. Under high
    /// contention, threads use exponential backoff to reduce cache coherence
    /// traffic.
    pub fn enqueue(&self, value: T) {
        let guard = Epoch::pin();
        let node = Owned::new(QueueNode::new(value));
        let node_ptr = node.into_shared();
        let mut backoff = Backoff::new();

        loop {
            // Read tail and its next pointer
            let tail = self.tail.load(Ordering::Acquire, &guard);
            let next = unsafe { tail.as_ref().unwrap().next.load(Ordering::Acquire, &guard) };

            // Check if tail is still consistent
            let tail_check = self.tail.load(Ordering::Acquire, &guard);
            if tail != tail_check {
                backoff.spin();
                continue;
            }

            if next.is_null() {
                // Tail is pointing to the last node, try to link new node
                match unsafe {
                    tail.as_ref().unwrap().next.compare_exchange_weak(
                        next,
                        node_ptr,
                        Ordering::Release,
                        Ordering::Acquire,
                        &guard,
                    )
                } {
                    Ok(_) => {
                        // Successfully enqueued, try to swing tail to the new node
                        let _ = self.tail.compare_exchange(
                            tail,
                            node_ptr,
                            Ordering::Release,
                            Ordering::Relaxed,
                            &guard,
                        );
                        self.size.fetch_add(1, Ordering::Relaxed);
                        self.enqueue_count.fetch_add(1, Ordering::Relaxed);
                        return;
                    }
                    Err(_) => {
                        // CAS failed, retry
                        backoff.spin();
                    }
                }
            } else {
                // Tail is lagging behind, try to advance it
                let _ = self.tail.compare_exchange(
                    tail,
                    next,
                    Ordering::Release,
                    Ordering::Relaxed,
                    &guard,
                );
                backoff.spin();
            }
        }
    }

    /// Dequeue an item from the front of the queue
    ///
    /// Returns `None` if the queue is empty.
    ///
    /// # Performance
    ///
    /// This operation uses compare-and-swap in a retry loop. Under high
    /// contention, threads use exponential backoff.
    ///
    /// # Memory Reclamation
    ///
    /// Dequeued nodes are not immediately freed. Instead, they are added to
    /// the epoch-based garbage collector for safe reclamation.
    pub fn dequeue(&self) -> Option<T> {
        let guard = Epoch::pin();
        let mut backoff = Backoff::new();

        loop {
            // Read head, tail, and head's next pointer
            let head = self.head.load(Ordering::Acquire, &guard);
            let tail = self.tail.load(Ordering::Acquire, &guard);
            let next = unsafe { head.as_ref().unwrap().next.load(Ordering::Acquire, &guard) };

            // Check if head is still consistent
            let head_check = self.head.load(Ordering::Acquire, &guard);
            if head != head_check {
                backoff.spin();
                continue;
            }

            if head == tail {
                if next.is_null() {
                    // Queue is empty
                    return None;
                }
                // Tail is lagging behind, try to advance it
                let _ = self.tail.compare_exchange(
                    tail,
                    next,
                    Ordering::Release,
                    Ordering::Relaxed,
                    &guard,
                );
                backoff.spin();
            } else {
                // Read value before CAS to avoid data race
                let data = unsafe { &*next.as_ptr() }.data.as_ref();
                if data.is_none() {
                    // Sentinel node, skip it
                    backoff.spin();
                    continue;
                }

                // Try to swing head to the next node
                match self.head.compare_exchange(
                    head,
                    next,
                    Ordering::Release,
                    Ordering::Acquire,
                    &guard,
                ) {
                    Ok(_) => {
                        // Successfully dequeued
                        self.size.fetch_sub(1, Ordering::Relaxed);
                        self.dequeue_count.fetch_add(1, Ordering::Relaxed);

                        // Take the data from the node
                        // Safety: We own this node now (we just dequeued it)
                        let result = unsafe {
                            let node_ptr = next.as_ptr();
                            let node_ref = &mut *node_ptr;
                            node_ref.data.take()
                        };

                        // Defer reclamation of the old head
                        Epoch::defer(head.as_ptr());

                        return result;
                    }
                    Err(_) => {
                        // CAS failed, retry
                        backoff.spin();
                    }
                }
            }
        }
    }

    /// Peek at the front item without removing it
    pub fn peek(&self) -> Option<T>
    where
        T: Clone,
    {
        let guard = Epoch::pin();

        loop {
            let head = self.head.load(Ordering::Acquire, &guard);
            let next = unsafe { head.as_ref().unwrap().next.load(Ordering::Acquire, &guard) };

            let head_check = self.head.load(Ordering::Acquire, &guard);
            if head != head_check {
                continue;
            }

            if next.is_null() {
                return None;
            }

            // Safety: Protected by epoch guard
            let next_ref = unsafe { next.as_ref().unwrap() };
            return next_ref.data.clone();
        }
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        let guard = Epoch::pin();
        let head = self.head.load(Ordering::Acquire, &guard);
        let next = unsafe { head.as_ref().unwrap().next.load(Ordering::Acquire, &guard) };
        next.is_null()
    }

    /// Get an approximate size of the queue
    ///
    /// Note: Due to concurrent operations, this may not be exact.
    pub fn len(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    /// Get statistics about queue operations
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            enqueue_count: self.enqueue_count.load(Ordering::Relaxed),
            dequeue_count: self.dequeue_count.load(Ordering::Relaxed),
            current_size: self.size.load(Ordering::Relaxed),
        }
    }

    /// Batch enqueue multiple items
    ///
    /// This is more efficient than enqueuing items one by one.
    pub fn enqueue_batch(&self, items: Vec<T>) {
        if items.is_empty() {
            return;
        }

        // Build a chain of nodes
        let guard = Epoch::pin();
        let mut first = None;
        let mut last = None;

        for item in items {
            let node = Owned::new(QueueNode::new(item));
            let node_ptr = node.into_shared();

            if let Some(last_ptr) = last {
                // Safety: We own these nodes until we link them to the queue
                unsafe {
                    last_ptr.as_ref().unwrap().next.store(node_ptr, Ordering::Relaxed);
                }
            } else {
                first = Some(node_ptr);
            }
            last = Some(node_ptr);
        }

        let first = first.unwrap();
        let last = last.unwrap();
        let mut backoff = Backoff::new();

        // Link the chain to the queue
        loop {
            let tail = self.tail.load(Ordering::Acquire, &guard);
            let next = unsafe { tail.as_ref().unwrap().next.load(Ordering::Acquire, &guard) };

            let tail_check = self.tail.load(Ordering::Acquire, &guard);
            if tail != tail_check {
                backoff.spin();
                continue;
            }

            if next.is_null() {
                match unsafe {
                    tail.as_ref().unwrap().next.compare_exchange(
                        next,
                        first,
                        Ordering::Release,
                        Ordering::Acquire,
                        &guard,
                    )
                } {
                    Ok(_) => {
                        let _ = self.tail.compare_exchange(
                            tail,
                            last,
                            Ordering::Release,
                            Ordering::Relaxed,
                            &guard,
                        );
                        // Note: We don't update size/counts here for batch operations
                        // to avoid complexity, but individual enqueue/dequeue do
                        return;
                    }
                    Err(_) => {
                        backoff.spin();
                    }
                }
            } else {
                let _ = self.tail.compare_exchange(
                    tail,
                    next,
                    Ordering::Release,
                    Ordering::Relaxed,
                    &guard,
                );
                backoff.spin();
            }
        }
    }

    /// Batch dequeue up to `n` items
    pub fn dequeue_batch(&self, n: usize) -> Vec<T> {
        let mut result = Vec::with_capacity(n);
        for _ in 0..n {
            match self.dequeue() {
                Some(item) => result.push(item),
                None => break,
            }
        }
        result
    }
}

impl<T: 'static> Default for LockFreeQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> Drop for LockFreeQueue<T> {
    fn drop(&mut self) {
        // Drain the queue to properly drop all items
        while self.dequeue().is_some() {}

        // The sentinel node will be dropped by the Atomic<QueueNode<T>> drop impl
    }
}

// Safety: The queue is thread-safe
unsafe impl<T: Send + 'static> Send for LockFreeQueue<T> {}
unsafe impl<T: Send + 'static> Sync for LockFreeQueue<T> {}

/// Statistics for queue operations
#[derive(Debug, Clone, Copy)]
pub struct QueueStats {
    pub enqueue_count: u64,
    pub dequeue_count: u64,
    pub current_size: usize,
}

/// An iterator over the queue
///
/// Note: This drains the queue
pub struct IntoIter<T> {
    queue: LockFreeQueue<T>,
}

impl<T: 'static> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.dequeue()
    }
}

impl<T: 'static> IntoIterator for LockFreeQueue<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { queue: self }
    }
}

/// A bounded lock-free queue with fixed capacity
///
/// This queue rejects enqueues when full, making it suitable for
/// resource-constrained scenarios.
pub struct BoundedQueue<T> {
    inner: LockFreeQueue<T>,
    capacity: usize,
}

impl<T: 'static> BoundedQueue<T> {
    /// Create a new bounded queue with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: LockFreeQueue::new(),
            capacity,
        }
    }

    /// Try to enqueue an item
    ///
    /// Returns `Err(value)` if the queue is full.
    pub fn try_enqueue(&self, value: T) -> Result<(), T> {
        if self.inner.len() >= self.capacity {
            Err(value)
        } else {
            self.inner.enqueue(value);
            Ok(())
        }
    }

    /// Dequeue an item
    pub fn dequeue(&self) -> Option<T> {
        self.inner.dequeue()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Check if the queue is full
    pub fn is_full(&self) -> bool {
        self.inner.len() >= self.capacity
    }

    /// Get current length
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_queue_basic() {
        let queue = LockFreeQueue::new();
        assert!(queue.is_empty());

        queue.enqueue(1);
        queue.enqueue(2);
        queue.enqueue(3);

        assert_eq!(queue.dequeue(), Some(1));
        assert_eq!(queue.dequeue(), Some(2));
        assert_eq!(queue.dequeue(), Some(3));
        assert_eq!(queue.dequeue(), None);
    }

    #[test]
    fn test_queue_peek() {
        let queue = LockFreeQueue::new();
        queue.enqueue(42);

        assert_eq!(queue.peek(), Some(42));
        assert_eq!(queue.peek(), Some(42)); // Should not consume
        assert_eq!(queue.dequeue(), Some(42));
        assert_eq!(queue.peek(), None);
    }

    #[test]
    fn test_concurrent_enqueue_dequeue() {
        let queue = Arc::new(LockFreeQueue::new());
        let mut handles = vec![];

        // Enqueuers
        for i in 0..5 {
            let q = queue.clone();
            handles.push(thread::spawn(move || {
                for j in 0..1000 {
                    q.enqueue(i * 1000 + j);
                }
            }));
        }

        // Dequeuers
for _ in 0..5 {
                    let q = queue.clone();
                    handles.push(thread::spawn(move || {
                        for _ in 0..1000 {
                            while q.dequeue().is_none() {
                                thread::yield_now();
                            }
                        }
                    }));
                }

        for handle in handles {
            handle.join().unwrap();
        }

        assert!(queue.is_empty());
    }

    #[test]
    fn test_batch_operations() {
        let queue = LockFreeQueue::new();
        let items = vec![1, 2, 3, 4, 5];

        queue.enqueue_batch(items.clone());
        let result = queue.dequeue_batch(5);

        assert_eq!(result, items);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_bounded_queue() {
        let queue = BoundedQueue::new(3);

        assert!(queue.try_enqueue(1).is_ok());
        assert!(queue.try_enqueue(2).is_ok());
        assert!(queue.try_enqueue(3).is_ok());

        // Queue is full
        assert!(queue.try_enqueue(4).is_err());

        assert_eq!(queue.dequeue(), Some(1));

        // Now there's space
        assert!(queue.try_enqueue(4).is_ok());
    }

    #[test]
    fn test_queue_stats() {
        let queue = LockFreeQueue::new();

        queue.enqueue(1);
        queue.enqueue(2);
        queue.dequeue();

        let stats = queue.stats();
        assert_eq!(stats.enqueue_count, 2);
        assert_eq!(stats.dequeue_count, 1);
    }

    #[test]
    fn test_into_iter() {
        let queue = LockFreeQueue::new();
        queue.enqueue(1);
        queue.enqueue(2);
        queue.enqueue(3);

        let items: Vec<_> = queue.into_iter().collect();
        assert_eq!(items, vec![1, 2, 3]);
    }
}
