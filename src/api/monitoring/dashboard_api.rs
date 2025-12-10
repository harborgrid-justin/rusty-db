// Monitoring Module
//
// Part of the comprehensive monitoring system for RustyDB

use std::sync::Arc;
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::api::{DashboardManager, HealthCheckCoordinator, MetricsRegistry, PrometheusExporter, TimeSeriesDatabase, TimeSeriesQuery, TimeSeriesResult, HealthCheckResult, HealthStatus, Alert, Dashboard};
use crate::api::monitoring::{MetricStream, AlertManager, RetentionPolicy, TimeSeriesPoint};
use crate::error::DbError;
use super::metrics_core::*;

// Export format for metrics data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    Prometheus,
}

// Metrics exporter
pub struct MetricsExporter {
    tsdb: Arc<TimeSeriesDatabase>,
}

impl MetricsExporter {
    pub fn new(tsdb: Arc<TimeSeriesDatabase>) -> Self {
        Self { tsdb }
    }

    pub fn export(&self, query: TimeSeriesQuery, format: ExportFormat) -> Result<String, DbError> {
        let result = self.tsdb.query(query);

        match format {
            ExportFormat::Json => self.export_json(&result),
            ExportFormat::Csv => self.export_csv(&result),
            ExportFormat::Prometheus => self.export_prometheus(&result),
        }
    }

    fn export_json(&self, result: &TimeSeriesResult) -> Result<String, DbError> {
        serde_json::to_string_pretty(result)
            .map_err(|e| DbError::Internal(format!("JSON serialization error: {}", e)))
    }

    fn export_csv(&self, result: &TimeSeriesResult) -> Result<String, DbError> {
        let mut csv = String::from("timestamp,value\n");

        for point in &result.points {
            let ts = point.timestamp.duration_since(UNIX_EPOCH)
                .map_err(|e| DbError::Internal(format!("Time error: {}", e)))?
                .as_secs();
            csv.push_str(&format!("{},{}\n", ts, point.value));
        }

        Ok(csv)
    }

    fn export_prometheus(&self, result: &TimeSeriesResult) -> Result<String, DbError> {
        let mut output = String::new();

        for point in &result.points {
            let labels: Vec<String> = point.labels.iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect();

            let label_str = if labels.is_empty() {
                String::new()
            } else {
                format!("{{{}}}", labels.join(","))
            };

            let ts = point.timestamp.duration_since(UNIX_EPOCH)
                .map_err(|e| DbError::Internal(format!("Time error: {}", e)))?
                .as_millis();

            output.push_str(&format!(
                "{}{} {} {}\n",
                result.metric_name,
                label_str,
                point.value,
                ts
            ));
        }

        Ok(output)
    }
}

// Top-level monitoring API
pub struct MonitoringApi {
    pub metrics_registry: Arc<MetricsRegistry>,
    pub prometheus_exporter: Arc<PrometheusExporter>,
    pub health_coordinator: Arc<HealthCheckCoordinator>,
    pub alert_manager: Arc<AlertManager>,
    pub dashboard_manager: Arc<DashboardManager>,
    pub tsdb: Arc<TimeSeriesDatabase>,
    pub metric_stream: Arc<MetricStream>,
    pub exporter: Arc<MetricsExporter>,
}

// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub retention_policy: RetentionPolicy,
    pub max_cardinality_per_metric: usize,
    pub enable_prometheus_push: bool,
    pub prometheus_push_interval: Duration,
    pub tsdb_max_points: usize,
    pub tsdb_retention: Duration,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            retention_policy: RetentionPolicy::default(),
            max_cardinality_per_metric: 10000,
            enable_prometheus_push: false,
            prometheus_push_interval: Duration::from_secs(60),
            tsdb_max_points: 100000,
            tsdb_retention: Duration::from_secs(7 * 24 * 3600), // 7 days
        }
    }
}

impl MonitoringApi {
    pub fn new(config: MonitoringConfig) -> Self {
        let metrics_registry = Arc::new(MetricsRegistry::new(config.retention_policy.clone()));
        let prometheus_exporter = Arc::new(PrometheusExporter::new(metrics_registry.clone()));
        let health_coordinator = Arc::new(HealthCheckCoordinator::new());
        let alert_manager = Arc::new(AlertManager::new());
        let dashboard_manager = Arc::new(DashboardManager::new());
        let tsdb = Arc::new(TimeSeriesDatabase::new(
            config.tsdb_max_points,
            config.tsdb_retention,
        ));
        let metric_stream = Arc::new(MetricStream::new());
        let exporter = Arc::new(MetricsExporter::new(tsdb.clone()));

        Self {
            metrics_registry,
            prometheus_exporter,
            health_coordinator,
            alert_manager,
            dashboard_manager,
            tsdb,
            metric_stream,
            exporter,
        }
    }

    // Metric recording convenience methods

    pub fn increment_counter(&self, name: &str, labels: &[(&str, &str)]) {
        let counter = self.metrics_registry.get_or_create_counter(
            name,
            labels,
            format!("{} counter", name),
        );
        counter.inc();
    }

    pub fn record_gauge(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        let gauge = self.metrics_registry.get_or_create_gauge(
            name,
            labels,
            format!("{} gauge", name),
        );
        gauge.set(value);
    }

    pub fn observe_histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        let histogram = self.metrics_registry.get_or_create_histogram(
            name,
            labels,
            format!("{} histogram", name),
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0],
        );
        histogram.observe(value);

        // Also record in time-series database
        self.tsdb.insert(name.to_string(), TimeSeriesPoint {
            timestamp: SystemTime::now(),
            value,
            labels: labels.iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        });

        // Publish to stream
        let label_map: BTreeMap<String, String> = labels.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        self.metric_stream.publish(name, value, &label_map);
    }

    pub fn start_timer(&self) -> Timer {
        Timer::new()
    }

    pub fn record_duration(&self, name: &str, timer: &Timer, labels: &[(&str, &str)]) {
        self.observe_histogram(name, timer.elapsed_seconds(), labels);
    }

    // Export methods

    pub fn export_prometheus_metrics(&self) -> String {
        self.prometheus_exporter.export_text()
    }

    pub fn query_time_series(&self, query: TimeSeriesQuery) -> TimeSeriesResult {
        self.tsdb.query(query)
    }

    pub fn export_metrics(&self, query: TimeSeriesQuery, format: ExportFormat) -> Result<String, DbError> {
        self.exporter.export(query, format)
    }

    // Health check methods

    pub fn check_health(&self) -> HealthCheckResult {
        let results = self.health_coordinator.check_all();
        let status = HealthStatus::worst(&results.iter().map(|r| r.status).collect::<Vec<_>>());

        HealthCheckResult {
            status,
            component: "system".to_string(),
            message: "Overall system health".to_string(),
            timestamp: SystemTime::now(),
            duration: Duration::from_secs(0),
            details: HashMap::new(),
        }
        .with_detail("component_results".to_string(), serde_json::to_value(&results).unwrap())
    }

    pub fn liveness(&self) -> HealthCheckResult {
        self.health_coordinator.liveness()
    }

    pub fn readiness(&self) -> HealthCheckResult {
        self.health_coordinator.readiness()
    }

    pub fn startup(&self) -> HealthCheckResult {
        self.health_coordinator.startup()
    }

    // Alert methods

    pub fn get_active_alerts(&self) -> Vec<Alert> {
        self.alert_manager.get_active_alerts()
    }

    pub fn get_alert_history(&self, limit: usize) -> Vec<Alert> {
        self.alert_manager.get_alert_history(limit)
    }

    // Dashboard methods

    pub fn create_dashboard(&self, dashboard: Dashboard) -> Result<(), DbError> {
        self.dashboard_manager.create_dashboard(dashboard)
    }

    pub fn get_dashboard(&self, id: &str) -> Option<Dashboard> {
        self.dashboard_manager.get_dashboard(id)
    }

    pub fn list_dashboards(&self) -> Vec<Dashboard> {
        self.dashboard_manager.list_dashboards()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::monitoring::{ComparisonOperator, LivenessProbe, ThresholdAlertRule};

    #[test]
    fn test_counter_metric() {
        let counter = CounterMetric::new("test counter");
        assert_eq!(counter.get(), 0);

        counter.inc();
        assert_eq!(counter.get(), 1);

        counter.inc_by(5);
        assert_eq!(counter.get(), 6);
    }

    #[test]
    fn test_gauge_metric() {
        let gauge = GaugeMetric::new("test gauge");
        assert_eq!(gauge.get(), 0.0);

        gauge.set(10.0);
        assert_eq!(gauge.get(), 10.0);

        gauge.add(5.0);
        assert_eq!(gauge.get(), 15.0);

        gauge.sub(3.0);
        assert_eq!(gauge.get(), 12.0);
    }

    #[test]
    fn test_histogram_metric() {
        let histogram = HistogramMetric::new(
            "test histogram",
            vec![1.0, 5.0, 10.0],
        );

        histogram.observe(0.5);
        histogram.observe(2.0);
        histogram.observe(7.0);
        histogram.observe(12.0);

        assert_eq!(histogram.get_count(), 4);
        assert_eq!(histogram.get_sum(), 21.5);

        let buckets = histogram.get_buckets();
        assert_eq!(buckets.len(), 4); // Including +Inf
    }

    #[test]
    fn test_monitoring_api() {
        let api = MonitoringApi::new(MonitoringConfig::default());

        api.increment_counter("test_counter", &[("env", "test")]);
        api.record_gauge("test_gauge", 42.0, &[]);
        api.observe_histogram("test_histogram", 1.5, &[]);

        let metrics = api.export_prometheus_metrics();
        assert!(!metrics.is_empty());
    }

    #[test]
    fn test_health_checks() {
        let coordinator = HealthCheckCoordinator::new();

        let liveness = Arc::new(LivenessProbe::new());
        liveness.mark_started();

        coordinator.add_checker(liveness.clone());

        let result = coordinator.check_all();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, HealthStatus::Healthy);
    }

    #[test]
    fn test_alert_threshold() {
        let rule = ThresholdAlertRule::new(
            "high_cpu".to_string(),
            "cpu_usage".to_string(),
            80.0,
            ComparisonOperator::GreaterThan,
            AlertSeverity::Warning,
        );

        // Below threshold
        assert!(rule.evaluate(75.0).is_none());

        // Above threshold but duration not met
        assert!(rule.evaluate(85.0).is_none());
    }
}
