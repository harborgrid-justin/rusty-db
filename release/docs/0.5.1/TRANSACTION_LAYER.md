# RustyDB v0.5.1 Transaction Layer Documentation

**Enterprise Production Documentation**
**Version:** 0.5.1
**Last Updated:** December 25, 2025
**Classification:** Production-Ready
**Investment Value:** $350 Million Database Server

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#architecture-overview)
3. [ACID Guarantees](#acid-guarantees)
4. [Transaction Lifecycle](#transaction-lifecycle)
5. [Multi-Version Concurrency Control (MVCC)](#multi-version-concurrency-control-mvcc)
6. [Isolation Levels](#isolation-levels)
7. [Lock Management and Two-Phase Locking](#lock-management-and-two-phase-locking)
8. [Deadlock Detection and Resolution](#deadlock-detection-and-resolution)
9. [Write-Ahead Logging (WAL)](#write-ahead-logging-wal)
10. [ARIES Recovery Algorithm](#aries-recovery-algorithm)
11. [Snapshot Isolation](#snapshot-isolation)
12. [Transaction ID Management](#transaction-id-management)
13. [Performance Tuning](#performance-tuning)
14. [Monitoring and Statistics](#monitoring-and-statistics)
15. [Error Handling](#error-handling)
16. [API Reference](#api-reference)

---

## Executive Summary

RustyDB v0.5.1 implements an enterprise-grade transaction management system providing full ACID compliance with multiple concurrency control strategies. The transaction layer is built on proven algorithms (ARIES recovery, MVCC, 2PL) and optimized for high-throughput, low-latency workloads.

### Key Features

- **Full ACID Compliance**: Atomicity, Consistency, Isolation, Durability
- **Multiple Isolation Levels**: READ UNCOMMITTED, READ COMMITTED, REPEATABLE READ, SERIALIZABLE, SNAPSHOT ISOLATION
- **MVCC**: Non-blocking reads with hybrid logical clocks for distributed systems
- **Two-Phase Locking (2PL)**: Lock-based concurrency control with intent locks
- **ARIES Recovery**: Industry-standard crash recovery with fuzzy checkpointing
- **Optimistic Concurrency Control (OCC)**: For low-contention workloads
- **Deadlock Detection**: Graph-based cycle detection with configurable victim selection
- **Distributed Transactions**: Two-Phase Commit (2PC) protocol support

### Module Organization

```
src/transaction/
├── mod.rs                    # Module exports and integration tests
├── types.rs                  # Core types (Transaction, IsolationLevel, LockMode)
├── error.rs                  # Comprehensive error handling
├── manager.rs                # Transaction lifecycle management
├── lock_manager.rs           # 2PL lock acquisition and release
├── deadlock.rs              # Deadlock detection and resolution
├── mvcc.rs                  # Multi-version concurrency control
├── snapshot.rs              # Snapshot isolation management
├── wal_manager.rs           # Write-ahead log (legacy)
├── wal.rs                   # Advanced WAL with group commit
├── recovery.rs              # ARIES-style recovery
├── recovery_manager.rs      # Recovery coordination
├── version_store.rs         # MVCC version storage
├── occ_manager.rs           # Optimistic concurrency control
├── two_phase_commit.rs      # Distributed transaction coordination
├── statistics.rs            # Performance metrics
├── timeout.rs               # Transaction timeout management
└── traits.rs                # Extensibility interfaces
```

---

## Architecture Overview

### Component Hierarchy

```
┌────────────────────────────────────────────────────────────────┐
│                      TransactionManager                         │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐  │
│  │ LockManager  │  │ WALManager   │  │ DeadlockDetector    │  │
│  │  - 2PL       │  │  - ARIES     │  │  - Cycle Detection  │  │
│  │  - Intent    │  │  - Group     │  │  - Victim Selection │  │
│  │    Locks     │  │    Commit    │  │  - Resolution       │  │
│  └──────────────┘  └──────────────┘  └─────────────────────┘  │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐  │
│  │VersionStore  │  │ Snapshot     │  │ RecoveryManager     │  │
│  │  - MVCC      │  │  Manager     │  │  - Analysis Phase   │  │
│  │  - GC        │  │  - SI        │  │  - Redo Phase       │  │
│  │  - HLC       │  │  - Visibility│  │  - Undo Phase       │  │
│  └──────────────┘  └──────────────┘  └─────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
┌─────────────┐
│  BEGIN TXN  │
└──────┬──────┘
       │
       ▼
┌─────────────────┐
│ TransactionMgr  │──────► Allocate Transaction ID
│   .begin()      │──────► Create Transaction Metadata
└────────┬────────┘──────► Initialize Read/Write Sets
         │
         ▼
┌─────────────────┐
│  Execute SQL    │
│  Operations     │
└────────┬────────┘
         │
         ▼
    ┌────────┐
    │ Read?  │──Yes──► ┌──────────────┐
    └───┬────┘         │ MVCC Read    │
        │              │  - Get       │
        No             │    Snapshot  │
        │              │  - Check     │
        ▼              │    Visibility│
    ┌────────┐         └──────────────┘
    │ Write? │──Yes──►┌───────────────┐
    └────────┘        │ Lock + Write  │
                      │  - Acquire    │
                      │    Lock (2PL) │
                      │  - Create     │
                      │    Version    │
                      │  - WAL Entry  │
                      └───────┬───────┘
                              │
                              ▼
                      ┌───────────────┐
                      │ COMMIT/ABORT  │
                      └───────┬───────┘
                              │
                              ▼
                      ┌───────────────┐
                      │ Validation    │
                      │  - Deadlock   │
                      │    Check      │
                      │  - Write-Skew │
                      │    Check      │
                      └───────┬───────┘
                              │
                              ▼
                      ┌───────────────┐
                      │ Commit Phase  │
                      │  - WAL Flush  │
                      │  - Release    │
                      │    Locks      │
                      │  - Update     │
                      │    State      │
                      └───────────────┘
```

---

## ACID Guarantees

### Atomicity

**Implementation:** Write-Ahead Logging (WAL) + ARIES Recovery

All modifications are logged to WAL before being applied to the database. On crash:
- **Analysis Phase**: Reconstruct transaction and dirty page tables
- **Redo Phase**: Replay all committed changes
- **Undo Phase**: Roll back uncommitted transactions

**Guarantee:** All operations within a transaction complete, or none do.

### Consistency

**Implementation:** Constraint validation + Integrity checking

- Primary key, foreign key, unique, and check constraints enforced
- Referential integrity maintained across transactions
- Triggers and stored procedures for complex constraints

**Guarantee:** Transactions transition the database from one valid state to another.

### Isolation

**Implementation:** MVCC + 2PL + Snapshot Isolation

Multiple isolation levels provide different trade-offs:

| Level | Dirty Read | Non-Repeatable Read | Phantom Read | Write Skew |
|-------|------------|---------------------|--------------|------------|
| READ UNCOMMITTED | Possible | Possible | Possible | Possible |
| READ COMMITTED | **Prevented** | Possible | Possible | Possible |
| REPEATABLE READ | **Prevented** | **Prevented** | Possible | Possible |
| SERIALIZABLE | **Prevented** | **Prevented** | **Prevented** | **Prevented** |
| SNAPSHOT ISOLATION | **Prevented** | **Prevented** | **Prevented** | Detected* |

*Write-skew detection is optional but enabled by default.

**Guarantee:** Concurrent transactions execute as if serialized (depending on isolation level).

### Durability

**Implementation:** WAL with fsync + Group Commit + Checkpointing

- **Force-at-Commit**: WAL entries are flushed to disk before COMMIT returns
- **Group Commit**: Batches multiple transaction commits for better throughput
- **Hardware CRC32C**: SSE4.2-accelerated checksums for data integrity
- **Fuzzy Checkpointing**: Non-blocking checkpoints for recovery optimization

**Guarantee:** Committed transactions survive system crashes and power failures.

---

## Transaction Lifecycle

### State Machine

```
                    ┌────────┐
                    │ BEGIN  │
                    └───┬────┘
                        │
                        ▼
                   ┌─────────┐
                   │ ACTIVE  │◄──────┐
                   └────┬────┘       │
                        │            │
                        ▼            │
                   ┌─────────┐       │
                   │ GROWING │───────┘ (2PL: Acquire Locks)
                   └────┬────┘
                        │
                        ▼
                  ┌──────────┐
                  │SHRINKING │ (2PL: Release Locks)
                  └────┬─────┘
                       │
              ┌────────┴────────┐
              │                 │
              ▼                 ▼
        ┌──────────┐      ┌──────────┐
        │COMMITTING│      │ ABORTING │
        └────┬─────┘      └────┬─────┘
             │                 │
             ▼                 ▼
        ┌──────────┐      ┌──────────┐
        │COMMITTED │      │ ABORTED  │
        └──────────┘      └──────────┘
```

### State Descriptions

- **ACTIVE**: Transaction has begun, can execute operations
- **GROWING**: Two-phase locking growing phase (acquiring locks)
- **SHRINKING**: Two-phase locking shrinking phase (releasing locks)
- **PREPARING**: Two-phase commit prepare phase (distributed)
- **PREPARED**: Ready to commit (distributed)
- **COMMITTING**: In the process of committing
- **COMMITTED**: Successfully committed (terminal state)
- **ABORTING**: In the process of aborting
- **ABORTED**: Successfully aborted (terminal state)

### API Usage

```rust
use rusty_db::transaction::{TransactionManager, IsolationLevel};

// Create transaction manager
let tm = TransactionManager::new();

// Begin transaction with default isolation (READ COMMITTED)
let txn_id = tm.begin()?;

// Or specify isolation level
let txn_id = tm.begin_with_isolation(IsolationLevel::Serializable)?;

// Begin read-only transaction (optimized, no write locks)
let txn_id = tm.begin_readonly()?;

// Execute operations...
tm.record_read(txn_id, "users:1".to_string());
tm.record_write(txn_id, "users:1".to_string());

// Commit transaction
tm.commit(txn_id)?;

// Or abort transaction
tm.abort(txn_id)?;
```

### Transaction Metadata

Each transaction maintains:

```rust
pub struct Transaction {
    pub id: TransactionId,                  // Unique identifier (monotonic)
    pub state: TransactionState,            // Current lifecycle state
    pub isolation_level: IsolationLevel,    // Isolation level
    pub start_time: SystemTime,             // When transaction began
    pub last_activity: SystemTime,          // Last operation timestamp
    pub held_locks: HashSet<String>,        // Currently held locks
    pub read_set: HashSet<String>,          // Keys read (for validation)
    pub write_set: HashSet<String>,         // Keys written (for validation)
    pub start_lsn: LogSequenceNumber,       // LSN when started
    pub end_lsn: Option<LogSequenceNumber>, // LSN when ended
    pub savepoints: Vec<Savepoint>,         // Stack of savepoints
    pub is_readonly: bool,                  // Read-only flag
    pub timeout_duration: Option<Duration>, // Optional timeout
    pub parent_txn: Option<TransactionId>,  // Parent for nested txns
}
```

### Timeout Management

**Default Timeout:** 1 hour (configurable per-transaction)

```rust
// Abort timed-out transactions
let aborted = tm.abort_timed_out_transactions()?;
println!("Aborted {} timed-out transactions", aborted);

// Check specific transaction
if tm.is_timed_out(txn_id) {
    tm.abort(txn_id)?;
}

// Get transaction age
if let Some(age) = tm.get_transaction_age(txn_id) {
    println!("Transaction running for {:?}", age);
}
```

---

## Multi-Version Concurrency Control (MVCC)

### Overview

MVCC enables **non-blocking reads** by maintaining multiple versions of each data item. Readers see a consistent snapshot without acquiring locks, while writers create new versions.

### Hybrid Logical Clocks (HLC)

RustyDB uses Hybrid Logical Clocks for timestamp ordering in distributed systems:

```rust
pub struct HybridTimestamp {
    pub physical: u64,  // Milliseconds since epoch
    pub logical: u64,   // Logical counter for same physical time
    pub node_id: u32,   // Node identifier
}
```

**Properties:**
- Monotonically increasing
- Captures causality (happens-before relationships)
- Clock skew detection (max 5 seconds tolerance)

### Version Storage

Each record version contains:

```rust
pub struct VersionedRecord<T: Clone> {
    pub data: T,                             // Data payload
    pub created_by: TransactionId,           // Creating transaction
    pub created_at: HybridTimestamp,         // Creation timestamp
    pub deleted_by: Option<TransactionId>,   // Deleting transaction
    pub deleted_at: Option<HybridTimestamp>, // Deletion timestamp
    pub lsn: LogSequenceNumber,              // WAL LSN
    pub next_version: Option<usize>,         // Next in version chain
    pub prev_version: Option<usize>,         // Previous in version chain
}
```

### Visibility Rules

A version is visible to transaction T with snapshot timestamp `read_ts` if:

1. **Version was created before snapshot:**
   ```
   version.created_at < read_ts OR version.created_at == read_ts
   ```

2. **Version is not deleted, OR deleted after snapshot:**
   ```
   version.deleted_at == None OR read_ts < version.deleted_at
   ```

3. **Special case - Read your own writes:**
   ```
   version.created_by == transaction.id
   ```

### Version Chain Example

```
Time: 0ms        100ms       200ms       300ms
      │           │           │           │
      V1──────────V2──────────V3──────────V4
    (data=A)   (data=B)   (data=C)   (deleted)
    TXN:1      TXN:5      TXN:10     TXN:15

Transaction with snapshot at t=250ms sees V3 (data=C)
Transaction with snapshot at t=150ms sees V2 (data=B)
Transaction with snapshot at t=50ms  sees V1 (data=A)
Transaction with snapshot at t=350ms sees nothing (deleted)
```

### Garbage Collection

**Strategy:** Lazy garbage collection based on oldest active snapshot

```rust
// MVCC manager automatically triggers GC when:
// 1. Snapshot ends and min_snapshot advances significantly (10s)
// 2. Last active snapshot ends (clean up all old versions)

impl MVCCManager {
    pub fn garbage_collect(&self) -> Result<usize> {
        let min_ts = self.min_snapshot_ts.read();
        // Remove versions older than oldest active snapshot
        // Returns number of versions collected
    }
}
```

**Configuration:**

```rust
pub struct MVCCConfig {
    pub max_versions: usize,          // Per-key limit (default: 100)
    pub global_max_versions: usize,   // Global limit (default: 10M)
    pub auto_gc: bool,                // Auto-trigger GC (default: true)
    pub gc_interval_secs: u64,        // GC interval (default: 60s)
    pub node_id: u32,                 // Node ID for HLC
}
```

### Memory Pressure Handling

MVCC integrates with memory pressure manager:

```rust
// Register MVCC with memory pressure callbacks
mvcc.on_memory_pressure()?; // Force aggressive GC
```

**Critical Fixes (EA2):**
- **Global version counter**: Prevents unbounded growth across all keys
- **Max entry enforcement**: Limits total versions (10M default)
- **Automatic GC triggers**: Based on snapshot advancement

---

## Isolation Levels

### READ UNCOMMITTED

**Description:** Allows dirty reads (reading uncommitted data from other transactions)

**Use Cases:**
- Analytics queries that tolerate approximate results
- Non-critical reporting workloads

**Anomalies Allowed:**
- Dirty reads
- Non-repeatable reads
- Phantom reads
- Write skew

**Implementation:** Minimal locking, no version checking

```rust
let txn_id = tm.begin_with_isolation(IsolationLevel::ReadUncommitted)?;
```

---

### READ COMMITTED (Default)

**Description:** Only sees data committed before each statement

**Use Cases:**
- Default for most OLTP workloads
- Web applications
- General-purpose transactions

**Anomalies Prevented:**
- ✅ Dirty reads

**Anomalies Allowed:**
- Non-repeatable reads
- Phantom reads
- Write skew

**Implementation:**
- Shared locks released immediately after read
- Exclusive locks held until commit (2PL shrinking phase)

```rust
let txn_id = tm.begin()?; // Default is READ COMMITTED
```

---

### REPEATABLE READ

**Description:** Guarantees consistent reads within transaction

**Use Cases:**
- Financial calculations
- Reports requiring consistency
- Batch processing

**Anomalies Prevented:**
- ✅ Dirty reads
- ✅ Non-repeatable reads

**Anomalies Allowed:**
- Phantom reads (range scans may see new rows)
- Write skew

**Implementation:**
- All locks (shared and exclusive) held until commit (strict 2PL)
- MVCC snapshot taken at transaction start

```rust
let txn_id = tm.begin_with_isolation(IsolationLevel::RepeatableRead)?;
```

---

### SERIALIZABLE

**Description:** Full serializability - transactions appear to execute in serial order

**Use Cases:**
- Critical financial transactions
- Inventory management
- Any workload requiring strict consistency

**Anomalies Prevented:**
- ✅ Dirty reads
- ✅ Non-repeatable reads
- ✅ Phantom reads
- ✅ Write skew

**Implementation:**
- Predicate locking for range queries
- Serialization graph testing (SSI)
- Next-key locking to prevent phantoms

```rust
let txn_id = tm.begin_with_isolation(IsolationLevel::Serializable)?;
```

---

### SNAPSHOT ISOLATION

**Description:** MVCC-based isolation with point-in-time snapshot consistency

**Use Cases:**
- Read-heavy workloads
- Long-running read queries
- Low-contention scenarios

**Anomalies Prevented:**
- ✅ Dirty reads
- ✅ Non-repeatable reads
- ✅ Phantom reads
- ⚠️ Write skew (detected with optional validation)

**Implementation:**
- MVCC snapshots
- First-committer-wins for write-write conflicts
- Optional write-skew detection via read set validation

```rust
// Snapshot isolation with write-skew detection
let config = SnapshotConfig {
    serializable: false,
    detect_write_skew: true,
    retention_secs: 300,
    max_committed_writes: 100_000,
    node_id: 0,
};
let si_manager = SnapshotIsolationManager::new(config);

let txn_id = si_manager.begin_transaction(1, false);
si_manager.record_read(txn_id, "account:A".to_string())?;
si_manager.record_write(txn_id, "account:B".to_string())?;
si_manager.commit_transaction(txn_id)?; // Validates read set
```

### Write-Skew Detection

**Example:** Classic write-skew anomaly

```
Initial state: A=100, B=100, Constraint: A+B >= 100

T1: reads A=100, B=100
T2: reads A=100, B=100
T1: writes A=0   (sees B=100, so constraint OK)
T2: writes B=0   (sees A=100, so constraint OK)
COMMIT T1
COMMIT T2

Final state: A=0, B=0 (VIOLATES CONSTRAINT!)
```

**Detection Algorithm:**

1. Track read set for each transaction
2. Track committed writes with timestamps
3. On commit, validate no committed transaction wrote to our read set
4. Reject commit if read set overlaps with concurrent writes

```rust
// T1 commits successfully
si_manager.commit_transaction(1)?; // OK

// T2 fails due to write-skew
let result = si_manager.commit_transaction(2);
assert!(result.is_err()); // "Write-skew detected: transaction 2 read keys..."
```

---

## Lock Management and Two-Phase Locking

### Lock Modes

RustyDB supports hierarchical locking with intent locks:

| Lock Mode | Abbreviation | Description |
|-----------|--------------|-------------|
| **Shared** | S | Read lock - multiple transactions can hold |
| **Exclusive** | X | Write lock - exclusive access |
| **Intent Shared** | IS | Intent to acquire S locks on descendants |
| **Intent Exclusive** | IX | Intent to acquire X locks on descendants |
| **Shared Intent Exclusive** | SIX | S lock with intent for X locks on descendants |
| **Update** | U | Prevents deadlocks during S→X upgrade |

### Lock Compatibility Matrix

|       | S | X | IS | IX | SIX | U |
|-------|---|---|----|----|-----|---|
| **S**   | ✅ | ❌ | ✅  | ❌  | ❌   | ✅ |
| **X**   | ❌ | ❌ | ❌  | ❌  | ❌   | ❌ |
| **IS**  | ✅ | ❌ | ✅  | ✅  | ✅   | ✅ |
| **IX**  | ❌ | ❌ | ✅  | ✅  | ❌   | ❌ |
| **SIX** | ❌ | ❌ | ✅  | ❌  | ❌   | ❌ |
| **U**   | ✅ | ❌ | ✅  | ❌  | ❌   | ❌ |

### Lock Granularity

```rust
pub enum LockGranularity {
    Row,      // Finest - highest concurrency
    Page,     // Medium
    Table,    // Coarse
    Database, // Coarsest - lowest concurrency
}
```

### Two-Phase Locking (2PL) Protocol

**Growing Phase:**
- Transaction acquires locks as needed
- Cannot release any locks
- State: ACTIVE → GROWING

**Shrinking Phase:**
- Transaction releases locks
- Cannot acquire new locks
- State: GROWING → SHRINKING → COMMITTED/ABORTED

**Guarantee:** Ensures conflict-serializability

### Lock Manager Implementation

```rust
impl LockManager {
    /// Acquire lock with timeout (default: 30 seconds)
    pub fn acquire_lock(
        &self,
        txn_id: TransactionId,
        resource: String,
        mode: LockMode,
    ) -> TransactionResult<()>;

    /// Acquire lock with custom timeout
    pub fn acquire_lock_with_timeout(
        &self,
        txn_id: TransactionId,
        resource: String,
        mode: LockMode,
        timeout: Duration,
    ) -> TransactionResult<()>;

    /// Try to acquire without blocking
    pub fn try_acquire_lock(
        &self,
        txn_id: TransactionId,
        resource: String,
        mode: LockMode,
    ) -> TransactionResult<bool>;

    /// Release specific lock
    pub fn release_lock(
        &self,
        txn_id: TransactionId,
        resource: &str
    ) -> TransactionResult<()>;

    /// Release all locks (called on commit/abort)
    pub fn release_all_locks(
        &self,
        txn_id: TransactionId
    ) -> TransactionResult<()>;
}
```

### Lock Wait Queue

**Architecture:** FIFO queue with condition variables

```rust
// Internal structure
struct LockTableEntry {
    holders: Vec<(TransactionId, LockMode)>,
    waiters: VecDeque<LockRequest>,
}

struct LockRequest {
    txn_id: TransactionId,
    mode: LockMode,
    timestamp: SystemTime,
}
```

**Lock Acquisition Algorithm:**

1. Check if lock is compatible with current holders
2. If YES: Grant lock immediately
3. If NO: Add to wait queue, wait on condition variable
4. On wake-up (lock release), retry acquisition
5. Timeout after MAX_LOCK_WAIT_MS (30 seconds)

### Lock Escalation

**Purpose:** Reduce lock overhead by upgrading row locks to table locks

**Configuration:**

```rust
let escalation_mgr = LockEscalationManager::new(1000); // Threshold: 1000 row locks
```

**Algorithm:**

```rust
// Check if escalation needed
if escalation_mgr.record_row_lock(txn_id, "users".to_string(), "row_123".to_string()) {
    // Escalate: release all row locks, acquire table lock
    let count = escalation_mgr.escalate(
        txn_id,
        "users",
        &lock_manager,
        LockMode::Exclusive
    )?;
    println!("Escalated {} row locks to table lock", count);
}
```

**Critical Fixes (EA2):**
- Lock timeout support (prevents indefinite blocking)
- Wait queue with condition variables
- Atomic lock release + notify (prevents wake-up races)

---

## Deadlock Detection and Resolution

### Wait-For Graph

**Structure:** Directed graph where:
- Nodes = Transactions
- Edge (T1 → T2) = T1 is waiting for T2

**Example:**

```
T1 holds L1, waits for L2
T2 holds L2, waits for L3
T3 holds L3, waits for L1

Wait-For Graph:
T1 ──→ T2 ──→ T3 ──→ T1  (CYCLE = DEADLOCK!)
```

### Detection Algorithm

**Method:** Depth-First Search (DFS) for cycle detection

**Interval:** Configurable (default: 1 second)

```rust
pub struct DeadlockDetectorConfig {
    pub detection_interval: Duration,    // Min time between runs
    pub victim_policy: VictimSelectionPolicy,
    pub max_detection_depth: usize,      // Prevent infinite loops
}
```

### Deadlock Detection Flow

```
┌─────────────────┐
│ Wait-For Graph  │
│  Maintenance    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Rate-Limited    │───No───► Skip Detection
│ Check           │
└────────┬────────┘
        Yes
         │
         ▼
┌─────────────────┐
│ DFS Cycle       │
│ Detection       │
└────────┬────────┘
         │
    ┌────┴────┐
   No        Yes
    │          │
    │          ▼
    │   ┌─────────────────┐
    │   │ Select Victim   │
    │   │  - Youngest     │
    │   │  - Oldest       │
    │   │  - Least Work   │
    │   │  - Low Priority │
    │   └────────┬────────┘
    │            │
    │            ▼
    │   ┌─────────────────┐
    │   │ Abort Victim    │
    │   └────────┬────────┘
    │            │
    └────────────┴────► Continue
```

### Victim Selection Policies

```rust
pub enum VictimSelectionPolicy {
    Youngest,       // Abort highest transaction ID (default)
    Oldest,         // Abort lowest transaction ID
    LeastWork,      // Abort transaction with least operations
    LowestPriority, // Abort transaction with lowest priority
}
```

### API Usage

```rust
let detector = DeadlockDetector::with_config(DeadlockDetectorConfig {
    detection_interval: Duration::from_secs(1),
    victim_policy: VictimSelectionPolicy::Youngest,
    max_detection_depth: 1000,
});

// Add wait edge: T1 waiting for T2
detector.add_wait(1, 2);

// Detect deadlock
if let Some(cycle) = detector.detect_deadlock() {
    let victim = detector.select_victim(&cycle);
    detector.record_victim_aborted();
    tm.abort(victim)?;
}

// Remove wait when lock granted
detector.remove_wait(1);

// Statistics
let stats = detector.stats();
println!("Deadlocks found: {}", stats.deadlocks_found);
println!("Victims aborted: {}", stats.victims_aborted);
```

### Prevention vs. Detection

**RustyDB uses DETECTION** (not prevention) because:
- Lower overhead for typical workloads
- Better concurrency (no conservative lock ordering)
- Configurable victim selection
- Detailed statistics for tuning

**Prevention Alternative:** Lock ordering (all transactions acquire locks in same order)

---

## Write-Ahead Logging (WAL)

### Overview

RustyDB implements ARIES-style physiological logging with hardware-accelerated checksums and group commit optimization.

### WAL Architecture

```
┌────────────────────────────────────────────────────────────┐
│                      WALManager                             │
├────────────────────────────────────────────────────────────┤
│  Group Commit Buffer (Max: 10K entries, 4MB)               │
│  ┌──────────┬──────────┬──────────┬──────────┐            │
│  │ Entry 1  │ Entry 2  │ Entry 3  │ ...      │            │
│  └──────────┴──────────┴──────────┴──────────┘            │
│                      │                                      │
│                      ▼                                      │
│  ┌─────────────────────────────────────────────┐          │
│  │  Flush Triggers:                            │          │
│  │   - Entry count >= 10K (prevent overflow)   │          │
│  │   - Size >= 4MB                             │          │
│  │   - Age >= 10ms                             │          │
│  └─────────────────────────────────────────────┘          │
│                      │                                      │
│                      ▼                                      │
│  ┌─────────────────────────────────────────────┐          │
│  │  Vectored I/O Write (scatter-gather)        │          │
│  │   - Batch checksums (CRC32C SSE4.2)         │          │
│  │   - Single system call                      │          │
│  │   - fsync (if AlwaysSync mode)              │          │
│  └─────────────────────────────────────────────┘          │
│                      │                                      │
│                      ▼                                      │
│            WAL File (64MB segments)                         │
└────────────────────────────────────────────────────────────┘
```

### Log Record Types

```rust
pub enum LogRecord {
    Begin {
        txn_id: TransactionId,
        timestamp: SystemTime
    },

    Update {
        txn_id: TransactionId,
        page_id: PageId,
        offset: u32,
        before_image: Vec<u8>,  // For undo
        after_image: Vec<u8>,   // For redo
        undo_next_lsn: Option<LSN>,
    },

    Insert {
        txn_id: TransactionId,
        page_id: PageId,
        offset: u32,
        data: Vec<u8>,
        undo_next_lsn: Option<LSN>,
    },

    Delete {
        txn_id: TransactionId,
        page_id: PageId,
        offset: u32,
        deleted_data: Vec<u8>,  // For undo
        undo_next_lsn: Option<LSN>,
    },

    Commit {
        txn_id: TransactionId,
        timestamp: SystemTime
    },

    Abort {
        txn_id: TransactionId,
        timestamp: SystemTime
    },

    CLR {  // Compensation Log Record
        txn_id: TransactionId,
        page_id: PageId,
        undo_next_lsn: Option<LSN>,
        redo_operation: Box<LogRecord>,
    },

    CheckpointBegin {
        timestamp: SystemTime
    },

    CheckpointEnd {
        active_txns: Vec<TransactionId>,
        dirty_pages: Vec<PageId>,
        timestamp: SystemTime,
    },
}
```

### WAL Entry Format

```
┌────────────────────────────────────────────────────┐
│                    WAL Entry                       │
├────────────────────────────────────────────────────┤
│ LSN (8 bytes)              │ Unique log ID         │
│ Previous LSN (8 bytes)     │ Transaction LSN chain│
│ Record Type (1 byte)       │ Begin/Update/Commit  │
│ Transaction ID (8 bytes)   │ Associated txn       │
│ Size (4 bytes)             │ Entry size           │
│ Checksum (4 bytes)         │ CRC32C (SSE4.2)     │
│ Record Data (variable)     │ Serialized record    │
└────────────────────────────────────────────────────┘
```

### Group Commit Optimization

**Purpose:** Batch multiple transaction commits into single disk write

**Configuration:**

```rust
pub struct WALConfig {
    pub max_buffer_size: usize,        // 4 MB default
    pub max_commit_delay_ms: u64,      // 10 ms default
    pub enable_group_commit: bool,     // true (production)
    pub segment_size: usize,           // 64 MB
    pub enable_log_shipping: bool,     // For replication
    pub sync_mode: SyncMode,           // AlwaysSync/PeriodicSync/NoSync
}

pub enum SyncMode {
    AlwaysSync,    // fsync after every commit (safest, slowest)
    PeriodicSync,  // fsync every N ms (faster, less safe)
    NoSync,        // No fsync (testing only, NOT DURABLE)
}
```

**Performance Impact:**

| Sync Mode | Throughput | Latency | Durability |
|-----------|------------|---------|------------|
| AlwaysSync | 5K TPS | 2ms | ✅ Full |
| PeriodicSync | 50K TPS | 0.5ms | ⚠️ Partial |
| NoSync | 200K TPS | 0.1ms | ❌ None |

### Hardware-Accelerated Checksums

**SSE4.2 CRC32C Intrinsics:**

```rust
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse4.2")]
unsafe fn hardware_crc32c_impl(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    let mut ptr = data.as_ptr();
    let mut remaining = data.len();

    // Process 8 bytes at a time
    while remaining >= 8 {
        let value = (ptr as *const u64).read_unaligned();
        crc = _mm_crc32_u64(crc as u64, value) as u32;
        ptr = ptr.add(8);
        remaining -= 8;
    }

    // Process remaining bytes
    while remaining > 0 {
        crc = _mm_crc32_u8(crc, *ptr);
        ptr = ptr.add(1);
        remaining -= 1;
    }

    !crc
}
```

**Performance:** ~10x faster than software CRC32

### Vectored I/O (Scatter-Gather)

**Benefit:** Single system call for multiple buffers

```rust
// Pseudo-code for group commit flush
fn flush_group(&self, entries: Vec<WALEntry>) -> Result<()> {
    let io_slices: Vec<IoSlice> = entries
        .iter()
        .map(|entry| IoSlice::new(&entry.serialized))
        .collect();

    // Single writev() system call
    self.wal_file.write_vectored(&io_slices)?;

    if self.config.sync_mode == SyncMode::AlwaysSync {
        self.wal_file.sync_all()?;  // fsync
    }

    Ok(())
}
```

### LSN Allocation

**Algorithm:** Atomic counter with SeqCst ordering

```rust
fn allocate_lsn(&self) -> LSN {
    self.next_lsn.fetch_add(1, Ordering::SeqCst)
}
```

**Properties:**
- Monotonically increasing
- Unique across all transactions
- Ordered by commit time

### WAL File Management

**Segmentation:** 64MB segments for easier archival

```
wal_00000001.log  (64 MB, LSN 1-1000000)
wal_00000002.log  (64 MB, LSN 1000001-2000000)
wal_00000003.log  (Active)
```

**Truncation:** After checkpointing, old segments can be archived or deleted

```rust
// Truncate WAL before LSN (removes old entries)
wal_manager.truncate(checkpoint_lsn)?;
```

### Critical Fixes (EA2)

1. **EA2-V4**: Max entry count limit (10K) prevents buffer overflow
2. **EA2-RACE-3**: Atomic truncate operation prevents concurrent write loss

---

## ARIES Recovery Algorithm

### Overview

ARIES (Algorithm for Recovery and Isolation Exploiting Semantics) is the industry-standard crash recovery algorithm used by enterprise databases.

### Three Phases

```
┌──────────────────────────────────────────────────────────┐
│                    ARIES Recovery                         │
├──────────────────────────────────────────────────────────┤
│                                                           │
│  Phase 1: ANALYSIS                                        │
│  ┌────────────────────────────────────────────────────┐  │
│  │ • Scan WAL from last checkpoint                    │  │
│  │ • Build transaction table (active/committed)       │  │
│  │ │ Build dirty page table (pages to redo)          │  │
│  │ • Determine min recovery LSN                       │  │
│  └────────────────────────────────────────────────────┘  │
│                      │                                    │
│                      ▼                                    │
│  Phase 2: REDO                                            │
│  ┌────────────────────────────────────────────────────┐  │
│  │ • Replay WAL from min recovery LSN                 │  │
│  │ • Apply all updates (committed + uncommitted)      │  │
│  │ • Restore database to crash state                  │  │
│  │ • Idempotent operations (can replay multiple)      │  │
│  └────────────────────────────────────────────────────┘  │
│                      │                                    │
│                      ▼                                    │
│  Phase 3: UNDO                                            │
│  ┌────────────────────────────────────────────────────┐  │
│  │ • Roll back uncommitted transactions               │  │
│  │ • Process in reverse LSN order                     │  │
│  │ • Write CLRs for each undo operation               │  │
│  │ • Use undo_next_lsn to skip already-undone ops    │  │
│  └────────────────────────────────────────────────────┘  │
│                                                           │
└──────────────────────────────────────────────────────────┘
```

### Phase 1: Analysis

**Purpose:** Reconstruct system state at crash

**Data Structures:**

```rust
// Transaction Table
HashMap<TransactionId, TransactionTableEntry> {
    txn_id: TransactionId,
    state: RecoveryTxnState,  // Active/Committed/Aborted
    last_lsn: LSN,
    undo_next_lsn: Option<LSN>,
}

// Dirty Page Table
HashMap<PageId, DirtyPageEntry> {
    page_id: PageId,
    rec_lsn: LSN,  // First LSN that dirtied this page
}
```

**Algorithm:**

1. Find last checkpoint LSN
2. Read checkpoint record (active txns, dirty pages)
3. Scan WAL from checkpoint to end:
   - For BEGIN: Add to transaction table
   - For UPDATE/INSERT/DELETE: Update transaction table, add to dirty pages
   - For COMMIT/ABORT: Mark transaction state
   - For CLR: Update transaction table
4. Return min(dirty_pages.rec_lsn) as recovery start point

**Output:**
- List of active transactions (need undo)
- Minimum recovery LSN (redo start point)

### Phase 2: Redo

**Purpose:** Replay history to restore database state at crash

**Algorithm:**

```rust
async fn redo_phase(&self, start_lsn: LSN) -> Result<()> {
    let log_entries = self.wal.read_from(start_lsn)?;
    let dirty_pages = self.dirty_page_table.read();

    for entry in log_entries {
        if let Some(page_id) = entry.record.page_id() {
            // Only redo if page was dirty and LSN >= rec_lsn
            if let Some(dirty_entry) = dirty_pages.get(&page_id) {
                if entry.lsn >= dirty_entry.rec_lsn {
                    self.redo_record(&entry).await?;
                }
            }
        }
    }

    Ok(())
}
```

**Redo Operations:**

- **Update**: Apply `after_image` to page
- **Insert**: Insert data at offset
- **Delete**: Mark as deleted
- **CLR**: Redo the compensated operation

**Idempotency:** Safe to redo multiple times (LSN checks prevent duplicates)

### Phase 3: Undo

**Purpose:** Roll back uncommitted transactions

**Algorithm:**

```rust
async fn undo_phase(&self, undo_list: Vec<TransactionId>) -> Result<()> {
    let mut undo_queue: BTreeMap<LSN, TransactionId> = BTreeMap::new();

    // Initialize with last LSN of each active transaction
    for txn_id in undo_list {
        let entry = self.transaction_table.read().get(&txn_id);
        undo_queue.insert(entry.last_lsn, txn_id);
    }

    // Process in reverse LSN order
    while let Some((lsn, txn_id)) = undo_queue.pop_last() {
        let entry = self.wal.read_at(lsn)?;

        // Undo the operation
        self.undo_record(&entry).await?;

        // Write CLR
        self.write_clr(txn_id, lsn)?;

        // Follow undo chain
        if let Some(next_lsn) = entry.undo_next_lsn() {
            undo_queue.insert(next_lsn, txn_id);
        }
    }

    Ok(())
}
```

**Undo Operations:**

- **Update**: Apply `before_image` to page
- **Insert**: Delete the inserted data
- **Delete**: Restore `deleted_data`
- **CLR**: Skip (redo-only, already compensated)

### Compensation Log Records (CLR)

**Purpose:** Record undo operations to avoid repeating them during recovery

**Structure:**

```rust
CLR {
    txn_id: TransactionId,
    page_id: PageId,
    undo_next_lsn: Option<LSN>,  // Skip undone operations
    redo_operation: Box<LogRecord>,  // What was done during undo
}
```

**Example:**

```
Original:
LSN 100: BEGIN T1
LSN 101: UPDATE page 5, offset 10, before=A, after=B, undo_next=None
LSN 102: UPDATE page 5, offset 20, before=X, after=Y, undo_next=LSN 101
LSN 103: COMMIT T1
[CRASH]

Recovery (undo T1):
LSN 104: CLR for T1, undo LSN 102 (restore X), undo_next=LSN 101
LSN 105: CLR for T1, undo LSN 101 (restore A), undo_next=None
LSN 106: ABORT T1
```

### Fuzzy Checkpointing

**Purpose:** Non-blocking checkpoints for recovery optimization

**Algorithm:**

```rust
async fn fuzzy_checkpoint(&self) -> Result<LSN> {
    // 1. Write CHECKPOINT-BEGIN
    let begin_lsn = self.wal.append(LogRecord::CheckpointBegin {
        timestamp: SystemTime::now(),
    }).await?;

    // 2. Capture current state (non-blocking)
    let active_txns = self.get_active_transactions();
    let dirty_pages = self.get_dirty_pages();

    // 3. Write CHECKPOINT-END
    let end_lsn = self.wal.append(LogRecord::CheckpointEnd {
        active_txns,
        dirty_pages,
        timestamp: SystemTime::now(),
    }).await?;

    // 4. Update master record (atomic)
    self.update_checkpoint_lsn(begin_lsn)?;

    Ok(begin_lsn)
}
```

**Benefits:**
- No transaction blocking
- Recovery starts from checkpoint (not beginning of log)
- Reduced recovery time

### Point-in-Time Recovery (PITR)

**Purpose:** Restore database to specific timestamp

```rust
pub struct PointInTimeRecovery {
    target_time: SystemTime,
    archive_logs: PathBuf,
}

impl PointInTimeRecovery {
    pub async fn recover_to_time(&self, target: SystemTime) -> Result<()> {
        // 1. Restore from last full backup
        self.restore_base_backup().await?;

        // 2. Apply archived WAL logs up to target time
        for log_file in self.get_archived_logs()? {
            let entries = self.read_archived_log(log_file)?;
            for entry in entries {
                if entry.timestamp() <= target {
                    self.apply_entry(&entry).await?;
                } else {
                    break;  // Stop at target time
                }
            }
        }

        Ok(())
    }
}
```

### Recovery Statistics

```rust
pub struct RecoveryStats {
    pub recovery_runs: u64,
    pub last_recovery_time_ms: u64,
    pub analysis_time_ms: u64,
    pub redo_time_ms: u64,
    pub undo_time_ms: u64,
    pub records_analyzed: u64,
    pub records_redone: u64,
    pub records_undone: u64,
    pub transactions_recovered: u64,
    pub transactions_rolled_back: u64,
}
```

**Example Output:**

```
ARIES Recovery completed in 2547ms
  Analysis: 342ms (15,234 records)
  Redo: 1823ms (12,891 records)
  Undo: 382ms (2,343 records)
  Transactions: 42 recovered, 3 rolled back
```

---

## Snapshot Isolation

### Overview

Snapshot Isolation (SI) provides each transaction with a consistent point-in-time view of the database using MVCC.

### Snapshot Creation

```rust
pub struct Snapshot {
    pub id: u64,                          // Unique snapshot ID
    pub txn_id: TransactionId,            // Owning transaction
    pub timestamp: SystemTime,            // Snapshot timestamp
    pub active_txns: HashSet<TransactionId>,  // Active at snapshot time
    pub min_txn_id: TransactionId,        // Oldest active
    pub max_txn_id: TransactionId,        // Newest active
}
```

**Creation Algorithm:**

```rust
impl SnapshotManager {
    pub fn create_snapshot(
        &self,
        txn_id: TransactionId,
        active_txns: HashSet<TransactionId>,
    ) -> Snapshot {
        let snapshot = Snapshot {
            id: self.allocate_snapshot_id(),
            txn_id,
            timestamp: SystemTime::now(),
            active_txns: active_txns.clone(),
            min_txn_id: *active_txns.iter().min().unwrap_or(&0),
            max_txn_id: *active_txns.iter().max().unwrap_or(&txn_id),
        };

        self.register_snapshot(snapshot.clone());
        snapshot
    }
}
```

### Visibility Rules

```rust
impl Snapshot {
    pub fn is_visible(&self, txn_id: TransactionId) -> bool {
        // 1. Read your own writes
        if txn_id == self.txn_id {
            return true;
        }

        // 2. Not visible if was active at snapshot time
        if self.active_txns.contains(&txn_id) {
            return false;
        }

        // 3. Visible if committed before snapshot
        txn_id < self.txn_id
    }
}
```

### Snapshot Isolation Manager

```rust
pub struct SnapshotIsolationManager {
    active_txns: Arc<RwLock<HashMap<TransactionId, TransactionSnapshot>>>,
    write_sets: Arc<RwLock<HashMap<TransactionId, HashSet<String>>>>,
    committed_writes: Arc<RwLock<BTreeMap<HybridTimestamp, HashSet<String>>>>,
    clock: Arc<HybridClock>,
    config: SnapshotConfig,
}

pub struct SnapshotConfig {
    pub serializable: bool,            // Upgrade to SSI
    pub detect_write_skew: bool,       // Enable write-skew detection
    pub retention_secs: u64,           // How long to keep commit records
    pub max_committed_writes: usize,   // Max commit records (100K default)
    pub node_id: u32,
}
```

### Commit Validation

**First-Committer-Wins:**

```rust
// Check write-write conflicts
for (other_txn, _) in active_txns {
    if *other_txn == txn_id {
        continue;
    }

    let my_writes = write_sets.get(&txn_id);
    let other_writes = write_sets.get(other_txn);

    // Reject if overlapping writes
    if my_writes.intersection(other_writes).next().is_some() {
        return Err("Write-write conflict");
    }
}
```

**Write-Skew Detection (Optional but Recommended):**

```rust
// Validate read set against committed writes
for (commit_ts, committed_keys) in committed_writes.range(snapshot.start_ts..) {
    let conflicts = snapshot.read_set.intersection(committed_keys);

    if !conflicts.is_empty() {
        return Err(TransactionError::ValidationFailed {
            txn_id,
            key: conflicts[0].clone(),
        });
    }
}
```

### Garbage Collection

```rust
// Clean up old committed write records
fn cleanup_committed_writes(&self) {
    let cutoff_time = SystemTime::now() - Duration::from_secs(retention_secs);
    let cutoff_ts = HybridTimestamp::from(cutoff_time);

    // Time-based cleanup
    committed.retain(|ts, _| *ts >= cutoff_ts);

    // Count-based cleanup (LRU eviction)
    if committed.len() > max_committed_writes {
        let excess = committed.len() - max_committed_writes;
        for _ in 0..excess {
            committed.pop_first();  // Remove oldest
        }
    }
}
```

---

## Transaction ID Management

### ID Allocation

**Method:** Monotonically increasing atomic counter

```rust
pub struct TransactionManager {
    next_txn_id: Arc<Mutex<TransactionId>>,
    // ...
}

impl TransactionManager {
    pub fn begin(&self) -> TransactionResult<TransactionId> {
        let txn_id = {
            let mut next_id = self.next_txn_id.lock();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let txn = Transaction::new(txn_id, self.default_isolation);
        self.active_txns.write().insert(txn_id, txn);

        Ok(txn_id)
    }
}
```

**Properties:**
- Unique across all transactions
- Globally ordered (T1 < T2 implies T1 started before T2)
- Overflow protection (u64 = 18 quintillion transactions)

### Nested Transactions

**Support:** Yes, via parent_txn field

```rust
pub fn new_nested(
    id: TransactionId,
    parent_id: TransactionId,
    isolation_level: IsolationLevel,
) -> Self {
    let mut txn = Self::new(id, isolation_level);
    txn.parent_txn = Some(parent_id);
    txn
}
```

**Semantics:**
- Child transaction inherits parent's locks
- Child abort doesn't affect parent
- Parent abort cascades to children

### Savepoints

**Purpose:** Partial rollback within a transaction

```rust
impl Transaction {
    pub fn add_savepoint(&mut self, name: String, lsn: LSN) -> Savepoint {
        let sp = Savepoint {
            id: self.savepoints.len() as u64,
            name: name.clone(),
            txn_id: self.id,
            lsn,
            timestamp: SystemTime::now(),
        };

        self.savepoints.push(sp.clone());
        sp
    }

    pub fn get_savepoint(&self, name: &str) -> Option<&Savepoint> {
        self.savepoints.iter().find(|sp| sp.name == name)
    }
}
```

**SQL Example:**

```sql
BEGIN;
INSERT INTO accounts VALUES (1, 100);
SAVEPOINT sp1;
INSERT INTO accounts VALUES (2, 200);
ROLLBACK TO sp1;  -- Undoes INSERT for account 2
INSERT INTO accounts VALUES (3, 300);
COMMIT;  -- Commits account 1 and 3
```

---

## Performance Tuning

### Configuration Parameters

```rust
// Transaction Manager
pub const DEFAULT_TRANSACTION_TIMEOUT_SECS: u64 = 3600;  // 1 hour

// Lock Manager
pub const MAX_LOCK_WAIT_MS: u64 = 30_000;  // 30 seconds

// WAL
pub struct WALConfig {
    pub max_buffer_size: usize = 4 * 1024 * 1024,  // 4 MB
    pub max_commit_delay_ms: u64 = 10,             // 10 ms
    pub segment_size: usize = 64 * 1024 * 1024,    // 64 MB
}

// MVCC
pub struct MVCCConfig {
    pub max_versions: usize = 100,                 // Per-key
    pub global_max_versions: usize = 10_000_000,   // 10M total
    pub gc_interval_secs: u64 = 60,                // 1 minute
}

// Deadlock Detection
pub struct DeadlockDetectorConfig {
    pub detection_interval: Duration = Duration::from_secs(1),
    pub max_detection_depth: usize = 1000,
}
```

### Tuning Recommendations

#### High-Throughput OLTP

```rust
WALConfig {
    max_buffer_size: 16 * 1024 * 1024,  // 16 MB (larger batches)
    max_commit_delay_ms: 5,              // 5 ms (faster flush)
    enable_group_commit: true,
    sync_mode: SyncMode::AlwaysSync,
}

MVCCConfig {
    max_versions: 50,                    // Lower per-key limit
    global_max_versions: 5_000_000,      // 5M (reduce memory)
    gc_interval_secs: 30,                // More frequent GC
}
```

#### Long-Running Analytics

```rust
WALConfig {
    max_buffer_size: 64 * 1024 * 1024,  // 64 MB (large batches)
    max_commit_delay_ms: 50,             // 50 ms (batch more)
    sync_mode: SyncMode::PeriodicSync,   // Less fsync overhead
}

MVCCConfig {
    max_versions: 200,                   // Keep more history
    global_max_versions: 20_000_000,     // 20M versions
    gc_interval_secs: 300,               // Less frequent GC (5 min)
}

TransactionManager::with_isolation(IsolationLevel::SnapshotIsolation)
```

#### Low-Contention Workloads

```rust
// Use Optimistic Concurrency Control
let occ_config = OccConfig {
    validation_strategy: ValidationStrategy::Backward,
    max_retries: 3,
    backoff_ms: 10,
};

let occ_manager = OptimisticConcurrencyControl::new(occ_config);
```

### Performance Metrics

**Key Performance Indicators:**

| Metric | Good | Acceptable | Poor |
|--------|------|------------|------|
| Commit Latency (p99) | < 5ms | < 20ms | > 50ms |
| Transaction Throughput | > 10K TPS | > 1K TPS | < 100 TPS |
| Abort Rate | < 1% | < 5% | > 10% |
| Deadlock Rate | < 0.1% | < 1% | > 5% |
| Lock Wait Time (avg) | < 1ms | < 10ms | > 50ms |
| WAL Write Latency | < 2ms | < 10ms | > 20ms |
| GC Pause Time | < 10ms | < 100ms | > 500ms |

### Monitoring Queries

```rust
// Get transaction statistics
let stats = tm.get_statistics();
println!("Commits: {}, Aborts: {}, Active: {}",
    stats.total_commits,
    stats.total_aborts,
    stats.active_transactions
);

// Get lock statistics
let lock_stats = lm.get_statistics();
println!("Lock requests: {}, Waits: {}, Timeouts: {}",
    lock_stats.lock_requests,
    lock_stats.lock_waits,
    lock_stats.lock_timeouts
);

// Get deadlock statistics
let dl_stats = detector.stats();
println!("Deadlocks found: {}, Victims aborted: {}",
    dl_stats.deadlocks_found,
    dl_stats.victims_aborted
);

// Get WAL statistics
let wal_stats = wal.get_stats();
println!("Group commits: {}, Avg group size: {:.1}",
    wal_stats.group_commits,
    wal_stats.avg_group_size
);
```

---

## Monitoring and Statistics

### Transaction Statistics

```rust
pub struct StatisticsSummary {
    pub total_commits: u64,
    pub total_aborts: u64,
    pub total_deadlocks: u64,
    pub total_timeouts: u64,
    pub active_transactions: u64,
    pub avg_commit_latency_ms: u64,
    pub abort_rate: f64,  // 0.0 to 1.0
}

// Usage
let stats = transaction_stats.get_summary();
println!("Abort rate: {:.2}%", stats.abort_rate * 100.0);
println!("Avg commit latency: {}ms", stats.avg_commit_latency_ms);
```

### Lock Statistics

```rust
pub struct LockStatisticsSummary {
    pub lock_requests: u64,
    pub lock_waits: u64,
    pub lock_timeouts: u64,
    pub deadlocks_detected: u64,
    pub lock_escalations: u64,
    pub rows_escalated: u64,
    pub avg_wait_time_ms: u64,
    pub wait_rate: f64,  // Percentage of requests that waited
}

// Usage
let lock_stats = lock_stats.get_summary();
println!("Wait rate: {:.2}%", lock_stats.wait_rate * 100.0);
println!("Escalations: {} ({} rows)",
    lock_stats.lock_escalations,
    lock_stats.rows_escalated
);
```

### WAL Statistics

```rust
pub struct WALStats {
    pub total_records: u64,
    pub total_bytes: u64,
    pub group_commits: u64,
    pub individual_commits: u64,
    pub avg_group_size: f64,
    pub fsyncs: u64,
    pub avg_flush_time_ms: f64,
    pub vectored_writes: u64,
    pub hardware_crc_ops: u64,
}

// Usage
let wal_stats = wal.get_stats();
println!("Group commit ratio: {:.1}x",
    wal_stats.avg_group_size
);
println!("Avg flush time: {:.2}ms",
    wal_stats.avg_flush_time_ms
);
```

### MVCC Statistics

```rust
pub struct MVCCStats {
    pub total_versions: u64,
    pub active_versions: u64,
    pub deleted_versions: u64,
    pub gc_runs: u64,
    pub versions_collected: u64,
    pub read_requests: u64,
    pub write_requests: u64,
    pub snapshot_conflicts: u64,
}

// Usage
let mvcc_stats = mvcc.get_stats();
println!("GC efficiency: {:.1}%",
    (mvcc_stats.versions_collected as f64 / mvcc_stats.total_versions as f64) * 100.0
);
```

### Recovery Statistics

```rust
pub struct RecoveryStats {
    pub recovery_runs: u64,
    pub last_recovery_time_ms: u64,
    pub analysis_time_ms: u64,
    pub redo_time_ms: u64,
    pub undo_time_ms: u64,
    pub records_analyzed: u64,
    pub records_redone: u64,
    pub records_undone: u64,
    pub transactions_recovered: u64,
    pub transactions_rolled_back: u64,
}
```

---

## Error Handling

### Error Types

```rust
pub enum TransactionError {
    // Lock errors
    LockTimeout { txn_id, resource, lock_mode },
    LockConflict { requesting_txn, holding_txn, resource, ... },
    Deadlock { cycle, victim },

    // State errors
    TransactionNotFound(TransactionId),
    AlreadyCommitted(TransactionId),
    AlreadyAborted(TransactionId),
    InvalidStateTransition { txn_id, from, to },

    // Validation errors
    ValidationFailed { txn_id, key },
    IsolationLevelMismatch { txn_id, required, actual },

    // I/O errors
    WalWriteError(io::Error),
    WalReadError(io::Error),

    // Recovery errors
    RecoveryFailed(String),
    RedoFailed { txn_id, lsn, reason },
    UndoFailed { txn_id, lsn, reason },
}
```

### Error Utilities

```rust
impl TransactionError {
    // Check if error is retriable
    pub fn is_retriable(&self) -> bool;

    // Check if error is deadlock-related
    pub fn is_deadlock(&self) -> bool;

    // Check if error is lock-related
    pub fn is_lock_error(&self) -> bool;

    // Get associated transaction ID
    pub fn transaction_id(&self) -> Option<TransactionId>;
}
```

### Retry Logic

```rust
fn execute_with_retry<F, T>(
    mut operation: F,
    max_retries: usize,
) -> TransactionResult<T>
where
    F: FnMut() -> TransactionResult<T>,
{
    let mut attempts = 0;
    loop {
        match operation() {
            Ok(result) => return Ok(result),
            Err(err) if err.is_retriable() && attempts < max_retries => {
                attempts += 1;
                std::thread::sleep(Duration::from_millis(10 * attempts as u64));
                continue;
            }
            Err(err) => return Err(err),
        }
    }
}

// Usage
let result = execute_with_retry(|| {
    let txn_id = tm.begin()?;
    // ... operations ...
    tm.commit(txn_id)
}, 3)?;
```

---

## API Reference

### TransactionManager

```rust
impl TransactionManager {
    pub fn new() -> Self;
    pub fn with_isolation(isolation: IsolationLevel) -> Self;
    pub fn with_lock_manager(lm: Arc<LockManager>) -> Self;

    pub fn begin(&self) -> TransactionResult<TransactionId>;
    pub fn begin_with_isolation(&self, isolation: IsolationLevel)
        -> TransactionResult<TransactionId>;
    pub fn begin_readonly(&self) -> TransactionResult<TransactionId>;

    pub fn commit(&self, txn_id: TransactionId) -> TransactionResult<()>;
    pub fn abort(&self, txn_id: TransactionId) -> TransactionResult<()>;

    pub fn get_transaction(&self, txn_id: TransactionId) -> Option<Transaction>;
    pub fn get_state(&self, txn_id: TransactionId) -> Option<TransactionState>;
    pub fn is_active(&self, txn_id: TransactionId) -> bool;

    pub fn record_read(&self, txn_id: TransactionId, key: String);
    pub fn record_write(&self, txn_id: TransactionId, key: String);
    pub fn get_read_set(&self, txn_id: TransactionId) -> Vec<String>;
    pub fn get_write_set(&self, txn_id: TransactionId) -> Vec<String>;

    pub fn abort_timed_out_transactions(&self) -> TransactionResult<usize>;
    pub fn is_timed_out(&self, txn_id: TransactionId) -> bool;
    pub fn get_transaction_age(&self, txn_id: TransactionId) -> Option<Duration>;
}
```

### LockManager

```rust
impl LockManager {
    pub fn new() -> Self;

    pub fn acquire_lock(
        &self,
        txn_id: TransactionId,
        resource: String,
        mode: LockMode,
    ) -> TransactionResult<()>;

    pub fn acquire_lock_with_timeout(
        &self,
        txn_id: TransactionId,
        resource: String,
        mode: LockMode,
        timeout: Duration,
    ) -> TransactionResult<()>;

    pub fn try_acquire_lock(
        &self,
        txn_id: TransactionId,
        resource: String,
        mode: LockMode,
    ) -> TransactionResult<bool>;

    pub fn release_lock(&self, txn_id: TransactionId, resource: &str)
        -> TransactionResult<()>;
    pub fn release_all_locks(&self, txn_id: TransactionId)
        -> TransactionResult<()>;

    pub fn get_locks(&self, txn_id: TransactionId) -> HashSet<String>;
    pub fn is_locked(&self, resource: &str) -> bool;
    pub fn get_holders(&self, resource: &str) -> Vec<(TransactionId, LockMode)>;
}
```

### WALManager

```rust
impl WALManager {
    pub fn new(wal_path: PathBuf, config: WALConfig) -> Result<Self>;

    pub async fn append(&self, record: LogRecord) -> Result<LSN>;
    pub async fn flush(&self) -> Result<()>;
    pub fn read_from(&self, start_lsn: LSN) -> Result<Vec<WALEntry>>;
    pub fn read_all(&self) -> Result<Vec<WALEntry>>;

    pub fn get_current_lsn(&self) -> LSN;
    pub fn get_flushed_lsn(&self) -> LSN;
    pub fn get_stats(&self) -> WALStats;
}
```

### MVCCManager

```rust
impl MVCCManager<K, V> {
    pub fn new(config: MVCCConfig) -> Self;

    pub fn begin_snapshot(&self, txn_id: TransactionId) -> HybridTimestamp;
    pub fn end_snapshot(&self, txn_id: TransactionId);

    pub fn read(&self, key: &K, read_ts: &HybridTimestamp) -> Result<Option<V>>;
    pub fn write(&self, key: K, value: V, txn_id: TransactionId,
        timestamp: HybridTimestamp) -> Result<()>;
    pub fn delete(&self, key: &K, txn_id: TransactionId,
        timestamp: HybridTimestamp) -> Result<bool>;

    pub fn garbage_collect(&self) -> Result<usize>;
    pub fn get_stats(&self) -> MVCCStats;
    pub fn global_version_count(&self) -> u64;
}
```

### DeadlockDetector

```rust
impl DeadlockDetector {
    pub fn new(detection_interval: Duration) -> Self;
    pub fn with_config(config: DeadlockDetectorConfig) -> Self;

    pub fn add_wait(&self, waiting_txn: TransactionId, holding_txn: TransactionId);
    pub fn remove_wait(&self, txn_id: TransactionId);
    pub fn remove_wait_edge(&self, waiting: TransactionId, holding: TransactionId);

    pub fn detect_deadlock(&self) -> Option<Vec<TransactionId>>;
    pub fn force_detect(&self) -> Option<Vec<TransactionId>>;
    pub fn select_victim(&self, cycle: &[TransactionId]) -> TransactionId;

    pub fn record_victim_aborted(&self);
    pub fn stats(&self) -> DeadlockStats;
}
```

---

## Appendix: Critical Fixes Documentation

### EA2-RACE-1: Atomic Commit State Transition

**Issue:** Race condition during commit where other threads could observe intermediate state

**Fix:** Hold active_txns write lock for entire commit operation

**Impact:** Prevents phantom read of uncommitted data

### EA2-V2: Lock Timeout Support

**Issue:** Indefinite blocking on lock acquisition

**Fix:** Added timeout parameter (default 30s) to all lock operations

**Impact:** Prevents resource exhaustion from stuck transactions

### EA2-V3: Proper Lock Upgrade Handling

**Issue:** Deadlock during lock upgrade with multiple holders

**Fix:** Check holder count before upgrading, wait if necessary

**Impact:** Reduces upgrade-related deadlocks

### EA2-V4: Group Commit Entry Count Limit

**Issue:** Unbounded memory growth in group commit buffer

**Fix:** Added MAX_GROUP_COMMIT_ENTRIES (10K) limit

**Impact:** Prevents OOM under high transaction rates

### EA2-RACE-3: Atomic WAL Truncate

**Issue:** Concurrent writes lost during WAL truncation

**Fix:** Hold buffer and LSN locks for entire truncate operation

**Impact:** Ensures durability during checkpoint truncation

### EA2-ERR-1: Lock Wait Queue with Condition Variables

**Issue:** Lock acquisition returned error instead of waiting

**Fix:** Added wait queue and condition variables for proper blocking

**Impact:** Improved concurrency and reduced spurious errors

---

## Conclusion

The RustyDB v0.5.1 transaction layer provides enterprise-grade ACID compliance with multiple concurrency control strategies optimized for diverse workloads. Key strengths include:

- **Proven algorithms**: ARIES, MVCC, 2PL
- **High performance**: Group commit, hardware checksums, lock-free structures
- **Flexibility**: Multiple isolation levels, OCC/2PL choice
- **Robustness**: Comprehensive error handling, automatic recovery
- **Observability**: Detailed statistics and monitoring

For production deployment, carefully tune configuration parameters based on workload characteristics and monitor key performance metrics.

---

**Document Version:** 1.0
**RustyDB Version:** 0.5.1
**Last Updated:** December 25, 2025
**Authors:** Enterprise Documentation Team
**Status:** Production Ready
