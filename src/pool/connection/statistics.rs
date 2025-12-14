// Pool statistics and monitoring module
//
// This module provides comprehensive statistics and monitoring for connection pools including:
// - Real-time metrics collection
// - Performance analytics
// - Leak detection
// - Export formats (JSON, Prometheus, CSV)

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// Comprehensive pool statistics
pub struct PoolStatistics {
    // Connection metrics
    connections_created: AtomicU64,
    connections_destroyed: AtomicU64,
    connections_acquired: AtomicU64,
    connections_released: AtomicU64,

    // Timing metrics
    acquire_attempts: AtomicU64,
    acquire_successes: AtomicU64,
    acquire_failures: AtomicU64,
    acquire_timeouts: AtomicU64,
    total_acquire_time: AtomicU64, // in microseconds

    // Wait queue metrics
    _queue_additions: AtomicU64,
    _queue_removals: AtomicU64,

    // Error metrics
    validation_failures: AtomicU64,
    creation_failures: AtomicU64,

    // Leak detection
    leaks_detected: AtomicU64,

    // Histogram data for wait times
    wait_time_histogram: Arc<RwLock<WaitTimeHistogram>>,

    // Connection usage patterns
    usage_patterns: Arc<RwLock<UsagePatterns>>,

    // Efficiency metrics
    efficiency_metrics: Arc<RwLock<EfficiencyMetrics>>,
}

impl PoolStatistics {
    pub fn new() -> Self {
        Self {
            connections_created: AtomicU64::new(0),
            connections_destroyed: AtomicU64::new(0),
            connections_acquired: AtomicU64::new(0),
            connections_released: AtomicU64::new(0),
            acquire_attempts: AtomicU64::new(0),
            acquire_successes: AtomicU64::new(0),
            acquire_failures: AtomicU64::new(0),
            acquire_timeouts: AtomicU64::new(0),
            total_acquire_time: AtomicU64::new(0),
            _queue_additions: AtomicU64::new(0),
            _queue_removals: AtomicU64::new(0),
            validation_failures: AtomicU64::new(0),
            creation_failures: AtomicU64::new(0),
            leaks_detected: AtomicU64::new(0),
            wait_time_histogram: Arc::new(RwLock::new(WaitTimeHistogram::new())),
            usage_patterns: Arc::new(RwLock::new(UsagePatterns::new())),
            efficiency_metrics: Arc::new(RwLock::new(EfficiencyMetrics::new())),
        }
    }

    pub fn record_connection_created(&self) {
        self.connections_created.fetch_add(1, Ordering::SeqCst);
    }

    pub fn record_connection_destroyed(&self) {
        self.connections_destroyed.fetch_add(1, Ordering::SeqCst);
    }

    pub fn record_acquire_attempt(&self) {
        self.acquire_attempts.fetch_add(1, Ordering::SeqCst);
    }

    pub fn record_acquire_success(&self, duration: Duration) {
        self.acquire_successes.fetch_add(1, Ordering::SeqCst);
        self.connections_acquired.fetch_add(1, Ordering::SeqCst);

        let micros = duration.as_micros() as u64;
        self.total_acquire_time.fetch_add(micros, Ordering::SeqCst);

        self.wait_time_histogram.write().record(duration);
        self.usage_patterns.write().record_acquisition();
    }

    pub fn record_acquire_failure(&self) {
        self.acquire_failures.fetch_add(1, Ordering::SeqCst);
    }

    pub fn record_acquire_timeout(&self) {
        self.acquire_timeouts.fetch_add(1, Ordering::SeqCst);
    }

    pub fn record_connection_released(&self) {
        self.connections_released.fetch_add(1, Ordering::SeqCst);
        self.usage_patterns.write().record_release();
    }

    pub fn record_leak_detected(&self) {
        self.leaks_detected.fetch_add(1, Ordering::SeqCst);
    }

    pub fn snapshot(&self) -> PoolStats {
        let acquire_attempts = self.acquire_attempts.load(Ordering::SeqCst);
        let acquire_successes = self.acquire_successes.load(Ordering::SeqCst);
        let total_acquire_time = self.total_acquire_time.load(Ordering::SeqCst);

        PoolStats {
            connections_created: self.connections_created.load(Ordering::SeqCst),
            connections_destroyed: self.connections_destroyed.load(Ordering::SeqCst),
            connections_acquired: self.connections_acquired.load(Ordering::SeqCst),
            connections_released: self.connections_released.load(Ordering::SeqCst),
            active_connections: self
                .connections_acquired
                .load(Ordering::SeqCst)
                .saturating_sub(self.connections_released.load(Ordering::SeqCst)),
            acquire_attempts,
            acquire_successes,
            acquire_failures: self.acquire_failures.load(Ordering::SeqCst),
            acquire_timeouts: self.acquire_timeouts.load(Ordering::SeqCst),
            average_acquire_time: if acquire_successes > 0 {
                Duration::from_micros(total_acquire_time / acquire_successes)
            } else {
                Duration::ZERO
            },
            success_rate: if acquire_attempts > 0 {
                acquire_successes as f64 / acquire_attempts as f64
            } else {
                1.0
            },
            validation_failures: self.validation_failures.load(Ordering::SeqCst),
            creation_failures: self.creation_failures.load(Ordering::SeqCst),
            leaks_detected: self.leaks_detected.load(Ordering::SeqCst),
            wait_time_histogram: self.wait_time_histogram.read().snapshot(),
            usage_patterns: self.usage_patterns.read().snapshot(),
            efficiency_metrics: self.efficiency_metrics.read().snapshot(),
        }
    }
}

impl Default for PoolStatistics {
    fn default() -> Self {
        Self::new()
    }
}

// Pool statistics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub connections_created: u64,
    pub connections_destroyed: u64,
    pub connections_acquired: u64,
    pub connections_released: u64,
    pub active_connections: u64,
    pub acquire_attempts: u64,
    pub acquire_successes: u64,
    pub acquire_failures: u64,
    pub acquire_timeouts: u64,
    pub average_acquire_time: Duration,
    pub success_rate: f64,
    pub validation_failures: u64,
    pub creation_failures: u64,
    pub leaks_detected: u64,
    pub wait_time_histogram: HistogramSnapshot,
    pub usage_patterns: UsagePatternsSnapshot,
    pub efficiency_metrics: EfficiencyMetricsSnapshot,
}

// Wait time histogram
struct WaitTimeHistogram {
    buckets: BTreeMap<u64, u64>, // microseconds -> count
    total_samples: u64,
}

impl WaitTimeHistogram {
    fn new() -> Self {
        Self {
            buckets: BTreeMap::new(),
            total_samples: 0,
        }
    }

    fn record(&mut self, duration: Duration) {
        let micros = duration.as_micros() as u64;

        // Bucket into powers of 2
        let bucket = if micros == 0 {
            0
        } else {
            let log2 = 63 - micros.leading_zeros();
            1u64 << log2
        };

        *self.buckets.entry(bucket).or_insert(0) += 1;
        self.total_samples += 1;
    }

    fn snapshot(&self) -> HistogramSnapshot {
        HistogramSnapshot {
            buckets: self.buckets.clone(),
            total_samples: self.total_samples,
            percentiles: Percentiles::default(),
        }
    }
}

// Histogram snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramSnapshot {
    pub buckets: BTreeMap<u64, u64>,
    pub total_samples: u64,
    pub percentiles: Percentiles,
}

// Percentile values
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Percentiles {
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
}

// Connection usage patterns
struct UsagePatterns {
    acquisitions_by_hour: [u64; 24],
    releases_by_hour: [u64; 24],
    peak_hour: usize,
    #[allow(dead_code)]
    last_update: Instant,
}

impl UsagePatterns {
    fn new() -> Self {
        Self {
            acquisitions_by_hour: [0; 24],
            releases_by_hour: [0; 24],
            peak_hour: 0,
            last_update: Instant::now(),
        }
    }

    fn record_acquisition(&mut self) {
        let hour = self.current_hour();
        self.acquisitions_by_hour[hour] += 1;
        self.update_peak_hour();
    }

    fn record_release(&mut self) {
        let hour = self.current_hour();
        self.releases_by_hour[hour] += 1;
    }

    fn current_hour(&self) -> usize {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        ((now.as_secs() / 3600) % 24) as usize
    }

    fn update_peak_hour(&mut self) {
        if let Some((hour, _)) = self
            .acquisitions_by_hour
            .iter()
            .enumerate()
            .max_by_key(|(_, &count)| count)
        {
            self.peak_hour = hour;
        }
    }

    fn snapshot(&self) -> UsagePatternsSnapshot {
        UsagePatternsSnapshot {
            acquisitions_by_hour: self.acquisitions_by_hour,
            releases_by_hour: self.releases_by_hour,
            peak_hour: self.peak_hour,
        }
    }
}

// Usage patterns snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePatternsSnapshot {
    pub acquisitions_by_hour: [u64; 24],
    pub releases_by_hour: [u64; 24],
    pub peak_hour: usize,
}

// Pool efficiency metrics
struct EfficiencyMetrics {
    cache_hit_rate: f64,
    connection_reuse_rate: f64,
    pool_utilization: f64,
    #[allow(dead_code)]
    _last_calculated: Instant,
}

impl EfficiencyMetrics {
    fn new() -> Self {
        Self {
            cache_hit_rate: 0.0,
            connection_reuse_rate: 0.0,
            pool_utilization: 0.0,
            _last_calculated: Instant::now(),
        }
    }

    fn snapshot(&self) -> EfficiencyMetricsSnapshot {
        EfficiencyMetricsSnapshot {
            cache_hit_rate: self.cache_hit_rate,
            connection_reuse_rate: self.connection_reuse_rate,
            pool_utilization: self.pool_utilization,
        }
    }
}

// Efficiency metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfficiencyMetricsSnapshot {
    pub cache_hit_rate: f64,
    pub connection_reuse_rate: f64,
    pub pool_utilization: f64,
}

// Dashboard data snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub timestamp: SystemTime,
    pub active_connections: u64,
    pub total_connections: u64,
    pub success_rate: f64,
    pub average_wait_time: Duration,
    pub leaks_detected: u64,
    pub pool_efficiency: f64,
    pub queue_length: usize,
    pub peak_hour: usize,
}

// Leak information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakInfo {
    pub connection_id: u64,
    #[serde(skip, default = "Instant::now")]
    pub acquired_at: Instant,
    pub active_duration: Duration,
    #[serde(skip, default = "Instant::now")]
    pub detected_at: Instant,
}

// Leak detector
pub struct LeakDetector {
    threshold: Duration,
    #[allow(dead_code)]
    _check_interval: Duration,
    detected_leaks: Arc<RwLock<Vec<LeakInfo>>>,
}

impl LeakDetector {
    pub fn new(threshold: Duration, check_interval: Duration) -> Self {
        Self {
            threshold,
            _check_interval: check_interval,
            detected_leaks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn check_leaks(&self, active: &HashMap<u64, Instant>) {
        let now = Instant::now();
        let mut leaks = self.detected_leaks.write();

        for (&conn_id, &acquired_at) in active {
            let active_duration = now - acquired_at;
            if active_duration > self.threshold {
                let leak = LeakInfo {
                    connection_id: conn_id,
                    acquired_at,
                    active_duration,
                    detected_at: now,
                };

                // Only add if not already detected
                if !leaks.iter().any(|l| l.connection_id == conn_id) {
                    tracing::warn!("Connection leak detected: {:?}", leak);
                    leaks.push(leak);
                }
            }
        }
    }

    pub fn get_leaks(&self) -> Vec<LeakInfo> {
        self.detected_leaks.read().clone()
    }

    pub fn clear_leaks(&self) {
        self.detected_leaks.write().clear();
    }
}

// Pool size information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolSizeInfo {
    pub total: usize,
    pub idle: usize,
    pub active: usize,
}

// Export format for monitoring data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Prometheus,
    Csv,
}

// Dashboard provider for pool monitoring
pub struct DashboardProvider {
    statistics: Arc<PoolStatistics>,
}

impl DashboardProvider {
    pub fn new(statistics: Arc<PoolStatistics>) -> Self {
        Self { statistics }
    }

    pub fn get_dashboard_data(&self) -> DashboardData {
        let stats = self.statistics.snapshot();
        DashboardData {
            timestamp: SystemTime::now(),
            active_connections: stats.active_connections,
            total_connections: stats.connections_created - stats.connections_destroyed,
            success_rate: stats.success_rate,
            average_wait_time: stats.average_acquire_time,
            leaks_detected: stats.leaks_detected,
            pool_efficiency: stats.efficiency_metrics.pool_utilization,
            queue_length: 0, // Would come from wait queue
            peak_hour: stats.usage_patterns.peak_hour,
        }
    }
}

// Monitoring exporter for external monitoring systems
pub struct MonitoringExporter {
    statistics: Arc<PoolStatistics>,
    format: ExportFormat,
}

impl MonitoringExporter {
    pub fn new(statistics: Arc<PoolStatistics>, format: ExportFormat) -> Self {
        Self { statistics, format }
    }

    pub fn export(&self) -> String {
        let stats = self.statistics.snapshot();
        match self.format {
            ExportFormat::Json => serde_json::to_string_pretty(&stats).unwrap_or_default(),
            ExportFormat::Prometheus => self.to_prometheus(&stats),
            ExportFormat::Csv => self.to_csv(&stats),
        }
    }

    fn to_prometheus(&self, stats: &PoolStats) -> String {
        format!(
            "# TYPE pool_connections_created counter\npool_connections_created {}\n\
             # TYPE pool_connections_destroyed counter\npool_connections_destroyed {}\n\
             # TYPE pool_active_connections gauge\npool_active_connections {}\n\
             # TYPE pool_success_rate gauge\npool_success_rate {}\n",
            stats.connections_created,
            stats.connections_destroyed,
            stats.active_connections,
            stats.success_rate,
        )
    }

    fn to_csv(&self, stats: &PoolStats) -> String {
        format!(
            "metric,value\n\
             connections_created,{}\n\
             connections_destroyed,{}\n\
             active_connections,{}\n\
             success_rate,{}\n",
            stats.connections_created,
            stats.connections_destroyed,
            stats.active_connections,
            stats.success_rate,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_statistics() {
        let stats = PoolStatistics::new();
        stats.record_connection_created();
        stats.record_acquire_success(Duration::from_millis(10));

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.connections_created, 1);
        assert_eq!(snapshot.acquire_successes, 1);
    }
}
