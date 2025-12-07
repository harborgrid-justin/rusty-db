# RustyDB Master Coordination Documentation

## Project Overview

**RustyDB** is an enterprise-grade, Oracle-compatible database management system written in Rust. This document serves as the central coordination guide for 10 parallel development agents building different subsystems.

**Architecture Philosophy**: Modular, scalable, fault-tolerant, with clear separation of concerns and well-defined APIs between components.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Module Descriptions](#module-descriptions)
3. [API Contracts](#api-contracts)
4. [Shared Data Structures](#shared-data-structures)
5. [Integration Points](#integration-points)
6. [Performance Targets](#performance-targets)
7. [Development Guidelines](#development-guidelines)
8. [Testing Requirements](#testing-requirements)

---

## Architecture Overview

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Client Applications                          │
│                    (SQL Clients, ORMs, Tools)                       │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Network/Protocol Layer                         │
│           (TCP Server, Connection Pool, Wire Protocol)              │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Security & Authentication                        │
│         (RBAC, Encryption, Audit Logging, Session Mgmt)             │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         SQL Parser                                  │
│              (SQL Parsing, AST Generation, Validation)              │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Query Planner/Optimizer                        │
│         (Cost-based Optimization, Plan Caching, Statistics)         │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Query Executor Engine                          │
│    (Vectorized Execution, Adaptive Processing, JIT Compilation)     │
└─────┬───────────┬──────────┬──────────┬──────────┬─────────────────┘
      │           │          │          │          │
      ▼           ▼          ▼          ▼          ▼
┌──────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌──────────┐
│ Storage  │ │ Index  │ │ Trans- │ │OLAP/   │ │ Stored   │
│ Engine   │ │ Engine │ │ action │ │Analytics│ │Procedures│
│          │ │        │ │ /MVCC  │ │        │ │          │
└────┬─────┘ └───┬────┘ └───┬────┘ └───┬────┘ └────┬─────┘
     │           │          │          │          │
     └───────────┴──────────┴──────────┴──────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Supporting Infrastructure                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐          │
│  │Clustering│  │ Backup/  │  │Monitoring│  │Replication│          │
│  │  & HA    │  │ Recovery │  │& Metrics │  │           │          │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘          │
└─────────────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **Separation of Concerns**: Each module has a single, well-defined responsibility
2. **Interface-Based Design**: Modules communicate through traits and clear APIs
3. **Async/Await**: Leveraging Tokio for high-performance async operations
4. **Thread Safety**: Using Arc, RwLock, and atomic operations for concurrent access
5. **Error Handling**: Consistent error types across all modules
6. **Zero-Copy Where Possible**: Minimize data copying for performance
7. **Extensibility**: Plugin architecture for custom functions and extensions

---

## Module Descriptions

### 1. Storage Engine (`src/storage/`)

**Responsibility**: Low-level data persistence, page management, buffer pool, and disk I/O.

**Key Components**:
- `DiskManager`: Direct disk I/O operations, file management
- `BufferPoolManager`: In-memory page cache with LRU/CLOCK eviction
- `Page`: Fixed-size data pages (default 4KB-8KB)
- `PartitionManager`: Table partitioning and partition pruning
- `JsonData`: Native JSON storage and operations

**Key Features**:
- Copy-on-write (CoW) for MVCC support
- Write-ahead logging (WAL) integration
- Direct I/O and async I/O support
- Compression and encryption at page level
- Multi-tier storage (hot/warm/cold)

**Exports**:
```rust
pub struct StorageEngine;
pub struct DiskManager;
pub struct BufferPoolManager;
pub struct Page;
pub type PageId = u64;
```

**Minimum LOC Target**: 3,000+ lines

---

### 2. Transaction/MVCC (`src/transaction/`)

**Responsibility**: Multi-version concurrency control, transaction management, isolation levels.

**Key Components**:
- `TransactionManager`: Transaction lifecycle management
- `MvccVersionStore`: Multi-version tuple storage
- `LockManager`: Row-level and table-level locking
- `DeadlockDetector`: Waits-for graph-based deadlock detection
- `SnapshotIsolation`: Snapshot-based isolation implementation

**Key Features**:
- ACID compliance
- Multiple isolation levels (Read Uncommitted, Read Committed, Repeatable Read, Serializable)
- Optimistic and pessimistic concurrency control
- Distributed transaction support (2PC/3PC)
- Transaction recovery and rollback

**Exports**:
```rust
pub struct TransactionManager;
pub struct Transaction;
pub type TransactionId = u64;
pub enum IsolationLevel { ReadUncommitted, ReadCommitted, RepeatableRead, Serializable }
```

**Minimum LOC Target**: 3,000+ lines

---

### 3. Query Execution Engine (`src/execution/`)

**Responsibility**: Execute query plans, optimize execution strategies, manage query lifecycle.

**Key Components**:
- `Executor`: Main query execution engine
- `Planner`: Physical plan generation
- `Optimizer`: Cost-based query optimization
- `ParallelExecutor`: Parallel query execution
- `CteEvaluator`: Common Table Expression handling
- `SubqueryEvaluator`: Subquery execution

**Key Features**:
- Vectorized execution (SIMD where applicable)
- Adaptive query processing
- JIT compilation patterns
- Operator pipelining
- Runtime query statistics collection

**Exports**:
```rust
pub struct Executor;
pub struct Planner;
pub struct Optimizer;
pub struct QueryResult;
pub struct ExecutionPlan;
```

**Minimum LOC Target**: 3,000+ lines

---

### 4. Analytics/OLAP (`src/analytics/`)

**Responsibility**: Analytical query processing, columnar storage, materialized views.

**Key Components**:
- `ColumnarStorage`: Column-oriented data storage
- `MaterializedViewManager`: Materialized view management
- `ApproximateQueryProcessor`: Approximate query processing (sampling, sketches)
- `WindowFunctionExecutor`: Window function execution
- `AggregationEngine`: High-performance aggregations

**Key Features**:
- Columnar data format (Parquet-like)
- Approximate query processing (HyperLogLog, Count-Min Sketch)
- Incremental view maintenance
- Star schema optimization
- Parallel aggregation

**Exports**:
```rust
pub struct ColumnarStorage;
pub struct MaterializedView;
pub struct ApproximateQueryProcessor;
```

**Minimum LOC Target**: 3,000+ lines

---

### 5. Security/RBAC (`src/security/`)

**Responsibility**: Authentication, authorization, encryption, audit logging.

**Key Components**:
- `SecurityManager`: Central security coordinator
- `AuthenticationProvider`: Multi-method authentication
- `RbacEngine`: Role-based access control
- `EncryptionManager`: Data encryption at rest and in transit
- `AuditLogger`: Comprehensive audit trail

**Key Features**:
- Fine-grained access control (table, row, column level)
- Multiple authentication methods (local, LDAP, OAuth, JWT)
- TLS/SSL support
- Transparent Data Encryption (TDE)
- Password policies and rotation
- Multi-factor authentication (MFA)

**Exports**:
```rust
pub struct SecurityManager;
pub struct User;
pub struct Role;
pub enum Permission;
pub struct Session;
```

**Minimum LOC Target**: 3,000+ lines

---

### 6. Clustering/High Availability (`src/clustering/`)

**Responsibility**: Distributed consensus, node coordination, automatic failover.

**Key Components**:
- `ClusterManager`: Cluster topology management
- `ConsensusEngine`: Raft-based distributed consensus
- `ShardingManager`: Automatic data sharding
- `FailoverController`: Automatic failover and recovery
- `LoadBalancer`: Query load balancing

**Key Features**:
- Multi-datacenter replication
- Automatic leader election
- Split-brain prevention
- Dynamic cluster membership
- Cross-datacenter consistency

**Exports**:
```rust
pub struct ClusterManager;
pub struct NodeInfo;
pub enum NodeRole { Leader, Follower, Candidate }
pub struct ShardingStrategy;
```

**Minimum LOC Target**: 3,000+ lines

---

### 7. Backup/Recovery (`src/backup/`)

**Responsibility**: Data backup, point-in-time recovery, disaster recovery.

**Key Components**:
- `BackupManager`: Backup orchestration
- `IncrementalBackup`: Incremental backup implementation
- `PointInTimeRecovery`: PITR functionality
- `SnapshotManager`: Consistent snapshots
- `RestoreEngine`: Backup restoration

**Key Features**:
- Full, incremental, and differential backups
- Point-in-time recovery (PITR)
- Continuous archiving
- Backup compression and encryption
- Cloud storage integration (S3, Azure Blob, GCS)
- Backup validation and verification

**Exports**:
```rust
pub struct BackupManager;
pub struct BackupMetadata;
pub enum BackupType { Full, Incremental, Differential }
pub struct RestorePoint;
```

**Minimum LOC Target**: 3,000+ lines

---

### 8. Stored Procedures (`src/procedures/`)

**Responsibility**: PL/SQL-like procedural language, user-defined functions, triggers.

**Key Components**:
- `ProcedureExecutor`: Stored procedure execution engine
- `ProcedureCompiler`: Procedure compilation to bytecode
- `FunctionRegistry`: User-defined function management
- `TriggerManager`: Trigger execution and management
- `CursorManager`: Cursor-based iteration

**Key Features**:
- PL/SQL-compatible syntax
- Control flow (IF, LOOP, WHILE, FOR)
- Exception handling
- Cursors and result sets
- Transaction control within procedures
- Function overloading

**Exports**:
```rust
pub struct ProcedureExecutor;
pub struct StoredProcedure;
pub struct UserDefinedFunction;
pub struct Trigger;
```

**Minimum LOC Target**: 3,000+ lines

---

### 9. Monitoring/Metrics (`src/monitoring/`)

**Responsibility**: Real-time metrics, query profiling, resource governance.

**Key Components**:
- `MetricsCollector`: System-wide metrics collection
- `QueryProfiler`: Query performance profiling
- `ResourceGovernor`: Resource limit enforcement
- `HealthChecker`: System health monitoring
- `AlertManager`: Alert generation and routing

**Key Features**:
- Real-time performance metrics
- Query execution statistics
- Resource usage tracking (CPU, memory, I/O)
- Slow query logging
- Wait event analysis
- Performance baselines and anomaly detection

**Exports**:
```rust
pub struct MetricsCollector;
pub struct QueryProfile;
pub struct ResourceLimits;
pub struct HealthStatus;
```

**Minimum LOC Target**: 3,000+ lines

---

### 10. Index Engine (`src/index/`)

**Responsibility**: Multiple index types, index management, index-only scans.

**Key Components**:
- `BTreeIndex`: B+ tree implementation
- `LsmTreeIndex`: Log-structured merge tree
- `SpatialIndex`: R-tree for spatial data
- `FullTextIndex`: Full-text search indexing
- `IndexManager`: Index lifecycle management

**Key Features**:
- Multiple index types (B+Tree, LSM, Hash, Spatial, Full-text)
- Covering indexes
- Partial indexes
- Index-only scans
- Online index building
- Index statistics and cardinality estimation

**Exports**:
```rust
pub struct IndexManager;
pub enum Index { BTree, LSM, Hash, Spatial, FullText }
pub type IndexKey;
pub type IndexValue;
```

**Minimum LOC Target**: 3,000+ lines

---

## API Contracts

### Common Trait Hierarchy

All modules should implement and use these common traits defined in `src/common.rs`:

```rust
/// Base trait for all major components
pub trait Component {
    fn initialize(&mut self) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
    fn health_check(&self) -> HealthStatus;
}

/// Transaction-aware components
pub trait Transactional {
    fn begin_transaction(&mut self, isolation: IsolationLevel) -> Result<TransactionId>;
    fn commit(&mut self, txn_id: TransactionId) -> Result<()>;
    fn rollback(&mut self, txn_id: TransactionId) -> Result<()>;
}

/// Recoverable components (for crash recovery)
pub trait Recoverable {
    fn checkpoint(&self) -> Result<()>;
    fn recover(&mut self, lsn: LogSequenceNumber) -> Result<()>;
}

/// Monitorable components (for metrics)
pub trait Monitorable {
    fn collect_metrics(&self) -> HashMap<String, MetricValue>;
    fn get_statistics(&self) -> ComponentStatistics;
}

/// Serializable components (for replication)
pub trait ReplicableState {
    fn serialize_state(&self) -> Result<Vec<u8>>;
    fn deserialize_state(&mut self, data: &[u8]) -> Result<()>;
}
```

### Module Interface Contracts

#### Storage Engine Interface

```rust
pub trait StorageInterface {
    // Page operations
    fn read_page(&self, page_id: PageId, txn_id: TransactionId) -> Result<Arc<Page>>;
    fn write_page(&mut self, page: Page, txn_id: TransactionId) -> Result<PageId>;
    fn flush_page(&mut self, page_id: PageId) -> Result<()>;

    // Scan operations
    fn sequential_scan(&self, table_id: TableId) -> Result<Box<dyn Iterator<Item = Tuple>>>;
    fn index_scan(&self, index_id: IndexId, key: &IndexKey) -> Result<Box<dyn Iterator<Item = Tuple>>>;
}
```

#### Transaction Manager Interface

```rust
pub trait TransactionInterface {
    fn begin(&mut self, isolation: IsolationLevel) -> Result<TransactionId>;
    fn commit(&mut self, txn_id: TransactionId) -> Result<()>;
    fn rollback(&mut self, txn_id: TransactionId) -> Result<()>;
    fn get_snapshot(&self, txn_id: TransactionId) -> Result<Snapshot>;
    fn acquire_lock(&mut self, txn_id: TransactionId, resource: LockResource, mode: LockMode) -> Result<()>;
    fn release_locks(&mut self, txn_id: TransactionId) -> Result<()>;
}
```

#### Execution Engine Interface

```rust
pub trait ExecutionInterface {
    fn execute_plan(&mut self, plan: ExecutionPlan, txn_id: TransactionId) -> Result<QueryResult>;
    fn explain_plan(&self, sql: &str) -> Result<String>;
    fn get_plan_cost(&self, plan: &ExecutionPlan) -> f64;
}
```

#### Security Manager Interface

```rust
pub trait SecurityInterface {
    fn authenticate(&self, username: &str, credentials: &Credentials) -> Result<Session>;
    fn authorize(&self, session: &Session, permission: Permission, resource: &Resource) -> Result<bool>;
    fn audit_log(&mut self, event: AuditEvent) -> Result<()>;
    fn encrypt_data(&self, data: &[u8], key_id: &str) -> Result<Vec<u8>>;
    fn decrypt_data(&self, data: &[u8], key_id: &str) -> Result<Vec<u8>>;
}
```

---

## Shared Data Structures

All shared data structures are defined in `src/common.rs`:

### Core Types

```rust
// Identifiers
pub type TransactionId = u64;
pub type PageId = u64;
pub type TableId = u32;
pub type IndexId = u32;
pub type ColumnId = u16;
pub type RowId = u64;
pub type LogSequenceNumber = u64;

// Value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Date(i64),
    Timestamp(i64),
    Json(serde_json::Value),
    Array(Vec<Value>),
}

// Tuple representation
#[derive(Debug, Clone)]
pub struct Tuple {
    pub values: Vec<Value>,
    pub row_id: RowId,
}

// Schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub columns: Vec<ColumnDef>,
    pub primary_key: Option<Vec<ColumnId>>,
    pub foreign_keys: Vec<ForeignKeyConstraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    Integer,
    BigInt,
    Float,
    Double,
    Varchar(usize),
    Text,
    Boolean,
    Date,
    Timestamp,
    Json,
    Blob,
}
```

### Error Handling

```rust
// Unified error type (already in src/error.rs, but enhanced)
#[derive(Error, Debug)]
pub enum DbError {
    // Storage errors
    #[error("Storage error: {0}")]
    Storage(String),

    // Transaction errors
    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Deadlock detected")]
    Deadlock,

    #[error("Lock timeout")]
    LockTimeout,

    // Execution errors
    #[error("Execution error: {0}")]
    Execution(String),

    // Security errors
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Authorization denied: {0}")]
    AuthorizationDenied(String),

    // Network errors
    #[error("Network error: {0}")]
    Network(String),

    // And others...
}

pub type Result<T> = std::result::Result<T, DbError>;
```

### Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    // Storage
    pub data_dir: String,
    pub page_size: usize,
    pub buffer_pool_size: usize,

    // Transaction
    pub default_isolation: IsolationLevel,
    pub lock_timeout: Duration,

    // Networking
    pub listen_address: String,
    pub port: u16,
    pub max_connections: usize,

    // Security
    pub enable_tls: bool,
    pub enable_encryption: bool,

    // Clustering
    pub cluster_enabled: bool,
    pub node_id: String,
    pub seed_nodes: Vec<String>,

    // Performance
    pub worker_threads: usize,
    pub enable_jit: bool,
}
```

---

## Integration Points

### Critical Dependencies Between Modules

```
Storage Engine
    ├─ Used by: Transaction Manager, Index Engine, Backup Manager
    └─ Depends on: None (foundation layer)

Transaction Manager
    ├─ Used by: Execution Engine, Security Manager, Stored Procedures
    └─ Depends on: Storage Engine, Index Engine

Index Engine
    ├─ Used by: Execution Engine, Query Optimizer
    └─ Depends on: Storage Engine, Transaction Manager

Execution Engine
    ├─ Used by: Network Server, Stored Procedures
    └─ Depends on: Storage Engine, Transaction Manager, Index Engine, Security Manager

Security Manager
    ├─ Used by: Network Server, All modules (authorization checks)
    └─ Depends on: Storage Engine (for metadata)

Clustering
    ├─ Used by: All modules (for distributed coordination)
    └─ Depends on: Network, Transaction Manager, Replication

Backup/Recovery
    ├─ Used by: Clustering (for snapshots)
    └─ Depends on: Storage Engine, Transaction Manager

Monitoring
    ├─ Used by: All modules (metrics collection)
    └─ Depends on: None (observer pattern)

Analytics/OLAP
    ├─ Used by: Execution Engine
    └─ Depends on: Storage Engine, Index Engine

Stored Procedures
    ├─ Used by: Execution Engine
    └─ Depends on: Execution Engine, Transaction Manager, Security Manager
```

### Initialization Order

**Critical**: Modules must be initialized in this order to respect dependencies:

1. `error` - Error types (no dependencies)
2. `common` - Shared types and traits
3. `storage` - Storage engine
4. `catalog` - System catalog (depends on storage)
5. `index` - Index engine (depends on storage)
6. `transaction` - Transaction manager (depends on storage, index)
7. `security` - Security manager (depends on storage, catalog)
8. `parser` - SQL parser (no runtime dependencies)
9. `execution` - Execution engine (depends on storage, transaction, index, security)
10. `analytics` - OLAP engine (depends on storage, execution)
11. `procedures` - Stored procedures (depends on execution)
12. `replication` - Replication (depends on transaction, storage)
13. `clustering` - Clustering (depends on replication, transaction)
14. `backup` - Backup/recovery (depends on storage, transaction)
15. `monitoring` - Monitoring (depends on all modules)
16. `network` - Network server (depends on all modules)

### Event-Driven Integration

Modules communicate via events for loose coupling:

```rust
pub enum SystemEvent {
    // Transaction events
    TransactionBegin(TransactionId),
    TransactionCommit(TransactionId),
    TransactionRollback(TransactionId),

    // Storage events
    PageEvicted(PageId),
    CheckpointStarted,
    CheckpointCompleted,

    // Cluster events
    NodeJoined(NodeId),
    NodeLeft(NodeId),
    LeaderElected(NodeId),

    // Security events
    UserLogin(String),
    AuthenticationFailed(String),
    PermissionDenied(String, String),

    // Monitoring events
    SlowQuery(String, Duration),
    ResourceThresholdExceeded(String, f64),
}

pub trait EventListener {
    fn on_event(&mut self, event: SystemEvent) -> Result<()>;
}
```

---

## Performance Targets

### Throughput Targets

| Operation | Target | Notes |
|-----------|--------|-------|
| Point SELECT | 50,000+ QPS | Single-row lookup with index |
| INSERT | 30,000+ TPS | Single row, no indexes |
| UPDATE | 25,000+ TPS | Single row with index |
| DELETE | 25,000+ TPS | Single row with index |
| Range Scan | 1M+ rows/sec | Sequential scan |
| Join (Hash) | 500K+ rows/sec | Two tables |
| Aggregation | 2M+ rows/sec | Simple SUM/COUNT |

### Latency Targets (p95)

| Operation | Target | Notes |
|-----------|--------|-------|
| Point SELECT | < 1ms | Hot cache |
| Simple Query | < 5ms | Single table, indexed |
| Complex Query | < 50ms | Multi-table join |
| Transaction Commit | < 10ms | With WAL flush |
| Index Lookup | < 0.5ms | In-memory B-tree |

### Resource Targets

- **Memory Efficiency**: < 50% overhead for metadata
- **Disk I/O**: > 80% sequential I/O ratio
- **CPU Utilization**: > 70% on analytical workloads
- **Network**: > 1GB/sec throughput for replication
- **Concurrent Connections**: Support 10,000+ connections

### Scalability Targets

- **Vertical**: Scale to 1TB+ of RAM, 100+ cores
- **Horizontal**: Scale to 100+ nodes in a cluster
- **Storage**: Support databases up to 100TB+
- **Partitioning**: Support 10,000+ partitions per table

---

## Development Guidelines

### Code Standards

1. **Documentation**
   - Every public function must have doc comments
   - Modules must have comprehensive module-level documentation
   - Complex algorithms must have inline comments

2. **Error Handling**
   - Use `Result<T>` for all fallible operations
   - Provide context in error messages
   - Never use `unwrap()` in production code (use `expect()` with reason)

3. **Testing**
   - Unit tests for all public APIs
   - Integration tests for module interactions
   - Benchmark tests for performance-critical code
   - Target: > 80% code coverage

4. **Performance**
   - Profile before optimizing
   - Use `#[inline]` for small, hot functions
   - Minimize allocations in hot paths
   - Use `Arc<RwLock<>>` for shared mutable state

5. **Concurrency**
   - Prefer message passing over shared state where possible
   - Use `parking_lot` locks for better performance
   - Avoid holding locks across await points
   - Document lock ordering to prevent deadlocks

### Module Structure Template

Each module should follow this structure:

```
src/module_name/
├── mod.rs              # Public API, exports, module documentation
├── core.rs             # Core logic and algorithms
├── manager.rs          # Component lifecycle management
├── config.rs           # Configuration structures
├── metrics.rs          # Metrics and monitoring hooks
├── tests.rs            # Unit tests
└── bench.rs            # Benchmarks
```

### Integration Checklist

Before marking a module as complete, ensure:

- [ ] All public APIs documented with examples
- [ ] Error handling uses DbError consistently
- [ ] Implements relevant common traits (Component, Monitorable, etc.)
- [ ] Registers metrics with monitoring system
- [ ] Integrates with transaction manager (if applicable)
- [ ] Integrates with security manager (authorization checks)
- [ ] Thread-safe and async-ready
- [ ] Unit tests with >80% coverage
- [ ] Integration tests with dependent modules
- [ ] Benchmarks for performance-critical paths
- [ ] No clippy warnings
- [ ] Formatted with rustfmt

### Coordination Protocol

1. **API Changes**: Notify orchestrator before making breaking changes
2. **Dependencies**: Update this document when adding cross-module dependencies
3. **Performance**: Report any performance regressions
4. **Testing**: Run full test suite before committing
5. **Documentation**: Keep module documentation in sync with implementation

---

## Testing Requirements

### Test Levels

1. **Unit Tests** (`#[cfg(test)]` modules)
   - Test individual functions and structs
   - Mock dependencies
   - Fast execution (< 1 second per test)

2. **Integration Tests** (`tests/` directory)
   - Test module interactions
   - Use real dependencies
   - Test failure scenarios

3. **System Tests**
   - End-to-end SQL query tests
   - Multi-client concurrency tests
   - Failure and recovery tests

4. **Performance Tests**
   - Benchmark critical paths using `criterion`
   - Measure throughput and latency
   - Track performance over time

### Test Data

- Use `tempfile` for temporary test databases
- Provide standard test datasets (small, medium, large)
- Include edge cases (empty tables, NULL values, etc.)

### Continuous Integration

All tests must pass before merging:
```bash
cargo test --all-features
cargo clippy -- -D warnings
cargo fmt -- --check
cargo bench
```

---

## Appendix: Quick Reference

### Common Imports

```rust
use crate::{Result, DbError};
use crate::common::*;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::mpsc;
```

### Logging Standards

```rust
use tracing::{info, warn, error, debug, trace};

// Example usage
info!(table_id = %table_id, "Creating new table");
warn!(txn_id = %txn_id, "Transaction timeout");
error!(error = %e, "Failed to write page");
```

### Metrics Registration

```rust
// Each module should register metrics on initialization
impl Component for MyModule {
    fn initialize(&mut self) -> Result<()> {
        metrics::register_counter("my_module_operations_total");
        metrics::register_histogram("my_module_operation_duration");
        Ok(())
    }
}
```

---

## Document Maintenance

**Last Updated**: December 7, 2025
**Version**: 1.0
**Maintained By**: Orchestrator Agent

**Change Log**:
- 2025-12-07: Initial version

For questions or clarifications, consult the orchestrator agent or refer to individual module documentation.
