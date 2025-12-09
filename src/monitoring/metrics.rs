// Metrics Collection System
// Prometheus-compatible metrics exposition with custom metric registration

use std::time::SystemTime;
use std::time::Instant;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration};


/// Metric types supported by the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Counter metric - monotonically increasing value
#[derive(Debug, Clone)]
pub struct Counter {
    name: String,
    help: String,
    value: Arc<RwLock<f64>>,
    labels: HashMap<String, String>,
    created_at: Instant,
}

impl Counter {
    pub fn new(name: impl Into<String>, help: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            help: help.into(),
            value: Arc::new(RwLock::new(0.0)),
            labels: HashMap::new(),
            created_at: Instant::now(),
        }
    }

    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels = labels;
        self
    }

    pub fn inc(&self) {
        *self.value.write() += 1.0;
    }

    pub fn inc_by(&self, value: f64) {
        if value >= 0.0 {
            *self.value.write() += value;
        }
    }

    pub fn get(&self) -> f64 {
        *self.value.read()
    }

    pub fn reset(&self) {
        *self.value.write() = 0.0;
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn help(&self) -> &str {
        &self.help
    }

    pub fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }
}

/// Gauge metric - value that can go up and down
#[derive(Debug, Clone)]
pub struct Gauge {
    name: String,
    help: String,
    value: Arc<RwLock<f64>>,
    labels: HashMap<String, String>,
    created_at: Instant,
}

impl Gauge {
    pub fn new(name: impl Into<String>, help: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            help: help.into(),
            value: Arc::new(RwLock::new(0.0)),
            labels: HashMap::new(),
            created_at: Instant::now(),
        }
    }

    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels = labels;
        self
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

    pub fn add(&self, value: f64) {
        *self.value.write() += value;
    }

    pub fn sub(&self, value: f64) {
        *self.value.write() -= value;
    }

    pub fn get(&self) -> f64 {
        *self.value.read()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn help(&self) -> &str {
        &self.help
    }

    pub fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }
}

/// Histogram bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub upper_bound: f64,
    pub count: u64,
}

/// Histogram metric - samples observations and counts them in buckets
#[derive(Debug, Clone)]
pub struct Histogram {
    name: String,
    help: String,
    buckets: Arc<RwLock<Vec<HistogramBucket>>>,
    sum: Arc<RwLock<f64>>,
    count: Arc<RwLock<u64>>,
    labels: HashMap<String, String>,
    created_at: Instant,
}

impl Histogram {
    pub fn new(name: impl Into<String>, help: impl Into<String>, buckets: Vec<f64>) -> Self {
        let mut bucket_vec = buckets
            .into_iter()
            .map(|upper_bound| HistogramBucket {
                upper_bound,
                count: 0,
            })
            .collect::<Vec<_>>();

        // Always add +Inf bucket
        bucket_vec.push(HistogramBucket {
            upper_bound: f64::INFINITY,
            count: 0,
        });

        Self {
            name: name.into(),
            help: help.into(),
            buckets: Arc::new(RwLock::new(bucket_vec)),
            sum: Arc::new(RwLock::new(0.0)),
            count: Arc::new(RwLock::new(0)),
            labels: HashMap::new(),
            created_at: Instant::now(),
        }
    }

    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels = labels;
        self
    }

    pub fn observe(&self, value: f64) {
        *self.sum.write() += value;
        *self.count.write() += 1;

        let mut buckets = self.buckets.write();
        for bucket in buckets.iter_mut() {
            if value <= bucket.upper_bound {
                bucket.count += 1;
            }
        }
    }

    pub fn get_sum(&self) -> f64 {
        *self.sum.read()
    }

    pub fn get_count(&self) -> u64 {
        *self.count.read()
    }

    pub fn get_buckets(&self) -> Vec<HistogramBucket> {
        self.buckets.read().clone()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn help(&self) -> &str {
        &self.help
    }

    pub fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }
}

/// Summary quantile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quantile {
    pub quantile: f64,
    pub value: f64,
}

/// Summary metric - similar to histogram but calculates quantiles
#[derive(Debug, Clone)]
pub struct Summary {
    name: String,
    help: String,
    observations: Arc<RwLock<Vec<f64>>>,
    sum: Arc<RwLock<f64>>,
    count: Arc<RwLock<u64>>,
    labels: HashMap<String, String>,
    max_age: Duration,
    created_at: Instant,
}

impl Summary {
    pub fn new(name: impl Into<String>, help: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            help: help.into(),
            observations: Arc::new(RwLock::new(Vec::new())),
            sum: Arc::new(RwLock::new(0.0)),
            count: Arc::new(RwLock::new(0)),
            labels: HashMap::new(),
            max_age: Duration::from_secs(600), // 10 minutes
            created_at: Instant::now(),
        }
    }

    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels = labels;
        self
    }

    pub fn observe(&self, value: f64) {
        *self.sum.write() += value;
        *self.count.write() += 1;
        self.observations.write().push(value);
    }

    pub fn get_sum(&self) -> f64 {
        *self.sum.read()
    }

    pub fn get_count(&self) -> u64 {
        *self.count.read()
    }

    pub fn get_quantile(&self, q: f64) -> Option<f64> {
        let mut obs = self.observations.read().clone();
        if obs.is_empty() {
            return None;
        }

        obs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let index = (q * (obs.len() - 1) as f64).round() as usize;
        Some(obs[index])
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn help(&self) -> &str {
        &self.help
    }

    pub fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }
}

/// Metric wrapper for different metric types
#[derive(Debug, Clone)]
pub enum Metric {
    Counter(Counter),
    Gauge(Gauge),
    Histogram(Histogram),
    Summary(Summary),
}

impl Metric {
    pub fn name(&self) -> &str {
        match self {
            Metric::Counter(c) => c.name(),
            Metric::Gauge(g) => g.name(),
            Metric::Histogram(h) => h.name(),
            Metric::Summary(s) => s.name(),
        }
    }

    pub fn metric_type(&self) -> MetricType {
        match self {
            Metric::Counter(_) => MetricType::Counter,
            Metric::Gauge(_) => MetricType::Gauge,
            Metric::Histogram(_) => MetricType::Histogram,
            Metric::Summary(_) => MetricType::Summary,
        }
    }
}

/// Metric registry for managing all metrics
pub struct MetricRegistry {
    metrics: Arc<RwLock<HashMap<String, Metric>>>,
    prefix: String,
}

impl MetricRegistry {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            prefix: prefix.into(),
        }
    }

    pub fn register_counter(&self, name: impl Into<String>, help: impl Into<String>) -> Counter {
        let full_name = format!("{}_{}", self.prefix, name.into()));
        let counter = Counter::new(full_name.clone(), help);
        self.metrics.write().insert(full_name, Metric::Counter(counter.clone()));
        counter
    }

    pub fn register_counter_with_labels(
        &self,
        name: impl Into<String>,
        help: impl Into<String>,
        labels: HashMap<String, String>,
    ) -> Counter {
        let full_name = format!("{}_{}", self.prefix, name.into()));
        let counter = Counter::new(full_name.clone(), help).with_labels(labels);
        self.metrics.write().insert(full_name, Metric::Counter(counter.clone()));
        counter
    }

    pub fn register_gauge(&self, name: impl Into<String>, help: impl Into<String>) -> Gauge {
        let full_name = format!("{}_{}", self.prefix, name.into()));
        let gauge = Gauge::new(full_name.clone(), help);
        self.metrics.write().insert(full_name, Metric::Gauge(gauge.clone()));
        gauge
    }

    pub fn register_gauge_with_labels(
        &self,
        name: impl Into<String>,
        help: impl Into<String>,
        labels: HashMap<String, String>,
    ) -> Gauge {
        let full_name = format!("{}_{}", self.prefix, name.into()));
        let gauge = Gauge::new(full_name.clone(), help).with_labels(labels);
        self.metrics.write().insert(full_name, Metric::Gauge(gauge.clone()));
        gauge
    }

    pub fn register_histogram(
        &self,
        name: impl Into<String>,
        help: impl Into<String>,
        buckets: Vec<f64>,
    ) -> Histogram {
        let full_name = format!("{}_{}", self.prefix, name.into()));
        let histogram = Histogram::new(full_name.clone(), help, buckets);
        self.metrics.write().insert(full_name, Metric::Histogram(histogram.clone()));
        histogram
    }

    pub fn register_histogram_with_labels(
        &self,
        name: impl Into<String>,
        help: impl Into<String>,
        buckets: Vec<f64>,
        labels: HashMap<String, String>,
    ) -> Histogram {
        let full_name = format!("{}_{}", self.prefix, name.into()));
        let histogram = Histogram::new(full_name.clone(), help, buckets).with_labels(labels);
        self.metrics.write().insert(full_name, Metric::Histogram(histogram.clone()));
        histogram
    }

    pub fn register_summary(&self, name: impl Into<String>, help: impl Into<String>) -> Summary {
        let full_name = format!("{}_{}", self.prefix, name.into()));
        let summary = Summary::new(full_name.clone(), help);
        self.metrics.write().insert(full_name, Metric::Summary(summary.clone()));
        summary
    }

    pub fn unregister(&self, name: &str) -> Option<Metric> {
        let full_name = format!("{}_{}", self.prefix, name));
        self.metrics.write().remove(&full_name)
    }

    pub fn get_metric(&self, name: &str) -> Option<Metric> {
        let full_name = format!("{}_{}", self.prefix, name));
        self.metrics.read().get(&full_name).cloned()
    }

    pub fn list_metrics(&self) -> Vec<String> {
        self.metrics.read().keys().cloned().collect()
    }

    /// Expose metrics in Prometheus text format
    pub fn expose_prometheus(&self) -> String {
        let mut output = String::new();
        let metrics = self.metrics.read();

        for (_, metric) in metrics.iter() {
            match metric {
                Metric::Counter(counter) => {
                    output.push_str(&format!("# HELP {} {}\n", counter.name(), counter.help())));
                    output.push_str(&format!("# TYPE {} counter\n", counter.name())));
                    output.push_str(&format!(
                        "{}{} {}\n",
                        counter.name(),
                        Self::format_labels(counter.labels()),
                        counter.get()
                    )));
                }
                Metric::Gauge(gauge) => {
                    output.push_str(&format!("# HELP {} {}\n", gauge.name(), gauge.help())));
                    output.push_str(&format!("# TYPE {} gauge\n", gauge.name())));
                    output.push_str(&format!(
                        "{}{} {}\n",
                        gauge.name(),
                        Self::format_labels(gauge.labels()),
                        gauge.get()
                    )));
                }
                Metric::Histogram(histogram) => {
                    output.push_str(&format!("# HELP {} {}\n", histogram.name(), histogram.help())));
                    output.push_str(&format!("# TYPE {} histogram\n", histogram.name())));

                    let buckets = histogram.get_buckets();
                    for bucket in &buckets {
                        let mut labels = histogram.labels().clone();
                        labels.insert("le".to_string(), bucket.upper_bound.to_string());
                        output.push_str(&format!(
                            "{}_bucket{} {}\n",
                            histogram.name(),
                            Self::format_labels(&labels),
                            bucket.count
                        )));
                    }

                    output.push_str(&format!(
                        "{}_sum{} {}\n",
                        histogram.name(),
                        Self::format_labels(histogram.labels()),
                        histogram.get_sum()
                    )));
                    output.push_str(&format!(
                        "{}_count{} {}\n",
                        histogram.name(),
                        Self::format_labels(histogram.labels()),
                        histogram.get_count()
                    )));
                }
                Metric::Summary(summary) => {
                    output.push_str(&format!("# HELP {} {}\n", summary.name(), summary.help())));
                    output.push_str(&format!("# TYPE {} summary\n", summary.name())));

                    for q in &[0.5, 0.9, 0.99] {
                        if let Some(value) = summary.get_quantile(*q) {
                            let mut labels = summary.labels().clone();
                            labels.insert("quantile".to_string(), q.to_string());
                            output.push_str(&format!(
                                "{}{} {}\n",
                                summary.name(),
                                Self::format_labels(&labels),
                                value
                            )));
                        }
                    }

                    output.push_str(&format!(
                        "{}_sum{} {}\n",
                        summary.name(),
                        Self::format_labels(summary.labels()),
                        summary.get_sum()
                    )));
                    output.push_str(&format!(
                        "{}_count{} {}\n",
                        summary.name(),
                        Self::format_labels(summary.labels()),
                        summary.get_count()
                    )));
                }
            }
            output.push('\n');
        }

        output
    }

    fn format_labels(labels: &HashMap<String, String>) -> String {
        if labels.is_empty() {
            return String::new();
        }

        let label_pairs: Vec<String> = labels
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, v))
            .collect());

        format!("{{{}}}", label_pairs.join(","))
    }
}

impl Default for MetricRegistry {
    fn default() -> Self {
        Self::new("rustydb")
    }
}

/// Metrics aggregator for rolling up metrics over time windows
pub struct MetricAggregator {
    window_size: Duration,
    aggregations: Arc<RwLock<HashMap<String, Vec<(SystemTime, f64)>>>>,
}

impl MetricAggregator {
    pub fn new(window_size: Duration) -> Self {
        Self {
            window_size,
            aggregations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn record(&self, metricname: impl Into<String>, value: f64) {
        let now = SystemTime::now());
        let name = metric_name.into();

        let mut aggs = self.aggregations.write();
        let entries = aggs.entry(name).or_insert_with(Vec::new);
        entries.push((now, value));

        // Clean up old entries
        let cutoff = now - self.window_size;
        entries.retain(|(timestamp, _)| timestamp >= &cutoff);
    }

    pub fn get_average(&self, metric_name: &str) -> Option<f64> {
        let aggs = self.aggregations.read();
        aggs.get(metric_name).and_then(|entries| {
            if entries.is_empty() {
                None
            } else {
                let sum: f64 = entries.iter().map(|(_, v)| v).sum();
                Some(sum / entries.len() as f64)
            }
        })
    }

    pub fn get_max(&self, metric_name: &str) -> Option<f64> {
        let aggs = self.aggregations.read();
        aggs.get(metric_name).and_then(|entries| {
            entries.iter().map(|(_, v)| v).cloned().fold(None, |max, v| {
                Some(max.map_or(v, |m| if v > m { v } else { m }))
            })
        })
    }

    pub fn get_min(&self, metric_name: &str) -> Option<f64> {
        let aggs = self.aggregations.read();
        aggs.get(metric_name).and_then(|entries| {
            entries.iter().map(|(_, v)| v).cloned().fold(None, |min, v| {
                Some(min.map_or(v, |m| if v < m { v } else { m }))
            })
        })
    }

    pub fn get_percentile(&self, metric_name: &str, percentile: f64) -> Option<f64> {
        let aggs = self.aggregations.read();
        aggs.get(metric_name).and_then(|entries| {
            if entries.is_empty() {
                return None;
            }

            let mut values: Vec<f64> = entries.iter().map(|(_, v)| *v).collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let index = (percentile * (values.len() - 1) as f64).round() as usize;
            Some(values[index])
        })
    }

    pub fn clear(&self, metric_name: &str) {
        self.aggregations.write().remove(metric_name);
    }

    pub fn clear_all(&self) {
        self.aggregations.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::new("test_counter", "Test counter metric");
        assert_eq!(counter.get(), 0.0);

        counter.inc();
        assert_eq!(counter.get(), 1.0);

        counter.inc_by(5.5);
        assert_eq!(counter.get(), 6.5);

        counter.reset();
        assert_eq!(counter.get(), 0.0);
    }

    #[test]
    fn test_gauge() {
        let gauge = Gauge::new("test_gauge", "Test gauge metric");
        assert_eq!(gauge.get(), 0.0);

        gauge.set(10.0);
        assert_eq!(gauge.get(), 10.0);

        gauge.inc();
        assert_eq!(gauge.get(), 11.0);

        gauge.dec();
        assert_eq!(gauge.get(), 10.0);

        gauge.add(5.0);
        assert_eq!(gauge.get(), 15.0);

        gauge.sub(3.0);
        assert_eq!(gauge.get(), 12.0);
    }

    #[test]
    fn test_histogram() {
        let histogram = Histogram::new(
            "test_histogram",
            "Test histogram metric",
            vec![1.0, 5.0, 10.0],
        );

        histogram.observe(0.5);
        histogram.observe(2.0);
        histogram.observe(7.0);
        histogram.observe(15.0);

        assert_eq!(histogram.get_count(), 4);
        assert_eq!(histogram.get_sum(), 24.5);

        let buckets = histogram.get_buckets();
        assert_eq!(buckets[0].count, 1); // <= 1.0
        assert_eq!(buckets[1].count, 2); // <= 5.0
        assert_eq!(buckets[2].count, 3); // <= 10.0
        assert_eq!(buckets[3].count, 4); // <= +Inf
    }

    #[test]
    fn test_registry() {
        let registry = MetricRegistry::new("test");

        let counter = registry.register_counter("requests", "Total requests");
        counter.inc_by(10.0);

        let gauge = registry.register_gauge("connections", "Active connections");
        gauge.set(5.0);

        let metrics = registry.list_metrics();
        assert_eq!(metrics.len(), 2);

        let prometheus_output = registry.expose_prometheus();
        assert!(prometheus_output.contains("test_requests"));
        assert!(prometheus_output.contains("test_connections"));
    }

    #[test]
    fn test_aggregator() {
        let aggregator = MetricAggregator::new(Duration::from_secs(60));

        aggregator.record("latency", 10.0);
        aggregator.record("latency", 20.0);
        aggregator.record("latency", 30.0);

        assert_eq!(aggregator.get_average("latency"), Some(20.0));
        assert_eq!(aggregator.get_max("latency"), Some(30.0));
        assert_eq!(aggregator.get_min("latency"), Some(10.0));
        assert_eq!(aggregator.get_percentile("latency", 0.5), Some(20.0));
    }
}


