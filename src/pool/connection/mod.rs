// Connection pool module
//
// This module provides enterprise-grade connection pooling with:
// - Dynamic pool sizing
// - Connection lifecycle management
// - Wait queue management
// - Pool partitioning
// - Comprehensive statistics and monitoring

pub mod core;
pub mod lifecycle;
pub mod wait_queue;
pub mod partitioning;
pub mod statistics;

// Re-export core types
pub use core::{
    ConnectionPool,
    PoolConfig,
    PoolConfigBuilder,
    PoolError,
    RecyclingStrategy,
    PooledConnectionGuard,
};

// Re-export lifecycle types
pub use lifecycle::{
    ConnectionFactory,
    AgingPolicy,
    StateResetManager,
    RecyclingManager,
    RecyclingStats,
    LifetimeEnforcer,
    LifetimeStatus,
    LifetimeEnforcementStats,
    ConnectionValidator,
    ValidationStats,
};

// Re-export wait queue types
pub use wait_queue::{
    WaitQueue,
    QueuePriority,
    QueueStats,
    DeadlockDetector,
    DeadlockStats,
    StarvationPrevention,
    StarvationStats,
};

// Re-export partitioning types
pub use partitioning::{
    PartitionManager,
    PartitionType,
    PartitionLimits,
    PartitionStats,
    PartitionRequest,
    RoutingStrategy,
    LoadBalancer,
    LoadBalancingAlgorithm,
};

// Re-export statistics types
pub use statistics::{
    PoolStatistics,
    PoolStats,
    PoolSizeInfo,
    LeakDetector,
    LeakInfo,
    DashboardData,
    DashboardProvider,
    MonitoringExporter,
    ExportFormat,
};
