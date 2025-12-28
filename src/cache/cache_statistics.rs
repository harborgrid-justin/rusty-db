// # Cache Statistics and Monitoring
//
// Comprehensive metrics for cache performance analysis and optimization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Reason for cache entry eviction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvictionReason {
    /// Entry expired (TTL)
    Expired,

    /// LRU eviction due to memory pressure
    LruEviction,

    /// Table invalidation
    TableInvalidation,

    /// Row invalidation
    RowInvalidation,

    /// Manual eviction
    Manual,

    /// Cache cleared
    CacheCleared,
}

/// Cache performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    /// Total number of cache hits
    pub hits: u64,

    /// Total number of cache misses
    pub misses: u64,

    /// Total number of cache puts
    pub puts: u64,

    /// Total number of evictions
    pub evictions: u64,

    /// Evictions by reason
    pub evictions_by_reason: HashMap<String, u64>,

    /// Total query execution time saved (estimated, in microseconds)
    pub saved_execution_time_us: u64,

    /// Average cache entry size in bytes
    pub avg_entry_size_bytes: usize,

    /// Peak memory usage in bytes
    pub peak_memory_bytes: usize,

    /// Current memory usage in bytes
    pub current_memory_bytes: usize,

    /// Total number of invalidations
    pub total_invalidations: u64,
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            puts: 0,
            evictions: 0,
            evictions_by_reason: HashMap::new(),
            saved_execution_time_us: 0,
            avg_entry_size_bytes: 0,
            peak_memory_bytes: 0,
            current_memory_bytes: 0,
            total_invalidations: 0,
        }
    }
}

impl CacheMetrics {
    /// Calculate hit rate as percentage (0.0 - 100.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }

    /// Calculate miss rate as percentage (0.0 - 100.0)
    pub fn miss_rate(&self) -> f64 {
        100.0 - self.hit_rate()
    }

    /// Get total number of requests
    pub fn total_requests(&self) -> u64 {
        self.hits + self.misses
    }

    /// Get eviction count for a specific reason
    pub fn evictions_for_reason(&self, reason: EvictionReason) -> u64 {
        let key = format!("{:?}", reason);
        *self.evictions_by_reason.get(&key).unwrap_or(&0)
    }
}

/// Cache statistics with time-series tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    /// Current metrics
    pub metrics: CacheMetrics,

    /// When statistics tracking started
    #[serde(skip)]
    pub start_time: Option<Instant>,

    /// Latency histogram (microseconds)
    pub latency_histogram: Vec<u64>,

    /// Request rate tracking (requests per second)
    #[serde(skip)]
    request_timestamps: Arc<RwLock<Vec<Instant>>>,

    /// Window size for rate calculation (seconds)
    rate_window_secs: u64,
}

impl CacheStatistics {
    /// Create new cache statistics
    pub fn new() -> Self {
        Self {
            metrics: CacheMetrics::default(),
            start_time: Some(Instant::now()),
            latency_histogram: Vec::new(),
            request_timestamps: Arc::new(RwLock::new(Vec::new())),
            rate_window_secs: 60, // 1 minute window
        }
    }

    /// Record a cache hit
    pub fn record_hit(&mut self) {
        self.metrics.hits += 1;
        self.record_request_timestamp();
    }

    /// Record a cache miss
    pub fn record_miss(&mut self) {
        self.metrics.misses += 1;
        self.record_request_timestamp();
    }

    /// Record a cache put operation
    pub fn record_put(&mut self, entry_size: usize) {
        self.metrics.puts += 1;

        // Update average entry size
        let total_puts = self.metrics.puts;
        let current_avg = self.metrics.avg_entry_size_bytes;
        self.metrics.avg_entry_size_bytes =
            ((current_avg as u64 * (total_puts - 1) + entry_size as u64) / total_puts) as usize;
    }

    /// Record a cache eviction
    pub fn record_eviction(&mut self, reason: EvictionReason) {
        self.metrics.evictions += 1;

        let key = format!("{:?}", reason);
        *self.metrics.evictions_by_reason.entry(key).or_insert(0) += 1;
    }

    /// Record cache invalidation
    pub fn record_invalidation(&mut self, count: u64) {
        self.metrics.total_invalidations += count;
    }

    /// Record query execution time saved (in microseconds)
    pub fn record_saved_time(&mut self, time_us: u64) {
        self.metrics.saved_execution_time_us += time_us;
    }

    /// Record latency for a cache operation (in microseconds)
    pub fn record_latency(&mut self, latency_us: u64) {
        self.latency_histogram.push(latency_us);

        // Keep histogram bounded (last 10000 entries)
        if self.latency_histogram.len() > 10_000 {
            self.latency_histogram.drain(0..1000);
        }
    }

    /// Update current memory usage
    pub fn update_memory_usage(&mut self, current_bytes: usize) {
        self.metrics.current_memory_bytes = current_bytes;

        if current_bytes > self.metrics.peak_memory_bytes {
            self.metrics.peak_memory_bytes = current_bytes;
        }
    }

    /// Get current requests per second
    pub fn requests_per_second(&self) -> f64 {
        let timestamps = self.request_timestamps.read().unwrap();

        if timestamps.is_empty() {
            return 0.0;
        }

        let now = Instant::now();
        let window = Duration::from_secs(self.rate_window_secs);

        // Count requests within the window
        let count = timestamps
            .iter()
            .filter(|&&ts| now.duration_since(ts) <= window)
            .count();

        count as f64 / self.rate_window_secs as f64
    }

    /// Get uptime duration
    pub fn uptime(&self) -> Duration {
        self.start_time
            .map(|start| start.elapsed())
            .unwrap_or(Duration::from_secs(0))
    }

    /// Get average latency (in microseconds)
    pub fn avg_latency_us(&self) -> f64 {
        if self.latency_histogram.is_empty() {
            return 0.0;
        }

        let sum: u64 = self.latency_histogram.iter().sum();
        sum as f64 / self.latency_histogram.len() as f64
    }

    /// Get median latency (in microseconds)
    pub fn median_latency_us(&self) -> u64 {
        if self.latency_histogram.is_empty() {
            return 0;
        }

        let mut sorted = self.latency_histogram.clone();
        sorted.sort_unstable();
        sorted[sorted.len() / 2]
    }

    /// Get 95th percentile latency (in microseconds)
    pub fn p95_latency_us(&self) -> u64 {
        if self.latency_histogram.is_empty() {
            return 0;
        }

        let mut sorted = self.latency_histogram.clone();
        sorted.sort_unstable();
        let index = (sorted.len() as f64 * 0.95) as usize;
        sorted[index.min(sorted.len() - 1)]
    }

    /// Get 99th percentile latency (in microseconds)
    pub fn p99_latency_us(&self) -> u64 {
        if self.latency_histogram.is_empty() {
            return 0;
        }

        let mut sorted = self.latency_histogram.clone();
        sorted.sort_unstable();
        let index = (sorted.len() as f64 * 0.99) as usize;
        sorted[index.min(sorted.len() - 1)]
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        self.metrics = CacheMetrics::default();
        self.start_time = Some(Instant::now());
        self.latency_histogram.clear();
        self.request_timestamps.write().unwrap().clear();
    }

    /// Get a summary report as a formatted string
    pub fn summary(&self) -> String {
        format!(
            "Cache Statistics:\n\
             - Uptime: {:?}\n\
             - Hit Rate: {:.2}%\n\
             - Total Requests: {}\n\
             - Hits: {}, Misses: {}\n\
             - Puts: {}, Evictions: {}\n\
             - Current Memory: {} bytes\n\
             - Peak Memory: {} bytes\n\
             - Avg Entry Size: {} bytes\n\
             - Requests/sec: {:.2}\n\
             - Avg Latency: {:.2} μs\n\
             - P95 Latency: {} μs\n\
             - P99 Latency: {} μs\n\
             - Total Invalidations: {}",
            self.uptime(),
            self.metrics.hit_rate(),
            self.metrics.total_requests(),
            self.metrics.hits,
            self.metrics.misses,
            self.metrics.puts,
            self.metrics.evictions,
            self.metrics.current_memory_bytes,
            self.metrics.peak_memory_bytes,
            self.metrics.avg_entry_size_bytes,
            self.requests_per_second(),
            self.avg_latency_us(),
            self.p95_latency_us(),
            self.p99_latency_us(),
            self.metrics.total_invalidations,
        )
    }

    /// Record a request timestamp for rate calculation
    fn record_request_timestamp(&mut self) {
        let now = Instant::now();
        let mut timestamps = self.request_timestamps.write().unwrap();

        timestamps.push(now);

        // Clean up old timestamps outside the window
        let window = Duration::from_secs(self.rate_window_secs * 2); // Keep 2x window
        timestamps.retain(|&ts| now.duration_since(ts) <= window);
    }
}

impl Default for CacheStatistics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_metrics_default() {
        let metrics = CacheMetrics::default();
        assert_eq!(metrics.hits, 0);
        assert_eq!(metrics.misses, 0);
        assert_eq!(metrics.hit_rate(), 0.0);
    }

    #[test]
    fn test_hit_rate_calculation() {
        let mut metrics = CacheMetrics::default();
        metrics.hits = 75;
        metrics.misses = 25;

        assert_eq!(metrics.hit_rate(), 75.0);
        assert_eq!(metrics.miss_rate(), 25.0);
        assert_eq!(metrics.total_requests(), 100);
    }

    #[test]
    fn test_eviction_tracking() {
        let mut stats = CacheStatistics::new();

        stats.record_eviction(EvictionReason::Expired);
        stats.record_eviction(EvictionReason::Expired);
        stats.record_eviction(EvictionReason::LruEviction);

        assert_eq!(stats.metrics.evictions, 3);
        assert_eq!(
            stats.metrics.evictions_for_reason(EvictionReason::Expired),
            2
        );
        assert_eq!(
            stats.metrics.evictions_for_reason(EvictionReason::LruEviction),
            1
        );
    }

    #[test]
    fn test_record_hit_miss() {
        let mut stats = CacheStatistics::new();

        stats.record_hit();
        stats.record_hit();
        stats.record_miss();

        assert_eq!(stats.metrics.hits, 2);
        assert_eq!(stats.metrics.misses, 1);
        assert_eq!(stats.metrics.hit_rate(), 66.66666666666666);
    }

    #[test]
    fn test_latency_tracking() {
        let mut stats = CacheStatistics::new();

        stats.record_latency(100);
        stats.record_latency(200);
        stats.record_latency(300);

        assert_eq!(stats.avg_latency_us(), 200.0);
        assert_eq!(stats.median_latency_us(), 200);
    }

    #[test]
    fn test_memory_tracking() {
        let mut stats = CacheStatistics::new();

        stats.update_memory_usage(1000);
        assert_eq!(stats.metrics.current_memory_bytes, 1000);
        assert_eq!(stats.metrics.peak_memory_bytes, 1000);

        stats.update_memory_usage(2000);
        assert_eq!(stats.metrics.current_memory_bytes, 2000);
        assert_eq!(stats.metrics.peak_memory_bytes, 2000);

        stats.update_memory_usage(1500);
        assert_eq!(stats.metrics.current_memory_bytes, 1500);
        assert_eq!(stats.metrics.peak_memory_bytes, 2000); // Peak unchanged
    }

    #[test]
    fn test_reset_statistics() {
        let mut stats = CacheStatistics::new();

        stats.record_hit();
        stats.record_miss();
        stats.record_latency(100);

        stats.reset();

        assert_eq!(stats.metrics.hits, 0);
        assert_eq!(stats.metrics.misses, 0);
        assert!(stats.latency_histogram.is_empty());
    }

    #[test]
    fn test_summary_report() {
        let mut stats = CacheStatistics::new();

        stats.record_hit();
        stats.record_miss();

        let summary = stats.summary();
        assert!(summary.contains("Hit Rate"));
        assert!(summary.contains("Total Requests"));
    }

    #[test]
    fn test_avg_entry_size() {
        let mut stats = CacheStatistics::new();

        stats.record_put(100);
        assert_eq!(stats.metrics.avg_entry_size_bytes, 100);

        stats.record_put(200);
        assert_eq!(stats.metrics.avg_entry_size_bytes, 150);

        stats.record_put(300);
        assert_eq!(stats.metrics.avg_entry_size_bytes, 200);
    }

    #[test]
    fn test_percentile_calculations() {
        let mut stats = CacheStatistics::new();

        for i in 1..=100 {
            stats.record_latency(i * 10);
        }

        let p95 = stats.p95_latency_us();
        let p99 = stats.p99_latency_us();

        assert!(p95 > 0);
        assert!(p99 > p95);
        assert!(p99 <= 1000); // Max value
    }
}
