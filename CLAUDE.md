# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build the project
cargo build --release

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test storage::
cargo test transaction::

# Start the database server (default port: 5432)
cargo run --bin rusty-db-server

# Start the CLI client
cargo run --bin rusty-db-cli

# Run benchmarks
cargo bench
```

## Project Status

**Current compilation status has errors.** Check git status for modified files needing attention.

## Architecture Overview

RustyDB is an enterprise-grade, Oracle-compatible database management system written in Rust. The codebase is organized into layered modules:

### Core Foundation Layer
- **error.rs**: Unified `DbError` enum with `thiserror` - all modules use `Result<T> = std::result::Result<T, DbError>`
- **common.rs**: Shared type aliases (`TransactionId`, `PageId`, `TableId`, `IndexId`, `SessionId`) and core traits (`Component`, `Transactional`, `Recoverable`, `Monitorable`)

### Storage Layer
- **storage/**: Page-based storage (4KB pages), disk I/O, buffer pool, partitioning
- **buffer/**: High-performance buffer manager with pluggable eviction (CLOCK, LRU, 2Q, LRU-K), lock-free page table
- **memory/**: Slab allocator, arena allocator, large object allocator, memory pressure management
- **io/**: Cross-platform async I/O (Windows IOCP, Linux io_uring), direct I/O, ring buffers

### Transaction Layer
- **transaction/**: MVCC, transaction lifecycle, lock manager, deadlock detection, WAL

### Query Processing
- **parser/**: SQL parsing using `sqlparser` crate
- **execution/**: Query executor, planner, cost-based optimizer, parallel execution, vectorized operations
- **optimizer_pro/**: Advanced cost-based optimization, adaptive execution, SQL plan baselines

### Index Structures
- **index/**: B-tree, LSM-tree, hash, spatial (R-tree), full-text, bitmap, partial indexes
- **simd/**: SIMD-accelerated filtering, aggregation, string operations

### Networking
- **network/**: TCP server, wire protocol, connection management
- **api/**: REST API (axum-based), GraphQL, OpenAPI documentation

### Enterprise Features
- **security/**: RBAC, encryption, authentication, audit logging (10 security modules)
- **security_vault/**: TDE, data masking, key management, VPD
- **clustering/**: Raft consensus, sharding, failover, geo-replication
- **rac/**: Cache Fusion (Oracle RAC-like), global resource directory, parallel query
- **replication/**, **advanced_replication/**: Multi-master, logical replication, CRDT-based conflict resolution
- **backup/**: Full/incremental backups, PITR, disaster recovery
- **monitoring/**: Metrics, profiling, resource governance, health checks

### Specialized Engines
- **graph/**: Property graph database with PGQL-like queries, graph algorithms
- **document_store/**: JSON/BSON document store (Oracle SODA-like), aggregation pipelines
- **spatial/**: Geospatial (R-tree, network routing, raster support)
- **ml/**, **ml_engine/**: In-database ML (regression, trees, clustering, neural networks)
- **inmemory/**: In-memory column store with SIMD vectorization

### Concurrent Data Structures
- **concurrent/**: Lock-free queue, stack, hash map, skip list, work-stealing deque, epoch-based reclamation

### Configuration
Default configuration in `Config` struct:
- Data directory: `./data`
- Page size: 4096 bytes
- Buffer pool: 1000 pages
- Server port: 5432

## Key Patterns

### Error Handling
All functions return `Result<T>` using `DbError` from `error.rs`. Use `?` operator for propagation.

### Component Lifecycle
Implement the `Component` trait from `common.rs` for components requiring initialization/shutdown:
```rust
impl Component for MyComponent {
    fn initialize(&mut self) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
    fn health_check(&self) -> HealthStatus;
}
```

### Module Dependencies
Modules depend on `error` and `common` for shared types. Higher layers (execution, network) depend on lower layers (storage, transaction).

## Feature Flags

- `simd`: Enable SIMD optimizations
- `iocp`: Enable Windows IOCP support
