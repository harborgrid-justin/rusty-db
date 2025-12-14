// # Advanced Replication Engine
//
// Enterprise-grade replication system for RustyDB with Oracle-like capabilities.
//
// ## Overview
//
// This module provides comprehensive replication features including:
//
// - **Multi-Master Replication**: Bidirectional replication with conflict detection and resolution
// - **Logical Replication**: Row-level replication with filtering and transformation
// - **Sharding**: Hash, range, list, and composite sharding strategies
// - **Global Data Services**: Region-aware routing and load balancing
// - **Conflict Resolution**: CRDT-based and traditional conflict resolution strategies
// - **Replication Monitoring**: Real-time metrics, alerts, and dashboards
// - **Apply Engine**: Parallel change application with dependency tracking
// - **XA Transactions**: Distributed two-phase commit protocol
//
// ## Key Innovations
//
// ### CRDT-Based Conflict-Free Replication
//
// Implements multiple Conflict-free Replicated Data Types (CRDTs) for automatic
// conflict resolution without manual intervention:
//
// - **LWW-Register**: Last-Writer-Wins with timestamp and site-ID tie-breaking
// - **G-Counter**: Grow-only counter for monotonically increasing values
// - **PN-Counter**: Positive-Negative counter for increment/decrement operations
// - **G-Set**: Grow-only set for append-only collections
// - **2P-Set**: Two-Phase Set supporting additions and removals
// - **OR-Set**: Observed-Remove Set for concurrent add/remove operations
//
// ### ML-Based Conflict Prediction
//
// The system can analyze historical conflict patterns to predict and prevent
// future conflicts:
//
// - Pattern recognition for conflict-prone data access patterns
// - Predictive routing to minimize conflicts
// - Automatic recommendation of optimal conflict resolution strategies
//
// ### Adaptive Replication Topology
//
// Dynamically adjusts replication topology based on:
//
// - Workload characteristics
// - Network latency patterns
// - Geographic distribution of clients
// - Read/write ratios
//
// ### Zero-Downtime Shard Migration
//
// Move data between shards without service interruption:
//
// - Live migration with minimal performance impact
// - Automatic failback on errors
// - Progress tracking and ETA prediction
//
// ## Architecture
//
// ```text
// ┌─────────────────────────────────────────────────────────────┐
// │                  Advanced Replication Engine                │
// └─────────────────────────────────────────────────────────────┘
//          │              │              │              │
//     ┌────▼────┐    ┌────▼────┐   ┌────▼────┐   ┌────▼────┐
//     │  Multi  │    │Logical  │   │Sharding │   │   GDS   │
//     │ Master  │    │  Repli  │   │ Engine  │   │ Routing │
//     └────┬────┘    └────┬────┘   └────┬────┘   └────┬────┘
//          │              │              │              │
//          └──────────────┴──────────────┴──────────────┘
//                           │
//          ┌────────────────┴────────────────┐
//          │                                 │
//     ┌────▼────┐                       ┌────▼────┐
//     │Conflict │                       │  Apply  │
//     │Resolver │                       │ Engine  │
//     └────┬────┘                       └────┬────┘
//          │                                 │
//          └─────────────┬───────────────────┘
//                        │
//                   ┌────▼────┐
//                   │   XA    │
//                   │  Trans  │
//                   └────┬────┘
//                        │
//                   ┌────▼────┐
//                   │Monitor  │
//                   │  ing    │
//                   └─────────┘
// ```
//
// ## Usage Examples
//
// ### Multi-Master Replication
//
// ```rust,no_run
// use rusty_db::advanced_replication::multi_master::*;
// use rusty_db::advanced_replication::conflicts::*;
//
// # async fn example() -> rusty_db::Result<()> {
// // Create multi-master replication manager
// let mm = MultiMasterReplication::new("site-1".to_string());
//
// // Create a replication group
// let group = ReplicationGroup {
//     id: "group-1".to_string(),
//     name: "Global Replication".to_string(),
//     members: vec![],
//     tables: vec!["users".to_string(), "orders".to_string()],
//     conflict_strategy: ConflictResolutionStrategy::LastWriterWins,
//     write_quorum: 2,
//     read_quorum: 1,
//     created_at: 0,
// };
//
// mm.create_group(group)?;
//
// // Add sites to the group
// let site = SiteInfo {
//     site_id: "site-2".to_string(),
//     name: "EU West".to_string(),
//     address: "eu-west.example.com:5432".to_string(),
//     priority: 1,
//     region: "eu-west-1".to_string(),
//     active: true,
//     last_heartbeat: 0,
// };
//
// mm.add_site_to_group("group-1", site)?;
// # Ok(())
// # }
// ```
//
// ### Logical Replication with Filtering
//
// ```rust,no_run
// use rusty_db::advanced_replication::logical::*;
//
// # fn example() -> rusty_db::Result<()> {
// // Create logical replication manager
// let lr = LogicalReplication::new();
//
// // Create a publication with column filtering
// let table_pub = TablePublication {
//     table_name: "users".to_string(),
//     schema_name: "public".to_string(),
//     columns: Some(vec!["id".to_string(), "email".to_string(), "created_at".to_string()]),
//     row_filter: Some("active = true".to_string()),
//     transformations: vec![
//         Transformation::Mask {
//             column: "email".to_string(),
//             mask_type: MaskType::Hash,
//         }
//     ],
//     replicate_insert: true,
//     replicate_update: true,
//     replicate_delete: false,
// };
//
// let publication = Publication {
//     name: "active_users_pub".to_string(),
//     tables: vec![table_pub],
//     replicate_ddl: false,
//     replicate_truncate: false,
//     owner: "admin".to_string(),
//     created_at: 0,
// };
//
// lr.create_publication(publication)?;
// # Ok(())
// # }
// ```
//
// ### Sharding with Auto-Rebalancing
//
// ```rust,no_run
// use rusty_db::advanced_replication::sharding::*;
//
// # async fn example() -> rusty_db::Result<()> {
// // Create sharding engine
// let engine = ShardingEngine::new();
//
// // Create hash-based sharding
// let strategy = ShardingStrategy::Hash {
//     num_shards: 16,
//     hash_function: HashFunction::Consistent,
// };
//
// let sharded_table = ShardedTable {
//     table_name: "orders".to_string(),
//     schema_name: "public".to_string(),
//     shard_key_columns: vec!["customer_id".to_string()],
//     strategy,
//     shards: vec![],
//     created_at: 0,
// };
//
// engine.create_sharded_table(sharded_table)?;
//
// // Plan and execute rebalance
// let plan = engine.plan_rebalance("public.orders", "shard-0", "shard-1")?;
// engine.execute_rebalance(&plan.id).await?;
// # Ok(())
// # }
// ```
//
// ### Global Data Services with Region-Aware Routing
//
// ```rust,no_run
// use rusty_db::advanced_replication::gds::*;
//
// # fn example() -> rusty_db::Result<()> {
// // Create GDS manager
// let gds = GlobalDataServices::new();
//
// // Register a global service
// let service = GlobalService {
//     name: "global-db".to_string(),
//     regions: vec![],
//     load_balancing: LoadBalancingStrategy::LocalityAware,
//     failover_policy: FailoverPolicy {
//         auto_failover: true,
//         timeout_ms: 5000,
//         max_retries: 3,
//         priority_order: vec!["us-east-1".to_string(), "eu-west-1".to_string()],
//     },
//     state: ServiceState::Active,
//     created_at: 0,
// };
//
// gds.register_service(service)?;
//
// // Route a request
// let request = ConnectionRequest {
//     id: "req-1".to_string(),
//     client_location: Some(GeoLocation {
//         latitude: 40.7128,
//         longitude: -74.0060,
//         country: "US".to_string(),
//         city: "New York".to_string(),
//     }),
//     request_type: RequestType::Read,
//     priority: 1,
//     timestamp: 0,
// };
//
// let decision = gds.route_request("global-db", &request)?;
// println!("Routed to region: {}", decision.region_id);
// # Ok(())
// # }
// ```
//
// ### XA Distributed Transactions
//
// ```rust,no_run
// use rusty_db::advanced_replication::xa::*;
//
// # async fn example() -> rusty_db::Result<()> {
// // Create XA transaction manager
// let xa_mgr = XaTransactionManager::new();
//
// // Register resource managers (databases)
// xa_mgr.register_resource_manager(ResourceManager {
//     id: "db1".to_string(),
//     name: "Database 1".to_string(),
//     connection: "host1:5432".to_string(),
//     state: RmState::Available,
// })?;
//
// xa_mgr.register_resource_manager(ResourceManager {
//     id: "db2".to_string(),
//     name: "Database 2".to_string(),
//     connection: "host2:5432".to_string(),
//     state: RmState::Available,
// })?;
//
// // Start distributed transaction
// let xid = Xid::generate();
// xa_mgr.xa_start(
//     xid.clone(),
//     vec!["db1".to_string(), "db2".to_string()],
//     30  // 30 second timeout
// )?;
//
// // Perform operations...
//
// // End transaction
// xa_mgr.xa_end(&xid)?;
//
// // Two-phase commit
// let votes = xa_mgr.xa_prepare(&xid).await?;
//
// if votes.iter().all(|v| *v == Vote::VoteCommit) {
//     xa_mgr.xa_commit(&xid, false).await?;
// } else {
//     xa_mgr.xa_rollback(&xid).await?;
// }
// # Ok(())
// # }
// ```
//
// ## Performance Considerations
//
// - **Parallel Apply**: The apply engine processes changes in parallel while respecting dependencies
// - **Batching**: Changes are grouped by transaction for efficient application
// - **CRDT Optimization**: CRDTs eliminate the need for coordination in many scenarios
// - **Consistent Hashing**: Minimizes data movement during shard rebalancing
// - **Locality-Aware Routing**: Reduces network latency by routing to nearby regions
//
// ## Monitoring and Observability
//
// The monitoring module provides comprehensive metrics:
//
// - Replication lag (time, bytes, transactions)
// - Throughput (changes/sec, bytes/sec, transactions/sec)
// - Error rates and conflict rates
// - Alert thresholds with configurable severity
// - Time-series data for historical analysis
// - Dashboard generation for real-time visibility

use serde::{Deserialize, Serialize};

// Re-export all public types from submodules

/// Multi-master replication with bidirectional sync
pub mod multi_master;

/// Logical replication with row and column filtering
pub mod logical;

/// Sharding engine with multiple strategies
pub mod sharding;

/// Global Data Services for geo-distributed deployments
pub mod gds;

/// Conflict detection and resolution
pub mod conflicts;

/// Replication monitoring and alerting
pub mod monitoring;

/// Apply engine for parallel change application
pub mod apply;

/// XA distributed transactions
pub mod xa;

// Re-export commonly used types
pub use multi_master::{
    ConvergenceReport, MultiMasterReplication, MultiMasterStats, OpType, QuorumResult,
    ReplicationGroup, ReplicationOp, SiteInfo,
};

pub use logical::{
    ChangeType, LogicalChange, LogicalReplication, LogicalReplicationStats, MaskType, Publication,
    Subscription, SubscriptionState, TablePublication, Transformation,
};

pub use sharding::{
    CrossShardQuery, HashFunction, RebalancePlan, RebalanceState, Shard, ShardKey, ShardKeyAdvice,
    ShardStatistics, ShardStatus, ShardedTable, ShardingEngine, ShardingStrategy,
};

pub use gds::{
    ConnectionRequest, DatabaseInstance, FailoverPolicy, GdsStats, GeoLocation, GlobalDataServices,
    GlobalService, HealthStatus, InstanceRole, LoadBalancingStrategy, RegionRole, RequestType,
    RoutingDecision, ServiceRegion, ServiceState,
};

pub use conflicts::{
    Conflict, ConflictResolution, ConflictResolutionStrategy, ConflictResolver, ConflictStats,
    ConflictType, ConflictingChange, CrdtType,
};

pub use monitoring::{
    Alert, AlertSeverity, AlertThreshold, ChannelHealth, ChannelStatus, ComparisonOperator,
    ConflictRateMetrics, DashboardData, ErrorRateMetrics, MetricType, ReplicationLag,
    ReplicationMonitor, ThroughputMetrics, TimeSeriesPoint,
};

pub use apply::{
    ApplyChange, ApplyCheckpoint, ApplyConfig, ApplyEngine, ApplyError, ApplyStats, GroupState,
    OperationType, TransactionGroup,
};

pub use xa::{
    HeuristicDecision, LogEntryType, ResourceManager, RmState, Vote, XaState, XaStats,
    XaTransaction, XaTransactionManager, Xid,
};

/// Advanced replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedReplicationConfig {
    /// Enable multi-master replication
    pub enable_multi_master: bool,
    /// Enable logical replication
    pub enable_logical: bool,
    /// Enable sharding
    pub enable_sharding: bool,
    /// Enable global data services
    pub enable_gds: bool,
    /// Default conflict resolution strategy
    pub default_conflict_strategy: ConflictResolutionStrategy,
    /// Apply engine configuration
    pub apply_config: ApplyConfig,
    /// Monitoring window (seconds)
    pub monitoring_window_seconds: u64,
}

impl Default for AdvancedReplicationConfig {
    fn default() -> Self {
        Self {
            enable_multi_master: true,
            enable_logical: true,
            enable_sharding: false,
            enable_gds: false,
            default_conflict_strategy: ConflictResolutionStrategy::LastWriterWins,
            apply_config: ApplyConfig::default(),
            monitoring_window_seconds: 3600,
        }
    }
}

/// Unified advanced replication manager
pub struct AdvancedReplicationManager {
    /// Configuration
    config: AdvancedReplicationConfig,
    /// Multi-master replication
    multi_master: Option<MultiMasterReplication>,
    /// Logical replication
    logical: Option<LogicalReplication>,
    /// Sharding engine
    sharding: Option<ShardingEngine>,
    /// Global data services
    gds: Option<GlobalDataServices>,
    /// Apply engine
    apply_engine: ApplyEngine,
    /// Monitoring
    monitor: ReplicationMonitor,
    /// XA transaction manager
    xa_manager: XaTransactionManager,
}

impl AdvancedReplicationManager {
    /// Create a new advanced replication manager
    pub fn new(config: AdvancedReplicationConfig) -> Self {
        Self {
            multi_master: if config.enable_multi_master {
                Some(MultiMasterReplication::new("default".to_string()))
            } else {
                None
            },
            logical: if config.enable_logical {
                Some(LogicalReplication::new())
            } else {
                None
            },
            sharding: if config.enable_sharding {
                Some(ShardingEngine::new())
            } else {
                None
            },
            gds: if config.enable_gds {
                Some(GlobalDataServices::new())
            } else {
                None
            },
            apply_engine: ApplyEngine::new(config.apply_config.clone()),
            monitor: ReplicationMonitor::new(config.monitoring_window_seconds),
            xa_manager: XaTransactionManager::new(),
            config,
        }
    }

    /// Get multi-master replication manager
    pub fn multi_master(&self) -> Option<&MultiMasterReplication> {
        self.multi_master.as_ref()
    }

    /// Get logical replication manager
    pub fn logical(&self) -> Option<&LogicalReplication> {
        self.logical.as_ref()
    }

    /// Get sharding engine
    pub fn sharding(&self) -> Option<&ShardingEngine> {
        self.sharding.as_ref()
    }

    /// Get global data services
    pub fn gds(&self) -> Option<&GlobalDataServices> {
        self.gds.as_ref()
    }

    /// Get apply engine
    pub fn apply_engine(&self) -> &ApplyEngine {
        &self.apply_engine
    }

    /// Get monitoring
    pub fn monitor(&self) -> &ReplicationMonitor {
        &self.monitor
    }

    /// Get XA transaction manager
    pub fn xa_manager(&self) -> &XaTransactionManager {
        &self.xa_manager
    }

    /// Get configuration
    pub fn config(&self) -> &AdvancedReplicationConfig {
        &self.config
    }

    /// Generate comprehensive status report
    pub fn status_report(&self) -> StatusReport {
        StatusReport {
            multi_master_stats: self.multi_master.as_ref().map(|mm| mm.get_stats()),
            logical_stats: self.logical.as_ref().map(|lr| lr.get_stats()),
            sharding_stats: self.sharding.as_ref().map(|se| se.get_stats()),
            gds_stats: self.gds.as_ref().map(|gds| gds.get_stats()),
            apply_stats: self.apply_engine.get_stats(),
            xa_stats: self.xa_manager.get_stats(),
            dashboard: self.monitor.generate_dashboard(),
        }
    }
}

/// Comprehensive status report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReport {
    pub multi_master_stats: Option<MultiMasterStats>,
    pub logical_stats: Option<LogicalReplicationStats>,
    pub sharding_stats: Option<sharding::ShardingStats>,
    pub gds_stats: Option<GdsStats>,
    pub apply_stats: ApplyStats,
    pub xa_stats: XaStats,
    pub dashboard: DashboardData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_manager() {
        let config = AdvancedReplicationConfig::default();
        let manager = AdvancedReplicationManager::new(config);

        assert!(manager.multi_master().is_some());
        assert!(manager.logical().is_some());
    }

    #[test]
    fn test_status_report() {
        let config = AdvancedReplicationConfig::default();
        let manager = AdvancedReplicationManager::new(config);

        let report = manager.status_report();
        assert!(report.multi_master_stats.is_some());
    }
}
