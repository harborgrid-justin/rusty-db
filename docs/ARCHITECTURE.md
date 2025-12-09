# RustyDB Architecture Documentation

**Last Updated**: 2025-12-09
**Version**: 0.1.0

## Table of Contents

1. [Overview](#overview)
2. [System Architecture](#system-architecture)
3. [Layer-by-Layer Design](#layer-by-layer-design)
4. [Data Flow](#data-flow)
5. [Concurrency Model](#concurrency-model)
6. [Storage Architecture](#storage-architecture)
7. [Transaction Management](#transaction-management)
8. [Query Processing Pipeline](#query-processing-pipeline)
9. [Security Architecture](#security-architecture)
10. [High Availability & Clustering](#high-availability--clustering)
11. [Performance Optimizations](#performance-optimizations)
12. [Module Dependencies](#module-dependencies)

---

## Overview

RustyDB is an enterprise-grade, ACID-compliant database management system built from scratch in Rust. It implements a layered architecture inspired by PostgreSQL, Oracle, and modern database research, with a focus on:

- **Safety**: Rust's ownership model eliminates entire classes of bugs
- **Performance**: Zero-cost abstractions, SIMD acceleration, lock-free data structures
- **Scalability**: Async I/O, parallel query execution, distributed clustering
- **Security**: 10 specialized security modules with defense-in-depth
- **Enterprise Features**: RAC-like clustering, advanced replication, in-database ML

### Design Principles

1. **Layered Architecture**: Clear separation of concerns with well-defined interfaces
2. **Pluggable Components**: Modular design allowing component replacement
3. **Async-First**: Non-blocking I/O throughout the stack
4. **Lock-Free Where Possible**: Minimize contention with lock-free data structures
5. **Type Safety**: Leverage Rust's type system for correctness
6. **Error Handling**: Unified error handling with `Result<T, DbError>`

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Applications                       │
└─────────────────────────────────────────────────────────────────┘
                                │
                ┌───────────────┴───────────────┐
                │                               │
        ┌───────▼──────┐               ┌───────▼──────┐
        │  CLI Client  │               │  REST/GraphQL│
        │              │               │     API      │
        └───────┬──────┘               └───────┬──────┘
                │                               │
                └───────────────┬───────────────┘
                                │
┌─────────────────────────────────────────────────────────────────┐
│                         Network Layer                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  TCP Server  │  │   Protocol   │  │ Connection   │          │
│  │              │  │   Handler    │  │     Pool     │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                                │
┌─────────────────────────────────────────────────────────────────┐
│                      Query Processing Layer                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  SQL Parser  │→ │Query Planner │→ │  Optimizer   │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                            │                                     │
│                    ┌───────▼────────┐                           │
│                    │  Query Executor │                           │
│                    └───────┬────────┘                           │
└─────────────────────────────────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
┌───────▼──────┐        ┌──────▼──────┐        ┌──────▼──────┐
│  Transaction │        │    Index    │        │  Catalog    │
│   Manager    │        │   Manager   │        │  Manager    │
└───────┬──────┘        └──────┬──────┘        └──────┬──────┘
        │                      │                       │
        └──────────────────────┼───────────────────────┘
                               │
┌─────────────────────────────────────────────────────────────────┐
│                         Storage Layer                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ Buffer Pool  │  │    Memory    │  │   I/O Layer  │          │
│  │   Manager    │  │  Management  │  │  (io_uring)  │          │
│  └───────┬──────┘  └──────┬───────┘  └──────┬───────┘          │
└──────────┼─────────────────┼─────────────────┼──────────────────┘
           │                 │                 │
           └─────────────────┼─────────────────┘
                             │
                    ┌────────▼─────────┐
                    │   Disk Storage   │
                    │  (Page-based)    │
                    └──────────────────┘
```

---

## Layer-by-Layer Design

### 1. Core Foundation Layer

**Purpose**: Provide common types, error handling, and utility functions

**Components**:
- `error.rs`: Unified error type (`DbError`) with conversions
- `common.rs`: Shared types (`TransactionId`, `PageId`, etc.) and traits (`Component`, `Transactional`)
- Core traits for lifecycle management

**Key Design Decisions**:
- Single error type throughout the codebase
- Type aliases for clarity (e.g., `type Result<T> = std::result::Result<T, DbError>`)
- Traits for component lifecycle (initialize, shutdown, health check)

### 2. Storage Layer

**Purpose**: Manage persistent storage with efficient I/O

#### Components

**Page Management** (`storage/page.rs`):
- Fixed 4KB pages (configurable)
- Page header with metadata (page ID, LSN, checksum)
- Slotted page layout for variable-length records
- Free space tracking

**Disk Manager** (`storage/disk.rs`):
- File-based storage
- Page read/write operations
- File growth and management
- Direct I/O support for performance

**Buffer Pool** (`buffer/manager.rs`):
- In-memory page cache
- Multiple eviction policies: LRU, CLOCK, 2Q, LRU-K, LIRS, ARC
- Lock-free page table for fast lookups
- Page pinning mechanism
- Write-ahead logging integration

**Memory Management** (`memory/`):
- Slab allocator for fixed-size allocations
- Arena allocator for temporary allocations
- Large object allocator
- Memory pressure handling

**I/O Layer** (`io/`):
- Async I/O with Tokio
- Platform-specific optimizations (io_uring on Linux, IOCP on Windows)
- Ring buffers for I/O batching
- Direct I/O bypass

**Design Patterns**:
- Page-oriented storage (like PostgreSQL, Oracle)
- Pluggable eviction policies (Strategy pattern)
- Lock-free page table for concurrency
- RAII for page pinning

### 3. Transaction Layer

**Purpose**: Ensure ACID properties with MVCC

**Components**:
- Transaction manager: Lifecycle, isolation
- Lock manager: Two-phase locking, deadlock detection
- MVCC: Multi-version concurrency control
- Write-Ahead Log (WAL): Durability and recovery

**Transaction Lifecycle**:
1. BEGIN: Assign transaction ID, snapshot
2. EXECUTE: Acquire locks, modify data with versioning
3. COMMIT: Write WAL, release locks, make visible
4. ROLLBACK: Release locks, mark versions invalid

**Isolation Levels**:
- Read Uncommitted
- Read Committed (default)
- Repeatable Read
- Serializable (via snapshot isolation + conflict detection)

**Deadlock Detection**:
- Waits-for graph
- Cycle detection algorithm
- Deadlock victim selection (youngest transaction)

### 4. Query Processing Layer

**Purpose**: Parse, optimize, and execute SQL queries

#### Pipeline

```
SQL Text → Parser → AST → Planner → Logical Plan → Optimizer →
Physical Plan → Executor → Results
```

**Parser** (`parser/`):
- SQL parsing using `sqlparser` crate
- AST generation
- Syntax validation

**Planner** (`execution/planner.rs`):
- Converts AST to logical plan
- Resolves table and column names
- Type checking
- Initial plan generation

**Optimizer** (`execution/optimizer.rs`, `optimizer_pro/`):
- **Basic Optimizer**:
  - Predicate pushdown
  - Constant folding
  - Expression simplification
- **Advanced Optimizer** (`optimizer_pro/`):
  - Cost-based optimization
  - Join order optimization
  - Index selection
  - Cardinality estimation
  - Adaptive query execution
  - SQL plan baselines

**Executor** (`execution/executor.rs`):
- Volcano-style iterator model
- Operator implementations (Scan, Join, Aggregate, Sort)
- Vectorized execution
- Parallel execution
- Pipeline breakers (sort, hash)

**Query Operators**:
- **Scan**: Sequential, Index, Bitmap
- **Join**: Nested Loop, Hash Join, Merge Join
- **Aggregate**: Hash Aggregate, Sort Aggregate
- **Sort**: External sort with disk spilling
- **Limit**: Top-N optimization

### 5. Index Layer

**Purpose**: Accelerate data access with multiple index types

**Index Types**:

1. **B-Tree Index** (`index/btree`):
   - Balanced tree structure
   - O(log n) lookup, insert, delete
   - Range queries
   - Supports unique and non-unique

2. **LSM-Tree Index** (`index/lsm`):
   - Write-optimized structure
   - Memtable + SSTables
   - Compaction strategies
   - High write throughput

3. **Hash Index** (`index/hash`):
   - O(1) equality lookups
   - No range query support
   - Extensible hashing

4. **Spatial Index** (`index/spatial`):
   - R-Tree for geospatial data
   - Bounding box queries
   - Nearest neighbor search

5. **Full-Text Index** (`index/fulltext`):
   - Inverted index
   - Tokenization and stemming
   - Ranking algorithms

6. **Bitmap Index** (`index/bitmap`):
   - Efficient for low-cardinality columns
   - Bitwise operations for AND/OR
   - Run-length encoding

### 6. Network Layer

**Purpose**: Handle client connections and protocol

**Components**:

**TCP Server** (`network/server.rs`):
- Async TCP listener (Tokio)
- Connection multiplexing
- Keep-alive and timeouts

**Protocol Handler** (`network/protocol.rs`):
- PostgreSQL wire protocol compatible
- Message framing
- Authentication flow
- Query/response cycle

**Connection Pool** (`pool/connection_pool.rs`):
- Min/max connection limits
- Connection recycling
- Health checks
- Timeout management

**Advanced Features**:
- Connection multiplexing
- Query pipelining
- Prepared statements
- Binary protocol

### 7. Security Layer

**Purpose**: Multi-layered defense-in-depth security

**10 Security Modules**:

1. **Memory Hardening**: Guard pages, canaries, safe allocation
2. **Buffer Overflow Protection**: Bounds checking, integer overflow detection
3. **Insider Threat Detection**: Behavioral analytics, anomaly detection
4. **Network Hardening**: DDoS protection, rate limiting, TLS
5. **Injection Prevention**: SQL/command injection defense, input validation
6. **Auto-Recovery**: Failure detection, automatic recovery
7. **Circuit Breaker**: Cascading failure prevention
8. **Encryption Engine**: TDE, column encryption, key management
9. **Secure Garbage Collection**: Memory sanitization, cryptographic erasure
10. **Security Core**: Policy engine, compliance, audit

**Authentication & Authorization**:
- RBAC (Role-Based Access Control)
- MFA (Multi-Factor Authentication)
- Password hashing (Argon2id)
- Session management
- Fine-grained access control

See `docs/SECURITY_ARCHITECTURE.md` for detailed security design.

---

## Data Flow

### Query Execution Flow

```
1. Client sends SQL query
   ↓
2. Network layer receives request
   ↓
3. Authentication/authorization check
   ↓
4. Parser converts SQL to AST
   ↓
5. Planner creates logical plan
   ↓
6. Optimizer generates physical plan (with cost estimation)
   ↓
7. Executor runs plan:
   - Acquires locks via Transaction Manager
   - Reads pages via Buffer Pool
   - Applies operators (scan, join, aggregate, etc.)
   ↓
8. Results sent to client
   ↓
9. Transaction commits (WAL, release locks)
```

### Write Path

```
1. INSERT/UPDATE/DELETE query
   ↓
2. Transaction Manager assigns TxnID
   ↓
3. Lock Manager acquires row locks
   ↓
4. Buffer Pool fetches target pages (pin)
   ↓
5. MVCC creates new version with TxnID
   ↓
6. WAL writes log record
   ↓
7. Modified pages marked dirty
   ↓
8. COMMIT:
   - WAL fsync (durability)
   - Locks released
   - Version visible
   ↓
9. Buffer Pool writer flushes dirty pages asynchronously
```

### Read Path

```
1. SELECT query
   ↓
2. Snapshot assigned (MVCC)
   ↓
3. Executor requests rows
   ↓
4. Index lookup (if applicable)
   ↓
5. Buffer Pool:
   - Check page table
   - If hit: return page
   - If miss: read from disk, add to pool
   ↓
6. MVCC visibility check (based on snapshot)
   ↓
7. Filter/project rows
   ↓
8. Return results to client
```

---

## Concurrency Model

### Thread Model

**Tokio Async Runtime**:
- Thread pool for CPU-bound tasks
- Async tasks for I/O-bound operations
- Work-stealing scheduler

**Component Threading**:
- **Network**: Async tasks per connection
- **Query Executor**: Parallel operators with thread pool
- **Buffer Pool**: Lock-free page table, background writer thread
- **WAL**: Dedicated WAL writer thread
- **Checkpoint**: Background checkpoint thread

### Synchronization Primitives

**Standard Primitives**:
- `Arc<Mutex<T>>`: Shared mutable state
- `Arc<RwLock<T>>`: Read-heavy workloads
- Atomic types: Counters, flags

**Lock-Free Data Structures** (`concurrent/`):
- Lock-free queue (Michael-Scott)
- Lock-free stack
- Lock-free hash map
- Lock-free skip list
- Epoch-based memory reclamation

### Deadlock Prevention

1. **Lock Ordering**: Acquire locks in consistent order
2. **Timeout**: Transaction aborts if lock timeout
3. **Deadlock Detection**: Waits-for graph cycle detection
4. **Deadlock Resolution**: Abort youngest transaction

---

## Storage Architecture

### Page Layout

```
┌──────────────────────────────────────────────┐
│          Page Header (32 bytes)              │
│  - Page ID (8 bytes)                         │
│  - LSN (8 bytes)                             │
│  - Checksum (4 bytes)                        │
│  - Free space offset (4 bytes)               │
│  - Slot count (4 bytes)                      │
│  - Flags (4 bytes)                           │
├──────────────────────────────────────────────┤
│          Slot Array (grows down)             │
│  - Slot 0: (offset, length)                  │
│  - Slot 1: (offset, length)                  │
│  - ...                                       │
├──────────────────────────────────────────────┤
│              Free Space                      │
├──────────────────────────────────────────────┤
│       Tuple Data (grows up)                  │
│  - Tuple N                                   │
│  - ...                                       │
│  - Tuple 1                                   │
│  - Tuple 0                                   │
└──────────────────────────────────────────────┘
```

### File Organization

```
data/
├── base/                   # Database data files
│   ├── table_1.dat        # Table heap files
│   ├── table_1_idx_1.dat  # Index files
│   └── ...
├── wal/                   # Write-Ahead Log
│   ├── 000000010000000000000001
│   ├── 000000010000000000000002
│   └── ...
└── pg_control             # Control file (cluster state)
```

### Buffer Pool Architecture

```
┌─────────────────────────────────────────────┐
│         Page Table (Lock-Free HashMap)      │
│         PageID → Frame Index                │
└─────────────────────────────────────────────┘
                    │
        ┌───────────┴───────────┐
        │                       │
┌───────▼───────┐       ┌───────▼───────┐
│  Frame Pool   │       │ Eviction List │
│  (Fixed Array)│       │  (Policy-    │
│               │       │   dependent)  │
│ - Frame 0     │       │               │
│ - Frame 1     │       │ - CLOCK       │
│ - ...         │       │ - LRU         │
│ - Frame N     │       │ - ARC         │
└───────┬───────┘       └───────────────┘
        │
        │ (on miss)
        ▼
┌─────────────────────────────────────────────┐
│              Disk I/O                        │
└─────────────────────────────────────────────┘
```

---

## Transaction Management

### MVCC Implementation

**Version Chain**:
Each tuple has multiple versions linked by `next_version` pointer.

```
Tuple V1 (TxnID=100, deleted_by=200) → Tuple V2 (TxnID=200, deleted_by=NULL)
```

**Visibility Rules**:
- Transaction sees versions created by committed transactions < snapshot
- Transaction sees its own uncommitted changes
- Transaction does not see versions created by concurrent transactions

**Garbage Collection**:
- Background vacuum process
- Removes versions no longer visible to any transaction
- Reclaims space in pages

### Write-Ahead Logging (WAL)

**Log Record Types**:
- INSERT, UPDATE, DELETE (data modifications)
- COMMIT, ABORT (transaction boundaries)
- CHECKPOINT (recovery points)

**WAL Protocol**:
1. Modify page in buffer pool
2. Write WAL record with LSN
3. Set page LSN = WAL record LSN
4. Mark page dirty
5. On COMMIT: fsync WAL

**Recovery (ARIES)**:
1. **Analysis**: Scan WAL, identify dirty pages
2. **Redo**: Replay all operations from checkpoint
3. **Undo**: Rollback uncommitted transactions

---

## Query Processing Pipeline

### Optimization Techniques

**Logical Optimizations**:
- Predicate pushdown
- Projection pushdown
- Constant folding
- Expression simplification
- Subquery unnesting
- Join elimination

**Physical Optimizations**:
- Join order optimization (dynamic programming)
- Index selection
- Access path selection
- Parallel execution planning
- Adaptive execution (re-optimize based on runtime stats)

### Join Algorithms

1. **Nested Loop Join**: O(n*m), good for small tables
2. **Hash Join**: O(n+m), best for large tables
3. **Merge Join**: O(n log n + m log m), good for sorted inputs

### Aggregation Algorithms

1. **Hash Aggregate**: Hash table, good for distinct aggregates
2. **Sort Aggregate**: Sort then aggregate, good for GROUP BY

---

## Security Architecture

See `docs/SECURITY_ARCHITECTURE.md` for comprehensive details.

**Defense-in-Depth Layers**:
1. Network security (TLS, rate limiting, DDoS protection)
2. Authentication (MFA, password policies)
3. Authorization (RBAC, FGAC)
4. Data protection (TDE, column encryption)
5. Audit logging (tamper-proof, chained hashes)
6. Anomaly detection (behavioral analytics)
7. Auto-recovery (failure detection, self-healing)
8. Memory security (guard pages, sanitization)

---

## High Availability & Clustering

### Replication

**Modes**:
- **Synchronous**: Wait for replica acknowledgment before commit
- **Asynchronous**: Don't wait for replica (higher performance)
- **Semi-Synchronous**: Wait for at least one replica

**Architecture**:
```
Primary Node
    │
    ├─→ Synchronous Replica 1 (ack required)
    ├─→ Synchronous Replica 2 (ack required)
    └─→ Async Replica (no ack)
```

### RAC (Real Application Clusters)

**Cache Fusion**:
- Shared-disk architecture
- Global Resource Directory (GRD)
- Block shipping (send modified blocks to requesting node)
- No disk writes for inter-node communication

**Components**:
- Global Cache Service (GCS)
- Global Enqueue Service (GES)
- Cluster Interconnect (high-speed network)

### Raft Consensus

For metadata management and cluster coordination:
- Leader election
- Log replication
- Safety guarantees

---

## Performance Optimizations

### SIMD Acceleration

**Use Cases**:
- Filtering (WHERE clauses)
- Aggregation (SUM, AVG, etc.)
- Hash computation
- String operations

**Instructions**:
- AVX2 (256-bit vectors)
- AVX-512 (512-bit vectors)

### Lock-Free Data Structures

**Benefits**:
- No lock contention
- Near-linear scalability
- Reduced context switching

**Implementations**:
- Page table in buffer pool
- Transaction ID allocation
- Statistics counters

### Prefetching

**Strategies**:
- Sequential prefetching (detect sequential scans)
- Index prefetching (prefetch child nodes)
- Adaptive prefetching (learn access patterns)

---

## Module Dependencies

```
common ←──────────────┐
   ↑                  │
error ←──────────────┐│
   ↑                 ││
   │                 ││
storage ←─────────────┤
buffer  ←─────────────┤
memory  ←─────────────┤
io      ←─────────────┤
   ↑                 ││
   │                 ││
transaction ←─────────┤
index       ←─────────┤
   ↑                 ││
   │                 ││
parser      ←─────────┤
execution   ←─────────┤
optimizer   ←─────────┤
   ↑                 ││
   │                 ││
network     ←─────────┤
api         ←─────────┤
pool        ←─────────┤
security    ←─────────┘
```

**Dependency Rules**:
- Lower layers don't depend on higher layers
- All modules depend on `error` and `common`
- Avoid circular dependencies
- Use traits for dependency inversion

---

## Future Enhancements

1. **Distributed Transactions**: Two-phase commit across nodes
2. **Columnar Storage**: Optimized for analytics
3. **Adaptive Indexing**: Auto-create indexes based on workload
4. **Query Compilation**: JIT compile hot queries
5. **GPU Acceleration**: Offload analytics to GPU
6. **Time-Series Optimization**: Specialized storage for time-series
7. **Graph Processing**: Native graph query execution

---

## References

- **PostgreSQL Internals**: Inspiration for storage and MVCC
- **Oracle Architecture**: RAC, advanced replication, security
- **CMU Database Systems Course**: Query optimization, concurrency control
- **ARIES**: Recovery algorithm (Mohan et al.)
- **Rust Performance Book**: Zero-cost abstractions, SIMD

---

*This document is continuously updated as the architecture evolves.*
