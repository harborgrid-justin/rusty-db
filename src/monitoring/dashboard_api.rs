// Dashboard Data API
// Real-time metrics endpoints, historical data aggregation, custom metric queries,
// dashboard widget data providers

use crate::error::{DbError, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use super::metrics::{MetricRegistry, Metric};
use super::dashboard::{TimeSeriesMetric, TopQuery, ConnectionPoolStats, ReplicationLag};

// SAFETY: Maximum query results to prevent OOM
const MAX_QUERY_RESULTS: usize = 10_000;
const MAX_WIDGET_CONFIGS: usize = 1_000;

// Time range for historical queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: SystemTime,
    pub end: SystemTime,
}

impl TimeRange {
    pub fn new(start: SystemTime, end: SystemTime) -> Self {
        Self { start, end }
    }

    pub fn last_minutes(minutes: u64) -> Self {
        let end = SystemTime::now();
        let start = end - Duration::from_secs(minutes * 60);
        Self { start, end }
    }

    pub fn last_hours(hours: u64) -> Self {
        Self::last_minutes(hours * 60)
    }

    pub fn last_days(days: u64) -> Self {
        Self::last_hours(days * 24)
    }

    pub fn duration(&self) -> Duration {
        self.end.duration_since(self.start).unwrap_or(Duration::ZERO)
    }
}

// Aggregation function types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AggregationFunction {
    Avg,
    Sum,
    Min,
    Max,
    Count,
    P50,   // 50th percentile (median)
    P95,   // 95th percentile
    P99,   // 99th percentile
    Rate,  // Rate per second
}

// Metric query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricQuery {
    pub metric_name: String,
    pub time_range: TimeRange,
    pub aggregation: AggregationFunction,
    pub interval: Option<Duration>, // Bucket interval for aggregation
    pub filters: HashMap<String, String>, // Label filters
}

impl MetricQuery {
    pub fn new(metric_name: impl Into<String>, time_range: TimeRange) -> Self {
        Self {
            metric_name: metric_name.into(),
            time_range,
            aggregation: AggregationFunction::Avg,
            interval: None,
            filters: HashMap::new(),
        }
    }

    pub fn with_aggregation(mut self, aggregation: AggregationFunction) -> Self {
        self.aggregation = aggregation;
        self
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval);
        self
    }

    pub fn with_filter(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.filters.insert(key.into(), value.into());
        self
    }
}

// Query result data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryDataPoint {
    pub timestamp: SystemTime,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

impl QueryDataPoint {
    pub fn new(timestamp: SystemTime, value: f64) -> Self {
        Self {
            timestamp,
            value,
            labels: HashMap::new(),
        }
    }

    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels = labels;
        self
    }
}

// Metric query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricQueryResult {
    pub metric_name: String,
    pub time_range: TimeRange,
    pub aggregation: AggregationFunction,
    pub data_points: Vec<QueryDataPoint>,
    pub total_points: usize,
}

impl MetricQueryResult {
    pub fn new(
        metric_name: impl Into<String>,
        time_range: TimeRange,
        aggregation: AggregationFunction,
    ) -> Self {
        Self {
            metric_name: metric_name.into(),
            time_range,
            aggregation,
            data_points: Vec::new(),
            total_points: 0,
        }
    }

    pub fn add_point(&mut self, point: QueryDataPoint) {
        if self.data_points.len() < MAX_QUERY_RESULTS {
            self.data_points.push(point);
        }
        self.total_points += 1;
    }

    pub fn is_truncated(&self) -> bool {
        self.total_points > self.data_points.len()
    }
}

// Dashboard widget types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WidgetType {
    LineChart,
    BarChart,
    Gauge,
    Counter,
    Table,
    Heatmap,
    Pie,
}

// Widget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub id: String,
    pub title: String,
    pub widget_type: WidgetType,
    pub queries: Vec<MetricQuery>,
    pub refresh_interval: Duration,
    pub display_options: HashMap<String, String>,
}

impl WidgetConfig {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        widget_type: WidgetType,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            widget_type,
            queries: Vec::new(),
            refresh_interval: Duration::from_secs(5),
            display_options: HashMap::new(),
        }
    }

    pub fn with_query(mut self, query: MetricQuery) -> Self {
        self.queries.push(query);
        self
    }

    pub fn with_refresh_interval(mut self, interval: Duration) -> Self {
        self.refresh_interval = interval;
        self
    }
}

// Widget data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetData {
    pub widget_id: String,
    pub results: Vec<MetricQueryResult>,
    pub last_updated: SystemTime,
}

impl WidgetData {
    pub fn new(widget_id: impl Into<String>) -> Self {
        Self {
            widget_id: widget_id.into(),
            results: Vec::new(),
            last_updated: SystemTime::now(),
        }
    }

    pub fn add_result(&mut self, result: MetricQueryResult) {
        self.results.push(result);
    }
}

// Real-time dashboard snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeDashboardSnapshot {
    pub timestamp: SystemTime,
    pub qps: f64,
    pub active_connections: usize,
    pub active_transactions: usize,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub cache_hit_ratio: f64,
    pub replication_lag_ms: f64,
    pub alert_count: usize,
}

impl Default for RealtimeDashboardSnapshot {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now(),
            qps: 0.0,
            active_connections: 0,
            active_transactions: 0,
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            cache_hit_ratio: 0.0,
            replication_lag_ms: 0.0,
            alert_count: 0,
        }
    }
}

// Dashboard API service
pub struct DashboardApi {
    metric_registry: Arc<MetricRegistry>,
    widgets: Arc<RwLock<HashMap<String, WidgetConfig>>>,
    widget_cache: Arc<RwLock<HashMap<String, WidgetData>>>,
    time_series_cache: Arc<RwLock<HashMap<String, TimeSeriesMetric>>>,
}

impl DashboardApi {
    pub fn new(metric_registry: Arc<MetricRegistry>) -> Self {
        Self {
            metric_registry,
            widgets: Arc::new(RwLock::new(HashMap::new())),
            widget_cache: Arc::new(RwLock::new(HashMap::new())),
            time_series_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Register a new dashboard widget
    pub fn register_widget(&self, config: WidgetConfig) -> Result<()> {
        let mut widgets = self.widgets.write();

        if widgets.len() >= MAX_WIDGET_CONFIGS {
            return Err(DbError::LimitExceeded(
                "Maximum number of dashboard widgets reached".to_string(),
            ));
        }

        widgets.insert(config.id.clone(), config);
        Ok(())
    }

    // Unregister a dashboard widget
    pub fn unregister_widget(&self, widget_id: &str) -> Result<()> {
        self.widgets.write().remove(widget_id);
        self.widget_cache.write().remove(widget_id);
        Ok(())
    }

    // Get widget configuration
    pub fn get_widget_config(&self, widget_id: &str) -> Option<WidgetConfig> {
        self.widgets.read().get(widget_id).cloned()
    }

    // List all widget configurations
    pub fn list_widgets(&self) -> Vec<WidgetConfig> {
        self.widgets.read().values().cloned().collect()
    }

    // Execute a metric query
    pub fn query_metrics(&self, query: &MetricQuery) -> Result<MetricQueryResult> {
        let mut result = MetricQueryResult::new(
            query.metric_name.clone(),
            query.time_range.clone(),
            query.aggregation,
        );

        // Get metric from registry
        if let Some(metric) = self.metric_registry.get_metric(&query.metric_name) {
            // Convert metric to data points based on aggregation
            let value = match metric {
                Metric::Counter(c) => c.get(),
                Metric::Gauge(g) => g.get(),
                Metric::Histogram(h) => {
                    match query.aggregation {
                        AggregationFunction::Avg => {
                            let count = h.get_count();
                            if count > 0 {
                                h.get_sum() / count as f64
                            } else {
                                0.0
                            }
                        }
                        AggregationFunction::Sum => h.get_sum(),
                        AggregationFunction::Count => h.get_count() as f64,
                        // Histograms don't track min/max/percentiles directly
                        // These would need bucket analysis or alternative storage
                        _ => {
                            let count = h.get_count();
                            if count > 0 {
                                h.get_sum() / count as f64
                            } else {
                                0.0
                            }
                        }
                    }
                }
                Metric::Summary(s) => {
                    match query.aggregation {
                        AggregationFunction::Sum => s.get_sum(),
                        AggregationFunction::Count => s.get_count() as f64,
                        AggregationFunction::Avg => {
                            let count = s.get_count();
                            if count > 0 {
                                s.get_sum() / count as f64
                            } else {
                                0.0
                            }
                        }
                        AggregationFunction::P50 => s.get_quantile(0.5).unwrap_or(0.0),
                        AggregationFunction::P95 => s.get_quantile(0.95).unwrap_or(0.0),
                        AggregationFunction::P99 => s.get_quantile(0.99).unwrap_or(0.0),
                        // Summary doesn't track min/max directly
                        _ => {
                            let count = s.get_count();
                            if count > 0 {
                                s.get_sum() / count as f64
                            } else {
                                0.0
                            }
                        }
                    }
                }
            };

            result.add_point(QueryDataPoint::new(SystemTime::now(), value));
        }

        Ok(result)
    }

    // Get widget data (executes all queries for the widget)
    pub fn get_widget_data(&self, widget_id: &str) -> Result<WidgetData> {
        let config = self.get_widget_config(widget_id)
            .ok_or_else(|| DbError::NotFound(format!("Widget not found: {}", widget_id)))?;

        let mut widget_data = WidgetData::new(widget_id);

        for query in &config.queries {
            let result = self.query_metrics(query)?;
            widget_data.add_result(result);
        }

        // Update cache
        self.widget_cache.write().insert(widget_id.to_string(), widget_data.clone());

        Ok(widget_data)
    }

    // Get realtime snapshot for quick dashboard overview
    pub fn get_realtime_snapshot(&self) -> RealtimeDashboardSnapshot {
        let mut snapshot = RealtimeDashboardSnapshot::default();

        // Query various metrics
        if let Some(metric) = self.metric_registry.get_metric("queries_per_second") {
            if let Metric::Gauge(g) = metric {
                snapshot.qps = g.get();
            }
        }

        if let Some(metric) = self.metric_registry.get_metric("active_connections") {
            if let Metric::Gauge(g) = metric {
                snapshot.active_connections = g.get() as usize;
            }
        }

        if let Some(metric) = self.metric_registry.get_metric("active_transactions") {
            if let Metric::Gauge(g) = metric {
                snapshot.active_transactions = g.get() as usize;
            }
        }

        if let Some(metric) = self.metric_registry.get_metric("cpu_usage_percent") {
            if let Metric::Gauge(g) = metric {
                snapshot.cpu_usage_percent = g.get();
            }
        }

        if let Some(metric) = self.metric_registry.get_metric("memory_usage_percent") {
            if let Metric::Gauge(g) = metric {
                snapshot.memory_usage_percent = g.get();
            }
        }

        if let Some(metric) = self.metric_registry.get_metric("cache_hit_ratio") {
            if let Metric::Gauge(g) = metric {
                snapshot.cache_hit_ratio = g.get();
            }
        }

        snapshot
    }

    // Register time series for tracking
    pub fn register_time_series(&self, name: impl Into<String>, unit: impl Into<String>) {
        let name = name.into();
        let mut cache = self.time_series_cache.write();

        if !cache.contains_key(&name) {
            cache.insert(
                name.clone(),
                TimeSeriesMetric::new(name, unit, 3600), // 1 hour default
            );
        }
    }

    // Update time series data point
    pub fn update_time_series(&self, name: &str, value: f64) {
        if let Some(series) = self.time_series_cache.write().get_mut(name) {
            series.add_point(value);
        }
    }

    // Get time series data
    pub fn get_time_series(&self, name: &str) -> Option<TimeSeriesMetric> {
        self.time_series_cache.read().get(name).cloned()
    }

    // Get multiple time series
    pub fn get_time_series_batch(&self, names: &[String]) -> HashMap<String, TimeSeriesMetric> {
        let cache = self.time_series_cache.read();
        names
            .iter()
            .filter_map(|name| {
                cache.get(name).map(|series| (name.clone(), series.clone()))
            })
            .collect()
    }

    // Clear widget cache
    pub fn clear_cache(&self) {
        self.widget_cache.write().clear();
    }

    // Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            widgets_registered: self.widgets.read().len(),
            widgets_cached: self.widget_cache.read().len(),
            time_series_tracked: self.time_series_cache.read().len(),
        }
    }
}

impl Default for DashboardApi {
    fn default() -> Self {
        Self::new(Arc::new(MetricRegistry::default()))
    }
}

// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub widgets_registered: usize,
    pub widgets_cached: usize,
    pub time_series_tracked: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_range() {
        let range = TimeRange::last_minutes(60);
        assert!(range.duration().as_secs() >= 3600);
    }

    #[test]
    fn test_metric_query() {
        let query = MetricQuery::new("test_metric", TimeRange::last_minutes(5))
            .with_aggregation(AggregationFunction::Avg)
            .with_interval(Duration::from_secs(60))
            .with_filter("host", "server1");

        assert_eq!(query.metric_name, "test_metric");
        assert_eq!(query.aggregation, AggregationFunction::Avg);
        assert!(query.interval.is_some());
        assert_eq!(query.filters.get("host").unwrap(), "server1");
    }

    #[test]
    fn test_dashboard_api() {
        let api = DashboardApi::default();

        // Register a widget
        let config = WidgetConfig::new("widget1", "CPU Usage", WidgetType::LineChart)
            .with_refresh_interval(Duration::from_secs(10));

        assert!(api.register_widget(config).is_ok());

        // Get widget config
        let retrieved = api.get_widget_config("widget1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "CPU Usage");

        // List widgets
        let widgets = api.list_widgets();
        assert_eq!(widgets.len(), 1);

        // Get cache stats
        let stats = api.get_cache_stats();
        assert_eq!(stats.widgets_registered, 1);
    }

    #[test]
    fn test_time_series() {
        let api = DashboardApi::default();

        api.register_time_series("cpu_usage", "%");
        api.update_time_series("cpu_usage", 45.5);
        api.update_time_series("cpu_usage", 50.2);

        let series = api.get_time_series("cpu_usage");
        assert!(series.is_some());

        let series = series.unwrap();
        assert_eq!(series.name, "cpu_usage");
        assert_eq!(series.data_points.len(), 2);
    }

    #[test]
    fn test_realtime_snapshot() {
        let api = DashboardApi::default();
        let snapshot = api.get_realtime_snapshot();

        // Should have default values
        assert_eq!(snapshot.qps, 0.0);
        assert_eq!(snapshot.active_connections, 0);
    }
}
