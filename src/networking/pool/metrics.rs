// # Pool Metrics
//
// Comprehensive metrics collection and monitoring for connection pools.
// Tracks pool health, performance, and resource utilization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use tokio::sync::RwLock;

/// Global pool metrics aggregator
pub struct PoolMetrics {
    /// Total connections created across all pools
    total_connections_created: AtomicU64,

    /// Total connections closed across all pools
    total_connections_closed: AtomicU64,

    /// Total acquire operations
    total_acquires: AtomicU64,

    /// Total failed acquire operations
    total_failed_acquires: AtomicU64,

    /// Total acquire wait time (milliseconds)
    total_acquire_wait_ms: AtomicU64,

    /// Maximum acquire wait time (milliseconds)
    max_acquire_wait_ms: AtomicU64,

    /// Total pool exhaustion events
    total_exhaustions: AtomicU64,

    /// Histogram of acquire times
    acquire_histogram: Arc<RwLock<Histogram>>,

    /// Per-pool metrics
    pool_metrics: Arc<RwLock<HashMap<String, PoolMetricsSnapshot>>>,

    /// Metrics start time
    start_time: Instant,
}

impl PoolMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            total_connections_created: AtomicU64::new(0),
            total_connections_closed: AtomicU64::new(0),
            total_acquires: AtomicU64::new(0),
            total_failed_acquires: AtomicU64::new(0),
            total_acquire_wait_ms: AtomicU64::new(0),
            max_acquire_wait_ms: AtomicU64::new(0),
            total_exhaustions: AtomicU64::new(0),
            acquire_histogram: Arc::new(RwLock::new(Histogram::new())),
            pool_metrics: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    /// Record a connection creation
    pub fn record_connection_created(&self) {
        self.total_connections_created
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a connection closure
    pub fn record_connection_closed(&self) {
        self.total_connections_closed
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful acquire operation
    pub fn record_acquire(&self, wait_time_ms: u64) {
        self.total_acquires.fetch_add(1, Ordering::Relaxed);
        self.total_acquire_wait_ms
            .fetch_add(wait_time_ms, Ordering::Relaxed);

        // Update maximum
        let mut current_max = self.max_acquire_wait_ms.load(Ordering::Relaxed);
        while wait_time_ms > current_max {
            match self.max_acquire_wait_ms.compare_exchange(
                current_max,
                wait_time_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }

        // Update histogram
        tokio::spawn({
            let histogram = Arc::clone(&self.acquire_histogram);
            async move {
                let mut hist = histogram.write().await;
                hist.record(wait_time_ms);
            }
        });
    }

    /// Record a failed acquire operation
    pub fn record_failed_acquire(&self) {
        self.total_failed_acquires.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a pool exhaustion event
    pub fn record_exhaustion(&self) {
        self.total_exhaustions.fetch_add(1, Ordering::Relaxed);
    }

    /// Update per-pool metrics
    pub async fn update_pool_metrics(&self, node_id: String, snapshot: PoolMetricsSnapshot) {
        let mut metrics = self.pool_metrics.write().await;
        metrics.insert(node_id, snapshot);
    }

    /// Get global statistics
    pub async fn global_stats(&self) -> GlobalPoolStats {
        let acquire_histogram = self.acquire_histogram.read().await;

        GlobalPoolStats {
            total_connections_created: self.total_connections_created.load(Ordering::Relaxed),
            total_connections_closed: self.total_connections_closed.load(Ordering::Relaxed),
            active_connections: self
                .total_connections_created
                .load(Ordering::Relaxed)
                .saturating_sub(self.total_connections_closed.load(Ordering::Relaxed)),
            total_acquires: self.total_acquires.load(Ordering::Relaxed),
            total_failed_acquires: self.total_failed_acquires.load(Ordering::Relaxed),
            avg_acquire_wait_ms: self.avg_acquire_wait_ms(),
            max_acquire_wait_ms: self.max_acquire_wait_ms.load(Ordering::Relaxed),
            p50_acquire_wait_ms: acquire_histogram.percentile(0.5),
            p95_acquire_wait_ms: acquire_histogram.percentile(0.95),
            p99_acquire_wait_ms: acquire_histogram.percentile(0.99),
            total_exhaustions: self.total_exhaustions.load(Ordering::Relaxed),
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }

    /// Get per-pool statistics
    pub async fn pool_stats(&self) -> HashMap<String, PoolMetricsSnapshot> {
        let metrics = self.pool_metrics.read().await;
        metrics.clone()
    }

    /// Calculate average acquire wait time
    fn avg_acquire_wait_ms(&self) -> f64 {
        let total = self.total_acquire_wait_ms.load(Ordering::Relaxed);
        let count = self.total_acquires.load(Ordering::Relaxed);

        if count > 0 {
            total as f64 / count as f64
        } else {
            0.0
        }
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        self.total_connections_created.store(0, Ordering::Relaxed);
        self.total_connections_closed.store(0, Ordering::Relaxed);
        self.total_acquires.store(0, Ordering::Relaxed);
        self.total_failed_acquires.store(0, Ordering::Relaxed);
        self.total_acquire_wait_ms.store(0, Ordering::Relaxed);
        self.max_acquire_wait_ms.store(0, Ordering::Relaxed);
        self.total_exhaustions.store(0, Ordering::Relaxed);

        let mut histogram = self.acquire_histogram.write().await;
        histogram.reset();

        let mut metrics = self.pool_metrics.write().await;
        metrics.clear();
    }
}

impl Default for PoolMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Global pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalPoolStats {
    /// Total connections created
    pub total_connections_created: u64,

    /// Total connections closed
    pub total_connections_closed: u64,

    /// Active connections
    pub active_connections: u64,

    /// Total acquire operations
    pub total_acquires: u64,

    /// Total failed acquires
    pub total_failed_acquires: u64,

    /// Average acquire wait time (ms)
    pub avg_acquire_wait_ms: f64,

    /// Maximum acquire wait time (ms)
    pub max_acquire_wait_ms: u64,

    /// 50th percentile acquire time (ms)
    pub p50_acquire_wait_ms: u64,

    /// 95th percentile acquire time (ms)
    pub p95_acquire_wait_ms: u64,

    /// 99th percentile acquire time (ms)
    pub p99_acquire_wait_ms: u64,

    /// Total exhaustion events
    pub total_exhaustions: u64,

    /// Uptime in seconds
    pub uptime_secs: u64,
}

/// Per-pool metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetricsSnapshot {
    /// Node identifier
    pub node_id: String,

    /// Total connections in pool
    pub total_connections: usize,

    /// Active connections
    pub active_connections: usize,

    /// Idle connections
    pub idle_connections: usize,

    /// Pending requests
    pub pending_requests: usize,

    /// Pool utilization (0.0 - 1.0)
    pub utilization: f64,

    /// Timestamp of snapshot
    pub timestamp: SystemTime,
}

/// Connection-level metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionMetrics {
    /// Connection identifier
    pub connection_id: u64,

    /// Bytes sent
    pub bytes_sent: u64,

    /// Bytes received
    pub bytes_received: u64,

    /// Requests processed
    pub requests_processed: u64,

    /// Errors encountered
    pub errors: u64,

    /// Connection uptime (seconds)
    pub uptime_secs: u64,

    /// Active streams (for multiplexed connections)
    pub active_streams: usize,

    /// Total streams created
    pub streams_created: u64,

    /// Total streams closed
    pub streams_closed: u64,
}

impl ConnectionMetrics {
    /// Create new connection metrics
    pub fn new(connection_id: u64) -> Self {
        Self {
            connection_id,
            ..Default::default()
        }
    }

    /// Record bytes sent
    pub fn record_bytes_sent(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
    }

    /// Record bytes received
    pub fn record_bytes_received(&mut self, bytes: u64) {
        self.bytes_received += bytes;
    }

    /// Record request processed
    pub fn record_request(&mut self) {
        self.requests_processed += 1;
    }

    /// Record error
    pub fn record_error(&mut self) {
        self.errors += 1;
    }
}

/// Stream-level metrics (for multiplexed connections)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamMetrics {
    /// Stream identifier
    pub stream_id: u32,

    /// Bytes sent on this stream
    pub bytes_sent: u64,

    /// Bytes received on this stream
    pub bytes_received: u64,

    /// Messages sent
    pub messages_sent: u64,

    /// Messages received
    pub messages_received: u64,

    /// Stream age (seconds)
    pub age_secs: u64,

    /// Stream priority
    pub priority: u8,
}

impl StreamMetrics {
    /// Create new stream metrics
    pub fn new(stream_id: u32, priority: u8) -> Self {
        Self {
            stream_id,
            priority,
            ..Default::default()
        }
    }
}

/// Simple histogram for latency tracking
struct Histogram {
    /// Buckets for different latency ranges
    buckets: Vec<(u64, u64)>, // (upper_bound_ms, count)

    /// Total samples
    total_samples: u64,

    /// All samples (for percentile calculation)
    samples: Vec<u64>,
}

impl Histogram {
    /// Create a new histogram
    fn new() -> Self {
        // Buckets: 0-1ms, 1-5ms, 5-10ms, 10-50ms, 50-100ms, 100-500ms, 500-1000ms, 1000+ms
        let buckets = vec![
            (1, 0),
            (5, 0),
            (10, 0),
            (50, 0),
            (100, 0),
            (500, 0),
            (1000, 0),
            (u64::MAX, 0),
        ];

        Self {
            buckets,
            total_samples: 0,
            samples: Vec::new(),
        }
    }

    /// Record a value
    fn record(&mut self, value: u64) {
        self.total_samples += 1;
        self.samples.push(value);

        // Keep only last 10000 samples
        if self.samples.len() > 10000 {
            self.samples.remove(0);
        }

        // Update buckets
        for (upper_bound, count) in &mut self.buckets {
            if value <= *upper_bound {
                *count += 1;
                break;
            }
        }
    }

    /// Calculate percentile
    fn percentile(&self, p: f64) -> u64 {
        if self.samples.is_empty() {
            return 0;
        }

        let mut sorted = self.samples.clone();
        sorted.sort_unstable();

        let index = ((p * sorted.len() as f64) as usize).min(sorted.len() - 1);
        sorted[index]
    }

    /// Reset the histogram
    fn reset(&mut self) {
        self.total_samples = 0;
        self.samples.clear();

        for (_, count) in &mut self.buckets {
            *count = 0;
        }
    }
}

/// Metrics exporter for monitoring systems
pub struct MetricsExporter {
    /// Metrics reference
    metrics: Arc<PoolMetrics>,

    /// Export format
    format: ExportFormat,
}

impl MetricsExporter {
    /// Create a new metrics exporter
    pub fn new(metrics: Arc<PoolMetrics>, format: ExportFormat) -> Self {
        Self { metrics, format }
    }

    /// Export metrics in the configured format
    pub async fn export(&self) -> String {
        match self.format {
            ExportFormat::Json => self.export_json().await,
            ExportFormat::Prometheus => self.export_prometheus().await,
            ExportFormat::Text => self.export_text().await,
        }
    }

    /// Export as JSON
    async fn export_json(&self) -> String {
        let stats = self.metrics.global_stats().await;
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    }

    /// Export in Prometheus format
    async fn export_prometheus(&self) -> String {
        let stats = self.metrics.global_stats().await;

        format!(
            "# HELP pool_connections_created_total Total connections created\n\
             # TYPE pool_connections_created_total counter\n\
             pool_connections_created_total {}\n\
             \n\
             # HELP pool_connections_closed_total Total connections closed\n\
             # TYPE pool_connections_closed_total counter\n\
             pool_connections_closed_total {}\n\
             \n\
             # HELP pool_connections_active Active connections\n\
             # TYPE pool_connections_active gauge\n\
             pool_connections_active {}\n\
             \n\
             # HELP pool_acquire_wait_ms_avg Average acquire wait time in milliseconds\n\
             # TYPE pool_acquire_wait_ms_avg gauge\n\
             pool_acquire_wait_ms_avg {}\n\
             \n\
             # HELP pool_acquire_wait_ms_max Maximum acquire wait time in milliseconds\n\
             # TYPE pool_acquire_wait_ms_max gauge\n\
             pool_acquire_wait_ms_max {}\n\
             \n\
             # HELP pool_exhaustions_total Total pool exhaustion events\n\
             # TYPE pool_exhaustions_total counter\n\
             pool_exhaustions_total {}\n",
            stats.total_connections_created,
            stats.total_connections_closed,
            stats.active_connections,
            stats.avg_acquire_wait_ms,
            stats.max_acquire_wait_ms,
            stats.total_exhaustions,
        )
    }

    /// Export as plain text
    async fn export_text(&self) -> String {
        let stats = self.metrics.global_stats().await;

        format!(
            "Pool Metrics:\n\
             =============\n\
             Total Connections Created: {}\n\
             Total Connections Closed: {}\n\
             Active Connections: {}\n\
             Total Acquires: {}\n\
             Failed Acquires: {}\n\
             Avg Acquire Wait (ms): {:.2}\n\
             Max Acquire Wait (ms): {}\n\
             P50 Acquire Wait (ms): {}\n\
             P95 Acquire Wait (ms): {}\n\
             P99 Acquire Wait (ms): {}\n\
             Total Exhaustions: {}\n\
             Uptime (seconds): {}\n",
            stats.total_connections_created,
            stats.total_connections_closed,
            stats.active_connections,
            stats.total_acquires,
            stats.total_failed_acquires,
            stats.avg_acquire_wait_ms,
            stats.max_acquire_wait_ms,
            stats.p50_acquire_wait_ms,
            stats.p95_acquire_wait_ms,
            stats.p99_acquire_wait_ms,
            stats.total_exhaustions,
            stats.uptime_secs,
        )
    }
}

/// Export format for metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON format
    Json,

    /// Prometheus format
    Prometheus,

    /// Plain text format
    Text,
}

impl Default for ExportFormat {
    fn default() -> Self {
        ExportFormat::Json
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_metrics_creation() {
        let metrics = PoolMetrics::new();
        assert_eq!(metrics.total_connections_created.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_connection_created() {
        let metrics = PoolMetrics::new();
        metrics.record_connection_created();
        assert_eq!(metrics.total_connections_created.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_record_acquire() {
        let metrics = PoolMetrics::new();
        metrics.record_acquire(100);

        assert_eq!(metrics.total_acquires.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.total_acquire_wait_ms.load(Ordering::Relaxed), 100);
        assert_eq!(metrics.max_acquire_wait_ms.load(Ordering::Relaxed), 100);
    }

    #[tokio::test]
    async fn test_global_stats() {
        let metrics = PoolMetrics::new();
        metrics.record_connection_created();
        metrics.record_acquire(50);

        let stats = metrics.global_stats().await;
        assert_eq!(stats.total_connections_created, 1);
        assert_eq!(stats.total_acquires, 1);
        assert_eq!(stats.avg_acquire_wait_ms, 50.0);
    }

    #[test]
    fn test_histogram() {
        let mut histogram = Histogram::new();

        histogram.record(5);
        histogram.record(10);
        histogram.record(15);
        histogram.record(20);
        histogram.record(100);

        assert_eq!(histogram.total_samples, 5);

        let p50 = histogram.percentile(0.5);
        assert!(p50 >= 10 && p50 <= 15);
    }

    #[test]
    fn test_connection_metrics() {
        let mut metrics = ConnectionMetrics::new(1);

        metrics.record_bytes_sent(1024);
        metrics.record_bytes_received(2048);
        metrics.record_request();
        metrics.record_error();

        assert_eq!(metrics.bytes_sent, 1024);
        assert_eq!(metrics.bytes_received, 2048);
        assert_eq!(metrics.requests_processed, 1);
        assert_eq!(metrics.errors, 1);
    }

    #[test]
    fn test_stream_metrics() {
        let metrics = StreamMetrics::new(1, 2);

        assert_eq!(metrics.stream_id, 1);
        assert_eq!(metrics.priority, 2);
    }

    #[tokio::test]
    async fn test_metrics_exporter_json() {
        let metrics = Arc::new(PoolMetrics::new());
        metrics.record_connection_created();

        let exporter = MetricsExporter::new(metrics, ExportFormat::Json);
        let output = exporter.export().await;

        assert!(output.contains("total_connections_created"));
    }

    #[tokio::test]
    async fn test_metrics_reset() {
        let metrics = PoolMetrics::new();
        metrics.record_connection_created();
        metrics.record_acquire(100);

        metrics.reset().await;

        assert_eq!(metrics.total_connections_created.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.total_acquires.load(Ordering::Relaxed), 0);
    }
}
