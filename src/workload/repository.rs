// RustyDB Workload Repository - AWR-like Automatic Workload Repository
// Provides comprehensive workload capture, analysis, and historical trending

use std::collections::VecDeque;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration};
use parking_lot::RwLock;
use crate::Result;
use crate::error::DbError;

/// Unique identifier for a workload snapshot
pub type SnapshotId = u64;

/// Unique identifier for a baseline
pub type BaselineId = u64;

/// AWR-like workload repository for automatic workload capture and analysis
pub struct WorkloadRepository {
    /// All captured snapshots, indexed by snapshot ID
    snapshots: Arc<RwLock<BTreeMap<SnapshotId, WorkloadSnapshot>>>,

    /// Named baselines for comparison
    baselines: Arc<RwLock<HashMap<BaselineId, Baseline>>>,

    /// Configuration settings
    config: Arc<RwLock<RepositoryConfig>>,

    /// Snapshot metadata index for quick lookup
    snapshot_index: Arc<RwLock<SnapshotIndex>>,

    /// Aggregated metrics cache
    metric_cache: Arc<RwLock<MetricCache>>,

    /// Next snapshot ID
    next_snapshot_id: Arc<RwLock<SnapshotId>>,

    /// Next baseline ID
    next_baseline_id: Arc<RwLock<BaselineId>>,

    /// Background collection status
    collection_status: Arc<RwLock<CollectionStatus>>,
}

/// Configuration for the workload repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    /// Snapshot interval in seconds
    pub snapshot_interval_secs: u64,

    /// Retention period in days
    pub retention_days: u32,

    /// Maximum number of snapshots to keep
    pub max_snapshots: usize,

    /// Enable automatic snapshot collection
    pub auto_collection_enabled: bool,

    /// Top N queries to capture per snapshot
    pub top_sql_count: usize,

    /// Threshold for capturing long-running queries (ms)
    pub long_query_threshold_ms: u64,

    /// Enable metric aggregation
    pub enable_aggregation: bool,

    /// Aggregation window size (number of snapshots)
    pub aggregation_window: usize,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self {
            snapshot_interval_secs: 3600, // 1 hour
            retention_days: 30,
            max_snapshots: 720, // 30 days at 1 hour intervals
            auto_collection_enabled: true,
            top_sql_count: 100,
            long_query_threshold_ms: 5000,
            enable_aggregation: true,
            aggregation_window: 24, // 1 day at hourly snapshots
        }
    }
}

/// Complete workload snapshot capturing database state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadSnapshot {
    /// Unique snapshot identifier
    pub snapshot_id: SnapshotId,

    /// Timestamp when snapshot was taken
    pub timestamp: SystemTime,

    /// Database instance information
    pub instance_info: InstanceInfo,

    /// System-wide statistics
    pub system_stats: SystemStatistics,

    /// Top SQL statements by various metrics
    pub top_sql: Vec<SqlStatementStats>,

    /// Session statistics
    pub session_stats: Vec<SessionStats>,

    /// Wait event statistics
    pub wait_events: Vec<WaitEventStats>,

    /// I/O statistics
    pub io_stats: IoStatistics,

    /// Memory statistics
    pub memory_stats: MemoryStatistics,

    /// Time model statistics
    pub time_model: TimeModelStats,

    /// Load profile
    pub load_profile: LoadProfile,

    /// Operating system statistics
    pub os_stats: OsStatistics,

    /// Tablespace usage
    pub tablespace_usage: Vec<TablespaceStats>,

    /// Segment statistics (top tables/indexes)
    pub segment_stats: Vec<SegmentStats>,
}

/// Database instance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub instance_name: String,
    pub host_name: String,
    pub version: String,
    pub startup_time: SystemTime,
    pub database_size_bytes: u64,
    pub uptime_seconds: u64,
}

/// System-wide statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatistics {
    pub transactions_committed: u64,
    pub transactions_rolled_back: u64,
    pub queries_executed: u64,
    pub queries_per_second: f64,
    pub transactions_per_second: f64,
    pub logical_reads: u64,
    pub physical_reads: u64,
    pub physical_writes: u64,
    pub buffer_cache_hit_ratio: f64,
    pub library_cache_hit_ratio: f64,
    pub parse_count_total: u64,
    pub parse_count_hard: u64,
    pub execute_count: u64,
    pub user_calls: u64,
    pub recursive_calls: u64,
    pub redo_size_bytes: u64,
    pub redo_writes: u64,
    pub sorts_memory: u64,
    pub sorts_disk: u64,
}

/// SQL statement statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlStatementStats {
    pub sql_id: String,
    pub sql_text: String,
    pub sql_hash: u64,
    pub executions: u64,
    pub elapsed_time_micros: u64,
    pub cpu_time_micros: u64,
    pub buffer_gets: u64,
    pub disk_reads: u64,
    pub rows_processed: u64,
    pub parse_calls: u64,
    pub sorts: u64,
    pub fetches: u64,
    pub px_servers_executions: u64,
    pub elapsed_time_per_exec_micros: u64,
    pub cpu_time_per_exec_micros: u64,
    pub module: Option<String>,
    pub action: Option<String>,
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub session_id: u64,
    pub user_name: String,
    pub program: String,
    pub machine: String,
    pub status: String,
    pub logical_reads: u64,
    pub physical_reads: u64,
    pub cpu_time_micros: u64,
    pub elapsed_time_micros: u64,
    pub active_time_micros: u64,
    pub current_sql_id: Option<String>,
    pub wait_class: Option<String>,
    pub wait_event: Option<String>,
}

/// Wait event statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitEventStats {
    pub wait_class: String,
    pub wait_event: String,
    pub total_waits: u64,
    pub total_wait_time_micros: u64,
    pub avg_wait_time_micros: u64,
    pub time_waited_pct: f64,
}

/// I/O statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoStatistics {
    pub datafile_reads: u64,
    pub datafile_writes: u64,
    pub datafile_read_bytes: u64,
    pub datafile_write_bytes: u64,
    pub tempfile_reads: u64,
    pub tempfile_writes: u64,
    pub redo_writes: u64,
    pub redo_write_bytes: u64,
    pub avg_read_time_ms: f64,
    pub avg_write_time_ms: f64,
    pub iops: f64,
    pub throughput_mbps: f64,
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatistics {
    pub sga_size_bytes: u64,
    pub pga_size_bytes: u64,
    pub shared_pool_size_bytes: u64,
    pub buffer_cache_size_bytes: u64,
    pub log_buffer_size_bytes: u64,
    pub pga_aggregate_target_bytes: u64,
    pub pga_aggregate_allocated_bytes: u64,
    pub pga_aggregate_auto_target_bytes: u64,
    pub process_count: u32,
    pub session_count: u32,
}

/// Time model statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeModelStats {
    pub db_time_micros: u64,
    pub db_cpu_time_micros: u64,
    pub background_cpu_time_micros: u64,
    pub connection_mgmt_time_micros: u64,
    pub parse_time_elapsed_micros: u64,
    pub hard_parse_time_micros: u64,
    pub sql_execute_time_micros: u64,
    pub pl_sql_execution_time_micros: u64,
    pub pl_sql_compilation_time_micros: u64,
}

/// Load profile showing database workload characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadProfile {
    pub transactions_per_second: f64,
    pub commits_per_second: f64,
    pub rollbacks_per_second: f64,
    pub physical_reads_per_second: f64,
    pub physical_writes_per_second: f64,
    pub logical_reads_per_second: f64,
    pub redo_size_per_second_bytes: f64,
    pub parses_per_second: f64,
    pub hard_parses_per_second: f64,
    pub executes_per_second: f64,
    pub user_calls_per_second: f64,
}

/// Operating system statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsStatistics {
    pub cpu_count: u32,
    pub cpu_usage_pct: f64,
    pub load_average: f64,
    pub physical_memory_bytes: u64,
    pub free_memory_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_free_bytes: u64,
    pub network_rx_bytes_per_sec: f64,
    pub network_tx_bytes_per_sec: f64,
}

/// Tablespace usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TablespaceStats {
    pub tablespace_name: String,
    pub size_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub usage_pct: f64,
    pub datafiles: u32,
}

/// Segment (table/index) statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentStats {
    pub owner: String,
    pub segment_name: String,
    pub segment_type: String,
    pub tablespace_name: String,
    pub logical_reads: u64,
    pub physical_reads: u64,
    pub physical_writes: u64,
    pub buffer_busy_waits: u64,
    pub row_lock_waits: u64,
    pub space_used_bytes: u64,
}

/// Named baseline for comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub baseline_id: BaselineId,
    pub name: String,
    pub description: String,
    pub start_snapshot_id: SnapshotId,
    pub end_snapshot_id: SnapshotId,
    pub created_time: SystemTime,
    pub baseline_type: BaselineType,
}

/// Type of baseline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BaselineType {
    /// Static baseline for a specific time range
    Static,
    /// Moving window baseline
    MovingWindow,
    /// Template baseline for similar workloads
    Template,
}

/// Snapshot index for fast lookup
#[derive(Debug, Default)]
struct SnapshotIndex {
    /// Snapshots by time range
    by_time: BTreeMap<SystemTime, SnapshotId>,
    /// Snapshots by date (for retention)
    by_date: HashMap<String, Vec<SnapshotId>>,
}

/// Metric cache for aggregated data
#[derive(Debug, Default)]
struct MetricCache {
    /// Aggregated metrics by time window
    hourly_metrics: VecDeque<AggregatedMetrics>,
    daily_metrics: VecDeque<AggregatedMetrics>,
    /// Last cache update time
    last_update: Option<SystemTime>,
}

/// Aggregated metrics for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    pub period_start: SystemTime,
    pub period_end: SystemTime,
    pub snapshot_count: usize,
    pub avg_transactions_per_sec: f64,
    pub avg_queries_per_sec: f64,
    pub avg_cpu_usage_pct: f64,
    pub avg_buffer_hit_ratio: f64,
    pub total_executions: u64,
    pub peak_active_sessions: u32,
}

/// Collection status
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CollectionStatus {
    enabled: bool,
    last_collection_time: Option<SystemTime>,
    next_collection_time: Option<SystemTime>,
    snapshots_collected: u64,
    collection_errors: u64,
}

impl WorkloadRepository {
    /// Create a new workload repository with default configuration
    pub fn new() -> Self {
        Self::with_config(RepositoryConfig::default())
    }

    /// Create a new workload repository with custom configuration
    pub fn with_config(config: RepositoryConfig) -> Self {
        let now = SystemTime::now();
        let interval = Duration::from_secs(config.snapshot_interval_secs);

        Self {
            snapshots: Arc::new(RwLock::new(BTreeMap::new())),
            baselines: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(config)),
            snapshot_index: Arc::new(RwLock::new(SnapshotIndex::default())),
            metric_cache: Arc::new(RwLock::new(MetricCache::default())),
            next_snapshot_id: Arc::new(RwLock::new(1)),
            next_baseline_id: Arc::new(RwLock::new(1)),
            collection_status: Arc::new(RwLock::new(CollectionStatus {
                enabled: true,
                last_collection_time: None,
                next_collection_time: Some(now + interval),
                snapshots_collected: 0,
                collection_errors: 0,
            })),
        }
    }

    /// Capture a workload snapshot
    pub fn capture_snapshot(&self, snapshot_data: WorkloadSnapshot) -> Result<SnapshotId> {
        let mut snapshots = self.snapshots.write();
        let mut index = self.snapshot_index.write();
        let mut next_id = self.next_snapshot_id.write();

        let snapshot_id = *next_id;
        *next_id += 1;

        let mut snapshot = snapshot_data;
        snapshot.snapshot_id = snapshot_id;

        // Update index
        index.by_time.insert(snapshot.timestamp, snapshot_id);

        let date_key = format!("{:?}", snapshot.timestamp));
        index.by_date.entry(date_key).or_insert_with(Vec::new).push(snapshot_id);

        // Store snapshot
        snapshots.insert(snapshot_id, snapshot);

        // Update collection status
        let mut status = self.collection_status.write();
        status.last_collection_time = Some(SystemTime::now());
        status.snapshots_collected += 1;

        // Check and apply retention policy
        drop(snapshots);
        drop(index);
        self.apply_retention_policy()?;

        Ok(snapshot_id)
    }

    /// Get a specific snapshot by ID
    pub fn get_snapshot(&self, snapshot_id: SnapshotId) -> Option<WorkloadSnapshot> {
        self.snapshots.read().get(&snapshot_id).cloned()
    }

    /// Get snapshots within a time range
    pub fn get_snapshots_in_range(
        &self,
        start_time: SystemTime,
        end_time: SystemTime,
    ) -> Vec<WorkloadSnapshot> {
        let snapshots = self.snapshots.read();
        let index = self.snapshot_index.read();

        let snapshot_ids: Vec<SnapshotId> = index
            .by_time
            .range(start_time..=end_time)
            .map(|(_, &id)| id)
            .collect();

        snapshot_ids
            .iter()
            .filter_map(|id| snapshots.get(id).cloned())
            .collect()
    }

    /// Create a named baseline
    pub fn create_baseline(
        &self,
        name: String,
        description: String,
        start_snapshot_id: SnapshotId,
        end_snapshot_id: SnapshotId,
        baseline_type: BaselineType,
    ) -> Result<BaselineId> {
        // Verify snapshots exist
        let snapshots = self.snapshots.read();
        if !snapshots.contains_key(&start_snapshot_id) || !snapshots.contains_key(&end_snapshot_id) {
            return Err(DbError::InvalidInput("Invalid snapshot IDs".to_string()));
        }
        drop(snapshots);

        let mut baselines = self.baselines.write();
        let mut next_id = self.next_baseline_id.write();

        let baseline_id = *next_id;
        *next_id += 1;

        let baseline = Baseline {
            baseline_id,
            name,
            description,
            start_snapshot_id,
            end_snapshot_id,
            created_time: SystemTime::now(),
            baseline_type,
        };

        baselines.insert(baseline_id, baseline);
        Ok(baseline_id)
    }

    /// Get baseline by ID
    pub fn get_baseline(&self, baseline_id: BaselineId) -> Option<Baseline> {
        self.baselines.read().get(&baseline_id).cloned()
    }

    /// List all baselines
    pub fn list_baselines(&self) -> Vec<Baseline> {
        self.baselines.read().values().cloned().collect()
    }

    /// Delete a baseline
    pub fn delete_baseline(&self, baseline_id: BaselineId) -> Result<()> {
        let mut baselines = self.baselines.write();
        if baselines.remove(&baseline_id).is_some() {
            Ok(())
        } else {
            Err(DbError::NotFound(format!("Baseline {} not found", baseline_id)))
        }
    }

    /// Compare two snapshots
    pub fn compare_snapshots(
        &self,
        snapshot1_id: SnapshotId,
        snapshot2_id: SnapshotId,
    ) -> Result<SnapshotComparison> {
        let snapshots = self.snapshots.read());

        let snap1 = snapshots
            .get(&snapshot1_id)
            .ok_or_else(|| DbError::NotFound(format!("Snapshot {} not found", snapshot1_id)))?);
        let snap2 = snapshots
            .get(&snapshot2_id)
            .ok_or_else(|| DbError::NotFound(format!("Snapshot {} not found", snapshot2_id)))?);

        Ok(SnapshotComparison::new(snap1.clone(), snap2.clone()))
    }

    /// Generate aggregated metrics for a time range
    pub fn get_aggregated_metrics(
        &self,
        start_time: SystemTime,
        end_time: SystemTime,
    ) -> AggregatedMetrics {
        let snapshots = self.get_snapshots_in_range(start_time, end_time);

        if snapshots.is_empty() {
            return AggregatedMetrics {
                period_start: start_time,
                period_end: end_time,
                snapshot_count: 0,
                avg_transactions_per_sec: 0.0,
                avg_queries_per_sec: 0.0,
                avg_cpu_usage_pct: 0.0,
                avg_buffer_hit_ratio: 0.0,
                total_executions: 0,
                peak_active_sessions: 0,
            };
        }

        let count = snapshots.len() as f64;
        let mut total_tps = 0.0;
        let mut total_qps = 0.0;
        let mut total_cpu = 0.0;
        let mut total_hit_ratio = 0.0;
        let mut total_execs = 0u64;
        let mut peak_sessions = 0u32;

        for snapshot in &snapshots {
            total_tps += snapshot.system_stats.transactions_per_second;
            total_qps += snapshot.system_stats.queries_per_second;
            total_cpu += snapshot.os_stats.cpu_usage_pct;
            total_hit_ratio += snapshot.system_stats.buffer_cache_hit_ratio;
            total_execs += snapshot.system_stats.execute_count;
            peak_sessions = peak_sessions.max(snapshot.memory_stats.session_count);
        }

        AggregatedMetrics {
            period_start: start_time,
            period_end: end_time,
            snapshot_count: snapshots.len(),
            avg_transactions_per_sec: total_tps / count,
            avg_queries_per_sec: total_qps / count,
            avg_cpu_usage_pct: total_cpu / count,
            avg_buffer_hit_ratio: total_hit_ratio / count,
            total_executions: total_execs,
            peak_active_sessions: peak_sessions,
        }
    }

    /// Apply retention policy to remove old snapshots
    fn apply_retention_policy(&self) -> Result<()> {
        let config = self.config.read();
        let retention_days = config.retention_days;
        let max_snapshots = config.max_snapshots;
        drop(config);

        let mut snapshots = self.snapshots.write();
        let mut index = self.snapshot_index.write();

        // Remove snapshots older than retention period
        let cutoff_time = SystemTime::now() - Duration::from_secs(retention_days as u64 * 86400);

        let to_remove: Vec<SnapshotId> = snapshots
            .iter()
            .filter(|(_, snap)| snap.timestamp < cutoff_time)
            .map(|(&id, _)| id)
            .collect();

        for id in to_remove {
            if let Some(snap) = snapshots.remove(&id) {
                index.by_time.remove(&snap.timestamp);
            }
        }

        // Limit total number of snapshots
        while snapshots.len() > max_snapshots {
            if let Some((&oldest_id, _)) = snapshots.iter().next() {
                if let Some(snap) = snapshots.remove(&oldest_id) {
                    index.by_time.remove(&snap.timestamp);
                }
            }
        }

        Ok(())
    }

    /// Get repository statistics
    pub fn get_repository_stats(&self) -> RepositoryStats {
        let snapshots = self.snapshots.read();
        let baselines = self.baselines.read();
        let status = self.collection_status.read();

        RepositoryStats {
            total_snapshots: snapshots.len(),
            total_baselines: baselines.len(),
            oldest_snapshot_time: snapshots.values().map(|s| s.timestamp).min(),
            newest_snapshot_time: snapshots.values().map(|s| s.timestamp).max(),
            collection_enabled: status.enabled,
            last_collection: status.last_collection_time,
            snapshots_collected: status.snapshots_collected,
            collection_errors: status.collection_errors,
        }
    }

    /// Purge all snapshots (use with caution!)
    pub fn purge_all_snapshots(&self) -> Result<usize> {
        let mut snapshots = self.snapshots.write();
        let mut index = self.snapshot_index.write();

        let count = snapshots.len();
        snapshots.clear();
        index.by_time.clear();
        index.by_date.clear();

        Ok(count)
    }
}

impl Default for WorkloadRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Repository statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryStats {
    pub total_snapshots: usize,
    pub total_baselines: usize,
    pub oldest_snapshot_time: Option<SystemTime>,
    pub newest_snapshot_time: Option<SystemTime>,
    pub collection_enabled: bool,
    pub last_collection: Option<SystemTime>,
    pub snapshots_collected: u64,
    pub collection_errors: u64,
}

/// Comparison between two snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotComparison {
    pub snapshot1: WorkloadSnapshot,
    pub snapshot2: WorkloadSnapshot,
    pub delta_stats: DeltaStatistics,
}

impl SnapshotComparison {
    fn new(snapshot1: WorkloadSnapshot, snapshot2: WorkloadSnapshot) -> Self {
        let delta_stats = DeltaStatistics {
            delta_transactions: snapshot2.system_stats.queries_executed as i64
                - snapshot1.system_stats.queries_executed as i64,
            delta_commits: snapshot2.system_stats.transactions_committed as i64
                - snapshot1.system_stats.transactions_committed as i64,
            delta_logical_reads: snapshot2.system_stats.logical_reads as i64
                - snapshot1.system_stats.logical_reads as i64,
            delta_physical_reads: snapshot2.system_stats.physical_reads as i64
                - snapshot1.system_stats.physical_reads as i64,
            delta_cpu_usage_pct: snapshot2.os_stats.cpu_usage_pct - snapshot1.os_stats.cpu_usage_pct,
            delta_buffer_hit_ratio: snapshot2.system_stats.buffer_cache_hit_ratio
                - snapshot1.system_stats.buffer_cache_hit_ratio,
        };

        Self {
            snapshot1,
            snapshot2,
            delta_stats,
        }
    }
}

/// Delta statistics between two snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaStatistics {
    pub delta_transactions: i64,
    pub delta_commits: i64,
    pub delta_logical_reads: i64,
    pub delta_physical_reads: i64,
    pub delta_cpu_usage_pct: f64,
    pub delta_buffer_hit_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_snapshot() -> WorkloadSnapshot {
        WorkloadSnapshot {
            snapshot_id: 0,
            timestamp: SystemTime::now(),
            instance_info: InstanceInfo {
                instance_name: "rustydb1".to_string(),
                host_name: "testhost".to_string(),
                version: "1.0.0".to_string(),
                startup_time: SystemTime::now(),
                database_size_bytes: 1024 * 1024 * 1024,
                uptime_seconds: 3600,
            },
            system_stats: SystemStatistics {
                transactions_committed: 1000,
                transactions_rolled_back: 10,
                queries_executed: 5000,
                queries_per_second: 100.0,
                transactions_per_second: 20.0,
                logical_reads: 100000,
                physical_reads: 5000,
                physical_writes: 2000,
                buffer_cache_hit_ratio: 95.0,
                library_cache_hit_ratio: 98.0,
                parse_count_total: 500,
                parse_count_hard: 50,
                execute_count: 5000,
                user_calls: 1000,
                recursive_calls: 4000,
                redo_size_bytes: 1024 * 1024,
                redo_writes: 100,
                sorts_memory: 450,
                sorts_disk: 50,
            },
            top_sql: vec![],
            session_stats: vec![],
            wait_events: vec![],
            io_stats: IoStatistics {
                datafile_reads: 5000,
                datafile_writes: 2000,
                datafile_read_bytes: 5000 * 8192,
                datafile_write_bytes: 2000 * 8192,
                tempfile_reads: 100,
                tempfile_writes: 100,
                redo_writes: 100,
                redo_write_bytes: 1024 * 1024,
                avg_read_time_ms: 2.5,
                avg_write_time_ms: 5.0,
                iops: 1000.0,
                throughput_mbps: 50.0,
            },
            memory_stats: MemoryStatistics {
                sga_size_bytes: 512 * 1024 * 1024,
                pga_size_bytes: 256 * 1024 * 1024,
                shared_pool_size_bytes: 128 * 1024 * 1024,
                buffer_cache_size_bytes: 256 * 1024 * 1024,
                log_buffer_size_bytes: 16 * 1024 * 1024,
                pga_aggregate_target_bytes: 256 * 1024 * 1024,
                pga_aggregate_allocated_bytes: 200 * 1024 * 1024,
                pga_aggregate_auto_target_bytes: 256 * 1024 * 1024,
                process_count: 100,
                session_count: 50,
            },
            time_model: TimeModelStats {
                db_time_micros: 3600 * 1_000_000,
                db_cpu_time_micros: 1800 * 1_000_000,
                background_cpu_time_micros: 100 * 1_000_000,
                connection_mgmt_time_micros: 10 * 1_000_000,
                parse_time_elapsed_micros: 50 * 1_000_000,
                hard_parse_time_micros: 25 * 1_000_000,
                sql_execute_time_micros: 3500 * 1_000_000,
                pl_sql_execution_time_micros: 100 * 1_000_000,
                pl_sql_compilation_time_micros: 10 * 1_000_000,
            },
            load_profile: LoadProfile {
                transactions_per_second: 20.0,
                commits_per_second: 18.0,
                rollbacks_per_second: 2.0,
                physical_reads_per_second: 50.0,
                physical_writes_per_second: 20.0,
                logical_reads_per_second: 1000.0,
                redo_size_per_second_bytes: 10240.0,
                parses_per_second: 5.0,
                hard_parses_per_second: 0.5,
                executes_per_second: 100.0,
                user_calls_per_second: 20.0,
            },
            os_stats: OsStatistics {
                cpu_count: 8,
                cpu_usage_pct: 45.0,
                load_average: 2.5,
                physical_memory_bytes: 16 * 1024 * 1024 * 1024,
                free_memory_bytes: 4 * 1024 * 1024 * 1024,
                swap_total_bytes: 8 * 1024 * 1024 * 1024,
                swap_free_bytes: 7 * 1024 * 1024 * 1024,
                network_rx_bytes_per_sec: 1024 * 1024.0,
                network_tx_bytes_per_sec: 512 * 1024.0,
            },
            tablespace_usage: vec![],
            segment_stats: vec![],
        }
    }

    #[test]
    fn test_repository_creation() {
        let repo = WorkloadRepository::new();
        let stats = repo.get_repository_stats();
        assert_eq!(stats.total_snapshots, 0);
        assert_eq!(stats.total_baselines, 0);
    }

    #[test]
    fn test_snapshot_capture() {
        let repo = WorkloadRepository::new();
        let snapshot = create_test_snapshot();

        let snapshot_id = repo.capture_snapshot(snapshot).unwrap();
        assert_eq!(snapshot_id, 1);

        let retrieved = repo.get_snapshot(snapshot_id);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_baseline_creation() {
        let repo = WorkloadRepository::new();
        let snap1 = create_test_snapshot();
        let snap2 = create_test_snapshot();

        let id1 = repo.capture_snapshot(snap1).unwrap();
        let id2 = repo.capture_snapshot(snap2).unwrap();

        let baseline_id = repo
            .create_baseline(
                "test_baseline".to_string(),
                "Test baseline".to_string(),
                id1,
                id2,
                BaselineType::Static,
            )
            .unwrap();

        assert_eq!(baseline_id, 1);

        let baseline = repo.get_baseline(baseline_id);
        assert!(baseline.is_some());
    }
}


