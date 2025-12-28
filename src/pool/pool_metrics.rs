// # Connection Pool Metrics and Monitoring
//
// This module provides comprehensive monitoring and metrics collection for connection pools:
// - Connection utilization tracking
// - Wait time statistics and percentiles
// - Connection lifetime analytics
// - Pool sizing recommendations based on historical data
// - Performance trending and anomaly detection
//
// ## Metrics Categories
//
// - **Utilization**: Active/idle connections, utilization percentage
// - **Latency**: Acquire times, wait times, percentiles (p50, p95, p99)
// - **Throughput**: Acquisitions/releases per second, connection churn
// - **Lifecycle**: Connection age, lifetime distribution
// - **Errors**: Timeouts, validation failures, creation errors
//
// ## Pool Sizing Recommendations
//
// Based on collected metrics, this module provides intelligent recommendations for:
// - Minimum pool size (based on baseline load)
// - Maximum pool size (based on peak load)
// - Optimal initial size (based on warm-up patterns)

use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;

// ============================================================================
// Metrics Collector
// ============================================================================

/// Central metrics collector for connection pool
pub struct PoolMetricsCollector {
    /// Pool identifier
    pool_name: String,
    /// Start time for uptime calculation
    start_time: Instant,
    /// Connection lifecycle metrics
    lifecycle: Arc<LifecycleMetrics>,
    /// Utilization metrics
    utilization: Arc<UtilizationMetrics>,
    /// Latency metrics
    latency: Arc<LatencyMetrics>,
    /// Error metrics
    errors: Arc<ErrorMetrics>,
    /// Historical data for trending
    history: Arc<RwLock<MetricsHistory>>,
}

impl PoolMetricsCollector {
    pub fn new(pool_name: impl Into<String>) -> Self {
        Self {
            pool_name: pool_name.into(),
            start_time: Instant::now(),
            lifecycle: Arc::new(LifecycleMetrics::default()),
            utilization: Arc::new(UtilizationMetrics::default()),
            latency: Arc::new(LatencyMetrics::new()),
            errors: Arc::new(ErrorMetrics::default()),
            history: Arc::new(RwLock::new(MetricsHistory::new())),
        }
    }

    /// Record connection creation
    pub fn record_connection_created(&self) {
        self.lifecycle.connections_created.fetch_add(1, Ordering::Relaxed);
        self.utilization.total_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Record connection closure
    pub fn record_connection_closed(&self, age: Duration) {
        self.lifecycle.connections_closed.fetch_add(1, Ordering::Relaxed);
        self.utilization.total_connections.fetch_sub(1, Ordering::Relaxed);
        self.lifecycle.record_lifetime(age);
    }

    /// Record connection acquisition
    pub fn record_connection_acquired(&self, wait_time: Duration) {
        self.lifecycle.acquisitions.fetch_add(1, Ordering::Relaxed);
        self.utilization.active_connections.fetch_add(1, Ordering::Relaxed);
        self.latency.record_wait_time(wait_time);
    }

    /// Record connection release
    pub fn record_connection_released(&self, hold_time: Duration) {
        self.lifecycle.releases.fetch_add(1, Ordering::Relaxed);
        self.utilization.active_connections.fetch_sub(1, Ordering::Relaxed);
        self.latency.record_hold_time(hold_time);
    }

    /// Record validation failure
    pub fn record_validation_failure(&self) {
        self.errors.validation_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Record creation failure
    pub fn record_creation_failure(&self) {
        self.errors.creation_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Record timeout
    pub fn record_timeout(&self) {
        self.errors.timeouts.fetch_add(1, Ordering::Relaxed);
    }

    /// Record pool exhaustion
    pub fn record_pool_exhausted(&self) {
        self.errors.pool_exhausted.fetch_add(1, Ordering::Relaxed);
    }

    /// Get snapshot of all metrics
    pub fn snapshot(&self) -> PoolMetricsSnapshot {
        PoolMetricsSnapshot {
            pool_name: self.pool_name.clone(),
            uptime: Instant::now().duration_since(self.start_time),
            lifecycle: self.lifecycle.snapshot(),
            utilization: self.utilization.snapshot(),
            latency: self.latency.snapshot(),
            errors: self.errors.snapshot(),
            timestamp: Instant::now(),
        }
    }

    /// Save current snapshot to history
    pub fn save_snapshot(&self) {
        let snapshot = self.snapshot();
        self.history.write().add_snapshot(snapshot);
    }

    /// Get historical metrics
    pub fn history(&self) -> MetricsHistory {
        self.history.read().clone()
    }

    /// Get pool sizing recommendation
    pub fn sizing_recommendation(&self) -> SizingRecommendation {
        let history = self.history.read();
        let current = self.snapshot();

        history.calculate_sizing_recommendation(&current)
    }
}

// ============================================================================
// Lifecycle Metrics
// ============================================================================

#[derive(Default)]
pub struct LifecycleMetrics {
    pub connections_created: AtomicU64,
    pub connections_closed: AtomicU64,
    pub acquisitions: AtomicU64,
    pub releases: AtomicU64,
    pub recycled: AtomicU64,
    lifetime_sum: AtomicU64,
    lifetime_count: AtomicU64,
}

impl LifecycleMetrics {
    fn record_lifetime(&self, duration: Duration) {
        self.lifetime_sum.fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
        self.lifetime_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> LifecycleSnapshot {
        let lifetime_count = self.lifetime_count.load(Ordering::Relaxed);
        let avg_lifetime = if lifetime_count > 0 {
            let sum = self.lifetime_sum.load(Ordering::Relaxed);
            Duration::from_millis(sum / lifetime_count)
        } else {
            Duration::ZERO
        };

        LifecycleSnapshot {
            connections_created: self.connections_created.load(Ordering::Relaxed),
            connections_closed: self.connections_closed.load(Ordering::Relaxed),
            acquisitions: self.acquisitions.load(Ordering::Relaxed),
            releases: self.releases.load(Ordering::Relaxed),
            recycled: self.recycled.load(Ordering::Relaxed),
            avg_connection_lifetime: avg_lifetime,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleSnapshot {
    pub connections_created: u64,
    pub connections_closed: u64,
    pub acquisitions: u64,
    pub releases: u64,
    pub recycled: u64,
    pub avg_connection_lifetime: Duration,
}

// ============================================================================
// Utilization Metrics
// ============================================================================

#[derive(Default)]
pub struct UtilizationMetrics {
    pub total_connections: AtomicUsize,
    pub active_connections: AtomicUsize,
    pub idle_connections: AtomicUsize,
    pub waiting_requests: AtomicUsize,
}

impl UtilizationMetrics {
    pub fn snapshot(&self) -> UtilizationSnapshot {
        let total = self.total_connections.load(Ordering::Relaxed);
        let active = self.active_connections.load(Ordering::Relaxed);
        let idle = total.saturating_sub(active);

        let utilization_pct = if total > 0 {
            (active as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        UtilizationSnapshot {
            total_connections: total,
            active_connections: active,
            idle_connections: idle,
            waiting_requests: self.waiting_requests.load(Ordering::Relaxed),
            utilization_percentage: utilization_pct,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilizationSnapshot {
    pub total_connections: usize,
    pub active_connections: usize,
    pub idle_connections: usize,
    pub waiting_requests: usize,
    pub utilization_percentage: f64,
}

// ============================================================================
// Latency Metrics
// ============================================================================

const HISTOGRAM_SIZE: usize = 1000;

pub struct LatencyMetrics {
    wait_times: RwLock<VecDeque<Duration>>,
    hold_times: RwLock<VecDeque<Duration>>,
    wait_sum: AtomicU64,
    wait_count: AtomicU64,
    hold_sum: AtomicU64,
    hold_count: AtomicU64,
}

impl LatencyMetrics {
    pub fn new() -> Self {
        Self {
            wait_times: RwLock::new(VecDeque::with_capacity(HISTOGRAM_SIZE)),
            hold_times: RwLock::new(VecDeque::with_capacity(HISTOGRAM_SIZE)),
            wait_sum: AtomicU64::new(0),
            wait_count: AtomicU64::new(0),
            hold_sum: AtomicU64::new(0),
            hold_count: AtomicU64::new(0),
        }
    }

    pub fn record_wait_time(&self, duration: Duration) {
        let millis = duration.as_millis() as u64;
        self.wait_sum.fetch_add(millis, Ordering::Relaxed);
        self.wait_count.fetch_add(1, Ordering::Relaxed);

        let mut times = self.wait_times.write();
        if times.len() >= HISTOGRAM_SIZE {
            times.pop_front();
        }
        times.push_back(duration);
    }

    pub fn record_hold_time(&self, duration: Duration) {
        let millis = duration.as_millis() as u64;
        self.hold_sum.fetch_add(millis, Ordering::Relaxed);
        self.hold_count.fetch_add(1, Ordering::Relaxed);

        let mut times = self.hold_times.write();
        if times.len() >= HISTOGRAM_SIZE {
            times.pop_front();
        }
        times.push_back(duration);
    }

    pub fn snapshot(&self) -> LatencySnapshot {
        let wait_count = self.wait_count.load(Ordering::Relaxed);
        let avg_wait = if wait_count > 0 {
            Duration::from_millis(self.wait_sum.load(Ordering::Relaxed) / wait_count)
        } else {
            Duration::ZERO
        };

        let hold_count = self.hold_count.load(Ordering::Relaxed);
        let avg_hold = if hold_count > 0 {
            Duration::from_millis(self.hold_sum.load(Ordering::Relaxed) / hold_count)
        } else {
            Duration::ZERO
        };

        let wait_percentiles = self.calculate_percentiles(&self.wait_times.read());
        let hold_percentiles = self.calculate_percentiles(&self.hold_times.read());

        LatencySnapshot {
            avg_wait_time: avg_wait,
            avg_hold_time: avg_hold,
            wait_p50: wait_percentiles.0,
            wait_p95: wait_percentiles.1,
            wait_p99: wait_percentiles.2,
            hold_p50: hold_percentiles.0,
            hold_p95: hold_percentiles.1,
            hold_p99: hold_percentiles.2,
        }
    }

    fn calculate_percentiles(&self, times: &VecDeque<Duration>) -> (Duration, Duration, Duration) {
        if times.is_empty() {
            return (Duration::ZERO, Duration::ZERO, Duration::ZERO);
        }

        let mut sorted: Vec<_> = times.iter().copied().collect();
        sorted.sort();

        let p50_idx = (sorted.len() as f64 * 0.50) as usize;
        let p95_idx = (sorted.len() as f64 * 0.95) as usize;
        let p99_idx = (sorted.len() as f64 * 0.99) as usize;

        (
            sorted.get(p50_idx).copied().unwrap_or(Duration::ZERO),
            sorted.get(p95_idx).copied().unwrap_or(Duration::ZERO),
            sorted.get(p99_idx).copied().unwrap_or(Duration::ZERO),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencySnapshot {
    pub avg_wait_time: Duration,
    pub avg_hold_time: Duration,
    pub wait_p50: Duration,
    pub wait_p95: Duration,
    pub wait_p99: Duration,
    pub hold_p50: Duration,
    pub hold_p95: Duration,
    pub hold_p99: Duration,
}

// ============================================================================
// Error Metrics
// ============================================================================

#[derive(Default)]
pub struct ErrorMetrics {
    pub validation_failures: AtomicU64,
    pub creation_failures: AtomicU64,
    pub timeouts: AtomicU64,
    pub pool_exhausted: AtomicU64,
    pub connection_leaks: AtomicU64,
}

impl ErrorMetrics {
    pub fn snapshot(&self) -> ErrorSnapshot {
        ErrorSnapshot {
            validation_failures: self.validation_failures.load(Ordering::Relaxed),
            creation_failures: self.creation_failures.load(Ordering::Relaxed),
            timeouts: self.timeouts.load(Ordering::Relaxed),
            pool_exhausted: self.pool_exhausted.load(Ordering::Relaxed),
            connection_leaks: self.connection_leaks.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSnapshot {
    pub validation_failures: u64,
    pub creation_failures: u64,
    pub timeouts: u64,
    pub pool_exhausted: u64,
    pub connection_leaks: u64,
}

// ============================================================================
// Complete Pool Metrics Snapshot
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetricsSnapshot {
    pub pool_name: String,
    pub uptime: Duration,
    pub lifecycle: LifecycleSnapshot,
    pub utilization: UtilizationSnapshot,
    pub latency: LatencySnapshot,
    pub errors: ErrorSnapshot,
    #[serde(skip, default = "Instant::now")]
    pub timestamp: Instant,
}

// ============================================================================
// Metrics History and Trending
// ============================================================================

const MAX_HISTORY_SIZE: usize = 1000;

#[derive(Clone)]
pub struct MetricsHistory {
    snapshots: VecDeque<PoolMetricsSnapshot>,
}

impl MetricsHistory {
    pub fn new() -> Self {
        Self {
            snapshots: VecDeque::with_capacity(MAX_HISTORY_SIZE),
        }
    }

    pub fn add_snapshot(&mut self, snapshot: PoolMetricsSnapshot) {
        if self.snapshots.len() >= MAX_HISTORY_SIZE {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back(snapshot);
    }

    pub fn snapshots(&self) -> &VecDeque<PoolMetricsSnapshot> {
        &self.snapshots
    }

    /// Calculate pool sizing recommendation based on historical data
    pub fn calculate_sizing_recommendation(&self, current: &PoolMetricsSnapshot) -> SizingRecommendation {
        if self.snapshots.is_empty() {
            return SizingRecommendation::default_for(current);
        }

        // Calculate min connections based on lowest utilization
        let min_active = self.snapshots
            .iter()
            .map(|s| s.utilization.active_connections)
            .min()
            .unwrap_or(1);

        // Calculate max connections based on peak utilization
        let max_active = self.snapshots
            .iter()
            .map(|s| s.utilization.active_connections)
            .max()
            .unwrap_or(10);

        // Calculate average for initial size
        let avg_active = if !self.snapshots.is_empty() {
            self.snapshots
                .iter()
                .map(|s| s.utilization.active_connections)
                .sum::<usize>() / self.snapshots.len()
        } else {
            5
        };

        // Add buffer for spikes
        let recommended_min = min_active.max(1);
        let recommended_max = (max_active as f64 * 1.5) as usize;
        let recommended_initial = (avg_active as f64 * 1.2) as usize;

        // Calculate confidence based on data points
        let confidence = if self.snapshots.len() > 100 {
            0.9
        } else if self.snapshots.len() > 50 {
            0.7
        } else {
            0.5
        };

        SizingRecommendation {
            min_connections: recommended_min,
            max_connections: recommended_max,
            initial_connections: recommended_initial,
            confidence,
            rationale: format!(
                "Based on {} samples: min={}, max={}, avg={}",
                self.snapshots.len(), min_active, max_active, avg_active
            ),
        }
    }
}

// ============================================================================
// Pool Sizing Recommendation
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizingRecommendation {
    pub min_connections: usize,
    pub max_connections: usize,
    pub initial_connections: usize,
    pub confidence: f64,
    pub rationale: String,
}

impl SizingRecommendation {
    fn default_for(snapshot: &PoolMetricsSnapshot) -> Self {
        let current_total = snapshot.utilization.total_connections;
        Self {
            min_connections: (current_total / 2).max(1),
            max_connections: current_total * 2,
            initial_connections: current_total,
            confidence: 0.3,
            rationale: "Insufficient historical data for accurate recommendation".to_string(),
        }
    }
}
