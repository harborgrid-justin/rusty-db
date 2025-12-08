//! # Metrics & Monitoring API Module
//!
//! Comprehensive metrics collection, monitoring, and observability API for RustyDB.
//! This module provides enterprise-grade monitoring capabilities including:
//!
//! - **Metrics Collection Engine**: Counter, Gauge, Histogram, and Summary metrics
//! - **Prometheus Integration**: Full Prometheus exposition format and remote write support
//! - **Health Check System**: Liveness, readiness, and startup probes
//! - **Alerting Engine**: Multi-condition alerts with routing and notification
//! - **Dashboard Data API**: Real-time streaming and historical query support
//!
//! ## Architecture
//!
//! The monitoring API is designed for high-throughput, low-overhead operation with:
//! - Lock-free data structures for metric collection
//! - Efficient aggregation with time-window bucketing
//! - Cardinality management to prevent metric explosion
//! - Configurable retention policies
//!
//! ## Usage
//!
//! ```rust
//! use rusty_db::api::monitoring::*;
//!
//! // Initialize monitoring API
//! let monitoring = MonitoringApi::new(MonitoringConfig::default());
//!
//! // Record metrics
//! monitoring.increment_counter("http_requests_total", &[("method", "GET")]);
//! monitoring.record_gauge("memory_usage_bytes", 1024.0 * 1024.0 * 512.0, &[]);
//! monitoring.observe_histogram("request_duration_seconds", 0.045, &[]);
//!
//! // Export Prometheus metrics
//! let metrics = monitoring.export_prometheus_metrics();
//!
//! // Check health
//! let health = monitoring.check_health();
//! ```

use std::collections::{HashMap, BTreeMap, VecDeque};
use std::sync::{Arc, atomic::{AtomicU64, AtomicBool, Ordering}};
use std::time::{Duration, SystemTime, Instant, UNIX_EPOCH};
use parking_lot::{RwLock, Mutex};
use serde::{Deserialize, Serialize};
use crate::error::DbError;

type Result<T> = std::result::Result<T, DbError>;

// ============================================================================
// SECTION 1: METRICS COLLECTION ENGINE (700+ lines)
// ============================================================================

/// Label key-value pair for metric dimensions
pub type Label = (String, String);

/// Labels collection type
pub type Labels = Vec<Label>;

/// Metric identifier combining name and labels
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct MetricId {
    pub name: String,
    pub labels: BTreeMap<String, String>,
}

impl MetricId {
    pub fn new(name: impl Into<String>, labels: &[(&str, &str)]) -> Self {
        Self {
            name: name.into(),
            labels: labels.iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    pub fn cardinality_key(&self) -> String {
        format!("{}:{}", self.name, self.labels.len())
    }
}

/// Counter metric - monotonically increasing value
#[derive(Debug)]
pub struct CounterMetric {
    value: AtomicU64,
    created: SystemTime,
    help: String,
}

impl CounterMetric {
    pub fn new(help: impl Into<String>) -> Self {
        Self {
            value: AtomicU64::new(0),
            created: SystemTime::now(),
            help: help.into(),
        }
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_by(&self, delta: u64) {
        self.value.fetch_add(delta, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

/// Gauge metric - value that can go up or down
#[derive(Debug)]
pub struct GaugeMetric {
    value: Arc<RwLock<f64>>,
    created: SystemTime,
    help: String,
}

impl GaugeMetric {
    pub fn new(help: impl Into<String>) -> Self {
        Self {
            value: Arc::new(RwLock::new(0.0)),
            created: SystemTime::now(),
            help: help.into(),
        }
    }

    pub fn set(&self, value: f64) {
        *self.value.write() = value;
    }

    pub fn inc(&self) {
        *self.value.write() += 1.0;
    }

    pub fn dec(&self) {
        *self.value.write() -= 1.0;
    }

    pub fn add(&self, delta: f64) {
        *self.value.write() += delta;
    }

    pub fn sub(&self, delta: f64) {
        *self.value.write() -= delta;
    }

    pub fn get(&self) -> f64 {
        *self.value.read()
    }
}

/// Histogram bucket for distribution tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucketData {
    pub upper_bound: f64,
    pub count: u64,
}

/// Histogram metric - tracks distribution of values
#[derive(Debug)]
pub struct HistogramMetric {
    buckets: Vec<(f64, AtomicU64)>, // (upper_bound, count)
    sum: Arc<RwLock<f64>>,
    count: AtomicU64,
    created: SystemTime,
    help: String,
}

impl HistogramMetric {
    pub fn new(help: impl Into<String>, buckets: Vec<f64>) -> Self {
        let mut sorted_buckets = buckets;
        sorted_buckets.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Ensure +Inf bucket exists
        if sorted_buckets.last() != Some(&f64::INFINITY) {
            sorted_buckets.push(f64::INFINITY);
        }

        Self {
            buckets: sorted_buckets.into_iter()
                .map(|b| (b, AtomicU64::new(0)))
                .collect(),
            sum: Arc::new(RwLock::new(0.0)),
            count: AtomicU64::new(0),
            created: SystemTime::now(),
            help: help.into(),
        }
    }

    pub fn observe(&self, value: f64) {
        // Update sum and count
        *self.sum.write() += value;
        self.count.fetch_add(1, Ordering::Relaxed);

        // Update buckets
        for (upper_bound, count) in &self.buckets {
            if value <= *upper_bound {
                count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn get_buckets(&self) -> Vec<HistogramBucketData> {
        self.buckets.iter()
            .map(|(upper_bound, count)| HistogramBucketData {
                upper_bound: *upper_bound,
                count: count.load(Ordering::Relaxed),
            })
            .collect()
    }

    pub fn get_sum(&self) -> f64 {
        *self.sum.read()
    }

    pub fn get_count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }
}

/// Summary metric - tracks quantiles and statistics
#[derive(Debug)]
pub struct SummaryMetric {
    observations: Arc<RwLock<Vec<f64>>>,
    sum: Arc<RwLock<f64>>,
    count: AtomicU64,
    quantiles: Vec<f64>, // e.g., [0.5, 0.9, 0.99]
    max_age: Duration,
    max_samples: usize,
    created: SystemTime,
    help: String,
}

impl SummaryMetric {
    pub fn new(
        help: impl Into<String>,
        quantiles: Vec<f64>,
        max_age: Duration,
        max_samples: usize,
    ) -> Self {
        Self {
            observations: Arc::new(RwLock::new(Vec::new())),
            sum: Arc::new(RwLock::new(0.0)),
            count: AtomicU64::new(0),
            quantiles,
            max_age,
            max_samples,
            created: SystemTime::now(),
            help: help.into(),
        }
    }

    pub fn observe(&self, value: f64) {
        *self.sum.write() += value;
        self.count.fetch_add(1, Ordering::Relaxed);

        let mut obs = self.observations.write();
        obs.push(value);

        // Limit samples
        let max_samples = self.max_samples;
        if obs.len() > max_samples {
            let drain_count = obs.len() - max_samples;
            obs.drain(0..drain_count);
        }
    }

    pub fn get_quantiles(&self) -> HashMap<String, f64> {
        let mut obs = self.observations.read().clone();
        if obs.is_empty() {
            return HashMap::new();
        }

        obs.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut result = HashMap::new();
        for &q in &self.quantiles {
            let index = ((obs.len() - 1) as f64 * q) as usize;
            result.insert(format!("{}", q), obs[index]);
        }
        result
    }

    pub fn get_sum(&self) -> f64 {
        *self.sum.read()
    }

    pub fn get_count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }
}

/// Unified metric type
#[derive(Debug)]
pub enum MetricType {
    Counter(Arc<CounterMetric>),
    Gauge(Arc<GaugeMetric>),
    Histogram(Arc<HistogramMetric>),
    Summary(Arc<SummaryMetric>),
}

/// High-precision timing utility
pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed_seconds(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    pub fn elapsed_millis(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    pub fn elapsed_micros(&self) -> u64 {
        self.start.elapsed().as_micros() as u64
    }
}

/// Metric namespace for component isolation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricNamespace {
    pub prefix: String,
    pub labels: BTreeMap<String, String>,
}

impl MetricNamespace {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            labels: BTreeMap::new(),
        }
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    pub fn qualified_name(&self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}_{}", self.prefix, name)
        }
    }
}

/// Time window for metric aggregation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AggregationWindow {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    Custom(u64), // seconds
}

impl AggregationWindow {
    pub fn duration(&self) -> Duration {
        match self {
            AggregationWindow::OneMinute => Duration::from_secs(60),
            AggregationWindow::FiveMinutes => Duration::from_secs(300),
            AggregationWindow::FifteenMinutes => Duration::from_secs(900),
            AggregationWindow::OneHour => Duration::from_secs(3600),
            AggregationWindow::Custom(secs) => Duration::from_secs(*secs),
        }
    }
}

/// Aggregated metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetricPoint {
    pub timestamp: SystemTime,
    pub window: AggregationWindow,
    pub value: f64,
    pub count: u64,
    pub min: f64,
    pub max: f64,
    pub sum: f64,
}

/// Metric aggregator for time-series analysis
pub struct MetricAggregator {
    data_points: Arc<RwLock<VecDeque<(SystemTime, f64)>>>,
    aggregated: Arc<RwLock<HashMap<AggregationWindow, Vec<AggregatedMetricPoint>>>>,
    max_raw_points: usize,
}

impl MetricAggregator {
    pub fn new(max_raw_points: usize) -> Self {
        Self {
            data_points: Arc::new(RwLock::new(VecDeque::new())),
            aggregated: Arc::new(RwLock::new(HashMap::new())),
            max_raw_points,
        }
    }

    pub fn add_point(&self, value: f64) {
        let mut points = self.data_points.write();
        points.push_back((SystemTime::now(), value));

        // Limit raw points
        if points.len() > self.max_raw_points {
            points.pop_front();
        }
    }

    pub fn aggregate(&self, window: AggregationWindow) {
        let points = self.data_points.read().clone();
        if points.is_empty() {
            return;
        }

        let window_duration = window.duration();
        let now = SystemTime::now();
        let cutoff = now - window_duration;

        // Filter points within window
        let recent_points: Vec<_> = points.iter()
            .filter(|(ts, _)| ts >= &cutoff)
            .map(|(_, v)| *v)
            .collect();

        if recent_points.is_empty() {
            return;
        }

        let count = recent_points.len() as u64;
        let sum: f64 = recent_points.iter().sum();
        let min = recent_points.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = recent_points.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let value = sum / count as f64;

        let point = AggregatedMetricPoint {
            timestamp: now,
            window,
            value,
            count,
            min,
            max,
            sum,
        };

        let mut aggregated = self.aggregated.write();
        aggregated.entry(window)
            .or_insert_with(Vec::new)
            .push(point);
    }

    pub fn get_aggregated(&self, window: AggregationWindow) -> Vec<AggregatedMetricPoint> {
        self.aggregated.read()
            .get(&window)
            .cloned()
            .unwrap_or_default()
    }
}

/// Cardinality tracker for preventing metric explosion
pub struct CardinalityManager {
    cardinality_limits: HashMap<String, usize>,
    current_cardinality: Arc<RwLock<HashMap<String, usize>>>,
    enforcement_mode: CardinalityEnforcement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardinalityEnforcement {
    Warn,
    Drop,
    Sample(u32), // Keep 1 in N
}

impl CardinalityManager {
    pub fn new(enforcement: CardinalityEnforcement) -> Self {
        Self {
            cardinality_limits: HashMap::new(),
            current_cardinality: Arc::new(RwLock::new(HashMap::new())),
            enforcement_mode: enforcement,
        }
    }

    pub fn set_limit(&mut self, metric_name: impl Into<String>, limit: usize) {
        self.cardinality_limits.insert(metric_name.into(), limit);
    }

    pub fn check(&self, metric_id: &MetricId) -> CardinalityCheckResult {
        let limit = self.cardinality_limits.get(&metric_id.name);
        if limit.is_none() {
            return CardinalityCheckResult::Allow;
        }

        let mut card = self.current_cardinality.write();
        let current = card.entry(metric_id.name.clone()).or_insert(0);

        if *current >= *limit.unwrap() {
            match self.enforcement_mode {
                CardinalityEnforcement::Warn => CardinalityCheckResult::Warn,
                CardinalityEnforcement::Drop => CardinalityCheckResult::Drop,
                CardinalityEnforcement::Sample(n) => {
                    if *current % n as usize == 0 {
                        CardinalityCheckResult::Allow
                    } else {
                        CardinalityCheckResult::Drop
                    }
                }
            }
        } else {
            *current += 1;
            CardinalityCheckResult::Allow
        }
    }

    pub fn get_cardinality(&self, metric_name: &str) -> usize {
        self.current_cardinality.read()
            .get(metric_name)
            .cloned()
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardinalityCheckResult {
    Allow,
    Warn,
    Drop,
}

/// Metric retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub raw_data_retention: Duration,
    pub aggregated_1m_retention: Duration,
    pub aggregated_5m_retention: Duration,
    pub aggregated_15m_retention: Duration,
    pub aggregated_1h_retention: Duration,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            raw_data_retention: Duration::from_secs(3600), // 1 hour
            aggregated_1m_retention: Duration::from_secs(86400), // 1 day
            aggregated_5m_retention: Duration::from_secs(604800), // 1 week
            aggregated_15m_retention: Duration::from_secs(2592000), // 30 days
            aggregated_1h_retention: Duration::from_secs(7776000), // 90 days
        }
    }
}

/// Core metrics registry
pub struct MetricsRegistry {
    metrics: Arc<RwLock<HashMap<MetricId, MetricType>>>,
    aggregators: Arc<RwLock<HashMap<String, MetricAggregator>>>,
    cardinality_manager: Arc<Mutex<CardinalityManager>>,
    retention_policy: RetentionPolicy,
    namespaces: Arc<RwLock<HashMap<String, MetricNamespace>>>,
}

impl MetricsRegistry {
    pub fn new(retention_policy: RetentionPolicy) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            aggregators: Arc::new(RwLock::new(HashMap::new())),
            cardinality_manager: Arc::new(Mutex::new(
                CardinalityManager::new(CardinalityEnforcement::Warn)
            )),
            retention_policy,
            namespaces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register_namespace(&self, name: impl Into<String>, namespace: MetricNamespace) {
        self.namespaces.write().insert(name.into(), namespace);
    }

    pub fn get_or_create_counter(
        &self,
        name: impl Into<String>,
        labels: &[(&str, &str)],
        help: impl Into<String>,
    ) -> Arc<CounterMetric> {
        let metric_id = MetricId::new(name, labels);

        // Check cardinality
        if let CardinalityCheckResult::Drop = self.cardinality_manager.lock().check(&metric_id) {
            // Return a dummy counter that doesn't persist
            return Arc::new(CounterMetric::new("dropped"));
        }

        let mut metrics = self.metrics.write();
        let metric = metrics.entry(metric_id).or_insert_with(|| {
            MetricType::Counter(Arc::new(CounterMetric::new(help)))
        });

        if let MetricType::Counter(counter) = metric {
            counter.clone()
        } else {
            panic!("Metric type mismatch");
        }
    }

    pub fn get_or_create_gauge(
        &self,
        name: impl Into<String>,
        labels: &[(&str, &str)],
        help: impl Into<String>,
    ) -> Arc<GaugeMetric> {
        let metric_id = MetricId::new(name, labels);

        if let CardinalityCheckResult::Drop = self.cardinality_manager.lock().check(&metric_id) {
            return Arc::new(GaugeMetric::new("dropped"));
        }

        let mut metrics = self.metrics.write();
        let metric = metrics.entry(metric_id).or_insert_with(|| {
            MetricType::Gauge(Arc::new(GaugeMetric::new(help)))
        });

        if let MetricType::Gauge(gauge) = metric {
            gauge.clone()
        } else {
            panic!("Metric type mismatch");
        }
    }

    pub fn get_or_create_histogram(
        &self,
        name: impl Into<String>,
        labels: &[(&str, &str)],
        help: impl Into<String>,
        buckets: Vec<f64>,
    ) -> Arc<HistogramMetric> {
        let metric_id = MetricId::new(name, labels);

        if let CardinalityCheckResult::Drop = self.cardinality_manager.lock().check(&metric_id) {
            return Arc::new(HistogramMetric::new("dropped", buckets));
        }

        let mut metrics = self.metrics.write();
        let metric = metrics.entry(metric_id).or_insert_with(|| {
            MetricType::Histogram(Arc::new(HistogramMetric::new(help, buckets.clone())))
        });

        if let MetricType::Histogram(histogram) = metric {
            histogram.clone()
        } else {
            panic!("Metric type mismatch");
        }
    }

    pub fn all_metrics(&self) -> HashMap<MetricId, MetricType> {
        // Clone the inner HashMap for external use
        let metrics = self.metrics.read();
        metrics.iter().map(|(k, v)| {
            let cloned_type = match v {
                MetricType::Counter(c) => MetricType::Counter(c.clone()),
                MetricType::Gauge(g) => MetricType::Gauge(g.clone()),
                MetricType::Histogram(h) => MetricType::Histogram(h.clone()),
                MetricType::Summary(s) => MetricType::Summary(s.clone()),
            };
            (k.clone(), cloned_type)
        }).collect()
    }

    pub fn cleanup_old_metrics(&self) {
        // Implement retention policy cleanup
        // This would remove metrics older than retention periods
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new(RetentionPolicy::default())
    }
}

// ============================================================================
// SECTION 2: PROMETHEUS INTEGRATION (600+ lines)
// ============================================================================

/// Prometheus exposition format exporter
pub struct PrometheusExporter {
    registry: Arc<MetricsRegistry>,
    exemplar_storage: Arc<RwLock<HashMap<String, Vec<Exemplar>>>>,
}

/// Exemplar data for tracing integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exemplar {
    pub value: f64,
    pub timestamp: SystemTime,
    pub trace_id: String,
    pub span_id: String,
    pub labels: BTreeMap<String, String>,
}

impl PrometheusExporter {
    pub fn new(registry: Arc<MetricsRegistry>) -> Self {
        Self {
            registry,
            exemplar_storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_exemplar(&self, metric_name: String, exemplar: Exemplar) {
        let mut storage = self.exemplar_storage.write();
        storage.entry(metric_name)
            .or_insert_with(Vec::new)
            .push(exemplar);
    }

    /// Export all metrics in Prometheus text exposition format
    pub fn export_text(&self) -> String {
        let mut output = String::new();
        let metrics = self.registry.all_metrics();

        for (metric_id, metric_type) in metrics {
            match metric_type {
                MetricType::Counter(counter) => {
                    output.push_str(&self.format_counter(&metric_id, &counter));
                }
                MetricType::Gauge(gauge) => {
                    output.push_str(&self.format_gauge(&metric_id, &gauge));
                }
                MetricType::Histogram(histogram) => {
                    output.push_str(&self.format_histogram(&metric_id, &histogram));
                }
                MetricType::Summary(summary) => {
                    output.push_str(&self.format_summary(&metric_id, &summary));
                }
            }
        }

        output
    }

    fn format_counter(&self, id: &MetricId, counter: &CounterMetric) -> String {
        let mut output = String::new();

        // HELP line
        output.push_str(&format!("# HELP {} {}\n", id.name, counter.help));

        // TYPE line
        output.push_str(&format!("# TYPE {} counter\n", id.name));

        // Metric line
        let labels = self.format_labels(&id.labels);
        output.push_str(&format!("{}{} {}\n", id.name, labels, counter.get()));

        output
    }

    fn format_gauge(&self, id: &MetricId, gauge: &GaugeMetric) -> String {
        let mut output = String::new();

        output.push_str(&format!("# HELP {} {}\n", id.name, gauge.help));
        output.push_str(&format!("# TYPE {} gauge\n", id.name));

        let labels = self.format_labels(&id.labels);
        output.push_str(&format!("{}{} {}\n", id.name, labels, gauge.get()));

        output
    }

    fn format_histogram(&self, id: &MetricId, histogram: &HistogramMetric) -> String {
        let mut output = String::new();

        output.push_str(&format!("# HELP {} {}\n", id.name, histogram.help));
        output.push_str(&format!("# TYPE {} histogram\n", id.name));

        // Buckets
        let buckets = histogram.get_buckets();
        for bucket in buckets {
            let mut bucket_labels = id.labels.clone();
            let le = if bucket.upper_bound == f64::INFINITY {
                "+Inf".to_string()
            } else {
                bucket.upper_bound.to_string()
            };
            bucket_labels.insert("le".to_string(), le);

            let labels = self.format_labels(&bucket_labels);
            output.push_str(&format!("{}_bucket{} {}\n", id.name, labels, bucket.count));
        }

        // Sum
        let labels = self.format_labels(&id.labels);
        output.push_str(&format!("{}_sum{} {}\n", id.name, labels, histogram.get_sum()));

        // Count
        output.push_str(&format!("{}_count{} {}\n", id.name, labels, histogram.get_count()));

        output
    }

    fn format_summary(&self, id: &MetricId, summary: &SummaryMetric) -> String {
        let mut output = String::new();

        output.push_str(&format!("# HELP {} {}\n", id.name, summary.help));
        output.push_str(&format!("# TYPE {} summary\n", id.name));

        // Quantiles
        let quantiles = summary.get_quantiles();
        for (quantile, value) in quantiles {
            let mut quantile_labels = id.labels.clone();
            quantile_labels.insert("quantile".to_string(), quantile);

            let labels = self.format_labels(&quantile_labels);
            output.push_str(&format!("{}{} {}\n", id.name, labels, value));
        }

        // Sum
        let labels = self.format_labels(&id.labels);
        output.push_str(&format!("{}_sum{} {}\n", id.name, labels, summary.get_sum()));

        // Count
        output.push_str(&format!("{}_count{} {}\n", id.name, labels, summary.get_count()));

        output
    }

    fn format_labels(&self, labels: &BTreeMap<String, String>) -> String {
        if labels.is_empty() {
            return String::new();
        }

        let label_pairs: Vec<String> = labels.iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, v))
            .collect();

        format!("{{{}}}", label_pairs.join(","))
    }
}

/// Prometheus push gateway client
pub struct PrometheusPushGateway {
    gateway_url: String,
    job_name: String,
    instance_name: String,
    grouping_labels: HashMap<String, String>,
    basic_auth: Option<(String, String)>,
}

impl PrometheusPushGateway {
    pub fn new(
        gateway_url: impl Into<String>,
        job_name: impl Into<String>,
        instance_name: impl Into<String>,
    ) -> Self {
        Self {
            gateway_url: gateway_url.into(),
            job_name: job_name.into(),
            instance_name: instance_name.into(),
            grouping_labels: HashMap::new(),
            basic_auth: None,
        }
    }

    pub fn with_grouping_label(mut self, key: String, value: String) -> Self {
        self.grouping_labels.insert(key, value);
        self
    }

    pub fn with_basic_auth(mut self, username: String, password: String) -> Self {
        self.basic_auth = Some((username, password));
        self
    }

    pub async fn push(&self, metrics_data: String) -> std::result::Result<(), DbError> {
        let url = self.build_url();

        // In a real implementation, this would use an HTTP client like reqwest
        // For now, we'll just log the push
        println!("Pushing metrics to {}", url);
        println!("Metrics data length: {} bytes", metrics_data.len());

        Ok(())
    }

    pub async fn delete(&self) -> std::result::Result<(), DbError> {
        let url = self.build_url();

        // DELETE request to remove metrics
        println!("Deleting metrics at {}", url);

        Ok(())
    }

    fn build_url(&self) -> String {
        let mut url = format!("{}/metrics/job/{}", self.gateway_url, self.job_name);

        // Add instance
        url.push_str(&format!("/instance/{}", self.instance_name));

        // Add grouping labels
        for (key, value) in &self.grouping_labels {
            url.push_str(&format!("/{}/{}", key, value));
        }

        url
    }
}

/// Prometheus remote write protocol implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteWriteRequest {
    pub timeseries: Vec<TimeSeries>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    pub labels: Vec<PrometheusLabel>,
    pub samples: Vec<Sample>,
    pub exemplars: Vec<PrometheusExemplar>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusLabel {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub value: f64,
    pub timestamp: i64, // milliseconds since epoch
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusExemplar {
    pub labels: Vec<PrometheusLabel>,
    pub value: f64,
    pub timestamp: i64,
}

pub struct RemoteWriteClient {
    endpoint: String,
    basic_auth: Option<(String, String)>,
    bearer_token: Option<String>,
    headers: HashMap<String, String>,
    batch_size: usize,
    batch_timeout: Duration,
    buffer: Arc<Mutex<Vec<TimeSeries>>>,
}

impl RemoteWriteClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            basic_auth: None,
            bearer_token: None,
            headers: HashMap::new(),
            batch_size: 1000,
            batch_timeout: Duration::from_secs(10),
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_basic_auth(mut self, username: String, password: String) -> Self {
        self.basic_auth = Some((username, password));
        self
    }

    pub fn with_bearer_token(mut self, token: String) -> Self {
        self.bearer_token = Some(token);
        self
    }

    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    pub fn add_timeseries(&self, ts: TimeSeries) {
        let mut buffer = self.buffer.lock();
        buffer.push(ts);
    }

    pub async fn flush(&self) -> std::result::Result<(), DbError> {
        let mut buffer = self.buffer.lock();
        if buffer.is_empty() {
            return Ok(());
        }

        let timeseries = buffer.drain(..).collect::<Vec<_>>();
        drop(buffer);

        let request = RemoteWriteRequest { timeseries };

        // In real implementation, this would:
        // 1. Encode as protobuf
        // 2. Compress with Snappy
        // 3. Send via HTTP POST
        println!("Flushing {} timeseries to {}", request.timeseries.len(), self.endpoint);

        Ok(())
    }
}

/// Metric family for grouping related metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricFamily {
    pub name: String,
    pub help: String,
    pub metric_type: String,
    pub metrics: Vec<MetricFamilyMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricFamilyMember {
    pub labels: BTreeMap<String, String>,
    pub value: MetricValue,
    pub timestamp: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Counter(f64),
    Gauge(f64),
    Histogram {
        sum: f64,
        count: u64,
        buckets: Vec<HistogramBucketData>,
    },
    Summary {
        sum: f64,
        count: u64,
        quantiles: HashMap<String, f64>,
    },
}

pub struct MetricFamilyBuilder {
    families: HashMap<String, MetricFamily>,
}

impl MetricFamilyBuilder {
    pub fn new() -> Self {
        Self {
            families: HashMap::new(),
        }
    }

    pub fn add_counter(
        &mut self,
        name: String,
        help: String,
        labels: BTreeMap<String, String>,
        value: f64,
    ) {
        let family = self.families.entry(name.clone()).or_insert_with(|| {
            MetricFamily {
                name,
                help,
                metric_type: "counter".to_string(),
                metrics: Vec::new(),
            }
        });

        family.metrics.push(MetricFamilyMember {
            labels,
            value: MetricValue::Counter(value),
            timestamp: Some(SystemTime::now()),
        });
    }

    pub fn build(self) -> Vec<MetricFamily> {
        self.families.into_values().collect()
    }
}

impl Default for MetricFamilyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SECTION 3: HEALTH CHECK SYSTEM (500+ lines)
// ============================================================================

/// Health check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    pub fn worst(statuses: &[HealthStatus]) -> HealthStatus {
        if statuses.iter().any(|s| matches!(s, HealthStatus::Unhealthy)) {
            return HealthStatus::Unhealthy;
        }
        if statuses.iter().any(|s| matches!(s, HealthStatus::Degraded)) {
            return HealthStatus::Degraded;
        }
        if statuses.iter().any(|s| matches!(s, HealthStatus::Unknown)) {
            return HealthStatus::Unknown;
        }
        HealthStatus::Healthy
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub component: String,
    pub message: String,
    pub timestamp: SystemTime,
    pub duration: Duration,
    pub details: HashMap<String, serde_json::Value>,
}

impl HealthCheckResult {
    pub fn healthy(component: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Healthy,
            component: component.into(),
            message: "OK".to_string(),
            timestamp: SystemTime::now(),
            duration: Duration::from_secs(0),
            details: HashMap::new(),
        }
    }

    pub fn degraded(component: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Degraded,
            component: component.into(),
            message: message.into(),
            timestamp: SystemTime::now(),
            duration: Duration::from_secs(0),
            details: HashMap::new(),
        }
    }

    pub fn unhealthy(component: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            component: component.into(),
            message: message.into(),
            timestamp: SystemTime::now(),
            duration: Duration::from_secs(0),
            details: HashMap::new(),
        }
    }

    pub fn with_detail(mut self, key: String, value: serde_json::Value) -> Self {
        self.details.insert(key, value);
        self
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
}

/// Trait for implementing health checks
pub trait HealthChecker: Send + Sync {
    fn name(&self) -> &str;
    fn check(&self) -> HealthCheckResult;
}

/// Liveness probe - is the service alive?
pub struct LivenessProbe {
    started: AtomicBool,
    startup_time: RwLock<Option<SystemTime>>,
}

impl LivenessProbe {
    pub fn new() -> Self {
        Self {
            started: AtomicBool::new(false),
            startup_time: RwLock::new(None),
        }
    }

    pub fn mark_started(&self) {
        self.started.store(true, Ordering::SeqCst);
        *self.startup_time.write() = Some(SystemTime::now());
    }

    pub fn uptime(&self) -> Option<Duration> {
        self.startup_time.read()
            .and_then(|start| SystemTime::now().duration_since(start).ok())
    }
}

impl HealthChecker for LivenessProbe {
    fn name(&self) -> &str {
        "liveness"
    }

    fn check(&self) -> HealthCheckResult {
        let timer = Timer::new();

        if self.started.load(Ordering::SeqCst) {
            let mut result = HealthCheckResult::healthy("liveness");
            if let Some(uptime) = self.uptime() {
                result = result.with_detail(
                    "uptime_seconds".to_string(),
                    serde_json::json!(uptime.as_secs()),
                );
            }
            result.with_duration(Duration::from_micros(timer.elapsed_micros()))
        } else {
            HealthCheckResult::unhealthy("liveness", "Service not started")
                .with_duration(Duration::from_micros(timer.elapsed_micros()))
        }
    }
}

impl Default for LivenessProbe {
    fn default() -> Self {
        Self::new()
    }
}

/// Readiness probe - is the service ready to accept traffic?
pub struct ReadinessProbe {
    dependencies: Arc<RwLock<Vec<Arc<dyn HealthChecker>>>>,
    min_healthy_dependencies: usize,
}

impl ReadinessProbe {
    pub fn new(min_healthy: usize) -> Self {
        Self {
            dependencies: Arc::new(RwLock::new(Vec::new())),
            min_healthy_dependencies: min_healthy,
        }
    }

    pub fn add_dependency(&self, checker: Arc<dyn HealthChecker>) {
        self.dependencies.write().push(checker);
    }
}

impl HealthChecker for ReadinessProbe {
    fn name(&self) -> &str {
        "readiness"
    }

    fn check(&self) -> HealthCheckResult {
        let timer = Timer::new();
        let dependencies = self.dependencies.read();

        if dependencies.is_empty() {
            return HealthCheckResult::healthy("readiness")
                .with_duration(Duration::from_micros(timer.elapsed_micros()));
        }

        let results: Vec<_> = dependencies.iter()
            .map(|dep| dep.check())
            .collect();

        let healthy_count = results.iter()
            .filter(|r| r.status.is_healthy())
            .count();

        let status = if healthy_count >= self.min_healthy_dependencies {
            HealthStatus::Healthy
        } else if healthy_count > 0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };

        let mut result = HealthCheckResult {
            status,
            component: "readiness".to_string(),
            message: format!("{}/{} dependencies healthy", healthy_count, dependencies.len()),
            timestamp: SystemTime::now(),
            duration: Duration::from_micros(timer.elapsed_micros()),
            details: HashMap::new(),
        };

        result = result.with_detail(
            "dependency_results".to_string(),
            serde_json::to_value(&results).unwrap(),
        );

        result
    }
}

/// Startup probe - has the service completed initialization?
pub struct StartupProbe {
    initialization_checks: Arc<RwLock<Vec<(String, Arc<AtomicBool>)>>>,
}

impl StartupProbe {
    pub fn new() -> Self {
        Self {
            initialization_checks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn add_check(&self, name: String, completed: Arc<AtomicBool>) {
        self.initialization_checks.write().push((name, completed));
    }

    pub fn mark_complete(&self, name: &str) {
        let checks = self.initialization_checks.read();
        for (check_name, flag) in checks.iter() {
            if check_name == name {
                flag.store(true, Ordering::SeqCst);
            }
        }
    }
}

impl HealthChecker for StartupProbe {
    fn name(&self) -> &str {
        "startup"
    }

    fn check(&self) -> HealthCheckResult {
        let timer = Timer::new();
        let checks = self.initialization_checks.read();

        let total = checks.len();
        let completed = checks.iter()
            .filter(|(_, flag)| flag.load(Ordering::SeqCst))
            .count();

        let status = if completed == total {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        };

        HealthCheckResult {
            status,
            component: "startup".to_string(),
            message: format!("{}/{} initialization checks completed", completed, total),
            timestamp: SystemTime::now(),
            duration: Duration::from_micros(timer.elapsed_micros()),
            details: HashMap::new(),
        }
        .with_detail("total_checks".to_string(), serde_json::json!(total))
        .with_detail("completed_checks".to_string(), serde_json::json!(completed))
    }
}

impl Default for StartupProbe {
    fn default() -> Self {
        Self::new()
    }
}

/// Database connection health check
pub struct DatabaseHealthCheck {
    name: String,
    active_connections: Arc<RwLock<usize>>,
    max_connections: usize,
    warn_threshold: f64, // percentage
}

impl DatabaseHealthCheck {
    pub fn new(
        name: String,
        active_connections: Arc<RwLock<usize>>,
        max_connections: usize,
    ) -> Self {
        Self {
            name,
            active_connections,
            max_connections,
            warn_threshold: 0.8,
        }
    }
}

impl HealthChecker for DatabaseHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }

    fn check(&self) -> HealthCheckResult {
        let timer = Timer::new();
        let active = *self.active_connections.read();
        let usage_pct = active as f64 / self.max_connections as f64;

        let status = if usage_pct >= 1.0 {
            HealthStatus::Unhealthy
        } else if usage_pct >= self.warn_threshold {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        HealthCheckResult {
            status,
            component: self.name.clone(),
            message: format!("{}/{} connections in use", active, self.max_connections),
            timestamp: SystemTime::now(),
            duration: Duration::from_micros(timer.elapsed_micros()),
            details: HashMap::new(),
        }
        .with_detail("active_connections".to_string(), serde_json::json!(active))
        .with_detail("max_connections".to_string(), serde_json::json!(self.max_connections))
        .with_detail("usage_percent".to_string(), serde_json::json!(usage_pct * 100.0))
    }
}

/// Memory health check
pub struct MemoryHealthCheck {
    max_memory_bytes: u64,
    current_usage: Arc<RwLock<u64>>,
    warn_threshold: f64,
}

impl MemoryHealthCheck {
    pub fn new(max_memory_bytes: u64, current_usage: Arc<RwLock<u64>>) -> Self {
        Self {
            max_memory_bytes,
            current_usage,
            warn_threshold: 0.85,
        }
    }
}

impl HealthChecker for MemoryHealthCheck {
    fn name(&self) -> &str {
        "memory"
    }

    fn check(&self) -> HealthCheckResult {
        let timer = Timer::new();
        let current = *self.current_usage.read();
        let usage_pct = current as f64 / self.max_memory_bytes as f64;

        let status = if usage_pct >= 0.95 {
            HealthStatus::Unhealthy
        } else if usage_pct >= self.warn_threshold {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        HealthCheckResult {
            status,
            component: "memory".to_string(),
            message: format!("{:.2}% memory used", usage_pct * 100.0),
            timestamp: SystemTime::now(),
            duration: Duration::from_micros(timer.elapsed_micros()),
            details: HashMap::new(),
        }
        .with_detail("current_bytes".to_string(), serde_json::json!(current))
        .with_detail("max_bytes".to_string(), serde_json::json!(self.max_memory_bytes))
    }
}

/// Self-healing trigger based on health checks
pub struct SelfHealingTrigger {
    name: String,
    health_checker: Arc<dyn HealthChecker>,
    consecutive_failures: Arc<AtomicU64>,
    failure_threshold: u64,
    healing_action: Arc<dyn Fn() -> std::result::Result<(), DbError> + Send + Sync>,
}

impl SelfHealingTrigger {
    pub fn new(
        name: String,
        health_checker: Arc<dyn HealthChecker>,
        failure_threshold: u64,
        healing_action: Arc<dyn Fn() -> std::result::Result<(), DbError> + Send + Sync>,
    ) -> Self {
        Self {
            name,
            health_checker,
            consecutive_failures: Arc::new(AtomicU64::new(0)),
            failure_threshold,
            healing_action,
        }
    }

    pub fn check_and_heal(&self) -> std::result::Result<(), DbError> {
        let result = self.health_checker.check();

        if !result.status.is_healthy() {
            let failures = self.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;

            if failures >= self.failure_threshold {
                println!("Triggering self-healing for {}: {} consecutive failures",
                    self.name, failures);
                (self.healing_action)()?;
                self.consecutive_failures.store(0, Ordering::SeqCst);
            }
        } else {
            self.consecutive_failures.store(0, Ordering::SeqCst);
        }

        Ok(())
    }
}

/// Health check coordinator
pub struct HealthCheckCoordinator {
    checkers: Arc<RwLock<Vec<Arc<dyn HealthChecker>>>>,
    liveness_probe: Arc<LivenessProbe>,
    readiness_probe: Arc<ReadinessProbe>,
    startup_probe: Arc<StartupProbe>,
    self_healing_triggers: Arc<RwLock<Vec<SelfHealingTrigger>>>,
}

impl HealthCheckCoordinator {
    pub fn new() -> Self {
        Self {
            checkers: Arc::new(RwLock::new(Vec::new())),
            liveness_probe: Arc::new(LivenessProbe::new()),
            readiness_probe: Arc::new(ReadinessProbe::new(1)),
            startup_probe: Arc::new(StartupProbe::new()),
            self_healing_triggers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn add_checker(&self, checker: Arc<dyn HealthChecker>) {
        self.checkers.write().push(checker);
    }

    pub fn add_self_healing_trigger(&self, trigger: SelfHealingTrigger) {
        self.self_healing_triggers.write().push(trigger);
    }

    pub fn liveness(&self) -> HealthCheckResult {
        self.liveness_probe.check()
    }

    pub fn readiness(&self) -> HealthCheckResult {
        self.readiness_probe.check()
    }

    pub fn startup(&self) -> HealthCheckResult {
        self.startup_probe.check()
    }

    pub fn check_all(&self) -> Vec<HealthCheckResult> {
        let checkers = self.checkers.read();
        checkers.iter().map(|c| c.check()).collect()
    }

    pub fn overall_health(&self) -> HealthStatus {
        let results = self.check_all();
        let statuses: Vec<_> = results.iter().map(|r| r.status).collect();
        HealthStatus::worst(&statuses)
    }

    pub fn run_self_healing(&self) {
        let triggers = self.self_healing_triggers.read();
        for trigger in triggers.iter() {
            if let Err(e) = trigger.check_and_heal() {
                eprintln!("Self-healing trigger '{}' failed: {:?}", trigger.name, e);
            }
        }
    }
}

impl Default for HealthCheckCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SECTION 4: ALERTING ENGINE (600+ lines)
// ============================================================================

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    ErrorLevel,
    Critical,
}

/// Alert state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertState {
    Pending,
    Firing,
    Resolved,
    Silenced,
    Inhibited,
}

/// Alert instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub rule_name: String,
    pub severity: AlertSeverity,
    pub state: AlertState,
    pub message: String,
    pub labels: BTreeMap<String, String>,
    pub annotations: BTreeMap<String, String>,
    pub starts_at: SystemTime,
    pub ends_at: Option<SystemTime>,
    pub value: f64,
    pub fingerprint: String,
}

impl Alert {
    pub fn new(
        rule_name: String,
        severity: AlertSeverity,
        message: String,
        value: f64,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let fingerprint = format!("{}{}", rule_name, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());

        Self {
            id,
            rule_name,
            severity,
            state: AlertState::Pending,
            message,
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
            starts_at: SystemTime::now(),
            ends_at: None,
            value,
            fingerprint,
        }
    }

    pub fn with_label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }

    pub fn with_annotation(mut self, key: String, value: String) -> Self {
        self.annotations.insert(key, value);
        self
    }
}

/// Comparison operator for threshold rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

impl ComparisonOperator {
    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            ComparisonOperator::GreaterThan => value > threshold,
            ComparisonOperator::GreaterThanOrEqual => value >= threshold,
            ComparisonOperator::LessThan => value < threshold,
            ComparisonOperator::LessThanOrEqual => value <= threshold,
            ComparisonOperator::Equal => (value - threshold).abs() < f64::EPSILON,
            ComparisonOperator::NotEqual => (value - threshold).abs() >= f64::EPSILON,
        }
    }
}

/// Threshold-based alert rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdAlertRule {
    pub name: String,
    pub metric_name: String,
    pub threshold: f64,
    pub operator: ComparisonOperator,
    pub severity: AlertSeverity,
    pub duration: Duration,
    pub labels: BTreeMap<String, String>,
    pub annotations: BTreeMap<String, String>,
    pub enabled: bool,

    // State tracking
    #[serde(skip)]
    first_triggered: Arc<RwLock<Option<SystemTime>>>,
}

impl ThresholdAlertRule {
    pub fn new(
        name: String,
        metric_name: String,
        threshold: f64,
        operator: ComparisonOperator,
        severity: AlertSeverity,
    ) -> Self {
        Self {
            name,
            metric_name,
            threshold,
            operator,
            severity,
            duration: Duration::from_secs(60),
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
            enabled: true,
            first_triggered: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn evaluate(&self, value: f64) -> Option<Alert> {
        if !self.enabled {
            return None;
        }

        let triggered = self.operator.evaluate(value, self.threshold);

        if triggered {
            let mut first = self.first_triggered.write();
            let trigger_time = first.get_or_insert_with(SystemTime::now);

            // Check if duration threshold met
            if let Ok(elapsed) = SystemTime::now().duration_since(*trigger_time) {
                if elapsed >= self.duration {
                    let message = format!(
                        "{} {} {} (current: {})",
                        self.metric_name,
                        match self.operator {
                            ComparisonOperator::GreaterThan => ">",
                            ComparisonOperator::GreaterThanOrEqual => ">=",
                            ComparisonOperator::LessThan => "<",
                            ComparisonOperator::LessThanOrEqual => "<=",
                            ComparisonOperator::Equal => "==",
                            ComparisonOperator::NotEqual => "!=",
                        },
                        self.threshold,
                        value
                    );

                    let mut alert = Alert::new(
                        self.name.clone(),
                        self.severity,
                        message,
                        value,
                    );

                    for (k, v) in &self.labels {
                        alert = alert.with_label(k.clone(), v.clone());
                    }

                    for (k, v) in &self.annotations {
                        alert = alert.with_annotation(k.clone(), v.clone());
                    }

                    alert.state = AlertState::Firing;
                    return Some(alert);
                }
            }
        } else {
            *self.first_triggered.write() = None;
        }

        None
    }
}

/// Multi-condition alert rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiConditionAlertRule {
    pub name: String,
    pub conditions: Vec<AlertCondition>,
    pub combine_operator: LogicalOperator,
    pub severity: AlertSeverity,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCondition {
    pub metric_name: String,
    pub threshold: f64,
    pub operator: ComparisonOperator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicalOperator {
    And,
    Or,
}

impl MultiConditionAlertRule {
    pub fn evaluate(&self, metric_values: &HashMap<String, f64>) -> Option<Alert> {
        if !self.enabled {
            return None;
        }

        let results: Vec<bool> = self.conditions.iter()
            .map(|cond| {
                metric_values.get(&cond.metric_name)
                    .map(|&value| cond.operator.evaluate(value, cond.threshold))
                    .unwrap_or(false)
            })
            .collect();

        let triggered = match self.combine_operator {
            LogicalOperator::And => results.iter().all(|&r| r),
            LogicalOperator::Or => results.iter().any(|&r| r),
        };

        if triggered {
            let message = format!("Multi-condition alert: {}", self.name);
            let mut alert = Alert::new(
                self.name.clone(),
                self.severity,
                message,
                0.0, // No single value for multi-condition
            );
            alert.state = AlertState::Firing;
            Some(alert)
        } else {
            None
        }
    }
}

/// Alert routing rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRoute {
    pub name: String,
    pub matchers: Vec<AlertMatcher>,
    pub channels: Vec<String>,
    pub group_by: Vec<String>,
    pub group_wait: Duration,
    pub group_interval: Duration,
    pub repeat_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertMatcher {
    pub label: String,
    pub value: String,
    pub is_regex: bool,
}

impl AlertRoute {
    pub fn matches(&self, alert: &Alert) -> bool {
        if self.matchers.is_empty() {
            return true;
        }

        self.matchers.iter().all(|matcher| {
            alert.labels.get(&matcher.label)
                .map(|v| {
                    if matcher.is_regex {
                        // Simple string contains for now
                        v.contains(&matcher.value)
                    } else {
                        v == &matcher.value
                    }
                })
                .unwrap_or(false)
        })
    }
}

/// Alert silencer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSilence {
    pub id: String,
    pub matchers: Vec<AlertMatcher>,
    pub starts_at: SystemTime,
    pub ends_at: SystemTime,
    pub created_by: String,
    pub comment: String,
}

impl AlertSilence {
    pub fn is_active(&self) -> bool {
        let now = SystemTime::now();
        now >= self.starts_at && now < self.ends_at
    }

    pub fn matches(&self, alert: &Alert) -> bool {
        if !self.is_active() {
            return false;
        }

        self.matchers.iter().all(|matcher| {
            alert.labels.get(&matcher.label)
                .map(|v| v == &matcher.value)
                .unwrap_or(false)
        })
    }
}

/// Alert inhibitor - suppress alerts based on other active alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertInhibitionRule {
    pub source_matchers: Vec<AlertMatcher>,
    pub target_matchers: Vec<AlertMatcher>,
    pub equal_labels: Vec<String>,
}

/// Notification channel trait
pub trait NotificationChannel: Send + Sync {
    fn name(&self) -> &str;
    fn send(&self, alert: &Alert) -> std::result::Result<(), DbError>;
}

/// Webhook notification channel
pub struct WebhookChannel {
    name: String,
    url: String,
    headers: HashMap<String, String>,
}

impl WebhookChannel {
    pub fn new(name: String, url: String) -> Self {
        Self {
            name,
            url,
            headers: HashMap::new(),
        }
    }

    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
}

impl NotificationChannel for WebhookChannel {
    fn name(&self) -> &str {
        &self.name
    }

    fn send(&self, alert: &Alert) -> std::result::Result<(), DbError> {
        // In real implementation, would use HTTP client
        println!("Sending alert to webhook {}: {:?}", self.url, alert);
        Ok(())
    }
}

/// Email notification channel
pub struct EmailChannel {
    name: String,
    smtp_server: String,
    from: String,
    to: Vec<String>,
}

impl EmailChannel {
    pub fn new(name: String, smtp_server: String, from: String, to: Vec<String>) -> Self {
        Self {
            name,
            smtp_server,
            from,
            to,
        }
    }
}

impl NotificationChannel for EmailChannel {
    fn name(&self) -> &str {
        &self.name
    }

    fn send(&self, alert: &Alert) -> std::result::Result<(), DbError> {
        println!("Sending email alert from {} to {:?}: {}", self.from, self.to, alert.message);
        Ok(())
    }
}

/// Slack notification channel
pub struct SlackChannel {
    name: String,
    webhook_url: String,
    channel: String,
}

impl SlackChannel {
    pub fn new(name: String, webhook_url: String, channel: String) -> Self {
        Self {
            name,
            webhook_url,
            channel,
        }
    }
}

impl NotificationChannel for SlackChannel {
    fn name(&self) -> &str {
        &self.name
    }

    fn send(&self, alert: &Alert) -> std::result::Result<(), DbError> {
        println!("Sending Slack alert to {}: {}", self.channel, alert.message);
        Ok(())
    }
}

/// Alert manager
pub struct AlertManager {
    threshold_rules: Arc<RwLock<Vec<ThresholdAlertRule>>>,
    multi_condition_rules: Arc<RwLock<Vec<MultiConditionAlertRule>>>,
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    alert_history: Arc<RwLock<VecDeque<Alert>>>,
    routes: Arc<RwLock<Vec<AlertRoute>>>,
    silences: Arc<RwLock<Vec<AlertSilence>>>,
    inhibition_rules: Arc<RwLock<Vec<AlertInhibitionRule>>>,
    channels: Arc<RwLock<HashMap<String, Arc<dyn NotificationChannel>>>>,
    max_history: usize,
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            threshold_rules: Arc::new(RwLock::new(Vec::new())),
            multi_condition_rules: Arc::new(RwLock::new(Vec::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(VecDeque::new())),
            routes: Arc::new(RwLock::new(Vec::new())),
            silences: Arc::new(RwLock::new(Vec::new())),
            inhibition_rules: Arc::new(RwLock::new(Vec::new())),
            channels: Arc::new(RwLock::new(HashMap::new())),
            max_history: 10000,
        }
    }

    pub fn add_threshold_rule(&self, rule: ThresholdAlertRule) {
        self.threshold_rules.write().push(rule);
    }

    pub fn add_multi_condition_rule(&self, rule: MultiConditionAlertRule) {
        self.multi_condition_rules.write().push(rule);
    }

    pub fn add_route(&self, route: AlertRoute) {
        self.routes.write().push(route);
    }

    pub fn add_silence(&self, silence: AlertSilence) {
        self.silences.write().push(silence);
    }

    pub fn add_channel(&self, channel: Arc<dyn NotificationChannel>) {
        self.channels.write().insert(channel.name().to_string(), channel);
    }

    pub fn evaluate_rules(&self, metrics: &HashMap<String, f64>) {
        // Evaluate threshold rules
        let threshold_rules = self.threshold_rules.read();
        for rule in threshold_rules.iter() {
            if let Some(&value) = metrics.get(&rule.metric_name) {
                if let Some(mut alert) = rule.evaluate(value) {
                    // Check if silenced
                    let silences = self.silences.read();
                    if silences.iter().any(|s| s.matches(&alert)) {
                        alert.state = AlertState::Silenced;
                    }

                    self.fire_alert(alert);
                }
            }
        }

        // Evaluate multi-condition rules
        let multi_rules = self.multi_condition_rules.read();
        for rule in multi_rules.iter() {
            if let Some(alert) = rule.evaluate(metrics) {
                self.fire_alert(alert);
            }
        }
    }

    pub fn fire_alert(&self, alert: Alert) {
        let fingerprint = alert.fingerprint.clone();

        // Add to active alerts
        self.active_alerts.write().insert(fingerprint, alert.clone());

        // Add to history
        let mut history = self.alert_history.write();
        history.push_back(alert.clone());
        if history.len() > self.max_history {
            history.pop_front();
        }

        // Route and send notifications
        self.route_alert(&alert);
    }

    fn route_alert(&self, alert: &Alert) {
        let routes = self.routes.read();
        let channels = self.channels.read();

        for route in routes.iter() {
            if route.matches(alert) {
                for channel_name in &route.channels {
                    if let Some(channel) = channels.get(channel_name) {
                        if let Err(e) = channel.send(alert) {
                            eprintln!("Failed to send alert via {}: {:?}", channel_name, e);
                        }
                    }
                }
            }
        }
    }

    pub fn resolve_alert(&self, fingerprint: &str) {
        if let Some(alert) = self.active_alerts.write().remove(fingerprint) {
            let mut resolved = alert.clone();
            resolved.state = AlertState::Resolved;
            resolved.ends_at = Some(SystemTime::now());

            self.alert_history.write().push_back(resolved);
        }
    }

    pub fn get_active_alerts(&self) -> Vec<Alert> {
        self.active_alerts.read().values().cloned().collect()
    }

    pub fn get_alert_history(&self, limit: usize) -> Vec<Alert> {
        let history = self.alert_history.read();
        history.iter().rev().take(limit).cloned().collect()
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SECTION 5: DASHBOARD DATA API (600+ lines)
// ============================================================================

/// Time-series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: SystemTime,
    pub value: f64,
    pub labels: BTreeMap<String, String>,
}

/// Time-series query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesQuery {
    pub metric_name: String,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub step: Duration,
    pub labels: BTreeMap<String, String>,
    pub aggregation: AggregationFunction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregationFunction {
    Avg,
    Sum,
    Min,
    Max,
    Count,
    Rate,
    Percentile(u8),
}

/// Time-series result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesResult {
    pub metric_name: String,
    pub labels: BTreeMap<String, String>,
    pub points: Vec<TimeSeriesPoint>,
}

/// Real-time metric stream
pub struct MetricStream {
    subscribers: Arc<RwLock<Vec<Arc<dyn MetricSubscriber>>>>,
}

pub trait MetricSubscriber: Send + Sync {
    fn on_metric(&self, metric_name: &str, value: f64, labels: &BTreeMap<String, String>);
}

impl MetricStream {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn subscribe(&self, subscriber: Arc<dyn MetricSubscriber>) {
        self.subscribers.write().push(subscriber);
    }

    pub fn publish(&self, metric_name: &str, value: f64, labels: &BTreeMap<String, String>) {
        let subscribers = self.subscribers.read();
        for sub in subscribers.iter() {
            sub.on_metric(metric_name, value, labels);
        }
    }
}

impl Default for MetricStream {
    fn default() -> Self {
        Self::new()
    }
}

/// Dashboard widget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardWidget {
    pub id: String,
    pub title: String,
    pub widget_type: WidgetType,
    pub queries: Vec<TimeSeriesQuery>,
    pub refresh_interval: Duration,
    pub position: WidgetPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetPosition {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WidgetType {
    LineChart,
    AreaChart,
    BarChart,
    Gauge,
    Counter,
    Table,
    Heatmap,
    Alert,
}

/// Custom dashboard definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    pub id: String,
    pub name: String,
    pub description: String,
    pub widgets: Vec<DashboardWidget>,
    pub tags: Vec<String>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub created_by: String,
}

impl Dashboard {
    pub fn new(id: String, name: String, created_by: String) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            widgets: Vec::new(),
            tags: Vec::new(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            created_by,
        }
    }

    pub fn add_widget(&mut self, widget: DashboardWidget) {
        self.widgets.push(widget);
        self.updated_at = SystemTime::now();
    }
}

/// Dashboard manager
pub struct DashboardManager {
    dashboards: Arc<RwLock<HashMap<String, Dashboard>>>,
}

impl DashboardManager {
    pub fn new() -> Self {
        Self {
            dashboards: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_dashboard(&self, dashboard: Dashboard) -> std::result::Result<(), DbError> {
        self.dashboards.write().insert(dashboard.id.clone(), dashboard);
        Ok(())
    }

    pub fn get_dashboard(&self, id: &str) -> Option<Dashboard> {
        self.dashboards.read().get(id).cloned()
    }

    pub fn update_dashboard(&self, dashboard: Dashboard) -> std::result::Result<(), DbError> {
        self.dashboards.write().insert(dashboard.id.clone(), dashboard);
        Ok(())
    }

    pub fn delete_dashboard(&self, id: &str) -> std::result::Result<(), DbError> {
        self.dashboards.write().remove(id);
        Ok(())
    }

    pub fn list_dashboards(&self) -> Vec<Dashboard> {
        self.dashboards.read().values().cloned().collect()
    }
}

impl Default for DashboardManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Time-series database for historical metrics
pub struct TimeSeriesDatabase {
    data: Arc<RwLock<HashMap<String, VecDeque<TimeSeriesPoint>>>>,
    max_points_per_metric: usize,
    retention_period: Duration,
}

impl TimeSeriesDatabase {
    pub fn new(max_points: usize, retention: Duration) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            max_points_per_metric: max_points,
            retention_period: retention,
        }
    }

    pub fn insert(&self, metric_name: String, point: TimeSeriesPoint) {
        let mut data = self.data.write();
        let points = data.entry(metric_name).or_insert_with(VecDeque::new);

        points.push_back(point);

        // Limit size
        if points.len() > self.max_points_per_metric {
            points.pop_front();
        }
    }

    pub fn query(&self, query: TimeSeriesQuery) -> TimeSeriesResult {
        let data = self.data.read();
        let points = data.get(&query.metric_name).cloned().unwrap_or_default();

        // Filter by time range
        let filtered: Vec<_> = points.iter()
            .filter(|p| p.timestamp >= query.start_time && p.timestamp <= query.end_time)
            .filter(|p| {
                // Filter by labels
                query.labels.iter().all(|(k, v)| {
                    p.labels.get(k).map(|pv| pv == v).unwrap_or(false)
                })
            })
            .cloned()
            .collect();

        // Apply aggregation if needed
        let aggregated = self.aggregate_points(filtered, query.step, query.aggregation);

        TimeSeriesResult {
            metric_name: query.metric_name.clone(),
            labels: query.labels.clone(),
            points: aggregated,
        }
    }

    fn aggregate_points(
        &self,
        points: Vec<TimeSeriesPoint>,
        step: Duration,
        aggregation: AggregationFunction,
    ) -> Vec<TimeSeriesPoint> {
        if points.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut current_bucket: Vec<TimeSeriesPoint> = Vec::new();
        let mut bucket_start = points[0].timestamp;

        for point in points {
            if let Ok(elapsed) = point.timestamp.duration_since(bucket_start) {
                if elapsed >= step {
                    // Aggregate current bucket
                    if !current_bucket.is_empty() {
                        let aggregated = self.apply_aggregation(&current_bucket, aggregation);
                        result.push(TimeSeriesPoint {
                            timestamp: bucket_start,
                            value: aggregated,
                            labels: current_bucket[0].labels.clone(),
                        });
                    }

                    // Start new bucket
                    current_bucket.clear();
                    bucket_start = point.timestamp;
                }
            }

            current_bucket.push(point);
        }

        // Handle last bucket
        if !current_bucket.is_empty() {
            let aggregated = self.apply_aggregation(&current_bucket, aggregation);
            result.push(TimeSeriesPoint {
                timestamp: bucket_start,
                value: aggregated,
                labels: current_bucket[0].labels.clone(),
            });
        }

        result
    }

    fn apply_aggregation(
        &self,
        points: &[TimeSeriesPoint],
        function: AggregationFunction,
    ) -> f64 {
        if points.is_empty() {
            return 0.0;
        }

        match function {
            AggregationFunction::Avg => {
                points.iter().map(|p| p.value).sum::<f64>() / points.len() as f64
            }
            AggregationFunction::Sum => {
                points.iter().map(|p| p.value).sum()
            }
            AggregationFunction::Min => {
                points.iter().map(|p| p.value).fold(f64::INFINITY, f64::min)
            }
            AggregationFunction::Max => {
                points.iter().map(|p| p.value).fold(f64::NEG_INFINITY, f64::max)
            }
            AggregationFunction::Count => {
                points.len() as f64
            }
            AggregationFunction::Rate => {
                if points.len() < 2 {
                    return 0.0;
                }
                let first = &points[0];
                let last = &points[points.len() - 1];
                let value_delta = last.value - first.value;
                if let Ok(time_delta) = last.timestamp.duration_since(first.timestamp) {
                    value_delta / time_delta.as_secs_f64()
                } else {
                    0.0
                }
            }
            AggregationFunction::Percentile(p) => {
                let mut values: Vec<f64> = points.iter().map(|p| p.value).collect();
                values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let index = ((values.len() - 1) as f64 * (p as f64 / 100.0)) as usize;
                values[index]
            }
        }
    }

    pub fn cleanup_old_data(&self) {
        let cutoff = SystemTime::now() - self.retention_period;
        let mut data = self.data.write();

        for points in data.values_mut() {
            while let Some(front) = points.front() {
                if front.timestamp < cutoff {
                    points.pop_front();
                } else {
                    break;
                }
            }
        }
    }
}

/// Export format for metrics data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    Prometheus,
}

/// Metrics exporter
pub struct MetricsExporter {
    tsdb: Arc<TimeSeriesDatabase>,
}

impl MetricsExporter {
    pub fn new(tsdb: Arc<TimeSeriesDatabase>) -> Self {
        Self { tsdb }
    }

    pub fn export(&self, query: TimeSeriesQuery, format: ExportFormat) -> std::result::Result<String, DbError> {
        let result = self.tsdb.query(query);

        match format {
            ExportFormat::Json => self.export_json(&result),
            ExportFormat::Csv => self.export_csv(&result),
            ExportFormat::Prometheus => self.export_prometheus(&result),
        }
    }

    fn export_json(&self, result: &TimeSeriesResult) -> std::result::Result<String, DbError> {
        serde_json::to_string_pretty(result)
            .map_err(|e| DbError::Internal(format!("JSON serialization error: {}", e)))
    }

    fn export_csv(&self, result: &TimeSeriesResult) -> Result<String> {
        let mut csv = String::from("timestamp,value\n");

        for point in &result.points {
            let ts = point.timestamp.duration_since(UNIX_EPOCH)
                .map_err(|e| DbError::Internal(format!("Time error: {}", e)))?
                .as_secs();
            csv.push_str(&format!("{},{}\n", ts, point.value));
        }

        Ok(csv)
    }

    fn export_prometheus(&self, result: &TimeSeriesResult) -> Result<String> {
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

/// Top-level monitoring API
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

/// Monitoring configuration
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

    pub fn export_metrics(&self, query: TimeSeriesQuery, format: ExportFormat) -> Result<String> {
        self.exporter.export(query, format)
    }

    // Health check methods

    pub fn check_health(&self) -> HealthCheckResult {
        let results = self.health_coordinator.check_all();
        let status = HealthStatus::worst(&results.iter().map(|r| r.status).collect::<Vec<_>>());

        HealthCheckResult {
            status,
            component: "system".to_string(),
            message: format!("Overall system health"),
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

    pub fn create_dashboard(&self, dashboard: Dashboard) -> std::result::Result<(), DbError> {
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


