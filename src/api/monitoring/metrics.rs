//! Core metric types and collection
//!
//! Provides fundamental metric types including counters, gauges, histograms,
//! and summaries for comprehensive system monitoring.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, Instant, Duration};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

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

    pub fn help(&self) -> &str {
        &self.help
    }

    pub fn created_at(&self) -> SystemTime {
        self.created
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

    pub fn help(&self) -> &str {
        &self.help
    }

    pub fn created_at(&self) -> SystemTime {
        self.created
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
    buckets: Vec<(f64, AtomicU64)>,
    sum: Arc<RwLock<f64>>,
    count: AtomicU64,
    created: SystemTime,
    help: String,
}

impl HistogramMetric {
    pub fn new(help: impl Into<String>, buckets: Vec<f64>) -> Self {
        let mut sorted_buckets = buckets;
        sorted_buckets.sort_by(|a, b| a.partial_cmp(b).unwrap());

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
        *self.sum.write() += value;
        self.count.fetch_add(1, Ordering::Relaxed);

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

    pub fn help(&self) -> &str {
        &self.help
    }

    pub fn created_at(&self) -> SystemTime {
        self.created
    }
}

/// Summary metric - tracks quantiles and statistics
#[derive(Debug)]
pub struct SummaryMetric {
    observations: Arc<RwLock<Vec<f64>>>,
    sum: Arc<RwLock<f64>>,
    count: AtomicU64,
    quantiles: Vec<f64>,
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

        if obs.len() > self.max_samples {
            let drain_count = obs.len() - self.max_samples;
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

    pub fn help(&self) -> &str {
        &self.help
    }

    pub fn created_at(&self) -> SystemTime {
        self.created
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

impl Default for Timer {
    fn default() -> Self {
        Self::new()
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
