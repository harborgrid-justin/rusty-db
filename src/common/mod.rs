// # Common Types and Traits
//
// This module defines shared types, traits, and interfaces used across all RustyDB modules.
// It serves as the foundation for inter-module communication and ensures consistency.
//
// ## Core Concepts
//
// - **Component Traits**: Standardized lifecycle and behavior interfaces
// - **Shared Types**: Common data structures (identifiers, values, schemas)
// - **Integration Contracts**: Well-defined APIs for module interaction
//
// ## Usage
//
// ```rust
// use rusty_db::common::*;
// use rusty_db::Result;
//
// struct MyComponent {
//     // ...
// }
//
// impl Component for MyComponent {
//     fn initialize(&mut self) -> Result<()> {
//         // Initialization logic
//         Ok(())
//     }
//
//     fn shutdown(&mut self) -> Result<()> {
//         // Cleanup logic
//         Ok(())
//     }
//
//     fn health_check(&self) -> HealthStatus {
//         HealthStatus::Healthy
//     }
// }
// ```

use crate::Result;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, SystemTime};

// ============================================================================
// Collection Size Limits - Prevent unbounded memory allocation
// ============================================================================

// TODO: Enforce these limits in collection operations to prevent DoS attacks
// These limits prevent unbounded memory growth from malicious or buggy code

/// Maximum number of columns per table (prevents schema bloat)
pub const MAX_COLUMNS_PER_TABLE: usize = 1024;

/// Maximum number of values in a tuple (same as MAX_COLUMNS_PER_TABLE)
pub const MAX_TUPLE_VALUES: usize = MAX_COLUMNS_PER_TABLE;

/// Maximum number of foreign keys per table
pub const MAX_FOREIGN_KEYS_PER_TABLE: usize = 256;

/// Maximum number of unique constraints per table
pub const MAX_UNIQUE_CONSTRAINTS_PER_TABLE: usize = 256;

/// Maximum number of active transactions in a snapshot
pub const MAX_ACTIVE_TRANSACTIONS: usize = 100_000;

/// Maximum number of custom metrics per component
pub const MAX_CUSTOM_METRICS: usize = 1_000;

/// Maximum depth of nested Value::Array to prevent stack overflow
pub const MAX_VALUE_NESTING_DEPTH: usize = 32;

/// Maximum size of error message strings (prevent memory exhaustion)
pub const MAX_ERROR_MESSAGE_LENGTH: usize = 4096;

// ============================================================================
// Type Aliases - Shared Identifiers
// ============================================================================

/// Unique identifier for transactions
pub type TransactionId = u64;

/// Unique identifier for pages in storage
pub type PageId = u64;

/// Unique identifier for tables in the catalog
pub type TableId = u32;

/// Unique identifier for indexes
pub type IndexId = u32;

/// Unique identifier for columns within a table
pub type ColumnId = u16;

/// Unique identifier for rows (physical location)
pub type RowId = u64;

/// Log Sequence Number for write-ahead logging
pub type LogSequenceNumber = u64;

/// Node identifier in a cluster
pub type NodeId = String;

/// Session identifier for user connections
pub type SessionId = u64;

// ============================================================================
// Core Value Types
// ============================================================================

/// Represents all possible data values in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    /// NULL value
    Null,

    /// Boolean true/false
    Boolean(bool),

    /// 64-bit signed integer
    Integer(i64),

    /// 64-bit floating point
    Float(f64),

    /// Variable-length string (UTF-8)
    String(String),

    /// Binary data
    Bytes(Vec<u8>),

    /// Date (days since epoch)
    Date(i64),

    /// Timestamp (microseconds since epoch)
    Timestamp(i64),

    /// JSON value
    Json(serde_json::Value),

    /// Array of values
    Array(Vec<Value>),
    Text,
}

impl Value {
    /// Check if value is NULL
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Get type name as string
    pub fn type_name(&self) -> &str {
        match self {
            Value::Null => "NULL",
            Value::Boolean(_) => "BOOLEAN",
            Value::Integer(_) => "INTEGER",
            Value::Float(_) => "FLOAT",
            Value::String(_) => "STRING",
            Value::Bytes(_) => "BYTES",
            Value::Date(_) => "DATE",
            Value::Timestamp(_) => "TIMESTAMP",
            Value::Json(_) => "JSON",
            Value::Array(_) => "ARRAY",
            Value::Text => "TEXT",
        }
    }

    /// Convert to string for display
    pub fn to_display_string(&self) -> String {
        match self {
            Value::Null => "NULL".to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Bytes(b) => format!("<{} bytes>", b.len()),
            Value::Date(d) => format!("DATE({})", d),
            Value::Timestamp(t) => format!("TIMESTAMP({})", t),
            Value::Json(j) => j.to_string(),
            Value::Array(a) => format!("[{}]", a.len()),
            Value::Text => "TEXT".to_string(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::Timestamp(a), Value::Timestamp(b)) => a == b,
            (Value::Json(a), Value::Json(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Null => {}
            Value::Boolean(b) => b.hash(state),
            Value::Integer(i) => i.hash(state),
            Value::Float(f) => f.to_bits().hash(state),
            Value::String(s) => s.hash(state),
            Value::Bytes(b) => b.hash(state),
            Value::Date(d) => d.hash(state),
            Value::Timestamp(t) => t.hash(state),
            Value::Json(j) => j.to_string().hash(state),
            Value::Array(a) => a.hash(state),
            Value::Text => "TEXT".hash(state),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Value::Null, Value::Null) => Some(Ordering::Equal),
            (Value::Null, _) => Some(Ordering::Less),
            (_, Value::Null) => Some(Ordering::Greater),
            (Value::Boolean(a), Value::Boolean(b)) => a.partial_cmp(b),
            (Value::Integer(a), Value::Integer(b)) => a.partial_cmp(b),
            (Value::Float(a), Value::Float(b)) => {
                if a.is_nan() && b.is_nan() {
                    Some(Ordering::Equal)
                } else if a.is_nan() {
                    Some(Ordering::Greater)
                } else if b.is_nan() {
                    Some(Ordering::Less)
                } else {
                    a.partial_cmp(b)
                }
            }
            (Value::String(a), Value::String(b)) => a.partial_cmp(b),
            (Value::Bytes(a), Value::Bytes(b)) => a.partial_cmp(b),
            (Value::Date(a), Value::Date(b)) => a.partial_cmp(b),
            (Value::Timestamp(a), Value::Timestamp(b)) => a.partial_cmp(b),
            (Value::Array(a), Value::Array(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

// ============================================================================
// Tuple and Schema Definitions
// ============================================================================

/// Represents a row of data with values and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tuple {
    /// Column values in order
    pub values: Vec<Value>,

    /// Physical row identifier
    pub row_id: RowId,

    /// Transaction ID that created this version (for MVCC)
    pub xmin: Option<TransactionId>,

    /// Transaction ID that deleted this version (for MVCC)
    pub xmax: Option<TransactionId>,
}

impl Default for Tuple {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            row_id: 0,
            xmin: None,
            xmax: None,
        }
    }
}

impl Tuple {
    /// Create a new tuple
    pub fn new(values: Vec<Value>, row_id: RowId) -> Self {
        Self {
            values,
            row_id,
            xmin: None,
            xmax: None,
        }
    }

    /// Get value at column index
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }

    /// Check if tuple is visible to a transaction
    pub fn is_visible(&self, _txn_id: TransactionId, snapshot: &Snapshot) -> bool {
        // Simplified visibility check (actual MVCC logic is more complex)
        match (self.xmin, self.xmax) {
            (Some(xmin), None) => snapshot.is_visible(xmin),
            (Some(xmin), Some(xmax)) => snapshot.is_visible(xmin) && !snapshot.is_visible(xmax),
            _ => false,
        }
    }
}

/// Database schema definition
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Schema {
    /// Table name
    pub table_name: String,

    /// Column definitions
    pub columns: Vec<ColumnDef>,

    /// Primary key column indices
    pub primary_key: Option<Vec<ColumnId>>,

    /// Foreign key constraints
    pub foreign_keys: Vec<ForeignKeyConstraint>,

    /// Unique constraints
    pub unique_constraints: Vec<Vec<ColumnId>>,
}

impl Schema {
    /// Create a new schema
    pub fn new(table_name: String, columns: Vec<ColumnDef>) -> Self {
        Self {
            table_name,
            columns,
            primary_key: None,
            foreign_keys: Vec::new(),
            unique_constraints: Vec::new(),
        }
    }

    /// Create an empty schema
    pub fn empty() -> Self {
        Self::default()
    }

    /// Get column by name
    pub fn get_column(&self, name: &str) -> Option<&ColumnDef> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// Get column index by name
    pub fn get_column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == name)
    }
}

/// Column definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColumnDef {
    /// Column name
    pub name: String,

    /// Data type
    pub data_type: DataType,

    /// Is nullable
    pub nullable: bool,

    /// Default value
    pub default: Option<Value>,

    /// Is auto-increment
    pub auto_increment: bool,
}

impl ColumnDef {
    /// Create a new column definition
    pub fn new(name: String, data_type: DataType) -> Self {
        Self {
            name,
            data_type,
            nullable: true,
            default: None,
            auto_increment: false,
        }
    }

    /// Make column non-nullable
    pub fn not_null(mut self) -> Self {
        self.nullable = false;
        self
    }

    /// Set default value
    pub fn with_default(mut self, default: Value) -> Self {
        self.default = Some(default);
        self
    }
}

/// SQL data types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    /// 32-bit signed integer
    Integer,

    /// 64-bit signed integer
    BigInt,

    /// 32-bit floating point
    Float,

    /// 64-bit floating point
    Double,

    /// Variable-length string with max length
    Varchar(usize),

    /// Unlimited-length text
    Text,

    /// Boolean
    Boolean,

    /// Date (no time component)
    Date,

    /// Timestamp with timezone
    Timestamp,

    /// JSON document
    Json,

    /// Binary large object
    Blob,

    /// Decimal with precision and scale
    Decimal(u8, u8),

    /// Array of a specific type
    Array(Box<DataType>),
}

/// Foreign key constraint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForeignKeyConstraint {
    pub name: String,
    pub columns: Vec<ColumnId>,
    pub referenced_table: TableId,
    pub referenced_columns: Vec<ColumnId>,
    pub on_delete: ReferentialAction,
    pub on_update: ReferentialAction,
}

/// Referential action for foreign keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferentialAction {
    NoAction,
    Cascade,
    SetNull,
    SetDefault,
    Restrict,
}

// ============================================================================
// Transaction Types
// ============================================================================

/// Transaction isolation levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// Dirty reads allowed
    ReadUncommitted,

    /// Prevents dirty reads
    ReadCommitted,

    /// Prevents dirty and non-repeatable reads
    RepeatableRead,

    /// Full isolation (no anomalies)
    Serializable,

    /// MVCC-based snapshot isolation
    SnapshotIsolation,
}

impl Default for IsolationLevel {
    fn default() -> Self {
        IsolationLevel::ReadCommitted
    }
}

/// Transaction snapshot for MVCC
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Transaction ID when snapshot was taken
    pub snapshot_txn_id: TransactionId,

    /// Active transactions at snapshot time
    pub active_txns: Vec<TransactionId>,

    /// Minimum active transaction ID
    pub min_active_txn: TransactionId,

    /// Maximum committed transaction ID
    pub max_committed_txn: TransactionId,
}

impl Snapshot {
    /// Check if a transaction is visible in this snapshot
    pub fn is_visible(&self, txn_id: TransactionId) -> bool {
        // Transaction is visible if:
        // 1. It's committed before the snapshot
        // 2. It's not in the active list
        txn_id < self.snapshot_txn_id && !self.active_txns.contains(&txn_id)
    }
}

/// Lock modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockMode {
    /// Shared lock (read)
    Shared,

    /// Exclusive lock (write)
    Exclusive,

    /// Intent shared lock
    IntentShared,

    /// Intent exclusive lock
    IntentExclusive,

    /// Shared intent exclusive lock
    SharedIntentExclusive,

    /// Update lock
    Update,
}

impl LockMode {
    /// Get lock strength for ordering (part of lock manager API)
    #[allow(dead_code)]
    pub(crate) fn strength(&self) -> u8 {
        match self {
            LockMode::Shared => 1,
            LockMode::IntentShared => 2,
            LockMode::Update => 3,
            LockMode::IntentExclusive => 4,
            LockMode::SharedIntentExclusive => 5,
            LockMode::Exclusive => 6,
        }
    }

    /// Check if two lock modes are compatible (part of lock manager API)
    ///
    /// Full compatibility matrix:
    /// ```
    ///               | IS  | IX  | S   | SIX | U   | X   |
    /// --------------|-----|-----|-----|-----|-----|-----|
    /// IS (IntentSh) | YES | YES | YES | YES | YES | NO  |
    /// IX (IntentEx) | YES | YES | NO  | NO  | NO  | NO  |
    /// S  (Shared)   | YES | NO  | YES | NO  | YES | NO  |
    /// SIX (ShIntEx) | YES | NO  | NO  | NO  | NO  | NO  |
    /// U  (Update)   | YES | NO  | YES | NO  | NO  | NO  |
    /// X  (Exclusive)| NO  | NO  | NO  | NO  | NO  | NO  |
    /// ```
    #[allow(dead_code)]
    pub(crate) fn is_compatible(&self, other: &LockMode) -> bool {
        use LockMode::*;
        match (self, other) {
            // Intent Shared is compatible with all except Exclusive
            (IntentShared, IntentShared) => true,
            (IntentShared, IntentExclusive) => true,
            (IntentShared, Shared) => true,
            (IntentShared, SharedIntentExclusive) => true,
            (IntentShared, Update) => true,
            (IntentShared, Exclusive) => false,

            // Intent Exclusive is compatible with IS and IX only
            (IntentExclusive, IntentShared) => true,
            (IntentExclusive, IntentExclusive) => true,
            (IntentExclusive, Shared) => false,
            (IntentExclusive, SharedIntentExclusive) => false,
            (IntentExclusive, Update) => false,
            (IntentExclusive, Exclusive) => false,

            // Shared is compatible with IS, S, and U
            (Shared, IntentShared) => true,
            (Shared, IntentExclusive) => false,
            (Shared, Shared) => true,
            (Shared, SharedIntentExclusive) => false,
            (Shared, Update) => true,
            (Shared, Exclusive) => false,

            // Shared Intent Exclusive is compatible with IS only
            (SharedIntentExclusive, IntentShared) => true,
            (SharedIntentExclusive, IntentExclusive) => false,
            (SharedIntentExclusive, Shared) => false,
            (SharedIntentExclusive, SharedIntentExclusive) => false,
            (SharedIntentExclusive, Update) => false,
            (SharedIntentExclusive, Exclusive) => false,

            // Update is compatible with IS and S only
            (Update, IntentShared) => true,
            (Update, IntentExclusive) => false,
            (Update, Shared) => true,
            (Update, SharedIntentExclusive) => false,
            (Update, Update) => false,
            (Update, Exclusive) => false,

            // Exclusive is compatible with nothing
            (Exclusive, _) => false,
        }
    }
}

/// Lock resource types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LockResource {
    /// Lock entire table
    Table(TableId),

    /// Lock specific page
    Page(PageId),

    /// Lock specific row
    Row(TableId, RowId),

    /// Lock database-level resource
    Database,
}

// ============================================================================
// Component Lifecycle Traits
// ============================================================================

/// Health status of a component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Component is fully operational
    Healthy,

    /// Component is operational but degraded
    Degraded,

    /// Component is not operational
    Unhealthy,

    /// Component state is unknown
    Unknown,
}

/// Base trait for all major components
pub trait Component: Send + Sync {
    /// Initialize the component
    fn initialize(&mut self) -> Result<()>;

    /// Shutdown the component gracefully
    fn shutdown(&mut self) -> Result<()>;

    /// Check health status
    fn health_check(&self) -> HealthStatus;
}

/// Transaction-aware components
pub trait Transactional: Component {
    /// Begin a new transaction
    fn begin_transaction(&mut self, isolation: IsolationLevel) -> Result<TransactionId>;

    /// Commit a transaction
    fn commit(&mut self, txn_id: TransactionId) -> Result<()>;

    /// Rollback a transaction
    fn rollback(&mut self, txn_id: TransactionId) -> Result<()>;
}

/// Recoverable components (for crash recovery)
pub trait Recoverable: Component {
    /// Create a checkpoint
    fn checkpoint(&self) -> Result<()>;

    /// Recover from a specific log sequence number
    fn recover(&mut self, lsn: LogSequenceNumber) -> Result<()>;
}

/// Metric value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
    Summary {
        count: u64,
        sum: f64,
        min: f64,
        max: f64,
    },
}

/// Component statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatistics {
    pub component_name: String,
    pub uptime: Duration,
    pub total_operations: u64,
    pub failed_operations: u64,
    pub avg_latency_ms: f64,
    pub custom_metrics: HashMap<String, MetricValue>,
}

impl ComponentStatistics {
    pub fn new(component_name: String) -> Self {
        Self {
            component_name,
            uptime: Duration::from_secs(0),
            total_operations: 0,
            failed_operations: 0,
            avg_latency_ms: 0.0,
            custom_metrics: HashMap::new(),
        }
    }
}

/// Monitorable components (for metrics and observability)
pub trait Monitorable: Component {
    /// Collect current metrics
    fn collect_metrics(&self) -> HashMap<String, MetricValue>;

    /// Get statistics
    fn get_statistics(&self) -> ComponentStatistics;
}

/// Serializable state for replication
pub trait ReplicableState: Component {
    /// Serialize component state
    fn serialize_state(&self) -> Result<Vec<u8>>;

    /// Deserialize and apply state
    fn deserialize_state(&mut self, data: &[u8]) -> Result<()>;

    /// Get current state version
    fn state_version(&self) -> u64;
}

// ============================================================================
// Event System
// ============================================================================

/// System-wide events for inter-module communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    // Transaction events
    TransactionBegin {
        txn_id: TransactionId,
        isolation: IsolationLevel,
    },
    TransactionCommit {
        txn_id: TransactionId,
    },
    TransactionRollback {
        txn_id: TransactionId,
    },

    // Storage events
    PageEvicted {
        page_id: PageId,
    },
    CheckpointStarted {
        lsn: LogSequenceNumber,
    },
    CheckpointCompleted {
        lsn: LogSequenceNumber,
    },

    // Cluster events
    NodeJoined {
        node_id: NodeId,
    },
    NodeLeft {
        node_id: NodeId,
    },
    LeaderElected {
        node_id: NodeId,
    },

    // Security events
    UserLogin {
        username: String,
        session_id: SessionId,
    },
    UserLogout {
        session_id: SessionId,
    },
    AuthenticationFailed {
        username: String,
        reason: String,
    },
    PermissionDenied {
        user: String,
        resource: String,
    },

    // Performance events
    SlowQuery {
        sql: String,
        duration: Duration,
    },
    ResourceThresholdExceeded {
        resource: String,
        value: f64,
    },

    // Backup events
    BackupStarted {
        backup_id: String,
    },
    BackupCompleted {
        backup_id: String,
    },
    RestoreStarted {
        backup_id: String,
    },
    RestoreCompleted {
        backup_id: String,
    },
}

/// Event listener trait
pub trait EventListener: Send + Sync {
    /// Handle a system event
    fn on_event(&mut self, event: SystemEvent) -> Result<()>;
}

// ============================================================================
// Configuration
// ============================================================================

/// Global database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    // Storage configuration
    pub data_dir: String,
    pub page_size: usize,
    pub buffer_pool_size: usize,
    pub wal_dir: String,
    pub checkpoint_interval: Duration,

    // Transaction configuration
    pub default_isolation: IsolationLevel,
    pub lock_timeout: Duration,
    pub deadlock_detection_interval: Duration,

    // Network configuration
    pub listen_address: String,
    pub port: u16,
    pub api_port: u16,
    pub enable_rest_api: bool,
    pub max_connections: usize,
    pub connection_timeout: Duration,

    // Security configuration
    pub enable_tls: bool,
    pub enable_encryption: bool,
    pub password_min_length: usize,
    pub session_timeout: Duration,

    // Clustering configuration
    pub cluster_enabled: bool,
    pub node_id: String,
    pub seed_nodes: Vec<String>,
    pub replication_factor: usize,

    // Performance configuration
    pub worker_threads: usize,
    pub enable_jit: bool,
    pub enable_vectorization: bool,
    pub query_timeout: Option<Duration>,
    pub max_memory_mb: usize,

    // Monitoring configuration
    pub enable_metrics: bool,
    pub metrics_port: u16,
    pub slow_query_threshold: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            // Storage
            data_dir: "./data".to_string(),
            page_size: 8192,
            buffer_pool_size: 1000,
            wal_dir: "./wal".to_string(),
            checkpoint_interval: Duration::from_secs(300),

            // Transaction
            default_isolation: IsolationLevel::ReadCommitted,
            lock_timeout: Duration::from_secs(30),
            deadlock_detection_interval: Duration::from_secs(1),

            // Network
            listen_address: "127.0.0.1".to_string(),
            port: 5432,
            api_port: 8080,
            enable_rest_api: true,
            max_connections: 100,
            connection_timeout: Duration::from_secs(30),

            // Security
            enable_tls: true,
            enable_encryption: true,
            password_min_length: 8,
            session_timeout: Duration::from_secs(3600),

            // Clustering
            cluster_enabled: false,
            node_id: "node1".to_string(),
            seed_nodes: Vec::new(),
            replication_factor: 3,

            // Performance
            worker_threads: num_cpus(),
            enable_jit: false,
            enable_vectorization: true,
            query_timeout: Some(Duration::from_secs(300)),
            max_memory_mb: 4096,

            // Monitoring
            enable_metrics: true,
            metrics_port: 9090,
            slow_query_threshold: Duration::from_millis(1000),
        }
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

// ============================================================================
// Resource Management
// ============================================================================

/// Resource limits for queries and connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_bytes: usize,
    pub max_cpu_time: Duration,
    pub max_io_operations: usize,
    pub max_temp_space_bytes: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 1024 * 1024 * 1024, // 1GB
            max_cpu_time: Duration::from_secs(300),
            max_io_operations: 1_000_000,
            max_temp_space_bytes: 10 * 1024 * 1024 * 1024, // 10GB
        }
    }
}

/// Resource usage tracking
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    pub memory_bytes: usize,
    pub cpu_time: Duration,
    pub io_operations: usize,
    pub temp_space_bytes: usize,
    pub start_time: Option<SystemTime>,
}

impl ResourceUsage {
    pub fn new() -> Self {
        Self {
            start_time: Some(SystemTime::now()),
            ..Default::default()
        }
    }

    pub fn elapsed(&self) -> Option<Duration> {
        self.start_time.and_then(|start| start.elapsed().ok())
    }
}

// ============================================================================
// Submodules
// ============================================================================

/// Concurrent map patterns and DashMap migration guide
pub mod concurrent_map;

/// Bounded HashMap with LRU eviction for memory-safe collections
pub mod bounded_map;
pub use bounded_map::BoundedHashMap;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_display() {
        assert_eq!(Value::Null.to_display_string(), "NULL");
        assert_eq!(Value::Integer(42).to_display_string(), "42");
        assert_eq!(
            Value::String("hello".to_string()).to_display_string(),
            "hello"
        );
        assert_eq!(Value::Boolean(true).to_display_string(), "true");
    }

    #[test]
    fn test_value_type_name() {
        assert_eq!(Value::Null.type_name(), "NULL");
        assert_eq!(Value::Integer(42).type_name(), "INTEGER");
        assert_eq!(Value::String("test".to_string()).type_name(), "STRING");
    }

    #[test]
    fn test_schema_creation() {
        let columns = vec![
            ColumnDef::new("id".to_string(), DataType::Integer).not_null(),
            ColumnDef::new("name".to_string(), DataType::Varchar(255)),
        ];

        let schema = Schema::new("users".to_string(), columns);
        assert_eq!(schema.table_name, "users");
        assert_eq!(schema.columns.len(), 2);

        let id_col = schema.get_column("id").unwrap();
        assert_eq!(id_col.name, "id");
        assert!(!id_col.nullable);
    }

    #[test]
    fn test_tuple_creation() {
        let values = vec![Value::Integer(1), Value::String("Alice".to_string())];

        let tuple = Tuple::new(values, 100);
        assert_eq!(tuple.row_id, 100);
        assert_eq!(tuple.values.len(), 2);
        assert_eq!(tuple.get(0), Some(&Value::Integer(1)));
    }

    #[test]
    fn test_snapshot_visibility() {
        let snapshot = Snapshot {
            snapshot_txn_id: 100,
            active_txns: vec![98, 99],
            min_active_txn: 98,
            max_committed_txn: 97,
        };

        assert!(snapshot.is_visible(50)); // Committed before snapshot
        assert!(!snapshot.is_visible(98)); // Active transaction
        assert!(!snapshot.is_visible(100)); // After snapshot
    }
}
