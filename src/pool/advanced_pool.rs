// # Advanced Connection Pool Features
//
// This module provides advanced connection pooling capabilities including:
// - Connection tagging and affinity for session state preservation
// - Connection validation and health checks
// - Automatic connection recycling based on age, usage, and errors
// - Workload-based pool partitioning (OLTP vs OLAP)
//
// ## Connection Tagging
//
// Tags allow applications to request connections with specific characteristics:
// - Session state preservation (temp tables, variables, cursors)
// - Application context (user, tenant, department)
// - Performance hints (read-only, batch mode)
//
// ## Connection Affinity
//
// Affinity ensures that repeated requests from the same source get the same
// connection when possible, improving performance through:
// - Prepared statement reuse
// - Session state preservation
// - Cache locality
//
// ## Health Checks
//
// Comprehensive health validation including:
// - Basic connectivity tests
// - Query execution validation
// - Transaction state verification
// - Resource leak detection

use crate::error::{DbError, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// Connection Tagging
// ============================================================================

/// Tag key-value pair for connection metadata
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionTag {
    pub key: String,
    pub value: String,
}

impl ConnectionTag {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }

    /// Common tags for application context
    pub fn app_name(name: &str) -> Self {
        Self::new("app_name", name)
    }

    pub fn user_name(name: &str) -> Self {
        Self::new("user_name", name)
    }

    pub fn tenant_id(id: &str) -> Self {
        Self::new("tenant_id", id)
    }

    pub fn workload_type(workload: WorkloadType) -> Self {
        Self::new("workload", workload.to_string())
    }

    pub fn read_only(enabled: bool) -> Self {
        Self::new("read_only", enabled.to_string())
    }
}

/// Set of tags associated with a connection
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagSet {
    tags: HashMap<String, String>,
}

impl TagSet {
    pub fn new() -> Self {
        Self {
            tags: HashMap::new(),
        }
    }

    pub fn with_tag(mut self, tag: ConnectionTag) -> Self {
        self.tags.insert(tag.key, tag.value);
        self
    }

    pub fn add_tag(&mut self, tag: ConnectionTag) {
        self.tags.insert(tag.key, tag.value);
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.tags.get(key)
    }

    pub fn matches(&self, other: &TagSet) -> bool {
        other.tags.iter().all(|(k, v)| self.tags.get(k) == Some(v))
    }

    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tags.len()
    }
}

impl Hash for TagSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Sort keys for consistent hashing
        let mut keys: Vec<_> = self.tags.keys().collect();
        keys.sort();
        for key in keys {
            key.hash(state);
            self.tags[key].hash(state);
        }
    }
}

// ============================================================================
// Workload Types
// ============================================================================

/// Types of database workloads for pool partitioning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkloadType {
    /// Online Transaction Processing - short, frequent transactions
    OLTP,
    /// Online Analytical Processing - long-running queries
    OLAP,
    /// Batch processing - bulk operations
    Batch,
    /// Real-time analytics - streaming workloads
    Streaming,
    /// Administrative tasks - schema changes, maintenance
    Admin,
    /// General purpose - mixed workload
    Mixed,
}

impl fmt::Display for WorkloadType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkloadType::OLTP => write!(f, "OLTP"),
            WorkloadType::OLAP => write!(f, "OLAP"),
            WorkloadType::Batch => write!(f, "Batch"),
            WorkloadType::Streaming => write!(f, "Streaming"),
            WorkloadType::Admin => write!(f, "Admin"),
            WorkloadType::Mixed => write!(f, "Mixed"),
        }
    }
}

// ============================================================================
// Connection Affinity
// ============================================================================

/// Connection affinity manager - ensures clients get the same connection
pub struct AffinityManager {
    /// Map of client ID to preferred connection ID
    affinity_map: Arc<RwLock<HashMap<String, u64>>>,
    /// Reverse map for cleanup
    connection_clients: Arc<RwLock<HashMap<u64, HashSet<String>>>>,
    /// Statistics
    stats: Arc<AffinityStats>,
}

#[derive(Default)]
pub struct AffinityStats {
    pub affinity_hits: AtomicU64,
    pub affinity_misses: AtomicU64,
    pub affinity_created: AtomicU64,
    pub affinity_removed: AtomicU64,
}

impl AffinityManager {
    pub fn new() -> Self {
        Self {
            affinity_map: Arc::new(RwLock::new(HashMap::new())),
            connection_clients: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(AffinityStats::default()),
        }
    }

    /// Get the preferred connection ID for a client
    pub fn get_affinity(&self, client_id: &str) -> Option<u64> {
        let map = self.affinity_map.read();
        let conn_id = map.get(client_id).copied();

        if conn_id.is_some() {
            self.stats.affinity_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.stats.affinity_misses.fetch_add(1, Ordering::Relaxed);
        }

        conn_id
    }

    /// Set connection affinity for a client
    pub fn set_affinity(&self, client_id: String, conn_id: u64) {
        let mut map = self.affinity_map.write();
        let mut clients = self.connection_clients.write();

        // Remove old affinity if exists
        if let Some(old_conn) = map.get(&client_id) {
            if let Some(client_set) = clients.get_mut(old_conn) {
                client_set.remove(&client_id);
            }
        }

        // Set new affinity
        map.insert(client_id.clone(), conn_id);
        clients.entry(conn_id).or_insert_with(HashSet::new).insert(client_id);

        self.stats.affinity_created.fetch_add(1, Ordering::Relaxed);
    }

    /// Remove affinity for a connection (when connection is closed)
    pub fn remove_connection(&self, conn_id: u64) {
        let mut map = self.affinity_map.write();
        let mut clients = self.connection_clients.write();

        if let Some(client_set) = clients.remove(&conn_id) {
            for client_id in client_set {
                map.remove(&client_id);
                self.stats.affinity_removed.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Clear affinity for a specific client
    pub fn clear_affinity(&self, client_id: &str) {
        let mut map = self.affinity_map.write();
        let mut clients = self.connection_clients.write();

        if let Some(conn_id) = map.remove(client_id) {
            if let Some(client_set) = clients.get_mut(&conn_id) {
                client_set.remove(client_id);
            }
            self.stats.affinity_removed.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn stats(&self) -> AffinityStatsSnapshot {
        AffinityStatsSnapshot {
            affinity_hits: self.stats.affinity_hits.load(Ordering::Relaxed),
            affinity_misses: self.stats.affinity_misses.load(Ordering::Relaxed),
            affinity_created: self.stats.affinity_created.load(Ordering::Relaxed),
            affinity_removed: self.stats.affinity_removed.load(Ordering::Relaxed),
            active_affinities: self.affinity_map.read().len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityStatsSnapshot {
    pub affinity_hits: u64,
    pub affinity_misses: u64,
    pub affinity_created: u64,
    pub affinity_removed: u64,
    pub active_affinities: usize,
}

// ============================================================================
// Connection Health Checks
// ============================================================================

/// Health check result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    pub fn is_usable(&self) -> bool {
        !matches!(self, HealthStatus::Unhealthy { .. })
    }
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Enable basic connectivity check
    pub check_connectivity: bool,
    /// Enable query execution check
    pub check_query_execution: bool,
    /// Test query to execute (e.g., "SELECT 1")
    pub test_query: Option<String>,
    /// Timeout for health check
    pub timeout: Duration,
    /// Maximum consecutive failures before marking unhealthy
    pub max_consecutive_failures: usize,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_connectivity: true,
            check_query_execution: true,
            test_query: Some("SELECT 1".to_string()),
            timeout: Duration::from_secs(5),
            max_consecutive_failures: 3,
        }
    }
}

/// Connection health tracker
pub struct HealthTracker {
    config: HealthCheckConfig,
    consecutive_failures: AtomicUsize,
    last_check: RwLock<Option<Instant>>,
    last_status: RwLock<HealthStatus>,
    total_checks: AtomicU64,
    total_failures: AtomicU64,
}

impl HealthTracker {
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            config,
            consecutive_failures: AtomicUsize::new(0),
            last_check: RwLock::new(None),
            last_status: RwLock::new(HealthStatus::Healthy),
            total_checks: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
        }
    }

    /// Record health check result
    pub fn record_check(&self, status: HealthStatus) {
        self.total_checks.fetch_add(1, Ordering::Relaxed);
        *self.last_check.write() = Some(Instant::now());

        if status.is_healthy() {
            self.consecutive_failures.store(0, Ordering::Relaxed);
        } else {
            self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
            self.total_failures.fetch_add(1, Ordering::Relaxed);
        }

        *self.last_status.write() = status;
    }

    /// Get current health status
    pub fn status(&self) -> HealthStatus {
        self.last_status.read().clone()
    }

    /// Check if connection should be considered unhealthy
    pub fn is_unhealthy(&self) -> bool {
        let failures = self.consecutive_failures.load(Ordering::Relaxed);
        failures >= self.config.max_consecutive_failures
    }

    pub fn stats(&self) -> HealthCheckStats {
        HealthCheckStats {
            total_checks: self.total_checks.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            consecutive_failures: self.consecutive_failures.load(Ordering::Relaxed),
            last_check: *self.last_check.read(),
            current_status: self.last_status.read().clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckStats {
    pub total_checks: u64,
    pub total_failures: u64,
    pub consecutive_failures: usize,
    #[serde(skip)]
    pub last_check: Option<Instant>,
    pub current_status: HealthStatus,
}

// ============================================================================
// Automatic Connection Recycling
// ============================================================================

/// Recycling triggers - conditions that cause connection recycling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecyclingTriggers {
    /// Recycle after this many uses
    pub max_uses: Option<u64>,
    /// Recycle after this amount of time
    pub max_age: Option<Duration>,
    /// Recycle after this many errors
    pub max_errors: Option<u64>,
    /// Recycle if idle for this long
    pub max_idle_time: Option<Duration>,
    /// Recycle on specific error types
    pub error_triggers: Vec<String>,
}

impl Default for RecyclingTriggers {
    fn default() -> Self {
        Self {
            max_uses: Some(10000),
            max_age: Some(Duration::from_secs(3600)), // 1 hour
            max_errors: Some(10),
            max_idle_time: Some(Duration::from_secs(600)), // 10 minutes
            error_triggers: vec![
                "connection_lost".to_string(),
                "protocol_error".to_string(),
                "authentication_failed".to_string(),
            ],
        }
    }
}

/// Connection usage tracker for recycling decisions
pub struct UsageTracker {
    created_at: Instant,
    use_count: AtomicU64,
    error_count: AtomicU64,
    last_used: RwLock<Instant>,
    triggers: RecyclingTriggers,
}

impl UsageTracker {
    pub fn new(triggers: RecyclingTriggers) -> Self {
        let now = Instant::now();
        Self {
            created_at: now,
            use_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            last_used: RwLock::new(now),
            triggers,
        }
    }

    /// Record a connection use
    pub fn record_use(&self) {
        self.use_count.fetch_add(1, Ordering::Relaxed);
        *self.last_used.write() = Instant::now();
    }

    /// Record an error
    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Check if connection should be recycled
    pub fn should_recycle(&self) -> bool {
        let now = Instant::now();

        // Check use count
        if let Some(max_uses) = self.triggers.max_uses {
            if self.use_count.load(Ordering::Relaxed) >= max_uses {
                return true;
            }
        }

        // Check age
        if let Some(max_age) = self.triggers.max_age {
            if now.duration_since(self.created_at) >= max_age {
                return true;
            }
        }

        // Check error count
        if let Some(max_errors) = self.triggers.max_errors {
            if self.error_count.load(Ordering::Relaxed) >= max_errors {
                return true;
            }
        }

        // Check idle time
        if let Some(max_idle) = self.triggers.max_idle_time {
            let last_used = *self.last_used.read();
            if now.duration_since(last_used) >= max_idle {
                return true;
            }
        }

        false
    }

    pub fn stats(&self) -> UsageStats {
        UsageStats {
            age: Instant::now().duration_since(self.created_at),
            use_count: self.use_count.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
            last_used: *self.last_used.read(),
            time_since_last_use: Instant::now().duration_since(*self.last_used.read()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub age: Duration,
    pub use_count: u64,
    pub error_count: u64,
    #[serde(skip, default = "Instant::now")]
    pub last_used: Instant,
    pub time_since_last_use: Duration,
}
