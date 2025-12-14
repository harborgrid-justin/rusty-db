// Replication and Clustering WebSocket Event Types
//
// Comprehensive event types for real-time replication and clustering monitoring

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

// ============================================================================
// Replication Events
// ============================================================================

/// Replication lag alert event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReplicationLagEvent {
    /// Event type
    pub event_type: String, // "replication_lag_alert"
    /// Replica identifier
    pub replica_id: String,
    /// Current lag in bytes
    pub lag_bytes: u64,
    /// Current lag in seconds
    pub lag_seconds: f64,
    /// Threshold that was exceeded
    pub threshold_bytes: u64,
    /// Severity: info, warning, critical
    pub severity: String,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
    /// Additional details
    pub details: Option<String>,
}

/// Replica status change event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReplicaStatusEvent {
    /// Event type
    pub event_type: String, // "replica_status_change"
    /// Replica identifier
    pub replica_id: String,
    /// Previous status
    pub old_status: String, // online, offline, catching_up, in_sync
    /// New status
    pub new_status: String,
    /// Reason for status change
    pub reason: Option<String>,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Replication error event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReplicationErrorEvent {
    /// Event type
    pub event_type: String, // "replication_error"
    /// Replica identifier
    pub replica_id: String,
    /// Error code
    pub error_code: String,
    /// Error message
    pub error_message: String,
    /// Error severity
    pub severity: String, // warning, error, critical
    /// Operation that failed
    pub failed_operation: Option<String>,
    /// Retry count
    pub retry_count: u32,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

/// WAL position update event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WalPositionEvent {
    /// Event type
    pub event_type: String, // "wal_position_update"
    /// Replica identifier
    pub replica_id: String,
    /// Write LSN (Log Sequence Number)
    pub write_lsn: String,
    /// Flush LSN
    pub flush_lsn: String,
    /// Replay LSN
    pub replay_lsn: String,
    /// Bytes behind primary
    pub bytes_behind: u64,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

/// Replication slot event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReplicationSlotEvent {
    /// Event type: slot_created, slot_dropped, slot_active, slot_inactive
    pub event_type: String,
    /// Slot name
    pub slot_name: String,
    /// Slot type: physical, logical
    pub slot_type: String,
    /// Database name
    pub database: Option<String>,
    /// Active status
    pub active: bool,
    /// Restart LSN
    pub restart_lsn: Option<String>,
    /// Confirmed flush LSN
    pub confirmed_flush_lsn: Option<String>,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

/// Conflict detection event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConflictEvent {
    /// Event type: conflict_detected, conflict_resolved
    pub event_type: String,
    /// Conflict ID
    pub conflict_id: String,
    /// Table name
    pub table_name: String,
    /// Conflict type: update_update, update_delete, insert_insert
    pub conflict_type: String,
    /// Local value
    pub local_value: serde_json::Value,
    /// Remote value
    pub remote_value: serde_json::Value,
    /// Resolution strategy used
    pub resolution_strategy: Option<String>,
    /// Winning value (if resolved)
    pub resolved_value: Option<serde_json::Value>,
    /// Source replica
    pub source_replica: String,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

// ============================================================================
// Clustering Events
// ============================================================================

/// Node membership event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeMembershipEvent {
    /// Event type: node_joined, node_left, node_evicted
    pub event_type: String,
    /// Node identifier
    pub node_id: String,
    /// Node address
    pub node_address: String,
    /// Node role: leader, follower, witness
    pub node_role: Option<String>,
    /// Reason for event
    pub reason: Option<String>,
    /// Cluster size after event
    pub cluster_size: usize,
    /// Has quorum
    pub has_quorum: bool,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

/// Node health change event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeHealthEvent {
    /// Event type: node_health_change
    pub event_type: String,
    /// Node identifier
    pub node_id: String,
    /// Previous health status
    pub old_status: String, // healthy, degraded, unreachable, failed
    /// New health status
    pub new_status: String,
    /// Health metrics
    pub metrics: NodeHealthMetrics,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

/// Node health metrics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeHealthMetrics {
    /// CPU usage percentage
    pub cpu_usage: f32,
    /// Memory usage percentage
    pub memory_usage: f32,
    /// Disk usage percentage
    pub disk_usage: f32,
    /// Active connections
    pub active_connections: usize,
    /// Queries per second
    pub queries_per_second: f64,
    /// Average response time (ms)
    pub avg_response_time_ms: f64,
}

/// Failover event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FailoverEvent {
    /// Event type: failover_initiated, failover_completed, failover_failed
    pub event_type: String,
    /// Failover ID
    pub failover_id: String,
    /// Failed node ID
    pub failed_node: String,
    /// Replacement node ID
    pub replacement_node: Option<String>,
    /// Failover type: automatic, manual, planned
    pub failover_type: String,
    /// Duration (seconds)
    pub duration_seconds: Option<f64>,
    /// Success status
    pub success: bool,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

/// Leader election event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LeaderElectionEvent {
    /// Event type: leader_elected, leader_stepped_down
    pub event_type: String,
    /// New leader node ID
    pub leader_id: String,
    /// Previous leader ID
    pub previous_leader: Option<String>,
    /// Election term
    pub term: u64,
    /// Number of votes received
    pub votes_received: usize,
    /// Total voters
    pub total_voters: usize,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

/// Quorum status event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuorumEvent {
    /// Event type: quorum_lost, quorum_restored
    pub event_type: String,
    /// Healthy nodes count
    pub healthy_nodes: usize,
    /// Total nodes count
    pub total_nodes: usize,
    /// Quorum size required
    pub quorum_size: usize,
    /// Has quorum
    pub has_quorum: bool,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

/// Data migration event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MigrationEvent {
    /// Event type: migration_started, migration_progress, migration_completed, migration_failed
    pub event_type: String,
    /// Migration ID
    pub migration_id: String,
    /// Source node
    pub source_node: String,
    /// Target node
    pub target_node: String,
    /// Table being migrated
    pub table_name: Option<String>,
    /// Progress percentage (0-100)
    pub progress_percent: f64,
    /// Bytes migrated
    pub bytes_migrated: u64,
    /// Total bytes to migrate
    pub total_bytes: u64,
    /// Estimated time remaining (seconds)
    pub eta_seconds: Option<f64>,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

// ============================================================================
// RAC Events
// ============================================================================

/// Cache Fusion block transfer event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CacheFusionEvent {
    /// Event type: block_transfer, block_request, block_grant
    pub event_type: String,
    /// Block ID
    pub block_id: String,
    /// Source instance
    pub source_instance: String,
    /// Target instance
    pub target_instance: String,
    /// Block mode: shared, exclusive
    pub block_mode: String,
    /// Transfer size (bytes)
    pub transfer_size: usize,
    /// Transfer duration (microseconds)
    pub duration_micros: u64,
    /// Success status
    pub success: bool,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

/// Resource lock event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResourceLockEvent {
    /// Event type: lock_granted, lock_released, lock_converted, lock_blocked
    pub event_type: String,
    /// Resource ID
    pub resource_id: String,
    /// Lock type: null, shared, exclusive
    pub lock_type: String,
    /// Previous lock type (for conversions)
    pub previous_lock_type: Option<String>,
    /// Requesting instance
    pub instance_id: String,
    /// Lock granted
    pub granted: bool,
    /// Wait time (milliseconds)
    pub wait_time_ms: Option<u64>,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

/// Resource remastering event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RemasteringEvent {
    /// Event type: remaster_started, remaster_completed
    pub event_type: String,
    /// Resource ID
    pub resource_id: String,
    /// Previous master instance
    pub old_master: String,
    /// New master instance
    pub new_master: String,
    /// Reason for remastering
    pub reason: String, // load_balance, affinity, failure
    /// Duration (milliseconds)
    pub duration_ms: u64,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

/// Instance recovery event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstanceRecoveryEvent {
    /// Event type: recovery_started, recovery_progress, recovery_completed
    pub event_type: String,
    /// Failed instance ID
    pub failed_instance: String,
    /// Recovering instance ID
    pub recovering_instance: Option<String>,
    /// Recovery phase: redo_apply, lock_reconfig, resource_remaster
    pub recovery_phase: String,
    /// Progress percentage (0-100)
    pub progress_percent: f64,
    /// Redo logs applied
    pub redo_logs_applied: u64,
    /// Total redo logs
    pub total_redo_logs: u64,
    /// Estimated time remaining (seconds)
    pub eta_seconds: Option<f64>,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

/// Parallel query execution event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParallelQueryEvent {
    /// Event type: query_started, query_progress, query_completed
    pub event_type: String,
    /// Query ID
    pub query_id: String,
    /// SQL text (truncated)
    pub sql_text: String,
    /// Degree of parallelism
    pub dop: usize,
    /// Instances involved
    pub instances: Vec<String>,
    /// Rows processed
    pub rows_processed: u64,
    /// Duration (milliseconds)
    pub duration_ms: u64,
    /// Success status
    pub success: bool,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

// ============================================================================
// Sharding Events
// ============================================================================

/// Shard management event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShardEvent {
    /// Event type: shard_added, shard_removed, shard_status_change
    pub event_type: String,
    /// Shard ID
    pub shard_id: String,
    /// Table name
    pub table_name: String,
    /// Shard status: active, inactive, migrating
    pub status: String,
    /// Node hosting the shard
    pub node_id: String,
    /// Shard key range
    pub key_range: Option<ShardKeyRange>,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

/// Shard key range
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShardKeyRange {
    /// Start key
    pub start_key: String,
    /// End key
    pub end_key: String,
}

/// Shard rebalance event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShardRebalanceEvent {
    /// Event type: rebalance_started, rebalance_progress, rebalance_completed
    pub event_type: String,
    /// Rebalance plan ID
    pub plan_id: String,
    /// Table being rebalanced
    pub table_name: String,
    /// Source shard
    pub source_shard: String,
    /// Target shard
    pub target_shard: String,
    /// Progress percentage (0-100)
    pub progress_percent: f64,
    /// Rows migrated
    pub rows_migrated: u64,
    /// Total rows to migrate
    pub total_rows: u64,
    /// Estimated time remaining (seconds)
    pub eta_seconds: Option<f64>,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

// ============================================================================
// Unified Event Envelope
// ============================================================================

/// Unified event envelope for all replication/clustering events
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClusterEvent {
    /// Event category: replication, clustering, rac, sharding
    pub category: String,
    /// Specific event type
    pub event_type: String,
    /// Event payload
    pub payload: serde_json::Value,
    /// Event severity: info, warning, error, critical
    pub severity: String,
    /// Source component
    pub source: String,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
    /// Event ID
    pub event_id: String,
}

impl ClusterEvent {
    /// Create a replication lag event
    pub fn replication_lag(event: ReplicationLagEvent) -> Self {
        Self {
            category: "replication".to_string(),
            event_type: event.event_type.clone(),
            severity: event.severity.clone(),
            source: format!("replica:{}", event.replica_id),
            timestamp: event.timestamp,
            event_id: uuid::Uuid::new_v4().to_string(),
            payload: serde_json::to_value(event).unwrap_or_default(),
        }
    }

    /// Create a node health event
    pub fn node_health(event: NodeHealthEvent) -> Self {
        let severity = match event.new_status.as_str() {
            "healthy" => "info",
            "degraded" => "warning",
            "unreachable" | "failed" => "critical",
            _ => "info",
        };

        Self {
            category: "clustering".to_string(),
            event_type: event.event_type.clone(),
            severity: severity.to_string(),
            source: format!("node:{}", event.node_id),
            timestamp: event.timestamp,
            event_id: uuid::Uuid::new_v4().to_string(),
            payload: serde_json::to_value(event).unwrap_or_default(),
        }
    }

    /// Create a failover event
    pub fn failover(event: FailoverEvent) -> Self {
        let severity = if event.success { "warning" } else { "critical" };

        Self {
            category: "clustering".to_string(),
            event_type: event.event_type.clone(),
            severity: severity.to_string(),
            source: "cluster:failover".to_string(),
            timestamp: event.timestamp,
            event_id: uuid::Uuid::new_v4().to_string(),
            payload: serde_json::to_value(event).unwrap_or_default(),
        }
    }

    /// Create a cache fusion event
    pub fn cache_fusion(event: CacheFusionEvent) -> Self {
        Self {
            category: "rac".to_string(),
            event_type: event.event_type.clone(),
            severity: "info".to_string(),
            source: format!("rac:cache_fusion"),
            timestamp: event.timestamp,
            event_id: uuid::Uuid::new_v4().to_string(),
            payload: serde_json::to_value(event).unwrap_or_default(),
        }
    }

    /// Create a shard rebalance event
    pub fn shard_rebalance(event: ShardRebalanceEvent) -> Self {
        Self {
            category: "sharding".to_string(),
            event_type: event.event_type.clone(),
            severity: "info".to_string(),
            source: format!("shard:{}", event.table_name),
            timestamp: event.timestamp,
            event_id: uuid::Uuid::new_v4().to_string(),
            payload: serde_json::to_value(event).unwrap_or_default(),
        }
    }
}
