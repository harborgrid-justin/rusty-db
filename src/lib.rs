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

/// High-performance buffer manager optimized for Windows/MSVC
///
/// Enterprise-grade buffer pool management system with zero-allocation hot path,
/// lock-free page table, per-core frame pools, and batch flush support.
///
/// **Key Components:**
/// - `BufferPoolManager`: Main buffer pool with pluggable eviction policies
/// - `PageCache`: Page-aligned buffers for direct I/O (4KB aligned)
/// - `EvictionPolicy`: CLOCK, LRU, 2Q, and LRU-K replacement algorithms
/// - `FrameGuard`: RAII pin/unpin management
///
/// **Performance Features:**
/// - Zero allocations in pin/unpin hot path
/// - Lock-free page table with partitioned hash maps
/// - Per-core frame pools to reduce contention
/// - Batch flush support for efficient sequential I/O
/// - Windows IOCP integration ready
///
/// **Target LOC:** 3,000+ lines
pub mod buffer;

/// Enterprise Memory Allocator System
///
/// Comprehensive memory management with multiple allocation strategies
/// optimized for database workloads.
///
/// **Key Components:**
/// - `SlabAllocator`: Size-class based allocation with thread-local caching
/// - `ArenaAllocator`: Bump allocation for per-query memory contexts
/// - `LargeObjectAllocator`: Direct mmap for huge allocations with huge page support
/// - `MemoryPressureManager`: Global memory monitoring and OOM prevention
/// - `MemoryDebugger`: Leak detection, profiling, and corruption detection
///
/// **Features:**
/// - Magazine-layer CPU caching for slab allocation
/// - Hierarchical memory contexts with automatic cleanup
/// - Huge page support (2MB, 1GB)
/// - Memory pressure callbacks and emergency release
/// - Comprehensive debugging with stack traces
///
/// **Target LOC:** 3,000+ lines
pub mod memory;

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

/// Advanced Security Vault Engine
///
/// Oracle-like comprehensive security vault providing enterprise-grade data protection,
/// transparent data encryption (TDE), data masking, hierarchical key management,
/// tamper-evident audit trails, and virtual private database (VPD) capabilities.
///
/// **Key Components:**
/// - `TdeEngine`: Transparent Data Encryption (tablespace & column-level)
/// - `MaskingEngine`: Static and dynamic data masking with FPE
/// - `KeyStore`: Hierarchical key management (MEK/DEK) with envelope encryption
/// - `AuditVault`: Fine-grained auditing with blockchain-backed tamper detection
/// - `VpdEngine`: Virtual Private Database with row-level security
/// - `PrivilegeAnalyzer`: Least privilege analysis and role mining
///
/// **Target LOC:** 3,000+ lines
pub mod security_vault;

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

/// Multi-Tenant Architecture Engine
///
/// Oracle Multitenant-compatible architecture with Pluggable Databases (PDBs),
/// Container Database (CDB), hot cloning, online relocation, and advanced resource isolation.
///
/// **Key Components:**
/// - `ContainerDatabase`: CDB root with PDB lifecycle management
/// - `PluggableDatabase`: Fully isolated tenant databases with lifecycle operations
/// - `ResourceIsolator`: Per-tenant CPU, memory, I/O, and storage limits
/// - `TenantProvisioningService`: Automated tenant onboarding workflows
/// - `CloningEngine`: Hot cloning with copy-on-write and snapshot support
/// - `RelocationEngine`: Online PDB migration with minimal downtime
/// - `SharedServices`: Common users, undo, and temp tablespaces
/// - `MeteringEngine`: Usage tracking, billing, and quota enforcement
///
/// **Innovations:**
/// - Kubernetes-native tenant management
/// - Serverless PDB scaling
/// - Cross-cloud tenant portability
/// - AI-driven resource optimization
///
/// **Target LOC:** 3,000+ lines
pub mod multitenant;

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

/// Advanced Replication Engine
///
/// Enterprise-grade replication system with Oracle-like capabilities including multi-master
/// replication, logical replication, sharding, and global data services.
///
/// **Key Components:**
/// - `MultiMasterReplication`: Bidirectional replication with conflict detection and quorum writes
/// - `LogicalReplication`: Row-level replication with filtering, transformation, and schema evolution
/// - `ShardingEngine`: Hash, range, list, and composite sharding with auto-rebalancing
/// - `GlobalDataServices`: Region-aware routing, load balancing, and failover
/// - `ConflictResolver`: CRDT-based and traditional conflict resolution strategies
/// - `ReplicationMonitor`: Real-time monitoring with alerts and dashboards
/// - `ApplyEngine`: Parallel change application with dependency tracking
/// - `XaTransactionManager`: Distributed two-phase commit protocol
///
/// **Key Innovations:**
/// - CRDT-based conflict-free replication (LWW-Register, G-Counter, PN-Counter, OR-Set)
/// - ML-based conflict prediction and prevention
/// - Adaptive replication topology based on workload
/// - Zero-downtime shard migration
///
/// **Target LOC:** 3,000+ lines
pub mod advanced_replication;

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

/// Real Application Clusters (RAC) Engine
///
/// Oracle RAC-like clustering with Cache Fusion technology for shared-disk clustering,
/// high availability, and horizontal scalability across multiple instances.
///
/// **Key Components:**
/// - `CacheFusionCoordinator`: Direct memory-to-memory block transfers with GCS/GES
/// - `GlobalResourceDirectory`: Resource ownership tracking and dynamic remastering
/// - `ClusterInterconnect`: High-speed message passing and heartbeat monitoring
/// - `InstanceRecoveryManager`: Automatic failure detection and recovery
/// - `ParallelQueryCoordinator`: Cross-instance parallel query execution
///
/// **Features:**
/// - Cache Fusion protocol with zero-copy transfers
/// - Global Cache Service (GCS) for block management
/// - Global Enqueue Service (GES) for distributed locking
/// - Split-brain detection and network partition handling
/// - Automatic instance recovery with redo log replay
/// - Parallel query execution across cluster nodes
/// - Resource affinity and load balancing
///
/// **Target LOC:** 3,000+ lines
pub mod rac;

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

/// Orchestration Framework
///
/// Provides comprehensive orchestration infrastructure for coordinating all RustyDB
/// enterprise modules with actor-based coordination, service registry, dependency
/// injection, circuit breakers, health aggregation, and graceful degradation.
///
/// **Key Components:**
/// - `ActorSystem`: Actor-based coordination with async message passing
/// - `ServiceRegistry`: Unified service registry and dependency injection
/// - `DependencyGraph`: Dependency resolution with cycle detection
/// - `CircuitBreaker`: Fault tolerance and cascading failure prevention
/// - `HealthAggregator`: Health monitoring and aggregation
/// - `PluginRegistry`: Plugin architecture for extensibility
/// - `DegradationStrategy`: Graceful degradation under load
/// - `RecoveryManager`: Unified error handling and recovery
///
/// **Features:**
/// - Actor-based coordination using async message passing
/// - Circuit breaker patterns for fault tolerance
/// - Service mesh-like internal communication
/// - Dynamic feature flag system
/// - Unified lifecycle management for all components
/// - Health aggregation and cascading failure prevention
/// - Dependency graph resolution with cycle detection
/// - Graceful degradation strategies
///
/// **Target LOC:** 3,000+ lines
pub mod orchestration;

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

/// Event Processing and Complex Event Processing (CEP) Engine
///
/// Comprehensive event stream processing with Oracle Streams-like capabilities,
/// complex event processing, windowing, continuous queries, and event sourcing.
///
/// **Key Components:**
/// - `EventStream`: Event stream management with partitioning and consumer groups
/// - `PatternMatcher`: CEP with MATCH_RECOGNIZE-like pattern matching
/// - `WindowManager`: Tumbling, sliding, session, and hopping windows
/// - `StreamOperators`: Filter, map, aggregations, joins, TopN, deduplication
/// - `ContinuousQuery`: Continuous queries with incremental view maintenance
/// - `EventStore`: Event sourcing with snapshots and replay
/// - `Connectors`: Kafka-compatible, JDBC, file, HTTP webhook connectors
/// - `StreamAnalytics`: Real-time dashboards, anomaly detection, trend analysis
///
/// **Features:**
/// - Out-of-order event handling with watermarks
/// - Exactly-once processing semantics
/// - Incremental checkpointing for fault tolerance
/// - GPU-accelerated pattern matching
/// - ML model serving in streams
/// - Event correlation and hierarchies
/// - Predictive analytics and alert generation
///
/// **Target LOC:** 3,000+ lines
pub mod event_processing;

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

/// Advanced In-Database Machine Learning Engine
///
/// Next-generation ML engine with comprehensive production features including AutoML,
/// time series forecasting, federated learning, and GPU acceleration.
///
/// **Key Components:**
/// - `MLEngine`: Core orchestration with zero-copy integration and GPU support
/// - `Algorithms`: Linear/Logistic Regression, Decision Trees, Random Forest,
///   Gradient Boosting, K-Means, DBSCAN, Naive Bayes, SVM, Neural Networks
/// - `FeatureEngine`: Normalization, standardization, one-hot encoding, binning,
///   imputation, polynomial features, feature selection
/// - `ModelStore`: Versioning, serialization, A/B testing, deployment pipeline
/// - `ScoringEngine`: Batch/real-time predictions, PMML import/export, SHAP-like explanations
/// - `AutoMLEngine`: Automated algorithm selection, hyperparameter tuning, cross-validation
/// - `TimeSeriesEngine`: ARIMA, Exponential Smoothing, seasonality detection, anomaly detection
/// - `TrainingEngine`: Mini-batch training, distributed coordination, early stopping,
///   learning rate scheduling
///
/// **Innovations:**
/// - Zero-copy integration with query engine
/// - GPU acceleration interface (CUDA/OpenCL)
/// - Federated learning support with differential privacy
/// - Incremental model updates without full retraining
/// - SQL-native ML operations
///
/// **Target LOC:** 3,700+ lines
pub mod ml_engine;

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

// ============================================================================
// Workload Intelligence
// ============================================================================

/// Workload intelligence and performance tuning
///
/// AWR-like workload repository, SQL Tuning Advisor, real-time monitoring,
/// and automatic diagnostic capabilities.
///
/// **Key Components:**
/// - `WorkloadRepository`: AWR-like automatic workload repository
/// - `SqlTuningAdvisor`: SQL performance tuning and recommendations
/// - `SqlMonitor`: Real-time SQL execution monitoring
/// - `PerformanceHub`: Unified performance dashboard
/// - `DiagnosticAdvisor`: ADDM-like automatic problem detection
///
/// **Target LOC:** 3,000+ lines
pub mod workload;

// ============================================================================
// Spatial Database Engine
// ============================================================================

/// Spatial database engine for geospatial data
///
/// Oracle Spatial-compatible geospatial database engine with comprehensive
/// vector and raster data support, spatial indexing, and analytical operations.
///
/// **Key Components:**
/// - `Geometry`: Point, LineString, Polygon, and complex geometries
/// - `SpatialIndex`: R-tree, Quadtree, and Grid-based spatial indexing
/// - `Operators`: Topological operators, distance calculations, buffer operations
/// - `Analysis`: K-nearest neighbors, clustering, Voronoi, Delaunay triangulation
/// - `SRS`: Coordinate reference systems and transformations
/// - `Raster`: Raster data types, algebra, and pyramids
/// - `Network`: Road network routing and optimization
///
/// **Features:**
/// - WKT/WKB/GeoJSON serialization
/// - 3D and measured geometries
/// - Spatial reference system transformations
/// - Geodetic calculations (great circle distance)
/// - Raster to vector conversion
/// - Network shortest path (Dijkstra, A*)
/// - Spatial clustering (DBSCAN, K-means)
///
/// **Target LOC:** 3,000+ lines
pub mod spatial;

// ============================================================================
// Document Store / JSON Database
// ============================================================================

/// JSON Document Store Engine (Oracle SODA-like)
///
/// Complete NoSQL document database with JSON/BSON support, aggregation pipelines,
/// change streams, and SQL/JSON integration.
///
/// **Key Components:**
/// - `DocumentStore`: Main document store interface
/// - `Document`: JSON document model with BSON and versioning
/// - `Collection`: Document collection with schema validation
/// - `JsonPath`: Full JSONPath query engine with filters
/// - `IndexManager`: B-tree, full-text, compound, and TTL indexes
/// - `QueryByExample`: MongoDB-like query syntax
/// - `AggregationPipeline`: $match, $project, $group, $sort, $limit, $unwind, $lookup
/// - `ChangeStreams`: Real-time change notifications with resume tokens
/// - `SqlJsonFunctions`: JSON_TABLE, JSON_QUERY, JSON_VALUE, JSON_EXISTS
///
/// **Features:**
/// - Multiple ID generation strategies (UUID, auto-increment, custom)
/// - Document versioning and metadata tracking
/// - Large document chunking support
/// - Schema validation with JSON Schema
/// - Collection-level settings and statistics
/// - Recursive descent and array slicing in JSONPath
/// - Full-text search with TF-IDF scoring
/// - Geospatial query support
/// - Multi-faceted aggregation
/// - Document diff generation
/// - IS JSON predicate
/// - JSON generation functions (JSON_OBJECT, JSON_ARRAY, JSON_MERGEPATCH)
///
/// **Target LOC:** 3,000+ lines
pub mod document_store;

// ============================================================================
// Query Optimizer Pro
// ============================================================================

/// Query Optimizer Pro - Advanced cost-based query optimization
///
/// Oracle-like query optimizer with advanced cost-based optimization, adaptive execution,
/// SQL plan management, and machine learning-based cardinality estimation.
///
/// **Key Components:**
/// - `CostModel`: CPU, I/O, network, and memory cost modeling with histogram-based cardinality estimation
/// - `PlanGenerator`: Bottom-up dynamic programming with join enumeration (bushy, left-deep, right-deep)
/// - `AdaptiveExecutor`: Runtime statistics feedback and automatic plan correction
/// - `PlanBaselineManager`: SQL Plan Baselines with plan capture, evolution, and stability guarantees
/// - `QueryTransformer`: Predicate pushdown, OR expansion, star transformation, MV rewrite
/// - `HintParser`: Oracle-compatible optimizer hints with validation and conflict resolution
///
/// **Key Features:**
/// - Multi-column statistics and selectivity estimation
/// - Access path selection (seq scan, index scan, bitmap scan)
/// - Join method selection (nested loop, hash, merge)
/// - Subquery unnesting and view merging
/// - Cardinality feedback loop and SQL Plan Directives
/// - Query fingerprinting and plan caching
/// - Machine learning-based cardinality estimation
/// - Parallel plan search
///
/// **Target LOC:** 3,000+ lines
pub mod optimizer_pro;

// ============================================================================
// Resource Management
// ============================================================================

/// Resource Manager for enterprise workload management
///
/// Oracle-like resource management with consumer groups, resource plans,
/// CPU/I/O/memory scheduling, parallel execution control, and session management.
///
/// **Key Components:**
/// - `ConsumerGroupManager`: Workload classification and user mapping
/// - `ResourcePlanManager`: Resource plan definitions and scheduling
/// - `CpuScheduler`: Fair-share and priority-based CPU scheduling
/// - `IoScheduler`: I/O bandwidth and IOPS limiting with priority queues
/// - `MemoryManager`: PGA limits, session quotas, and automatic memory management
/// - `ParallelExecutionController`: Parallel degree limits and auto DOP calculation
/// - `SessionController`: Active session pools and timeout management
///
/// **Innovations:**
/// - ML-based workload prediction
/// - Dynamic resource rebalancing
/// - Container-aware resource limits
/// - SLA-based resource allocation
///
/// **Target LOC:** 3,000+ lines
pub mod resource_manager;

// ============================================================================
// Session Management and Connection Pooling
// ============================================================================

/// Session Management & Connection Pooling System
///
/// Enterprise-grade session management and connection pooling with Oracle DRCP-like
/// capabilities, multi-method authentication, resource control, and comprehensive
/// lifecycle events and monitoring.
///
/// **Session Management Components:**
/// - `SessionManager`: Main session lifecycle coordinator
/// - `SessionState`: Session context preservation (variables, settings, transactions, cursors)
/// - `AuthenticationProvider`: Multi-method authentication (LDAP, Kerberos, SAML, tokens)
/// - `ResourceController`: Per-session resource quotas (CPU, memory, I/O, temp space)
/// - `SessionPool`: DRCP-like connection pooling with session multiplexing
/// - `SessionEventManager`: Lifecycle events (login, logoff, idle timeout, migration)
///
/// **Connection Pool Components:**
/// - `ConnectionPool`: Elastic pool with min/max sizing and throttling
/// - `ConnectionFactory`: Factory pattern for connection creation and validation
/// - `WaitQueue`: Fair/priority queuing with deadlock detection and starvation prevention
/// - `PartitionManager`: Multi-tenant pool partitioning and routing
/// - `LifetimeEnforcer`: Connection aging policies (time/usage/adaptive)
/// - `PoolStatistics`: Real-time metrics, leak detection, and dashboard data
/// - `RecyclingManager`: Connection state reset and recycling strategies
///
/// **Features:**
/// - Session state management with variable and cursor tracking
/// - Multi-method authentication with privilege caching
/// - Granular resource limits and consumer groups
/// - Tag-based session selection and affinity
/// - Session multiplexing with purity levels (NEW, SELF)
/// - Login/logoff triggers and state change callbacks
/// - Elastic pool sizing with min/max/initial configuration
/// - Connection creation throttling and lazy initialization
/// - Background connection maintenance and validation
/// - Statement and cursor caching per connection
/// - Fair and priority-based wait queues
/// - User/application/tenant-based pool partitioning
/// - Service-based routing and load balancing
/// - Comprehensive statistics and real-time monitoring
/// - Connection leak detection and alerting
/// - Prometheus/JSON/CSV metrics export
///
/// **Target LOC:** 6,000+ lines (3,000+ session management, 3,000+ connection pooling)
pub mod pool;

// ============================================================================
// I/O Layer - High-Performance I/O Engine
// ============================================================================

/// High-Performance I/O Layer
///
/// Cross-platform asynchronous I/O engine optimized for database workloads
/// with Windows IOCP and Unix io_uring support.
///
/// **Key Components:**
/// - `AsyncIoEngine`: Async I/O engine with completion-based model
/// - `FileManager`: High-level file manager with batched operations
/// - `IoRingBuffer`: Lock-free ring buffer for I/O queue
/// - `BufferPool`: Pre-allocated aligned buffer pool for Direct I/O
/// - `IoMetrics`: Comprehensive I/O metrics and performance monitoring
/// - `WindowsIocp`: Windows I/O Completion Ports implementation
/// - `IoUringEngine`: Linux io_uring implementation
///
/// **Features:**
/// - Direct I/O (bypass OS cache)
/// - Batched I/O operations (multiple pages per syscall)
/// - Zero-copy where possible
/// - Lock-free submission and completion queues
/// - Page-aligned buffers (4KB) for Direct I/O
/// - Fixed thread pool for I/O workers
/// - Cross-platform abstraction (Windows IOCP / Unix io_uring)
///
/// **Target LOC:** 3,000+ lines
pub mod io;

// ============================================================================
// SIMD Query Execution Engine
// ============================================================================

/// SIMD-Optimized Scan Engine
///
/// High-performance vectorized query execution using AVX2 SIMD instructions
/// for processing 8-16 elements per instruction with zero-allocation scan loops.
///
/// **Key Components:**
/// - `filter`: SIMD filter operations for predicate evaluation
/// - `scan`: Columnar scanning with late materialization
/// - `aggregate`: Vectorized SUM, COUNT, MIN, MAX, AVG operations
/// - `string`: SIMD string comparison and pattern matching
///
/// **Performance Features:**
/// - Processes 8-16 elements per SIMD instruction
/// - Zero allocations in scan loop
/// - Automatic fallback for non-AVX2 CPUs
/// - Cache-oblivious algorithms
/// - Explicit prefetch instructions for sequential access
///
/// **Target LOC:** 3,000+ lines
pub mod simd;

// ============================================================================
// Lock-Free Concurrent Data Structures
// ============================================================================

/// Lock-Free Concurrent Data Structures
///
/// High-performance lock-free data structures for concurrent database operations
/// with cache-line optimization and epoch-based memory reclamation.
///
/// **Key Components:**
/// - `LockFreeQueue`: Michael-Scott lock-free FIFO queue
/// - `LockFreeStack`: Treiber stack with ABA prevention
/// - `ConcurrentHashMap`: Fine-grained locking hash map
/// - `WorkStealingDeque`: Chase-Lev work-stealing deque
/// - `Epoch`: Epoch-based memory reclamation system
///
/// **Performance Features:**
/// - Cache-line aligned structures to prevent false sharing
/// - Minimal CAS retries with exponential backoff
/// - Optimized memory ordering (Relaxed where safe)
/// - Batch operations for improved throughput
/// - Per-core pools to reduce contention
///
/// **Safety:**
/// - Every unsafe block has safety comments
/// - Clear ownership semantics
/// - Documented memory ordering guarantees
///
/// **Target LOC:** 3,000+ lines
pub mod concurrent;

// ============================================================================
// Benchmarking Suite
// ============================================================================

/// Performance Benchmark Suite
///
/// Comprehensive benchmarking infrastructure for measuring and optimizing
/// critical database operations including storage, indexing, concurrency,
/// and SIMD operations.
///
/// **Key Components:**
/// - Page scan throughput benchmarks (sequential, random, filtered)
/// - Index lookup latency benchmarks (B-tree, hash, range scans)
/// - Buffer manager pin/unpin cycle benchmarks
/// - Lock-free queue operation benchmarks
/// - SIMD filter and aggregation benchmarks
/// - Transaction overhead benchmarks
/// - Memory allocation benchmarks
///
/// **Target LOC:** 1,200+ lines
pub mod bench;

// ============================================================================
// Core Integration Layer
// ============================================================================

/// Core Integration and Orchestration
///
/// Central coordination layer for all database subsystems including buffer pool,
/// I/O engine, worker pools, memory arenas, and lifecycle management.
///
/// **Key Components:**
/// - `DatabaseCore`: Main database coordinator with initialization and shutdown
/// - `BufferPoolManager`: Buffer pool with CLOCK eviction policy
/// - `IoEngine`: High-performance I/O engine with thread pool
/// - `WorkerPool`: Worker thread pool for query execution
/// - `MemoryArena`: Memory allocation and management
/// - `CoreMetrics`: Metrics collection and monitoring
///
/// **Initialization Phases:**
/// 1. Bootstrap: Configuration and logging
/// 2. Foundation: Memory and I/O subsystems
/// 3. Storage: Buffer pool initialization
/// 4. Execution: Worker pool setup
/// 5. Service: Monitoring and health checks
///
/// **Target LOC:** 1,700+ lines
pub mod core;

// ============================================================================
// REST API Layer
// ============================================================================

/// REST API Management Layer
///
/// Comprehensive REST API server exposing all database functionality via HTTP endpoints.
/// Built on axum web framework with full OpenAPI/Swagger documentation.
///
/// **Key Components:**
/// - `RestApiServer`: Main HTTP server with routing and middleware
/// - `ApiConfig`: Configuration for API server
/// - Core Database Operations: Query execution, transactions, CRUD operations
/// - Administration API: Config, backup, health checks, user/role management
/// - Monitoring API: Prometheus metrics, session stats, performance data
/// - Pool Management API: Connection pool configuration and monitoring
/// - Cluster Management API: Node management, topology, replication status
///
/// **Features:**
/// - OpenAPI/Swagger documentation
/// - Request validation and response pagination
/// - Rate limiting and CORS support
/// - WebSocket support for real-time query streaming
/// - Prometheus-compatible metrics endpoint
/// - Comprehensive error handling
///
/// **Target LOC:** 3,000+ lines
pub mod api;
