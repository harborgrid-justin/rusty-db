// Performance Monitoring and Analysis

use super::common::*;

pub struct PerformanceCounter {
    // Fast path allocations (thread-local cache hits)
    fast_path: AtomicU64,
    // Medium path allocations (depot hits)
    medium_path: AtomicU64,
    // Slow path allocations (new slab/chunk)
    slow_path: AtomicU64,
    // Cache line conflicts
    cache_conflicts: AtomicU64,
    // TLB misses
    tlb_misses: AtomicU64,
    // Page faults
    page_faults: AtomicU64,
}

impl PerformanceCounter {
    pub fn new() -> Self {
        Self {
            fast_path: AtomicU64::new(0),
            medium_path: AtomicU64::new(0),
            slow_path: AtomicU64::new(0),
            cache_conflicts: AtomicU64::new(0),
            tlb_misses: AtomicU64::new(0),
            page_faults: AtomicU64::new(0),
        }
    }

    pub fn record_fast_path(&self) {
        self.fast_path.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_medium_path(&self) {
        self.medium_path.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_slow_path(&self) {
        self.slow_path.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> PerformanceStats {
        let fast = self.fast_path.load(Ordering::Relaxed);
        let medium = self.medium_path.load(Ordering::Relaxed);
        let slow = self.slow_path.load(Ordering::Relaxed);
        let total = fast + medium + slow;

        PerformanceStats {
            fast_path_count: fast,
            medium_path_count: medium,
            slow_path_count: slow,
            fast_path_ratio: if total > 0 { fast as f64 / total as f64 } else { 0.0 },
            cache_conflicts: self.cache_conflicts.load(Ordering::Relaxed),
            tlb_misses: self.tlb_misses.load(Ordering::Relaxed),
            page_faults: self.page_faults.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub fast_path_count: u64,
    pub medium_path_count: u64,
    pub slow_path_count: u64,
    pub fast_path_ratio: f64,
    pub cache_conflicts: u64,
    pub tlb_misses: u64,
    pub page_faults: u64,
}

// Memory access pattern analyzer
pub struct AccessPatternAnalyzer {
    // Recent allocations
    recent_allocations: RwLock<VecDeque<AllocationRecord>>,
    // Temporal locality score
    temporal_locality: AtomicU64,
    // Spatial locality score
    spatial_locality: AtomicU64,
    // Sequential access count
    sequential_access: AtomicU64,
    // Random access count
    random_access: AtomicU64,
}

#[derive(Debug, Clone)]
struct AllocationRecord {
    address: usize,
    size: usize,
    timestamp: Instant,
}

impl AccessPatternAnalyzer {
    pub fn new() -> Self {
        Self {
            recent_allocations: RwLock::new(VecDeque::with_capacity(1000)),
            temporal_locality: AtomicU64::new(0),
            spatial_locality: AtomicU64::new(0),
            sequential_access: AtomicU64::new(0),
            random_access: AtomicU64::new(0),
        }
    }

    pub fn record_allocation(&self, address: usize, size: usize) {
        let record = AllocationRecord {
            address,
            size,
            timestamp: Instant::now(),
        };

        let mut recent = self.recent_allocations.write().unwrap();

        // Analyze pattern
        if let Some(last) = recent.back() {
            let time_diff = record.timestamp.duration_since(last.timestamp);
            let addr_diff = (record.address as i64 - last.address as i64).abs() as usize;

            // Check temporal locality (reuse within 1ms)
            if time_diff < Duration::from_millis(1) {
                self.temporal_locality.fetch_add(1, Ordering::Relaxed);
            }

            // Check spatial locality (nearby addresses)
            if addr_diff < 4096 {
                self.spatial_locality.fetch_add(1, Ordering::Relaxed);
            }

            // Check sequential vs random
            if addr_diff == last.size {
                self.sequential_access.fetch_add(1, Ordering::Relaxed);
            } else {
                self.random_access.fetch_add(1, Ordering::Relaxed);
            }
        }

        recent.push_back(record);
        if recent.len() > 1000 {
            recent.pop_front();
        }
    }

    pub fn get_pattern_stats(&self) -> AccessPatternStats {
        let temporal = self.temporal_locality.load(Ordering::Relaxed);
        let spatial = self.spatial_locality.load(Ordering::Relaxed);
        let sequential = self.sequential_access.load(Ordering::Relaxed);
        let random = self.random_access.load(Ordering::Relaxed);
        let total_access = sequential + random;

        AccessPatternStats {
            temporal_locality_score: temporal,
            spatial_locality_score: spatial,
            sequential_access_ratio: if total_access > 0 {
                sequential as f64 / total_access as f64
            } else {
                0.0
            },
            random_access_ratio: if total_access > 0 {
                random as f64 / total_access as f64
            } else {
                0.0
            },
            recent_allocation_count: self.recent_allocations.read().unwrap().len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccessPatternStats {
    pub temporal_locality_score: u64,
    pub spatial_locality_score: u64,
    pub sequential_access_ratio: f64,
    pub random_access_ratio: f64,
    pub recent_allocation_count: usize,
}

// Memory bandwidth monitor
pub struct BandwidthMonitor {
    // Bytes allocated per second
    alloc_bandwidth: AtomicU64,
    // Bytes deallocated per second
    dealloc_bandwidth: AtomicU64,
    // Last measurement time
    last_measurement: RwLock<Instant>,
    // Total bytes allocated in current window
    window_allocated: AtomicU64,
    // Total bytes deallocated in current window
    window_deallocated: AtomicU64,
}

impl BandwidthMonitor {
    pub fn new() -> Self {
        Self {
            alloc_bandwidth: AtomicU64::new(0),
            dealloc_bandwidth: AtomicU64::new(0),
            last_measurement: RwLock::new(Instant::now()),
            window_allocated: AtomicU64::new(0),
            window_deallocated: AtomicU64::new(0),
        }
    }

    pub fn record_allocation(&self, size: u64) {
        self.window_allocated.fetch_add(size, Ordering::Relaxed);
        self.update_bandwidth();
    }

    pub fn record_deallocation(&self, size: u64) {
        self.window_deallocated.fetch_add(size, Ordering::Relaxed);
        self.update_bandwidth();
    }

    fn update_bandwidth(&self) {
        let mut last = self.last_measurement.write().unwrap();
        let now = Instant::now();
        let elapsed = now.duration_since(*last);

        if elapsed >= Duration::from_secs(1) {
            let alloc_bytes = self.window_allocated.swap(0, Ordering::Relaxed);
            let dealloc_bytes = self.window_deallocated.swap(0, Ordering::Relaxed);

            let elapsed_secs = elapsed.as_secs_f64();
            self.alloc_bandwidth.store(
                (alloc_bytes as f64 / elapsed_secs) as u64,
                Ordering::Relaxed
            );
            self.dealloc_bandwidth.store(
                (dealloc_bytes as f64 / elapsed_secs) as u64,
                Ordering::Relaxed
            );

            *last = now;
        }
    }

    pub fn get_bandwidth(&self) -> BandwidthStats {
        BandwidthStats {
            alloc_bandwidth_bytes_per_sec: self.alloc_bandwidth.load(Ordering::Relaxed),
            dealloc_bandwidth_bytes_per_sec: self.dealloc_bandwidth.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BandwidthStats {
    pub alloc_bandwidth_bytes_per_sec: u64,
    pub dealloc_bandwidth_bytes_per_sec: u64,
}

// ============================================================================
// WEB API INTEGRATION (200+ lines)
// ============================================================================
