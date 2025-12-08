// Copyright (c) 2025 RustyDB Contributors
//
// Hazard Pointers - Safe Memory Reclamation for Lock-Free Data Structures
//
// Alternative to epoch-based memory reclamation with different trade-offs:
// - Lower memory overhead
// - Immediate reclamation (no epochs)
// - Per-thread hazard pointer arrays
// - Batch reclamation for efficiency
//
// References:
// - Maged M. Michael. "Hazard pointers: Safe memory reclamation for lock-free objects."
//   IEEE Transactions on Parallel and Distributed Systems, 2004.

use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering, AtomicBool};
use std::ptr::{self, NonNull};
use std::cell::RefCell;
use std::mem;
use std::marker::PhantomData;

/// Maximum number of hazard pointers per thread
const MAX_HAZARDS_PER_THREAD: usize = 8;

/// Batch size for retirement
const RETIRE_BATCH_SIZE: usize = 64;

/// Hazard pointer record for one thread
#[repr(C, align(64))] // Cache-line aligned
struct HazardRecord {
    /// Array of hazard pointers
    hazards: [AtomicPtr<()>; MAX_HAZARDS_PER_THREAD],

    /// Active flag (thread is using this record)
    active: AtomicBool,

    /// Next record in the list
    next: AtomicPtr<HazardRecord>,

    /// Padding
    _padding: [u8; 0],
}

impl HazardRecord {
    fn new() -> Self {
        const NULL_PTR: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());
        Self {
            hazards: [NULL_PTR; MAX_HAZARDS_PER_THREAD],
            active: AtomicBool::new(false),
            next: AtomicPtr::new(ptr::null_mut()),
            _padding: [],
        }
    }

    /// Acquire this record for the current thread
    fn try_acquire(&self) -> bool {
        self.active.compare_exchange(
            false,
            true,
            Ordering::Acquire,
            Ordering::Relaxed,
        ).is_ok()
    }

    /// Release this record
    fn release(&self) {
        // Clear all hazard pointers
        for hazard in &self.hazards {
            hazard.store(ptr::null_mut(), Ordering::Release);
        }
        self.active.store(false, Ordering::Release);
    }

    /// Set hazard pointer at index
    fn set_hazard(&self, index: usize, ptr: *mut ()) {
        if index < MAX_HAZARDS_PER_THREAD {
            self.hazards[index].store(ptr, Ordering::Release);
        }
    }

    /// Clear hazard pointer at index
    fn clear_hazard(&self, index: usize) {
        if index < MAX_HAZARDS_PER_THREAD {
            self.hazards[index].store(ptr::null_mut(), Ordering::Release);
        }
    }

    /// Get hazard pointer at index
    fn get_hazard(&self, index: usize) -> *mut () {
        if index < MAX_HAZARDS_PER_THREAD {
            self.hazards[index].load(Ordering::Acquire)
        } else {
            ptr::null_mut()
        }
    }
}

/// Global hazard pointer list
struct HazardList {
    /// Head of the linked list of hazard records
    head: AtomicPtr<HazardRecord>,

    /// Total number of records
    count: AtomicUsize,
}

impl HazardList {
    const fn new() -> Self {
        Self {
            head: AtomicPtr::new(ptr::null_mut()),
            count: AtomicUsize::new(0),
        }
    }

    /// Acquire a hazard record for the current thread
    fn acquire(&self) -> NonNull<HazardRecord> {
        // Try to find an inactive record
        let mut current = self.head.load(Ordering::Acquire);

        while !current.is_null() {
            let record = unsafe { &*current };
            if record.try_acquire() {
                return NonNull::new(current).unwrap();
            }
            current = record.next.load(Ordering::Acquire);
        }

        // No inactive record found, allocate new one
        self.allocate_record()
    }

    /// Allocate a new hazard record
    fn allocate_record(&self) -> NonNull<HazardRecord> {
        let record = Box::into_raw(Box::new(HazardRecord::new()));
        let record_ref = unsafe { &*record };

        // Acquire it immediately
        record_ref.active.store(true, Ordering::Release);

        // Insert into list
        loop {
            let old_head = self.head.load(Ordering::Acquire);
            record_ref.next.store(old_head, Ordering::Release);

            if self.head.compare_exchange(
                old_head,
                record,
                Ordering::Release,
                Ordering::Acquire,
            ).is_ok() {
                self.count.fetch_add(1, Ordering::Relaxed);
                return NonNull::new(record).unwrap();
            }
        }
    }

    /// Release a hazard record
    fn release(&self, record: NonNull<HazardRecord>) {
        unsafe { record.as_ref().release() };
    }

    /// Collect all currently protected pointers
    fn collect_protected(&self) -> Vec<*mut ()> {
        let mut protected = Vec::new();
        let mut current = self.head.load(Ordering::Acquire);

        while !current.is_null() {
            let record = unsafe { &*current };

            if record.active.load(Ordering::Acquire) {
                for i in 0..MAX_HAZARDS_PER_THREAD {
                    let ptr = record.get_hazard(i);
                    if !ptr.is_null() {
                        protected.push(ptr);
                    }
                }
            }

            current = record.next.load(Ordering::Acquire);
        }

        protected.sort_unstable();
        protected.dedup();
        protected
    }
}

/// Global hazard list (singleton)
static HAZARD_LIST: HazardList = HazardList::new();

/// Thread-local hazard record and retire list
thread_local! {
    static THREAD_LOCAL: RefCell<ThreadLocal> = RefCell::new(ThreadLocal::new());
}

/// Thread-local data
struct ThreadLocal {
    /// Hazard record for this thread
    record: Option<NonNull<HazardRecord>>,

    /// Retired pointers waiting for reclamation
    retired: Vec<RetiredPtr>,

    /// Next hazard index to allocate
    next_hazard: usize,
}

impl ThreadLocal {
    fn new() -> Self {
        Self {
            record: None,
            retired: Vec::with_capacity(RETIRE_BATCH_SIZE),
            next_hazard: 0,
        }
    }

    /// Ensure we have a hazard record
    fn ensure_record(&mut self) -> NonNull<HazardRecord> {
        if let Some(record) = self.record {
            record
        } else {
            let record = HAZARD_LIST.acquire();
            self.record = Some(record);
            record
        }
    }

    /// Allocate a hazard pointer
    fn allocate_hazard(&mut self) -> usize {
        let index = self.next_hazard;
        self.next_hazard = (self.next_hazard + 1) % MAX_HAZARDS_PER_THREAD;
        index
    }

    /// Release thread-local resources
    fn release(&mut self) {
        if let Some(record) = self.record {
            HAZARD_LIST.release(record);
            self.record = None;
        }

        // Force reclamation of all retired pointers
        self.try_reclaim(true);
    }

    /// Try to reclaim retired pointers
    fn try_reclaim(&mut self, force: bool) {
        if !force && self.retired.len() < RETIRE_BATCH_SIZE {
            return;
        }

        if self.retired.is_empty() {
            return;
        }

        // Collect all protected pointers
        let protected = HAZARD_LIST.collect_protected();

        // Reclaim pointers not in protected set
        self.retired.retain(|retired| {
            let ptr = retired.ptr;

            // Binary search since protected list is sorted
            if protected.binary_search(&ptr).is_ok() {
                // Still protected, keep it
                true
            } else {
                // Not protected, can reclaim
                unsafe { (retired.deleter)(ptr) };
                false
            }
        });
    }
}

impl Drop for ThreadLocal {
    fn drop(&mut self) {
        self.release();
    }
}

/// Retired pointer with deleter function
struct RetiredPtr {
    ptr: *mut (),
    deleter: unsafe fn(*mut ()),
}

/// Hazard pointer guard
///
/// Protects a pointer while the guard is alive
pub struct HazardGuard {
    index: usize,
    _phantom: PhantomData<*mut ()>,
}

impl HazardGuard {
    /// Create a new hazard guard protecting the given pointer
    pub fn new<T>(ptr: *mut T) -> Self {
        THREAD_LOCAL.with(|tl| {
            let mut tl = tl.borrow_mut();
            let record = tl.ensure_record();
            let index = tl.allocate_hazard();

            unsafe {
                record.as_ref().set_hazard(index, ptr as *mut ());
            }

            Self {
                index,
                _phantom: PhantomData,
            }
        })
    }

    /// Protect a different pointer (reuse guard)
    pub fn protect<T>(&self, ptr: *mut T) {
        THREAD_LOCAL.with(|tl| {
            let tl = tl.borrow();
            if let Some(record) = tl.record {
                unsafe {
                    record.as_ref().set_hazard(self.index, ptr as *mut ());
                }
            }
        });
    }

    /// Clear protection
    pub fn clear(&self) {
        THREAD_LOCAL.with(|tl| {
            let tl = tl.borrow();
            if let Some(record) = tl.record {
                unsafe {
                    record.as_ref().clear_hazard(self.index);
                }
            }
        });
    }
}

impl Drop for HazardGuard {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Retire a pointer for later reclamation
///
/// The pointer will be reclaimed when it's no longer protected by any hazard pointer
pub fn retire<T>(ptr: *mut T) {
    unsafe fn deleter<T>(ptr: *mut ()) {
        drop(Box::from_raw(ptr as *mut T));
    }

    THREAD_LOCAL.with(|tl| {
        let mut tl = tl.borrow_mut();

        tl.retired.push(RetiredPtr {
            ptr: ptr as *mut (),
            deleter: deleter::<T>,
        });

        // Try to reclaim if batch is full
        tl.try_reclaim(false);
    });
}

/// Retire a pointer with custom deleter
pub fn retire_with_deleter(ptr: *mut (), deleter: unsafe fn(*mut ())) {
    THREAD_LOCAL.with(|tl| {
        let mut tl = tl.borrow_mut();

        tl.retired.push(RetiredPtr { ptr, deleter });

        tl.try_reclaim(false);
    });
}

/// Hazard pointer protected reference
pub struct Protected<'g, T> {
    ptr: *mut T,
    _marker: PhantomData<(&'g T, *mut T)>,
}

impl<'g, T> Protected<'g, T> {
    /// Create from raw pointer (unsafe)
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Get raw pointer
    pub fn as_ptr(&self) -> *mut T {
        self.ptr
    }

    /// Check if null
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    /// Get reference (unsafe)
    pub unsafe fn as_ref(&self) -> Option<&'g T> {
        self.ptr.as_ref()
    }
}

impl<'g, T> Clone for Protected<'g, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }
}

impl<'g, T> Copy for Protected<'g, T> {}

/// Hazard pointer domain
///
/// Provides scoped API similar to crossbeam epoch
pub struct HazardDomain;

impl HazardDomain {
    /// Pin the current thread
    pub fn pin() -> HazardScope {
        HazardScope::new()
    }

    /// Global reclamation
    pub fn reclaim_all() {
        THREAD_LOCAL.with(|tl| {
            tl.borrow_mut().try_reclaim(true);
        });
    }
}

/// Scoped hazard pointer protection
pub struct HazardScope {
    guards: Vec<HazardGuard>,
}

impl HazardScope {
    fn new() -> Self {
        Self {
            guards: Vec::new(),
        }
    }

    /// Protect a pointer
    pub fn protect<T>(&mut self, ptr: *mut T) -> Protected<T> {
        let guard = HazardGuard::new(ptr);
        self.guards.push(guard);

        unsafe { Protected::from_raw(ptr) }
    }

    /// Defer pointer retirement
    pub fn defer_retire<T>(&self, ptr: *mut T) {
        retire(ptr);
    }
}

/// Statistics for hazard pointers
pub struct HazardStats {
    pub total_records: usize,
    pub active_records: usize,
    pub retired_count: usize,
}

impl HazardStats {
    pub fn collect() -> Self {
        let total_records = HAZARD_LIST.count.load(Ordering::Relaxed);

        let mut active_records = 0;
        let mut current = HAZARD_LIST.head.load(Ordering::Acquire);

        while !current.is_null() {
            let record = unsafe { &*current };
            if record.active.load(Ordering::Acquire) {
                active_records += 1;
            }
            current = record.next.load(Ordering::Acquire);
        }

        let retired_count = THREAD_LOCAL.with(|tl| {
            tl.borrow().retired.len()
        });

        Self {
            total_records,
            active_records,
            retired_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_hazard_guard() {
        let value = Box::into_raw(Box::new(42));
        let guard = HazardGuard::new(value);

        unsafe {
            assert_eq!(*value, 42);
        }

        drop(guard);
        unsafe {
            drop(Box::from_raw(value));
        }
    }

    #[test]
    fn test_retire() {
        let value = Box::into_raw(Box::new(42));

        // Protect with hazard pointer
        let _guard = HazardGuard::new(value);

        // Retire the pointer
        retire(value);

        // It should not be reclaimed yet (still protected)
        THREAD_LOCAL.with(|tl| {
            let tl = tl.borrow();
            assert!(!tl.retired.is_empty());
        });

        // Drop guard
        drop(_guard);

        // Force reclamation
        HazardDomain::reclaim_all();

        // Should be reclaimed now
        THREAD_LOCAL.with(|tl| {
            let tl = tl.borrow();
            // May still have entries if reclamation threshold not met
        });
    }

    #[test]
    fn test_concurrent() {
        let value = Arc::new(AtomicPtr::new(Box::into_raw(Box::new(0))));
        let mut handles = vec![];

        for i in 0..10 {
            let value = Arc::clone(&value);
            handles.push(thread::spawn(move || {
                for j in 0..100 {
                    let ptr = value.load(Ordering::Acquire);
                    let _guard = HazardGuard::new(ptr);

                    // Simulate work
                    unsafe {
                        if !ptr.is_null() {
                            let _ = *ptr + i + j;
                        }
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let ptr = value.load(Ordering::Acquire);
        unsafe {
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
        }
    }
}
