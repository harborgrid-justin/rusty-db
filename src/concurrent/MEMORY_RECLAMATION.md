# Memory Reclamation Strategy in RustyDB

## Overview

RustyDB implements two safe memory reclamation techniques for lock-free data structures:

1. **Epoch-Based Reclamation** (`epoch.rs`)
2. **Hazard Pointers** (`hazard.rs`)

Both techniques solve the **ABA problem** and prevent **use-after-free** in concurrent data structures, but with different trade-offs.

---

## When to Use Each Technique

### Use Epoch-Based Reclamation When:

✅ **Multiple concurrent reads per data structure**
- Example: Lock-free skip list, lock-free queue, lock-free hash table
- Reader threads frequently traverse multiple nodes

✅ **High throughput, read-heavy workloads**
- Epoch-based has lower per-operation overhead
- No need to acquire/release hazard pointers for each node

✅ **Batch operations**
- Reading or modifying multiple entries at once
- Single epoch pin protects entire traversal

✅ **Acceptable memory overhead**
- Can tolerate delayed reclamation (garbage accumulates until epoch advances)
- Memory freed in batches when epochs advance

**Example Use Cases in RustyDB:**
```rust
// Lock-free queue for transaction log
use crate::concurrent::epoch::{Epoch, Atomic};

let guard = Epoch::pin();
let head = queue.head.load(Ordering::Acquire, &guard);
// Traverse multiple nodes under single pin
while let Some(node) = head.as_ref() {
    process(node);
    head = node.next.load(Ordering::Acquire, &guard);
}
// guard drop triggers epoch leave
```

---

### Use Hazard Pointers When:

✅ **Low memory overhead required**
- Immediate reclamation when no hazards protect the pointer
- No accumulation of garbage

✅ **Bounded memory usage critical**
- Real-time systems
- Embedded environments
- Strict memory quotas

✅ **Few concurrent accesses per structure**
- Protecting individual nodes/objects
- Short-lived critical sections

✅ **Predictable latency required**
- Hazard pointer reclamation happens deterministically
- No waiting for global epoch to advance

**Example Use Cases in RustyDB:**
```rust
// Lock-free stack with bounded memory
use crate::concurrent::hazard::{HazardGuard, retire};

let ptr = stack.head.load(Ordering::Acquire);
let guard = HazardGuard::new(ptr);
// Only this specific pointer is protected
unsafe {
    if !ptr.is_null() {
        process(*ptr);
    }
}
// Drop guard, retire immediately if no other hazards
```

---

## Detailed Comparison

| Aspect | Epoch-Based | Hazard Pointers |
|--------|-------------|-----------------|
| **Memory Overhead** | Higher (garbage accumulates) | Lower (immediate reclamation) |
| **Per-operation Cost** | Lower (single pin for traversal) | Higher (guard per pointer) |
| **Reclamation Timing** | Delayed (batch at epoch advance) | Immediate (when no hazards) |
| **Worst-case Memory** | Unbounded (if epochs don't advance) | Bounded (R × H pointers) |
| **Cache Performance** | Better (fewer atomic ops) | Worse (more atomic ops) |
| **Complexity** | Simpler API | More complex API |
| **Suitable For** | Read-heavy, batch operations | Write-heavy, memory-constrained |

**Where:**
- R = number of threads
- H = max hazard pointers per thread

---

## Implementation Details

### Epoch-Based Reclamation (epoch.rs)

**Key Components:**
```rust
static GLOBAL_EPOCH: AtomicU64;           // Global epoch counter
thread_local! {
    static LOCAL_EPOCH: Cell<u64>;        // Thread's current epoch
    static GARBAGE_BAGS: [Vec<Garbage>];  // Per-epoch garbage
}
```

**Algorithm:**
1. `Epoch::pin()` - Thread enters current epoch
2. Access protected data structures
3. `Epoch::defer(ptr)` - Queue garbage for reclamation
4. `EpochGuard` drop - Thread leaves epoch
5. Periodically advance epoch when all threads have caught up
6. Reclaim garbage from (epoch - 2) bags

**Safety Guarantee:**
- Data from epoch N is safe to reclaim at epoch N+2
- Ensures all threads have exited epoch N before reclaiming

---

### Hazard Pointers (hazard.rs)

**Key Components:**
```rust
static HAZARD_LIST: HazardList;          // Global hazard record list
thread_local! {
    static THREAD_LOCAL: ThreadLocal;     // Per-thread hazards & retired list
}
```

**Algorithm:**
1. `HazardGuard::new(ptr)` - Protect pointer in hazard array
2. Access data through protected pointer
3. `HazardGuard` drop - Clear hazard slot
4. `retire(ptr)` - Add to retired list
5. Periodically scan all hazard arrays
6. Reclaim pointers not in any hazard array

**Safety Guarantee:**
- Pointer is reclaimed only when not in any thread's hazard array
- Binary search on sorted hazard list for O(log n) check

---

## Performance Characteristics

### Epoch-Based:
```
Pin:     O(1)     - Enter epoch, increment counter
Unpin:   O(1)     - Leave epoch, decrement counter
Defer:   O(1)     - Add to garbage bag
Collect: O(G)     - Reclaim G garbage items in batch
```

### Hazard Pointers:
```
Protect:   O(1)   - Set hazard pointer
Unprotect: O(1)   - Clear hazard pointer
Retire:    O(1)   - Add to retired list
Collect:   O(R×H + M log(R×H)) - Scan hazards, binary search for M retired
```

---

## Usage Guidelines

### Decision Tree

```
Start
  │
  ├─ Memory constrained? ──YES──> Hazard Pointers
  │  (Real-time, embedded)
  │
  ├─ Read-heavy workload? ──YES──> Epoch-Based
  │  (Multiple reads per pin)
  │
  ├─ Single-object protection? ──YES──> Hazard Pointers
  │  (Point operations)
  │
  ├─ Batch operations? ──YES──> Epoch-Based
  │  (Range scans, multi-step)
  │
  └─ DEFAULT ──────────────────> Epoch-Based
     (Simpler API, better cache performance)
```

### Code Examples

#### Example 1: Lock-Free Queue (Epoch-Based)
```rust
use crate::concurrent::epoch::{Epoch, Atomic, Owned};

pub struct Queue<T> {
    head: Atomic<Node<T>>,
    tail: Atomic<Node<T>>,
}

impl<T> Queue<T> {
    pub fn push(&self, value: T) {
        let guard = Epoch::pin();
        let node = Owned::new(Node::new(value));

        loop {
            let tail = self.tail.load(Ordering::Acquire, &guard);
            // Multiple loads under single pin
            let next = tail.as_ref().unwrap().next.load(Ordering::Acquire, &guard);

            if next.is_null() {
                if tail.as_ref().unwrap().next
                    .compare_exchange(next, node.into_shared(), ..., &guard)
                    .is_ok()
                {
                    break;
                }
            }
        }
    }
}
```

#### Example 2: Lock-Free Stack (Hazard Pointers)
```rust
use crate::concurrent::hazard::{HazardGuard, retire};

pub struct Stack<T> {
    head: AtomicPtr<Node<T>>,
}

impl<T> Stack<T> {
    pub fn pop(&self) -> Option<T> {
        loop {
            let head_ptr = self.head.load(Ordering::Acquire);
            if head_ptr.is_null() {
                return None;
            }

            // Protect before dereferencing
            let guard = HazardGuard::new(head_ptr);

            // Re-check after protection (ABA prevention)
            if self.head.load(Ordering::Acquire) != head_ptr {
                continue;
            }

            let head = unsafe { &*head_ptr };
            let next = head.next.load(Ordering::Relaxed);

            if self.head.compare_exchange(head_ptr, next, ...).is_ok() {
                retire(head_ptr);  // Immediate retirement
                return Some(head.value.clone());
            }
        }
    }
}
```

---

## Current Usage in RustyDB

### Epoch-Based Reclamation Used In:
- `concurrent/queue.rs` - Lock-free FIFO queue
- `concurrent/stack.rs` - Lock-free LIFO stack
- `concurrent/hashmap.rs` - Lock-free hash table
- `concurrent/skiplist.rs` - Lock-free skip list

**Rationale:** These structures involve traversing multiple nodes, benefit from single pin, and operate in read-heavy workloads.

### Hazard Pointers Used In:
- `concurrent/deque.rs` - Work-stealing deque (memory-bounded)
- Future: Real-time components with strict memory limits

**Rationale:** Work-stealing requires bounded memory and individual node protection.

---

## Migration Guide

### Switching from Epoch to Hazard Pointers:

**Before (Epoch):**
```rust
let guard = Epoch::pin();
let ptr = atomic.load(Ordering::Acquire, &guard);
let data = ptr.as_ref().unwrap();
// Use data
Epoch::defer(old_ptr);
```

**After (Hazard):**
```rust
let ptr = atomic.load(Ordering::Acquire);
let guard = HazardGuard::new(ptr);
// Re-validate after protection
if atomic.load(Ordering::Acquire) == ptr {
    let data = unsafe { &*ptr };
    // Use data
}
retire(old_ptr);
```

---

## Testing & Debugging

### Epoch-Based:
```rust
#[test]
fn test_epoch_reclamation() {
    let guard = Epoch::pin();
    // ... operations ...
    drop(guard);

    // Force garbage collection for testing
    #[cfg(test)]
    Epoch::force_collect();
}
```

### Hazard Pointers:
```rust
#[test]
fn test_hazard_reclamation() {
    let ptr = Box::into_raw(Box::new(42));
    let guard = HazardGuard::new(ptr);
    retire(ptr);

    // Force reclamation
    HazardDomain::reclaim_all();

    assert!(!is_leaked(ptr));
}
```

---

## References

### Academic Papers:
1. **Epoch-Based:**
   - Fraser, K. (2004). "Practical lock-freedom"
   - Hart et al. (2007). "Performance of memory reclamation for lockless synchronization"

2. **Hazard Pointers:**
   - Michael, M. M. (2004). "Hazard pointers: Safe memory reclamation for lock-free objects"

### Implementation References:
- `crossbeam-epoch` - Rust epoch-based reclamation library
- `folly::hazptr` - Facebook's hazard pointer implementation

---

## Future Work

- [ ] Hybrid approach: Epoch-based for reads, hazard for writes
- [ ] Automatic selection based on workload profiling
- [ ] QSBR (Quiescent State Based Reclamation) for read-dominated workloads
- [ ] Reference counting with deferred reclamation for complex ownership

---

**Last Updated:** 2025-12-16
**Maintained By:** RustyDB Concurrency Team
