// Buffer Pool Statistics
//
// Comprehensive statistics tracking and reporting.

use super::common::*;
use serde::{Serialize, Deserialize};

pub struct BufferPoolStatisticsTracker {
    // Per-pool hit ratio tracking
    pool_hit_ratios: PRwLock<HashMap<String, PoolHitRatio>>,
    // Page type distribution
    page_type_dist: PRwLock<HashMap<PageType, AtomicU64>>,
    // Wait statistics
    wait_stats: WaitStatistics,
    // Buffer busy waits
    busy_waits: BusyWaitStatistics,
    // Memory pressure
    pub(crate) memory_pressure: MemoryPressureMonitor,
    // Real-time metrics
    realtime_metrics: RealtimeMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PageType {
    Data,
    Index,
    Undo,
    Redo,
    Temp,
    System,
}

#[derive(Debug)]
struct PoolHitRatio {
    hits: AtomicU64,
    misses: AtomicU64,
    accesses: AtomicU64,
}

impl PoolHitRatio {
    fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            accesses: AtomicU64::new(0),
        }
    }

    fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
        self.accesses.fetch_add(1, Ordering::Relaxed);
    }

    fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
        self.accesses.fetch_add(1, Ordering::Relaxed);
    }

    fn hit_ratio(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let accesses = self.accesses.load(Ordering::Relaxed) as f64;
        if accesses == 0.0 {
            0.0
        } else {
            hits / accesses
        }
    }
}

// Wait statistics for buffer operations
#[derive(Debug)]
pub struct WaitStatistics {
    // Wait time for free buffers
    free_buffer_waits: AtomicU64,
    free_buffer_wait_time_ns: AtomicU64,
    // Wait time for buffer locks
    buffer_lock_waits: AtomicU64,
    buffer_lock_wait_time_ns: AtomicU64,
    // Wait time for I/O completion
    io_waits: AtomicU64,
    io_wait_time_ns: AtomicU64,
}

impl WaitStatistics {
    pub fn new() -> Self {
        Self {
            free_buffer_waits: AtomicU64::new(0),
            free_buffer_wait_time_ns: AtomicU64::new(0),
            buffer_lock_waits: AtomicU64::new(0),
            buffer_lock_wait_time_ns: AtomicU64::new(0),
            io_waits: AtomicU64::new(0),
            io_wait_time_ns: AtomicU64::new(0),
        }
    }

    // Record free buffer wait
    #[allow(dead_code)]
    pub fn record_free_buffer_wait(&self, duration: Duration) {
        self.free_buffer_waits.fetch_add(1, Ordering::Relaxed);
        self.free_buffer_wait_time_ns.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    // Record buffer lock wait
    #[allow(dead_code)]
    pub fn record_buffer_lock_wait(&self, duration: Duration) {
        self.buffer_lock_waits.fetch_add(1, Ordering::Relaxed);
        self.buffer_lock_wait_time_ns.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    // Record I/O wait
    #[allow(dead_code)]
    pub fn record_io_wait(&self, duration: Duration) {
        self.io_waits.fetch_add(1, Ordering::Relaxed);
        self.io_wait_time_ns.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    // Get snapshot
    pub fn snapshot(&self) -> WaitStatisticsSnapshot {
        WaitStatisticsSnapshot {
            free_buffer_waits: self.free_buffer_waits.load(Ordering::Relaxed),
            free_buffer_wait_time_ns: self.free_buffer_wait_time_ns.load(Ordering::Relaxed),
            buffer_lock_waits: self.buffer_lock_waits.load(Ordering::Relaxed),
            buffer_lock_wait_time_ns: self.buffer_lock_wait_time_ns.load(Ordering::Relaxed),
            io_waits: self.io_waits.load(Ordering::Relaxed),
            io_wait_time_ns: self.io_wait_time_ns.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitStatisticsSnapshot {
    pub free_buffer_waits: u64,
    pub free_buffer_wait_time_ns: u64,
    pub buffer_lock_waits: u64,
    pub buffer_lock_wait_time_ns: u64,
    pub io_waits: u64,
    pub io_wait_time_ns: u64,
}

// Buffer busy wait statistics
#[derive(Debug)]
pub struct BusyWaitStatistics {
    // Waits by page type
    waits_by_type: PRwLock<HashMap<PageType, AtomicU64>>,
    // Waits by tablespace
    waits_by_tablespace: PRwLock<HashMap<u32, AtomicU64>>,
    // Total busy waits
    total_waits: AtomicU64,
    // Total wait time
    total_wait_time_ns: AtomicU64,
}

impl BusyWaitStatistics {
    pub fn new() -> Self {
        Self {
            waits_by_type: PRwLock::new(HashMap::new()),
            waits_by_tablespace: PRwLock::new(HashMap::new()),
            total_waits: AtomicU64::new(0),
            total_wait_time_ns: AtomicU64::new(0),
        }
    }

    // Record busy wait
    #[allow(dead_code)]
    pub fn record_wait(&self, page_type: PageType, tablespace_id: u32, duration: Duration) {
        self.total_waits.fetch_add(1, Ordering::Relaxed);
        self.total_wait_time_ns.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);

        // Record by type
        let types = self.waits_by_type.read();
        if let Some(counter) = types.get(&page_type) {
            counter.fetch_add(1, Ordering::Relaxed);
        } else {
            drop(types);
            let mut types = self.waits_by_type.write();
            types.entry(page_type).or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }

        // Record by tablespace
        let spaces = self.waits_by_tablespace.read();
        if let Some(counter) = spaces.get(&tablespace_id) {
            counter.fetch_add(1, Ordering::Relaxed);
        } else {
            drop(spaces);
            let mut spaces = self.waits_by_tablespace.write();
            spaces.entry(tablespace_id).or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    // Get snapshot
    pub fn snapshot(&self) -> BusyWaitStatisticsSnapshot {
        let types = self.waits_by_type.read();
        let waits_by_type: HashMap<PageType, u64> = types.iter()
            .map(|(k, v)| (*k, v.load(Ordering::Relaxed)))
            .collect();

        let spaces = self.waits_by_tablespace.read();
        let waits_by_tablespace: HashMap<u32, u64> = spaces.iter()
            .map(|(k, v)| (*k, v.load(Ordering::Relaxed)))
            .collect();

        BusyWaitStatisticsSnapshot {
            waits_by_type,
            waits_by_tablespace,
            total_waits: self.total_waits.load(Ordering::Relaxed),
            total_wait_time_ns: self.total_wait_time_ns.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusyWaitStatisticsSnapshot {
    pub waits_by_type: HashMap<PageType, u64>,
    pub waits_by_tablespace: HashMap<u32, u64>,
    pub total_waits: u64,
    pub total_wait_time_ns: u64,
}

// Memory pressure monitor
#[derive(Debug)]
pub struct MemoryPressureMonitor {
    // Current memory usage
    current_usage: AtomicU64,
    // Peak memory usage
    peak_usage: AtomicU64,
    // Memory limit
    limit: AtomicU64,
    // Pressure events
    pressure_events: AtomicU64,
    // Last pressure check
    _last_check: Mutex<Instant>,
}

impl MemoryPressureMonitor {
    pub fn new(limit: u64) -> Self {
        Self {
            current_usage: AtomicU64::new(0),
            peak_usage: AtomicU64::new(0),
            limit: AtomicU64::new(limit),
            pressure_events: AtomicU64::new(0),
            _last_check: Mutex::new(Instant::now()),
        }
    }

    // Update memory usage
    #[allow(dead_code)]
    pub fn update_usage(&self, usage: u64) {
        self.current_usage.store(usage, Ordering::Relaxed);

        // Update peak if necessary
        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        while usage > peak {
            match self.peak_usage.compare_exchange_weak(
                peak,
                usage,
                Ordering::Relaxed,
                Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => peak = x,
            }
        }

        // Check for pressure
        if self.is_under_pressure() {
            self.pressure_events.fetch_add(1, Ordering::Relaxed);
        }

        *self._last_check.lock() = Instant::now();
    }

    // Check if under memory pressure
    pub fn is_under_pressure(&self) -> bool {
        let usage = self.current_usage.load(Ordering::Relaxed);
        let limit = self.limit.load(Ordering::Relaxed);
        usage as f64 / limit as f64 > 0.9 // 90% threshold
    }

    // Get pressure level (0.0 - 1.0)
    pub fn pressure_level(&self) -> f64 {
        let usage = self.current_usage.load(Ordering::Relaxed);
        let limit = self.limit.load(Ordering::Relaxed);
        (usage as f64 / limit as f64).min(1.0)
    }

    // Get snapshot
    pub fn snapshot(&self) -> MemoryPressureSnapshot {
        MemoryPressureSnapshot {
            current_usage: self.current_usage.load(Ordering::Relaxed),
            peak_usage: self.peak_usage.load(Ordering::Relaxed),
            limit: self.limit.load(Ordering::Relaxed),
            pressure_events: self.pressure_events.load(Ordering::Relaxed),
            pressure_level: self.pressure_level(),
            under_pressure: self.is_under_pressure(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPressureSnapshot {
    pub current_usage: u64,
    pub peak_usage: u64,
    pub limit: u64,
    pub pressure_events: u64,
    pub pressure_level: f64,
    pub under_pressure: bool,
}

// Real-time metrics exporter
#[derive(Debug)]
pub struct RealtimeMetrics {
    // Metrics update interval
    _interval: Duration,
    // Current metrics
    current: Mutex<MetricsSnapshot>,
    // Last update time
    _last_update: Mutex<Instant>,
}

impl RealtimeMetrics {
    pub fn new(interval_secs: u64) -> Self {
        Self {
            _interval: Duration::from_secs(interval_secs),
            current: Mutex::new(MetricsSnapshot::default()),
            _last_update: Mutex::new(Instant::now()),
        }
    }

    // Update metrics
    #[allow(dead_code)]
    pub fn update(&self, snapshot: MetricsSnapshot) {
        *self.current.lock() = snapshot;
        *self._last_update.lock() = Instant::now();
    }

    // Get current metrics
    pub fn get(&self) -> MetricsSnapshot {
        self.current.lock().clone()
    }

    // Check if metrics are stale
    #[allow(dead_code)]
    pub fn is_stale(&self) -> bool {
        self._last_update.lock().elapsed() > self._interval * 2
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub buffer_pool_size: usize,
    pub pages_in_use: usize,
    pub dirty_pages: usize,
    pub hit_ratio: f64,
    pub pages_read: u64,
    pub pages_written: u64,
    pub io_operations: u64,
}

impl BufferPoolStatisticsTracker {
    pub fn new() -> Self {
        Self {
            pool_hit_ratios: PRwLock::new(HashMap::new()),
            page_type_dist: PRwLock::new(HashMap::new()),
            wait_stats: WaitStatistics::new(),
            busy_waits: BusyWaitStatistics::new(),
            memory_pressure: MemoryPressureMonitor::new(1024 * 1024 * 1024), // 1GB default
            realtime_metrics: RealtimeMetrics::new(1),
        }
    }

    // Record hit for a pool
    pub fn record_hit(&self, pool_name: &str) {
        let ratios = self.pool_hit_ratios.read();
        if let Some(ratio) = ratios.get(pool_name) {
            ratio.record_hit();
        } else {
            drop(ratios);
            let mut ratios = self.pool_hit_ratios.write();
            let ratio = ratios.entry(pool_name.to_string())
                .or_insert_with(PoolHitRatio::new);
            ratio.record_hit();
        }
    }

    // Record miss for a pool
    pub fn record_miss(&self, pool_name: &str) {
        let ratios = self.pool_hit_ratios.read();
        if let Some(ratio) = ratios.get(pool_name) {
            ratio.record_miss();
        } else {
            drop(ratios);
            let mut ratios = self.pool_hit_ratios.write();
            let ratio = ratios.entry(pool_name.to_string())
                .or_insert_with(PoolHitRatio::new);
            ratio.record_miss();
        }
    }

    // Record page type access
    pub fn record_page_type(&self, page_type: PageType) {
        let types = self.page_type_dist.read();
        if let Some(counter) = types.get(&page_type) {
            counter.fetch_add(1, Ordering::Relaxed);
        } else {
            drop(types);
            let mut types = self.page_type_dist.write();
            types.entry(page_type)
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    // Get comprehensive statistics
    pub fn get_comprehensive_stats(&self) -> ComprehensiveBufferStats {
        let ratios = self.pool_hit_ratios.read();
        let pool_stats: HashMap<String, PoolStatsSnapshot> = ratios.iter()
            .map(|(name, ratio)| {
                (name.clone(), PoolStatsSnapshot {
                    hits: ratio.hits.load(Ordering::Relaxed),
                    misses: ratio.misses.load(Ordering::Relaxed),
                    accesses: ratio.accesses.load(Ordering::Relaxed),
                    hit_ratio: ratio.hit_ratio(),
                })
            })
            .collect();

        let types = self.page_type_dist.read();
        let page_type_distribution: HashMap<PageType, u64> = types.iter()
            .map(|(pt, count)| (*pt, count.load(Ordering::Relaxed)))
            .collect();

        ComprehensiveBufferStats {
            pool_stats,
            page_type_distribution,
            wait_stats: self.wait_stats.snapshot(),
            busy_waits: self.busy_waits.snapshot(),
            memory_pressure: self.memory_pressure.snapshot(),
            realtime_metrics: self.realtime_metrics.get(),
        }
    }

    // Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let stats = self.get_comprehensive_stats();
        let mut output = String::new();

        // Buffer pool hit ratios
        for (pool_name, pool_stats) in &stats.pool_stats {
            output.push_str(&format!(
                "buffer_pool_hit_ratio{{pool=\"{}\"}} {}\n",
                pool_name, pool_stats.hit_ratio
            ));
            output.push_str(&format!(
                "buffer_pool_accesses_total{{pool=\"{}\"}} {}\n",
                pool_name, pool_stats.accesses
            ));
        }

        // Page type distribution
        for (page_type, count) in &stats.page_type_distribution {
            output.push_str(&format!(
                "buffer_pool_pages_by_type{{type=\"{:?}\"}} {}\n",
                page_type, count
            ));
        }

        // Wait statistics
        output.push_str(&format!(
            "buffer_pool_free_buffer_waits_total {}\n",
            stats.wait_stats.free_buffer_waits
        ));
        output.push_str(&format!(
            "buffer_pool_io_waits_total {}\n",
            stats.wait_stats.io_waits
        ));

        // Memory pressure
        output.push_str(&format!(
            "buffer_pool_memory_usage_bytes {}\n",
            stats.memory_pressure.current_usage
        ));
        output.push_str(&format!(
            "buffer_pool_memory_pressure_level {}\n",
            stats.memory_pressure.pressure_level
        ));

        output
    }

    // Export metrics in JSON format
    pub fn export_json(&self) -> String {
        let stats = self.get_comprehensive_stats();
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatsSnapshot {
    pub hits: u64,
    pub misses: u64,
    pub accesses: u64,
    pub hit_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveBufferStats {
    pub pool_stats: HashMap<String, PoolStatsSnapshot>,
    pub page_type_distribution: HashMap<PageType, u64>,
    pub wait_stats: WaitStatisticsSnapshot,
    pub busy_waits: BusyWaitStatisticsSnapshot,
    pub memory_pressure: MemoryPressureSnapshot,
    pub realtime_metrics: MetricsSnapshot,
}
