// GraphQL Monitoring and System Types
//
// Type definitions for monitoring, cluster, storage, and admin queries

use async_graphql::{Enum, SimpleObject};
use serde::{Deserialize, Serialize};
use super::types::{BigInt, DateTime};

// ============================================================================
// MONITORING TYPES
// ============================================================================

/// System metrics response
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    /// CPU usage percentage (0-100)
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_used: BigInt,
    /// Total memory in bytes
    pub memory_total: BigInt,
    /// Memory usage percentage (0-100)
    pub memory_percent: f64,
    /// Disk usage in bytes
    pub disk_used: BigInt,
    /// Total disk in bytes
    pub disk_total: BigInt,
    /// Disk usage percentage (0-100)
    pub disk_percent: f64,
    /// Active connections count
    pub active_connections: i32,
    /// Total connections count
    pub total_connections: i32,
    /// Queries per second
    pub qps: f64,
    /// Cache hit ratio (0-1)
    pub cache_hit_ratio: f64,
    /// Timestamp of metrics
    pub timestamp: DateTime,
}

/// Session statistics
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct SessionStats {
    /// Number of active sessions
    pub active_sessions: i32,
    /// Number of idle sessions
    pub idle_sessions: i32,
    /// Total sessions
    pub total_sessions: i32,
    /// Average session duration in seconds
    pub avg_session_duration: f64,
    /// Peak concurrent sessions
    pub peak_sessions: i32,
    /// Timestamp
    pub timestamp: DateTime,
}

/// Query execution statistics
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct QueryStats {
    /// Total queries executed
    pub total_queries: BigInt,
    /// Successful queries
    pub successful_queries: BigInt,
    /// Failed queries
    pub failed_queries: BigInt,
    /// Average execution time in milliseconds
    pub avg_execution_time_ms: f64,
    /// Median execution time in milliseconds
    pub median_execution_time_ms: f64,
    /// 95th percentile execution time in milliseconds
    pub p95_execution_time_ms: f64,
    /// 99th percentile execution time in milliseconds
    pub p99_execution_time_ms: f64,
    /// Queries per second
    pub qps: f64,
    /// Timestamp
    pub timestamp: DateTime,
}

/// Comprehensive performance data
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceData {
    /// CPU metrics
    pub cpu_usage: f64,
    /// Memory metrics
    pub memory_usage: f64,
    /// Disk I/O read bytes per second
    pub disk_read_bps: BigInt,
    /// Disk I/O write bytes per second
    pub disk_write_bps: BigInt,
    /// Network receive bytes per second
    pub network_rx_bps: BigInt,
    /// Network transmit bytes per second
    pub network_tx_bps: BigInt,
    /// Active queries count
    pub active_queries: i32,
    /// Waiting queries count
    pub waiting_queries: i32,
    /// Buffer pool hit ratio
    pub buffer_hit_ratio: f64,
    /// Transaction commit rate (per second)
    pub commit_rate: f64,
    /// Transaction rollback rate (per second)
    pub rollback_rate: f64,
    /// Timestamp
    pub timestamp: DateTime,
}

/// Currently active query
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct ActiveQuery {
    /// Query ID
    pub query_id: String,
    /// Session ID
    pub session_id: String,
    /// User executing the query
    pub username: String,
    /// SQL text
    pub sql_text: String,
    /// Query state
    pub state: String,
    /// Start time
    pub start_time: DateTime,
    /// Duration in milliseconds
    pub duration_ms: BigInt,
    /// Rows processed
    pub rows_processed: BigInt,
    /// Wait event (if any)
    pub wait_event: Option<String>,
}

/// Slow query record
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct SlowQuery {
    /// Query ID
    pub query_id: String,
    /// SQL text
    pub sql_text: String,
    /// Execution time in milliseconds
    pub execution_time_ms: BigInt,
    /// Start time
    pub start_time: DateTime,
    /// End time
    pub end_time: DateTime,
    /// User
    pub username: String,
    /// Database
    pub database: String,
    /// Rows returned
    pub rows_returned: BigInt,
}

// ============================================================================
// CLUSTER TYPES
// ============================================================================

/// Cluster node information
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct ClusterNode {
    /// Node ID
    pub id: String,
    /// Node name
    pub name: String,
    /// Node role (leader, follower, candidate)
    pub role: String,
    /// Node status (healthy, unhealthy, unreachable)
    pub status: String,
    /// Node address
    pub address: String,
    /// Last heartbeat time
    pub last_heartbeat: DateTime,
    /// Uptime in seconds
    pub uptime_seconds: BigInt,
    /// Current term (for Raft)
    pub term: BigInt,
    /// Is this the leader
    pub is_leader: bool,
    /// CPU usage
    pub cpu_usage: f64,
    /// Memory usage
    pub memory_usage: f64,
}

/// Cluster topology information
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct ClusterTopology {
    /// Total nodes in cluster
    pub total_nodes: i32,
    /// Healthy nodes
    pub healthy_nodes: i32,
    /// Current leader ID
    pub leader_id: Option<String>,
    /// Current term
    pub current_term: BigInt,
    /// Has quorum
    pub has_quorum: bool,
    /// Nodes
    pub nodes: Vec<ClusterNode>,
    /// Timestamp
    pub timestamp: DateTime,
}

/// Replication status
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct ReplicationStatus {
    /// Replication mode (sync, async, semi-sync)
    pub mode: String,
    /// Replication state (streaming, catching-up, stopped)
    pub state: String,
    /// Replication lag in milliseconds
    pub lag_ms: BigInt,
    /// Bytes behind leader
    pub bytes_behind: BigInt,
    /// Last WAL position received
    pub last_wal_received: String,
    /// Last WAL position applied
    pub last_wal_applied: String,
    /// Timestamp
    pub timestamp: DateTime,
}

/// Cluster configuration
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Cluster name
    pub cluster_name: String,
    /// Replication factor
    pub replication_factor: i32,
    /// Minimum quorum size
    pub min_quorum_size: i32,
    /// Election timeout in milliseconds
    pub election_timeout_ms: i32,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: i32,
    /// Auto failover enabled
    pub auto_failover: bool,
    /// Geo-replication enabled
    pub geo_replication: bool,
}

// ============================================================================
// STORAGE TYPES
// ============================================================================

/// Storage status
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct StorageStatus {
    /// Total storage capacity in bytes
    pub total_bytes: BigInt,
    /// Used storage in bytes
    pub used_bytes: BigInt,
    /// Available storage in bytes
    pub available_bytes: BigInt,
    /// Usage percentage (0-100)
    pub usage_percent: f64,
    /// Number of data files
    pub data_files: i32,
    /// Total data file size
    pub data_size: BigInt,
    /// Number of index files
    pub index_files: i32,
    /// Total index size
    pub index_size: BigInt,
    /// WAL size
    pub wal_size: BigInt,
    /// Timestamp
    pub timestamp: DateTime,
}

/// Buffer pool statistics
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct BufferPoolStats {
    /// Total buffer pool size in bytes
    pub size_bytes: BigInt,
    /// Number of pages in pool
    pub total_pages: i32,
    /// Free pages
    pub free_pages: i32,
    /// Dirty pages
    pub dirty_pages: i32,
    /// Buffer hit ratio (0-1)
    pub hit_ratio: f64,
    /// Total reads
    pub total_reads: BigInt,
    /// Total writes
    pub total_writes: BigInt,
    /// Cache hits
    pub cache_hits: BigInt,
    /// Cache misses
    pub cache_misses: BigInt,
    /// Evictions
    pub evictions: BigInt,
    /// Timestamp
    pub timestamp: DateTime,
}

/// Tablespace information
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct Tablespace {
    /// Tablespace ID
    pub id: String,
    /// Tablespace name
    pub name: String,
    /// Location path
    pub location: String,
    /// Size in bytes
    pub size_bytes: BigInt,
    /// Used space in bytes
    pub used_bytes: BigInt,
    /// Number of tables
    pub table_count: i32,
    /// Is default tablespace
    pub is_default: bool,
    /// Creation time
    pub created_at: DateTime,
}

/// I/O statistics
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct IoStats {
    /// Total read operations
    pub reads: BigInt,
    /// Total write operations
    pub writes: BigInt,
    /// Total bytes read
    pub bytes_read: BigInt,
    /// Total bytes written
    pub bytes_written: BigInt,
    /// Average read latency in microseconds
    pub avg_read_latency_us: f64,
    /// Average write latency in microseconds
    pub avg_write_latency_us: f64,
    /// Read throughput (bytes per second)
    pub read_throughput_bps: BigInt,
    /// Write throughput (bytes per second)
    pub write_throughput_bps: BigInt,
    /// Timestamp
    pub timestamp: DateTime,
}

// ============================================================================
// TRANSACTION TYPES
// ============================================================================

/// Active transaction
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct ActiveTransaction {
    /// Transaction ID
    pub transaction_id: String,
    /// Session ID
    pub session_id: String,
    /// User
    pub username: String,
    /// Transaction state
    pub state: String,
    /// Isolation level
    pub isolation_level: String,
    /// Start time
    pub start_time: DateTime,
    /// Duration in milliseconds
    pub duration_ms: BigInt,
    /// Number of queries
    pub query_count: i32,
    /// Rows modified
    pub rows_modified: BigInt,
}

/// Lock information
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct Lock {
    /// Lock ID
    pub lock_id: String,
    /// Transaction ID holding the lock
    pub transaction_id: String,
    /// Lock type (shared, exclusive)
    pub lock_type: String,
    /// Lock mode (row, table, page)
    pub lock_mode: String,
    /// Resource being locked
    pub resource: String,
    /// Table name
    pub table_name: Option<String>,
    /// Row ID
    pub row_id: Option<String>,
    /// Granted time
    pub granted_at: DateTime,
    /// Wait time in milliseconds (if waiting)
    pub wait_time_ms: Option<BigInt>,
}

/// Deadlock information
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct Deadlock {
    /// Deadlock ID
    pub deadlock_id: String,
    /// Detection time
    pub detected_at: DateTime,
    /// Transactions involved
    pub transactions: Vec<String>,
    /// Victim transaction (chosen to rollback)
    pub victim_transaction: String,
    /// Resource graph (JSON representation)
    pub resource_graph: String,
    /// Resolution strategy
    pub resolution: String,
}

/// MVCC status
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct MvccStatus {
    /// Current snapshot ID
    pub current_snapshot_id: String,
    /// Oldest active transaction
    pub oldest_transaction_id: Option<String>,
    /// Number of active snapshots
    pub active_snapshots: i32,
    /// Total versions in database
    pub total_versions: BigInt,
    /// Dead versions (candidates for vacuuming)
    pub dead_versions: BigInt,
    /// Last vacuum time
    pub last_vacuum: Option<DateTime>,
    /// Timestamp
    pub timestamp: DateTime,
}

// ============================================================================
// ADMIN TYPES
// ============================================================================

/// Server configuration
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server version
    pub version: String,
    /// Server port
    pub port: i32,
    /// Maximum connections
    pub max_connections: i32,
    /// Buffer pool size in bytes
    pub buffer_pool_size: BigInt,
    /// WAL buffer size in bytes
    pub wal_buffer_size: BigInt,
    /// Data directory
    pub data_directory: String,
    /// Log level
    pub log_level: String,
    /// SSL enabled
    pub ssl_enabled: bool,
    /// Uptime in seconds
    pub uptime_seconds: BigInt,
    /// Start time
    pub start_time: DateTime,
}

/// User information
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct User {
    /// User ID
    pub id: String,
    /// Username
    pub username: String,
    /// Email
    pub email: Option<String>,
    /// Roles
    pub roles: Vec<String>,
    /// Is admin
    pub is_admin: bool,
    /// Is active
    pub is_active: bool,
    /// Last login
    pub last_login: Option<DateTime>,
    /// Created at
    pub created_at: DateTime,
}

/// Role information
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct Role {
    /// Role ID
    pub id: String,
    /// Role name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Permissions
    pub permissions: Vec<String>,
    /// Is system role
    pub is_system: bool,
    /// Created at
    pub created_at: DateTime,
}

/// Health status
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall status (healthy, degraded, unhealthy)
    pub status: String,
    /// Component health checks
    pub components: Vec<ComponentHealth>,
    /// Error messages (if any)
    pub errors: Vec<String>,
    /// Warnings (if any)
    pub warnings: Vec<String>,
    /// Last check time
    pub checked_at: DateTime,
}

/// Individual component health
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Component status
    pub status: String,
    /// Response time in milliseconds
    pub response_time_ms: f64,
    /// Additional details
    pub details: Option<String>,
}

// ============================================================================
// POOL TYPES
// ============================================================================

/// Connection pool information
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct ConnectionPool {
    /// Pool ID
    pub id: String,
    /// Pool name
    pub name: String,
    /// Minimum connections
    pub min_connections: i32,
    /// Maximum connections
    pub max_connections: i32,
    /// Current active connections
    pub active_connections: i32,
    /// Current idle connections
    pub idle_connections: i32,
    /// Total connections
    pub total_connections: i32,
    /// Waiting requests
    pub waiting_requests: i32,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: i32,
    /// Idle timeout in seconds
    pub idle_timeout_seconds: i32,
}

/// Pool statistics
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct PoolStats {
    /// Pool ID
    pub pool_id: String,
    /// Connections created
    pub connections_created: BigInt,
    /// Connections destroyed
    pub connections_destroyed: BigInt,
    /// Connections acquired
    pub connections_acquired: BigInt,
    /// Connections released
    pub connections_released: BigInt,
    /// Acquire successes
    pub acquire_successes: BigInt,
    /// Acquire failures
    pub acquire_failures: BigInt,
    /// Acquire timeouts
    pub acquire_timeouts: BigInt,
    /// Average acquire time in milliseconds
    pub avg_acquire_time_ms: f64,
    /// Validation failures
    pub validation_failures: BigInt,
    /// Creation failures
    pub creation_failures: BigInt,
    /// Leaks detected
    pub leaks_detected: BigInt,
    /// Timestamp
    pub timestamp: DateTime,
}

/// Individual connection information
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct Connection {
    /// Connection ID
    pub id: String,
    /// Session ID
    pub session_id: Option<String>,
    /// Username
    pub username: String,
    /// Client address
    pub client_address: String,
    /// Database name
    pub database: String,
    /// Connection state (active, idle, waiting)
    pub state: String,
    /// Connected at
    pub connected_at: DateTime,
    /// Last activity
    pub last_activity: DateTime,
    /// Current query
    pub current_query: Option<String>,
    /// Queries executed
    pub queries_executed: BigInt,
    /// Transactions count
    pub transactions_count: BigInt,
}

/// Session information
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    /// Session ID
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Username
    pub username: String,
    /// Client address
    pub client_address: String,
    /// Database
    pub database: String,
    /// Session state
    pub state: String,
    /// Started at
    pub started_at: DateTime,
    /// Last command
    pub last_command: Option<String>,
    /// Last command time
    pub last_command_at: Option<DateTime>,
    /// Queries executed
    pub queries_executed: BigInt,
    /// Idle time in seconds
    pub idle_seconds: BigInt,
}

/// Table partition information
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct Partition {
    /// Partition ID
    pub id: String,
    /// Partition name
    pub name: String,
    /// Table name
    pub table_name: String,
    /// Partition type (range, list, hash)
    pub partition_type: String,
    /// Partition key
    pub partition_key: String,
    /// Partition value/expression
    pub partition_value: String,
    /// Row count
    pub row_count: BigInt,
    /// Size in bytes
    pub size_bytes: BigInt,
    /// Is default partition
    pub is_default: bool,
    /// Created at
    pub created_at: DateTime,
}

// ============================================================================
// ALERT TYPES
// ============================================================================

/// Alert severity enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// System alert
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct Alert {
    /// Alert ID
    pub id: String,
    /// Alert name
    pub name: String,
    /// Category
    pub category: String,
    /// Severity level
    pub severity: AlertSeverity,
    /// Alert state (active, acknowledged, resolved)
    pub state: String,
    /// Alert message
    pub message: String,
    /// Additional details
    pub details: Option<String>,
    /// Triggered at
    pub triggered_at: DateTime,
    /// Acknowledged at
    pub acknowledged_at: Option<DateTime>,
    /// Resolved at
    pub resolved_at: Option<DateTime>,
    /// Acknowledged by user
    pub acknowledged_by: Option<String>,
    /// Escalation level
    pub escalation_level: i32,
    /// Occurrence count
    pub occurrence_count: BigInt,
}

/// Server information
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server version
    pub version: String,
    /// Server build date
    pub build_date: String,
    /// Git commit hash
    pub git_commit: Option<String>,
    /// Uptime in seconds
    pub uptime_seconds: BigInt,
    /// Start time
    pub start_time: DateTime,
    /// Server hostname
    pub hostname: String,
    /// Operating system
    pub os: String,
    /// CPU architecture
    pub arch: String,
    /// Number of CPU cores
    pub cpu_cores: i32,
    /// Total memory in bytes
    pub total_memory: BigInt,
    /// Page size in bytes
    pub page_size: i32,
}
