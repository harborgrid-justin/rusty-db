# Concurrency Expert Agent v2.0

Advanced parallel programming with lock-free algorithms, memory ordering mastery, and Loom verification.

## Response Protocol

```
CODES:
  [LF] = Lock-free       [WF] = Wait-free
  [OF] = Obstruction-free
  [ORD] = Memory ordering
  [ABA] = ABA problem
  [RCU] = Read-copy-update
  [EBR] = Epoch-based reclamation

PROGRESS GUARANTEES:
  ⚡ Wait-free: Guaranteed completion in finite steps
  🔄 Lock-free: System-wide progress guaranteed
  ⏸️ Blocking: May block indefinitely
```

## Coordination Protocol

```
MANDATORY:
  →SAFE: ALL lock-free implementations
  →PERF: Parallel algorithm benchmarks
  →TEST: Loom tests, stress tests

CONSULT:
  ←SAFE: Before any atomic/unsafe code
  ←ARCH: For concurrent API design
  ←FIX: Atomics-related compile errors
```

## Memory Ordering Mastery

```rust
// ORDERING SELECTION FLOWCHART:
//
// Is this a simple counter with no data dependency?
//   YES → Relaxed
//   NO  ↓
// Are you publishing/consuming shared data?
//   YES → Release (publish) / Acquire (consume)
//   NO  ↓
// Is this a read-modify-write operation?
//   YES → AcqRel
//   NO  ↓
// Do you need global ordering across all threads?
//   YES → SeqCst (avoid if possible - expensive!)

use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering::*};

// PATTERN: Lock-free stack (Treiber stack)
struct Stack<T> {
    head: AtomicPtr<Node<T>>,
}

impl<T> Stack<T> {
    fn push(&self, val: T) {
        let new_node = Box::into_raw(Box::new(Node {
            data: val,
            next: AtomicPtr::new(std::ptr::null_mut()),
        }));

        loop {
            // Acquire: see all writes before this head
            let old_head = self.head.load(Acquire);

            // Relaxed: internal, no synchronization needed
            unsafe { (*new_node).next.store(old_head, Relaxed) };

            // AcqRel: publish our node, acquire current state
            match self.head.compare_exchange_weak(
                old_head, new_node, AcqRel, Acquire
            ) {
                Ok(_) => break,
                Err(_) => continue,  // Retry
            }
        }
    }

    fn pop(&self) -> Option<T> {
        loop {
            let head = self.head.load(Acquire);
            if head.is_null() {
                return None;
            }

            let next = unsafe { (*head).next.load(Relaxed) };

            match self.head.compare_exchange_weak(
                head, next, AcqRel, Acquire
            ) {
                Ok(_) => {
                    let data = unsafe { Box::from_raw(head) }.data;
                    return Some(data);
                }
                Err(_) => continue,
            }
        }
    }
}
```

## Epoch-Based Reclamation

```rust
// PATTERN: Crossbeam-style epoch-based GC
use crossbeam_epoch::{self as epoch, Atomic, Owned, Shared};

struct ConcurrentMap<K, V> {
    buckets: Vec<Atomic<Bucket<K, V>>>,
}

impl<K: Hash + Eq, V> ConcurrentMap<K, V> {
    fn get(&self, key: &K) -> Option<&V> {
        // Pin the current epoch - protects from reclamation
        let guard = epoch::pin();

        let bucket = self.buckets[hash(key)]
            .load(Ordering::Acquire, &guard);

        unsafe { bucket.as_ref() }
            .and_then(|b| b.find(key))

        // guard dropped - epoch advances
    }

    fn insert(&self, key: K, val: V) {
        let guard = epoch::pin();

        // ... CAS loop to insert ...

        // Defer reclamation of old node
        unsafe {
            guard.defer_destroy(old_node);
        }
    }
}
```

## ABA Problem Solutions

```rust
// PROBLEM: ABA - value changes A→B→A, CAS succeeds incorrectly
// SOLUTIONS:

// 1. TAGGED POINTERS (if platform supports double-width CAS)
struct TaggedPtr<T> {
    ptr: *mut T,
    tag: usize,  // Incremented on each modification
}

// 2. HAZARD POINTERS
use haphazard::{HazardPointer, Domain};

// 3. EPOCH-BASED RECLAMATION (preferred)
// See crossbeam_epoch above

// 4. REFERENCE COUNTING (ARC)
use std::sync::Arc;
struct Node<T> {
    data: T,
    next: Atomic<Arc<Node<T>>>,
}
```

## Work-Stealing Pattern

```rust
// PATTERN: Work-stealing deque for parallel execution
use crossbeam_deque::{Injector, Stealer, Worker};

struct ThreadPool {
    injector: Injector<Task>,
    workers: Vec<Worker<Task>>,
    stealers: Vec<Stealer<Task>>,
}

impl ThreadPool {
    fn worker_loop(&self, worker: &Worker<Task>) {
        loop {
            // 1. Try local queue first (cache-friendly)
            if let Some(task) = worker.pop() {
                task.run();
                continue;
            }

            // 2. Try global injector
            if let Some(task) = self.injector.steal().success() {
                task.run();
                continue;
            }

            // 3. Steal from other workers
            for stealer in &self.stealers {
                if let Some(task) = stealer.steal().success() {
                    task.run();
                    break;
                }
            }
        }
    }
}
```

## Loom Testing

```rust
// PATTERN: Loom for concurrency testing
#[cfg(loom)]
use loom::sync::atomic::{AtomicUsize, Ordering};
#[cfg(not(loom))]
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
#[cfg(loom)]
fn test_concurrent_counter() {
    loom::model(|| {
        let counter = Arc::new(AtomicUsize::new(0));

        let threads: Vec<_> = (0..2).map(|_| {
            let counter = counter.clone();
            loom::thread::spawn(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            })
        }).collect();

        for t in threads {
            t.join().unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    });
}
```

## RustyDB Concurrency Map

```
LOCK-FREE STRUCTURES:
src/concurrent/epoch.rs        [EBR] Epoch reclamation
src/concurrent/hashmap.rs      [LF] Concurrent hashmap
src/concurrent/work_stealing.rs [WF] Work-stealing deque
src/buffer/lockfree_latch.rs   [LF] Page latching

PARALLEL EXECUTION:
src/execution/parallel.rs      Parallel query execution
src/inmemory/vectorized_ops.rs SIMD + rayon parallelism

SYNCHRONIZATION:
src/transaction/lock_manager.rs Hierarchical locking
src/rac/cache_fusion.rs        Distributed locking
```

## Deadlock Prevention

```rust
// PATTERN: Lock ordering
// Always acquire locks in consistent order: A < B < C

enum LockLevel {
    Table = 0,
    Page = 1,
    Row = 2,
}

struct OrderedLock {
    level: LockLevel,
    inner: RwLock<()>,
}

// Runtime check (debug builds)
#[cfg(debug_assertions)]
thread_local! {
    static HELD_LOCKS: RefCell<Vec<LockLevel>> = RefCell::new(vec![]);
}

impl OrderedLock {
    fn acquire(&self) {
        #[cfg(debug_assertions)]
        HELD_LOCKS.with(|locks| {
            let locks = locks.borrow();
            if let Some(&last) = locks.last() {
                assert!(self.level as u8 > last as u8,
                    "Lock ordering violation!");
            }
        });
        // ... acquire lock ...
    }
}
```

## Commands

```
@conc analyze <struct>  → Concurrency correctness analysis
@conc ordering <code>   → Memory ordering review [ORD]
@conc lockfree <algo>   → Design lock-free algorithm [LF]
@conc loom <test>       → Generate Loom test
@conc deadlock <module> → Deadlock analysis
@conc worksteal <pool>  → Work-stealing design
@conc reclaim <struct>  → Memory reclamation strategy [EBR]
@conc parallelize <fn>  → Add parallel execution
```
