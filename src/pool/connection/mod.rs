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
pub mod partitioning;
pub mod statistics;
pub mod wait_queue;

// Re-export core types
pub use core::{
    ConnectionPool, PoolConfig, PoolConfigBuilder, PoolError, PooledConnectionGuard,
    RecyclingStrategy,
};

// Re-export lifecycle types
pub use lifecycle::{
    AgingPolicy, ConnectionFactory, ConnectionValidator, LifetimeEnforcementStats,
    LifetimeEnforcer, LifetimeStatus, RecyclingManager, RecyclingStats, StateResetManager,
    ValidationStats,
};

// Re-export wait queue types
pub use wait_queue::{
    DeadlockDetector, DeadlockStats, QueuePriority, QueueStats, StarvationPrevention,
    StarvationStats, WaitQueue,
};

// Re-export partitioning types
pub use partitioning::{
    LoadBalancer, LoadBalancingAlgorithm, PartitionLimits, PartitionManager, PartitionRequest,
    PartitionStats, PartitionType, RoutingStrategy,
};

// Re-export statistics types
pub use statistics::{
    DashboardData, DashboardProvider, ExportFormat, LeakDetector, LeakInfo, MonitoringExporter,
    PoolSizeInfo, PoolStatistics, PoolStats,
};
