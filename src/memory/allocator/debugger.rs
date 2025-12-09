//\! Memory Debugging and Profiling
//\! 
//\! Leak detection and memory profiling capabilities.

use super::common::*;


/// Allocation tracking entry
#[derive(Debug, Clone)]
struct AllocationEntry {
    /// Allocation address
    address: usize,
    /// Allocation size
    size: usize,
    /// Allocation source
    source: AllocationSource,
    /// Allocation timestamp
    timestamp: Instant,
    /// Stack trace (simplified)
    stack_trace: String,
    /// Guard before allocation
    guard_before: u64,
    /// Guard after allocation
    guard_after: u64,
}

/// Memory debugger and profiler
pub struct MemoryDebugger {
    /// Tracking enabled
    tracking_enabled: AtomicBool,
    /// Active allocations
    allocations: RwLock<HashMap<usize, AllocationEntry>>,
    /// Per-component statistics
    component_stats: RwLock<HashMap<AllocationSource, ComponentMemoryStats>>,
    /// Leak detection enabled
    leak_detection_enabled: AtomicBool,
    /// Use-after-free detection enabled
    uaf_detection_enabled: AtomicBool,
    /// Memory guards enabled
    guards_enabled: AtomicBool,
    /// Stack trace capture enabled
    stack_traces_enabled: AtomicBool,
    /// Statistics
    stats: DebugStats,
}

struct DebugStats {
    total_allocations: AtomicU64,
    total_deallocations: AtomicU64,
    leaks_detected: AtomicU64,
    uaf_detected: AtomicU64,
    corruption_detected: AtomicU64,
    stack_traces_captured: AtomicU64,
}

impl DebugStats {
    fn new() -> Self {
        Self {
            total_allocations: AtomicU64::new(0),
            total_deallocations: AtomicU64::new(0),
            leaks_detected: AtomicU64::new(0),
            uaf_detected: AtomicU64::new(0),
            corruption_detected: AtomicU64::new(0),
            stack_traces_captured: AtomicU64::new(0),
        }
    }
}

/// Per-component memory statistics
#[derive(Debug, Clone)]
struct ComponentMemoryStats {
    allocations: u64,
    deallocations: u64,
    bytes_allocated: u64,
    bytes_deallocated: u64,
    active_allocations: u64,
    active_bytes: u64,
    peak_allocations: u64,
    peak_bytes: u64,
}

impl ComponentMemoryStats {
    fn new() -> Self {
        Self {
            allocations: 0,
            deallocations: 0,
            bytes_allocated: 0,
            bytes_deallocated: 0,
            active_allocations: 0,
            active_bytes: 0,
            peak_allocations: 0,
            peak_bytes: 0,
        }
    }
}

impl MemoryDebugger {
    /// Create a new memory debugger
    pub fn new() -> Self {
        Self {
            tracking_enabled: AtomicBool::new(false),
            allocations: RwLock::new(HashMap::new()),
            component_stats: RwLock::new(HashMap::new()),
            leak_detection_enabled: AtomicBool::new(false),
            uaf_detection_enabled: AtomicBool::new(false),
            guards_enabled: AtomicBool::new(false),
            stack_traces_enabled: AtomicBool::new(false),
            stats: DebugStats::new(),
        }
    }

    /// Enable tracking
    pub fn enable_tracking(&self) {
        self.tracking_enabled.store(true, Ordering::Relaxed);
    }

    /// Disable tracking
    pub fn disable_tracking(&self) {
        self.tracking_enabled.store(false, Ordering::Relaxed);
    }

    /// Enable leak detection
    pub fn enable_leak_detection(&self) {
        self.leak_detection_enabled.store(true, Ordering::Relaxed);
        self.enable_tracking();
    }

    /// Enable use-after-free detection
    pub fn enable_uaf_detection(&self) {
        self.uaf_detection_enabled.store(true, Ordering::Relaxed);
        self.enable_tracking();
    }

    /// Enable memory guards
    pub fn enable_guards(&self) {
        self.guards_enabled.store(true, Ordering::Relaxed);
        self.enable_tracking();
    }

    /// Enable stack trace capture
    pub fn enable_stack_traces(&self) {
        self.stack_traces_enabled.store(true, Ordering::Relaxed);
        self.stats.stack_traces_captured.fetch_add(1, Ordering::Relaxed);
    }

    /// Track allocation
    pub fn track_allocation(
        &self,
        address: usize,
        size: usize,
        source: AllocationSource,
    ) {
        if !self.tracking_enabled.load(Ordering::Relaxed) {
            return;
        }

        self.stats.total_allocations.fetch_add(1, Ordering::Relaxed);

        let stack_trace = if self.stack_traces_enabled.load(Ordering::Relaxed) {
            self.capture_stack_trace()
        } else {
            String::new()
        };

        let entry = AllocationEntry {
            address,
            size,
            source,
            timestamp: Instant::now(),
            stack_trace,
            guard_before: GUARD_PATTERN,
            guard_after: GUARD_PATTERN,
        };

        self.allocations.write().unwrap().insert(address, entry);

        // Update component stats
        let mut stats = self.component_stats.write().unwrap();
        let component_stat = stats.entry(source).or_insert_with(ComponentMemoryStats::new);

        component_stat.allocations += 1;
        component_stat.bytes_allocated += size as u64;
        component_stat.active_allocations += 1;
        component_stat.active_bytes += size as u64;

        if component_stat.active_allocations > component_stat.peak_allocations {
            component_stat.peak_allocations = component_stat.active_allocations;
        }
        if component_stat.active_bytes > component_stat.peak_bytes {
            component_stat.peak_bytes = component_stat.active_bytes;
        }
    }

    /// Track deallocation
    pub fn track_deallocation(&self, address: usize) -> Result<()> {
        if !self.tracking_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.stats.total_deallocations.fetch_add(1, Ordering::Relaxed);

        let mut allocations = self.allocations.write().unwrap();

        if let Some(entry) = allocations.remove(&address) {
            // Check guards if enabled
            if self.guards_enabled.load(Ordering::Relaxed) {
                if entry.guard_before != GUARD_PATTERN || entry.guard_after != GUARD_PATTERN {
                    self.stats.corruption_detected.fetch_add(1, Ordering::Relaxed);
                    return Err(DbError::Internal(
                        format!("Memory corruption detected at address 0x{:x}", address)
                    ));
                }
            }

            // Update component stats
            let mut stats = self.component_stats.write().unwrap();
            if let Some(component_stat) = stats.get_mut(&entry.source) {
                component_stat.deallocations += 1;
                component_stat.bytes_deallocated += entry.size as u64;
                component_stat.active_allocations = component_stat.active_allocations.saturating_sub(1);
                component_stat.active_bytes = component_stat.active_bytes.saturating_sub(entry.size as u64);
            }

            Ok(())
        } else if self.uaf_detection_enabled.load(Ordering::Relaxed) {
            self.stats.uaf_detected.fetch_add(1, Ordering::Relaxed);
            Err(DbError::Internal(
                format!("Use-after-free or double-free detected at address 0x{:x}", address)
            ))
        } else {
            Ok(())
        }
    }

    /// Capture stack trace
    fn capture_stack_trace(&self) -> String {
        // Note: Backtrace::capture() is nightly-only
        // For stable builds, we use a simple placeholder
        String::from("<stack trace not available in stable build>")
    }

    /// Detect memory leaks
    pub fn detect_leaks(&self, min_age: Duration) -> Vec<LeakReport> {
        if !self.leak_detection_enabled.load(Ordering::Relaxed) {
            return Vec::new();
        }

        let now = Instant::now();
        let allocations = self.allocations.read().unwrap();

        let leaks: Vec<LeakReport> = allocations
            .values()
            .filter(|entry| now.duration_since(entry.timestamp) >= min_age)
            .map(|entry| {
                self.stats.leaks_detected.fetch_add(1, Ordering::Relaxed);

                LeakReport {
                    address: entry.address,
                    size: entry.size,
                    source: entry.source,
                    allocated_at: SystemTime::now() - now.duration_since(entry.timestamp),
                    stack_trace: entry.stack_trace.clone(),
                }
            })
            .collect();

        leaks
    }

    /// Get component statistics
    pub fn get_component_stats(&self, source: AllocationSource) -> Option<ComponentMemoryStats> {
        self.component_stats.read().unwrap().get(&source).cloned()
    }

    /// Get all component statistics
    pub fn get_all_component_stats(&self) -> HashMap<AllocationSource, ComponentMemoryStats> {
        self.component_stats.read().unwrap().clone()
    }

    /// Generate memory usage report
    pub fn generate_report(&self) -> MemoryReport {
        let allocations = self.allocations.read().unwrap();
        let component_stats = self.component_stats.read().unwrap();

        let total_active_allocations = allocations.len() as u64;
        let total_active_bytes: usize = allocations.values().map(|e| e.size).sum();

        let mut component_breakdown = Vec::new();
        for (source, stats) in component_stats.iter() {
            component_breakdown.push(ComponentBreakdown {
                source: *source,
                active_allocations: stats.active_allocations,
                active_bytes: stats.active_bytes,
                peak_allocations: stats.peak_allocations,
                peak_bytes: stats.peak_bytes,
                total_allocations: stats.allocations,
                total_deallocations: stats.deallocations,
            });
        }

        component_breakdown.sort_by_key(|c| std::cmp::Reverse(c.active_bytes));

        MemoryReport {
            timestamp: SystemTime::now(),
            total_active_allocations,
            total_active_bytes: total_active_bytes as u64,
            component_breakdown,
            total_allocations: self.stats.total_allocations.load(Ordering::Relaxed),
            total_deallocations: self.stats.total_deallocations.load(Ordering::Relaxed),
            leaks_detected: self.stats.leaks_detected.load(Ordering::Relaxed),
            uaf_detected: self.stats.uaf_detected.load(Ordering::Relaxed),
            corruption_detected: self.stats.corruption_detected.load(Ordering::Relaxed),
        }
    }

    /// Get debugger statistics
    pub fn get_stats(&self) -> MemoryDebuggerStats {
        MemoryDebuggerStats {
            tracking_enabled: self.tracking_enabled.load(Ordering::Relaxed),
            leak_detection_enabled: self.leak_detection_enabled.load(Ordering::Relaxed),
            uaf_detection_enabled: self.uaf_detection_enabled.load(Ordering::Relaxed),
            guards_enabled: self.guards_enabled.load(Ordering::Relaxed),
            total_allocations: self.stats.total_allocations.load(Ordering::Relaxed),
            total_deallocations: self.stats.total_deallocations.load(Ordering::Relaxed),
            active_allocations: self.allocations.read().unwrap().len() as u64,
            leaks_detected: self.stats.leaks_detected.load(Ordering::Relaxed),
            uaf_detected: self.stats.uaf_detected.load(Ordering::Relaxed),
            corruption_detected: self.stats.corruption_detected.load(Ordering::Relaxed),
        }
    }

    /// Clear all tracked allocations (use with caution)
    pub fn clear_tracking(&self) {
        self.allocations.write().unwrap().clear();
        self.component_stats.write().unwrap().clear();
    }
}

/// Memory usage report
#[derive(Debug, Clone)]
pub struct MemoryReport {
    pub timestamp: SystemTime,
    pub total_active_allocations: u64,
    pub total_active_bytes: u64,
    pub component_breakdown: Vec<ComponentBreakdown>,
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub leaks_detected: u64,
    pub uaf_detected: u64,
    pub corruption_detected: u64,
}

/// Component memory breakdown
#[derive(Debug, Clone)]
pub struct ComponentBreakdown {
    pub source: AllocationSource,
    pub active_allocations: u64,
    pub active_bytes: u64,
    pub peak_allocations: u64,
    pub peak_bytes: u64,
    pub total_allocations: u64,
    pub total_deallocations: u64,
}

/// Memory debugger statistics
#[derive(Debug, Clone)]
pub struct MemoryDebuggerStats {
    pub tracking_enabled: bool,
    pub leak_detection_enabled: bool,
    pub uaf_detection_enabled: bool,
    pub guards_enabled: bool,
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub active_allocations: u64,
    pub leaks_detected: u64,
    pub uaf_detected: u64,
    pub corruption_detected: u64,
}

// ============================================================================
