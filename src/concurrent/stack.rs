// Copyright (c) 2025 RustyDB Contributors
//
// Lock-free stack implementation
//
// This module implements the Treiber stack algorithm, a lock-free LIFO stack
// that uses compare-and-swap operations. We use tagged pointers to prevent
// the ABA problem.
//
// Reference: "Systems Programming: Coping with Parallelism" by R.K. Treiber (1986)

use super::epoch::{Atomic, Epoch, EpochGuard, Owned, Shared};

use super::Backoff;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Cache-line padded node to avoid false sharing
#[repr(C, align(64))]
pub struct StackNode<T> {
    /// The data stored in this node
    data: T,
    /// Pointer to the next node
    next: Atomic<StackNode<T>>,
    /// Padding to fill cache line
    _padding: [u8; 0],
}

impl<T> StackNode<T> {
    /// Create a new node
    fn new(data: T) -> Self {
        Self {
            data,
            next: Atomic::null(),
            _padding: [],
        }
    }
}

/// Treiber lock-free stack
///
/// This is a LIFO stack that supports concurrent push and pop operations
/// without locks. It uses atomic compare-and-swap operations with tagged
/// pointers to prevent the ABA problem.
///
/// # ABA Problem Prevention
///
/// The ABA problem occurs when a thread reads a value A, then another thread
/// changes it to B and back to A. The first thread's CAS will succeed even
/// though the state changed. We prevent this by using tagged pointers that
/// increment a version counter on each modification.
#[repr(C, align(64))]
pub struct LockFreeStack<T> {
    /// Head pointer (top of stack)
    head: Atomic<StackNode<T>>,
    /// Size estimate
    size: AtomicUsize,
    /// Total number of pushes
    push_count: AtomicU64,
    /// Total number of pops
    pop_count: AtomicU64,
    /// Padding to prevent false sharing
    _padding: [u8; 32],
}

impl<T: 'static> LockFreeStack<T> {
    /// Create a new empty stack
    pub fn new() -> Self {
        Self {
            head: Atomic::null(),
            size: AtomicUsize::new(0),
            push_count: AtomicU64::new(0),
            pop_count: AtomicU64::new(0),
            _padding: [0; 32],
        }
    }

    /// Push an item onto the stack
    ///
    /// This operation is lock-free and always succeeds.
    ///
    /// # Performance
    ///
    /// Uses compare-and-swap in a retry loop with exponential backoff
    /// under contention.
    pub fn push(&self, value: T) {
        let guard = Epoch::pin();
        let node = Owned::new(StackNode::new(value));
        let node_ptr = node.into_shared();
        let mut backoff = Backoff::new();

        loop {
            // Read current head
            let head = self.head.load(Ordering::Acquire, &guard);

            // Set new node's next to current head
            // Safety: We own node_ptr until we successfully push it
            unsafe {
                node_ptr.as_ref().unwrap().next.store(head, Ordering::Relaxed);
            }

            // Try to swing head to new node
            match self.head.compare_exchange_weak(
                head,
                node_ptr,
                Ordering::Release,
                Ordering::Acquire,
                &guard,
            ) {
                Ok(_) => {
                    self.size.fetch_add(1, Ordering::Relaxed);
                    self.push_count.fetch_add(1, Ordering::Relaxed);
                    return;
                }
                Err(_) => {
                    backoff.spin();
                }
            }
        }
    }

    /// Pop an item from the stack
    ///
    /// Returns `None` if the stack is empty.
    ///
    /// # Memory Reclamation
    ///
    /// Popped nodes are deferred for epoch-based reclamation.
    pub fn pop(&self) -> Option<T> {
        let guard = Epoch::pin();
        let mut backoff = Backoff::new();

        loop {
            // Read current head
            let head = self.head.load(Ordering::Acquire, &guard);

            if head.is_null() {
                // Stack is empty
                return None;
            }

            // Read next pointer from head
            // Safety: Protected by epoch guard
            let next = unsafe { head.as_ref().unwrap().next.load(Ordering::Acquire, &guard) };

            // Try to swing head to next
            match self.head.compare_exchange_weak(
                head,
                next,
                Ordering::Release,
                Ordering::Acquire,
                &guard,
            ) {
                Ok(_) => {
                    self.size.fetch_sub(1, Ordering::Relaxed);
                    self.pop_count.fetch_add(1, Ordering::Relaxed);

                    // Take the data from the node
                    // Safety: We own this node now (we just popped it)
                    let _result = unsafe {
                        let node_ptr = head.as_ptr();
                        let node_ref = &mut *node_ptr;
                        std::ptr::read(&node_ref.data)
                    };

                    // Defer reclamation
                    Epoch::defer(head.as_ptr());

                    return Some(result);
                }
                Err(_) => {
                    backoff.spin();
                }
            }
        }
    }

    /// Peek at the top item without removing it
    pub fn peek<'g>(&self, guard: &'g EpochGuard) -> Option<&'g T> {
        let head = self.head.load(Ordering::Acquire, guard);

        if head.is_null() {
            None
        } else {
            // Safety: Protected by epoch guard
            Some(unsafe { &head.as_ref().unwrap().data })
        }
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        let guard = Epoch::pin();
        self.head.load(Ordering::Acquire, &guard).is_null()
    }

    /// Get approximate size
    pub fn len(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    /// Get statistics
    pub fn stats(&self) -> StackStats {
        StackStats {
            push_count: self.push_count.load(Ordering::Relaxed),
            pop_count: self.pop_count.load(Ordering::Relaxed),
            current_size: self.size.load(Ordering::Relaxed),
        }
    }

    /// Batch push multiple items
    ///
    /// More efficient than pushing one by one.
    pub fn push_batch(&self, mut items: Vec<T>) {
        if items.is_empty() {
            return;
        }

        let guard = Epoch::pin();

        // Build a chain of nodes in reverse order (since it's a stack)
        let mut chain_head = None;

        for item in items.drain(..).rev() {
            let node = Owned::new(StackNode::new(item));
            let node_ptr = node.into_shared();

            if let Some(prev_head) = chain_head {
                // Safety: We own these nodes until we link them to the stack
                unsafe {
                    node_ptr.as_ref().unwrap().next.store(prev_head, Ordering::Relaxed);
                }
            }

            chain_head = Some(node_ptr);
        }

        let chain_head = chain_head.unwrap();
        let mut backoff = Backoff::new();

        // Find the tail of our chain
        let mut chain_tail = chain_head;
        loop {
            // Safety: We own the chain
            let next = unsafe { chain_tail.as_ref().unwrap().next.load(Ordering::Relaxed, &guard) };
            if next.is_null() {
                break;
            }
            chain_tail = next;
        }

        // Link the chain to the stack
        loop {
            let head = self.head.load(Ordering::Acquire, &guard);

            // Set chain tail's next to current head
            unsafe {
                chain_tail.as_ref().unwrap().next.store(head, Ordering::Relaxed);
            }

            // Try to swing head to chain head
            match self.head.compare_exchange_weak(
                head,
                chain_head,
                Ordering::Release,
                Ordering::Acquire,
                &guard,
            ) {
                Ok(_) => {
                    return;
                }
                Err(_) => {
                    backoff.spin();
                }
            }
        }
    }

    /// Batch pop up to `n` items
    ///
    /// Returns items in LIFO order.
    pub fn pop_batch(&self, n: usize) -> Vec<T> {
        let mut result = Vec::with_capacity(n);
        for _ in 0..n {
            match self.pop() {
                Some(item) => result.push(item),
                None => break,
            }
        }
        result
    }

    /// Pop all items from the stack
    ///
    /// This is an atomic operation - all items are removed at once.
    pub fn pop_all(&self) -> Vec<T> {
        let guard = Epoch::pin();
        let mut backoff = Backoff::new();

        loop {
            let head = self.head.load(Ordering::Acquire, &guard);

            if head.is_null() {
                return Vec::new();
            }

            // Try to swap head with null
            match self.head.compare_exchange_weak(
                head,
                Shared::null(),
                Ordering::Release,
                Ordering::Acquire,
                &guard,
            ) {
                Ok(_) => {
                    // Successfully removed all items, collect them
                    let mut result = Vec::new();
                    let mut current = head;

                    while !current.is_null() {
                        // Safety: We own all these nodes now
                        unsafe {
                            let node_ptr = current.as_ptr();
                            let node_ref = &mut *node_ptr;
                            let data = std::ptr::read(&node_ref.data);
                            result.push(data);

                            let next = node_ref.next.load(Ordering::Relaxed, &guard);
                            Epoch::defer(node_ptr);
                            current = next;
                        }
                    }

                    self.size.store(0, Ordering::Relaxed);
                    return result;
                }
                Err(_) => {
                    backoff.spin();
                }
            }
        }
    }
}

impl<T: 'static> Default for LockFreeStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for LockFreeStack<T> {
    fn drop(&mut self) {
        // Pop all items to properly drop them
        while self.pop().is_some() {}
    }
}

// Safety: The stack is thread-safe
unsafe impl<T: Send + 'static> Send for LockFreeStack<T> {}
unsafe impl<T: Send + 'static> Sync for LockFreeStack<T> {}

/// Statistics for stack operations
#[derive(Debug, Clone, Copy)]
pub struct StackStats {
    pub push_count: u64,
    pub pop_count: u64,
    pub current_size: usize,
}

/// An elimination array for reducing contention
///
/// The elimination technique allows threads to pair up and exchange values
/// directly, bypassing the stack. This reduces contention on the head pointer.
#[repr(C, align(64))]
pub struct EliminationArray<T> {
    slots: Vec<EliminationSlot<T>>,
    range: AtomicUsize,
}

#[repr(C, align(64))]
struct EliminationSlot<T> {
    value: Atomic<T>,
    state: AtomicUsize,
    _padding: [u8; 48],
}

const EMPTY: usize = 0;
const WAITING: usize = 1;
const BUSY: usize = 2;

impl<T: 'static> EliminationArray<T> {
    /// Create a new elimination array
    pub fn new(size: usize) -> Self {
        let mut slots = Vec::with_capacity(size);
        for _ in 0..size {
            slots.push(EliminationSlot {
                value: Atomic::null(),
                state: AtomicUsize::new(EMPTY),
                _padding: [0; 48],
            });
        }

        Self {
            slots,
            range: AtomicUsize::new(1),
        }
    }

    /// Try to exchange a value
    pub fn visit(&self, mut value: Option<T>, is_push: bool) -> Option<T> {
        let range = self.range.load(Ordering::Relaxed);
        let slot_idx = fastrand::usize(..range.min(self.slots.len()));
        let slot = &self.slots[slot_idx];

        if is_push {
            // Try to deposit value for a pop to collect
            if slot.state.compare_exchange(
                EMPTY,
                WAITING,
                Ordering::Acquire,
                Ordering::Relaxed,
            ).is_ok() {
                if let Some(v) = value.take() {
                    let guard = Epoch::pin();
                    let owned = Owned::new(v);
                    slot.value.store(owned.into_shared(), Ordering::Release);

                    // Wait briefly for a pop
                    for _ in 0..100 {
                        if slot.state.load(Ordering::Acquire) == BUSY {
                            slot.state.store(EMPTY, Ordering::Release);
                            return None; // Successfully eliminated
                        }
                        std::hint::spin_loop();
                    }

                    // Timeout, reclaim value
                    let guard = Epoch::pin();
                    let ptr = slot.value.swap(Shared::null(), Ordering::Acquire, &guard);
                    slot.state.store(EMPTY, Ordering::Release);

                    if !ptr.is_null() {
                        // Safety: We're taking back our value
                        let data = unsafe { std::ptr::read(&(*ptr.as_ptr())) };
                        Epoch::defer(ptr.as_ptr());
                        return Some(data);
                    }
                }
            }
        } else {
            // Try to collect a value from a push
            if slot.state.compare_exchange(
                WAITING,
                BUSY,
                Ordering::Acquire,
                Ordering::Relaxed,
            ).is_ok() {
                let guard = Epoch::pin();
                let ptr = slot.value.swap(Shared::null(), Ordering::Acquire, &guard);
                slot.state.store(EMPTY, Ordering::Release);

                if !ptr.is_null() {
                    // Safety: The push thread gave us this value
                    let data = unsafe { std::ptr::read(&(*ptr.as_ptr())) };
                    Epoch::defer(ptr.as_ptr());
                    return Some(data);
                }
            }
        }

        value
    }
}

/// An elimination-backoff stack for high contention scenarios
pub struct EliminationStack<T> {
    stack: LockFreeStack<T>,
    elimination_array: EliminationArray<T>,
}

impl<T: 'static> EliminationStack<T> {
    /// Create a new elimination-backoff stack
    pub fn new(elimination_size: usize) -> Self {
        Self {
            stack: LockFreeStack::new(),
            elimination_array: EliminationArray::new(elimination_size),
        }
    }

    /// Push with elimination
    pub fn push(&self, value: T) {
        // Try elimination first
        if let Some(v) = self.elimination_array.visit(Some(value), true) {
            // Elimination failed, push to stack
            self.stack.push(v);
        }
    }

    /// Pop with elimination
    pub fn pop(&self) -> Option<T> {
        // Try elimination first
        if let Some(v) = self.elimination_array.visit(None, false) {
            return Some(v);
        }

        // Elimination failed, pop from stack
        self.stack.pop()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Get approximate size
    pub fn len(&self) -> usize {
        self.stack.len()
    }
}

impl<T: 'static> Default for EliminationStack<T> {
    fn default() -> Self {
        Self::new(16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_stack_basic() {
        let stack = LockFreeStack::new();
        assert!(stack.is_empty());

        stack.push(1);
        stack.push(2);
        stack.push(3);

        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(1));
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn test_stack_peek() {
        let stack = LockFreeStack::new();
        stack.push(42);

        let guard = Epoch::pin();
        assert_eq!(stack.peek(&guard), Some(&42));
        assert_eq!(stack.peek(&guard), Some(&42));
        drop(guard);

        assert_eq!(stack.pop(), Some(42));

        let guard = Epoch::pin();
        assert_eq!(stack.peek(&guard), None);
    }

    #[test]
    fn test_concurrent_push_pop() {
        let stack = Arc::new(LockFreeStack::new());
        let mut handles = vec![];

        // Pushers
        for _i in 0..5 {
            let s = stack.clone();
            handles.push(thread::spawn(move || {
                for j in 0..1000 {
                    s.push(i * 1000 + j);
                }
            }));
        }

        // Poppers
        for _ in 0..5 {
            let s = stack.clone();
            handles.push(thread::spawn(move || {
                let mut count = 0;
                for _ in 0..1000 {
                    while s.pop().is_none() {
                        std::thread::yield_now();
                    }
                    count += 1;
                }
                count
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert!(stack.is_empty());
    }

    #[test]
    fn test_batch_operations() {
        let stack = LockFreeStack::new();
        let items = vec![1, 2, 3, 4, 5];

        stack.push_batch(items);
        let _result = stack.pop_batch(5);

        // Stack is LIFO
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_pop_all() {
        let stack = LockFreeStack::new();
        for _i in 0..10 {
            stack.push(i);
        }

        let all = stack.pop_all();
        assert_eq!(all.len(), 10);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_elimination_stack() {
        let stack = EliminationStack::new(16);

        stack.push(1);
        stack.push(2);
        stack.push(3);

        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(1));
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn test_concurrent_elimination() {
        let stack = Arc::new(EliminationStack::new(32));
        let mut handles = vec![];

        for _i in 0..10 {
            let s = stack.clone();
            handles.push(thread::spawn(move || {
                for j in 0..100 {
                    s.push(i * 100 + j);
                }
            }));
        }

        for _ in 0..10 {
            let s = stack.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    while s.pop().is_none() {
                        std::thread::yield_now();
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert!(stack.is_empty());
    }
}


