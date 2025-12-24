// Enterprise Metrics Collection
//
// Comprehensive metrics for monitoring enterprise optimization effectiveness.
// Provides real-time and historical metrics for all optimization subsystems.
//
// ## Metrics Categories
//
// - Transaction metrics (TPS, latency, lock contention)
// - Memory metrics (usage, fragmentation, pressure)
// - Buffer pool metrics (hit rate, evictions, dirty pages)
// - Query metrics (execution time, plan quality)
// - Replication metrics (lag, throughput)
// - Security metrics (threat detections, audit rate)

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Metric value types
#[derive(Debug, Clone)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
    Rate(f64), // per second
}

/// Metric metadata
#[derive(Debug, Clone)]
pub struct MetricMeta {
    pub name: String,
    pub description: String,
    pub unit: String,
    pub metric_type: MetricType,
}

/// Metric types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Rate,
}

/// Atomic counter metric
pub struct Counter {
    value: AtomicU64,
    meta: MetricMeta,
}

impl Counter {
    pub fn new(name: &str, description: &str, unit: &str) -> Self {
        Self {
            value: AtomicU64::new(0),
            meta: MetricMeta {
                name: name.to_string(),
                description: description.to_string(),
                unit: unit.to_string(),
                metric_type: MetricType::Counter,
            },
        }
    }

    #[inline]
    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn add(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    #[inline]
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn meta(&self) -> &MetricMeta {
        &self.meta
    }
}

/// Atomic gauge metric
pub struct Gauge {
    value: AtomicU64, // Store f64 as bits
    meta: MetricMeta,
}

impl Gauge {
    pub fn new(name: &str, description: &str, unit: &str) -> Self {
        Self {
            value: AtomicU64::new(0),
            meta: MetricMeta {
                name: name.to_string(),
                description: description.to_string(),
                unit: unit.to_string(),
                metric_type: MetricType::Gauge,
            },
        }
    }

    #[inline]
    pub fn set(&self, value: f64) {
        self.value.store(value.to_bits(), Ordering::Relaxed);
    }

    #[inline]
    pub fn get(&self) -> f64 {
        f64::from_bits(self.value.load(Ordering::Relaxed))
    }

    pub fn meta(&self) -> &MetricMeta {
        &self.meta
    }
}

/// Histogram metric for distributions
pub struct Histogram {
    buckets: Vec<AtomicU64>,
    bucket_bounds: Vec<f64>,
    sum: AtomicU64,
    count: AtomicU64,
    meta: MetricMeta,
}

impl Histogram {
    /// Create histogram with default latency buckets (microseconds)
    pub fn with_latency_buckets(name: &str, description: &str) -> Self {
        Self::new(
            name,
            description,
            "microseconds",
            vec![10.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0, f64::INFINITY],
        )
    }

    pub fn new(name: &str, description: &str, unit: &str, bounds: Vec<f64>) -> Self {
        let bucket_count = bounds.len();
        Self {
            buckets: (0..bucket_count).map(|_| AtomicU64::new(0)).collect(),
            bucket_bounds: bounds,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
            meta: MetricMeta {
                name: name.to_string(),
                description: description.to_string(),
                unit: unit.to_string(),
                metric_type: MetricType::Histogram,
            },
        }
    }

    #[inline]
    pub fn observe(&self, value: f64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(value.to_bits(), Ordering::Relaxed);

        for (i, &bound) in self.bucket_bounds.iter().enumerate() {
            if value <= bound {
                self.buckets[i].fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
    }

    pub fn percentile(&self, p: f64) -> f64 {
        let total = self.count.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }

        let target = (total as f64 * p / 100.0) as u64;
        let mut cumulative = 0u64;

        for (i, bucket) in self.buckets.iter().enumerate() {
            cumulative += bucket.load(Ordering::Relaxed);
            if cumulative >= target {
                return self.bucket_bounds[i];
            }
        }

        *self.bucket_bounds.last().unwrap_or(&0.0)
    }

    pub fn mean(&self) -> f64 {
        let count = self.count.load(Ordering::Relaxed);
        if count == 0 {
            return 0.0;
        }
        let sum = f64::from_bits(self.sum.load(Ordering::Relaxed));
        sum / count as f64
    }

    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    pub fn meta(&self) -> &MetricMeta {
        &self.meta
    }
}

/// Rate metric (events per second)
pub struct RateMetric {
    events: AtomicU64,
    window_start: Mutex<Instant>,
    window_duration: Duration,
    last_rate: Mutex<f64>,
    meta: MetricMeta,
}

impl RateMetric {
    pub fn new(name: &str, description: &str, window_secs: u64) -> Self {
        Self {
            events: AtomicU64::new(0),
            window_start: Mutex::new(Instant::now()),
            window_duration: Duration::from_secs(window_secs),
            last_rate: Mutex::new(0.0),
            meta: MetricMeta {
                name: name.to_string(),
                description: description.to_string(),
                unit: "per_second".to_string(),
                metric_type: MetricType::Rate,
            },
        }
    }

    #[inline]
    pub fn record(&self) {
        self.events.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_n(&self, n: u64) {
        self.events.fetch_add(n, Ordering::Relaxed);
    }

    pub fn rate(&self) -> f64 {
        let mut start = self.window_start.lock().unwrap();
        let elapsed = start.elapsed();

        if elapsed >= self.window_duration {
            let events = self.events.swap(0, Ordering::Relaxed);
            let rate = events as f64 / elapsed.as_secs_f64();
            *self.last_rate.lock().unwrap() = rate;
            *start = Instant::now();
            rate
        } else {
            *self.last_rate.lock().unwrap()
        }
    }

    pub fn meta(&self) -> &MetricMeta {
        &self.meta
    }
}

/// Enterprise metrics registry
pub struct MetricsRegistry {
    // Transaction metrics
    pub transaction_count: Counter,
    pub transaction_latency: Histogram,
    pub commit_rate: RateMetric,
    pub rollback_rate: RateMetric,
    pub lock_acquisitions: Counter,
    pub lock_contentions: Counter,
    pub deadlocks: Counter,

    // Memory metrics
    pub memory_usage: Gauge,
    pub memory_pressure: Gauge,
    pub allocations: Counter,
    pub frees: Counter,
    pub fragmentation_ratio: Gauge,

    // Buffer pool metrics
    pub buffer_pool_hit_rate: Gauge,
    pub buffer_pool_size: Gauge,
    pub dirty_page_count: Gauge,
    pub evictions: Counter,
    pub page_reads: Counter,
    pub page_writes: Counter,

    // Query metrics
    pub queries_executed: Counter,
    pub query_latency: Histogram,
    pub full_table_scans: Counter,
    pub index_scans: Counter,

    // Replication metrics
    pub replication_lag_ms: Gauge,
    pub wal_writes: Counter,
    pub wal_bytes: Counter,

    // Security metrics
    pub threat_detections: Counter,
    pub blocked_queries: Counter,
    pub audit_entries: Counter,

    // SIMD metrics
    pub simd_operations: Counter,
    pub simd_utilization: Gauge,

    // Session metrics
    pub active_sessions: Gauge,
    pub session_attaches: Counter,
    pub session_detaches: Counter,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            // Transaction metrics
            transaction_count: Counter::new(
                "transaction_count",
                "Total number of transactions",
                "count",
            ),
            transaction_latency: Histogram::with_latency_buckets(
                "transaction_latency",
                "Transaction latency distribution",
            ),
            commit_rate: RateMetric::new("commit_rate", "Commits per second", 10),
            rollback_rate: RateMetric::new("rollback_rate", "Rollbacks per second", 10),
            lock_acquisitions: Counter::new(
                "lock_acquisitions",
                "Total lock acquisitions",
                "count",
            ),
            lock_contentions: Counter::new(
                "lock_contentions",
                "Lock contention events",
                "count",
            ),
            deadlocks: Counter::new("deadlocks", "Deadlocks detected", "count"),

            // Memory metrics
            memory_usage: Gauge::new("memory_usage", "Current memory usage", "bytes"),
            memory_pressure: Gauge::new("memory_pressure", "Memory pressure level", "level"),
            allocations: Counter::new("allocations", "Total allocations", "count"),
            frees: Counter::new("frees", "Total frees", "count"),
            fragmentation_ratio: Gauge::new(
                "fragmentation_ratio",
                "Memory fragmentation ratio",
                "ratio",
            ),

            // Buffer pool metrics
            buffer_pool_hit_rate: Gauge::new(
                "buffer_pool_hit_rate",
                "Buffer pool hit rate",
                "ratio",
            ),
            buffer_pool_size: Gauge::new(
                "buffer_pool_size",
                "Buffer pool size",
                "pages",
            ),
            dirty_page_count: Gauge::new(
                "dirty_page_count",
                "Number of dirty pages",
                "pages",
            ),
            evictions: Counter::new("evictions", "Page evictions", "count"),
            page_reads: Counter::new("page_reads", "Page reads from disk", "count"),
            page_writes: Counter::new("page_writes", "Page writes to disk", "count"),

            // Query metrics
            queries_executed: Counter::new("queries_executed", "Queries executed", "count"),
            query_latency: Histogram::with_latency_buckets(
                "query_latency",
                "Query latency distribution",
            ),
            full_table_scans: Counter::new(
                "full_table_scans",
                "Full table scans",
                "count",
            ),
            index_scans: Counter::new("index_scans", "Index scans", "count"),

            // Replication metrics
            replication_lag_ms: Gauge::new(
                "replication_lag_ms",
                "Replication lag",
                "milliseconds",
            ),
            wal_writes: Counter::new("wal_writes", "WAL writes", "count"),
            wal_bytes: Counter::new("wal_bytes", "WAL bytes written", "bytes"),

            // Security metrics
            threat_detections: Counter::new(
                "threat_detections",
                "Security threats detected",
                "count",
            ),
            blocked_queries: Counter::new(
                "blocked_queries",
                "Queries blocked by security",
                "count",
            ),
            audit_entries: Counter::new("audit_entries", "Audit log entries", "count"),

            // SIMD metrics
            simd_operations: Counter::new(
                "simd_operations",
                "SIMD operations executed",
                "count",
            ),
            simd_utilization: Gauge::new(
                "simd_utilization",
                "SIMD utilization ratio",
                "ratio",
            ),

            // Session metrics
            active_sessions: Gauge::new("active_sessions", "Active sessions", "count"),
            session_attaches: Counter::new(
                "session_attaches",
                "Session attach operations",
                "count",
            ),
            session_detaches: Counter::new(
                "session_detaches",
                "Session detach operations",
                "count",
            ),
        }
    }

    /// Get comprehensive metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            timestamp: Instant::now(),

            // Transaction metrics
            transaction_count: self.transaction_count.get(),
            transaction_latency_p50: self.transaction_latency.percentile(50.0),
            transaction_latency_p99: self.transaction_latency.percentile(99.0),
            commit_rate: self.commit_rate.rate(),
            rollback_rate: self.rollback_rate.rate(),
            lock_acquisitions: self.lock_acquisitions.get(),
            lock_contentions: self.lock_contentions.get(),
            deadlocks: self.deadlocks.get(),

            // Memory metrics
            memory_usage: self.memory_usage.get() as u64,
            memory_pressure: self.memory_pressure.get(),
            allocations: self.allocations.get(),
            fragmentation_ratio: self.fragmentation_ratio.get(),

            // Buffer pool metrics
            buffer_pool_hit_rate: self.buffer_pool_hit_rate.get(),
            dirty_page_count: self.dirty_page_count.get() as u64,
            evictions: self.evictions.get(),
            page_reads: self.page_reads.get(),
            page_writes: self.page_writes.get(),

            // Query metrics
            queries_executed: self.queries_executed.get(),
            query_latency_p50: self.query_latency.percentile(50.0),
            query_latency_p99: self.query_latency.percentile(99.0),
            full_table_scans: self.full_table_scans.get(),
            index_scans: self.index_scans.get(),

            // Replication metrics
            replication_lag_ms: self.replication_lag_ms.get(),
            wal_writes: self.wal_writes.get(),
            wal_bytes: self.wal_bytes.get(),

            // Security metrics
            threat_detections: self.threat_detections.get(),
            blocked_queries: self.blocked_queries.get(),
            audit_entries: self.audit_entries.get(),

            // SIMD metrics
            simd_operations: self.simd_operations.get(),
            simd_utilization: self.simd_utilization.get(),

            // Session metrics
            active_sessions: self.active_sessions.get() as u64,
            session_attaches: self.session_attaches.get(),
            session_detaches: self.session_detaches.get(),
        }
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics snapshot at a point in time
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub timestamp: Instant,

    // Transaction metrics
    pub transaction_count: u64,
    pub transaction_latency_p50: f64,
    pub transaction_latency_p99: f64,
    pub commit_rate: f64,
    pub rollback_rate: f64,
    pub lock_acquisitions: u64,
    pub lock_contentions: u64,
    pub deadlocks: u64,

    // Memory metrics
    pub memory_usage: u64,
    pub memory_pressure: f64,
    pub allocations: u64,
    pub fragmentation_ratio: f64,

    // Buffer pool metrics
    pub buffer_pool_hit_rate: f64,
    pub dirty_page_count: u64,
    pub evictions: u64,
    pub page_reads: u64,
    pub page_writes: u64,

    // Query metrics
    pub queries_executed: u64,
    pub query_latency_p50: f64,
    pub query_latency_p99: f64,
    pub full_table_scans: u64,
    pub index_scans: u64,

    // Replication metrics
    pub replication_lag_ms: f64,
    pub wal_writes: u64,
    pub wal_bytes: u64,

    // Security metrics
    pub threat_detections: u64,
    pub blocked_queries: u64,
    pub audit_entries: u64,

    // SIMD metrics
    pub simd_operations: u64,
    pub simd_utilization: f64,

    // Session metrics
    pub active_sessions: u64,
    pub session_attaches: u64,
    pub session_detaches: u64,
}

impl MetricsSnapshot {
    /// Calculate TPS (transactions per second)
    pub fn tps(&self) -> f64 {
        self.commit_rate
    }

    /// Calculate lock contention ratio
    pub fn lock_contention_ratio(&self) -> f64 {
        if self.lock_acquisitions == 0 {
            0.0
        } else {
            self.lock_contentions as f64 / self.lock_acquisitions as f64
        }
    }

    /// Calculate read/write ratio
    pub fn read_write_ratio(&self) -> f64 {
        if self.page_writes == 0 {
            f64::INFINITY
        } else {
            self.page_reads as f64 / self.page_writes as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::new("test", "Test counter", "count");
        counter.increment();
        counter.add(5);
        assert_eq!(counter.get(), 6);
    }

    #[test]
    fn test_gauge() {
        let gauge = Gauge::new("test", "Test gauge", "bytes");
        gauge.set(123.456);
        assert!((gauge.get() - 123.456).abs() < 0.001);
    }

    #[test]
    fn test_histogram() {
        let hist = Histogram::with_latency_buckets("test", "Test histogram");

        for i in 0..100 {
            hist.observe(i as f64 * 10.0);
        }

        assert_eq!(hist.count(), 100);
        assert!(hist.percentile(50.0) > 0.0);
        assert!(hist.percentile(99.0) > hist.percentile(50.0));
    }

    #[test]
    fn test_rate_metric() {
        let rate = RateMetric::new("test", "Test rate", 1);

        for _ in 0..100 {
            rate.record();
        }

        // Rate depends on timing, just verify it works
        let _ = rate.rate();
    }

    #[test]
    fn test_metrics_registry() {
        let registry = MetricsRegistry::new();

        registry.transaction_count.increment();
        registry.transaction_latency.observe(100.0);
        registry.memory_usage.set(1024.0 * 1024.0);

        let snapshot = registry.snapshot();
        assert_eq!(snapshot.transaction_count, 1);
        assert!(snapshot.memory_usage > 0);
    }
}
