//! # RustyDB - Enterprise-Grade Database Management System
//!
//! RustyDB is a high-performance, Oracle-compatible database management system written in Rust.
//! It provides ACID compliance, advanced SQL features, clustering, and enterprise-grade security.
//!
//! ## Architecture Overview
//!
//! RustyDB is organized into modular components with clear separation of concerns:
//!
//! ### Core Engine Modules
//!
//! - **storage**: Low-level data persistence, buffer management, and disk I/O
//! - **transaction**: MVCC, transaction management, and concurrency control
//! - **index**: Multiple index types (B-tree, LSM, spatial, full-text)
//! - **execution**: Query execution engine with vectorization and JIT
//! - **parser**: SQL parsing and query planning
//! - **catalog**: System catalog and metadata management
//!
//! ### Enterprise Features
//!
//! - **security**: RBAC, encryption, authentication, and audit logging
//! - **clustering**: Distributed consensus, sharding, and high availability
//! - **replication**: Multi-datacenter replication and failover
//! - **backup**: Point-in-time recovery and disaster recovery
//! - **monitoring**: Real-time metrics, profiling, and resource governance
//! - **analytics**: OLAP processing, columnar storage, and materialized views
//!
//! ### Advanced SQL Features
//!
//! - **procedures**: Stored procedures and user-defined functions
//! - **triggers**: Event-driven database triggers
//! - **constraints**: Advanced constraint checking
//! - **operations**: Database operations and maintenance
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rusty_db::{Config, Result};
//! use rusty_db::network::Server;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let config = Config::default();
//!     let server = Server::new();
//!     server.run(&format!("127.0.0.1:{}", config.port)).await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Module Integration
//!
//! For detailed information about module interactions, API contracts, and development
//! guidelines, see `MASTER_COORDINATION.md` in the repository root.
//!
//! ## Key Concepts
//!
//! ### Transactions
//!
//! ```rust,no_run
//! # use rusty_db::common::IsolationLevel;
//! # use rusty_db::transaction::TransactionManager;
//! # fn example() -> rusty_db::Result<()> {
//! let mut txn_mgr = TransactionManager::new();
//! let txn_id = txn_mgr.begin(IsolationLevel::ReadCommitted)?;
//! // Perform operations...
//! txn_mgr.commit(txn_id)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Security
//!
//! ```rust,no_run
//! # use rusty_db::security::SecurityManager;
//! # fn example() -> rusty_db::Result<()> {
//! let security = SecurityManager::new();
//! let session = security.authenticate("user", "password")?;
//! // Check permissions...
//! # Ok(())
//! # }
//! ```

// ============================================================================
// Core Modules - Foundation layer (no dependencies on other app modules)
// ============================================================================

/// Error types and result handling
///
/// Provides unified error types used across all modules with detailed error contexts.
pub mod error;

/// Common types, traits, and interfaces
///
/// Shared data structures and trait definitions for inter-module communication.
/// All modules should use types from this module for consistency.
pub mod common;

// ============================================================================
// Storage Layer - Data persistence and buffer management
// ============================================================================

/// Storage engine for data persistence
///
/// Provides low-level disk I/O, page management, buffer pool, and copy-on-write support.
/// This is the foundation for all data storage in RustyDB.
///
/// **Key Components:**
/// - `DiskManager`: Direct disk I/O operations
/// - `BufferPoolManager`: In-memory page cache with LRU eviction
/// - `Page`: Fixed-size data pages
/// - `PartitionManager`: Table partitioning support
///
/// **Target LOC:** 3,000+ lines
pub mod storage;

/// System catalog for metadata management
///
/// Manages database metadata including tables, columns, indexes, and constraints.
pub mod catalog;

/// Index structures for fast data access
///
/// Provides multiple index types: B-tree, LSM-tree, hash, spatial, and full-text.
///
/// **Key Components:**
/// - `BTreeIndex`: B+ tree for ordered data
/// - `LsmTreeIndex`: Log-structured merge tree
/// - `SpatialIndex`: R-tree for spatial queries
/// - `FullTextIndex`: Inverted index for text search
///
/// **Target LOC:** 3,000+ lines
pub mod index;

// ============================================================================
// Transaction Layer - Concurrency control and ACID properties
// ============================================================================

/// Transaction management and MVCC
///
/// Implements multi-version concurrency control, transaction isolation,
/// lock management, and deadlock detection.
///
/// **Key Components:**
/// - `TransactionManager`: Transaction lifecycle management
/// - `MvccVersionStore`: Multi-version storage
/// - `LockManager`: Fine-grained locking
/// - `DeadlockDetector`: Deadlock prevention and detection
///
/// **Target LOC:** 3,000+ lines
pub mod transaction;

// ============================================================================
// Query Processing - SQL parsing and execution
// ============================================================================

/// SQL parser and query planner
///
/// Parses SQL statements into abstract syntax trees and generates execution plans.
pub mod parser;

/// Query execution engine
///
/// Executes query plans with vectorization, parallelization, and adaptive optimization.
///
/// **Key Components:**
/// - `Executor`: Main execution engine
/// - `Planner`: Physical plan generation
/// - `Optimizer`: Cost-based optimization
/// - `ParallelExecutor`: Parallel query execution
///
/// **Target LOC:** 3,000+ lines
pub mod execution;

// ============================================================================
// Network Layer - Client/server communication
// ============================================================================

/// Network protocol and server
///
/// Implements the wire protocol, connection pooling, and TCP server.
pub mod network;

// ============================================================================
// Enterprise Features
// ============================================================================

/// Security, authentication, and authorization
///
/// Provides RBAC, encryption at rest and in transit, audit logging, and
/// multi-factor authentication.
///
/// **Key Components:**
/// - `SecurityManager`: Central security coordinator
/// - `AuthenticationProvider`: Multi-method authentication
/// - `RbacEngine`: Role-based access control
/// - `EncryptionManager`: Data encryption
/// - `AuditLogger`: Comprehensive audit trails
///
/// **Target LOC:** 3,000+ lines
pub mod security;

/// Monitoring, metrics, and profiling
///
/// Real-time performance metrics, query profiling, and resource governance.
///
/// **Key Components:**
/// - `MetricsCollector`: System-wide metrics
/// - `QueryProfiler`: Query performance analysis
/// - `ResourceGovernor`: Resource limit enforcement
/// - `HealthChecker`: Health monitoring
///
/// **Target LOC:** 3,000+ lines
pub mod monitoring;

/// Backup and recovery
///
/// Point-in-time recovery, incremental backups, and disaster recovery.
///
/// **Key Components:**
/// - `BackupManager`: Backup orchestration
/// - `IncrementalBackup`: Incremental backup support
/// - `PointInTimeRecovery`: PITR functionality
/// - `SnapshotManager`: Consistent snapshots
///
/// **Target LOC:** 3,000+ lines
pub mod backup;

/// Constraint checking and validation
///
/// Enforces integrity constraints including foreign keys, unique constraints, and check constraints.
pub mod constraints;

/// Analytics and OLAP processing
///
/// Columnar storage, approximate query processing, and materialized views.
///
/// **Key Components:**
/// - `ColumnarStorage`: Column-oriented storage
/// - `MaterializedViewManager`: Materialized views
/// - `ApproximateQueryProcessor`: Sampling and sketching
/// - `WindowFunctionExecutor`: Window functions
///
/// **Target LOC:** 3,000+ lines
pub mod analytics;

/// Database operations and maintenance
///
/// Administrative operations, vacuum, analyze, and maintenance tasks.
pub mod operations;

// ============================================================================
// Advanced SQL Features
// ============================================================================

/// Stored procedures and user-defined functions
///
/// PL/SQL-compatible procedural language with control flow, exception handling, and cursors.
///
/// **Key Components:**
/// - `ProcedureExecutor`: Procedure execution engine
/// - `ProcedureCompiler`: Bytecode compilation
/// - `FunctionRegistry`: UDF management
/// - `CursorManager`: Cursor support
///
/// **Target LOC:** 3,000+ lines
pub mod procedures;

/// Database triggers
///
/// Event-driven triggers for before/after insert, update, delete operations.
pub mod triggers;

/// Data replication
///
/// Multi-datacenter replication, logical and physical replication, and conflict resolution.
pub mod replication;

// ============================================================================
// Clustering and High Availability
// ============================================================================

/// Clustering and distributed consensus
///
/// Raft-based consensus, automatic sharding, failover, and multi-datacenter support.
///
/// **Key Components:**
/// - `ClusterManager`: Cluster topology management
/// - `ConsensusEngine`: Raft consensus algorithm
/// - `ShardingManager`: Automatic data sharding
/// - `FailoverController`: Automatic failover
///
/// **Target LOC:** 3,000+ lines
pub mod clustering;

// ============================================================================
// Performance Optimization
// ============================================================================

/// Performance optimization utilities
///
/// Query caching, statistics collection, and performance tuning.
pub mod performance;

// ============================================================================
// Re-exports for convenience
// ============================================================================

pub use error::{DbError, Result};
pub use common::{
    Component, Transactional, Recoverable, Monitorable, ReplicableState,
    IsolationLevel, TransactionId, PageId, TableId, IndexId, Value, Tuple, Schema,
    DatabaseConfig, HealthStatus, SystemEvent, ResourceLimits,
};

// ============================================================================
// Database Configuration (backward compatibility)
// ============================================================================

/// Database configuration (deprecated - use common::DatabaseConfig)
///
/// This is kept for backward compatibility. New code should use `common::DatabaseConfig`.
#[deprecated(since = "0.1.0", note = "Use common::DatabaseConfig instead")]
#[derive(Debug, Clone)]
pub struct Config {
    pub data_dir: String,
    pub page_size: usize,
    pub buffer_pool_size: usize,
    pub port: u16,
}

#[allow(deprecated)]
impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: "./data".to_string(),
            page_size: 4096,
            buffer_pool_size: 1000,
            port: 5432,
        }
    }
}

// ============================================================================
// Library Information
// ============================================================================

/// RustyDB version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// RustyDB build information
pub const BUILD_INFO: &str = concat!(
    "RustyDB v",
    env!("CARGO_PKG_VERSION"),
    " - Enterprise Database Management System"
);

/// Print library information
pub fn print_info() {
    println!("{}", BUILD_INFO);
    println!("Rust Version: {}", env!("CARGO_PKG_RUST_VERSION"));
}
