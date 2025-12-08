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

/// Advanced Compression Engine
///
/// Oracle-like HCC (Hybrid Columnar Compression) and advanced compression algorithms
/// for both OLAP and OLTP workloads with deduplication and tiered compression.
///
/// **Key Components:**
/// - `HCCEngine`: Hybrid Columnar Compression for OLAP
/// - `OLTPCompressor`: Update-friendly compression for transactional data
/// - `DedupEngine`: Block-level deduplication with content-defined chunking
/// - `TieredCompressionManager`: Temperature-based compression tiers
/// - `Algorithms`: LZ4, Zstandard, Dictionary, Huffman, Arithmetic coding
///
/// **Target LOC:** 3,000+ lines
pub mod compression;

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

/// Flashback technology
///
/// Oracle-like time travel and point-in-time recovery capabilities.
///
/// **Key Components:**
/// - `TimeTravelEngine`: AS OF TIMESTAMP/SCN queries
/// - `VersionManager`: VERSIONS BETWEEN and row version tracking
/// - `TableRestoreManager`: FLASHBACK TABLE implementation
/// - `DatabaseFlashbackManager`: FLASHBACK DATABASE recovery
/// - `TransactionFlashbackManager`: FLASHBACK TRANSACTION analysis
///
/// **Target LOC:** 3,000+ lines
pub mod flashback;

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

/// In-Memory Column Store
///
/// Oracle-like in-memory database option with SIMD vectorization, dual-format architecture,
/// and advanced compression for analytical workloads.
///
/// **Key Components:**
/// - `ColumnStore`: Dual-format (row+column) storage
/// - `HybridCompressor`: Advanced compression algorithms (dictionary, RLE, bit-packing, delta)
/// - `VectorizedFilter`: SIMD-accelerated filtering
/// - `VectorizedAggregator`: SIMD aggregations (SUM, AVG, MIN, MAX)
/// - `PopulationManager`: Background population from disk
/// - `HashJoinEngine`: Vectorized hash joins with Bloom filters
///
/// **Target LOC:** 3,000+ lines
pub mod inmemory;

/// Multi-Tenant Architecture
///
/// Oracle-like Pluggable Database (PDB) / Container Database (CDB) architecture with
/// complete tenant isolation, resource governance, and self-service provisioning.
///
/// **Key Components:**
/// - `ContainerDatabase`: CDB root container with PDB lifecycle management
/// - `Tenant`: Tenant-level isolation and resource controls
/// - `MemoryIsolator`: Memory isolation using Rust ownership
/// - `IoBandwidthAllocator`: I/O bandwidth allocation with token bucket
/// - `CpuScheduler`: Fair share CPU scheduling
/// - `ConsolidationPlanner`: Intelligent tenant placement and rebalancing
/// - `ProvisioningService`: Self-service tenant provisioning
///
/// **Target LOC:** 3,000+ lines
pub mod multitenancy;

// ============================================================================
// Graph Database Engine
// ============================================================================

/// Graph database engine
///
/// Property graph database with PGQL-like queries, graph algorithms, and analytics.
///
/// **Key Components:**
/// - `PropertyGraph`: Vertices, edges, and properties with multi-graph support
/// - `QueryEngine`: PGQL-like pattern matching and path queries
/// - `Algorithms`: PageRank, centrality, community detection, clustering
/// - `Storage`: Adjacency list, CSR format, compression, and persistence
/// - `Analytics`: Temporal graphs, ML features, and recommendations
///
/// **Target LOC:** 3,000+ lines
pub mod graph;

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
// Enterprise Integration Layer
// ============================================================================

/// Enterprise integration and orchestration
///
/// Provides enterprise-grade infrastructure for coordinating all subsystems including
/// service bus, configuration management, feature flags, lifecycle management, and
/// cross-cutting concerns.
///
/// **Key Components:**
/// - `ServiceBus`: Async message routing and event-driven architecture
/// - `ConfigManager`: Hierarchical configuration with hot-reload
/// - `FeatureFlagManager`: Runtime feature toggles and A/B testing
/// - `LifecycleManager`: Graceful startup/shutdown orchestration
/// - `Cross-Cutting`: Tracing, circuit breakers, rate limiting
///
/// **Target LOC:** 3,000+ lines
pub mod enterprise;

// ============================================================================
// Streaming and Change Data Capture
// ============================================================================

/// Streams and Change Data Capture (CDC)
///
/// Change data capture, event streaming, logical replication, and integration patterns.
///
/// **Key Components:**
/// - `CDCEngine`: Change data capture from WAL
/// - `EventPublisher`: Kafka-like event publishing
/// - `EventSubscriber`: Consumer groups and subscriptions
/// - `LogicalReplication`: Table-level replication
/// - `OutboxProcessor`: Transactional outbox pattern
/// - `EventStore`: Event sourcing support
/// - `CQRSCoordinator`: CQRS pattern implementation
///
/// **Target LOC:** 3,000+ lines
pub mod streams;

// ============================================================================
// Machine Learning
// ============================================================================

/// In-database machine learning engine
///
/// Provides comprehensive ML capabilities directly within the database,
/// including model training, inference, and SQL integration without data export.
///
/// **Key Components:**
/// - `MLEngine`: Core ML orchestration and model lifecycle management
/// - `Algorithm`: Pure Rust ML algorithm implementations
/// - `Preprocessor`: Data preprocessing and feature engineering
/// - `InferenceEngine`: Real-time and batch prediction
/// - `MLSqlParser`: SQL syntax extensions for ML operations
///
/// **Supported Algorithms:**
/// - Linear Regression
/// - Logistic Regression
/// - Decision Trees (CART)
/// - Random Forests
/// - K-Means Clustering
/// - Naive Bayes
///
/// **Target LOC:** 3,000+ lines
pub mod ml;

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

// ============================================================================
// Autonomous Database Features
// ============================================================================

/// Autonomous database with ML-driven optimization
///
/// Self-driving database capabilities including automatic tuning, self-healing,
/// intelligent indexing, and predictive analytics.
///
/// **Key Components:**
/// - `AutoTuner`: ML-driven parameter optimization with reinforcement learning
/// - `SelfHealingEngine`: Automatic issue detection and repair
/// - `WorkloadMLAnalyzer`: Workload classification and prediction
/// - `AutoIndexingEngine`: Intelligent index management
/// - `CapacityPlanner`: Predictive capacity planning
///
/// **Target LOC:** 3,000+ lines
pub mod autonomous;

/// Blockchain tables and immutable ledgers
pub mod blockchain;
