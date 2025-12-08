// Copyright (c) 2025 RustyDB Contributors
//
// Epoch-based memory reclamation for lock-free data structures
//
// This module implements an epoch-based garbage collection scheme similar to
// the one described in "Fast and Portable Concurrent FIFO Queues with Timeout"
// by Hoffman et al. It allows safe reclamation of memory in lock-free structures
// by tracking which threads are accessing which epoch.

use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::mem::{self, ManuallyDrop};
use std::ops::{Deref, DerefMut};
use std::ptr;
use std::sync::atomic::{fence, AtomicPtr, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Number of epochs to track (3 provides enough lag for reclamation)
const EPOCH_COUNT: usize = 3;

/// Batch size for garbage collection
const GC_BATCH_SIZE: usize = 64;

/// Global epoch counter
static GLOBAL_EPOCH: AtomicU64 = AtomicU64::new(0);

/// Thread-local participant in epoch-based reclamation
thread_local! {
    static LOCAL_EPOCH: Cell<u64> = const { Cell::new(0) };
    static GARBAGE_BAGS: RefCell<[Vec<Garbage>; EPOCH_COUNT]> = RefCell::new([
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ]);
    static IS_PINNED: Cell<bool> = const { Cell::new(false) };
    static PARTICIPANT: RefCell<Option<Arc<Participant>>> = const { RefCell::new(None) };
}

/// Global list of all participants
static PARTICIPANTS: Mutex<Vec<Arc<Participant>>> = Mutex::new(Vec::new());

/// Represents a piece of garbage to be reclaimed
struct Garbage {
    ptr: *mut u8,
    deleter: unsafe fn(*mut u8),
}

impl Garbage {
    /// Create new garbage
    fn new<T>(ptr: *mut T) -> Self {
        unsafe fn deleter<T>(ptr: *mut u8) {
            drop(Box::from_raw(ptr as *mut T));
        }

        Self {
            ptr: ptr as *mut u8,
            deleter: deleter::<T>,
        }
    }

    /// Reclaim this garbage
    unsafe fn reclaim(self) {
        (self.deleter)(self.ptr);
    }
}

/// A participant in epoch-based reclamation
#[repr(C, align(64))]
pub struct Participant {
    /// Current epoch this participant is in (0 means not active)
    epoch: AtomicU64,
    /// Number of times this participant has been pinned
    pin_count: AtomicUsize,
    /// Padding to prevent false sharing
    _padding: [u8; 48],
}

impl Participant {
    /// Create a new participant
    fn new() -> Self {
        Self {
            epoch: AtomicU64::new(0),
            pin_count: AtomicUsize::new(0),
            _padding: [0; 48],
        }
    }

    /// Enter an epoch
    fn enter(&self) -> u64 {
        let count = self.pin_count.fetch_add(1, Ordering::Relaxed);
        if count == 0 {
            let global = GLOBAL_EPOCH.load(Ordering::Relaxed);
            self.epoch.store(global, Ordering::Release);
            fence(Ordering::SeqCst);
        }
        self.epoch.load(Ordering::Relaxed)
    }

    /// Leave an epoch
    fn leave(&self) {
        let count = self.pin_count.fetch_sub(1, Ordering::Relaxed);
        if count == 1 {
            self.epoch.store(0, Ordering::Release);
        }
    }

    /// Check if this participant is active
    fn is_active(&self) -> bool {
        self.epoch.load(Ordering::Acquire) != 0
    }

    /// Get current epoch
    fn current_epoch(&self) -> u64 {
        self.epoch.load(Ordering::Acquire)
    }
}

/// The global epoch-based reclamation system
pub struct Epoch;

impl Epoch {
    /// Pin the current thread to the current epoch
    pub fn pin() -> EpochGuard {
        IS_PINNED.with(|is_pinned| {
            if is_pinned.get() {
                // Already pinned, just increment reference count
                PARTICIPANT.with(|p| {
                    if let Some(participant) = p.borrow().as_ref() {
                        participant.enter();
                    }
                });
            } else {
                // First pin - register participant if needed
                PARTICIPANT.with(|p| {
                    let mut p_ref = p.borrow_mut();
                    if p_ref.is_none() {
                        let participant = Arc::new(Participant::new());
                        PARTICIPANTS.lock().unwrap().push(participant.clone());
                        *p_ref = Some(participant);
                    }
                    if let Some(participant) = p_ref.as_ref() {
                        let epoch = participant.enter();
                        LOCAL_EPOCH.with(|e| e.set(epoch));
                        is_pinned.set(true);
                    }
                });
            }
        });

        EpochGuard {
            _marker: PhantomData,
        }
    }

    /// Try to advance the global epoch
    pub fn try_advance() -> bool {
        let global = GLOBAL_EPOCH.load(Ordering::Relaxed);

        // Check if all participants are in the current epoch or inactive
        let participants = PARTICIPANTS.lock().unwrap();
        let min_epoch = participants
            .iter()
            .filter(|p| p.is_active())
            .map(|p| p.current_epoch())
            .min()
            .unwrap_or(global);

        if min_epoch == global {
            // All active participants have caught up, advance the epoch
            GLOBAL_EPOCH.compare_exchange(
                global,
                global + 1,
                Ordering::Release,
                Ordering::Relaxed,
            ).is_ok()
        } else {
            false
        }
    }

    /// Defer garbage collection of a pointer
    pub fn defer<T>(ptr: *mut T) {
        LOCAL_EPOCH.with(|epoch| {
            let epoch_idx = (epoch.get() % EPOCH_COUNT as u64) as usize;
            GARBAGE_BAGS.with(|bags| {
                bags.borrow_mut()[epoch_idx].push(Garbage::new(ptr));
            });
        });

        // Periodically try to collect garbage
        if LOCAL_EPOCH.with(|e| e.get()) % 100 == 0 {
            Self::try_collect();
        }
    }

    /// Try to collect garbage
    pub fn try_collect() {
        let global = GLOBAL_EPOCH.load(Ordering::Acquire);

        // We can safely reclaim garbage from 2 epochs ago
        if global >= 2 {
            let safe_epoch = global - 2;
            let safe_idx = (safe_epoch % EPOCH_COUNT as u64) as usize;

            GARBAGE_BAGS.with(|bags| {
                let mut bags = bags.borrow_mut();
                let bag = &mut bags[safe_idx];

                // Reclaim garbage in batches
                while !bag.is_empty() {
                    let batch_size = bag.len().min(GC_BATCH_SIZE);
                    for garbage in bag.drain(..batch_size) {
                        // Safety: We're at least 2 epochs ahead, so no thread can be
                        // accessing this memory
                        unsafe {
                            garbage.reclaim();
                        }
                    }
                }
            });
        }

        // Try to advance the global epoch
        Self::try_advance();
    }

    /// Force garbage collection (for testing)
    #[cfg(test)]
    pub fn force_collect() {
        for _ in 0..10 {
            Self::try_advance();
            std::thread::yield_now();
        }
        Self::try_collect();
    }
}

/// Guard representing a pinned epoch
pub struct EpochGuard {
    _marker: PhantomData<*mut ()>,
}

impl Drop for EpochGuard {
    fn drop(&mut self) {
        PARTICIPANT.with(|p| {
            if let Some(participant) = p.borrow().as_ref() {
                participant.leave();
                if participant.pin_count.load(Ordering::Relaxed) == 0 {
                    IS_PINNED.with(|is_pinned| is_pinned.set(false));
                }
            }
        });
    }
}

// Prevent Send/Sync - guards are thread-local
// Use PhantomData to make EpochGuard not Send/Sync
// Note: EpochGuard contains a raw pointer which is already !Send + !Sync

/// An atomic pointer with epoch-based reclamation
pub struct Atomic<T> {
    ptr: AtomicPtr<T>,
    _marker: PhantomData<T>,
}

impl<T> Atomic<T> {
    /// Create a new atomic pointer
    pub fn null() -> Self {
        Self {
            ptr: AtomicPtr::new(ptr::null_mut()),
            _marker: PhantomData,
        }
    }

    /// Create a new atomic pointer from an owned value
    pub fn new(value: T) -> Self {
        Self {
            ptr: AtomicPtr::new(Box::into_raw(Box::new(value))),
            _marker: PhantomData,
        }
    }

    /// Load the pointer
    pub fn load<'g>(&self, ord: Ordering, _guard: &'g EpochGuard) -> Shared<'g, T> {
        Shared {
            ptr: self.ptr.load(ord),
            _marker: PhantomData,
        }
    }

    /// Store a pointer
    pub fn store(&self, ptr: Shared<T>, ord: Ordering) {
        self.ptr.store(ptr.ptr, ord);
    }

    /// Compare and swap
    pub fn compare_exchange<'g>(
        &self,
        current: Shared<T>,
        new: Shared<T>,
        success: Ordering,
        failure: Ordering,
        _guard: &'g EpochGuard,
    ) -> Result<Shared<'g, T>, Shared<'g, T>> {
        match self.ptr.compare_exchange(
            current.ptr,
            new.ptr,
            success,
            failure,
        ) {
            Ok(ptr) => Ok(Shared {
                ptr,
                _marker: PhantomData,
            }),
            Err(ptr) => Err(Shared {
                ptr,
                _marker: PhantomData,
            }),
        }
    }

    /// Compare and swap (weak version)
    pub fn compare_exchange_weak<'g>(
        &self,
        current: Shared<T>,
        new: Shared<T>,
        success: Ordering,
        failure: Ordering,
        _guard: &'g EpochGuard,
    ) -> Result<Shared<'g, T>, Shared<'g, T>> {
        match self.ptr.compare_exchange_weak(
            current.ptr,
            new.ptr,
            success,
            failure,
        ) {
            Ok(ptr) => Ok(Shared {
                ptr,
                _marker: PhantomData,
            }),
            Err(ptr) => Err(Shared {
                ptr,
                _marker: PhantomData,
            }),
        }
    }

    /// Swap the pointer
    pub fn swap<'g>(&self, new: Shared<T>, ord: Ordering, _guard: &'g EpochGuard) -> Shared<'g, T> {
        Shared {
            ptr: self.ptr.swap(new.ptr, ord),
            _marker: PhantomData,
        }
    }
}

impl<T> Default for Atomic<T> {
    fn default() -> Self {
        Self::null()
    }
}

impl<'g, T> From<Shared<'g, T>> for Atomic<T> {
    fn from(shared: Shared<'g, T>) -> Self {
        Self {
            ptr: AtomicPtr::new(shared.ptr),
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for Atomic<T> {
    fn drop(&mut self) {
        let ptr = self.ptr.load(Ordering::Relaxed);
        if !ptr.is_null() {
            // Safety: We have exclusive access during drop
            unsafe {
                drop(Box::from_raw(ptr));
            }
        }
    }
}

// Safety: Atomic operations are thread-safe
unsafe impl<T: Send> Send for Atomic<T> {}
unsafe impl<T: Send> Sync for Atomic<T> {}

/// An owned pointer (uniquely owned)
pub struct Owned<T> {
    ptr: *mut T,
    _marker: PhantomData<T>,
}

impl<T> Owned<T> {
    /// Create a new owned pointer
    pub fn new(value: T) -> Self {
        Self {
            ptr: Box::into_raw(Box::new(value)),
            _marker: PhantomData,
        }
    }

    /// Create a null owned pointer
    pub fn null() -> Self {
        Self {
            ptr: ptr::null_mut(),
            _marker: PhantomData,
        }
    }

    /// Convert to a shared pointer
    pub fn into_shared<'g>(self) -> Shared<'g, T> {
        let ptr = self.ptr;
        mem::forget(self);
        Shared {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut T {
        self.ptr
    }
}

impl<T> Drop for Owned<T> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // Safety: We own this pointer
            unsafe {
                drop(Box::from_raw(self.ptr));
            }
        }
    }
}

impl<T> Deref for Owned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        // Safety: Owned guarantees the pointer is valid and uniquely owned
        unsafe { &*self.ptr }
    }
}

impl<T> DerefMut for Owned<T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safety: Owned guarantees the pointer is valid and uniquely owned
        unsafe { &mut *self.ptr }
    }
}

/// A shared pointer (may be accessed by multiple threads)
#[derive(Debug)]
pub struct Shared<'g, T> {
    ptr: *mut T,
    _marker: PhantomData<&'g T>,
}

impl<'g, T> Shared<'g, T> {
    /// Create a null shared pointer
    pub fn null() -> Self {
        Self {
            ptr: ptr::null_mut(),
            _marker: PhantomData,
        }
    }

    /// Check if the pointer is null
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut T {
        self.ptr
    }

    /// Dereference the pointer
    pub fn as_ref(&self) -> Option<&'g T> {
        if self.ptr.is_null() {
            None
        } else {
            // Safety: Protected by epoch guard, pointer is valid for 'g
            Some(unsafe { &*self.ptr })
        }
    }

    /// Convert to an owned pointer (takes ownership)
    pub fn into_owned(self) -> Owned<T> {
        Owned {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }

    /// Create a shared pointer from a raw pointer
    pub fn from_raw(ptr: *mut T) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }
}

impl<'g, T> Clone for Shared<'g, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }
}

impl<'g, T> Copy for Shared<'g, T> {}

impl<'g, T> PartialEq for Shared<'g, T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<'g, T> Eq for Shared<'g, T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_epoch_pin() {
        let guard1 = Epoch::pin();
        let guard2 = Epoch::pin();
        drop(guard1);
        drop(guard2);
    }

    #[test]
    fn test_atomic_operations() {
        let atomic = Atomic::new(42);
        let guard = Epoch::pin();

        let shared = atomic.load(Ordering::SeqCst, &guard);
        assert_eq!(*shared.as_ref().unwrap(), 42);

        let new_value = Owned::new(100);
        atomic.store(new_value.into_shared(), Ordering::SeqCst);

        let updated = atomic.load(Ordering::SeqCst, &guard);
        assert_eq!(*updated.as_ref().unwrap(), 100);
    }

    #[test]
    fn test_compare_exchange() {
        let atomic = Atomic::new(42);
        let guard = Epoch::pin();

        let current = atomic.load(Ordering::SeqCst, &guard);
        let new_value = Owned::new(100).into_shared();

        let result = atomic.compare_exchange(
            current,
            new_value,
            Ordering::SeqCst,
            Ordering::SeqCst,
            &guard,
        );

        assert!(result.is_ok());
        let updated = atomic.load(Ordering::SeqCst, &guard);
        assert_eq!(*updated.as_ref().unwrap(), 100);
    }

    #[test]
    fn test_concurrent_epochs() {
        let atomic = Arc::new(Atomic::new(0));
        let mut handles = vec![];

        for i in 0..10 {
            let atomic = atomic.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let guard = Epoch::pin();
                    let current = atomic.load(Ordering::Acquire, &guard);
                    let new_val = current.as_ref().map(|v| *v + 1).unwrap_or(i);
                    let _ = atomic.compare_exchange(
                        current,
                        Owned::new(new_val).into_shared(),
                        Ordering::Release,
                        Ordering::Acquire,
                        &guard,
                    );
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_garbage_collection() {
        static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

        struct DropCounter;
        impl Drop for DropCounter {
            fn drop(&mut self) {
                DROP_COUNT.fetch_add(1, Ordering::SeqCst);
            }
        }

        let count_before = DROP_COUNT.load(Ordering::SeqCst);

        {
            let guard = Epoch::pin();
            let ptr = Box::into_raw(Box::new(DropCounter));
            Epoch::defer(ptr);
        }

        // Force collection
        Epoch::force_collect();

        let count_after = DROP_COUNT.load(Ordering::SeqCst);
        assert!(count_after > count_before);
    }
}


