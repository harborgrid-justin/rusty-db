// # Session Pool and Connection Management Module
//
// This module provides enterprise-grade session management and connection pooling
// capabilities for RustyDB, including DRCP-like connection pooling, session lifecycle
// management, and resource control.
//
// ## Modules
//
// - `session`: Refactored session management with strong types and focused modules
// - `session_manager`: Complete session management system with authentication,
//   resource control, pooling, and lifecycle events (legacy - being migrated)
// - `connection_pool`: Enterprise connection pooling engine with elastic sizing,
//   partitioning, wait queue management, and comprehensive monitoring
//
// ## Connection Pool Features
//
// The connection pool provides Oracle-inspired capabilities:
// - **Elastic Pool Sizing**: Dynamic adjustment between min/max connections
// - **Connection Lifecycle**: Factory pattern with state reset and caching
// - **Advanced Wait Queue**: Fair/priority queuing with deadlock detection
// - **Pool Partitioning**: User/application/tenant-based isolation
// - **Comprehensive Monitoring**: Real-time metrics and leak detection
//
// ## Example Usage
//
// ```rust,no_run
// use rusty_db::pool::{ConnectionPool, PoolConfig};
// use std::time::Duration;
//
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // Configure the pool
//     let config = PoolConfig::builder()
//         .min_size(10)
//         .max_size(100)
//         .initial_size(20)
//         .acquire_timeout(Duration::from_secs(30))
//         .build()?;
//
//     // Create factory (user-provided implementation)
//     // let factory = Arc::new(MyConnectionFactory::new());
//
//     // Create pool
//     // let pool = ConnectionPool::new(config, factory).await?;
//
//     // Acquire connection
//     // let conn = pool.acquire().await?;
//
//     // Use connection
//     // conn.connection().execute_query("SELECT * FROM users").await?;
//
//     // Connection automatically returned to pool on drop
//
//     Ok(())
// }
// ```

pub mod connection; // Connection pool submodules
pub mod connection_pool;
pub mod session; // New refactored session module
pub mod session_manager; // Legacy - being migrated
pub mod sessions; // Session management submodules

// Re-export new session types (preferred)
pub use crate::common::SessionId;
pub use session::{
    Authenticator, DatabaseAuthenticator, PrivilegeSet, SchemaName, SessionState as SessionStateV2,
    TokenAuthenticator, Username,
};

// Re-export legacy session management types
pub use session_manager::{
    AuthenticationProvider, AuthenticationResult, Credentials, PoolConfig as SessionPoolConfig,
    PoolStatistics as SessionPoolStatistics, PurityLevel, ResourceController, ResourceLimits,
    SessionCallback, SessionConfig, SessionEvent, SessionEventManager, SessionManager, SessionPool,
    SessionSettings, SessionState, SessionStatus, SessionTrigger,
};

// Re-export connection pool types
pub use connection_pool::{
    AgingPolicy,
    ConnectionFactory,

    // Core types
    ConnectionPool,
    ConnectionValidator,

    DashboardData,
    DashboardProvider,
    DeadlockDetector,
    ExportFormat,
    LeakDetector,
    LeakInfo,
    LifetimeEnforcer,
    LoadBalancingAlgorithm,

    MonitoringExporter,
    PartitionLimits,
    // Partitioning
    PartitionManager,
    PartitionRequest,
    PartitionType,
    PoolConfig,
    PoolConfigBuilder,
    PoolError,
    // Statistics and monitoring
    PoolStatistics,
    PoolStats,
    PooledConnectionGuard,
    QueuePriority,
    RecyclingManager,
    // Lifecycle management
    RecyclingStrategy,
    RoutingStrategy,
    StarvationPrevention,

    // Wait queue
    WaitQueue,
};
