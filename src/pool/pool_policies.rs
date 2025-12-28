// # Connection Pool Policies Engine
//
// This module provides a comprehensive policy engine for managing connection pool behavior:
// - Min/max pool size enforcement policies
// - Connection timeout policies (acquire, idle, lifetime)
// - Idle connection management strategies
// - Overflow handling strategies (queue, reject, create, adaptive)
// - Policy-based auto-tuning
//
// ## Policy Types
//
// - **Size Policies**: Control pool growth and shrinkage
// - **Timeout Policies**: Manage connection timeouts at various stages
// - **Idle Policies**: Handle idle connections efficiently
// - **Overflow Policies**: Determine behavior when pool is exhausted
// - **Maintenance Policies**: Background cleanup and optimization
//
// ## Auto-Tuning
//
// The policy engine can automatically adjust pool parameters based on:
// - Load patterns (time of day, day of week)
// - Performance metrics (latency, throughput)
// - Resource availability (memory, CPU)
// - Error rates and patterns

use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;

// ============================================================================
// Pool Size Policy
// ============================================================================

/// Policy for controlling pool size
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizePolicy {
    /// Minimum number of connections to maintain
    pub min_connections: usize,
    /// Maximum number of connections allowed
    pub max_connections: usize,
    /// Initial number of connections to create
    pub initial_connections: usize,
    /// Growth rate when pool is under pressure (connections per second)
    pub growth_rate: f64,
    /// Shrink rate when pool has excess capacity (connections per minute)
    pub shrink_rate: f64,
    /// Time window to wait before shrinking
    pub shrink_delay: Duration,
    /// Enable dynamic adjustment based on load
    pub auto_scale: bool,
}

impl Default for SizePolicy {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 100,
            initial_connections: 10,
            growth_rate: 5.0,
            shrink_rate: 1.0,
            shrink_delay: Duration::from_secs(300), // 5 minutes
            auto_scale: true,
        }
    }
}

impl SizePolicy {
    pub fn validate(&self) -> Result<()> {
        if self.min_connections > self.max_connections {
            return Err(DbError::InvalidInput(
                "min_connections cannot exceed max_connections".to_string(),
            ));
        }
        if self.initial_connections < self.min_connections {
            return Err(DbError::InvalidInput(
                "initial_connections cannot be less than min_connections".to_string(),
            ));
        }
        if self.initial_connections > self.max_connections {
            return Err(DbError::InvalidInput(
                "initial_connections cannot exceed max_connections".to_string(),
            ));
        }
        Ok(())
    }

    /// Calculate how many connections to add based on current demand
    pub fn calculate_growth(&self, current: usize, waiting: usize) -> usize {
        if !self.auto_scale || current >= self.max_connections {
            return 0;
        }

        let available_capacity = self.max_connections - current;
        let desired_growth = (waiting as f64 * 1.2).ceil() as usize;

        desired_growth.min(available_capacity)
    }

    /// Calculate how many connections to remove based on idle capacity
    pub fn calculate_shrink(&self, current: usize, idle: usize, idle_duration: Duration) -> usize {
        if !self.auto_scale || current <= self.min_connections || idle_duration < self.shrink_delay {
            return 0;
        }

        let excess_capacity = current - self.min_connections;
        let idle_to_remove = (idle as f64 * 0.5).ceil() as usize;

        idle_to_remove.min(excess_capacity)
    }
}

// ============================================================================
// Timeout Policy
// ============================================================================

/// Policy for managing various connection timeouts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutPolicy {
    /// Maximum time to wait when acquiring a connection
    pub acquire_timeout: Duration,
    /// Maximum time a connection can be idle before being closed
    pub idle_timeout: Duration,
    /// Maximum lifetime of a connection before forced recycling
    pub lifetime_timeout: Duration,
    /// Maximum time for connection validation
    pub validation_timeout: Duration,
    /// Maximum time for connection creation
    pub creation_timeout: Duration,
    /// Enable adaptive timeout adjustment based on performance
    pub adaptive: bool,
}

impl Default for TimeoutPolicy {
    fn default() -> Self {
        Self {
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600), // 10 minutes
            lifetime_timeout: Duration::from_secs(3600), // 1 hour
            validation_timeout: Duration::from_secs(5),
            creation_timeout: Duration::from_secs(10),
            adaptive: false,
        }
    }
}

impl TimeoutPolicy {
    /// Adjust acquire timeout based on current performance
    pub fn adjust_acquire_timeout(&self, avg_wait_time: Duration) -> Duration {
        if !self.adaptive {
            return self.acquire_timeout;
        }

        // If average wait time is high, increase timeout
        let timeout_millis = if avg_wait_time > self.acquire_timeout / 2 {
            (avg_wait_time.as_millis() as f64 * 1.5) as u64
        } else {
            self.acquire_timeout.as_millis() as u64
        }
        .max(1000) // At least 1 second
        .min(120000); // At most 2 minutes

        Duration::from_millis(timeout_millis)
    }
}

// ============================================================================
// Idle Connection Policy
// ============================================================================

/// Strategy for managing idle connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdleStrategy {
    /// Keep all idle connections up to max
    KeepAll,
    /// Close idle connections after timeout
    CloseAfterTimeout,
    /// Periodically validate idle connections
    PeriodicValidation,
    /// Use idle connections for background tasks
    BackgroundTasks,
}

impl fmt::Display for IdleStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdleStrategy::KeepAll => write!(f, "KeepAll"),
            IdleStrategy::CloseAfterTimeout => write!(f, "CloseAfterTimeout"),
            IdleStrategy::PeriodicValidation => write!(f, "PeriodicValidation"),
            IdleStrategy::BackgroundTasks => write!(f, "BackgroundTasks"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlePolicy {
    /// Strategy for handling idle connections
    pub strategy: IdleStrategy,
    /// Minimum number of idle connections to maintain
    pub min_idle: usize,
    /// Maximum number of idle connections to keep
    pub max_idle: usize,
    /// Validation interval for idle connections
    pub validation_interval: Duration,
}

impl Default for IdlePolicy {
    fn default() -> Self {
        Self {
            strategy: IdleStrategy::CloseAfterTimeout,
            min_idle: 2,
            max_idle: 10,
            validation_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

// ============================================================================
// Overflow Handling Policy
// ============================================================================

/// Strategy for handling pool exhaustion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverflowStrategy {
    /// Queue requests and wait
    Queue,
    /// Reject requests immediately
    Reject,
    /// Create temporary connections beyond max
    CreateTemporary,
    /// Adaptive: switch strategies based on conditions
    Adaptive,
}

impl fmt::Display for OverflowStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OverflowStrategy::Queue => write!(f, "Queue"),
            OverflowStrategy::Reject => write!(f, "Reject"),
            OverflowStrategy::CreateTemporary => write!(f, "CreateTemporary"),
            OverflowStrategy::Adaptive => write!(f, "Adaptive"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverflowPolicy {
    /// Strategy for handling overflow
    pub strategy: OverflowStrategy,
    /// Maximum number of queued requests
    pub max_queue_size: usize,
    /// Maximum number of temporary connections
    pub max_temporary: usize,
    /// Threshold for switching strategies (in adaptive mode)
    pub adaptive_threshold: f64,
}

impl Default for OverflowPolicy {
    fn default() -> Self {
        Self {
            strategy: OverflowStrategy::Queue,
            max_queue_size: 1000,
            max_temporary: 10,
            adaptive_threshold: 0.8, // 80% utilization
        }
    }
}

impl OverflowPolicy {
    /// Determine the appropriate strategy based on current conditions
    pub fn resolve_strategy(&self, utilization: f64, queue_length: usize) -> OverflowStrategy {
        if self.strategy != OverflowStrategy::Adaptive {
            return self.strategy;
        }

        // Adaptive logic
        if utilization > self.adaptive_threshold && queue_length > self.max_queue_size / 2 {
            OverflowStrategy::CreateTemporary
        } else if queue_length >= self.max_queue_size {
            OverflowStrategy::Reject
        } else {
            OverflowStrategy::Queue
        }
    }
}

// ============================================================================
// Maintenance Policy
// ============================================================================

/// Policy for background maintenance tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenancePolicy {
    /// Enable background maintenance
    pub enabled: bool,
    /// Interval between maintenance runs
    pub interval: Duration,
    /// Validate idle connections during maintenance
    pub validate_idle: bool,
    /// Remove excess idle connections
    pub remove_excess_idle: bool,
    /// Recycle old connections
    pub recycle_old_connections: bool,
    /// Collect and report statistics
    pub collect_statistics: bool,
}

impl Default for MaintenancePolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(60), // 1 minute
            validate_idle: true,
            remove_excess_idle: true,
            recycle_old_connections: true,
            collect_statistics: true,
        }
    }
}

// ============================================================================
// Complete Policy Configuration
// ============================================================================

/// Complete set of pool policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolPolicies {
    pub size: SizePolicy,
    pub timeout: TimeoutPolicy,
    pub idle: IdlePolicy,
    pub overflow: OverflowPolicy,
    pub maintenance: MaintenancePolicy,
}

impl Default for PoolPolicies {
    fn default() -> Self {
        Self {
            size: SizePolicy::default(),
            timeout: TimeoutPolicy::default(),
            idle: IdlePolicy::default(),
            overflow: OverflowPolicy::default(),
            maintenance: MaintenancePolicy::default(),
        }
    }
}

impl PoolPolicies {
    pub fn validate(&self) -> Result<()> {
        self.size.validate()?;
        Ok(())
    }

    /// Builder for creating custom policies
    pub fn builder() -> PoolPoliciesBuilder {
        PoolPoliciesBuilder::default()
    }
}

// ============================================================================
// Policy Builder
// ============================================================================

#[derive(Default)]
pub struct PoolPoliciesBuilder {
    size: Option<SizePolicy>,
    timeout: Option<TimeoutPolicy>,
    idle: Option<IdlePolicy>,
    overflow: Option<OverflowPolicy>,
    maintenance: Option<MaintenancePolicy>,
}

impl PoolPoliciesBuilder {
    pub fn size(mut self, policy: SizePolicy) -> Self {
        self.size = Some(policy);
        self
    }

    pub fn timeout(mut self, policy: TimeoutPolicy) -> Self {
        self.timeout = Some(policy);
        self
    }

    pub fn idle(mut self, policy: IdlePolicy) -> Self {
        self.idle = Some(policy);
        self
    }

    pub fn overflow(mut self, policy: OverflowPolicy) -> Self {
        self.overflow = Some(policy);
        self
    }

    pub fn maintenance(mut self, policy: MaintenancePolicy) -> Self {
        self.maintenance = Some(policy);
        self
    }

    pub fn build(self) -> Result<PoolPolicies> {
        let policies = PoolPolicies {
            size: self.size.unwrap_or_default(),
            timeout: self.timeout.unwrap_or_default(),
            idle: self.idle.unwrap_or_default(),
            overflow: self.overflow.unwrap_or_default(),
            maintenance: self.maintenance.unwrap_or_default(),
        };

        policies.validate()?;
        Ok(policies)
    }
}

// ============================================================================
// Policy Enforcement Engine
// ============================================================================

/// Policy enforcement engine that applies policies to pool operations
pub struct PolicyEngine {
    policies: Arc<RwLock<PoolPolicies>>,
    enforcement_stats: Arc<EnforcementStats>,
}

impl PolicyEngine {
    pub fn new(policies: PoolPolicies) -> Self {
        Self {
            policies: Arc::new(RwLock::new(policies)),
            enforcement_stats: Arc::new(EnforcementStats::default()),
        }
    }

    /// Check if a connection acquisition should be allowed
    pub fn should_allow_acquire(&self, current_size: usize, waiting: usize) -> bool {
        let policies = self.policies.read();

        // Check if we've exceeded max connections
        if current_size >= policies.size.max_connections {
            self.enforcement_stats.size_limit_enforced.fetch_add(1, Ordering::Relaxed);
            return false;
        }

        // Check overflow policy
        match policies.overflow.strategy {
            OverflowStrategy::Reject if waiting >= policies.overflow.max_queue_size => {
                self.enforcement_stats.overflow_rejected.fetch_add(1, Ordering::Relaxed);
                return false;
            }
            _ => {}
        }

        true
    }

    /// Check if a connection should be recycled
    pub fn should_recycle(&self, age: Duration, idle_time: Duration, error_count: u64) -> bool {
        let policies = self.policies.read();

        // Check lifetime timeout
        if age >= policies.timeout.lifetime_timeout {
            self.enforcement_stats.lifetime_enforced.fetch_add(1, Ordering::Relaxed);
            return true;
        }

        // Check idle timeout
        if idle_time >= policies.timeout.idle_timeout {
            self.enforcement_stats.idle_timeout_enforced.fetch_add(1, Ordering::Relaxed);
            return true;
        }

        false
    }

    /// Get current policies
    pub fn policies(&self) -> PoolPolicies {
        self.policies.read().clone()
    }

    /// Update policies
    pub fn update_policies(&self, policies: PoolPolicies) -> Result<()> {
        policies.validate()?;
        *self.policies.write() = policies;
        self.enforcement_stats.policies_updated.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Get enforcement statistics
    pub fn stats(&self) -> EnforcementStatsSnapshot {
        EnforcementStatsSnapshot {
            size_limit_enforced: self.enforcement_stats.size_limit_enforced.load(Ordering::Relaxed),
            overflow_rejected: self.enforcement_stats.overflow_rejected.load(Ordering::Relaxed),
            lifetime_enforced: self.enforcement_stats.lifetime_enforced.load(Ordering::Relaxed),
            idle_timeout_enforced: self.enforcement_stats.idle_timeout_enforced.load(Ordering::Relaxed),
            policies_updated: self.enforcement_stats.policies_updated.load(Ordering::Relaxed),
        }
    }
}

#[derive(Default)]
struct EnforcementStats {
    size_limit_enforced: AtomicU64,
    overflow_rejected: AtomicU64,
    lifetime_enforced: AtomicU64,
    idle_timeout_enforced: AtomicU64,
    policies_updated: AtomicU64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementStatsSnapshot {
    pub size_limit_enforced: u64,
    pub overflow_rejected: u64,
    pub lifetime_enforced: u64,
    pub idle_timeout_enforced: u64,
    pub policies_updated: u64,
}
