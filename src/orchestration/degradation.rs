//! # Graceful Degradation Strategies
//!
//! This module provides graceful degradation strategies for RustyDB,
//! allowing the system to continue operating at reduced capacity when
//! components fail or resources are constrained.
//!
//! ## Features
//!
//! - **Feature Toggles**: Disable non-essential features under load
//! - **Quality Reduction**: Reduce quality of service to maintain availability
//! - **Load Shedding**: Reject low-priority requests under pressure
//! - **Failover Strategies**: Switch to backup implementations
//! - **Rate Limiting**: Throttle requests to protect resources
//! - **Caching Strategies**: Serve stale data when necessary
//!
//! ## Degradation Levels
//!
//! ```text
//! NORMAL → DEGRADED_L1 → DEGRADED_L2 → DEGRADED_L3 → CRITICAL
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{DbError, Result};

/// Degradation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DegradationLevel {
    /// Normal operation
    Normal = 0,
    /// Level 1 degradation - minor features disabled
    DegradedL1 = 1,
    /// Level 2 degradation - significant features disabled
    DegradedL2 = 2,
    /// Level 3 degradation - only critical features enabled
    DegradedL3 = 3,
    /// Critical - minimal functionality only
    Critical = 4,
}

impl DegradationLevel {
    /// Check if this level allows a feature
    pub fn allows_feature(&self, feature: Feature) -> bool {
        feature.min_level() <= *self
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            DegradationLevel::Normal => "Normal operation",
            DegradationLevel::DegradedL1 => "Minor degradation - analytics disabled",
            DegradationLevel::DegradedL2 => "Moderate degradation - complex queries limited",
            DegradationLevel::DegradedL3 => "Severe degradation - read-only mode",
            DegradationLevel::Critical => "Critical - emergency mode",
        }
    }
}

/// Features that can be degraded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Feature {
    /// Analytics and OLAP queries
    Analytics,
    /// Full-text search
    FullTextSearch,
    /// Complex joins
    ComplexJoins,
    /// Materialized views
    MaterializedViews,
    /// Background indexing
    BackgroundIndexing,
    /// Query optimization
    QueryOptimization,
    /// Write operations
    WriteOperations,
    /// Transaction logging
    TransactionLogging,
    /// Replication
    Replication,
}

impl Feature {
    /// Get the minimum degradation level required to use this feature
    pub fn min_level(&self) -> DegradationLevel {
        match self {
            Feature::Analytics => DegradationLevel::Normal,
            Feature::FullTextSearch => DegradationLevel::Normal,
            Feature::ComplexJoins => DegradationLevel::DegradedL1,
            Feature::MaterializedViews => DegradationLevel::Normal,
            Feature::BackgroundIndexing => DegradationLevel::Normal,
            Feature::QueryOptimization => DegradationLevel::DegradedL2,
            Feature::WriteOperations => DegradationLevel::DegradedL2,
            Feature::TransactionLogging => DegradationLevel::DegradedL3,
            Feature::Replication => DegradationLevel::DegradedL2,
        }
    }

    /// Check if this is a critical feature
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            Feature::TransactionLogging | Feature::WriteOperations
        )
    }
}

/// Degradation trigger that determines when to degrade
#[derive(Debug, Clone)]
pub struct DegradationTrigger {
    /// Trigger name
    pub name: String,
    /// CPU threshold (0.0 to 1.0)
    pub cpu_threshold: Option<f64>,
    /// Memory threshold (0.0 to 1.0)
    pub memory_threshold: Option<f64>,
    /// Error rate threshold (0.0 to 1.0)
    pub error_rate_threshold: Option<f64>,
    /// Latency threshold (milliseconds)
    pub latency_threshold: Option<u64>,
    /// Custom condition
    pub custom_condition: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
}

impl DegradationTrigger {
    /// Create a new trigger
    pub fn new(name: String) -> Self {
        Self {
            name,
            cpu_threshold: None,
            memory_threshold: None,
            error_rate_threshold: None,
            latency_threshold: None,
            custom_condition: None,
        }
    }

    /// Set CPU threshold
    pub fn with_cpu_threshold(mut self, threshold: f64) -> Self {
        self.cpu_threshold = Some(threshold);
        self
    }

    /// Set memory threshold
    pub fn with_memory_threshold(mut self, threshold: f64) -> Self {
        self.memory_threshold = Some(threshold);
        self
    }

    /// Set error rate threshold
    pub fn with_error_rate_threshold(mut self, threshold: f64) -> Self {
        self.error_rate_threshold = Some(threshold);
        self
    }

    /// Set latency threshold
    pub fn with_latency_threshold(mut self, threshold_ms: u64) -> Self {
        self.latency_threshold = Some(threshold_ms);
        self
    }

    /// Evaluate if the trigger should fire
    pub fn evaluate(&self, metrics: &SystemMetrics) -> bool {
        if let Some(threshold) = self.cpu_threshold {
            if metrics.cpu_usage > threshold {
                return true;
            }
        }

        if let Some(threshold) = self.memory_threshold {
            if metrics.memory_usage > threshold {
                return true;
            }
        }

        if let Some(threshold) = self.error_rate_threshold {
            if metrics.error_rate > threshold {
                return true;
            }
        }

        if let Some(threshold) = self.latency_threshold {
            if metrics.avg_latency_ms > threshold {
                return true;
            }
        }

        if let Some(ref condition) = self.custom_condition {
            if condition() {
                return true;
            }
        }

        false
    }
}

/// System metrics for degradation decisions
#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    /// CPU usage (0.0 to 1.0)
    pub cpu_usage: f64,
    /// Memory usage (0.0 to 1.0)
    pub memory_usage: f64,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,
    /// Average latency (milliseconds)
    pub avg_latency_ms: u64,
    /// Active connections
    pub active_connections: usize,
    /// Queue depth
    pub queue_depth: usize,
}

/// Degradation strategy
pub struct DegradationStrategy {
    /// Current degradation level
    current_level: RwLock<DegradationLevel>,
    /// Triggers for each level
    triggers: RwLock<HashMap<DegradationLevel, Vec<DegradationTrigger>>>,
    /// Disabled features
    disabled_features: RwLock<HashMap<Feature, bool>>,
    /// Metrics
    metrics: Arc<RwLock<SystemMetrics>>,
    /// Statistics
    level_changes: Arc<AtomicU64>,
    /// Last level change time
    last_change: RwLock<Option<Instant>>,
}

impl DegradationStrategy {
    /// Create a new degradation strategy
    pub fn new() -> Self {
        Self {
            current_level: RwLock::new(DegradationLevel::Normal),
            triggers: RwLock::new(HashMap::new()),
            disabled_features: RwLock::new(HashMap::new()),
            metrics: Arc::new(RwLock::new(SystemMetrics::default())),
            level_changes: Arc::new(AtomicU64::new(0)),
            last_change: RwLock::new(None),
        }
    }

    /// Get current degradation level
    pub fn current_level(&self) -> DegradationLevel {
        *self.current_level.read()
    }

    /// Set degradation level manually
    pub fn set_level(&self, level: DegradationLevel) {
        let mut current = self.current_level.write();
        if *current != level {
            info!(
                "Degradation level changed: {:?} -> {:?}",
                *current, level
            );
            *current = level;
            self.level_changes.fetch_add(1, Ordering::Relaxed);
            *self.last_change.write() = Some(Instant::now());

            // Update disabled features
            self.update_disabled_features(level);
        }
    }

    /// Update disabled features based on level
    fn update_disabled_features(&self, level: DegradationLevel) {
        let mut disabled = self.disabled_features.write();
        disabled.clear();

        // Disable features that require higher level
        for &feature in &[
            Feature::Analytics,
            Feature::FullTextSearch,
            Feature::ComplexJoins,
            Feature::MaterializedViews,
            Feature::BackgroundIndexing,
            Feature::QueryOptimization,
            Feature::WriteOperations,
            Feature::TransactionLogging,
            Feature::Replication,
        ] {
            if !level.allows_feature(feature) {
                disabled.insert(feature, true);
                debug!("Disabled feature: {:?}", feature);
            }
        }
    }

    /// Check if a feature is enabled
    pub fn is_feature_enabled(&self, feature: Feature) -> bool {
        let disabled = self.disabled_features.read();
        !disabled.get(&feature).copied().unwrap_or(false)
    }

    /// Register a trigger for a degradation level
    pub fn register_trigger(&self, level: DegradationLevel, trigger: DegradationTrigger) {
        let mut triggers = self.triggers.write();
        triggers
            .entry(level)
            .or_insert_with(Vec::new)
            .push(trigger);
    }

    /// Update system metrics
    pub fn update_metrics(&self, metrics: SystemMetrics) {
        let mut current = self.metrics.write();
        *current = metrics;
    }

    /// Evaluate triggers and adjust degradation level
    pub fn evaluate(&self) {
        let metrics = self.metrics.read().clone();
        let triggers = self.triggers.read();

        // Check triggers from highest to lowest level
        let levels = vec![
            DegradationLevel::Critical,
            DegradationLevel::DegradedL3,
            DegradationLevel::DegradedL2,
            DegradationLevel::DegradedL1,
            DegradationLevel::Normal,
        ];

        for level in levels {
            if let Some(level_triggers) = triggers.get(&level) {
                for trigger in level_triggers {
                    if trigger.evaluate(&metrics) {
                        // Set to this level if higher than current
                        let current = self.current_level();
                        if level > current {
                            warn!(
                                "Degradation trigger '{}' fired, changing to level: {:?}",
                                trigger.name, level
                            );
                            self.set_level(level);
                            return;
                        }
                    }
                }
            }
        }

        // If no triggers fired and we're degraded, try to recover
        let current = self.current_level();
        if current > DegradationLevel::Normal {
            // Check if we should recover (basic heuristic)
            if metrics.cpu_usage < 0.5
                && metrics.memory_usage < 0.7
                && metrics.error_rate < 0.01
            {
                // Recover one level
                let new_level = match current {
                    DegradationLevel::Critical => DegradationLevel::DegradedL3,
                    DegradationLevel::DegradedL3 => DegradationLevel::DegradedL2,
                    DegradationLevel::DegradedL2 => DegradationLevel::DegradedL1,
                    DegradationLevel::DegradedL1 => DegradationLevel::Normal,
                    DegradationLevel::Normal => DegradationLevel::Normal,
                };

                if new_level < current {
                    info!("System recovering, upgrading to level: {:?}", new_level);
                    self.set_level(new_level);
                }
            }
        }
    }

    /// Get disabled features
    pub fn get_disabled_features(&self) -> Vec<Feature> {
        let disabled = self.disabled_features.read();
        disabled
            .iter()
            .filter(|(_, &v)| v)
            .map(|(&k, _)| k)
            .collect()
    }

    /// Get statistics
    pub fn statistics(&self) -> DegradationStats {
        DegradationStats {
            current_level: self.current_level(),
            level_changes: self.level_changes.load(Ordering::Relaxed),
            disabled_features_count: self.disabled_features.read().len(),
            last_change: *self.last_change.read(),
        }
    }

    /// Reset to normal
    pub fn reset(&self) {
        self.set_level(DegradationLevel::Normal);
        info!("Degradation strategy reset to normal");
    }
}

impl Default for DegradationStrategy {
    fn default() -> Self {
        Self::new()
    }
}

/// Degradation statistics
#[derive(Debug, Clone)]
pub struct DegradationStats {
    /// Current degradation level
    pub current_level: DegradationLevel,
    /// Number of level changes
    pub level_changes: u64,
    /// Number of disabled features
    pub disabled_features_count: usize,
    /// Last level change time
    pub last_change: Option<Instant>,
}

/// Load shedding strategy
pub struct LoadShedder {
    /// Rejection rate (0.0 to 1.0)
    rejection_rate: RwLock<f64>,
    /// Total requests
    total_requests: Arc<AtomicU64>,
    /// Rejected requests
    rejected_requests: Arc<AtomicU64>,
    /// Priority threshold (requests below this are rejected)
    priority_threshold: RwLock<u8>,
}

impl LoadShedder {
    /// Create a new load shedder
    pub fn new() -> Self {
        Self {
            rejection_rate: RwLock::new(0.0),
            total_requests: Arc::new(AtomicU64::new(0)),
            rejected_requests: Arc::new(AtomicU64::new(0)),
            priority_threshold: RwLock::new(0),
        }
    }

    /// Set rejection rate
    pub fn set_rejection_rate(&self, rate: f64) {
        let rate = rate.clamp(0.0, 1.0);
        *self.rejection_rate.write() = rate;
        if rate > 0.0 {
            info!("Load shedding enabled: {}% rejection", rate * 100.0);
        } else {
            info!("Load shedding disabled");
        }
    }

    /// Set priority threshold
    pub fn set_priority_threshold(&self, threshold: u8) {
        *self.priority_threshold.write() = threshold;
        info!("Priority threshold set to: {}", threshold);
    }

    /// Check if a request should be accepted
    pub fn should_accept(&self, priority: u8) -> bool {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        // Check priority first
        let threshold = *self.priority_threshold.read();
        if priority < threshold {
            self.rejected_requests.fetch_add(1, Ordering::Relaxed);
            return false;
        }

        // Check rejection rate
        let rate = *self.rejection_rate.read();
        if rate > 0.0 {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            if rng.gen::<f64>() < rate {
                self.rejected_requests.fetch_add(1, Ordering::Relaxed);
                return false;
            }
        }

        true
    }

    /// Get current rejection rate
    pub fn current_rejection_rate(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }

        let rejected = self.rejected_requests.load(Ordering::Relaxed);
        rejected as f64 / total as f64
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.rejected_requests.store(0, Ordering::Relaxed);
    }

    /// Get statistics
    pub fn statistics(&self) -> LoadShedderStats {
        LoadShedderStats {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            rejected_requests: self.rejected_requests.load(Ordering::Relaxed),
            configured_rejection_rate: *self.rejection_rate.read(),
            actual_rejection_rate: self.current_rejection_rate(),
        }
    }
}

impl Default for LoadShedder {
    fn default() -> Self {
        Self::new()
    }
}

/// Load shedder statistics
#[derive(Debug, Clone)]
pub struct LoadShedderStats {
    /// Total requests processed
    pub total_requests: u64,
    /// Total requests rejected
    pub rejected_requests: u64,
    /// Configured rejection rate
    pub configured_rejection_rate: f64,
    /// Actual rejection rate
    pub actual_rejection_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_degradation_levels() {
        assert!(DegradationLevel::Normal < DegradationLevel::DegradedL1);
        assert!(DegradationLevel::DegradedL3 < DegradationLevel::Critical);
    }

    #[test]
    fn test_feature_enablement() {
        assert!(DegradationLevel::Normal.allows_feature(Feature::Analytics));
        assert!(!DegradationLevel::DegradedL1.allows_feature(Feature::Analytics));
        assert!(DegradationLevel::DegradedL1.allows_feature(Feature::ComplexJoins));
    }

    #[test]
    fn test_degradation_strategy() {
        let strategy = DegradationStrategy::new();

        assert_eq!(strategy.current_level(), DegradationLevel::Normal);
        assert!(strategy.is_feature_enabled(Feature::Analytics));

        strategy.set_level(DegradationLevel::DegradedL1);
        assert_eq!(strategy.current_level(), DegradationLevel::DegradedL1);
        assert!(!strategy.is_feature_enabled(Feature::Analytics));
    }

    #[test]
    fn test_degradation_trigger() {
        let trigger = DegradationTrigger::new("high_cpu".into())
            .with_cpu_threshold(0.8)
            .with_memory_threshold(0.9);

        let mut metrics = SystemMetrics::default();
        metrics.cpu_usage = 0.7;
        assert!(!trigger.evaluate(&metrics));

        metrics.cpu_usage = 0.85;
        assert!(trigger.evaluate(&metrics));
    }

    #[test]
    fn test_load_shedder() {
        let shedder = LoadShedder::new();

        // No rejection initially
        assert!(shedder.should_accept(5));

        // Set 50% rejection rate
        shedder.set_rejection_rate(0.5);

        // Check multiple requests
        let mut accepted = 0;
        let mut rejected = 0;
        for _ in 0..1000 {
            if shedder.should_accept(5) {
                accepted += 1;
            } else {
                rejected += 1;
            }
        }

        // Should be roughly 50/50 (with some variance)
        let actual_rate = rejected as f64 / 1000.0;
        assert!(actual_rate > 0.4 && actual_rate < 0.6);
    }

    #[test]
    fn test_priority_threshold() {
        let shedder = LoadShedder::new();
        shedder.set_priority_threshold(5);

        // Low priority should be rejected
        assert!(!shedder.should_accept(3));

        // High priority should be accepted
        assert!(shedder.should_accept(7));
    }

    #[test]
    fn test_degradation_stats() {
        let strategy = DegradationStrategy::new();

        strategy.set_level(DegradationLevel::DegradedL1);
        strategy.set_level(DegradationLevel::DegradedL2);

        let stats = strategy.statistics();
        assert_eq!(stats.current_level, DegradationLevel::DegradedL2);
        assert_eq!(stats.level_changes, 2);
        assert!(stats.last_change.is_some());
    }
}
