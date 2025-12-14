// # System Metrics Collector
//
// Lock-free metrics collection using atomic operations for high-performance monitoring
// Implements HyperLogLog for cardinality estimation of unique metric values
//
// ## Performance Characteristics
// - Lock-free counters: O(1) increment/read with no contention
// - Atomic operations: Uses Relaxed ordering for maximum throughput
// - HyperLogLog: O(1) insert, ~0.81% standard error with 2^14 registers

use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Lock-free system metrics collector
pub struct SystemMetricsCollector {
    // Request metrics - lock-free atomics
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,

    // Response time tracking - uses lock-free averaging
    total_response_time_micros: AtomicU64,
    response_time_count: AtomicU64,

    // Disk I/O metrics - lock-free counters
    disk_read_bytes: AtomicU64,
    disk_write_bytes: AtomicU64,
    disk_read_ops: AtomicU64,
    disk_write_ops: AtomicU64,

    // Cache metrics - lock-free counters
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,

    // Buffer pool metrics
    buffer_pool_hits: AtomicU64,
    buffer_pool_misses: AtomicU64,

    // Query metrics
    queries_executed: AtomicU64,
    slow_queries: AtomicU64,

    // Transaction metrics
    transactions_committed: AtomicU64,
    transactions_rolled_back: AtomicU64,

    // Lock metrics
    locks_acquired: AtomicU64,
    deadlocks_detected: AtomicU64,

    // Cardinality estimation using HyperLogLog
    hll_registers: Arc<RwLock<HyperLogLog>>,

    // Metrics start time
    start_time: Instant,
}

impl SystemMetricsCollector {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            total_response_time_micros: AtomicU64::new(0),
            response_time_count: AtomicU64::new(0),
            disk_read_bytes: AtomicU64::new(0),
            disk_write_bytes: AtomicU64::new(0),
            disk_read_ops: AtomicU64::new(0),
            disk_write_ops: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            buffer_pool_hits: AtomicU64::new(0),
            buffer_pool_misses: AtomicU64::new(0),
            queries_executed: AtomicU64::new(0),
            slow_queries: AtomicU64::new(0),
            transactions_committed: AtomicU64::new(0),
            transactions_rolled_back: AtomicU64::new(0),
            locks_acquired: AtomicU64::new(0),
            deadlocks_detected: AtomicU64::new(0),
            hll_registers: Arc::new(RwLock::new(HyperLogLog::new(14))), // 2^14 registers
            start_time: Instant::now(),
        }
    }

    // Request tracking
    #[inline]
    pub fn record_request(&self, success: bool, response_time: Duration) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }

        // Record response time
        let micros = response_time.as_micros() as u64;
        self.total_response_time_micros
            .fetch_add(micros, Ordering::Relaxed);
        self.response_time_count.fetch_add(1, Ordering::Relaxed);
    }

    // Disk I/O tracking
    #[inline]
    pub fn record_disk_read(&self, bytes: u64) {
        self.disk_read_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.disk_read_ops.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_disk_write(&self, bytes: u64) {
        self.disk_write_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.disk_write_ops.fetch_add(1, Ordering::Relaxed);
    }

    // Cache tracking
    #[inline]
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    // Buffer pool tracking
    #[inline]
    pub fn record_buffer_pool_hit(&self) {
        self.buffer_pool_hits.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_buffer_pool_miss(&self) {
        self.buffer_pool_misses.fetch_add(1, Ordering::Relaxed);
    }

    // Query tracking
    #[inline]
    pub fn record_query(&self, is_slow: bool) {
        self.queries_executed.fetch_add(1, Ordering::Relaxed);
        if is_slow {
            self.slow_queries.fetch_add(1, Ordering::Relaxed);
        }
    }

    // Transaction tracking
    #[inline]
    pub fn record_transaction_commit(&self) {
        self.transactions_committed.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_transaction_rollback(&self) {
        self.transactions_rolled_back
            .fetch_add(1, Ordering::Relaxed);
    }

    // Lock tracking
    #[inline]
    pub fn record_lock_acquired(&self) {
        self.locks_acquired.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_deadlock(&self) {
        self.deadlocks_detected.fetch_add(1, Ordering::Relaxed);
    }

    // Cardinality tracking (e.g., unique users, queries)
    pub fn add_cardinality_item(&self, item: &str) {
        let mut hll = self.hll_registers.write();
        hll.add(item);
    }

    // Get metrics
    pub fn get_total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    pub fn get_successful_requests(&self) -> u64 {
        self.successful_requests.load(Ordering::Relaxed)
    }

    pub fn get_failed_requests(&self) -> u64 {
        self.failed_requests.load(Ordering::Relaxed)
    }

    pub fn get_avg_response_time_ms(&self) -> f64 {
        let count = self.response_time_count.load(Ordering::Relaxed);
        if count == 0 {
            return 0.0;
        }

        let total_micros = self.total_response_time_micros.load(Ordering::Relaxed);
        (total_micros as f64 / count as f64) / 1000.0 // Convert to milliseconds
    }

    pub fn get_disk_read_bytes(&self) -> u64 {
        self.disk_read_bytes.load(Ordering::Relaxed)
    }

    pub fn get_disk_write_bytes(&self) -> u64 {
        self.disk_write_bytes.load(Ordering::Relaxed)
    }

    pub fn get_cache_hit_ratio(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            return 0.0;
        }

        hits as f64 / total as f64
    }

    pub fn get_buffer_pool_hit_ratio(&self) -> f64 {
        let hits = self.buffer_pool_hits.load(Ordering::Relaxed);
        let misses = self.buffer_pool_misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            return 0.0;
        }

        hits as f64 / total as f64
    }

    pub fn get_queries_executed(&self) -> u64 {
        self.queries_executed.load(Ordering::Relaxed)
    }

    pub fn get_slow_queries(&self) -> u64 {
        self.slow_queries.load(Ordering::Relaxed)
    }

    pub fn get_transactions_committed(&self) -> u64 {
        self.transactions_committed.load(Ordering::Relaxed)
    }

    pub fn get_transactions_rolled_back(&self) -> u64 {
        self.transactions_rolled_back.load(Ordering::Relaxed)
    }

    pub fn get_locks_acquired(&self) -> u64 {
        self.locks_acquired.load(Ordering::Relaxed)
    }

    pub fn get_deadlocks_detected(&self) -> u64 {
        self.deadlocks_detected.load(Ordering::Relaxed)
    }

    pub fn get_unique_items_estimate(&self) -> u64 {
        let hll = self.hll_registers.read();
        hll.count()
    }

    pub fn get_uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

impl Default for SystemMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// HyperLogLog implementation for cardinality estimation
/// Uses 2^precision registers with 6-bit values
/// Standard error: 1.04 / sqrt(2^precision)
pub struct HyperLogLog {
    registers: Vec<AtomicUsize>,
    precision: u8,
    alpha: f64,
}

impl HyperLogLog {
    pub fn new(precision: u8) -> Self {
        let m = 1 << precision; // 2^precision

        // Calculate alpha_m constant based on m
        let alpha = match m {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / m as f64),
        };

        let mut registers = Vec::with_capacity(m);
        for _ in 0..m {
            registers.push(AtomicUsize::new(0));
        }

        Self {
            registers,
            precision,
            alpha,
        }
    }

    pub fn add(&mut self, item: &str) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        let hash = hasher.finish();

        // Use first `precision` bits as register index
        let register_idx = (hash >> (64 - self.precision)) as usize;

        // Count leading zeros in remaining bits + 1
        let remaining = hash << self.precision;
        let leading_zeros: usize = if remaining == 0 {
            (64 - self.precision + 1) as usize
        } else {
            (remaining.leading_zeros() as usize) + 1
        };

        // Update register if new value is larger (using atomic max)
        let register = &self.registers[register_idx];
        let mut current = register.load(Ordering::Relaxed);

        while leading_zeros > current {
            match register.compare_exchange_weak(
                current,
                leading_zeros,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current = x,
            }
        }
    }

    pub fn count(&self) -> u64 {
        let m = self.registers.len() as f64;

        // Calculate harmonic mean of 2^register_values
        let mut sum = 0.0;
        let mut zero_count = 0;

        for register in &self.registers {
            let val = register.load(Ordering::Relaxed);
            if val == 0 {
                zero_count += 1;
            }
            sum += 1.0 / (1u64 << val) as f64;
        }

        // Raw estimate
        let raw_estimate = self.alpha * m * m / sum;

        // Apply bias correction for small/large estimates
        if raw_estimate <= 2.5 * m {
            // Small range correction
            if zero_count > 0 {
                (m * (m / zero_count as f64).ln()) as u64
            } else {
                raw_estimate as u64
            }
        } else if raw_estimate <= (1u64 << 32) as f64 / 30.0 {
            // No correction
            raw_estimate as u64
        } else {
            // Large range correction
            let two_32 = (1u64 << 32) as f64;
            (-two_32 * (1.0 - raw_estimate / two_32).ln()) as u64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_free_counters() {
        let collector = SystemMetricsCollector::new();

        collector.record_request(true, Duration::from_millis(10));
        collector.record_request(true, Duration::from_millis(20));
        collector.record_request(false, Duration::from_millis(30));

        assert_eq!(collector.get_total_requests(), 3);
        assert_eq!(collector.get_successful_requests(), 2);
        assert_eq!(collector.get_failed_requests(), 1);

        let avg = collector.get_avg_response_time_ms();
        assert!(avg > 19.0 && avg < 21.0); // Should be ~20ms
    }

    #[test]
    fn test_disk_io_tracking() {
        let collector = SystemMetricsCollector::new();

        collector.record_disk_read(1024);
        collector.record_disk_read(2048);
        collector.record_disk_write(512);

        assert_eq!(collector.get_disk_read_bytes(), 3072);
        assert_eq!(collector.get_disk_write_bytes(), 512);
    }

    #[test]
    fn test_cache_hit_ratio() {
        let collector = SystemMetricsCollector::new();

        for _ in 0..95 {
            collector.record_cache_hit();
        }
        for _ in 0..5 {
            collector.record_cache_miss();
        }

        let ratio = collector.get_cache_hit_ratio();
        assert!(ratio > 0.94 && ratio < 0.96); // Should be 0.95
    }

    #[test]
    fn test_hyperloglog_cardinality() {
        let mut hll = HyperLogLog::new(14);

        // Add 10000 unique items
        for i in 0..10000 {
            hll.add(&format!("item_{}", i));
        }

        let count = hll.count();
        // HyperLogLog has ~0.81% standard error with 2^14 registers
        // So for 10000 items, we expect count in range [9919, 10081]
        assert!(count > 9500 && count < 10500);
    }
}
