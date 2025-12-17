// Real-Time Dashboard Data
// WebSocket-ready metrics streaming, top queries, connection pool status, replication lag

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

// SAFETY: Maximum time series points to prevent OOM (Issue C-06)
// At 1-second granularity, 86400 = 24 hours of history
const MAX_TIME_SERIES_POINTS: usize = 86_400; // 24 hours at 1Hz
const DEFAULT_DASHBOARD_HISTORY: usize = 3_600; // 1 hour default

// Real-time metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDataPoint {
    pub timestamp: SystemTime,
    pub value: f64,
}

impl MetricDataPoint {
    pub fn new(value: f64) -> Self {
        Self {
            timestamp: SystemTime::now(),
            value,
        }
    }

    pub fn with_timestamp(mut self, timestamp: SystemTime) -> Self {
        self.timestamp = timestamp;
        self
    }
}

// Time series metric for dashboard visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesMetric {
    pub name: String,
    pub unit: String,
    pub data_points: VecDeque<MetricDataPoint>,
    pub max_points: usize,
}

impl TimeSeriesMetric {
    pub fn new(name: impl Into<String>, unit: impl Into<String>, max_points: usize) -> Self {
        // SAFETY: Clamp to prevent OOM (Issue C-06)
        let safe_max_points = max_points.min(MAX_TIME_SERIES_POINTS);

        Self {
            name: name.into(),
            unit: unit.into(),
            data_points: VecDeque::with_capacity(safe_max_points),
            max_points: safe_max_points,
        }
    }

    pub fn add_point(&mut self, value: f64) {
        if self.data_points.len() >= self.max_points {
            self.data_points.pop_front();
        }
        self.data_points.push_back(MetricDataPoint::new(value));
    }

    pub fn get_latest(&self) -> Option<f64> {
        self.data_points.back().map(|p| p.value)
    }

    pub fn get_average(&self) -> Option<f64> {
        if self.data_points.is_empty() {
            None
        } else {
            let sum: f64 = self.data_points.iter().map(|p| p.value).sum();
            Some(sum / self.data_points.len() as f64)
        }
    }

    pub fn get_min(&self) -> Option<f64> {
        self.data_points
            .iter()
            .map(|p| p.value)
            .fold(None, |min, v| {
                Some(min.map_or(v, |m| if v < m { v } else { m }))
            })
    }

    pub fn get_max(&self) -> Option<f64> {
        self.data_points
            .iter()
            .map(|p| p.value)
            .fold(None, |max, v| {
                Some(max.map_or(v, |m| if v > m { v } else { m }))
            })
    }
}

// Top query information for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopQuery {
    pub sql_id: u64,
    pub sql_text: String,
    pub executions: u64,
    pub total_elapsed_ms: f64,
    pub avg_elapsed_ms: f64,
    pub cpu_time_ms: f64,
    pub rows_returned: u64,
    pub disk_reads: u64,
    pub buffer_gets: u64,
    pub last_execution: SystemTime,
}

impl TopQuery {
    pub fn new(sql_id: u64, sql_text: impl Into<String>) -> Self {
        Self {
            sql_id,
            sql_text: sql_text.into(),
            executions: 0,
            total_elapsed_ms: 0.0,
            avg_elapsed_ms: 0.0,
            cpu_time_ms: 0.0,
            rows_returned: 0,
            disk_reads: 0,
            buffer_gets: 0,
            last_execution: SystemTime::now(),
        }
    }

    pub fn update_stats(&mut self, elapsed_ms: f64, cpu_ms: f64, rows: u64, reads: u64, gets: u64) {
        self.executions += 1;
        self.total_elapsed_ms += elapsed_ms;
        self.avg_elapsed_ms = self.total_elapsed_ms / self.executions as f64;
        self.cpu_time_ms += cpu_ms;
        self.rows_returned += rows;
        self.disk_reads += reads;
        self.buffer_gets += gets;
        self.last_execution = SystemTime::now();
    }
}

// Connection pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub idle_connections: usize,
    pub waiting_requests: usize,
    pub max_connections: usize,
    pub total_created: u64,
    pub total_closed: u64,
    pub connection_timeouts: u64,
    pub avg_wait_time_ms: f64,
    pub max_wait_time_ms: f64,
}

impl ConnectionPoolStats {
    pub fn new(max_connections: usize) -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            idle_connections: 0,
            waiting_requests: 0,
            max_connections,
            total_created: 0,
            total_closed: 0,
            connection_timeouts: 0,
            avg_wait_time_ms: 0.0,
            max_wait_time_ms: 0.0,
        }
    }

    pub fn utilization_percent(&self) -> f64 {
        if self.max_connections == 0 {
            0.0
        } else {
            (self.active_connections as f64 / self.max_connections as f64) * 100.0
        }
    }

    pub fn is_saturated(&self) -> bool {
        self.active_connections >= self.max_connections
    }
}

// Replication lag information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationLag {
    pub replica_id: String,
    pub replica_name: String,
    pub lag_seconds: f64,
    pub lag_bytes: u64,
    pub last_received_lsn: u64,
    pub last_applied_lsn: u64,
    pub is_healthy: bool,
    pub last_update: SystemTime,
}

impl ReplicationLag {
    pub fn new(replica_id: impl Into<String>, replica_name: impl Into<String>) -> Self {
        Self {
            replica_id: replica_id.into(),
            replica_name: replica_name.into(),
            lag_seconds: 0.0,
            lag_bytes: 0,
            last_received_lsn: 0,
            last_applied_lsn: 0,
            is_healthy: true,
            last_update: SystemTime::now(),
        }
    }

    pub fn update_lag(
        &mut self,
        lag_seconds: f64,
        lag_bytes: u64,
        received_lsn: u64,
        applied_lsn: u64,
    ) {
        self.lag_seconds = lag_seconds;
        self.lag_bytes = lag_bytes;
        self.last_received_lsn = received_lsn;
        self.last_applied_lsn = applied_lsn;
        self.is_healthy = lag_seconds < 10.0; // Consider unhealthy if lag > 10 seconds
        self.last_update = SystemTime::now();
    }
}

// System resource snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    pub timestamp: SystemTime,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_available_bytes: u64,
    pub disk_read_mb_per_sec: f64,
    pub disk_write_mb_per_sec: f64,
    pub network_in_mb_per_sec: f64,
    pub network_out_mb_per_sec: f64,
    pub active_queries: usize,
    pub queued_queries: usize,
}

impl ResourceSnapshot {
    pub fn new() -> Self {
        Self {
            timestamp: SystemTime::now(),
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            memory_used_bytes: 0,
            memory_available_bytes: 0,
            disk_read_mb_per_sec: 0.0,
            disk_write_mb_per_sec: 0.0,
            network_in_mb_per_sec: 0.0,
            network_out_mb_per_sec: 0.0,
            active_queries: 0,
            queued_queries: 0,
        }
    }
}

impl Default for ResourceSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

// Database performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub timestamp: SystemTime,
    pub queries_per_second: f64,
    pub transactions_per_second: f64,
    pub avg_query_time_ms: f64,
    pub cache_hit_ratio: f64,
    pub active_sessions: usize,
    pub blocked_sessions: usize,
    pub deadlocks: u64,
    pub errors: u64,
}

impl PerformanceSummary {
    pub fn new() -> Self {
        Self {
            timestamp: SystemTime::now(),
            queries_per_second: 0.0,
            transactions_per_second: 0.0,
            avg_query_time_ms: 0.0,
            cache_hit_ratio: 0.0,
            active_sessions: 0,
            blocked_sessions: 0,
            deadlocks: 0,
            errors: 0,
        }
    }
}

impl Default for PerformanceSummary {
    fn default() -> Self {
        Self::new()
    }
}

// Dashboard data aggregator
pub struct DashboardDataAggregator {
    time_series: Arc<RwLock<HashMap<String, TimeSeriesMetric>>>,
    top_queries_by_time: Arc<RwLock<Vec<TopQuery>>>,
    top_queries_by_executions: Arc<RwLock<Vec<TopQuery>>>,
    connection_stats: Arc<RwLock<ConnectionPoolStats>>,
    replication_lags: Arc<RwLock<HashMap<String, ReplicationLag>>>,
    resource_history: Arc<RwLock<VecDeque<ResourceSnapshot>>>,
    performance_history: Arc<RwLock<VecDeque<PerformanceSummary>>>,
    max_history: usize,
    max_top_queries: usize,
}

impl DashboardDataAggregator {
    pub fn new(max_history: usize, max_top_queries: usize) -> Self {
        // SAFETY: Clamp to prevent OOM (Issue C-06)
        let safe_max_history = max_history.min(MAX_TIME_SERIES_POINTS);

        Self {
            time_series: Arc::new(RwLock::new(HashMap::new())),
            top_queries_by_time: Arc::new(RwLock::new(Vec::new())),
            top_queries_by_executions: Arc::new(RwLock::new(Vec::new())),
            connection_stats: Arc::new(RwLock::new(ConnectionPoolStats::new(100))),
            replication_lags: Arc::new(RwLock::new(HashMap::new())),
            resource_history: Arc::new(RwLock::new(VecDeque::with_capacity(safe_max_history))),
            performance_history: Arc::new(RwLock::new(VecDeque::with_capacity(safe_max_history))),
            max_history: safe_max_history,
            max_top_queries,
        }
    }

    // Time series operations
    pub fn register_time_series(&self, name: impl Into<String>, unit: impl Into<String>) {
        let name = name.into();
        let metric = TimeSeriesMetric::new(name.clone(), unit, self.max_history);
        self.time_series.write().insert(name, metric);
    }

    pub fn record_metric(&self, name: &str, value: f64) {
        if let Some(metric) = self.time_series.write().get_mut(name) {
            metric.add_point(value);
        }
    }

    pub fn get_time_series(&self, name: &str) -> Option<TimeSeriesMetric> {
        self.time_series.read().get(name).cloned()
    }

    pub fn get_all_time_series(&self) -> HashMap<String, TimeSeriesMetric> {
        self.time_series.read().clone()
    }

    // Top queries operations
    pub fn update_query_stats(
        &self,
        sql_id: u64,
        sql_text: impl Into<String>,
        elapsed_ms: f64,
        cpu_ms: f64,
        rows: u64,
        reads: u64,
        gets: u64,
    ) {
        let sql_text = sql_text.into();

        // Update top queries by time
        let mut by_time = self.top_queries_by_time.write();
        if let Some(query) = by_time.iter_mut().find(|q| q.sql_id == sql_id) {
            query.update_stats(elapsed_ms, cpu_ms, rows, reads, gets);
        } else {
            let mut query = TopQuery::new(sql_id, sql_text.clone());
            query.update_stats(elapsed_ms, cpu_ms, rows, reads, gets);
            by_time.push(query);
        }
        by_time.sort_by(|a, b| b.total_elapsed_ms.partial_cmp(&a.total_elapsed_ms).unwrap());
        by_time.truncate(self.max_top_queries);
        drop(by_time);

        // Update top queries by executions
        let mut by_exec = self.top_queries_by_executions.write();
        if let Some(query) = by_exec.iter_mut().find(|q| q.sql_id == sql_id) {
            query.update_stats(elapsed_ms, cpu_ms, rows, reads, gets);
        } else {
            let mut query = TopQuery::new(sql_id, sql_text);
            query.update_stats(elapsed_ms, cpu_ms, rows, reads, gets);
            by_exec.push(query);
        }
        by_exec.sort_by(|a, b| b.executions.cmp(&a.executions));
        by_exec.truncate(self.max_top_queries);
    }

    pub fn get_top_queries_by_time(&self, limit: usize) -> Vec<TopQuery> {
        self.top_queries_by_time
            .read()
            .iter()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn get_top_queries_by_executions(&self, limit: usize) -> Vec<TopQuery> {
        self.top_queries_by_executions
            .read()
            .iter()
            .take(limit)
            .cloned()
            .collect()
    }

    // Connection pool operations
    pub fn update_connection_stats<F>(&self, updater: F)
    where
        F: FnOnce(&mut ConnectionPoolStats),
    {
        let mut stats = self.connection_stats.write();
        updater(&mut *stats);
    }

    pub fn get_connection_stats(&self) -> ConnectionPoolStats {
        self.connection_stats.read().clone()
    }

    // Replication operations
    pub fn register_replica(&self, replica: ReplicationLag) {
        self.replication_lags
            .write()
            .insert(replica.replica_id.clone(), replica);
    }

    pub fn update_replica_lag(
        &self,
        replica_id: &str,
        lag_seconds: f64,
        lag_bytes: u64,
        received_lsn: u64,
        applied_lsn: u64,
    ) {
        if let Some(replica) = self.replication_lags.write().get_mut(replica_id) {
            replica.update_lag(lag_seconds, lag_bytes, received_lsn, applied_lsn);
        }
    }

    pub fn get_replication_lag(&self, replica_id: &str) -> Option<ReplicationLag> {
        self.replication_lags.read().get(replica_id).cloned()
    }

    pub fn get_all_replication_lags(&self) -> Vec<ReplicationLag> {
        self.replication_lags.read().values().cloned().collect()
    }

    pub fn get_unhealthy_replicas(&self) -> Vec<ReplicationLag> {
        self.replication_lags
            .read()
            .values()
            .filter(|r| !r.is_healthy)
            .cloned()
            .collect()
    }

    // Resource snapshot operations
    pub fn record_resource_snapshot(&self, snapshot: ResourceSnapshot) {
        let mut history = self.resource_history.write();
        if history.len() >= self.max_history {
            history.pop_front();
        }
        history.push_back(snapshot);
    }

    pub fn get_latest_resource_snapshot(&self) -> Option<ResourceSnapshot> {
        self.resource_history.read().back().cloned()
    }

    pub fn get_resource_history(&self, limit: usize) -> Vec<ResourceSnapshot> {
        self.resource_history
            .read()
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    // Performance summary operations
    pub fn record_performance_summary(&self, summary: PerformanceSummary) {
        let mut history = self.performance_history.write();
        if history.len() >= self.max_history {
            history.pop_front();
        }
        history.push_back(summary);
    }

    pub fn get_latest_performance_summary(&self) -> Option<PerformanceSummary> {
        self.performance_history.read().back().cloned()
    }

    pub fn get_performance_history(&self, limit: usize) -> Vec<PerformanceSummary> {
        self.performance_history
            .read()
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    // Dashboard snapshot
    pub fn get_dashboard_snapshot(&self) -> DashboardSnapshot {
        DashboardSnapshot {
            timestamp: SystemTime::now(),
            resource_snapshot: self.get_latest_resource_snapshot(),
            performance_summary: self.get_latest_performance_summary(),
            connection_stats: self.get_connection_stats(),
            top_queries_by_time: self.get_top_queries_by_time(10),
            top_queries_by_executions: self.get_top_queries_by_executions(10),
            replication_lags: self.get_all_replication_lags(),
            time_series_latest: self.get_latest_time_series_values(),
        }
    }

    fn get_latest_time_series_values(&self) -> HashMap<String, f64> {
        self.time_series
            .read()
            .iter()
            .filter_map(|(name, metric)| metric.get_latest().map(|v| (name.clone(), v)))
            .collect()
    }
}

impl Default for DashboardDataAggregator {
    fn default() -> Self {
        Self::new(1000, 50)
    }
}

// Complete dashboard snapshot for WebSocket streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    pub timestamp: SystemTime,
    pub resource_snapshot: Option<ResourceSnapshot>,
    pub performance_summary: Option<PerformanceSummary>,
    pub connection_stats: ConnectionPoolStats,
    pub top_queries_by_time: Vec<TopQuery>,
    pub top_queries_by_executions: Vec<TopQuery>,
    pub replication_lags: Vec<ReplicationLag>,
    pub time_series_latest: HashMap<String, f64>,
}

// WebSocket message types for dashboard updates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DashboardMessage {
    FullSnapshot {
        data: DashboardSnapshot,
    },
    MetricUpdate {
        name: String,
        value: f64,
        timestamp: SystemTime,
    },
    QueryUpdate {
        query: TopQuery,
    },
    ConnectionUpdate {
        stats: ConnectionPoolStats,
    },
    ReplicationUpdate {
        lags: Vec<ReplicationLag>,
    },
    ResourceUpdate {
        snapshot: ResourceSnapshot,
    },
    PerformanceUpdate {
        summary: PerformanceSummary,
    },
}

// Dashboard update streamer for WebSocket connections
pub struct DashboardStreamer {
    aggregator: Arc<DashboardDataAggregator>,
    update_interval: Duration,
    last_update: Arc<RwLock<Instant>>,
}

impl DashboardStreamer {
    pub fn new(aggregator: Arc<DashboardDataAggregator>, update_interval: Duration) -> Self {
        Self {
            aggregator,
            update_interval,
            last_update: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub fn should_update(&self) -> bool {
        self.last_update.read().elapsed() >= self.update_interval
    }

    pub fn get_update(&self) -> Option<DashboardMessage> {
        if !self.should_update() {
            return None;
        }

        *self.last_update.write() = Instant::now();

        Some(DashboardMessage::FullSnapshot {
            data: self.aggregator.get_dashboard_snapshot(),
        })
    }

    pub fn get_full_snapshot(&self) -> DashboardMessage {
        DashboardMessage::FullSnapshot {
            data: self.aggregator.get_dashboard_snapshot(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_series_metric() {
        let mut metric = TimeSeriesMetric::new("cpu_usage", "percent", 10);

        metric.add_point(50.0);
        metric.add_point(60.0);
        metric.add_point(55.0);

        assert_eq!(metric.get_latest(), Some(55.0));
        assert_eq!(metric.get_average(), Some(55.0));
        assert_eq!(metric.get_min(), Some(50.0));
        assert_eq!(metric.get_max(), Some(60.0));
    }

    #[test]
    fn test_top_query() {
        let mut query = TopQuery::new(1, "SELECT * FROM users");

        query.update_stats(100.0, 50.0, 10, 5, 20);
        query.update_stats(150.0, 75.0, 15, 8, 30);

        assert_eq!(query.executions, 2);
        assert_eq!(query.total_elapsed_ms, 250.0);
        assert_eq!(query.avg_elapsed_ms, 125.0);
    }

    #[test]
    fn test_connection_pool_stats() {
        let stats = ConnectionPoolStats::new(100);
        assert_eq!(stats.utilization_percent(), 0.0);
        assert!(!stats.is_saturated());
    }

    #[test]
    fn test_replication_lag() {
        let mut lag = ReplicationLag::new("replica1", "Replica 1");
        lag.update_lag(5.0, 1024, 1000, 950);

        assert_eq!(lag.lag_seconds, 5.0);
        assert!(lag.is_healthy);

        lag.update_lag(15.0, 2048, 2000, 1500);
        assert!(!lag.is_healthy);
    }

    #[test]
    fn test_dashboard_aggregator() {
        let aggregator = DashboardDataAggregator::new(100, 10);

        aggregator.register_time_series("qps", "queries/sec");
        aggregator.record_metric("qps", 100.0);
        aggregator.record_metric("qps", 120.0);

        let metric = aggregator.get_time_series("qps").unwrap();
        assert_eq!(metric.data_points.len(), 2);

        aggregator.update_query_stats(1, "SELECT * FROM users", 50.0, 25.0, 10, 5, 20);

        let top_queries = aggregator.get_top_queries_by_time(10);
        assert_eq!(top_queries.len(), 1);
    }

    #[test]
    fn test_dashboard_snapshot() {
        let aggregator = DashboardDataAggregator::new(100, 10);

        let snapshot = aggregator.get_dashboard_snapshot();
        assert!(snapshot.resource_snapshot.is_none());
        assert!(snapshot.performance_summary.is_none());
    }
}
