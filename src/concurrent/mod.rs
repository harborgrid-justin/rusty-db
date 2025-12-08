// Copyright (c) 2025 RustyDB Contributors
//
// Lock-free concurrent data structures module
//
// This module provides highly concurrent, lock-free data structures optimized for
// multi-threaded database operations. All structures use atomic operations and
// careful memory ordering to achieve linearizability without locks.

pub mod epoch;
pub mod queue;
pub mod stack;
pub mod hashmap;
pub mod work_stealing;
pub mod skiplist;
pub mod rwlock_wp;
pub mod hazard;

// Re-export main types
pub use epoch::{Epoch, EpochGuard, Atomic, Owned, Shared};
pub use queue::{LockFreeQueue, QueueNode};
pub use stack::{LockFreeStack, StackNode};
pub use hashmap::{ConcurrentHashMap, Bucket};
pub use work_stealing::{WorkStealingDeque, Worker, Stealer};
pub use skiplist::{LockFreeSkipList, SkipListStats};
pub use rwlock_wp::{RwLockWP, RwLockReadGuard, RwLockWriteGuard};
pub use hazard::{HazardGuard, HazardDomain, HazardScope, Protected, retire, HazardStats};

/// Cache line size for padding to avoid false sharing
pub const CACHE_LINE_SIZE: usize = 64;

/// Tagged pointer utilities for ABA problem prevention
pub mod tagged_ptr {
    use std::marker::PhantomData;

    /// A tagged pointer combines a pointer with a version counter to prevent ABA problems
    #[derive(Debug)]
    pub struct TaggedPtr<T> {
        data: usize,
        _marker: PhantomData<*mut T>,
    }

    impl<T> TaggedPtr<T> {
        const TAG_BITS: usize = 16;
        const TAG_MASK: usize = (1 << Self::TAG_BITS) - 1;
        const PTR_MASK: usize = !Self::TAG_MASK;

        /// Create a new tagged pointer
        pub fn new(ptr: *mut T, tag: usize) -> Self {
            let addr = ptr as usize;
            debug_assert_eq!(addr & Self::TAG_MASK, 0, "Pointer not aligned");
            Self {
                data: addr | (tag & Self::TAG_MASK),
                _marker: PhantomData,
            }
        }

        /// Create a null tagged pointer
        pub fn null() -> Self {
            Self {
                data: 0,
                _marker: PhantomData,
            }
        }

        /// Extract the pointer
        pub fn ptr(&self) -> *mut T {
            (self.data & Self::PTR_MASK) as *mut T
        }

        /// Extract the tag
        pub fn tag(&self) -> usize {
            self.data & Self::TAG_MASK
        }

        /// Check if pointer is null
        pub fn is_null(&self) -> bool {
            self.ptr().is_null()
        }

        /// Decompose into pointer and tag
        pub fn decompose(&self) -> (*mut T, usize) {
            (self.ptr(), self.tag())
        }

        /// Create from raw usize value
        pub fn from_raw(data: usize) -> Self {
            Self {
                data,
                _marker: PhantomData,
            }
        }

        /// Convert to raw usize value
        pub fn into_raw(self) -> usize {
            self.data
        }
    }

    impl<T> Clone for TaggedPtr<T> {
        fn clone(&self) -> Self {
            Self {
                data: self.data,
                _marker: PhantomData,
            }
        }
    }

    impl<T> Copy for TaggedPtr<T> {}

    impl<T> PartialEq for TaggedPtr<T> {
        fn eq(&self, other: &Self) -> bool {
            self.data == other.data
        }
    }

    impl<T> Eq for TaggedPtr<T> {}
}

/// Backoff strategy for CAS retry loops
pub struct Backoff {
    step: u32,
    max_step: u32,
}

impl Backoff {
    /// Create a new backoff strategy
    pub fn new() -> Self {
        Self {
            step: 0,
            max_step: 10,
        }
    }

    /// Perform a backoff step
    pub fn spin(&mut self) {
        for _ in 0..(1 << self.step.min(self.max_step)) {
            std::hint::spin_loop();
        }
        self.step = self.step.saturating_add(1);
    }

    /// Reset backoff to initial state
    pub fn reset(&mut self) {
        self.step = 0;
    }

    /// Check if we should yield to scheduler
    pub fn should_yield(&self) -> bool {
        self.step > 6
    }

    /// Snooze - either spin or yield
    pub fn snooze(&mut self) {
        if self.should_yield() {
            std::thread::yield_now();
        } else {
            self.spin();
        }
    }
}

impl Default for Backoff {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_tagged_ptr() {
        let value = Box::into_raw(Box::new(42));
        let tagged = tagged_ptr::TaggedPtr::new(value, 5);

        assert_eq!(tagged.ptr(), value);
        assert_eq!(tagged.tag(), 5);
        assert!(!tagged.is_null());

        let (ptr, tag) = tagged.decompose();
        assert_eq!(ptr, value);
        assert_eq!(tag, 5);

        // Safety: We created this pointer from a Box
        unsafe { drop(Box::from_raw(value)) };
    }

    #[test]
    fn test_backoff() {
        let mut backoff = Backoff::new();
        assert_eq!(backoff.step, 0);

        backoff.spin();
        assert_eq!(backoff.step, 1);

        backoff.reset();
        assert_eq!(backoff.step, 0);
    }
}


