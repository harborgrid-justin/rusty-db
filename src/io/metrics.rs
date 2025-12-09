// # I/O Metrics and Performance Monitoring
//
// Comprehensive metrics collection for I/O operations.

use std::time::Instant;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration};

// ============================================================================
// I/O Counters
// ============================================================================

/// Basic I/O operation counters
#[derive(Debug, Default)]
pub struct IoCounters {
    /// Number of read operations
    pub reads: AtomicU64,

    /// Number of write operations
    pub writes: AtomicU64,

    /// Number of sync operations
    pub syncs: AtomicU64,

    /// Total bytes read
    pub bytes_read: AtomicU64,

    /// Total bytes written
    pub bytes_written: AtomicU64,

    /// Number of errors
    pub errors: AtomicU64,
}

impl IoCounters {
    /// Create new counters
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment read counter
    #[inline]
    pub fn inc_reads(&self, bytes: u64) {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.bytes_read.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Increment write counter
    #[inline]
    pub fn inc_writes(&self, bytes: u64) {
        self.writes.fetch_add(1, Ordering::Relaxed);
        self.bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Increment sync counter
    #[inline]
    pub fn inc_syncs(&self) {
        self.syncs.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment error counter
    #[inline]
    pub fn inc_errors(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Get total operations
    #[inline]
    pub fn total_ops(&self) -> u64 {
        self.reads.load(Ordering::Relaxed)
            + self.writes.load(Ordering::Relaxed)
            + self.syncs.load(Ordering::Relaxed)
    }

    /// Get total bytes transferred
    #[inline]
    pub fn total_bytes(&self) -> u64 {
        self.bytes_read.load(Ordering::Relaxed) + self.bytes_written.load(Ordering::Relaxed)
    }

    /// Reset all counters
    pub fn reset(&self) {
        self.reads.store(0, Ordering::Relaxed);
        self.writes.store(0, Ordering::Relaxed);
        self.syncs.store(0, Ordering::Relaxed);
        self.bytes_read.store(0, Ordering::Relaxed);
        self.bytes_written.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
    }

    /// Get snapshot
    pub fn snapshot(&self) -> IoCountersSnapshot {
        IoCountersSnapshot {
            reads: self.reads.load(Ordering::Relaxed),
            writes: self.writes.load(Ordering::Relaxed),
            syncs: self.syncs.load(Ordering::Relaxed),
            bytes_read: self.bytes_read.load(Ordering::Relaxed),
            bytes_written: self.bytes_written.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of I/O counters
#[derive(Debug, Clone, Copy)]
pub struct IoCountersSnapshot {
    pub reads: u64,
    pub writes: u64,
    pub syncs: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub errors: u64,
}

// ============================================================================
// Latency Histogram
// ============================================================================

/// Latency histogram for tracking operation latencies
pub struct LatencyHistogram {
    /// Buckets for latency ranges (in microseconds)
    /// [0-10, 10-100, 100-1000, 1000-10000, 10000+]
    buckets: [AtomicU64; 10],

    /// Total samples
    total_samples: AtomicU64,

    /// Sum of all latencies (for average calculation)
    total_latency_us: AtomicU64,

    /// Minimum latency
    min_latency_us: AtomicU64,

    /// Maximum latency
    max_latency_us: AtomicU64,
}

impl Default for LatencyHistogram {
    fn default() -> Self {
        Self::new()
    }
}

impl LatencyHistogram {
    /// Create a new histogram
    pub fn new() -> Self {
        const INIT: AtomicU64 = AtomicU64::new(0);
        Self {
            buckets: [INIT; 10],
            total_samples: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            min_latency_us: AtomicU64::new(u64::MAX),
            max_latency_us: AtomicU64::new(0),
        }
    }

    /// Record a latency sample
    pub fn record(&self, duration: Duration) {
        let micros = duration.as_micros() as u64;

        // Update min/max
        let mut current_min = self.min_latency_us.load(Ordering::Relaxed);
        while micros < current_min {
            match self.min_latency_us.compare_exchange(
                current_min,
                micros,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }

        let mut current_max = self.max_latency_us.load(Ordering::Relaxed);
        while micros > current_max {
            match self.max_latency_us.compare_exchange(
                current_max,
                micros,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }

        // Update totals
        self.total_samples.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us.fetch_add(micros, Ordering::Relaxed);

        // Update histogram bucket
        let bucket_idx = match micros {
            0..=10 => 0,
            11..=100 => 1,
            101..=1_000 => 2,
            1_001..=10_000 => 3,
            10_001..=100_000 => 4,
            100_001..=1_000_000 => 5,
            1_000_001..=10_000_000 => 6,
            10_000_001..=100_000_000 => 7,
            100_000_001..=1_000_000_000 => 8,
            _ => 9,
        };

        self.buckets[bucket_idx].fetch_add(1, Ordering::Relaxed);
    }

    /// Get average latency
    #[inline]
    pub fn avg_latency_us(&self) -> u64 {
        let samples = self.total_samples.load(Ordering::Relaxed);
        if samples == 0 {
            0
        } else {
            self.total_latency_us.load(Ordering::Relaxed) / samples
        }
    }

    /// Get minimum latency
    #[inline]
    pub fn min_latency_us(&self) -> u64 {
        let min = self.min_latency_us.load(Ordering::Relaxed);
        if min == u64::MAX {
            0
        } else {
            min
        }
    }

    /// Get maximum latency
    #[inline]
    pub fn max_latency_us(&self) -> u64 {
        self.max_latency_us.load(Ordering::Relaxed)
    }

    /// Get total samples
    #[inline]
    pub fn total_samples(&self) -> u64 {
        self.total_samples.load(Ordering::Relaxed)
    }

    /// Get percentile estimate
    pub fn percentile(&self, p: f64) -> u64 {
        let total = self.total_samples.load(Ordering::Relaxed);
        if total == 0 {
            return 0;
        }

        let target = (total as f64 * p / 100.0) as u64;
        let mut cumulative = 0u64;

        let bucket_ranges = [
            (0, 10),
            (10, 100),
            (100, 1_000),
            (1_000, 10_000),
            (10_000, 100_000),
            (100_000, 1_000_000),
            (1_000_000, 10_000_000),
            (10_000_000, 100_000_000),
            (100_000_000, 1_000_000_000),
            (1_000_000_000, u64::MAX),
        ];

        for (i, (low, high)) in bucket_ranges.iter().enumerate() {
            let count = self.buckets[i].load(Ordering::Relaxed);
            cumulative += count;

            if cumulative >= target {
                // Linear interpolation within bucket
                let bucket_start = cumulative - count;
                let offset = target - bucket_start;
                let fraction = offset as f64 / count as f64;
                return low + ((high - low) as f64 * fraction) as u64;
            }
        }

        self.max_latency_us()
    }

    /// Reset histogram
    pub fn reset(&self) {
        for bucket in &self.buckets {
            bucket.store(0, Ordering::Relaxed);
        }
        self.total_samples.store(0, Ordering::Relaxed);
        self.total_latency_us.store(0, Ordering::Relaxed);
        self.min_latency_us.store(u64::MAX, Ordering::Relaxed);
        self.max_latency_us.store(0, Ordering::Relaxed);
    }

    /// Get histogram snapshot
    pub fn snapshot(&self) -> LatencyHistogramSnapshot {
        let mut buckets = [0u64; 10];
        for (i, bucket) in self.buckets.iter().enumerate() {
            buckets[i] = bucket.load(Ordering::Relaxed);
        }

        LatencyHistogramSnapshot {
            buckets,
            total_samples: self.total_samples.load(Ordering::Relaxed),
            avg_latency_us: self.avg_latency_us(),
            min_latency_us: self.min_latency_us(),
            max_latency_us: self.max_latency_us(),
        }
    }
}

/// Snapshot of latency histogram
#[derive(Debug, Clone)]
pub struct LatencyHistogramSnapshot {
    pub buckets: [u64; 10],
    pub total_samples: u64,
    pub avg_latency_us: u64,
    pub min_latency_us: u64,
    pub max_latency_us: u64,
}

// ============================================================================
// Throughput Metrics
// ============================================================================

/// Throughput metrics with time windows
pub struct ThroughputMetrics {
    /// Start time
    start_time: Instant,

    /// Operations per second (sliding window)
    ops_per_sec: RwLock<Vec<(Instant, u64)>>,

    /// Bytes per second (sliding window)
    bytes_per_sec: RwLock<Vec<(Instant, u64)>>,

    /// Window size
    window_size: Duration,
}

impl ThroughputMetrics {
    /// Create new throughput metrics
    pub fn new(window_size: Duration) -> Self {
        Self {
            start_time: Instant::now(),
            ops_per_sec: RwLock::new(Vec::new()),
            bytes_per_sec: RwLock::new(Vec::new()),
            window_size,
        }
    }

    /// Record operations
    pub fn record_ops(&self, count: u64) {
        let now = Instant::now();
        let mut ops = self.ops_per_sec.write();

        // Remove old entries outside window
        ops.retain(|(time, _)| now.duration_since(*time) < self.window_size);

        ops.push((now, count));
    }

    /// Record bytes
    pub fn record_bytes(&self, bytes: u64) {
        let now = Instant::now();
        let mut byte_samples = self.bytes_per_sec.write();

        // Remove old entries outside window
        byte_samples.retain(|(time, _)| now.duration_since(*time) < self.window_size);

        byte_samples.push((now, bytes));
    }

    /// Get current operations per second
    pub fn current_ops_per_sec(&self) -> f64 {
        let ops = self.ops_per_sec.read();
        if ops.is_empty() {
            return 0.0;
        }

        let now = Instant::now();
        let total_ops: u64 = ops.iter().map(|(_, count)| count).sum();
        let oldest_time = ops.first().unwrap().0;
        let duration_secs = now.duration_since(oldest_time).as_secs_f64();

        if duration_secs > 0.0 {
            total_ops as f64 / duration_secs
        } else {
            0.0
        }
    }

    /// Get current bytes per second
    pub fn current_bytes_per_sec(&self) -> f64 {
        let bytes = self.bytes_per_sec.read();
        if bytes.is_empty() {
            return 0.0;
        }

        let now = Instant::now();
        let total_bytes: u64 = bytes.iter().map(|(_, count)| count).sum();
        let oldest_time = bytes.first().unwrap().0;
        let duration_secs = now.duration_since(oldest_time).as_secs_f64();

        if duration_secs > 0.0 {
            total_bytes as f64 / duration_secs
        } else {
            0.0
        }
    }

    /// Get total uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Reset metrics
    pub fn reset(&self) {
        self.ops_per_sec.write().clear();
        self.bytes_per_sec.write().clear();
    }
}

impl Default for ThroughputMetrics {
    fn default() -> Self {
        Self::new(Duration::from_secs(60))
    }
}

// ============================================================================
// I/O Statistics
// ============================================================================

/// Comprehensive I/O statistics
#[derive(Debug, Clone)]
pub struct IoStats {
    /// Read statistics
    pub read_count: u64,
    pub read_bytes: u64,
    pub read_avg_latency_us: u64,
    pub read_min_latency_us: u64,
    pub read_max_latency_us: u64,

    /// Write statistics
    pub write_count: u64,
    pub write_bytes: u64,
    pub write_avg_latency_us: u64,
    pub write_min_latency_us: u64,
    pub write_max_latency_us: u64,

    /// Sync statistics
    pub sync_count: u64,
    pub sync_avg_latency_us: u64,

    /// Error count
    pub error_count: u64,

    /// Throughput
    pub ops_per_sec: f64,
    pub bytes_per_sec: f64,
}

impl Default for IoStats {
    fn default() -> Self {
        Self {
            read_count: 0,
            read_bytes: 0,
            read_avg_latency_us: 0,
            read_min_latency_us: 0,
            read_max_latency_us: 0,
            write_count: 0,
            write_bytes: 0,
            write_avg_latency_us: 0,
            write_min_latency_us: 0,
            write_max_latency_us: 0,
            sync_count: 0,
            sync_avg_latency_us: 0,
            error_count: 0,
            ops_per_sec: 0.0,
            bytes_per_sec: 0.0,
        }
    }
}

// ============================================================================
// I/O Metrics Collector
// ============================================================================

/// Main I/O metrics collector
pub struct IoMetrics {
    /// Counters
    counters: Arc<IoCounters>,

    /// Read latency histogram
    read_latency: Arc<LatencyHistogram>,

    /// Write latency histogram
    write_latency: Arc<LatencyHistogram>,

    /// Sync latency histogram
    sync_latency: Arc<LatencyHistogram>,

    /// Throughput metrics
    throughput: Arc<ThroughputMetrics>,

    /// Start time
    start_time: Instant,
}

impl Default for IoMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl IoMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            counters: Arc::new(IoCounters::new()),
            read_latency: Arc::new(LatencyHistogram::new()),
            write_latency: Arc::new(LatencyHistogram::new()),
            sync_latency: Arc::new(LatencyHistogram::new()),
            throughput: Arc::new(ThroughputMetrics::default()),
            start_time: Instant::now(),
        }
    }

    /// Record a read operation
    #[inline]
    pub fn record_read(&self, bytes: u64, latency: Duration) {
        self.counters.inc_reads(bytes);
        self.read_latency.record(latency);
        self.throughput.record_ops(1);
        self.throughput.record_bytes(bytes);
    }

    /// Record a write operation
    #[inline]
    pub fn record_write(&self, bytes: u64, latency: Duration) {
        self.counters.inc_writes(bytes);
        self.write_latency.record(latency);
        self.throughput.record_ops(1);
        self.throughput.record_bytes(bytes);
    }

    /// Record a sync operation
    #[inline]
    pub fn record_sync(&self, latency: Duration) {
        self.counters.inc_syncs();
        self.sync_latency.record(latency);
        self.throughput.record_ops(1);
    }

    /// Record an error
    #[inline]
    pub fn record_error(&self) {
        self.counters.inc_errors();
    }

    /// Get current statistics
    pub fn stats(&self) -> IoStats {
        let counters = self.counters.snapshot();

        IoStats {
            read_count: counters.reads,
            read_bytes: counters.bytes_read,
            read_avg_latency_us: self.read_latency.avg_latency_us(),
            read_min_latency_us: self.read_latency.min_latency_us(),
            read_max_latency_us: self.read_latency.max_latency_us(),
            write_count: counters.writes,
            write_bytes: counters.bytes_written,
            write_avg_latency_us: self.write_latency.avg_latency_us(),
            write_min_latency_us: self.write_latency.min_latency_us(),
            write_max_latency_us: self.write_latency.max_latency_us(),
            sync_count: counters.syncs,
            sync_avg_latency_us: self.sync_latency.avg_latency_us(),
            error_count: counters.errors,
            ops_per_sec: self.throughput.current_ops_per_sec(),
            bytes_per_sec: self.throughput.current_bytes_per_sec(),
        }
    }

    /// Get read percentile
    pub fn read_percentile(&self, p: f64) -> u64 {
        self.read_latency.percentile(p)
    }

    /// Get write percentile
    pub fn write_percentile(&self, p: f64) -> u64 {
        self.write_latency.percentile(p)
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.counters.reset();
        self.read_latency.reset();
        self.write_latency.reset();
        self.sync_latency.reset();
        self.throughput.reset();
    }
}

// ============================================================================
// Performance Stats
// ============================================================================

/// Performance statistics summary
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    /// Total I/O operations
    pub total_ops: u64,

    /// Total bytes transferred
    pub total_bytes: u64,

    /// Average throughput (ops/sec)
    pub avg_ops_per_sec: f64,

    /// Average bandwidth (MB/sec)
    pub avg_mbytes_per_sec: f64,

    /// Read/Write ratio
    pub read_write_ratio: f64,

    /// Error rate (%)
    pub error_rate: f64,

    /// Uptime
    pub uptime_secs: u64,
}

impl PerformanceStats {
    /// Create from IoStats
    pub fn from_stats(stats: &IoStats, uptime: Duration) -> Self {
        let total_ops = stats.read_count + stats.write_count + stats.sync_count;
        let total_bytes = stats.read_bytes + stats.write_bytes;
        let uptime_secs = uptime.as_secs();

        let avg_ops_per_sec = if uptime_secs > 0 {
            total_ops as f64 / uptime_secs as f64
        } else {
            0.0
        };

        let avg_mbytes_per_sec = if uptime_secs > 0 {
            (total_bytes as f64 / 1_048_576.0) / uptime_secs as f64
        } else {
            0.0
        };

        let read_write_ratio = if stats.write_count > 0 {
            stats.read_count as f64 / stats.write_count as f64
        } else {
            0.0
        };

        let error_rate = if total_ops > 0 {
            (stats.error_count as f64 / total_ops as f64) * 100.0
        } else {
            0.0
        };

        Self {
            total_ops,
            total_bytes,
            avg_ops_per_sec,
            avg_mbytes_per_sec,
            read_write_ratio,
            error_rate,
            uptime_secs,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_counters() {
        let counters = IoCounters::new();

        counters.inc_reads(1024);
        counters.inc_writes(2048);
        counters.inc_syncs();
        counters.inc_errors();

        assert_eq!(counters.reads.load(Ordering::Relaxed), 1);
        assert_eq!(counters.writes.load(Ordering::Relaxed), 1);
        assert_eq!(counters.syncs.load(Ordering::Relaxed), 1);
        assert_eq!(counters.errors.load(Ordering::Relaxed), 1);
        assert_eq!(counters.bytes_read.load(Ordering::Relaxed), 1024);
        assert_eq!(counters.bytes_written.load(Ordering::Relaxed), 2048);
        assert_eq!(counters.total_ops(), 3);
        assert_eq!(counters.total_bytes(), 3072);
    }

    #[test]
    fn test_latency_histogram() {
        let histogram = LatencyHistogram::new();

        histogram.record(Duration::from_micros(5));
        histogram.record(Duration::from_micros(50));
        histogram.record(Duration::from_micros(500));
        histogram.record(Duration::from_micros(5000));

        assert_eq!(histogram.total_samples(), 4);
        assert_eq!(histogram.min_latency_us(), 5);
        assert_eq!(histogram.max_latency_us(), 5000);

        let avg = histogram.avg_latency_us();
        assert!(avg > 0);
    }

    #[test]
    fn test_io_metrics() {
        let metrics = IoMetrics::new();

        metrics.record_read(4096, Duration::from_micros(100));
        metrics.record_write(4096, Duration::from_micros(200));
        metrics.record_sync(Duration::from_micros(1000));

        let _stats = metrics.stats();
        assert_eq!(stats.read_count, 1);
        assert_eq!(stats.write_count, 1);
        assert_eq!(stats.sync_count, 1);
        assert_eq!(stats.read_bytes, 4096);
        assert_eq!(stats.write_bytes, 4096);
    }
}
