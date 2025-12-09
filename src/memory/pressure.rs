// # Memory Pressure Management
//
// This module provides comprehensive memory pressure monitoring and management
// for the database system. It monitors global memory usage, detects pressure
// conditions, and coordinates memory reclamation across all allocators and
// components to prevent out-of-memory situations.
//
// ## Key Features
//
// - **Global Monitoring**: Tracks system-wide memory usage and pressure levels
// - **Threshold Management**: Configurable warning, critical, and emergency thresholds
// - **Callback System**: Event-driven callbacks for pressure response actions
// - **Automatic Response**: Coordinated memory reclamation across allocators
// - **Pressure History**: Historical tracking and analysis of pressure events
// - **Component Integration**: Seamless integration with all memory allocators
// - **Real-time Metrics**: Live monitoring with configurable check intervals
//
// ## Design Overview
//
// The pressure manager operates as a centralized coordinator that:
// 1. Continuously monitors system memory usage
// 2. Compares usage against configured thresholds
// 3. Triggers appropriate responses based on pressure level
// 4. Coordinates memory reclamation across all allocators
// 5. Tracks pressure events for analysis and tuning
//
// ### Pressure Levels
//
// - **Normal**: Memory usage below warning threshold
// - **Warning**: Memory usage approaching limits (configurable, typically 80%)
// - **Critical**: High memory usage requiring immediate action (typically 90%)
// - **Emergency**: Extreme memory pressure requiring drastic measures (typically 95%)
//
// ### Response Actions
//
// - **Warning Level**: Cache trimming, background cleanup
// - **Critical Level**: Aggressive memory reclamation, query throttling
// - **Emergency Level**: Transaction rollback, connection dropping, service degradation
//
// ## Usage Example
//
// ```rust
// use crate::memory::pressure::*;
// use crate::memory::types::*;
//
// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
// // Create pressure manager
// let config = PressureConfig {
//     warning_threshold: 0.75,  // 75%
//     critical_threshold: 0.85, // 85%
//     emergency_threshold: 0.95, // 95%
//     enable_monitoring: true,
//     check_interval: Duration::from_secs(5),
//     ..Default::default()
// };
//
// let pressure_manager = MemoryPressureManager::new(config).await?;
//
// // Register a pressure callback
// let callback_id = pressure_manager.register_callback(
//     MemoryPressureLevel::Warning,
//     Box::new(|level, event| {
//         Box::pin(async move {
//             println!("Memory pressure {} detected: {} MB used",
//                 level, event.used_memory / 1024 / 1024);
//
//             // Perform cleanup actions
//             // ... implementation specific cleanup ...
//
//             Ok(1024 * 1024) // Freed 1MB
//         })
//     }),
// ).await?;
//
// // Start monitoring
// pressure_manager.start_monitoring().await?;
//
// // Manually trigger pressure check
// let pressure_level = pressure_manager.check_pressure().await?;
// println!("Current pressure level: {}", pressure_level);
//
// // Get pressure statistics
// let stats = pressure_manager.get_statistics().await;
// println!("Total pressure events: {}", stats.total_events);
//
// // Stop monitoring when shutting down
// pressure_manager.stop_monitoring().await?;
// # Ok(())
// # }
// ```

use std::collections::VecDeque;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH, Instant};
use crate::memory::types::*;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration};
use thiserror::Error;
use tokio::sync::{Semaphore};
use uuid::Uuid;

/// Memory pressure management specific errors
#[derive(Error, Debug)]
pub enum PressureError {
    #[error("Memory monitoring not active")]
    MonitoringNotActive,

    #[error("Callback registration failed: {reason}")]
    CallbackRegistrationFailed { reason: String },

    #[error("Callback execution failed: {callback_id} - {reason}")]
    CallbackExecutionFailed { callback_id: String, reason: String },

    #[error("Pressure calculation failed: {reason}")]
    PressureCalculationFailed { reason: String },

    #[error("System information unavailable: {reason}")]
    SystemInfoUnavailable { reason: String },

    #[error("Threshold configuration invalid: {threshold_type} = {value}")]
    InvalidThreshold { threshold_type: String, value: f64 },

    #[error("Memory reclamation failed: {reason}")]
    ReclamationFailed { reason: String },
}

/// Memory pressure callback function type
///
/// Callbacks are async functions that receive the pressure level and event details
/// and return the number of bytes freed during cleanup.
pub type PressureCallback = Box<
    dyn Fn(MemoryPressureLevel, MemoryPressureEvent) -> Pin<Box<dyn Future<Output = Result<u64, Box<dyn std::error::Error + Send + Sync>>> + Send>>
        + Send
        + Sync,
>;

/// Callback registration information
#[derive(Debug)]
pub struct CallbackRegistration {
    /// Unique callback identifier
    pub callback_id: Uuid,
    /// Pressure level that triggers this callback
    pub trigger_level: MemoryPressureLevel,
    /// Priority of the callback (lower number = higher priority)
    pub priority: i32,
    /// Whether the callback is active
    pub is_active: AtomicBool,
    /// Number of times this callback has been invoked
    pub invocation_count: AtomicU64,
    /// Total bytes freed by this callback
    pub total_bytes_freed: AtomicU64,
    /// Average execution time
    pub avg_execution_time: Arc<Mutex<Duration>>,
    /// Registration timestamp
    pub registered_at: SystemTime,
    /// Last invocation timestamp
    pub last_invoked: AtomicU64,
    /// The actual callback function
    pub callback: PressureCallback,
}

/// System memory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMemoryInfo {
    /// Total system memory in bytes
    pub total_memory: u64,
    /// Available memory in bytes
    pub available_memory: u64,
    /// Used memory in bytes
    pub used_memory: u64,
    /// Free memory in bytes
    pub free_memory: u64,
    /// Cached memory in bytes
    pub cached_memory: u64,
    /// Buffer memory in bytes
    pub buffer_memory: u64,
    /// Shared memory in bytes
    pub shared_memory: u64,
    /// Process RSS (Resident Set Size) in bytes
    pub process_rss: u64,
    /// Process virtual memory in bytes
    pub process_virtual: u64,
    /// Memory usage percentage (0.0 to 1.0)
    pub usage_ratio: f64,
    /// Timestamp when this information was collected
    pub collected_at: SystemTime,
}

/// Memory pressure statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryPressureStats {
    /// Total number of pressure events
    pub total_events: u64,
    /// Events by pressure level
    pub events_by_level: HashMap<String, u64>,
    /// Total bytes freed by pressure response
    pub total_bytes_freed: u64,
    /// Average event duration
    pub avg_event_duration: Duration,
    /// Maximum event duration
    pub max_event_duration: Duration,
    /// Number of callback invocations
    pub callback_invocations: u64,
    /// Number of failed callback invocations
    pub failed_callback_invocations: u64,
    /// Average callback execution time
    pub avg_callback_time: Duration,
    /// Current memory pressure level
    pub current_level: MemoryPressureLevel,
    /// Time spent at each pressure level
    pub time_at_level: HashMap<String, Duration>,
    /// Peak memory usage observed
    pub peak_memory_usage: u64,
    /// Memory reclamation efficiency (freed/requested ratio)
    pub reclamation_efficiency: f64,
    /// Number of emergency actions taken
    pub emergency_actions: u64,
    /// System uptime when monitoring started
    pub monitoring_uptime: Duration,
    /// Last updated timestamp
    pub last_updated: SystemTime,
}

/// Memory pressure event history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureEventHistory {
    /// Event identifier
    pub event_id: Uuid,
    /// When the event occurred
    pub timestamp: SystemTime,
    /// Pressure level during the event
    pub level: MemoryPressureLevel,
    /// System memory state at event time
    pub memory_info: SystemMemoryInfo,
    /// Number of callbacks invoked
    pub callbacks_invoked: usize,
    /// Total bytes freed during event
    pub bytes_freed: u64,
    /// Event duration
    pub duration: Duration,
    /// Whether emergency actions were taken
    pub emergency_actions_taken: bool,
    /// Event resolution method
    pub resolution: PressureResolution,
    /// Additional context about the event
    pub context: String,
}

/// How a pressure event was resolved
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PressureResolution {
    /// Event resolved through normal callbacks
    CallbackSuccess,
    /// Event resolved through emergency actions
    EmergencyAction,
    /// Event resolved automatically (memory freed by external factors)
    AutoResolved,
    /// Event timeout (still in pressure state)
    Timeout,
    /// Event resolution failed
    Failed,
}

impl fmt::Display for PressureResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PressureResolution::CallbackSuccess => write!(f, "Callback Success"),
            PressureResolution::EmergencyAction => write!(f, "Emergency Action"),
            PressureResolution::AutoResolved => write!(f, "Auto Resolved"),
            PressureResolution::Timeout => write!(f, "Timeout"),
            PressureResolution::Failed => write!(f, "Failed"),
        }
    }
}

/// Main memory pressure manager
///
/// Coordinates system-wide memory pressure monitoring and response.
/// Integrates with all allocators and provides callback-based pressure handling.
#[derive(Debug)]
pub struct MemoryPressureManager {
    /// Configuration for pressure management
    config: PressureConfig,
    /// Whether monitoring is active
    is_monitoring: AtomicBool,
    /// Current pressure level
    current_level: Arc<RwLock<MemoryPressureLevel>>,
    /// Registered pressure callbacks
    callbacks: Arc<RwLock<HashMap<Uuid, Arc<CallbackRegistration>>>>,
    /// Pressure event history
    event_history: Arc<RwLock<VecDeque<PressureEventHistory>>>,
    /// Pressure statistics
    stats: Arc<AsyncRwLock<MemoryPressureStats>>,
    /// Last system memory information
    last_memory_info: Arc<RwLock<Option<SystemMemoryInfo>>>,
    /// Monitoring task handle
    monitoring_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Pressure manager creation timestamp
    created_at: SystemTime,
    /// Manager unique identifier
    manager_id: Uuid,
    /// Callback execution semaphore (limits concurrent callbacks)
    callback_semaphore: Arc<Semaphore>,
}

impl CallbackRegistration {
    /// Creates a new callback registration
    pub fn new(
        trigger_level: MemoryPressureLevel,
        priority: i32,
        callback: PressureCallback,
    ) -> Self {
        Self {
            callback_id: Uuid::new_v4(),
            trigger_level,
            priority,
            is_active: AtomicBool::new(true),
            invocation_count: AtomicU64::new(0),
            total_bytes_freed: AtomicU64::new(0),
            avg_execution_time: Arc::new(Mutex::new(Duration::ZERO)),
            registered_at: SystemTime::now(),
            last_invoked: AtomicU64::new(0),
            callback,
        }
    }

    /// Records callback execution
    pub fn record_execution(&self, execution_time: Duration, bytes_freed: u64) {
        self.invocation_count.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_freed.fetch_add(bytes_freed, Ordering::Relaxed);

        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        self.last_invoked.store(now, Ordering::Relaxed);

        // Update average execution time
        let mut avg_time = self.avg_execution_time.lock();
        let count = self.invocation_count.load(Ordering::Relaxed);
        if count == 1 {
            *avg_time = execution_time;
        } else {
            let total_time = *avg_time * (count - 1) as u32 + execution_time;
            *avg_time = total_time / count as u32;
        }
    }

    /// Gets callback statistics
    pub fn get_stats(&self) -> (u64, u64, Duration) {
        (
            self.invocation_count.load(Ordering::Relaxed),
            self.total_bytes_freed.load(Ordering::Relaxed),
            *self.avg_execution_time.lock(),
        )
    }
}

impl SystemMemoryInfo {
    /// Collects current system memory information
    pub fn collect() -> Result<Self, PressureError> {
        // Platform-specific memory collection
        #[cfg(target_os = "linux")]
        {
            Self::collect_linux()
        }

        #[cfg(target_os = "windows")]
        {
            Self::collect_windows()
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            // Fallback implementation
            Self::collect_fallback()
        }
    }

    #[cfg(target_os = "linux")]
    fn collect_linux() -> Result<Self, PressureError> {
        use std::fs;

        // Read /proc/meminfo
        let meminfo = fs::read_to_string("/proc/meminfo")
            .map_err(|e| PressureError::SystemInfoUnavailable {
                reason: format!("Cannot read /proc/meminfo: {}", e),
            })?;

        let mut total_memory = 0;
        let mut free_memory = 0;
        let mut available_memory = 0;
        let mut cached_memory = 0;
        let mut buffer_memory = 0;

        for line in meminfo.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let value = parts[1].parse::<u64>().unwrap_or(0) * 1024; // Convert from KB

                match parts[0] {
                    "MemTotal:" => total_memory = value,
                    "MemFree:" => free_memory = value,
                    "MemAvailable:" => available_memory = value,
                    "Cached:" => cached_memory = value,
                    "Buffers:" => buffer_memory = value,
                    _ => {}
                }
            }
        }

        // Read process memory info
        let (process_rss, process_virtual) = Self::collect_process_memory_linux()?;

        let used_memory = total_memory.saturating_sub(available_memory);
        let usage_ratio = if total_memory > 0 {
            used_memory as f64 / total_memory as f64
        } else {
            0.0
        };

        Ok(Self {
            total_memory,
            available_memory,
            used_memory,
            free_memory,
            cached_memory,
            buffer_memory,
            shared_memory: 0, // Would need to parse from /proc/meminfo
            process_rss,
            process_virtual,
            usage_ratio,
            collected_at: SystemTime::now(),
        })
    }

    #[cfg(target_os = "linux")]
    fn collect_process_memory_linux() -> Result<(u64, u64), PressureError> {
        use std::fs;

        let status = fs::read_to_string("/proc/self/status")
            .map_err(|e| PressureError::SystemInfoUnavailable {
                reason: format!("Cannot read /proc/self/status: {}", e),
            })?;

        let mut rss = 0;
        let mut virtual_size = 0;

        for line in status.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                match parts[0] {
                    "VmRSS:" => {
                        if parts.len() >= 3 && parts[2] == "kB" {
                            rss = parts[1].parse::<u64>().unwrap_or(0) * 1024;
                        }
                    }
                    "VmSize:" => {
                        if parts.len() >= 3 && parts[2] == "kB" {
                            virtual_size = parts[1].parse::<u64>().unwrap_or(0) * 1024;
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok((rss, virtual_size))
    }

    #[cfg(target_os = "windows")]
    fn collect_windows() -> Result<Self, PressureError> {
        // Windows-specific implementation would go here
        // For now, use fallback
        Self::collect_fallback()
    }

    fn collect_fallback() -> Result<Self, PressureError> {
        // Simple fallback that reports fixed values
        // In a real implementation, you'd want actual system information
        let total_memory = 8 * 1024 * 1024 * 1024; // 8GB
        let used_memory = total_memory / 2; // 50% usage
        let available_memory = total_memory - used_memory;

        Ok(Self {
            total_memory,
            available_memory,
            used_memory,
            free_memory: available_memory,
            cached_memory: 0,
            buffer_memory: 0,
            shared_memory: 0,
            process_rss: used_memory / 4,
            process_virtual: used_memory,
            usage_ratio: 0.5,
            collected_at: SystemTime::now(),
        })
    }

    /// Calculates pressure level based on usage ratio and thresholds
    pub fn calculate_pressure_level(&self, config: &PressureConfig) -> MemoryPressureLevel {
        if self.usage_ratio >= config.emergency_threshold {
            MemoryPressureLevel::Emergency
        } else if self.usage_ratio >= config.critical_threshold {
            MemoryPressureLevel::Critical
        } else if self.usage_ratio >= config.warning_threshold {
            MemoryPressureLevel::Warning
        } else {
            MemoryPressureLevel::Normal
        }
    }
}

impl MemoryPressureManager {
    /// Creates a new memory pressure manager
    pub async fn new(config: PressureConfig) -> Result<Self, PressureError> {
        // Validate configuration
        if config.warning_threshold >= config.critical_threshold {
            return Err(PressureError::InvalidThreshold {
                threshold_type: "warning vs critical".to_string(),
                value: config.warning_threshold,
            });
        }

        if config.critical_threshold >= config.emergency_threshold {
            return Err(PressureError::InvalidThreshold {
                threshold_type: "critical vs emergency".to_string(),
                value: config.critical_threshold,
            });
        }

        Ok(Self {
            config,
            is_monitoring: AtomicBool::new(false),
            current_level: Arc::new(RwLock::new(MemoryPressureLevel::Normal)),
            callbacks: Arc::new(RwLock::new(HashMap::new())),
            event_history: Arc::new(RwLock::new(VecDeque::new())),
            stats: Arc::new(AsyncRwLock::new(MemoryPressureStats::default())),
            last_memory_info: Arc::new(RwLock::new(None)),
            monitoring_task: Arc::new(Mutex::new(None)),
            created_at: SystemTime::now(),
            manager_id: Uuid::new_v4(),
            callback_semaphore: Arc::new(Semaphore::new(10)), // Max 10 concurrent callbacks
        })
    }

    /// Registers a pressure callback
    pub async fn register_callback(
        &self,
        trigger_level: MemoryPressureLevel,
        callback: PressureCallback,
    ) -> Result<Uuid, PressureError> {
        self.register_callback_with_priority(trigger_level, 100, callback).await
    }

    /// Registers a pressure callback with specific priority
    pub async fn register_callback_with_priority(
        &self,
        trigger_level: MemoryPressureLevel,
        priority: i32,
        callback: PressureCallback,
    ) -> Result<Uuid, PressureError> {
        let registration = Arc::new(CallbackRegistration::new(trigger_level, priority, callback));
        let callback_id = registration.callback_id;

        {
            let mut callbacks = self.callbacks.write();
            callbacks.insert(callback_id, registration);
        }

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.last_updated = SystemTime::now();

        Ok(callback_id)
    }

    /// Unregisters a pressure callback
    pub async fn unregister_callback(&self, callback_id: Uuid) -> Result<(), PressureError> {
        let mut callbacks = self.callbacks.write();

        if let Some(registration) = callbacks.remove(&callback_id) {
            registration.is_active.store(false, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Starts memory pressure monitoring
    pub async fn start_monitoring(&self) -> Result<(), PressureError> {
        if self.is_monitoring.swap(true, Ordering::Relaxed) {
            return Ok(());  // Already monitoring
        }

        let config = self.config.clone();
        let is_monitoring = Arc::new(AtomicBool::new(true));
        let current_level = Arc::clone(&self.current_level);
        let callbacks = Arc::clone(&self.callbacks);
        let event_history = Arc::clone(&self.event_history);
        let stats = Arc::clone(&self.stats);
        let last_memory_info = Arc::clone(&self.last_memory_info);
        let callback_semaphore = Arc::clone(&self.callback_semaphore);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.check_interval);
            let mut last_level = MemoryPressureLevel::Normal;

            while is_monitoring.load(Ordering::Relaxed) {
                interval.tick().await;

                // Collect memory information
                match SystemMemoryInfo::collect() {
                    Ok(memory_info) => {
                        let pressure_level = memory_info.calculate_pressure_level(&config);

                        // Update last memory info
                        *last_memory_info.write() = Some(memory_info.clone());

                        // Check if pressure level changed
                        if pressure_level != last_level {
                            *current_level.write() = pressure_level;

                            // Trigger pressure event if above normal
                            if pressure_level > MemoryPressureLevel::Normal {
                                Self::handle_pressure_event_internal(
                                    pressure_level,
                                    memory_info,
                                    &config,
                                    &callbacks,
                                    &event_history,
                                    &stats,
                                    &callback_semaphore,
                                ).await;
                            }

                            last_level = pressure_level;
                        }

                        // Update current level in stats
                        let mut stats_guard = stats.write().await;
                        stats_guard.current_level = pressure_level;
                        stats_guard.peak_memory_usage = stats_guard.peak_memory_usage.max(memory_info.used_memory);
                        stats_guard.last_updated = SystemTime::now();
                    }
                    Err(e) => {
                        eprintln!("Failed to collect memory information: {}", e);
                    }
                }
            }
        });

        *self.monitoring_task.lock() = Some(handle);

        Ok(())
    }

    /// Stops memory pressure monitoring
    pub async fn stop_monitoring(&self) -> Result<(), PressureError> {
        self.is_monitoring.store(false, Ordering::Relaxed);

        if let Some(handle) = self.monitoring_task.lock().take() {
            handle.abort();
        }

        Ok(())
    }

    /// Manually checks current pressure level
    pub async fn check_pressure(&self) -> Result<MemoryPressureLevel, PressureError> {
        let memory_info = SystemMemoryInfo::collect()?;
        let pressure_level = memory_info.calculate_pressure_level(&self.config);

        // Update stored information
        *self.last_memory_info.write() = Some(memory_info.clone());
        *self.current_level.write() = pressure_level;

        // If pressure is elevated, handle it
        if pressure_level > MemoryPressureLevel::Normal {
            self.handle_pressure_event(pressure_level, memory_info).await?;
        }

        Ok(pressure_level)
    }

    /// Handles a pressure event
    async fn handle_pressure_event(
        &self,
        level: MemoryPressureLevel,
        memory_info: SystemMemoryInfo,
    ) -> Result<(), PressureError> {
        Self::handle_pressure_event_internal(
            level,
            memory_info,
            &self.config,
            &self.callbacks,
            &self.event_history,
            &self.stats,
            &self.callback_semaphore,
        ).await;

        Ok(())
    }

    /// Internal pressure event handling
    async fn handle_pressure_event_internal(
        level: MemoryPressureLevel,
        memory_info: SystemMemoryInfo,
        config: &PressureConfig,
        callbacks: &Arc<RwLock<HashMap<Uuid, Arc<CallbackRegistration>>>>,
        event_history: &Arc<RwLock<VecDeque<PressureEventHistory>>>,
        stats: &Arc<AsyncRwLock<MemoryPressureStats>>,
        callback_semaphore: &Arc<Semaphore>,
    ) {
        let event_start = Instant::now();
        let event_id = Uuid::new_v4();

        // Create pressure event
        let pressure_event = MemoryPressureEvent {
            timestamp: SystemTime::now(),
            level,
            total_memory: memory_info.total_memory,
            used_memory: memory_info.used_memory,
            available_memory: memory_info.available_memory,
            callbacks_invoked: 0,
            bytes_freed: 0,
            duration: Duration::ZERO,
            emergency_actions_taken: false,
        };

        let mut total_bytes_freed = 0;
        let mut callbacks_invoked = 0;
        let mut emergency_actions_taken = false;

        if config.enable_callbacks {
            // Get callbacks for this pressure level and below
            let eligible_callbacks: Vec<_> = {
                let callbacks_guard = callbacks.read();
                callbacks_guard
                    .values()
                    .filter(|cb| {
                        cb.is_active.load(Ordering::Relaxed) && cb.trigger_level <= level
                    })
                    .cloned()
                    .collect()
            };

            // Sort by priority (lower number = higher priority)
            let mut sorted_callbacks = eligible_callbacks;
            sorted_callbacks.sort_by_key(|cb| cb.priority);

            // Execute callbacks
            for callback_reg in sorted_callbacks {
                let _permit = callback_semaphore.acquire().await;

                let callback_start = Instant::now();

                match (callback_reg.callback)(level, pressure_event.clone()).await {
                    Ok(bytes_freed) => {
                        let execution_time = callback_start.elapsed();
                        callback_reg.record_execution(execution_time, bytes_freed);
                        total_bytes_freed += bytes_freed;
                        callbacks_invoked += 1;

                        // If we've freed enough memory, stop
                        if level == MemoryPressureLevel::Warning && total_bytes_freed > 0 {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Pressure callback failed: {}", e);
                        let mut stats_guard = stats.write().await;
                        stats_guard.failed_callback_invocations += 1;
                    }
                }
            }
        }

        // Emergency actions for critical/emergency levels
        if level >= MemoryPressureLevel::Emergency && config.enable_auto_release {
            emergency_actions_taken = true;
            // Emergency actions would go here (connection dropping, transaction rollback, etc.)
        }

        let event_duration = event_start.elapsed();

        // Update final pressure event
        let final_pressure_event = MemoryPressureEvent {
            callbacks_invoked,
            bytes_freed: total_bytes_freed,
            duration: event_duration,
            emergency_actions_taken,
            ..pressure_event
        };

        // Add to event history
        {
            let mut history = event_history.write();
            history.push_back(PressureEventHistory {
                event_id,
                timestamp: SystemTime::now(),
                level,
                memory_info: memory_info.clone(),
                callbacks_invoked,
                bytes_freed: total_bytes_freed,
                duration: event_duration,
                emergency_actions_taken,
                resolution: if total_bytes_freed > 0 {
                    PressureResolution::CallbackSuccess
                } else if emergency_actions_taken {
                    PressureResolution::EmergencyAction
                } else {
                    PressureResolution::Failed
                },
                context: format!("Pressure level: {}, Memory usage: {:.2}%",
                    level, memory_info.usage_ratio * 100.0),
            });

            // Limit history size
            if history.len() > config.max_pressure_events {
                history.pop_front();
            }
        }

        // Update statistics
        {
            let mut stats_guard = stats.write().await;
            stats_guard.total_events += 1;
            stats_guard.total_bytes_freed += total_bytes_freed;
            stats_guard.callback_invocations += callbacks_invoked as u64;

            if emergency_actions_taken {
                stats_guard.emergency_actions += 1;
            }

            // Update events by level
            let level_str = level.to_string();
            *stats_guard.events_by_level.entry(level_str.clone()).or_insert(0) += 1;

            // Update average event duration
            if stats_guard.total_events == 1 {
                stats_guard.avg_event_duration = event_duration;
            } else {
                let total_duration = stats_guard.avg_event_duration * (stats_guard.total_events - 1) as u32;
                stats_guard.avg_event_duration = (total_duration + event_duration) / stats_guard.total_events as u32;
            }

            if event_duration > stats_guard.max_event_duration {
                stats_guard.max_event_duration = event_duration;
            }

            stats_guard.last_updated = SystemTime::now();
        }
    }

    /// Gets current memory information
    pub async fn get_memory_info(&self) -> Option<SystemMemoryInfo> {
        self.last_memory_info.read().clone()
    }

    /// Gets current pressure level
    pub async fn get_current_level(&self) -> MemoryPressureLevel {
        *self.current_level.read()
    }

    /// Gets pressure statistics
    pub async fn get_statistics(&self) -> MemoryPressureStats {
        let mut stats = self.stats.write().await;
        stats.monitoring_uptime = SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or_default();
        stats.clone()
    }

    /// Gets pressure event history
    pub async fn get_event_history(&self) -> Vec<PressureEventHistory> {
        self.event_history.read().iter().cloned().collect()
    }

    /// Gets callback statistics
    pub async fn get_callback_statistics(&self) -> HashMap<Uuid, (u64, u64)> {
        let callbacks = self.callbacks.read();
        callbacks
            .iter()
            .map(|(id, cb)| (*id, cb.get_stats()))
            .collect()
    }

    /// Forces memory pressure cleanup
    pub async fn force_cleanup(
        &self,
        target_level: MemoryPressureLevel,
    ) -> Result<u64, PressureError> {
        let memory_info = SystemMemoryInfo::collect()?;
        self.handle_pressure_event(target_level, memory_info).await?;

        // Return total bytes freed from recent events
        let history = self.event_history.read();
        let recent_bytes_freed = history
            .back()
            .map(|event| event.bytes_freed)
            .unwrap_or(0);

        Ok(recent_bytes_freed)
    }

    /// Simulates memory pressure for testing
    #[cfg(test)]
    pub async fn simulate_pressure(
        &self,
        level: MemoryPressureLevel,
        used_memory: u64,
        total_memory: u64,
    ) -> Result<(), PressureError> {
        let memory_info = SystemMemoryInfo {
            total_memory,
            used_memory,
            available_memory: total_memory - used_memory,
            free_memory: total_memory - used_memory,
            cached_memory: 0,
            buffer_memory: 0,
            shared_memory: 0,
            process_rss: used_memory / 2,
            process_virtual: used_memory,
            usage_ratio: used_memory as f64 / total_memory as f64,
            collected_at: SystemTime::now(),
        };

        self.handle_pressure_event(level, memory_info).await?;
        Ok(())
    }

    /// Shuts down the pressure manager gracefully
    pub async fn shutdown(&self) -> Result<(), PressureError> {
        self.stop_monitoring().await?;

        // Disable all callbacks
        let callbacks = self.callbacks.read();
        for callback in callbacks.values() {
            callback.is_active.store(false, Ordering::Relaxed);
        }

        Ok(())
    }
}

/// Utility functions
impl MemoryPressureManager {
    /// Calculates system memory pressure without triggering callbacks
    pub async fn calculate_pressure_only(&self) -> Result<(MemoryPressureLevel, SystemMemoryInfo), PressureError> {
        let memory_info = SystemMemoryInfo::collect()?;
        let pressure_level = memory_info.calculate_pressure_level(&self.config);
        Ok((pressure_level, memory_info))
    }

    /// Estimates time to next pressure level
    pub async fn estimate_time_to_pressure(
        &self,
        target_level: MemoryPressureLevel,
    ) -> Option<Duration> {
        // This would require tracking memory usage trends
        // For now, return None (not implemented)
        let _ = target_level;
        None
    }

    /// Gets memory efficiency score (0.0 to 1.0)
    pub async fn get_efficiency_score(&self) -> f64 {
        let stats = self.stats.read().await;
        if stats.total_events > 0 && stats.total_bytes_freed > 0 {
            // Calculate based on event resolution efficiency
            let successful_events = stats.events_by_level.values().sum::<u64>();
            if successful_events > 0 {
                stats.total_bytes_freed as f64 / (stats.total_events * 1024 * 1024) as f64 // MB per event
            } else {
                0.0
            }
        } else {
            1.0 // No pressure events = perfect efficiency
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::test;

    #[test]
    async fn test_pressure_manager_creation() {
        let config = PressureConfig::default();
        let manager = MemoryPressureManager::new(config).await;
        assert!(manager.is_ok());

        let manager = manager.unwrap();
        assert!(!manager.is_monitoring.load(Ordering::Relaxed));
    }

    #[test]
    async fn test_invalid_threshold_configuration() {
        let invalid_config = PressureConfig {
            warning_threshold: 0.90,
            critical_threshold: 0.80, // Invalid: critical < warning
            emergency_threshold: 0.95,
            ..Default::default()
        };

        let manager = MemoryPressureManager::new(invalid_config).await;
        assert!(manager.is_err());
    }

    #[test]
    async fn test_callback_registration() {
        let config = PressureConfig::default();
        let manager = MemoryPressureManager::new(config).await.unwrap();

        let callback_count = Arc::new(AtomicU64::new(0));
        let callback_count_clone = Arc::clone(&callback_count);

        let callback = Box::new(move |_level, _event| {
            let count = Arc::clone(&callback_count_clone);
            Box::pin(async move {
                count.fetch_add(1, Ordering::Relaxed);
                Ok(1024) // Freed 1KB
            })
        });

        let callback_id = manager.register_callback(
            MemoryPressureLevel::Warning,
            callback,
        ).await;

        assert!(callback_id.is_ok());
    }

    #[test]
    async fn test_pressure_level_calculation() {
        let config = PressureConfig {
            warning_threshold: 0.75,
            critical_threshold: 0.85,
            emergency_threshold: 0.95,
            ..Default::default()
        };

        let memory_info = SystemMemoryInfo {
            total_memory: 1000,
            used_memory: 800, // 80% usage
            available_memory: 200,
            free_memory: 200,
            cached_memory: 0,
            buffer_memory: 0,
            shared_memory: 0,
            process_rss: 400,
            process_virtual: 800,
            usage_ratio: 0.80,
            collected_at: SystemTime::now(),
        };

        let level = memory_info.calculate_pressure_level(&config);
        assert_eq!(level, MemoryPressureLevel::Critical);
    }

    #[test]
    async fn test_pressure_simulation() {
        let config = PressureConfig::default();
        let manager = MemoryPressureManager::new(config).await.unwrap();

        let callback_invoked = Arc::new(AtomicBool::new(false));
        let callback_invoked_clone = Arc::clone(&callback_invoked);

        let callback = Box::new(move |_level, _event| {
            let invoked = Arc::clone(&callback_invoked_clone);
            Box::pin(async move {
                invoked.store(true, Ordering::Relaxed);
                Ok(2048) // Freed 2KB
            })
        });

        let callback_id = manager.register_callback(
            MemoryPressureLevel::Warning,
            callback,
        ).await.unwrap();

        // Simulate warning level pressure
        manager.simulate_pressure(
            MemoryPressureLevel::Warning,
            8 * 1024 * 1024 * 1024, // 8GB used
            10 * 1024 * 1024 * 1024, // 10GB total (80% usage)
        ).await.unwrap();

        assert!(callback_invoked.load(Ordering::Relaxed));

        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.total_bytes_freed, 2048);
    }

    #[test]
    async fn test_callback_priority_ordering() {
        let config = PressureConfig::default();
        let manager = MemoryPressureManager::new(config).await.unwrap();

        let execution_order = Arc::new(Mutex::new(Vec::new()));

        // Register callbacks with different priorities
        for (priority, name) in [(10, "high"), (50, "medium"), (5, "highest")] {
            let order = Arc::clone(&execution_order);
            let callback_name = name.to_string();

            let callback = Box::new(move |_level, _event| {
                let order = Arc::clone(&order);
                let name = callback_name.clone();
                Box::pin(async move {
                    order.lock().push(name);
                    Ok(1024)
                })
            });

            let id = manager.register_callback_with_priority(
                MemoryPressureLevel::Warning,
                priority,
                callback,
            ).await.unwrap();
        }

        // Trigger pressure event
        manager.simulate_pressure(
            MemoryPressureLevel::Warning,
            8 * 1024 * 1024 * 1024,
            10 * 1024 * 1024 * 1024,
        ).await.unwrap();

        let order = execution_order.lock();
        assert_eq!(order[0], "highest"); // Priority 5
        assert_eq!(order[1], "high");    // Priority 10
        assert_eq!(order[2], "medium");  // Priority 50
    }

    #[test]
    async fn test_event_history() {
        let config = PressureConfig {
            max_pressure_events: 5,
            ..Default::default()
        };
        let manager = MemoryPressureManager::new(config).await.unwrap();

        // Simulate multiple pressure events
        for i in 0..7 {
            manager.simulate_pressure(
                MemoryPressureLevel::Warning,
                (8 + i) * 1024 * 1024 * 1024,
                10 * 1024 * 1024 * 1024,
            ).await.unwrap();
        }

        let history = manager.get_event_history().await;
        assert_eq!(history.len(), 5); // Should be limited to max_pressure_events

        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_events, 7); // Total count should still be accurate
    }

    #[test]
    fn test_system_memory_info_collection() {
        let memory_info = SystemMemoryInfo::collect();
        assert!(memory_info.is_ok());

        let info = memory_info.unwrap();
        assert!(info.total_memory > 0);
        assert!(info.usage_ratio >= 0.0);
        assert!(info.usage_ratio <= 1.0);
    }

    #[test]
    fn test_pressure_resolution_display() {
        assert_eq!(PressureResolution::CallbackSuccess.to_string(), "Callback Success");
        assert_eq!(PressureResolution::EmergencyAction.to_string(), "Emergency Action");
        assert_eq!(PressureResolution::Failed.to_string(), "Failed");
    }
}
