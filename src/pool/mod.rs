//! # Session Pool and Connection Management Module
//!
//! This module provides enterprise-grade session management and connection pooling
//! capabilities for RustyDB, including DRCP-like connection pooling, session lifecycle
//! management, and resource control.
//!
//! ## Modules
//!
//! - `session_manager`: Complete session management system with authentication,
//!   resource control, pooling, and lifecycle events
//! - `connection_pool`: Enterprise connection pooling engine with elastic sizing,
//!   partitioning, wait queue management, and comprehensive monitoring
//!
//! ## Connection Pool Features
//!
//! The connection pool provides Oracle-inspired capabilities:
//! - **Elastic Pool Sizing**: Dynamic adjustment between min/max connections
//! - **Connection Lifecycle**: Factory pattern with state reset and caching
//! - **Advanced Wait Queue**: Fair/priority queuing with deadlock detection
//! - **Pool Partitioning**: User/application/tenant-based isolation
//! - **Comprehensive Monitoring**: Real-time metrics and leak detection
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use rusty_db::pool::{ConnectionPool, PoolConfig};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Configure the pool
//!     let config = PoolConfig::builder()
//!         .min_size(10)
//!         .max_size(100)
//!         .initial_size(20)
//!         .acquire_timeout(Duration::from_secs(30))
//!         .build()?;
//!
//!     // Create factory (user-provided implementation)
//!     // let factory = Arc::new(MyConnectionFactory::new());
//!
//!     // Create pool
//!     // let pool = ConnectionPool::new(config, factory).await?;
//!
//!     // Acquire connection
//!     // let conn = pool.acquire().await?;
//!
//!     // Use connection
//!     // conn.connection().execute_query("SELECT * FROM users").await?;
//!
//!     // Connection automatically returned to pool on drop
//!
//!     Ok(())
//! }
//! ```

pub mod session_manager;
pub mod connection_pool;

// Re-export session management types
pub use session_manager::{
    SessionManager,
    SessionConfig,
    SessionState,
    SessionStatus,
    SessionSettings,
    ResourceLimits,
    ResourceController,
    SessionPool,
    PoolConfig as SessionPoolConfig,
    PoolStatistics as SessionPoolStatistics,
    AuthenticationProvider,
    Credentials,
    AuthenticationResult,
    SessionEventManager,
    SessionEvent,
    SessionTrigger,
    SessionCallback,
    PurityLevel,
};

// Re-export connection pool types
pub use connection_pool::{
    // Core types
    ConnectionPool,
    PoolConfig,
    PoolConfigBuilder,
    PoolError,
    PooledConnectionGuard,
    ConnectionFactory,

    // Lifecycle management
    RecyclingStrategy,
    AgingPolicy,
    RecyclingManager,
    LifetimeEnforcer,
    ConnectionValidator,

    // Wait queue
    WaitQueue,
    QueuePriority,
    DeadlockDetector,
    StarvationPrevention,

    // Partitioning
    PartitionManager,
    PartitionType,
    PartitionLimits,
    RoutingStrategy,
    PartitionRequest,
    LoadBalancingAlgorithm,

    // Statistics and monitoring
    PoolStatistics,
    PoolStats,
    DashboardProvider,
    DashboardData,
    LeakDetector,
    LeakInfo,
    MonitoringExporter,
    ExportFormat,
};
