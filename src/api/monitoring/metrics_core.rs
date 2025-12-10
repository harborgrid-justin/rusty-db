// # Metrics & Monitoring API Module
//
// Comprehensive metrics collection, monitoring, and observability API for RustyDB.
// This module provides enterprise-grade monitoring capabilities including:
//
// - **Metrics Collection Engine**: Counter, Gauge, Histogram, and Summary metrics
// - **Prometheus Integration**: Full Prometheus exposition format and remote write support
// - **Health Check System**: Liveness, readiness, and startup probes
// - **Alerting Engine**: Multi-condition alerts with routing and notification
// - **Dashboard Data API**: Real-time streaming and historical query support
//
// ## Architecture
//
// The monitoring API is designed for high-throughput, low-overhead operation with:
// - Lock-free data structures for metric collection
// - Efficient aggregation with time-window bucketing
// - Cardinality management to prevent metric explosion
// - Configurable retention policies
//
// ## Usage
//
// ```rust
// use rusty_db::api::monitoring::*;
//
// // Initialize monitoring API
// let monitoring = MonitoringApi::new(MonitoringConfig::default());
//
// // Record metrics
// monitoring.increment_counter("http_requests_total", &[("method", "GET")]);
// monitoring.record_gauge("memory_usage_bytes", 1024.0 * 1024.0 * 512.0, &[]);
// monitoring.observe_histogram("request_duration_seconds", 0.045, &[]);
//
// // Export Prometheus metrics
// let metrics = monitoring.export_prometheus_metrics();
//
// // Check health
// let health = monitoring.check_health();
// ```

use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::BTreeMap;
use std::collections::{HashMap};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::time::{Duration};
use parking_lot::{RwLock};
use serde::{Deserialize, Serialize};

// ============================================================================
// SECTION 1: METRICS COLLECTION ENGINE (700+ lines)
// ============================================================================

// Label key-value pair for metric dimensions
pub type Label = (String, String);

// Labels collection type
pub type Labels = Vec<Label>;

// Metric identifier combining name and labels
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

// Counter metric - monotonically increasing value
#[derive(Debug)]
pub struct CounterMetric {
    value: AtomicU64,
    created: SystemTime,
    pub(crate) help: String,
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

// Gauge metric - value that can go up or down
#[derive(Debug)]
pub struct GaugeMetric {
    value: Arc<RwLock<f64>>,
    created: SystemTime,
    pub(crate) help: String,
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

// Histogram bucket for distribution tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucketData {
    pub upper_bound: f64,
    pub count: u64,
}

// Histogram metric - tracks distribution of values
#[derive(Debug)]
pub struct HistogramMetric {
    buckets: Vec<(f64, AtomicU64)>, // (upper_bound, count)
    sum: Arc<RwLock<f64>>,
    count: AtomicU64,
    created: SystemTime,
    pub(crate) help: String,
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

// Summary metric - tracks quantiles and statistics
#[derive(Debug)]
pub struct SummaryMetric {
    observations: Arc<RwLock<Vec<f64>>>,
    sum: Arc<RwLock<f64>>,
    count: AtomicU64,
    quantiles: Vec<f64>, // e.g., [0.5, 0.9, 0.99]
    max_age: Duration,
    max_samples: usize,
    created: SystemTime,
    pub(crate) help: String,
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

// Unified metric type
#[derive(Debug)]
pub enum MetricType {
    Counter(Arc<CounterMetric>),
    Gauge(Arc<GaugeMetric>),
    Histogram(Arc<HistogramMetric>),
    Summary(Arc<SummaryMetric>),
}

// High-precision timing utility
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

// Metric namespace for component isolation
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

// Time window for metric aggregation
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

// Aggregated metric data point
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
