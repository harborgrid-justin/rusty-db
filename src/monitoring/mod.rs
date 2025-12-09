// RustyDB Monitoring and Observability Module
// Enterprise-grade monitoring with ASH, profiling, resource management, and real-time dashboards

use std::time::SystemTime;
pub mod metrics;
pub mod profiler;
pub mod ash;
pub mod resource_manager;
pub mod alerts;
pub mod statistics;
pub mod diagnostics;
pub mod dashboard;

// Re-export commonly used types
pub use metrics::{
    Counter, Gauge, Histogram, Summary, Metric, MetricType, MetricRegistry, MetricAggregator,
    HistogramBucket, Quantile,
};

pub use profiler::{
    QueryProfiler, QueryProfile, ProfileBuilder, PlanOperator, OperatorType,
    WaitEvent, WaitEventType,
};

pub use ash::{
    ActiveSessionHistory, AshSample, AshReportGenerator, SessionState, WaitClass,
    SqlStatistics, SessionStatistics,
};

pub use resource_manager::{
    ResourceManager, ResourceGroup, ResourceLimit, ResourceType, QueryResourceUsage,
    ResourceLimitStatus, EnforcementPolicy, ResourcePlanner, ResourceGroupStatistics,
};

pub use alerts::{
    AlertManager, Alert, AlertSeverity, AlertState, AlertCategory,
    ThresholdRule, AnomalyRule, ComparisonOperator, AnomalyDetectionAlgorithm,
};

pub use statistics::{
    StatisticsCollector, VSession, VSql, VSysstat, VSystemEvent, VSesstat,
    VLock, VTransaction, VSqlarea, VBgprocess, VParameter, VDatabase,
};

pub use diagnostics::{
    DiagnosticRepository, Incident, DiagnosticDump, HealthCheck, HealthCheckResult,
    IncidentType, IncidentSeverity, DumpType, HealthStatus,
    ConnectionHealthCheck, MemoryHealthCheck,
};

pub use dashboard::{
    DashboardDataAggregator, DashboardSnapshot, DashboardStreamer, DashboardMessage,
    TimeSeriesMetric, TopQuery, ConnectionPoolStats, ReplicationLag,
    ResourceSnapshot, PerformanceSummary, MetricDataPoint,
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration};

/// Query statistics and performance metrics (legacy compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStats {
    pub query_id: u64,
    pub sql: String,
    pub execution_time_ms: u64,
    pub rows_affected: usize,
    pub bytes_read: usize,
    pub bytes_written: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub timestamp: SystemTime,
}

/// System metrics (legacy compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub active_connections: usize,
    pub total_queries: u64,
    pub queries_per_second: f64,
    pub buffer_pool_hit_rate: f64,
    pub active_transactions: usize,
    pub locks_held: usize,
    pub disk_reads: u64,
    pub disk_writes: u64,
}

/// Slow query record (legacy compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowQuery {
    pub query: String,
    pub execution_time_ms: u64,
    pub timestamp: SystemTime,
}

/// Legacy monitoring system for backward compatibility
pub struct MonitoringSystem {
    query_stats: Arc<RwLock<Vec<QueryStats>>>,
    slow_queries: Arc<RwLock<Vec<SlowQuery>>>,
    slow_query_threshold_ms: u64,
    metrics: Arc<RwLock<SystemMetrics>>,
}

impl MonitoringSystem {
    pub fn new() -> Self {
        Self {
            query_stats: Arc::new(RwLock::new(Vec::new())),
            slow_queries: Arc::new(RwLock::new(Vec::new())),
            slow_query_threshold_ms: 1000, // 1 second
            metrics: Arc::new(RwLock::new(SystemMetrics {
                active_connections: 0,
                total_queries: 0,
                queries_per_second: 0.0,
                buffer_pool_hit_rate: 0.0,
                active_transactions: 0,
                locks_held: 0,
                disk_reads: 0,
                disk_writes: 0,
            })),
        }
    }

    pub fn record_query(&self, stats: QueryStats) {
        if stats.execution_time_ms >= self.slow_query_threshold_ms {
            self.slow_queries.write().push(SlowQuery {
                query: stats.sql.clone(),
                execution_time_ms: stats.execution_time_ms,
                timestamp: stats.timestamp,
            });
        }

        self.query_stats.write().push(stats);
        self.metrics.write().total_queries += 1;
    }

    pub fn get_slow_queries(&self) -> Vec<SlowQuery> {
        self.slow_queries.read().clone()
    }

    pub fn get_metrics(&self) -> SystemMetrics {
        self.metrics.read().clone()
    }

    pub fn update_metrics<F>(&self, updater: F)
    where F: FnOnce(&mut SystemMetrics) {
        let mut metrics = self.metrics.write();
        updater(&mut *metrics);
    }

    pub fn get_query_stats(&self, limit: usize) -> Vec<QueryStats> {
        let stats = self.query_stats.read();
        stats.iter().rev().take(limit).cloned().collect()
    }
}

impl Default for MonitoringSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive monitoring hub that integrates all monitoring components
pub struct MonitoringHub {
    // Core components
    pub metrics_registry: Arc<MetricRegistry>,
    pub query_profiler: Arc<QueryProfiler>,
    pub ash: Arc<ActiveSessionHistory>,
    pub resource_manager: Arc<ResourceManager>,
    pub alert_manager: Arc<AlertManager>,
    pub statistics: Arc<StatisticsCollector>,
    pub diagnostics: Arc<DiagnosticRepository>,
    pub dashboard: Arc<DashboardDataAggregator>,

    // Legacy compatibility
    pub legacy_monitoring: Arc<MonitoringSystem>,
}

impl MonitoringHub {
    pub fn new(adr_base: impl Into<std::path::PathBuf>) -> Self {
        Self {
            metrics_registry: Arc::new(MetricRegistry::default()),
            query_profiler: Arc::new(QueryProfiler::default()),
            ash: Arc::new(ActiveSessionHistory::default()),
            resource_manager: Arc::new(ResourceManager::default()),
            alert_manager: Arc::new(AlertManager::default()),
            statistics: Arc::new(StatisticsCollector::default()),
            diagnostics: Arc::new(DiagnosticRepository::new(adr_base.into(), 10000)),
            dashboard: Arc::new(DashboardDataAggregator::default()),
            legacy_monitoring: Arc::new(MonitoringSystem::default()),
        }
    }

    /// Initialize default metrics
    pub fn initialize_default_metrics(&self) {
        // Register core metrics
        self.metrics_registry.register_counter("queries_total", "Total number of queries executed");
        self.metrics_registry.register_counter("queries_errors", "Total number of query errors");
        self.metrics_registry.register_gauge("active_connections", "Current active connections");
        self.metrics_registry.register_gauge("active_transactions", "Current active transactions");

        // Query latency histogram
        self.metrics_registry.register_histogram(
            "query_duration_ms",
            "Query execution duration in milliseconds",
            vec![1.0, 5.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0],
        );

        // Register time series for dashboard
        self.dashboard.register_time_series("queries_per_second", "qps");
        self.dashboard.register_time_series("cpu_usage_percent", "%");
        self.dashboard.register_time_series("memory_usage_percent", "%");
        self.dashboard.register_time_series("cache_hit_ratio", "%");
        self.dashboard.register_time_series("active_connections", "count");

        // Register default alert rules
        self.alert_manager.add_threshold_rule(
            ThresholdRule::new(
                "high_cpu",
                "cpu_usage_percent",
                80.0,
                ComparisonOperator::GreaterThan,
                AlertSeverity::Warning,
            )
            .with_category(AlertCategory::Performance)
        );

        self.alert_manager.add_threshold_rule(
            ThresholdRule::new(
                "high_memory",
                "memory_usage_percent",
                90.0,
                ComparisonOperator::GreaterThan,
                AlertSeverity::Error,
            )
            .with_category(AlertCategory::Capacity)
        );

        // Register default health checks
        let conn_check = Arc::new(ConnectionHealthCheck::new(
            100,
            Arc::new(RwLock::new(0)),
        ));
        self.diagnostics.register_health_check(conn_check);

        let mem_check = Arc::new(MemoryHealthCheck::new(
            1024 * 1024 * 1024, // 1GB
            Arc::new(RwLock::new(0)),
        ));
        self.diagnostics.register_health_check(mem_check);
    }

    /// Record a query execution for comprehensive monitoring
    pub fn record_query_execution(
        &self,
        query_id: u64,
        sql: String,
        session_id: u64,
        user_id: u64,
        execution_time: Duration,
        rows_returned: u64,
        bytes_read: u64,
        bytes_written: u64,
        cache_hits: u64,
        cache_misses: u64,
    ) {
        let execution_ms = execution_time.as_secs_f64() * 1000.0;

        // Update metrics
        if let Some(counter) = self.metrics_registry.get_metric("queries_total") {
            if let Metric::Counter(c) = counter {
                c.inc();
            }
        }

        if let Some(histogram) = self.metrics_registry.get_metric("query_duration_ms") {
            if let Metric::Histogram(h) = histogram {
                h.observe(execution_ms);
            }
        }

        // Record in profiler (simplified profile)
        let mut profile = QueryProfile::new(query_id, sql.clone());
        profile.rows_returned = rows_returned;
        profile.bytes_read = bytes_read;
        profile.bytes_written = bytes_written;
        profile.cache_hits = cache_hits;
        profile.cache_misses = cache_misses;
        profile.total_execution_time = execution_time;
        self.query_profiler.record_profile(profile);

        // Record ASH sample
        let sample = AshSample::new(0, session_id, user_id)
            .with_state(SessionState::Active)
            .with_sql(query_id, sql.clone(), 0)
            .with_timing(execution_time.as_micros() as u64, execution_time.as_micros() as u64);
        self.ash.record_sample(sample);

        // Update SQL statistics
        self.statistics.update_sql_stats(
            query_id,
            execution_time.as_micros() as u64,
            execution_time.as_micros() as u64,
            rows_returned,
        );

        // Update dashboard
        self.dashboard.update_query_stats(
            query_id,
            sql.clone(),
            execution_ms,
            execution_ms / 2.0, // Approximate CPU time
            rows_returned,
            bytes_read,
            cache_hits + cache_misses,
        );

        // Legacy monitoring
        let legacy_stats = QueryStats {
            query_id,
            sql,
            execution_time_ms: execution_ms as u64,
            rows_affected: rows_returned as usize,
            bytes_read: bytes_read as usize,
            bytes_written: bytes_written as usize,
            cache_hits: cache_hits as usize,
            cache_misses: cache_misses as usize,
            timestamp: SystemTime::now(),
        };
        self.legacy_monitoring.record_query(legacy_stats);
    }

    /// Get comprehensive system status
    pub fn get_system_status(&self) -> SystemStatus {
        SystemStatus {
            active_alerts: self.alert_manager.get_active_alert_count(),
            critical_incidents: self.diagnostics.get_critical_incidents().len(),
            health_status: self.diagnostics.get_overall_health(),
            active_sessions: self.statistics.get_active_sessions().len(),
            top_queries: self.dashboard.get_top_queries_by_time(5),
            replication_status: self.dashboard.get_unhealthy_replicas().len() == 0,
        }
    }
}

impl Default for MonitoringHub {
    fn default() -> Self {
        Self::new("./adr")
    }
}

/// System status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub active_alerts: usize,
    pub critical_incidents: usize,
    pub health_status: HealthStatus,
    pub active_sessions: usize,
    pub top_queries: Vec<TopQuery>,
    pub replication_status: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitoring_system() {
        let monitor = MonitoringSystem::new();
        let stats = QueryStats {
            query_id: 1,
            sql: "SELECT * FROM users".to_string(),
            execution_time_ms: 50,
            rows_affected: 10,
            bytes_read: 1024,
            bytes_written: 0,
            cache_hits: 5,
            cache_misses: 2,
            timestamp: SystemTime::now(),
        };

        monitor.record_query(stats);
        assert_eq!(monitor.get_metrics().total_queries, 1);
    }

    #[test]
    fn test_monitoring_hub() {
        let hub = MonitoringHub::new("./test_adr");
        hub.initialize_default_metrics();

        hub.record_query_execution(
            1,
            "SELECT * FROM users".to_string(),
            100,
            1,
            Duration::from_millis(50),
            10,
            1024,
            0,
            5,
            2,
        );

        let status = hub.get_system_status();
        assert_eq!(status.active_sessions, 0); // No sessions registered yet
    }
}


