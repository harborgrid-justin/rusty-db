// Monitoring Module
//
// Part of the comprehensive monitoring system for RustyDB

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use super::metrics_core::*;

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
        let recent_points: Vec<_> = points
            .iter()
            .filter(|(ts, _)| ts >= &cutoff)
            .map(|(_, v)| *v)
            .collect();

        if recent_points.is_empty() {
            return;
        }

        let count = recent_points.len() as u64;
        let sum: f64 = recent_points.iter().sum();
        let min = recent_points.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = recent_points
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
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
        aggregated
            .entry(window)
            .or_insert_with(Vec::new)
            .push(point);
    }

    pub fn get_aggregated(&self, window: AggregationWindow) -> Vec<AggregatedMetricPoint> {
        self.aggregated
            .read()
            .get(&window)
            .cloned()
            .unwrap_or_default()
    }
}

// Cardinality tracker for preventing metric explosion
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
        self.current_cardinality
            .read()
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

// Metric retention policy
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

// Core metrics registry
pub struct MetricsRegistry {
    metrics: Arc<RwLock<HashMap<MetricId, MetricType>>>,
    #[allow(dead_code)]
    aggregators: Arc<RwLock<HashMap<String, MetricAggregator>>>,
    cardinality_manager: Arc<Mutex<CardinalityManager>>,
    #[allow(dead_code)]
    retention_policy: RetentionPolicy,
    namespaces: Arc<RwLock<HashMap<String, MetricNamespace>>>,
}

impl MetricsRegistry {
    pub fn new(retention_policy: RetentionPolicy) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            aggregators: Arc::new(RwLock::new(HashMap::new())),
            cardinality_manager: Arc::new(Mutex::new(CardinalityManager::new(
                CardinalityEnforcement::Warn,
            ))),
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
        if let CardinalityCheckResult::Drop =
            self.cardinality_manager.lock().unwrap().check(&metric_id)
        {
            // Return a dummy counter that doesn't persist
            return Arc::new(CounterMetric::new("dropped"));
        }

        let mut metrics = self.metrics.write();
        let metric = metrics
            .entry(metric_id)
            .or_insert_with(|| MetricType::Counter(Arc::new(CounterMetric::new(help))));

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

        if let CardinalityCheckResult::Drop =
            self.cardinality_manager.lock().unwrap().check(&metric_id)
        {
            return Arc::new(GaugeMetric::new("dropped"));
        }

        let mut metrics = self.metrics.write();
        let metric = metrics
            .entry(metric_id)
            .or_insert_with(|| MetricType::Gauge(Arc::new(GaugeMetric::new(help))));

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

        if let CardinalityCheckResult::Drop =
            self.cardinality_manager.lock().unwrap().check(&metric_id)
        {
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
        metrics
            .iter()
            .map(|(k, v)| {
                let cloned_type = match v {
                    MetricType::Counter(c) => MetricType::Counter(c.clone()),
                    MetricType::Gauge(g) => MetricType::Gauge(g.clone()),
                    MetricType::Histogram(h) => MetricType::Histogram(h.clone()),
                    MetricType::Summary(s) => MetricType::Summary(s.clone()),
                };
                (k.clone(), cloned_type)
            })
            .collect()
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
