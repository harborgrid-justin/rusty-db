// # RustyDB Benchmark Suite
//
// Comprehensive benchmark suite for measuring and optimizing critical database operations.
// This module provides detailed performance measurements for:
//
// - Page scan throughput
// - Index lookup latency
// - Buffer manager pin/unpin operations
// - Lock-free queue operations
// - SIMD filter operations
// - Transaction commit latency
// - Concurrent access patterns
// - Memory allocator performance
//
// ## Usage
//
// Run benchmarks with:
// ```bash
// cargo bench
// ```
//
// Run specific benchmark:
// ```bash
// cargo bench --bench page_scan
// ```
//
// ## Benchmark Categories
//
// ### Storage Layer Benchmarks
// - **Page Scan**: Sequential and random page access patterns
// - **Buffer Pool**: Pin/unpin cycles, cache hit rates, eviction performance
// - **Disk I/O**: Sequential/random reads and writes with various block sizes
//
// ### Index Benchmarks
// - **B-tree**: Lookup, insert, update, delete operations
// - **Hash Index**: Point lookups and range scans
// - **LSM Tree**: Write throughput and compaction performance
//
// ### Concurrency Benchmarks
// - **Lock-Free Structures**: Queue, stack, hash map operations
// - **MVCC**: Transaction start/commit overhead
// - **Lock Manager**: Lock acquisition and release latency
//
// ### SIMD Benchmarks
// - **Filter Operations**: Equality, inequality, range predicates
// - **Aggregations**: SUM, AVG, MIN, MAX operations
// - **String Operations**: Pattern matching and comparisons

use std::sync::Arc;
use std::time::{Instant, Duration};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::collections::{BTreeMap, HashMap};
use std::hint::black_box;
use std::thread;

// ============================================================================
// Benchmark Configuration
// ============================================================================

// Benchmark configuration parameters
#[derive(Debug, Clone)]
pub struct BenchConfig {
    // Number of iterations for each benchmark
    pub iterations: usize,
    // Number of warmup iterations
    pub warmup_iterations: usize,
    // Page size in bytes (default: 4KB)
    pub page_size: usize,
    // Number of pages for scan benchmarks
    pub num_pages: usize,
    // Buffer pool size in pages
    pub buffer_pool_size: usize,
    // Number of concurrent threads
    pub num_threads: usize,
    // Enable SIMD optimizations
    pub enable_simd: bool,
    // Enable detailed metrics collection
    pub collect_metrics: bool,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            iterations: 10_000,
            warmup_iterations: 1_000,
            page_size: 4096,
            num_pages: 10_000,
            buffer_pool_size: 1_000,
            num_threads: num_cpus::get(),
            enable_simd: cfg!(target_feature = "avx2"),
            collect_metrics: true,
        }
    }
}

// ============================================================================
// Performance Metrics
// ============================================================================

// Detailed performance metrics for benchmarks
#[derive(Debug)]
pub struct BenchMetrics {
    // Total operations executed
    pub total_ops: AtomicU64,
    // Total bytes processed
    pub total_bytes: AtomicU64,
    // Number of cache hits
    pub cache_hits: AtomicU64,
    // Number of cache misses
    pub cache_misses: AtomicU64,
    // Total latency in nanoseconds
    pub total_latency_ns: AtomicU64,
    // Minimum latency observed
    pub min_latency_ns: AtomicU64,
    // Maximum latency observed
    pub max_latency_ns: AtomicU64,
    // Number of errors
    pub errors: AtomicU64,
}

impl Clone for BenchMetrics {
    fn clone(&self) -> Self {
        Self {
            total_ops: AtomicU64::new(self.total_ops.load(Ordering::Relaxed)),
            total_bytes: AtomicU64::new(self.total_bytes.load(Ordering::Relaxed)),
            cache_hits: AtomicU64::new(self.cache_hits.load(Ordering::Relaxed)),
            cache_misses: AtomicU64::new(self.cache_misses.load(Ordering::Relaxed)),
            total_latency_ns: AtomicU64::new(self.total_latency_ns.load(Ordering::Relaxed)),
            min_latency_ns: AtomicU64::new(self.min_latency_ns.load(Ordering::Relaxed)),
            max_latency_ns: AtomicU64::new(self.max_latency_ns.load(Ordering::Relaxed)),
            errors: AtomicU64::new(self.errors.load(Ordering::Relaxed)),
        }
    }
}

impl Default for BenchMetrics {
    fn default() -> Self {
        Self {
            total_ops: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
            min_latency_ns: AtomicU64::new(u64::MAX),
            max_latency_ns: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }
}

impl BenchMetrics {
    // Record an operation with its latency
    pub fn record_op(&self, latency_ns: u64, bytes: u64) {
        self.total_ops.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.total_latency_ns.fetch_add(latency_ns, Ordering::Relaxed);

        // Update min latency
        let mut min = self.min_latency_ns.load(Ordering::Relaxed);
        while latency_ns < min {
            match self.min_latency_ns.compare_exchange_weak(
                min,
                latency_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => min = x,
            }
        }

        // Update max latency
        let mut max = self.max_latency_ns.load(Ordering::Relaxed);
        while latency_ns > max {
            match self.max_latency_ns.compare_exchange_weak(
                max,
                latency_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => max = x,
            }
        }
    }

    // Record a cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    // Record a cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    // Record an error
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    // Calculate average latency in nanoseconds
    pub fn avg_latency_ns(&self) -> f64 {
        let total_ops = self.total_ops.load(Ordering::Relaxed);
        if total_ops == 0 {
            return 0.0;
        }
        let total_latency = self.total_latency_ns.load(Ordering::Relaxed);
        total_latency as f64 / total_ops as f64
    }

    // Calculate cache hit rate as percentage
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            return 0.0;
        }
        (hits as f64 / total as f64) * 100.0
    }

    // Calculate throughput in ops/sec
    pub fn throughput(&self, duration_secs: f64) -> f64 {
        let total_ops = self.total_ops.load(Ordering::Relaxed);
        total_ops as f64 / duration_secs
    }

    // Calculate bandwidth in MB/s
    pub fn bandwidth_mbps(&self, duration_secs: f64) -> f64 {
        let total_bytes = self.total_bytes.load(Ordering::Relaxed);
        (total_bytes as f64 / duration_secs) / (1024.0 * 1024.0)
    }

    // Print detailed metrics summary
    pub fn print_summary(&self, duration: Duration) {
        let duration_secs = duration.as_secs_f64();
        println!("\n=== Benchmark Metrics ===");
        println!("Total Operations: {}", self.total_ops.load(Ordering::Relaxed));
        println!("Total Bytes: {} MB", self.total_bytes.load(Ordering::Relaxed) / (1024 * 1024));
        println!("Duration: {:.2}s", duration_secs);
        println!("Throughput: {:.2} ops/sec", self.throughput(duration_secs));
        println!("Bandwidth: {:.2} MB/s", self.bandwidth_mbps(duration_secs));
        println!("Avg Latency: {:.2} µs", self.avg_latency_ns() / 1000.0);
        println!("Min Latency: {:.2} µs", self.min_latency_ns.load(Ordering::Relaxed) as f64 / 1000.0);
        println!("Max Latency: {:.2} µs", self.max_latency_ns.load(Ordering::Relaxed) as f64 / 1000.0);
        println!("Cache Hit Rate: {:.2}%", self.cache_hit_rate());
        println!("Errors: {}", self.errors.load(Ordering::Relaxed));
    }
}

// ============================================================================
// Mock Data Structures for Benchmarking
// ============================================================================

// Mock page structure for storage benchmarks
#[repr(align(4096))]
pub struct Page {
    pub data: [u8; 4096],
    pub page_id: u32,
    pub pin_count: AtomicU32,
    pub dirty: AtomicBool,
}

impl Page {
    pub fn new(page_id: u32) -> Self {
        Self {
            data: [0u8; 4096],
            page_id,
            pin_count: AtomicU32::new(0),
            dirty: AtomicBool::new(false),
        }
    }

    pub fn pin(&self) {
        self.pin_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn unpin(&self) {
        self.pin_count.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn is_pinned(&self) -> bool {
        self.pin_count.load(Ordering::Relaxed) > 0
    }

    pub fn set_dirty(&self) {
        self.dirty.store(true, Ordering::Relaxed);
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Relaxed)
    }
}

// Mock buffer pool for buffer manager benchmarks
pub struct MockBufferPool {
    pages: Vec<Arc<Page>>,
    page_table: parking_lot::RwLock<HashMap<u32, usize>>,
    config: BenchConfig,
    metrics: BenchMetrics,
}

impl MockBufferPool {
    pub fn new(config: BenchConfig) -> Self {
        let mut pages = Vec::with_capacity(config.buffer_pool_size);
        for i in 0..config.buffer_pool_size {
            pages.push(Arc::new(Page::new(i as u32)));
        }

        let mut page_table = HashMap::new();
        for (idx, page) in pages.iter().enumerate() {
            page_table.insert(page.page_id, idx);
        }

        Self {
            pages,
            page_table: parking_lot::RwLock::new(page_table),
            config,
            metrics: BenchMetrics::default(),
        }
    }

    pub fn pin_page(&self, page_id: u32) -> Option<Arc<Page>> {
        let start = Instant::now();

        let table = self.page_table.read();
        let result = if let Some(&idx) = table.get(&page_id) {
            self.metrics.record_cache_hit();
            let page = self.pages[idx].clone();
            page.pin();
            Some(page)
        } else {
            self.metrics.record_cache_miss();
            None
        };

        let latency = start.elapsed().as_nanos() as u64;
        self.metrics.record_op(latency, self.config.page_size as u64);

        result
    }

    pub fn unpin_page(&self, page: &Arc<Page>) {
        page.unpin();
    }

    pub fn get_metrics(&self) -> &BenchMetrics {
        &self.metrics
    }
}

// Mock lock-free queue for concurrency benchmarks
pub struct MockLockFreeQueue<T> {
    queue: crossbeam::queue::SegQueue<T>,
    metrics: BenchMetrics,
}

impl<T> MockLockFreeQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: crossbeam::queue::SegQueue::new(),
            metrics: BenchMetrics::default(),
        }
    }

    pub fn push(&self, value: T) {
        let start = Instant::now();
        self.queue.push(value);
        let latency = start.elapsed().as_nanos() as u64;
        self.metrics.record_op(latency, size_of::<T>() as u64);
    }

    pub fn pop(&self) -> Option<T> {
        let start = Instant::now();
        let result = self.queue.pop();
        let latency = start.elapsed().as_nanos() as u64;
        self.metrics.record_op(latency, size_of::<T>() as u64);
        result
    }

    pub fn get_metrics(&self) -> &BenchMetrics {
        &self.metrics
    }
}

// ============================================================================
// Page Scan Benchmarks
// ============================================================================

// Sequential page scan benchmark
pub fn bench_sequential_page_scan(config: &BenchConfig) -> BenchMetrics {
    let metrics = BenchMetrics::default();
    let pages: Vec<Page> = (0..config.num_pages)
        .map(|i| Page::new(i as u32))
        .collect();

    let start = Instant::now();

    for _ in 0..config.iterations {
        for page in &pages {
            let page_start = Instant::now();

            // Simulate scanning page data
            let mut sum: u64 = 0;
            for &byte in &page.data {
                sum = sum.wrapping_add(byte as u64);
            }
            black_box(sum);

            let latency = page_start.elapsed().as_nanos() as u64;
            metrics.record_op(latency, config.page_size as u64);
        }
    }

    metrics.print_summary(start.elapsed());
    metrics
}

// Random page scan benchmark
pub fn bench_random_page_scan(config: &BenchConfig) -> BenchMetrics {
    use rand::Rng;

    let metrics = BenchMetrics::default();
    let pages: Vec<Page> = (0..config.num_pages)
        .map(|i| Page::new(i as u32))
        .collect();

    let mut rng = rand::rng();
    let start = Instant::now();

    for _ in 0..config.iterations {
        let idx = rng.random_range(0..config.num_pages);
        let page = &pages[idx];

        let page_start = Instant::now();

        // Simulate random page access
        let mut sum: u64 = 0;
        for &byte in &page.data {
            sum = sum.wrapping_add(byte as u64);
        }
        black_box(sum);

        let latency = page_start.elapsed().as_nanos() as u64;
        metrics.record_op(latency, config.page_size as u64);
    }

    metrics.print_summary(start.elapsed());
    metrics
}

// Page scan with predicate filtering
pub fn bench_filtered_page_scan(config: &BenchConfig) -> BenchMetrics {
    let metrics = BenchMetrics::default();
    let pages: Vec<Page> = (0..config.num_pages)
        .map(|i| Page::new(i as u32))
        .collect();

    let start = Instant::now();

    for _ in 0..config.iterations {
        for page in &pages {
            let page_start = Instant::now();

            // Simulate filtering predicate (e.g., value > 128)
            let mut count = 0;
            for &byte in &page.data {
                if byte > 128 {
                    count += 1;
                }
            }
            black_box(count);

            let latency = page_start.elapsed().as_nanos() as u64;
            metrics.record_op(latency, config.page_size as u64);
        }
    }

    metrics.print_summary(start.elapsed());
    metrics
}

// ============================================================================
// Index Lookup Benchmarks
// ============================================================================

// B-tree index lookup benchmark
pub fn bench_btree_lookup(config: &BenchConfig) -> BenchMetrics {
    use std::collections::BTreeMap;

    let metrics = BenchMetrics::default();
    let mut btree = BTreeMap::new();

    // Populate index
    for i in 0..config.num_pages {
        btree.insert(i as u64, vec![i as u32]);
    }

    let start = Instant::now();

    for i in 0..config.iterations {
        let key = (i % config.num_pages) as u64;
        let lookup_start = Instant::now();

        let result = btree.get(&key);
        black_box(result);

        let latency = lookup_start.elapsed().as_nanos() as u64;
        metrics.record_op(latency, 8); // 8 bytes for u64 key

        if result.is_some() {
            metrics.record_cache_hit();
        } else {
            metrics.record_cache_miss();
        }
    }

    metrics.print_summary(start.elapsed());
    metrics
}

// Hash index lookup benchmark
pub fn bench_hash_lookup(config: &BenchConfig) -> BenchMetrics {
    let metrics = BenchMetrics::default();
    let mut hash_map = HashMap::new();

    // Populate index
    for i in 0..config.num_pages {
        hash_map.insert(i as u64, vec![i as u32]);
    }

    let start = Instant::now();

    for i in 0..config.iterations {
        let key = (i % config.num_pages) as u64;
        let lookup_start = Instant::now();

        let result = hash_map.get(&key);
        black_box(result);

        let latency = lookup_start.elapsed().as_nanos() as u64;
        metrics.record_op(latency, 8);

        if result.is_some() {
            metrics.record_cache_hit();
        } else {
            metrics.record_cache_miss();
        }
    }

    metrics.print_summary(start.elapsed());
    metrics
}

// Range scan benchmark (B-tree)
pub fn bench_range_scan(config: &BenchConfig) -> BenchMetrics {

    let metrics = BenchMetrics::default();
    let mut btree = BTreeMap::new();

    // Populate index
    for i in 0..config.num_pages {
        btree.insert(i as u64, vec![i as u32]);
    }

    let start = Instant::now();
    let range_size = 100;

    for i in 0..(config.iterations / range_size) {
        let start_key = ((i * range_size) % config.num_pages) as u64;
        let end_key = start_key + range_size as u64;

        let scan_start = Instant::now();

        let mut count = 0;
        for (_key, _value) in btree.range(start_key..end_key) {
            count += 1;
        }
        black_box(count);

        let latency = scan_start.elapsed().as_nanos() as u64;
        metrics.record_op(latency, (range_size * 8) as u64);
    }

    metrics.print_summary(start.elapsed());
    metrics
}

// ============================================================================
// Buffer Manager Benchmarks
// ============================================================================

// Pin/unpin cycle benchmark
pub fn bench_pin_unpin_cycles(config: &BenchConfig) -> BenchMetrics {
    let buffer_pool = MockBufferPool::new(config.clone());

    let start = Instant::now();

    for i in 0..config.iterations {
        let page_id = (i % config.buffer_pool_size) as u32;

        if let Some(page) = buffer_pool.pin_page(page_id) {
            // Simulate some work
            black_box(&page.data[0..64]);
            buffer_pool.unpin_page(&page);
        }
    }

    let metrics = buffer_pool.get_metrics();
    metrics.print_summary(start.elapsed());
    metrics.clone()
}

// Concurrent buffer pool access benchmark
pub fn bench_concurrent_buffer_access(config: &BenchConfig) -> BenchMetrics {
    use std::thread;

    let buffer_pool = Arc::new(MockBufferPool::new(config.clone()));
    let num_threads = config.num_threads;
    let iterations_per_thread = config.iterations / num_threads;

    let start = Instant::now();
    let mut handles = vec![];

    for thread_id in 0..num_threads {
        let pool = buffer_pool.clone();
        let buffer_size = config.buffer_pool_size;

        let handle = thread::spawn(move || {
            for i in 0..iterations_per_thread {
                let page_id = ((thread_id * 1000 + i) % buffer_size) as u32;

                if let Some(page) = pool.pin_page(page_id) {
                    black_box(&page.data[0..64]);
                    pool.unpin_page(&page);
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let metrics = buffer_pool.get_metrics();
    metrics.print_summary(start.elapsed());
    metrics.clone()
}

// Buffer eviction benchmark
pub fn bench_buffer_eviction(config: &BenchConfig) -> BenchMetrics {
    // Simulate LRU eviction by accessing more pages than buffer pool size
    let buffer_pool = MockBufferPool::new(config.clone());
    let num_unique_pages = config.buffer_pool_size * 2; // Force evictions

    let start = Instant::now();

    for i in 0..config.iterations {
        let page_id = (i % num_unique_pages) as u32;

        if let Some(page) = buffer_pool.pin_page(page_id) {
            black_box(&page.data[0..64]);
            buffer_pool.unpin_page(&page);
        } else {
            // Simulate page load from disk (cache miss)
            thread::sleep(Duration::from_micros(10));
        }
    }

    let metrics = buffer_pool.get_metrics();
    metrics.print_summary(start.elapsed());
    metrics.clone()
}

// ============================================================================
// Lock-Free Queue Benchmarks
// ============================================================================

// Single-threaded queue operations
pub fn bench_queue_single_threaded(config: &BenchConfig) -> BenchMetrics {
    let queue = MockLockFreeQueue::<u64>::new();

    let start = Instant::now();

    // Push phase
    for i in 0..config.iterations {
        queue.push(i as u64);
    }

    // Pop phase
    for _ in 0..config.iterations {
        black_box(queue.pop());
    }

    let metrics = queue.get_metrics();
    metrics.print_summary(start.elapsed());
    metrics.clone()
}

// Multi-threaded queue operations (producer-consumer)
pub fn bench_queue_multi_threaded(config: &BenchConfig) -> BenchMetrics {

    let queue = Arc::new(MockLockFreeQueue::<u64>::new());
    let num_producers = config.num_threads / 2;
    let num_consumers = config.num_threads / 2;
    let items_per_producer = config.iterations / num_producers;

    let start = Instant::now();
    let mut handles = vec![];

    // Spawn producers
    for _ in 0..num_producers {
        let q = queue.clone();
        let handle = thread::spawn(move || {
            for i in 0..items_per_producer {
                q.push(i as u64);
            }
        });
        handles.push(handle);
    }

    // Spawn consumers
    for _ in 0..num_consumers {
        let q = queue.clone();
        let handle = thread::spawn(move || {
            let mut consumed = 0;
            while consumed < items_per_producer {
                if q.pop().is_some() {
                    consumed += 1;
                } else {
                    thread::yield_now();
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let metrics = queue.get_metrics();
    metrics.print_summary(start.elapsed());
    metrics.clone()
}

// ============================================================================
// SIMD Filter Benchmarks
// ============================================================================

// SIMD equality filter (i32)
pub fn bench_simd_filter_eq(config: &BenchConfig) -> BenchMetrics {
    let metrics = BenchMetrics::default();
    let data: Vec<i32> = (0..config.num_pages * 1024)
        .map(|i| (i % 1000) as i32)
        .collect();

    let target = 500i32;
    let start = Instant::now();

    for _ in 0..config.iterations {
        let filter_start = Instant::now();

        #[cfg(target_feature = "avx2")]
        {
            if config.enable_simd {
                // SIMD path would go here
                // For now, use scalar fallback
                let mut count = 0;
                for &value in &data {
                    if value == target {
                        count += 1;
                    }
                }
                black_box(count);
            }
        }

        // Scalar fallback
        #[cfg(not(target_feature = "avx2"))]
        {
            let mut count = 0;
            for &value in &data {
                if value == target {
                    count += 1;
                }
            }
            black_box(count);
        }

        let latency = filter_start.elapsed().as_nanos() as u64;
        metrics.record_op(latency, (data.len() * 4) as u64);
    }

    metrics.print_summary(start.elapsed());
    metrics
}

// SIMD range filter (i32)
pub fn bench_simd_filter_range(config: &BenchConfig) -> BenchMetrics {
    let metrics = BenchMetrics::default();
    let data: Vec<i32> = (0..config.num_pages * 1024)
        .map(|i| (i % 1000) as i32)
        .collect();

    let min = 200i32;
    let max = 800i32;
    let start = Instant::now();

    for _ in 0..config.iterations {
        let filter_start = Instant::now();

        let mut count = 0;
        for &value in &data {
            if value >= min && value <= max {
                count += 1;
            }
        }
        black_box(count);

        let latency = filter_start.elapsed().as_nanos() as u64;
        metrics.record_op(latency, (data.len() * 4) as u64);
    }

    metrics.print_summary(start.elapsed());
    metrics
}

// SIMD aggregation (SUM)
pub fn bench_simd_aggregate_sum(config: &BenchConfig) -> BenchMetrics {
    let metrics = BenchMetrics::default();
    let data: Vec<i64> = (0..config.num_pages * 1024)
        .map(|i| i as i64)
        .collect();

    let start = Instant::now();

    for _ in 0..config.iterations {
        let agg_start = Instant::now();

        let mut sum: i64 = 0;
        for &value in &data {
            sum = sum.wrapping_add(value);
        }
        black_box(sum);

        let latency = agg_start.elapsed().as_nanos() as u64;
        metrics.record_op(latency, (data.len() * 8) as u64);
    }

    metrics.print_summary(start.elapsed());
    metrics
}

// SIMD aggregation (MIN/MAX)
pub fn bench_simd_aggregate_minmax(config: &BenchConfig) -> BenchMetrics {
    let metrics = BenchMetrics::default();
    let data: Vec<i32> = (0..config.num_pages * 1024)
        .map(|i| (i % 10000) as i32)
        .collect();

    let start = Instant::now();

    for _ in 0..config.iterations {
        let agg_start = Instant::now();

        let mut min = i32::MAX;
        let mut max = i32::MIN;

        for &value in &data {
            if value < min {
                min = value;
            }
            if value > max {
                max = value;
            }
        }
        black_box((min, max));

        let latency = agg_start.elapsed().as_nanos() as u64;
        metrics.record_op(latency, (data.len() * 4) as u64);
    }

    metrics.print_summary(start.elapsed());
    metrics
}

// ============================================================================
// Transaction Benchmarks
// ============================================================================

// Transaction begin/commit overhead
pub fn bench_transaction_overhead(config: &BenchConfig) -> BenchMetrics {
    let metrics = BenchMetrics::default();
    let txn_counter = AtomicU64::new(0);

    let start = Instant::now();

    for _ in 0..config.iterations {
        let txn_start = Instant::now();

        // Simulate transaction begin
        let txn_id = txn_counter.fetch_add(1, Ordering::SeqCst);
        black_box(txn_id);

        // Simulate minimal work
        thread::sleep(Duration::from_nanos(100));

        // Simulate transaction commit
        black_box(txn_id);

        let latency = txn_start.elapsed().as_nanos() as u64;
        metrics.record_op(latency, 0);
    }

    metrics.print_summary(start.elapsed());
    metrics
}

// MVCC version chain traversal
pub fn bench_mvcc_traversal(config: &BenchConfig) -> BenchMetrics {
    let metrics = BenchMetrics::default();

    // Simulate version chain
    #[derive(Clone)]
    struct Version {
        txn_id: u64,
        data: [u8; 128],
        prev: Option<Box<Version>>,
    }

    let mut head = Version {
        txn_id: 100,
        data: [0u8; 128],
        prev: None,
    };

    // Build chain of 10 versions
    for i in 0..10 {
        head = Version {
            txn_id: 100 - i,
            data: [i as u8; 128],
            prev: Some(Box::new(head)),
        };
    }

    let start = Instant::now();

    for _ in 0..config.iterations {
        let traverse_start = Instant::now();

        // Traverse to find visible version
        let target_txn = 95u64;
        let mut current = &head;
        let mut found = false;

        loop {
            if current.txn_id <= target_txn {
                black_box(&current.data);
                found = true;
                break;
            }

            if let Some(ref prev) = current.prev {
                current = prev;
            } else {
                break;
            }
        }

        black_box(found);

        let latency = traverse_start.elapsed().as_nanos() as u64;
        metrics.record_op(latency, 128);
    }

    metrics.print_summary(start.elapsed());
    metrics
}

// ============================================================================
// Memory Allocation Benchmarks
// ============================================================================

// Small allocation benchmark (< 1KB)
pub fn bench_small_allocations(config: &BenchConfig) -> BenchMetrics {
    let metrics = BenchMetrics::default();

    let start = Instant::now();

    for _ in 0..config.iterations {
        let alloc_start = Instant::now();

        let data = vec![0u8; 256];
        black_box(data);

        let latency = alloc_start.elapsed().as_nanos() as u64;
        metrics.record_op(latency, 256);
    }

    metrics.print_summary(start.elapsed());
    metrics
}

// Large allocation benchmark (> 1MB)
pub fn bench_large_allocations(config: &BenchConfig) -> BenchMetrics {
    let metrics = BenchMetrics::default();
    let alloc_size = 1024 * 1024; // 1MB

    let start = Instant::now();

    for _ in 0..(config.iterations / 100) { // Reduce iterations for large allocs
        let alloc_start = Instant::now();

        let data = vec![0u8; alloc_size];
        black_box(data);

        let latency = alloc_start.elapsed().as_nanos() as u64;
        metrics.record_op(latency, alloc_size as u64);
    }

    metrics.print_summary(start.elapsed());
    metrics
}

// ============================================================================
// Benchmark Suite Runner
// ============================================================================

// Run all benchmarks with default configuration
pub fn run_all_benchmarks() {
    let config = BenchConfig::default();

    println!("\n╔════════════════════════════════════════════════════════╗");
    println!("║        RustyDB Performance Benchmark Suite            ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!("\nConfiguration:");
    println!("  Iterations: {}", config.iterations);
    println!("  Page Size: {} KB", config.page_size / 1024);
    println!("  Buffer Pool: {} pages", config.buffer_pool_size);
    println!("  Threads: {}", config.num_threads);
    println!("  SIMD: {}", if config.enable_simd { "enabled" } else { "disabled" });

    println!("\n[1/20] Sequential Page Scan...");
    bench_sequential_page_scan(&config);

    println!("\n[2/20] Random Page Scan...");
    bench_random_page_scan(&config);

    println!("\n[3/20] Filtered Page Scan...");
    bench_filtered_page_scan(&config);

    println!("\n[4/20] B-tree Lookup...");
    bench_btree_lookup(&config);

    println!("\n[5/20] Hash Lookup...");
    bench_hash_lookup(&config);

    println!("\n[6/20] Range Scan...");
    bench_range_scan(&config);

    println!("\n[7/20] Pin/Unpin Cycles...");
    bench_pin_unpin_cycles(&config);

    println!("\n[8/20] Concurrent Buffer Access...");
    bench_concurrent_buffer_access(&config);

    println!("\n[9/20] Buffer Eviction...");
    bench_buffer_eviction(&config);

    println!("\n[10/20] Queue Single-Threaded...");
    bench_queue_single_threaded(&config);

    println!("\n[11/20] Queue Multi-Threaded...");
    bench_queue_multi_threaded(&config);

    println!("\n[12/20] SIMD Filter Equality...");
    bench_simd_filter_eq(&config);

    println!("\n[13/20] SIMD Filter Range...");
    bench_simd_filter_range(&config);

    println!("\n[14/20] SIMD Aggregate SUM...");
    bench_simd_aggregate_sum(&config);

    println!("\n[15/20] SIMD Aggregate MIN/MAX...");
    bench_simd_aggregate_minmax(&config);

    println!("\n[16/20] Transaction Overhead...");
    bench_transaction_overhead(&config);

    println!("\n[17/20] MVCC Traversal...");
    bench_mvcc_traversal(&config);

    println!("\n[18/20] Small Allocations...");
    bench_small_allocations(&config);

    println!("\n[19/20] Large Allocations...");
    bench_large_allocations(&config);

    println!("\n[20/20] Complete!");
    println!("\n╔════════════════════════════════════════════════════════╗");
    println!("║           Benchmark Suite Completed                    ║");
    println!("╚════════════════════════════════════════════════════════╝\n");
}

// ============================================================================
// Criterion Integration (for cargo bench)
// ============================================================================

#[cfg(all(test, not(target_os = "unknown")))]
mod criterion_benches {
    use super::*;

    #[allow(unused_imports)]
    use criterion::{criterion_group, criterion_main, Criterion};

    fn criterion_page_scan(c: &mut Criterion) {
        let config = BenchConfig::default();

        c.bench_function("sequential_page_scan", |b| {
            b.iter(|| bench_sequential_page_scan(&config))
        });

        c.bench_function("random_page_scan", |b| {
            b.iter(|| bench_random_page_scan(&config))
        });
    }

    fn criterion_index_lookup(c: &mut Criterion) {
        let config = BenchConfig::default();

        c.bench_function("btree_lookup", |b| {
            b.iter(|| bench_btree_lookup(&config))
        });

        c.bench_function("hash_lookup", |b| {
            b.iter(|| bench_hash_lookup(&config))
        });
    }

    fn criterion_buffer_manager(c: &mut Criterion) {
        let config = BenchConfig::default();

        c.bench_function("pin_unpin_cycles", |b| {
            b.iter(|| bench_pin_unpin_cycles(&config))
        });
    }

    criterion_group!(benches, criterion_page_scan, criterion_index_lookup, criterion_buffer_manager);
    criterion_main!(benches);
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use crate::bench::{BenchConfig, BenchMetrics, Page};

    #[test]
    fn test_bench_config_default() {
        let config = BenchConfig::default();
        assert_eq!(config.page_size, 4096);
        assert!(config.iterations > 0);
    }

    #[test]
    fn test_bench_metrics() {
        let metrics = BenchMetrics::default();
        metrics.record_op(1000, 4096);
        metrics.record_cache_hit();

        assert_eq!(metrics.total_ops.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.cache_hits.load(Ordering::Relaxed), 1);
        assert!(metrics.avg_latency_ns() > 0.0);
    }

    #[test]
    fn test_page_pin_unpin() {
        let page = Page::new(1);
        assert!(!page.is_pinned());

        page.pin();
        assert!(page.is_pinned());

        page.unpin();
        assert!(!page.is_pinned());
    }
}
