# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build the project
cargo build --release

# Check compilation without building
cargo check

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test storage::
cargo test transaction::
cargo test security::

# Start the database server (default port: 5432)
cargo run --bin rusty-db-server

# Start the CLI client
cargo run --bin rusty-db-cli

# Run benchmarks
cargo bench

# Format code
cargo fmt

# Run linter
cargo clippy

# Auto-fix issues
cargo clippy --fix
```

## Project Status

**Current Status**: Active Refactoring (Module Reorganization)
**Build Status**: In Progress
**Last Updated**: 2025-12-09

The project is undergoing a major refactoring where large files (>1300 lines) are being split into smaller submodules (<500 lines ideally). See `.scratchpad/COORDINATION_MASTER.md` for current agent assignments and progress.

## Architecture Overview

RustyDB is an enterprise-grade, Oracle-compatible database management system written in Rust. The codebase is organized into layered modules with clear separation of concerns.

### Core Foundation Layer

**Location**: `src/`

- **error.rs**: Unified `DbError` enum with `thiserror` - all modules use `Result<T> = std::result::Result<T, DbError>`
- **common.rs**: Shared type aliases (`TransactionId`, `PageId`, `TableId`, `IndexId`, `SessionId`) and core traits (`Component`, `Transactional`, `Recoverable`, `Monitorable`)
- **lib.rs**: Main library entry point, module declarations, public API

### Storage Layer

**Location**: `src/storage/`, `src/buffer/`, `src/memory/`, `src/io/`

- **storage/**: Page-based storage (4KB pages), disk I/O, LSM trees, columnar storage, partitioning
  - `page.rs` - Page structure and layout
  - `disk.rs` - Disk I/O operations
  - `buffer.rs` - Buffer pool integration
  - `partitioning/` - Table partitioning (submodules: types, manager, operations, execution, optimizer, pruning)
  - `lsm.rs` - LSM tree storage
  - `columnar.rs` - Column-oriented storage
  - `tiered.rs` - Tiered storage
  - `json.rs` - JSON storage

- **buffer/**: High-performance buffer manager
  - `manager.rs` - Buffer pool management with pluggable eviction (CLOCK, LRU, 2Q, LRU-K, LIRS, ARC)
  - Lock-free page table

- **memory/**: Memory management subsystem
  - `allocator.rs` - Slab allocator, arena allocator, large object allocator
  - `buffer_pool.rs` - Memory buffer pooling
  - `debug.rs` - Memory debugging utilities
  - Memory pressure management

- **io/**: Cross-platform async I/O
  - Direct I/O, ring buffers
  - Platform-specific: Windows IOCP, Linux io_uring

### Transaction Layer

**Location**: `src/transaction/`

- **transaction/**: Transaction management
  - MVCC (Multi-Version Concurrency Control)
  - Transaction lifecycle management
  - Lock manager with deadlock detection
  - Write-Ahead Logging (WAL)
  - Snapshot isolation

### Query Processing

**Location**: `src/parser/`, `src/execution/`, `src/optimizer_pro/`

- **parser/**: SQL parsing using `sqlparser` crate
  - SQL statement parsing
  - AST generation

- **execution/**: Query execution engine
  - `executor.rs` - Query executor
  - `planner.rs` - Query planner
  - `optimizer.rs` - Basic optimization
  - `cte.rs` - Common Table Expressions (CTEs)
  - Parallel execution
  - Vectorized operations

- **optimizer_pro/**: Advanced query optimization
  - `cost_model.rs` - Cost-based optimization
  - `plan_generator.rs` - Plan generation
  - `plan_baselines.rs` - SQL plan baselines
  - `adaptive.rs` - Adaptive query execution
  - `transformations.rs` - Query transformations
  - `hints.rs` - Optimizer hints

### Index Structures

**Location**: `src/index/`, `src/simd/`

- **index/**: Multiple index implementations
  - B-Tree index
  - LSM-Tree index
  - Hash index
  - Spatial index (R-Tree)
  - Full-text search index
  - Bitmap index
  - Partial indexes

- **simd/**: SIMD-accelerated operations
  - AVX2/AVX-512 filtering
  - SIMD aggregation
  - SIMD string operations
  - Vectorized hash operations

### Networking & API

**Location**: `src/network/`, `src/api/`, `src/pool/`

- **network/**: Network layer
  - TCP server
  - Wire protocol
  - Connection management
  - `advanced_protocol.rs` - Advanced protocol features
  - `cluster_network.rs` - Cluster networking

- **api/**: REST and GraphQL APIs
  - `monitoring.rs` - Monitoring endpoints
  - `gateway.rs` - API gateway
  - `enterprise_integration.rs` - Enterprise integrations
  - `graphql/` - GraphQL implementation (submodules: schema, queries, mutations, subscriptions, etc.)
  - REST API (axum-based)
  - OpenAPI documentation

- **pool/**: Connection pooling
  - `connection_pool.rs` - Connection pool manager
  - `session_manager.rs` - Session management
  - Connection lifecycle

### Enterprise Features

**Location**: `src/security/`, `src/security_vault/`, `src/clustering/`, `src/rac/`, `src/replication/`, `src/advanced_replication/`, `src/backup/`, `src/monitoring/`

- **security/**: 10 specialized security modules
  - `memory_hardening.rs` - Buffer overflow protection, guard pages
  - `buffer_overflow.rs` - Bounds checking, stack canaries
  - `insider_threat.rs` - Behavioral analytics, anomaly detection
  - `network_hardening.rs` - DDoS protection, rate limiting
  - `injection_prevention.rs` - SQL/command injection defense
  - `auto_recovery.rs` - Automatic failure detection and recovery
  - `circuit_breaker.rs` - Cascading failure prevention
  - `encryption.rs` - Encryption engine
  - `garbage_collection.rs` - Secure memory sanitization
  - `security_core.rs` - Unified policy engine, compliance validation
  - RBAC, authentication, audit logging

- **security_vault/**: Advanced data protection
  - Transparent Data Encryption (TDE)
  - Data masking
  - Key management
  - Virtual Private Database (VPD)

- **clustering/**: Distributed clustering
  - Raft consensus
  - Sharding
  - Automatic failover
  - Geo-replication

- **rac/**: Real Application Clusters (Oracle RAC-like)
  - `cache_fusion.rs` - Cache Fusion protocol
  - Global resource directory
  - Parallel query execution

- **replication/**: Database replication
  - `mod.rs` - Core replication
  - `snapshots.rs` - Snapshot replication
  - `slots.rs` - Replication slots
  - `monitor.rs` - Replication monitoring
  - Synchronous, asynchronous, semi-synchronous modes

- **advanced_replication/**: Advanced replication features
  - Multi-master replication
  - Logical replication
  - CRDT-based conflict resolution

- **backup/**: Backup and recovery
  - Full backups
  - Incremental backups
  - Point-in-Time Recovery (PITR)
  - Disaster recovery

- **monitoring/**: System monitoring
  - Metrics collection
  - Profiling
  - Resource governance
  - Health checks
  - Performance diagnostics

### Specialized Engines

**Location**: `src/graph/`, `src/document_store/`, `src/spatial/`, `src/ml/`, `src/ml_engine/`, `src/inmemory/`

- **graph/**: Graph database engine
  - Property graph database
  - PGQL-like query language
  - Graph algorithms (shortest path, centrality, community detection)

- **document_store/**: Document database
  - JSON/BSON document store
  - Oracle SODA-like API
  - Aggregation pipelines

- **spatial/**: Geospatial database
  - R-Tree indexing
  - Network routing
  - Raster support
  - Spatial queries

- **ml/**: Machine learning models
  - `algorithms.rs` - ML algorithms
  - Regression models
  - Decision trees
  - Clustering
  - Neural networks

- **ml_engine/**: ML execution engine
  - In-database ML execution
  - Model training and inference

- **inmemory/**: In-memory column store
  - SIMD vectorization
  - Columnar storage
  - High-performance analytics

### Additional Modules

**Location**: `src/[module]/`

- **concurrent/**: Lock-free data structures
  - Lock-free queue, stack, hash map, skip list
  - Work-stealing deque
  - Epoch-based reclamation

- **compression/**: Data compression
  - `algorithms.rs` - Compression algorithms
  - LZ4, Snappy, Zstd support

- **procedures/**: Stored procedures
  - `parser.rs` - PL/SQL-like parser
  - Procedure execution

- **triggers/**: Database triggers
  - Row-level and statement-level triggers
  - Trigger execution

- **event_processing/**: Complex Event Processing (CEP)
  - `cep.rs` - CEP engine
  - `operators.rs` - Stream operators
  - Real-time event processing

- **analytics/**: Analytics engine
  - OLAP operations
  - Analytical queries

- **performance/**: Performance optimization
  - Performance monitoring
  - Query profiling

- **operations/**: Operational features
  - `resources.rs` - Resource management

- **workload/**: Workload management
  - Resource groups
  - Query prioritization

- **streams/**: Data streaming
  - Change Data Capture (CDC)
  - Stream processing
  - Pub/Sub

- **catalog/**: System catalog
  - Schema management
  - Metadata storage

- **constraints/**: Constraint management
  - Primary keys, foreign keys
  - Unique and check constraints

- **flashback/**: Flashback operations
  - Time-travel queries
  - Flashback database

- **blockchain/**: Blockchain integration
  - Immutable audit logs
  - Cryptographic verification

- **multitenancy/** / **multitenant/**: Multi-tenant support
  - Tenant isolation
  - Resource governance

- **autonomous/**: Autonomous features
  - Self-tuning
  - Automatic optimization

- **enterprise/**: Enterprise integrations

- **orchestration/**: System orchestration

- **core/**: Core utilities

- **bench/**: Benchmarking utilities

### Configuration

Default configuration in `Config` struct (see `src/lib.rs`):
- Data directory: `./data`
- Page size: 4096 bytes (4 KB)
- Buffer pool: 1000 pages (~4 MB)
- Server port: 5432
- Max connections: 100

## Key Patterns

### Error Handling

All functions return `Result<T>` using `DbError` from `error.rs`. Use `?` operator for propagation.

```rust
use crate::error::{DbError, Result};

pub fn my_function() -> Result<()> {
    // Operations that may fail
    some_operation()?;
    Ok(())
}
```

### Component Lifecycle

Implement the `Component` trait from `common.rs` for components requiring initialization/shutdown:

```rust
use crate::common::{Component, HealthStatus};
use crate::error::Result;

impl Component for MyComponent {
    fn initialize(&mut self) -> Result<()> {
        // Initialize resources
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        // Clean up resources
        Ok(())
    }

    fn health_check(&self) -> HealthStatus {
        // Return component health
        HealthStatus::Healthy
    }
}
```

### Module Dependencies

- All modules depend on `error` and `common` for shared types
- Higher layers (execution, network, api) depend on lower layers (storage, transaction)
- Avoid circular dependencies
- Use dependency injection for testability

### Async/Await

- Use Tokio runtime for async operations
- Prefer async I/O for network and disk operations
- Use `tokio::spawn` for concurrent tasks

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn async_operation() -> Result<()> {
    // Async operations
    Ok(())
}
```

### Thread Safety

- Use `Arc<Mutex<T>>` or `Arc<RwLock<T>>` for shared mutable state
- Prefer lock-free data structures from `concurrent/` module where applicable
- Be mindful of deadlocks

## Feature Flags

- `simd`: Enable SIMD optimizations (AVX2/AVX-512)
- `iocp`: Enable Windows IOCP support
- `io_uring`: Enable Linux io_uring support

## Module Refactoring Guidelines

When refactoring large files:

1. **Target Size**: Aim for <500 lines per file
2. **Logical Grouping**: Group related functionality together
3. **Preserve APIs**: Keep public APIs intact, use re-exports in mod.rs
4. **Naming Conventions**: Use descriptive submodule names
5. **Documentation**: Update module-level documentation
6. **Testing**: Ensure tests still pass after refactoring

Example structure:
```
module/
├── mod.rs          # Public API, re-exports
├── core.rs         # Core functionality
├── types.rs        # Type definitions
├── operations.rs   # Operations
└── utils.rs        # Utilities
```

## Common Issues and Solutions

### Import Issues
- Use `crate::` for absolute imports within the project
- Use `super::` for parent module imports
- Use `self::` for current module imports

### Type Mismatches
- Check that types match exactly (no implicit conversions)
- Use `.into()` or `.try_into()` for type conversions
- Ensure generic type parameters are correctly specified

### Trait Bounds
- Ensure all required trait bounds are specified
- Use `where` clauses for complex bounds
- Import traits that provide methods

### Lifetime Issues
- Use explicit lifetime annotations where needed
- Prefer owned types over references when lifetime is unclear
- Use `'static` lifetime for data that lives for program duration

## Testing

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test storage::
cargo test security::memory_hardening

# Run with output
cargo test -- --nocapture

# Run benchmarks
cargo bench

# Check without building
cargo check
```

## Documentation

- Use `///` for public API documentation
- Use `//!` for module-level documentation
- Include examples in doc comments
- Run `cargo doc --open` to view generated documentation

## Performance Considerations

- Use SIMD operations for data-intensive tasks (enable `simd` feature)
- Leverage buffer pool to minimize disk I/O
- Use prepared statements for repeated queries
- Consider using lock-free data structures for high concurrency
- Profile before optimizing: `cargo bench`

## Security Considerations

- Always validate user input
- Use parameterized queries to prevent SQL injection
- Encrypt sensitive data at rest and in transit
- Implement proper authentication and authorization
- Follow principle of least privilege
- Enable all security modules in production

## References

- **docs/ARCHITECTURE.md** - Detailed architecture documentation
- **docs/DEVELOPMENT.md** - Development guidelines
- **docs/SECURITY_ARCHITECTURE.md** - Security design
- **.scratchpad/COORDINATION_MASTER.md** - Current refactoring coordination

---

*Last Updated: 2025-12-09*
