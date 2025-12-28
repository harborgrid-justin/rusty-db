// Metrics Aggregation System
// Time-series aggregation, rollup policies, metric retention management, downsampling

use crate::error::{DbError, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

// SAFETY: Maximum aggregation buckets and rollup policies to prevent OOM
const MAX_AGGREGATION_BUCKETS: usize = 100_000;
const MAX_ROLLUP_POLICIES: usize = 1_000;
const MAX_RAW_DATAPOINTS_PER_METRIC: usize = 10_000;

// Aggregation interval for downsampling
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AggregationInterval {
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
    Days(u64),
}

impl AggregationInterval {
    pub fn to_duration(&self) -> Duration {
        match self {
            Self::Seconds(n) => Duration::from_secs(*n),
            Self::Minutes(n) => Duration::from_secs(*n * 60),
            Self::Hours(n) => Duration::from_secs(*n * 3600),
            Self::Days(n) => Duration::from_secs(*n * 86400),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Self::Seconds(n) => format!("{}s", n),
            Self::Minutes(n) => format!("{}m", n),
            Self::Hours(n) => format!("{}h", n),
            Self::Days(n) => format!("{}d", n),
        }
    }
}

// Aggregation function for downsampling
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AggregationMethod {
    Avg,
    Sum,
    Min,
    Max,
    Count,
    First,
    Last,
}

// Aggregated data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedDataPoint {
    pub timestamp: SystemTime,
    pub value: f64,
    pub count: u64,           // Number of raw points aggregated
    pub min: f64,
    pub max: f64,
    pub sum: f64,
}

impl AggregatedDataPoint {
    pub fn new(timestamp: SystemTime, value: f64) -> Self {
        Self {
            timestamp,
            value,
            count: 1,
            min: value,
            max: value,
            sum: value,
        }
    }

    pub fn from_values(timestamp: SystemTime, values: &[f64]) -> Self {
        if values.is_empty() {
            return Self::new(timestamp, 0.0);
        }

        let sum: f64 = values.iter().sum();
        let avg = sum / values.len() as f64;
        let min = values.iter().copied().fold(f64::INFINITY, f64::min);
        let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        Self {
            timestamp,
            value: avg,
            count: values.len() as u64,
            min,
            max,
            sum,
        }
    }

    pub fn apply_method(&self, method: AggregationMethod) -> f64 {
        match method {
            AggregationMethod::Avg => self.value,
            AggregationMethod::Sum => self.sum,
            AggregationMethod::Min => self.min,
            AggregationMethod::Max => self.max,
            AggregationMethod::Count => self.count as f64,
            AggregationMethod::First | AggregationMethod::Last => self.value,
        }
    }
}

// Retention policy for metric data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub name: String,
    pub raw_retention: Duration,           // How long to keep raw data
    pub aggregated_retention: Duration,    // How long to keep aggregated data
    pub aggregation_interval: AggregationInterval,
    pub aggregation_method: AggregationMethod,
}

impl RetentionPolicy {
    pub fn new(
        name: impl Into<String>,
        raw_retention: Duration,
        aggregated_retention: Duration,
        aggregation_interval: AggregationInterval,
        aggregation_method: AggregationMethod,
    ) -> Self {
        Self {
            name: name.into(),
            raw_retention,
            aggregated_retention,
            aggregation_interval,
            aggregation_method,
        }
    }

    // Standard policies
    pub fn short_term() -> Self {
        Self::new(
            "short_term",
            Duration::from_secs(3600),      // 1 hour raw
            Duration::from_secs(86400),     // 24 hours aggregated
            AggregationInterval::Minutes(1),
            AggregationMethod::Avg,
        )
    }

    pub fn medium_term() -> Self {
        Self::new(
            "medium_term",
            Duration::from_secs(86400),     // 24 hours raw
            Duration::from_secs(604800),    // 7 days aggregated
            AggregationInterval::Minutes(5),
            AggregationMethod::Avg,
        )
    }

    pub fn long_term() -> Self {
        Self::new(
            "long_term",
            Duration::from_secs(604800),    // 7 days raw
            Duration::from_secs(2592000),   // 30 days aggregated
            AggregationInterval::Hours(1),
            AggregationMethod::Avg,
        )
    }
}

// Rollup policy for cascading aggregations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollupPolicy {
    pub id: String,
    pub source_metric: String,
    pub target_metric: String,
    pub interval: AggregationInterval,
    pub method: AggregationMethod,
    pub retention: Duration,
    pub enabled: bool,
}

impl RollupPolicy {
    pub fn new(
        id: impl Into<String>,
        source_metric: impl Into<String>,
        target_metric: impl Into<String>,
        interval: AggregationInterval,
        method: AggregationMethod,
    ) -> Self {
        Self {
            id: id.into(),
            source_metric: source_metric.into(),
            target_metric: target_metric.into(),
            interval,
            method,
            retention: Duration::from_secs(2592000), // 30 days default
            enabled: true,
        }
    }

    pub fn with_retention(mut self, retention: Duration) -> Self {
        self.retention = retention;
        self
    }
}

// Time-series bucket for aggregation
#[derive(Debug, Clone)]
struct TimeBucket {
    start_time: SystemTime,
    end_time: SystemTime,
    values: Vec<f64>,
}

impl TimeBucket {
    fn new(start_time: SystemTime, interval: Duration) -> Self {
        Self {
            start_time,
            end_time: start_time + interval,
            values: Vec::new(),
        }
    }

    fn add_value(&mut self, value: f64) {
        self.values.push(value);
    }

    fn to_aggregated(&self, method: AggregationMethod) -> Option<AggregatedDataPoint> {
        if self.values.is_empty() {
            return None;
        }

        let mut agg = AggregatedDataPoint::from_values(self.start_time, &self.values);
        agg.value = agg.apply_method(method);
        Some(agg)
    }
}

// Metrics aggregator
pub struct MetricsAggregator {
    // Raw data points (before aggregation)
    raw_data: Arc<RwLock<HashMap<String, BTreeMap<SystemTime, f64>>>>,

    // Aggregated data (after downsampling)
    aggregated_data: Arc<RwLock<HashMap<String, BTreeMap<SystemTime, AggregatedDataPoint>>>>,

    // Retention policies per metric
    retention_policies: Arc<RwLock<HashMap<String, RetentionPolicy>>>,

    // Rollup policies
    rollup_policies: Arc<RwLock<HashMap<String, RollupPolicy>>>,

    // Statistics
    stats: Arc<RwLock<AggregatorStats>>,
}

impl MetricsAggregator {
    pub fn new() -> Self {
        Self {
            raw_data: Arc::new(RwLock::new(HashMap::new())),
            aggregated_data: Arc::new(RwLock::new(HashMap::new())),
            retention_policies: Arc::new(RwLock::new(HashMap::new())),
            rollup_policies: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(AggregatorStats::default())),
        }
    }

    // Add a raw data point
    pub fn add_data_point(&self, metric_name: &str, timestamp: SystemTime, value: f64) -> Result<()> {
        let mut raw_data = self.raw_data.write();
        let series = raw_data.entry(metric_name.to_string())
            .or_insert_with(BTreeMap::new);

        // Enforce limit on raw data points
        if series.len() >= MAX_RAW_DATAPOINTS_PER_METRIC {
            // Remove oldest entry
            if let Some(&oldest_key) = series.keys().next() {
                series.remove(&oldest_key);
            }
        }

        series.insert(timestamp, value);

        self.stats.write().raw_points_added += 1;
        Ok(())
    }

    // Set retention policy for a metric
    pub fn set_retention_policy(&self, metric_name: &str, policy: RetentionPolicy) {
        self.retention_policies.write()
            .insert(metric_name.to_string(), policy);
    }

    // Get retention policy for a metric
    pub fn get_retention_policy(&self, metric_name: &str) -> Option<RetentionPolicy> {
        self.retention_policies.read().get(metric_name).cloned()
    }

    // Add rollup policy
    pub fn add_rollup_policy(&self, policy: RollupPolicy) -> Result<()> {
        let mut policies = self.rollup_policies.write();

        if policies.len() >= MAX_ROLLUP_POLICIES {
            return Err(DbError::LimitExceeded(
                "Maximum number of rollup policies reached".to_string(),
            ));
        }

        policies.insert(policy.id.clone(), policy);
        Ok(())
    }

    // Remove rollup policy
    pub fn remove_rollup_policy(&self, policy_id: &str) -> Result<()> {
        self.rollup_policies.write().remove(policy_id);
        Ok(())
    }

    // Aggregate raw data for a metric
    pub fn aggregate_metric(
        &self,
        metric_name: &str,
        interval: AggregationInterval,
        method: AggregationMethod,
    ) -> Result<Vec<AggregatedDataPoint>> {
        let raw_data = self.raw_data.read();

        let series = raw_data.get(metric_name)
            .ok_or_else(|| DbError::NotFound(format!("Metric not found: {}", metric_name)))?;

        if series.is_empty() {
            return Ok(Vec::new());
        }

        let interval_duration = interval.to_duration();
        let mut buckets: BTreeMap<SystemTime, TimeBucket> = BTreeMap::new();

        // Assign data points to buckets
        for (&timestamp, &value) in series.iter() {
            let bucket_start = self.align_timestamp(timestamp, interval_duration);

            buckets.entry(bucket_start)
                .or_insert_with(|| TimeBucket::new(bucket_start, interval_duration))
                .add_value(value);
        }

        // Convert buckets to aggregated data points
        let aggregated: Vec<AggregatedDataPoint> = buckets.values()
            .filter_map(|bucket| bucket.to_aggregated(method))
            .collect();

        self.stats.write().aggregations_performed += 1;

        Ok(aggregated)
    }

    // Execute all rollup policies
    pub fn execute_rollups(&self) -> Result<usize> {
        let policies: Vec<RollupPolicy> = self.rollup_policies.read()
            .values()
            .filter(|p| p.enabled)
            .cloned()
            .collect();

        let mut executed = 0;

        for policy in policies {
            if let Ok(aggregated) = self.aggregate_metric(
                &policy.source_metric,
                policy.interval,
                policy.method,
            ) {
                // Store aggregated data
                let mut agg_data = self.aggregated_data.write();
                let target_series = agg_data.entry(policy.target_metric.clone())
                    .or_insert_with(BTreeMap::new);

                for point in aggregated {
                    target_series.insert(point.timestamp, point);
                }

                executed += 1;
            }
        }

        Ok(executed)
    }

    // Clean up old data based on retention policies
    pub fn apply_retention(&self) -> Result<RetentionStats> {
        let mut stats = RetentionStats::default();
        let now = SystemTime::now();

        // Apply retention to raw data
        {
            let policies = self.retention_policies.read();
            let mut raw_data = self.raw_data.write();

            for (metric_name, series) in raw_data.iter_mut() {
                if let Some(policy) = policies.get(metric_name) {
                    let cutoff = now - policy.raw_retention;

                    let initial_count = series.len();
                    series.retain(|&timestamp, _| timestamp >= cutoff);
                    stats.raw_points_deleted += initial_count - series.len();
                }
            }
        }

        // Apply retention to aggregated data
        {
            let policies = self.retention_policies.read();
            let mut agg_data = self.aggregated_data.write();

            for (metric_name, series) in agg_data.iter_mut() {
                if let Some(policy) = policies.get(metric_name) {
                    let cutoff = now - policy.aggregated_retention;

                    let initial_count = series.len();
                    series.retain(|&timestamp, _| timestamp >= cutoff);
                    stats.aggregated_points_deleted += initial_count - series.len();
                }
            }
        }

        self.stats.write().retention_runs += 1;

        Ok(stats)
    }

    // Get raw data for a metric
    pub fn get_raw_data(
        &self,
        metric_name: &str,
        start: SystemTime,
        end: SystemTime,
    ) -> Result<Vec<(SystemTime, f64)>> {
        let raw_data = self.raw_data.read();

        let series = raw_data.get(metric_name)
            .ok_or_else(|| DbError::NotFound(format!("Metric not found: {}", metric_name)))?;

        let data: Vec<(SystemTime, f64)> = series.range(start..=end)
            .map(|(&ts, &val)| (ts, val))
            .collect();

        Ok(data)
    }

    // Get aggregated data for a metric
    pub fn get_aggregated_data(
        &self,
        metric_name: &str,
        start: SystemTime,
        end: SystemTime,
    ) -> Result<Vec<AggregatedDataPoint>> {
        let agg_data = self.aggregated_data.read();

        let series = agg_data.get(metric_name)
            .ok_or_else(|| DbError::NotFound(format!("Metric not found: {}", metric_name)))?;

        let data: Vec<AggregatedDataPoint> = series.range(start..=end)
            .map(|(_, point)| point.clone())
            .collect();

        Ok(data)
    }

    // Align timestamp to bucket boundary
    fn align_timestamp(&self, timestamp: SystemTime, interval: Duration) -> SystemTime {
        let duration_since_epoch = timestamp.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);

        let interval_secs = interval.as_secs();
        if interval_secs == 0 {
            return timestamp;
        }

        let aligned_secs = (duration_since_epoch.as_secs() / interval_secs) * interval_secs;

        SystemTime::UNIX_EPOCH + Duration::from_secs(aligned_secs)
    }

    // Get aggregator statistics
    pub fn get_stats(&self) -> AggregatorStats {
        self.stats.read().clone()
    }

    // Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.write() = AggregatorStats::default();
    }
}

impl Default for MetricsAggregator {
    fn default() -> Self {
        Self::new()
    }
}

// Aggregator statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregatorStats {
    pub raw_points_added: u64,
    pub aggregations_performed: u64,
    pub retention_runs: u64,
}

// Retention cleanup statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetentionStats {
    pub raw_points_deleted: usize,
    pub aggregated_points_deleted: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregation_interval() {
        let interval = AggregationInterval::Minutes(5);
        assert_eq!(interval.to_duration(), Duration::from_secs(300));
        assert_eq!(interval.name(), "5m");
    }

    #[test]
    fn test_aggregated_data_point() {
        let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let point = AggregatedDataPoint::from_values(SystemTime::now(), &values);

        assert_eq!(point.value, 30.0); // Average
        assert_eq!(point.min, 10.0);
        assert_eq!(point.max, 50.0);
        assert_eq!(point.sum, 150.0);
        assert_eq!(point.count, 5);
    }

    #[test]
    fn test_retention_policies() {
        let short = RetentionPolicy::short_term();
        assert_eq!(short.raw_retention.as_secs(), 3600);

        let medium = RetentionPolicy::medium_term();
        assert_eq!(medium.raw_retention.as_secs(), 86400);

        let long = RetentionPolicy::long_term();
        assert_eq!(long.raw_retention.as_secs(), 604800);
    }

    #[test]
    fn test_metrics_aggregator() {
        let aggregator = MetricsAggregator::new();

        // Add data points
        let now = SystemTime::now();
        assert!(aggregator.add_data_point("cpu_usage", now, 45.5).is_ok());
        assert!(aggregator.add_data_point("cpu_usage", now + Duration::from_secs(60), 50.2).is_ok());

        // Set retention policy
        aggregator.set_retention_policy("cpu_usage", RetentionPolicy::short_term());

        // Verify policy was set
        let policy = aggregator.get_retention_policy("cpu_usage");
        assert!(policy.is_some());

        // Get stats
        let stats = aggregator.get_stats();
        assert_eq!(stats.raw_points_added, 2);
    }

    #[test]
    fn test_rollup_policy() {
        let policy = RollupPolicy::new(
            "rollup1",
            "cpu_usage_raw",
            "cpu_usage_1m",
            AggregationInterval::Minutes(1),
            AggregationMethod::Avg,
        ).with_retention(Duration::from_secs(86400));

        assert_eq!(policy.source_metric, "cpu_usage_raw");
        assert_eq!(policy.target_metric, "cpu_usage_1m");
        assert!(policy.enabled);
    }
}
