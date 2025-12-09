//\! Memory Pressure Management
//\! 
//\! Global memory monitoring and OOM prevention.

use super::common::*;


/// Memory pressure callback type
pub type PressureCallback = Arc<dyn Fn(MemoryPressureLevel) -> Result<usize> + Send + Sync>;

/// Memory pressure event
#[derive(Debug, Clone)]
pub struct MemoryPressureEvent {
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Pressure level
    pub level: MemoryPressureLevel,
    /// Total memory
    pub total_memory: u64,
    /// Used memory
    pub used_memory: u64,
    /// Available memory
    pub available_memory: u64,
    /// Number of callbacks invoked
    pub callbacks_invoked: usize,
    /// Total bytes freed
    pub bytes_freed: u64,
}

/// Memory pressure manager
pub struct MemoryPressureManager {
    /// Total memory limit
    total_memory: AtomicU64,
    /// Current used memory
    used_memory: AtomicU64,
    /// Memory pressure callbacks
    callbacks: RwLock<Vec<PressureCallback>>,
    /// Current pressure level
    current_level: RwLock<MemoryPressureLevel>,
    /// Pressure events history
    events: RwLock<VecDeque<MemoryPressureEvent>>,
    /// Emergency mode flag
    emergency_mode: AtomicBool,
    /// Statistics
    stats: PressureStats,
    /// Monitoring enabled
    monitoring_enabled: AtomicBool,
}

struct PressureStats {
    pressure_events: AtomicU64,
    callbacks_invoked: AtomicU64,
    emergency_releases: AtomicU64,
    total_freed: AtomicU64,
    oom_prevented: AtomicU64,
}

impl PressureStats {
    fn new() -> Self {
        Self {
            pressure_events: AtomicU64::new(0),
            callbacks_invoked: AtomicU64::new(0),
            emergency_releases: AtomicU64::new(0),
            total_freed: AtomicU64::new(0),
            oom_prevented: AtomicU64::new(0),
        }
    }
}

impl MemoryPressureManager {
    /// Create a new memory pressure manager
    pub fn new(total_memory: u64) -> Self {
        Self {
            total_memory: AtomicU64::new(total_memory),
            used_memory: AtomicU64::new(0),
            callbacks: RwLock::new(Vec::new()),
            current_level: RwLock::new(MemoryPressureLevel::Normal),
            events: RwLock::new(VecDeque::new()),
            emergency_mode: AtomicBool::new(false),
            stats: PressureStats::new(),
            monitoring_enabled: AtomicBool::new(true),
        }
    }

    /// Register a pressure callback
    pub fn register_callback(&self, callback: PressureCallback) {
        self.callbacks.write().unwrap().push(callback);
    }

    /// Record memory allocation
    pub fn record_allocation(&self, size: u64) -> Result<()> {
        if !self.monitoring_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        let new_used = self.used_memory.fetch_add(size, Ordering::SeqCst) + size;
        let total = self.total_memory.load(Ordering::Relaxed);

        let usage_ratio = new_used as f64 / total as f64;
        let new_level = self.calculate_pressure_level(usage_ratio);

        let mut current_level = self.current_level.write().unwrap();

        if new_level != *current_level {
            *current_level = new_level;
            drop(current_level);

            self.handle_pressure_change(new_level)?;
        }

        Ok(())
    }

    /// Record memory deallocation
    pub fn record_deallocation(&self, size: u64) {
        if !self.monitoring_enabled.load(Ordering::Relaxed) {
            return;
        }

        self.used_memory.fetch_sub(size, Ordering::SeqCst);

        let used = self.used_memory.load(Ordering::Relaxed);
        let total = self.total_memory.load(Ordering::Relaxed);
        let usage_ratio = used as f64 / total as f64;

        let new_level = self.calculate_pressure_level(usage_ratio);
        let mut current_level = self.current_level.write().unwrap();

        if new_level != *current_level && usage_ratio < MEMORY_PRESSURE_WARNING {
            *current_level = new_level;

            if self.emergency_mode.load(Ordering::Relaxed) {
                self.emergency_mode.store(false, Ordering::Relaxed);
            }
        }
    }

    /// Calculate pressure level from usage ratio
    fn calculate_pressure_level(&self, usage_ratio: f64) -> MemoryPressureLevel {
        if usage_ratio >= 0.95 {
            MemoryPressureLevel::Emergency
        } else if usage_ratio >= MEMORY_PRESSURE_CRITICAL {
            MemoryPressureLevel::Critical
        } else if usage_ratio >= MEMORY_PRESSURE_WARNING {
            MemoryPressureLevel::Warning
        } else {
            MemoryPressureLevel::Normal
        }
    }

    /// Handle pressure level change
    fn handle_pressure_change(&self, new_level: MemoryPressureLevel) -> Result<()> {
        self.stats.pressure_events.fetch_add(1, Ordering::Relaxed);

        // Invoke callbacks
        let callbacks = self.callbacks.read().unwrap();
        let mut total_freed = 0u64;
        let mut callbacks_invoked: usize = 0;

        for callback in callbacks.iter() {
            match callback(new_level) {
                Ok(freed) => {
                    total_freed += freed as u64;
                    callbacks_invoked += 1;
                }
                Err(e) => {
                    eprintln!("Pressure callback failed: {}", e);
                }
            }
        }

        drop(callbacks);

        self.stats.callbacks_invoked.fetch_add(callbacks_invoked as u64, Ordering::Relaxed);
        self.stats.total_freed.fetch_add(total_freed, Ordering::Relaxed);

        // Record event
        let event = MemoryPressureEvent {
            timestamp: SystemTime::now(),
            level: new_level,
            total_memory: self.total_memory.load(Ordering::Relaxed),
            used_memory: self.used_memory.load(Ordering::Relaxed),
            available_memory: self.total_memory.load(Ordering::Relaxed)
                - self.used_memory.load(Ordering::Relaxed),
            callbacks_invoked,
            bytes_freed: total_freed,
        };

        let mut events = self.events.write().unwrap();
        events.push_back(event);

        // Keep only last 1000 events
        while events.len() > 1000 {
            events.pop_front();
        }

        // Enter emergency mode if critical
        if new_level == MemoryPressureLevel::Emergency {
            self.emergency_mode.store(true, Ordering::Relaxed);
            self.emergency_release()?;
        }

        Ok(())
    }

    /// Emergency memory release
    pub(crate) fn emergency_release(&self) -> Result<()> {
        self.stats.emergency_releases.fetch_add(1, Ordering::Relaxed);

        // Trigger all callbacks with emergency level
        let callbacks = self.callbacks.read().unwrap();
        let mut total_freed = 0u64;

        for callback in callbacks.iter() {
            if let Ok(freed) = callback(MemoryPressureLevel::Emergency) {
                total_freed += freed as u64;
            }
        }

        if total_freed > 0 {
            self.stats.oom_prevented.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Check if allocation would cause OOM
    pub fn check_allocation(&self, size: u64) -> Result<()> {
        let used = self.used_memory.load(Ordering::Relaxed);
        let total = self.total_memory.load(Ordering::Relaxed);

        if used + size > total {
            // Try emergency release
            self.emergency_release()?;

            let used = self.used_memory.load(Ordering::Relaxed);
            if used + size > total {
                return Err(DbError::OutOfMemory(
                    format!("Cannot allocate {} bytes (used: {}, total: {})", size, used, total)
                ));
            }
        }

        Ok(())
    }

    /// Set total memory limit
    pub fn set_total_memory(&self, total: u64) {
        self.total_memory.store(total, Ordering::Relaxed);
    }

    /// Get current pressure level
    pub fn get_pressure_level(&self) -> MemoryPressureLevel {
        *self.current_level.read().unwrap()
    }

    /// Get memory usage
    pub fn get_usage(&self) -> MemoryUsage {
        let total = self.total_memory.load(Ordering::Relaxed);
        let used = self.used_memory.load(Ordering::Relaxed);

        MemoryUsage {
            total_memory: total,
            used_memory: used,
            available_memory: total.saturating_sub(used),
            usage_ratio: used as f64 / total as f64,
            pressure_level: self.get_pressure_level(),
            emergency_mode: self.emergency_mode.load(Ordering::Relaxed),
        }
    }

    /// Get pressure statistics
    pub fn get_stats(&self) -> MemoryPressureStats {
        MemoryPressureStats {
            pressure_events: self.stats.pressure_events.load(Ordering::Relaxed),
            callbacks_invoked: self.stats.callbacks_invoked.load(Ordering::Relaxed),
            emergency_releases: self.stats.emergency_releases.load(Ordering::Relaxed),
            total_freed: self.stats.total_freed.load(Ordering::Relaxed),
            oom_prevented: self.stats.oom_prevented.load(Ordering::Relaxed),
            current_level: self.get_pressure_level(),
            current_usage: self.get_usage(),
        }
    }

    /// Get recent pressure events
    pub fn get_recent_events(&self, count: usize) -> Vec<MemoryPressureEvent> {
        let events = self.events.read().unwrap();
        events.iter().rev().take(count).cloned().collect()
    }

    /// Enable/disable monitoring
    pub fn set_monitoring_enabled(&self, enabled: bool) {
        self.monitoring_enabled.store(enabled, Ordering::Relaxed);
    }
}

/// Memory usage snapshot
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    pub total_memory: u64,
    pub used_memory: u64,
    pub available_memory: u64,
    pub usage_ratio: f64,
    pub pressure_level: MemoryPressureLevel,
    pub emergency_mode: bool,
}

/// Memory pressure statistics
#[derive(Debug, Clone)]
pub struct MemoryPressureStats {
    pub pressure_events: u64,
    pub callbacks_invoked: u64,
    pub emergency_releases: u64,
    pub total_freed: u64,
    pub oom_prevented: u64,
    pub current_level: MemoryPressureLevel,
    pub current_usage: MemoryUsage,
}

// ============================================================================
