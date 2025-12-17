# RustyDB Master Architecture & Data Flow Diagram

**Document:** Master Architecture Overview
**Version:** 1.0
**Date:** 2025-12-17
**Author:** Agent #9 - Coordination & Cross-Cutting Analysis
**Branch:** claude/data-flow-diagrams-bxsJ7

---

## Table of Contents
1. [High-Level System Architecture](#high-level-system-architecture)
2. [Module Dependency Graph](#module-dependency-graph)
3. [Data Flow: Query Execution](#data-flow-query-execution)
4. [Data Flow: Transaction Lifecycle](#data-flow-transaction-lifecycle)
5. [Data Flow: Replication](#data-flow-replication)
6. [Integration Points Matrix](#integration-points-matrix)
7. [Critical Paths](#critical-paths)
8. [Component Lifecycle](#component-lifecycle)

---

## High-Level System Architecture

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                          RustyDB Enterprise Database                           │
│                         Oracle-Compatible DBMS in Rust                         │
└───────────────────────────────────────────────────────────────────────────────┘
                                        │
        ┌───────────────────────────────┼───────────────────────────────┐
        │                               │                               │
        ▼                               ▼                               ▼
┌────────────────┐            ┌────────────────┐            ┌────────────────┐
│  Client APIs   │            │  Network Layer │            │ Admin/Monitor  │
├────────────────┤            ├────────────────┤            ├────────────────┤
│ • REST API     │            │ • TCP Server   │            │ • GraphQL      │
│ • GraphQL      │◄───────────│ • Wire Proto   │───────────►│ • Monitoring   │
│ • WebSocket    │            │ • Connection   │            │ • Metrics      │
│ • CLI Client   │            │   Pool         │            │ • Dashboard    │
└────────────────┘            └────────────────┘            └────────────────┘
                                        │
        ┌───────────────────────────────┼───────────────────────────────┐
        │                               │                               │
        ▼                               ▼                               ▼
┌────────────────┐            ┌────────────────┐            ┌────────────────┐
│  SQL Parser    │            │  Orchestrator  │            │   Security     │
├────────────────┤            ├────────────────┤            ├────────────────┤
│ • SQL Parsing  │            │ • Service Reg  │            │ • RBAC         │
│ • AST Gen      │───────────►│ • Actor System │◄───────────│ • Encryption   │
│ • Validation   │            │ • Health Check │            │ • Audit        │
└────────────────┘            │ • Circuit Brk  │            │ • TDE          │
        │                     └────────────────┘            └────────────────┘
        │                               │                               │
        ▼                               │                               ▼
┌────────────────┐                      │                     ┌────────────────┐
│ Query Planner  │                      │                     │  Workload Mgmt │
├────────────────┤                      │                     ├────────────────┤
│ • Logical Plan │                      │                     │ • SQL Tuning   │
│ • Physical Plan│◄─────────────────────┼─────────────────────│ • AWR Snapshots│
│ • Cost Model   │                      │                     │ • Diagnostics  │
│ • Optimizer    │                      │                     │ • Perf Hub     │
└────────────────┘                      │                     └────────────────┘
        │                               │                               │
        ▼                               │                               ▼
┌────────────────┐                      │                     ┌────────────────┐
│ Query Executor │                      │                     │   Autonomous   │
├────────────────┤                      │                     ├────────────────┤
│ • Vectorized   │                      │                     │ • Auto-Tuning  │
│ • Parallel     │◄─────────────────────┼─────────────────────│ • Self-Healing │
│ • SIMD Ops     │                      │                     │ • Auto-Index   │
│ • Operators    │                      │                     │ • ML Workload  │
└────────────────┘                      │                     └────────────────┘
        │                               │
        │                               │
        ▼                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Transaction & Concurrency Layer                       │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │ Transaction  │  │ Lock Manager │  │  MVCC Engine │  │     WAL      │   │
│  │  Manager     │──│ (2PL, DLD)   │──│ (Snapshots)  │──│  (Durability)│   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                        │
        ┌───────────────────────────────┼───────────────────────────────┐
        │                               │                               │
        ▼                               ▼                               ▼
┌────────────────┐            ┌────────────────┐            ┌────────────────┐
│  Index Layer   │            │  Catalog Mgmt  │            │  Constraints   │
├────────────────┤            ├────────────────┤            ├────────────────┤
│ • B-Tree       │◄───────────│ • Metadata     │───────────►│ • Foreign Key  │
│ • LSM Tree     │            │ • Schemas      │            │ • Unique       │
│ • Hash Index   │            │ • Views        │            │ • Check        │
│ • Spatial      │            │ • System Cat   │            │ • Cascade      │
│ • Full-Text    │            └────────────────┘            └────────────────┘
│ • Bitmap       │                      │
└────────────────┘                      │
        │                               │
        └───────────────┬───────────────┘
                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Storage & Buffer Layer                              │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │ Buffer Pool  │  │  Page Cache  │  │ Memory Mgmt  │  │  Disk I/O    │   │
│  │  Manager     │──│ (4KB Pages)  │──│ (Slab,Arena) │──│ (Direct IO)  │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
│         │                 │                  │                  │           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │ Partitioning │  │   Columnar   │  │   LSM Tree   │  │ Compression  │   │
│  │   Manager    │  │   Storage    │  │   Storage    │  │   Engine     │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                        │
        ┌───────────────────────────────┼───────────────────────────────┐
        │                               │                               │
        ▼                               ▼                               ▼
┌────────────────┐            ┌────────────────┐            ┌────────────────┐
│  Replication   │            │  Clustering    │            │  Backup/Recov  │
├────────────────┤            ├────────────────┤            ├────────────────┤
│ • Sync/Async   │◄───────────│ • Raft         │───────────►│ • Full Backup  │
│ • Multi-Master │            │ • Sharding     │            │ • Incremental  │
│ • Logical Rep  │            │ • Failover     │            │ • PITR         │
│ • CRDT         │            │ • Cache Fusion │            │ • Flashback    │
└────────────────┘            └────────────────┘            └────────────────┘
                                        │
        ┌───────────────────────────────┼───────────────────────────────┐
        │                               │                               │
        ▼                               ▼                               ▼
┌────────────────┐            ┌────────────────┐            ┌────────────────┐
│  Specialized   │            │ Multi-Tenancy  │            │   Enterprise   │
├────────────────┤            ├────────────────┤            ├────────────────┤
│ • Graph DB     │            │ • PDB/CDB      │            │ • Blockchain   │
│ • Document DB  │◄───────────│ • Isolation    │───────────►│ • Event Proc   │
│ • Spatial DB   │            │ • Metering     │            │ • Streams      │
│ • ML Engine    │            │ • Hot Clone    │            │ • Analytics    │
│ • In-Memory    │            │ • Relocation   │            │ • Triggers     │
└────────────────┘            └────────────────┘            └────────────────┘
```

---

## Module Dependency Graph

### Layer 0: Foundation (No Dependencies)
```
┌─────────────────────────────────────────────────────────────┐
│  Foundation Layer - Core Building Blocks                     │
├─────────────────────────────────────────────────────────────┤
│  • error.rs         - Unified DbError enum                   │
│  • common.rs        - Shared types and traits                │
│  • metadata.rs      - Instance metadata                      │
│  • compat.rs        - Compatibility checking                 │
└─────────────────────────────────────────────────────────────┘
                         │
                         │ (Used by all modules)
                         ▼
```

### Layer 1: Storage & Memory (Depends on Foundation)
```
┌─────────────────────────────────────────────────────────────┐
│  Storage & Memory Layer                                      │
├─────────────────────────────────────────────────────────────┤
│  • storage/         - Page-based storage, disk I/O           │
│  • buffer/          - Buffer pool manager                    │
│  • memory/          - Slab allocator, arena allocator        │
│  • catalog/         - Metadata management                    │
│  • io/              - Cross-platform async I/O               │
│  • concurrent/      - Lock-free data structures              │
└─────────────────────────────────────────────────────────────┘
                         │
                         │ (Used by Transaction & Index)
                         ▼
```

### Layer 2: Transaction & Index (Depends on Storage)
```
┌─────────────────────────────────────────────────────────────┐
│  Transaction & Index Layer                                   │
├─────────────────────────────────────────────────────────────┤
│  • transaction/     - MVCC, lock manager, WAL                │
│  • index/           - B-Tree, LSM, spatial, full-text        │
│  • constraints/     - Foreign key, unique, check             │
│  • simd/            - SIMD-accelerated operations            │
└─────────────────────────────────────────────────────────────┘
                         │
                         │ (Used by Query Processing)
                         ▼
```

### Layer 3: Query Processing (Depends on Transaction & Index)
```
┌─────────────────────────────────────────────────────────────┐
│  Query Processing Layer                                      │
├─────────────────────────────────────────────────────────────┤
│  • parser/          - SQL parsing                            │
│  • execution/       - Query executor (vectorized, parallel)  │
│  • optimizer_pro/   - Cost-based optimizer                   │
│  • procedures/      - Stored procedures (PL/SQL-like)        │
│  • triggers/        - Database triggers                      │
└─────────────────────────────────────────────────────────────┘
                         │
                         │ (Used by Network & API)
                         ▼
```

### Layer 4: Network & API (Depends on Query Processing)
```
┌─────────────────────────────────────────────────────────────┐
│  Network & API Layer                                         │
├─────────────────────────────────────────────────────────────┤
│  • network/         - TCP server, wire protocol              │
│  • api/             - REST, GraphQL APIs                     │
│  • pool/            - Connection pooling                     │
│  • websocket/       - WebSocket support                      │
│  • networking/      - Load balancing, discovery              │
└─────────────────────────────────────────────────────────────┘
```

### Layer 5: Enterprise Features (Cross-Layer Dependencies)
```
┌─────────────────────────────────────────────────────────────┐
│  Enterprise Layer - Advanced Features                        │
├─────────────────────────────────────────────────────────────┤
│  • security/        - RBAC, encryption, audit                │
│  • replication/     - Multi-datacenter replication           │
│  • clustering/      - Raft, sharding, failover               │
│  • rac/             - Real Application Clusters              │
│  • backup/          - Full/incremental backup, PITR          │
│  • monitoring/      - Metrics, profiling, governance         │
│  • workload/        - AWR, SQL tuning, diagnostics           │
│  • autonomous/      - Self-tuning, self-healing, auto-index  │
└─────────────────────────────────────────────────────────────┘
```

### Layer 6: Specialized Engines (Optional Components)
```
┌─────────────────────────────────────────────────────────────┐
│  Specialized Engines - Domain-Specific Features              │
├─────────────────────────────────────────────────────────────┤
│  • graph/           - Property graph database                │
│  • document_store/  - JSON/BSON document database            │
│  • spatial/         - Geospatial database (R-Tree)           │
│  • ml/              - Machine learning models                │
│  • ml_engine/       - In-database ML execution               │
│  • inmemory/        - In-memory column store                 │
│  • analytics/       - OLAP, materialized views               │
└─────────────────────────────────────────────────────────────┘
```

### Layer 7: Cross-Cutting Infrastructure
```
┌─────────────────────────────────────────────────────────────┐
│  Cross-Cutting Infrastructure - System Coordination          │
├─────────────────────────────────────────────────────────────┤
│  • orchestration/   - Service registry, actor system         │
│  • flashback/       - Time-travel queries, PITR              │
│  • blockchain/      - Immutable audit trail                  │
│  • multitenant/     - PDB/CDB multi-tenancy                  │
│  • event_processing/- Complex event processing (CEP)         │
│  • streams/         - CDC, pub/sub                           │
│  • operations/      - Resource management                    │
│  • enterprise/      - Enterprise runtime                     │
└─────────────────────────────────────────────────────────────┘
```

---

## Data Flow: Query Execution

### SELECT Query Execution Path

```
┌──────────────────────────────────────────────────────────────────────┐
│ 1. Client Request                                                     │
│    ┌─────────────────────────────────────────────────────────┐       │
│    │ SELECT u.name, o.total                                  │       │
│    │ FROM users u JOIN orders o ON u.id = o.user_id          │       │
│    │ WHERE u.age > 18 AND o.total > 100                      │       │
│    │ ORDER BY o.total DESC LIMIT 10;                         │       │
│    └─────────────────────────────────────────────────────────┘       │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 2. Network Layer (src/network/)                                      │
│    • TCP server receives query                                       │
│    • Wire protocol deserialization                                   │
│    • Connection pool: session assignment                             │
│    • Authentication & authorization check                            │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 3. Security Check (src/security/)                                    │
│    • RBAC: Check SELECT permission on users, orders                  │
│    • Fine-grained access control (FGAC)                              │
│    • Audit logging: Record query execution                           │
│    • Injection prevention: SQL injection scan                        │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 4. SQL Parser (src/parser/)                                          │
│    • Lexical analysis (tokenization)                                 │
│    • Syntax analysis (AST generation)                                │
│    • Semantic analysis (schema validation)                           │
│    Output: Abstract Syntax Tree (AST)                                │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 5. Catalog Lookup (src/catalog/)                                     │
│    • Retrieve schema for 'users' table                               │
│    • Retrieve schema for 'orders' table                              │
│    • Verify column existence: u.name, u.age, o.total                 │
│    • Check index availability for u.age, o.total                     │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 6. Query Planner (src/execution/planner.rs)                          │
│    • Logical plan generation                                         │
│    • Predicate pushdown (age > 18, total > 100)                      │
│    • Join reordering                                                 │
│    Output: Logical Plan                                              │
│    ┌───────────────────────────────────────────┐                     │
│    │  Limit(10)                                │                     │
│    │    ↑                                      │                     │
│    │  Sort(o.total DESC)                       │                     │
│    │    ↑                                      │                     │
│    │  Project(u.name, o.total)                 │                     │
│    │    ↑                                      │                     │
│    │  HashJoin(u.id = o.user_id)               │                     │
│    │    ↑                        ↑             │                     │
│    │  Filter(age>18)        Filter(total>100)  │                     │
│    │    ↑                        ↑             │                     │
│    │  TableScan(users)      TableScan(orders)  │                     │
│    └───────────────────────────────────────────┘                     │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 7. Cost-Based Optimizer (src/optimizer_pro/)                         │
│    • Cardinality estimation (from statistics)                        │
│    • Cost calculation for each plan alternative                      │
│    • Index selection: Use index on users.age? orders.total?          │
│    • Join algorithm selection: Hash join vs. nested loop             │
│    • Parallel execution consideration                                │
│    Output: Optimized Physical Plan                                   │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 8. Transaction Begin (src/transaction/)                              │
│    • Allocate transaction ID (UUID-based)                            │
│    • Set isolation level: READ_COMMITTED (default)                   │
│    • Create MVCC snapshot (visible SCN threshold)                    │
│    • Acquire shared locks on users, orders tables                    │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 9. Query Execution (src/execution/executor.rs)                       │
│                                                                       │
│    9a. Left Side: Scan 'users' table                                 │
│    ┌──────────────────────────────────────────────────────────┐     │
│    │ • Index seek on users.age > 18 (if index exists)         │     │
│    │ • Otherwise: Full table scan with SIMD filter            │     │
│    │ • Buffer pool: Load pages (via src/buffer/)              │     │
│    │ • MVCC: Filter visible rows (SCN check)                  │     │
│    │ • Vectorized execution: Process 1024 rows at a time      │     │
│    │ Output: [user_id, name] tuples where age > 18            │     │
│    └──────────────────────────────────────────────────────────┘     │
│                                                                       │
│    9b. Right Side: Scan 'orders' table                               │
│    ┌──────────────────────────────────────────────────────────┐     │
│    │ • Index seek on orders.total > 100 (if index exists)     │     │
│    │ • Otherwise: Full table scan with SIMD filter            │     │
│    │ • Buffer pool: Load pages                                │     │
│    │ • MVCC: Filter visible rows                              │     │
│    │ • Vectorized execution                                   │     │
│    │ Output: [user_id, total] tuples where total > 100        │     │
│    └──────────────────────────────────────────────────────────┘     │
│                                                                       │
│    9c. Hash Join (src/execution/)                                    │
│    ┌──────────────────────────────────────────────────────────┐     │
│    │ • Build hash table on smaller side (orders)              │     │
│    │ • Probe with users side                                  │     │
│    │ • Match on u.id = o.user_id                              │     │
│    │ Output: [name, total] joined tuples                      │     │
│    └──────────────────────────────────────────────────────────┘     │
│                                                                       │
│    9d. Sort (src/execution/)                                         │
│    ┌──────────────────────────────────────────────────────────┐     │
│    │ • In-memory sort on o.total DESC                         │     │
│    │ • If too large: External merge sort (disk-based)         │     │
│    │ Output: Sorted tuples by total DESC                      │     │
│    └──────────────────────────────────────────────────────────┘     │
│                                                                       │
│    9e. Limit (src/execution/)                                        │
│    ┌──────────────────────────────────────────────────────────┐     │
│    │ • Take first 10 rows                                     │     │
│    │ Output: Top 10 results                                   │     │
│    └──────────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 10. Transaction Commit (src/transaction/)                            │
│     • Release shared locks                                           │
│     • Update transaction stats                                       │
│     • Commit timestamp                                               │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 11. Result Serialization (src/network/)                              │
│     • Convert tuples to wire protocol format                         │
│     • Apply compression if enabled                                   │
│     • Send to client over TCP connection                             │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 12. Monitoring & Analytics (src/workload/, src/autonomous/)          │
│     • SQL Monitor: Record execution stats                            │
│     • AWR: Update workload statistics                                │
│     • Autonomous: ML-based workload classification                   │
│     • Performance Hub: Real-time query metrics                       │
└──────────────────────────────────────────────────────────────────────┘
```

### Buffer Pool Interaction Details

```
Buffer Pool Manager (src/buffer/manager.rs)
    │
    ├─► Page Request: Load page for 'users' table
    │   ├─► Check page table (lock-free hash table)
    │   ├─► Page found? → Return frame reference
    │   └─► Page not found?
    │       ├─► Allocate frame from free list
    │       ├─► Evict page if needed (CLOCK/LRU/2Q policy)
    │       ├─► Read page from disk (via src/storage/disk.rs)
    │       └─► Insert into page table
    │
    └─► MVCC Version Check (src/transaction/)
        ├─► Read row header: xmin, xmax, SCN
        ├─► Compare with snapshot SCN
        ├─► Visible? → Include in result
        └─► Not visible? → Skip row
```

---

## Data Flow: Transaction Lifecycle

### INSERT Transaction Path

```
┌──────────────────────────────────────────────────────────────────────┐
│ 1. Client Request                                                     │
│    INSERT INTO users (id, name, age) VALUES (101, 'Alice', 25);      │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 2. Transaction Begin (src/transaction/)                              │
│    • txn_id = UUID::new_v4()                                         │
│    • isolation_level = READ_COMMITTED                                │
│    • state = Active                                                  │
│    • Log: "BEGIN TRANSACTION txn_id"                                 │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 3. Lock Acquisition (src/transaction/lock_manager.rs)                │
│    • Request exclusive lock on 'users' table                         │
│    • Deadlock detection: Check wait-for graph                        │
│    • Lock granted: Add to transaction lock list                      │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 4. Constraint Validation (src/constraints/)                          │
│    • Primary key check: Is id=101 unique?                            │
│    • Foreign key check: Validate referential integrity               │
│    • Check constraint: Verify age > 0 (if defined)                   │
│    • Unique constraint: Check unique columns                         │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 5. WAL Logging (src/transaction/wal.rs)                              │
│    • Generate WAL record:                                            │
│      ┌──────────────────────────────────────────────────┐            │
│      │ WAL Record                                       │            │
│      │ ┌──────────────────────────────────────────────┐ │            │
│      │ │ LSN: 1234567                                 │ │            │
│      │ │ Type: INSERT                                 │ │            │
│      │ │ Txn ID: txn_id                               │ │            │
│      │ │ Table: users                                 │ │            │
│      │ │ Row: (101, 'Alice', 25)                      │ │            │
│      │ │ Timestamp: 2025-12-17 00:00:00               │ │            │
│      │ └──────────────────────────────────────────────┘ │            │
│      └──────────────────────────────────────────────────┘            │
│    • Append to WAL buffer                                            │
│    • Flush to disk (fsync) - ensures durability                      │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 6. Storage Layer (src/storage/)                                      │
│    • Find page for 'users' table (via page directory)                │
│    • Buffer pool: Pin page (exclusive mode)                          │
│    • Allocate row slot in page                                       │
│    • Write row data with MVCC metadata:                              │
│      ┌──────────────────────────────────────────────────┐            │
│      │ Row Header                                       │            │
│      │ ┌──────────────────────────────────────────────┐ │            │
│      │ │ xmin: txn_id (creating transaction)          │ │            │
│      │ │ xmax: NULL (not deleted)                     │ │            │
│      │ │ SCN: current_scn                             │ │            │
│      │ └──────────────────────────────────────────────┘ │            │
│      │ Row Data: (101, 'Alice', 25)                     │            │
│      └──────────────────────────────────────────────────┘            │
│    • Mark page as dirty                                              │
│    • Unpin page                                                      │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 7. Index Update (src/index/)                                         │
│    • Update B-Tree index on 'id' column (primary key)                │
│    • Insert entry: key=101 → row_id                                  │
│    • Update any secondary indexes (if defined)                       │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 8. Transaction Commit (src/transaction/)                             │
│    • state = Committing                                              │
│    • Flush WAL log (ensure all records on disk)                      │
│    • Update transaction status in transaction table                  │
│    • Commit SCN = next_scn()                                         │
│    • Make row visible to other transactions (xmin committed)         │
│    • Release all locks                                               │
│    • state = Committed                                               │
│    • Log: "COMMIT TRANSACTION txn_id"                                │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 9. Async Background Tasks                                            │
│    • Checkpointing: Flush dirty pages to disk                        │
│    • Garbage collection: Cleanup old row versions                    │
│    • Statistics update: Recalculate table stats                      │
│    • Replication: Send changes to replicas (if enabled)              │
└──────────────────────────────────────────────────────────────────────┘
```

### MVCC Snapshot Isolation

```
┌────────────────────────────────────────────────────────────────────┐
│ Transaction T1 (SCN = 100)                                          │
│ ┌────────────────────────────────────────────────────────────────┐ │
│ │ BEGIN                                                           │ │
│ │ SELECT * FROM users WHERE id = 101                              │ │
│ │ ↓                                                               │ │
│ │ Snapshot: SCN = 100                                             │ │
│ │ Visibility Check:                                               │ │
│ │   • Row xmin = 50 (committed before SCN 100) → VISIBLE         │ │
│ │   • Row xmax = NULL (not deleted) → VISIBLE                    │ │
│ │ Result: Row returned                                            │ │
│ └────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────┐
│ Transaction T2 (SCN = 150)                                          │
│ ┌────────────────────────────────────────────────────────────────┐ │
│ │ BEGIN                                                           │ │
│ │ UPDATE users SET age = 30 WHERE id = 101                        │ │
│ │ ↓                                                               │ │
│ │ Lock acquisition: Exclusive lock on row 101                     │ │
│ │ WAL log: UPDATE record                                          │ │
│ │ Create new row version:                                         │ │
│ │   Old row: xmax = T2 (txn_id)                                   │ │
│ │   New row: xmin = T2 (txn_id), xmax = NULL                      │ │
│ │ COMMIT (SCN = 151)                                              │ │
│ └────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────┐
│ Transaction T3 (SCN = 200)                                          │
│ ┌────────────────────────────────────────────────────────────────┐ │
│ │ BEGIN                                                           │ │
│ │ SELECT * FROM users WHERE id = 101                              │ │
│ │ ↓                                                               │ │
│ │ Snapshot: SCN = 200                                             │ │
│ │ Visibility Check:                                               │ │
│ │   • Old row: xmax = T2 (committed at SCN 151 < 200) → INVISIBLE│ │
│ │   • New row: xmin = T2 (committed at SCN 151 < 200) → VISIBLE  │ │
│ │ Result: New row returned (age = 30)                             │ │
│ └────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────┘
```

---

## Data Flow: Replication

### Synchronous Replication Flow

```
┌──────────────────────────────────────────────────────────────────────┐
│ Primary Node                                                          │
│ ┌──────────────────────────────────────────────────────────────────┐ │
│ │ 1. Client writes to primary                                      │ │
│ │    INSERT INTO users VALUES (101, 'Alice', 25);                  │ │
│ └──────────────────────────────────────────────────────────────────┘ │
│                            │                                          │
│                            ▼                                          │
│ ┌──────────────────────────────────────────────────────────────────┐ │
│ │ 2. WAL Generation (src/transaction/wal.rs)                       │ │
│ │    • Create WAL record for INSERT                                │ │
│ │    • LSN = 1234567                                               │ │
│ │    • Append to WAL buffer                                        │ │
│ └──────────────────────────────────────────────────────────────────┘ │
│                            │                                          │
│                            ▼                                          │
│ ┌──────────────────────────────────────────────────────────────────┐ │
│ │ 3. Replication Sender (src/replication/)                         │ │
│ │    • Read WAL records from buffer                                │ │
│ │    • Serialize WAL records                                       │ │
│ │    • Send to all synchronous replicas                            │ │
│ └──────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘
                            │
            ┌───────────────┼───────────────┐
            │               │               │
            ▼               ▼               ▼
┌───────────────────┐ ┌───────────────────┐ ┌───────────────────┐
│ Replica 1         │ │ Replica 2         │ │ Replica 3         │
├───────────────────┤ ├───────────────────┤ ├───────────────────┤
│ 4. Receive WAL    │ │ 4. Receive WAL    │ │ 4. Receive WAL    │
│    • Network recv │ │    • Network recv │ │    • Network recv │
│    • Deserialize  │ │    • Deserialize  │ │    • Deserialize  │
│    • Validate LSN │ │    • Validate LSN │ │    • Validate LSN │
├───────────────────┤ ├───────────────────┤ ├───────────────────┤
│ 5. Apply WAL      │ │ 5. Apply WAL      │ │ 5. Apply WAL      │
│    • Parse INSERT │ │    • Parse INSERT │ │    • Parse INSERT │
│    • Apply to DB  │ │    • Apply to DB  │ │    • Apply to DB  │
│    • Update indexes│ │    • Update indexes│ │    • Update indexes│
├───────────────────┤ ├───────────────────┤ ├───────────────────┤
│ 6. Persist WAL    │ │ 6. Persist WAL    │ │ 6. Persist WAL    │
│    • Write to disk│ │    • Write to disk│ │    • Write to disk│
│    • fsync()      │ │    • fsync()      │ │    • fsync()      │
├───────────────────┤ ├───────────────────┤ ├───────────────────┤
│ 7. ACK to Primary │ │ 7. ACK to Primary │ │ 7. ACK to Primary │
│    • Send ACK     │ │    • Send ACK     │ │    • Send ACK     │
│    • LSN confirmed│ │    • LSN confirmed│ │    • LSN confirmed│
└───────────────────┘ └───────────────────┘ └───────────────────┘
            │               │               │
            └───────────────┼───────────────┘
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Primary Node                                                          │
│ ┌──────────────────────────────────────────────────────────────────┐ │
│ │ 8. Wait for ACKs (src/replication/)                              │ │
│ │    • Collect ACKs from all sync replicas                         │ │
│ │    • Quorum: Wait for majority (e.g., 2 out of 3)                │ │
│ └──────────────────────────────────────────────────────────────────┘ │
│                            │                                          │
│                            ▼                                          │
│ ┌──────────────────────────────────────────────────────────────────┐ │
│ │ 9. Transaction Commit                                            │ │
│ │    • All replicas confirmed → Safe to commit                     │ │
│ │    • Respond to client: INSERT successful                        │ │
│ └──────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘
```

### Failover Scenario

```
┌──────────────────────────────────────────────────────────────────────┐
│ 1. Primary Failure Detection (src/clustering/)                       │
│    • Health monitor detects primary node down                        │
│    • Timeout: No heartbeat for 5 seconds                             │
│    • Raft leader election triggered                                  │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 2. Raft Consensus (src/clustering/raft.rs)                           │
│    • Candidate replica starts election                               │
│    • Request votes from all nodes                                    │
│    • Majority votes received → Elected as new leader                 │
│    • Broadcast new leader to all nodes                               │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 3. Promote Replica to Primary                                        │
│    • New primary: Enable write mode                                  │
│    • Recover any uncommitted transactions                            │
│    • Update DNS/load balancer: Point to new primary                  │
│    • Notify monitoring systems                                       │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 4. Client Reconnection                                               │
│    • Clients detect connection failure                               │
│    • Retry connection (exponential backoff)                          │
│    • Connect to new primary                                          │
│    • Resume normal operations                                        │
└──────────────────────────────────────────────────────────────────────┘
```

---

## Integration Points Matrix

| Source Module | Target Module | Integration Type | Data Flow | Critical Path |
|--------------|---------------|------------------|-----------|---------------|
| **network** | parser | Function call | SQL text → AST | ✅ Yes |
| **parser** | catalog | Function call | Table lookup | ✅ Yes |
| **parser** | execution | Function call | AST → Plan | ✅ Yes |
| **execution** | transaction | Function call | Begin/Commit | ✅ Yes |
| **execution** | storage | Function call | Read/Write rows | ✅ Yes |
| **execution** | index | Function call | Index seek | ✅ Yes |
| **transaction** | storage | Function call | MVCC metadata | ✅ Yes |
| **transaction** | wal | Function call | Log records | ✅ Yes |
| **storage** | buffer | Function call | Page cache | ✅ Yes |
| **buffer** | io | Function call | Disk I/O | ✅ Yes |
| **replication** | wal | Data stream | WAL shipping | No |
| **clustering** | replication | Coordination | Failover | No |
| **security** | network | Middleware | Auth/Audit | ✅ Yes |
| **autonomous** | optimizer | Feedback loop | ML tuning | No |
| **workload** | monitoring | Data collection | Metrics | No |
| **flashback** | transaction | Data read | Version chains | No |
| **blockchain** | storage | Data write | Immutable rows | No |
| **multitenant** | storage | Resource isolation | Quotas | No |
| **orchestration** | **ALL** | Lifecycle mgmt | Health checks | No |

---

## Critical Paths

### 1. Query Execution Critical Path (Latency-Sensitive)
```
Client → Network → Security → Parser → Catalog → Planner →
Optimizer → Transaction → Execution → Storage → Buffer →
Disk I/O → Network → Client

Estimated Latency Budget:
• Network: 1-5 ms
• Security: 0.1 ms
• Parser: 0.5 ms
• Planner: 1 ms
• Optimizer: 2-10 ms
• Transaction: 0.2 ms
• Execution: 5-50 ms (depends on query complexity)
• Storage: 1-10 ms (buffer hit) or 5-15 ms (disk read)
• Total: 15-100 ms for simple queries
```

### 2. Transaction Commit Critical Path (Durability-Sensitive)
```
Client → Transaction Begin → Execution → WAL Log →
Disk fsync → Transaction Commit → Replication ACK →
Client Response

Estimated Latency Budget:
• WAL append: 0.1 ms
• Disk fsync: 1-10 ms (SSD) or 10-30 ms (HDD)
• Replication ACK: 1-5 ms (local) or 50-200 ms (cross-region)
• Total: 2-50 ms (local) or 60-240 ms (cross-region)
```

### 3. Failover Critical Path (Availability-Sensitive)
```
Primary Failure → Health Detection → Raft Election →
Replica Promotion → DNS Update → Client Reconnection

Estimated Recovery Time:
• Health detection: 5 seconds (timeout)
• Raft election: 1-3 seconds
• Promotion: 1 second
• DNS propagation: 5-60 seconds (depends on TTL)
• Total: 12-69 seconds (RTO = Recovery Time Objective)
```

---

## Component Lifecycle

### Startup Sequence (Ordered by Dependency)

```
1. Foundation Layer
   ├─► error::init()
   ├─► common::init()
   ├─► metadata::load_instance_metadata()
   └─► compat::check_compatibility()

2. Storage Layer
   ├─► memory::init_allocator()
   ├─► buffer::init_buffer_pool()
   ├─► storage::init_disk_manager()
   ├─► catalog::init_system_catalog()
   └─► io::init_async_runtime()

3. Transaction Layer
   ├─► transaction::init_manager()
   ├─► wal::init_wal_writer()
   ├─► index::init_index_manager()
   └─► concurrent::init_lock_manager()

4. Query Processing Layer
   ├─► parser::init_parser()
   ├─► execution::init_executor()
   ├─► optimizer::init_optimizer()
   └─► procedures::init_procedure_runtime()

5. Network Layer
   ├─► pool::init_connection_pool()
   ├─► network::init_tcp_server()
   ├─► api::init_rest_api()
   ├─► api::init_graphql_api()
   └─► websocket::init_websocket_server()

6. Enterprise Layer
   ├─► security::init_security_manager()
   ├─► replication::init_replication_manager()
   ├─► clustering::init_clustering()
   ├─► backup::init_backup_manager()
   ├─► monitoring::init_monitoring()
   ├─► workload::init_workload_intelligence()
   └─► autonomous::init_autonomous_db()

7. Orchestration Layer
   ├─► orchestration::init_service_registry()
   ├─► orchestration::init_actor_system()
   ├─► orchestration::init_health_aggregator()
   ├─► orchestration::register_all_services()
   └─► orchestration::start_all_components()

Total Startup Time: 5-15 seconds (depends on data size)
```

### Shutdown Sequence (Reverse Order)

```
1. Orchestration Layer
   ├─► orchestration::stop_all_components()
   └─► orchestration::shutdown_actor_system()

2. Enterprise Layer
   ├─► autonomous::shutdown()
   ├─► workload::shutdown()
   ├─► monitoring::shutdown()
   ├─► backup::shutdown()
   ├─► clustering::shutdown()
   ├─► replication::shutdown()
   └─► security::shutdown()

3. Network Layer
   ├─► websocket::shutdown()
   ├─► api::shutdown_graphql()
   ├─► api::shutdown_rest()
   ├─► network::shutdown_tcp_server()
   └─► pool::shutdown_connection_pool()

4. Query Processing Layer
   ├─► procedures::shutdown()
   ├─► optimizer::shutdown()
   ├─► execution::shutdown()
   └─► parser::shutdown()

5. Transaction Layer
   ├─► concurrent::shutdown()
   ├─► index::shutdown()
   ├─► wal::flush_and_close()
   └─► transaction::shutdown()

6. Storage Layer
   ├─► io::shutdown()
   ├─► catalog::persist_metadata()
   ├─► storage::flush_all_pages()
   ├─► buffer::flush_and_release()
   └─► memory::shutdown_allocator()

Total Shutdown Time: 2-10 seconds (depends on dirty pages)
```

---

## Key Observations

### Architectural Strengths
1. ✅ **Layered Architecture**: Clear separation of concerns
2. ✅ **Unified Error Handling**: DbError enum across all modules
3. ✅ **MVCC**: Industry-standard concurrency control (100% test pass rate)
4. ✅ **Comprehensive Features**: 100+ modules, 300+ submodules
5. ✅ **Enterprise-Grade**: Replication, clustering, RAC, security
6. ✅ **Orchestration**: Service registry, actor system, health monitoring
7. ✅ **Observability**: AWR, SQL tuning, autonomous features

### Areas for Improvement
1. 🔄 **Catalog Integration**: Expand to full system catalog (sys_tables, sys_indexes)
2. 🔄 **Multi-Tenancy Consolidation**: Merge duplicate modules (multitenancy vs multitenant)
3. 🔄 **Connection Pool Duplication**: Remove from operations module
4. 🔄 **Flashback + WAL**: Integrate for database-level flashback
5. 🔄 **Blockchain + Security**: Unified audit trail
6. 🔄 **Metrics Centralization**: Unified metrics collection

### Performance Optimizations
1. ✅ **SIMD**: Vectorized operations (AVX2/AVX-512)
2. ✅ **Lock-Free**: Page table, hash maps, queues
3. ✅ **Zero-Allocation**: Buffer pool hot path
4. ✅ **Batch Flush**: Sequential I/O optimization
5. ✅ **Direct I/O**: Bypass OS page cache (Linux io_uring, Windows IOCP)

---

## Next Steps
1. Review and validate data flows with domain experts
2. Create detailed sequence diagrams for complex operations
3. Implement missing integrations (catalog, flashback, blockchain)
4. Consolidate duplicate modules (multitenancy)
5. Create comprehensive test suite for critical paths
6. Performance profiling and optimization

---

**Document Status:** ✅ COMPLETE
**Review Status:** Pending
**Last Updated:** 2025-12-17
