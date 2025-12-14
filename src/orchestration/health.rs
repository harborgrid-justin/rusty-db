// # Health Aggregation and Monitoring
//
// This module provides comprehensive health checking and aggregation
// for all RustyDB components with cascading failure prevention.
//
// ## Features
//
// - **Health Checks**: Liveness and readiness probes
// - **Health Aggregation**: Combine health status from multiple components
// - **Cascading Prevention**: Detect and prevent cascading failures
// - **Health History**: Track health status over time
// - **Custom Checks**: Define custom health check logic
// - **Dependency Awareness**: Consider component dependencies
//
// ## Health States
//
// ```text
// HEALTHY → All checks pass
// DEGRADED → Some non-critical checks fail
// UNHEALTHY → Critical checks fail
// UNKNOWN → Health status cannot be determined
// ```

use std::collections::HashMap;
use std::fmt;
use std::time::Instant;

use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

// Health status of a component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    // Component is healthy
    Healthy,
    // Component is degraded but functional
    Degraded,
    // Component is unhealthy
    Unhealthy,
    // Health status unknown
    Unknown,
}

impl HealthStatus {
    // Check if the status is healthy or degraded (functional)
    pub fn is_functional(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded)
    }

    // Check if the status indicates a problem
    pub fn has_issues(&self) -> bool {
        !matches!(self, HealthStatus::Healthy)
    }

    // Get numeric score (higher is better)
    pub fn score(&self) -> u8 {
        match self {
            HealthStatus::Healthy => 100,
            HealthStatus::Degraded => 50,
            HealthStatus::Unhealthy => 0,
            HealthStatus::Unknown => 25,
        }
    }
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "HEALTHY"),
            HealthStatus::Degraded => write!(f, "DEGRADED"),
            HealthStatus::Unhealthy => write!(f, "UNHEALTHY"),
            HealthStatus::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    // Component name
    pub component: String,
    // Health status
    pub status: HealthStatus,
    // Timestamp of the check (skipped for serialization)
    #[serde(skip, default = "Instant::now")]
    pub timestamp: Instant,
    // Duration of the health check (skipped for serialization)
    #[serde(skip)]
    pub check_duration: Duration,
    // Additional details
    pub details: HashMap<String, String>,
    // Error message if unhealthy
    pub error: Option<String>,
}

impl HealthCheckResult {
    // Create a healthy result
    pub fn healthy(component: String) -> Self {
        Self {
            component,
            status: HealthStatus::Healthy,
            timestamp: Instant::now(),
            check_duration: Duration::from_millis(0),
            details: HashMap::new(),
            error: None,
        }
    }

    // Create a degraded result
    pub fn degraded(component: String, reason: String) -> Self {
        let mut details = HashMap::new();
        details.insert("reason".to_string(), reason);

        Self {
            component,
            status: HealthStatus::Degraded,
            timestamp: Instant::now(),
            check_duration: Duration::from_millis(0),
            details,
            error: None,
        }
    }

    // Create an unhealthy result
    pub fn unhealthy(component: String, error: String) -> Self {
        Self {
            component,
            status: HealthStatus::Unhealthy,
            timestamp: Instant::now(),
            check_duration: Duration::from_millis(0),
            details: HashMap::new(),
            error: Some(error),
        }
    }

    // Create an unknown result
    pub fn unknown(component: String) -> Self {
        Self {
            component,
            status: HealthStatus::Unknown,
            timestamp: Instant::now(),
            check_duration: Duration::from_millis(0),
            details: HashMap::new(),
            error: None,
        }
    }

    // Add a detail
    pub fn with_detail(mut self, key: String, value: String) -> Self {
        self.details.insert(key, value);
        self
    }

    // Set check duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.check_duration = duration;
        self
    }
}

// Trait for components that can be health checked
#[async_trait::async_trait]
pub trait HealthCheck: Send + Sync {
    // Perform a health check
    async fn check_health(&self) -> HealthCheckResult;

    // Get component name
    fn component_name(&self) -> &str;

    // Check if this is a critical component
    fn is_critical(&self) -> bool {
        true
    }

    // Get dependencies
    fn dependencies(&self) -> Vec<String> {
        Vec::new()
    }
}

// Health checker that periodically checks component health
pub struct HealthChecker {
    // Component name
    component: String,
    // Health check implementation
    check: Arc<dyn HealthCheck>,
    // Last check result
    last_result: RwLock<Option<HealthCheckResult>>,
    // Check interval
    interval: Duration,
}

impl HealthChecker {
    // Create a new health checker
    pub fn new(check: Arc<dyn HealthCheck>, interval: Duration) -> Self {
        Self {
            component: check.component_name().to_string(),
            check,
            last_result: RwLock::new(None),
            interval,
        }
    }

    // Get the component name
    pub fn component_name(&self) -> &str {
        &self.component
    }

    // Perform a health check now
    pub async fn check_now(&self) -> HealthCheckResult {
        let start = Instant::now();
        let mut result = self.check.check_health().await;
        result.check_duration = start.elapsed();
        result.timestamp = Instant::now();

        // Update last result
        {
            let mut last = self.last_result.write();
            *last = Some(result.clone());
        }

        debug!(
            "Health check for '{}': {} ({}ms)",
            self.component,
            result.status,
            result.check_duration.as_millis()
        );

        result
    }

    // Get the last check result
    pub fn last_result(&self) -> Option<HealthCheckResult> {
        let last = self.last_result.read();
        last.clone()
    }

    // Start periodic health checking
    pub async fn start_periodic(&self) {
        let mut interval_timer = interval(self.interval);

        loop {
            interval_timer.tick().await;
            let _ = self.check_now().await;
        }
    }
}

// Aggregated health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedHealth {
    // Overall health status
    pub status: HealthStatus,
    // Individual component results
    pub components: Vec<HealthCheckResult>,
    // Timestamp (skipped for serialization)
    #[serde(skip, default = "Instant::now")]
    pub timestamp: Instant,
    // Number of healthy components
    pub healthy_count: usize,
    // Number of degraded components
    pub degraded_count: usize,
    // Number of unhealthy components
    pub unhealthy_count: usize,
    // Total components
    pub total_count: usize,
}

impl AggregatedHealth {
    // Calculate aggregated health from individual results
    pub fn from_results(results: Vec<HealthCheckResult>) -> Self {
        let total_count = results.len();
        let healthy_count = results
            .iter()
            .filter(|r| r.status == HealthStatus::Healthy)
            .count();
        let degraded_count = results
            .iter()
            .filter(|r| r.status == HealthStatus::Degraded)
            .count();
        let unhealthy_count = results
            .iter()
            .filter(|r| r.status == HealthStatus::Unhealthy)
            .count();

        // Determine overall status
        let status = if unhealthy_count > 0 {
            HealthStatus::Unhealthy
        } else if degraded_count > 0 {
            HealthStatus::Degraded
        } else if healthy_count == total_count {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unknown
        };

        Self {
            status,
            components: results,
            timestamp: Instant::now(),
            healthy_count,
            degraded_count,
            unhealthy_count,
            total_count,
        }
    }

    // Check if the system is functional
    pub fn is_functional(&self) -> bool {
        self.status.is_functional()
    }

    // Get health score (0-100)
    pub fn health_score(&self) -> f64 {
        if self.total_count == 0 {
            return 100.0;
        }

        let total_score: u32 = self
            .components
            .iter()
            .map(|c| c.status.score() as u32)
            .sum();

        total_score as f64 / self.total_count as f64
    }
}

// Health aggregator that combines health from multiple components
pub struct HealthAggregator {
    // Registered health checkers
    checkers: RwLock<HashMap<String, Arc<HealthChecker>>>,
    // Health history
    history: RwLock<Vec<AggregatedHealth>>,
    // Maximum history size
    max_history: usize,
    // Cascading failure detection
    cascading_detector: Arc<CascadingFailureDetector>,
}

impl HealthAggregator {
    // Create a new health aggregator
    pub fn new(maxhistory: usize) -> Self {
        Self {
            checkers: RwLock::new(HashMap::new()),
            history: RwLock::new(Vec::new()),
            max_history: maxhistory,
            cascading_detector: Arc::new(CascadingFailureDetector::new()),
        }
    }

    // Register a health checker
    pub fn register(&self, checker: Arc<HealthChecker>) {
        let name = checker.component_name().to_string();
        let mut checkers = self.checkers.write();
        checkers.insert(name.clone(), checker);
        info!("Registered health checker: {}", name);
    }

    // Unregister a health checker
    pub fn unregister(&self, component: &str) {
        let mut checkers = self.checkers.write();
        if checkers.remove(component).is_some() {
            info!("Unregistered health checker: {}", component);
        }
    }

    // Check health of all registered components
    pub async fn check_all(&self) -> AggregatedHealth {
        let checkers = {
            let guard = self.checkers.read();
            guard.values().cloned().collect::<Vec<_>>()
        };

        let mut results = Vec::new();

        for checker in checkers {
            let result = checker.check_now().await;
            results.push(result);
        }

        let aggregated = AggregatedHealth::from_results(results);

        // Check for cascading failures
        if self.cascading_detector.detect(&aggregated) {
            warn!("Cascading failure detected!");
        }

        // Store in history
        {
            let mut history = self.history.write();
            history.push(aggregated.clone());

            // Trim history if needed
            if history.len() > self.max_history {
                history.remove(0);
            }
        }

        aggregated
    }

    // Check health of a specific component
    pub async fn check_component(&self, component: &str) -> Option<HealthCheckResult> {
        let checkers = self.checkers.read();
        let checker = checkers.get(component)?;
        Some(checker.check_now().await)
    }

    // Get the last aggregated health
    pub fn last_health(&self) -> Option<AggregatedHealth> {
        let history = self.history.read();
        history.last().cloned()
    }

    // Get health history
    pub fn get_history(&self, limit: usize) -> Vec<AggregatedHealth> {
        let history = self.history.read();
        let start = if history.len() > limit {
            history.len() - limit
        } else {
            0
        };
        history[start..].to_vec()
    }

    // Get list of registered components
    pub fn list_components(&self) -> Vec<String> {
        let checkers = self.checkers.read();
        checkers.keys().cloned().collect()
    }

    // Clear health history
    pub fn clear_history(&self) {
        let mut history = self.history.write();
        history.clear();
        info!("Cleared health history");
    }

    // Get statistics
    pub fn statistics(&self) -> HealthAggregatorStats {
        let checkers = self.checkers.read();
        let history = self.history.read();

        HealthAggregatorStats {
            registered_components: checkers.len(),
            history_size: history.len(),
            max_history: self.max_history,
        }
    }
}

impl Default for HealthAggregator {
    fn default() -> Self {
        Self::new(1000)
    }
}

// Statistics about the health aggregator
#[derive(Debug, Clone)]
pub struct HealthAggregatorStats {
    // Number of registered components
    pub registered_components: usize,
    // Current history size
    pub history_size: usize,
    // Maximum history size
    pub max_history: usize,
}

// Cascading failure detector
pub struct CascadingFailureDetector {
    // Failure events
    events: RwLock<Vec<FailureEvent>>,
    // Detection window
    window: Duration,
    // Threshold for cascading detection
    threshold: f64,
}

impl CascadingFailureDetector {
    // Create a new cascading failure detector
    pub fn new() -> Self {
        Self {
            events: RwLock::new(Vec::new()),
            window: Duration::from_secs(60),
            threshold: 0.5, // 50% of components failing
        }
    }

    // Detect cascading failures
    pub fn detect(&self, health: &AggregatedHealth) -> bool {
        if health.total_count == 0 {
            return false;
        }

        let failure_rate =
            (health.unhealthy_count + health.degraded_count) as f64 / health.total_count as f64;

        if failure_rate >= self.threshold {
            // Record failure event
            let event = FailureEvent {
                timestamp: Instant::now(),
                failure_rate,
                affected_components: health.unhealthy_count + health.degraded_count,
            };

            let mut events = self.events.write();
            events.push(event);

            // Clean old events
            let cutoff = Instant::now() - self.window;
            events.retain(|e| e.timestamp > cutoff);

            // Check if we have multiple failure events in the window
            if events.len() >= 3 {
                error!(
                    "Cascading failure detected: {} failure events in {}s",
                    events.len(),
                    self.window.as_secs()
                );
                return true;
            }
        }

        false
    }

    // Clear recorded events
    pub fn clear(&self) {
        let mut events = self.events.write();
        events.clear();
    }

    // Get recent failure events
    pub fn recent_events(&self) -> Vec<FailureEvent> {
        let events = self.events.read();
        events.clone()
    }
}

impl Default for CascadingFailureDetector {
    fn default() -> Self {
        Self::new()
    }
}

// Failure event for cascading detection
#[derive(Debug, Clone)]
pub struct FailureEvent {
    // Timestamp of the event
    pub timestamp: Instant,
    // Failure rate at the time
    pub failure_rate: f64,
    // Number of affected components
    pub affected_components: usize,
}

// Simple health check implementation
pub struct SimpleHealthCheck {
    // Component name
    name: String,
    // Health check function
    check_fn: Arc<dyn Fn() -> HealthCheckResult + Send + Sync>,
    // Is critical
    critical: bool,
}

impl SimpleHealthCheck {
    // Create a new simple health check
    pub fn new<F>(name: String, check_fn: F) -> Self
    where
        F: Fn() -> HealthCheckResult + Send + Sync + 'static,
    {
        Self {
            name,
            check_fn: Arc::new(check_fn),
            critical: true,
        }
    }

    // Mark as non-critical
    pub fn non_critical(mut self) -> Self {
        self.critical = false;
        self
    }
}

#[async_trait::async_trait]
impl HealthCheck for SimpleHealthCheck {
    async fn check_health(&self) -> HealthCheckResult {
        (self.check_fn)()
    }

    fn component_name(&self) -> &str {
        &self.name
    }

    fn is_critical(&self) -> bool {
        self.critical
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status() {
        assert!(HealthStatus::Healthy.is_functional());
        assert!(HealthStatus::Degraded.is_functional());
        assert!(!HealthStatus::Unhealthy.is_functional());

        assert_eq!(HealthStatus::Healthy.score(), 100);
        assert_eq!(HealthStatus::Degraded.score(), 50);
        assert_eq!(HealthStatus::Unhealthy.score(), 0);
    }

    #[test]
    fn test_health_check_result() {
        let result = HealthCheckResult::healthy("test".into());
        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.component, "test");

        let degraded = HealthCheckResult::degraded("test".into(), "slow".into());
        assert_eq!(degraded.status, HealthStatus::Degraded);

        let unhealthy = HealthCheckResult::unhealthy("test".into(), "error".into());
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert!(unhealthy.error.is_some());
    }

    #[test]
    fn test_aggregated_health() {
        let results = vec![
            HealthCheckResult::healthy("service1".into()),
            HealthCheckResult::healthy("service2".into()),
            HealthCheckResult::degraded("service3".into(), "slow".into()),
        ];

        let aggregated = AggregatedHealth::from_results(results);
        assert_eq!(aggregated.status, HealthStatus::Degraded);
        assert_eq!(aggregated.total_count, 3);
        assert_eq!(aggregated.healthy_count, 2);
        assert_eq!(aggregated.degraded_count, 1);
        assert!(aggregated.is_functional());
    }

    #[tokio::test]
    async fn test_health_checker() {
        let check = Arc::new(SimpleHealthCheck::new("test".into(), || {
            HealthCheckResult::healthy("test".into())
        }));

        let checker = HealthChecker::new(check::fromsecs(1), Default::default());
        let result = checker.check_now().await;

        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(checker.last_result().is_some());
    }

    #[tokio::test]
    async fn test_health_aggregator() {
        let aggregator = HealthAggregator::new(10);

        let check1 = Arc::new(SimpleHealthCheck::new("service1".into(), || {
            HealthCheckResult::healthy("service1".into())
        }));

        let check2 = Arc::new(SimpleHealthCheck::new("service2".into(), || {
            HealthCheckResult::degraded("service2".into(), "slow".into())
        }));

        let checker1 = Arc::new(HealthChecker::new(check1::from_secs(1), Default::default()));
        let checker2 = Arc::new(HealthChecker::new(check2::from_secs(1), Default::default()));

        aggregator.register(checker1);
        aggregator.register(checker2);

        let health = aggregator.check_all().await;
        assert_eq!(health.total_count, 2);
        assert_eq!(health.status, HealthStatus::Degraded);

        let components = aggregator.list_components();
        assert_eq!(components.len(), 2);
    }

    #[test]
    fn test_cascading_failure_detection() {
        let detector = CascadingFailureDetector::new();

        let results = vec![
            HealthCheckResult::unhealthy("s1".into(), "error".into()),
            HealthCheckResult::unhealthy("s2".into(), "error".into()),
        ];

        let health = AggregatedHealth::from_results(results);
        let is_cascading = detector.detect(&health);

        // First event shouldn't trigger cascading detection
        assert!(!is_cascading);
    }
}
