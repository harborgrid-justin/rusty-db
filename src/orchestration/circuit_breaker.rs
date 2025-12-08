//! # Circuit Breaker Pattern Implementation
//!
//! This module provides a robust circuit breaker implementation for fault tolerance
//! and preventing cascading failures in RustyDB.
//!
//! ## Features
//!
//! - **State Management**: Closed, Open, and Half-Open states
//! - **Failure Tracking**: Configurable failure thresholds
//! - **Automatic Recovery**: Half-open state for testing recovery
//! - **Timeout Detection**: Identify slow calls as failures
//! - **Metrics Collection**: Track success/failure rates
//! - **Custom Fallbacks**: Define fallback behavior when circuit is open
//!
//! ## State Transitions
//!
//! ```text
//! ┌─────────┐
//! │ CLOSED  │ ◄──────────┐
//! └────┬────┘            │
//!      │                 │
//!      │ Failures >= Threshold
//!      │                 │
//!      ▼                 │
//! ┌─────────┐       Success
//! │  OPEN   │            │
//! └────┬────┘            │
//!      │                 │
//!      │ After Timeout   │
//!      │                 │
//!      ▼                 │
//! ┌──────────┐           │
//! │HALF-OPEN │───────────┘
//! └──────────┘
//! ```

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::error::{DbError, Result};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests pass through
    Closed,
    /// Circuit is open, requests fail immediately
    Open,
    /// Circuit is half-open, testing if service recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: usize,
    /// Success threshold in half-open state before closing
    pub success_threshold: usize,
    /// Timeout duration for requests
    pub timeout: Duration,
    /// Duration to wait before transitioning from Open to Half-Open
    pub reset_timeout: Duration,
    /// Rolling window size for tracking failures
    pub window_size: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(10),
            reset_timeout: Duration::from_secs(60),
            window_size: 10,
        }
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone, Default)]
pub struct CircuitBreakerStats {
    /// Total number of calls
    pub total_calls: u64,
    /// Number of successful calls
    pub successful_calls: u64,
    /// Number of failed calls
    pub failed_calls: u64,
    /// Number of rejected calls (circuit open)
    pub rejected_calls: u64,
    /// Number of timeout calls
    pub timeout_calls: u64,
    /// Current consecutive failures
    pub consecutive_failures: usize,
    /// Current consecutive successes (in half-open)
    pub consecutive_successes: usize,
    /// Last state transition time
    pub last_state_change: Option<Instant>,
}

/// Internal state for circuit breaker
struct CircuitBreakerState {
    /// Current circuit state
    state: CircuitState,
    /// Statistics
    stats: CircuitBreakerStats,
    /// Configuration
    config: CircuitBreakerConfig,
}

/// Circuit breaker for fault tolerance
pub struct CircuitBreaker {
    /// Service name
    name: String,
    /// Internal state
    state: Arc<RwLock<CircuitBreakerState>>,
    /// Atomic counters for lock-free reads
    total_calls: Arc<AtomicU64>,
    successful_calls: Arc<AtomicU64>,
    failed_calls: Arc<AtomicU64>,
    rejected_calls: Arc<AtomicU64>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(name: String, config: CircuitBreakerConfig) -> Self {
        Self {
            name,
            state: Arc::new(RwLock::new(CircuitBreakerState {
                state: CircuitState::Closed,
                stats: CircuitBreakerStats::default(),
                config,
            })),
            total_calls: Arc::new(AtomicU64::new(0)),
            successful_calls: Arc::new(AtomicU64::new(0)),
            failed_calls: Arc::new(AtomicU64::new(0)),
            rejected_calls: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create with default configuration
    pub fn with_defaults(name: String) -> Self {
        Self::new(name, CircuitBreakerConfig::default())
    }

    /// Get the current circuit state
    pub fn state(&self) -> CircuitState {
        let state = self.state.read();
        state.state
    }

    /// Get the circuit breaker name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Execute a call through the circuit breaker
    pub async fn call<F, T>(&self, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        self.total_calls.fetch_add(1, Ordering::Relaxed);

        // Check if we should allow the call
        if !self.should_allow_request().await {
            self.rejected_calls.fetch_add(1, Ordering::Relaxed);
            return Err(DbError::Internal(format!(
                "Circuit breaker '{}' is OPEN",
                self.name
            )));
        }

        // Execute with timeout
        let timeout_duration = {
            let state = self.state.read();
            state.config.timeout
        };

        let result = timeout(timeout_duration, f).await;

        match result {
            Ok(Ok(value)) => {
                self.on_success().await;
                Ok(value)
            }
            Ok(Err(e)) => {
                self.on_failure().await;
                Err(e)
            }
            Err(_) => {
                self.on_timeout().await;
                Err(DbError::Internal(format!(
                    "Circuit breaker '{}' request timeout",
                    self.name
                )))
            }
        }
    }

    /// Execute with fallback
    pub async fn call_with_fallback<F, FB, T>(&self, f: F, fallback: FB) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
        FB: FnOnce() -> Result<T>,
    {
        match self.call(f).await {
            Ok(value) => Ok(value),
            Err(_) if self.state() == CircuitState::Open => {
                debug!("Circuit breaker '{}' open, using fallback", self.name);
                fallback()
            }
            Err(e) => Err(e),
        }
    }

    /// Check if request should be allowed
    async fn should_allow_request(&self) -> bool {
        let mut state = self.state.write();

        match state.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if we should transition to half-open
                if let Some(last_change) = state.stats.last_state_change {
                    if last_change.elapsed() >= state.config.reset_timeout {
                        info!("Circuit breaker '{}' transitioning to HALF-OPEN", self.name);
                        state.state = CircuitState::HalfOpen;
                        state.stats.last_state_change = Some(Instant::now());
                        state.stats.consecutive_successes = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Handle successful call
    async fn on_success(&self) {
        self.successful_calls.fetch_add(1, Ordering::Relaxed);

        let mut state = self.state.write();
        state.stats.successful_calls += 1;
        state.stats.consecutive_failures = 0;

        match state.state {
            CircuitState::HalfOpen => {
                state.stats.consecutive_successes += 1;
                if state.stats.consecutive_successes >= state.config.success_threshold {
                    info!("Circuit breaker '{}' transitioning to CLOSED", self.name);
                    state.state = CircuitState::Closed;
                    state.stats.last_state_change = Some(Instant::now());
                    state.stats.consecutive_successes = 0;
                }
            }
            CircuitState::Closed => {
                // Stay closed
            }
            CircuitState::Open => {
                // Should not happen
                warn!("Success in OPEN state for circuit breaker '{}'", self.name);
            }
        }
    }

    /// Handle failed call
    async fn on_failure(&self) {
        self.failed_calls.fetch_add(1, Ordering::Relaxed);

        let mut state = self.state.write();
        state.stats.failed_calls += 1;
        state.stats.consecutive_failures += 1;
        state.stats.consecutive_successes = 0;

        match state.state {
            CircuitState::Closed => {
                if state.stats.consecutive_failures >= state.config.failure_threshold {
                    warn!(
                        "Circuit breaker '{}' transitioning to OPEN (failures: {})",
                        self.name, state.stats.consecutive_failures
                    );
                    state.state = CircuitState::Open;
                    state.stats.last_state_change = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                warn!(
                    "Circuit breaker '{}' transitioning back to OPEN from HALF-OPEN",
                    self.name
                );
                state.state = CircuitState::Open;
                state.stats.last_state_change = Some(Instant::now());
            }
            CircuitState::Open => {
                // Stay open
            }
        }
    }

    /// Handle timeout
    async fn on_timeout(&self) {
        let mut state = self.state.write();
        state.stats.timeout_calls += 1;
        drop(state);

        // Treat timeout as failure
        self.on_failure().await;
    }

    /// Force the circuit to open
    pub async fn force_open(&self) {
        let mut state = self.state.write();
        if state.state != CircuitState::Open {
            info!("Force opening circuit breaker '{}'", self.name);
            state.state = CircuitState::Open;
            state.stats.last_state_change = Some(Instant::now());
        }
    }

    /// Force the circuit to close
    pub async fn force_close(&self) {
        let mut state = self.state.write();
        if state.state != CircuitState::Closed {
            info!("Force closing circuit breaker '{}'", self.name);
            state.state = CircuitState::Closed;
            state.stats.last_state_change = Some(Instant::now());
            state.stats.consecutive_failures = 0;
            state.stats.consecutive_successes = 0;
        }
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        let mut state = self.state.write();
        state.stats = CircuitBreakerStats::default();

        self.total_calls.store(0, Ordering::Relaxed);
        self.successful_calls.store(0, Ordering::Relaxed);
        self.failed_calls.store(0, Ordering::Relaxed);
        self.rejected_calls.store(0, Ordering::Relaxed);

        debug!("Reset statistics for circuit breaker '{}'", self.name);
    }

    /// Get current statistics
    pub fn statistics(&self) -> CircuitBreakerStats {
        let state = self.state.read();
        let mut stats = state.stats.clone();

        // Update with atomic counters
        stats.total_calls = self.total_calls.load(Ordering::Relaxed);
        stats.successful_calls = self.successful_calls.load(Ordering::Relaxed);
        stats.failed_calls = self.failed_calls.load(Ordering::Relaxed);
        stats.rejected_calls = self.rejected_calls.load(Ordering::Relaxed);

        stats
    }

    /// Get success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        let total = self.total_calls.load(Ordering::Relaxed);
        if total == 0 {
            return 1.0;
        }

        let successful = self.successful_calls.load(Ordering::Relaxed);
        successful as f64 / total as f64
    }

    /// Get failure rate (0.0 to 1.0)
    pub fn failure_rate(&self) -> f64 {
        1.0 - self.success_rate()
    }
}

impl Clone for CircuitBreaker {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            state: Arc::clone(&self.state),
            total_calls: Arc::clone(&self.total_calls),
            successful_calls: Arc::clone(&self.successful_calls),
            failed_calls: Arc::clone(&self.failed_calls),
            rejected_calls: Arc::clone(&self.rejected_calls),
        }
    }
}

/// Circuit breaker registry for managing multiple circuit breakers
pub struct CircuitBreakerRegistry {
    /// Registered circuit breakers
    breakers: RwLock<std::collections::HashMap<String, CircuitBreaker>>,
    /// Default configuration
    default_config: CircuitBreakerConfig,
}

impl CircuitBreakerRegistry {
    /// Create a new registry
    pub fn new(default_config: CircuitBreakerConfig) -> Self {
        Self {
            breakers: RwLock::new(std::collections::HashMap::new()),
            default_config,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// Get or create a circuit breaker
    pub fn get_or_create(&self, name: &str) -> CircuitBreaker {
        // Try to get existing breaker
        {
            let breakers = self.breakers.read();
            if let Some(breaker) = breakers.get(name) {
                return breaker.clone();
            }
        }

        // Create new breaker
        let mut breakers = self.breakers.write();

        // Double-check after acquiring write lock
        if let Some(breaker) = breakers.get(name) {
            return breaker.clone();
        }

        let breaker = CircuitBreaker::new(name.to_string(), self.default_config.clone());
        breakers.insert(name.to_string(), breaker.clone());
        info!("Created circuit breaker: {}", name);

        breaker
    }

    /// Get a circuit breaker by name
    pub fn get(&self, name: &str) -> Option<CircuitBreaker> {
        let breakers = self.breakers.read();
        breakers.get(name).cloned()
    }

    /// Register a circuit breaker with custom config
    pub fn register(&self, name: String, config: CircuitBreakerConfig) -> CircuitBreaker {
        let mut breakers = self.breakers.write();

        let breaker = CircuitBreaker::new(name.clone(), config);
        breakers.insert(name.clone(), breaker.clone());
        info!("Registered circuit breaker: {}", name);

        breaker
    }

    /// List all circuit breakers
    pub fn list(&self) -> Vec<String> {
        let breakers = self.breakers.read();
        breakers.keys().cloned().collect()
    }

    /// Get statistics for all circuit breakers
    pub fn all_statistics(&self) -> Vec<(String, CircuitBreakerStats)> {
        let breakers = self.breakers.read();
        breakers
            .iter()
            .map(|(name, breaker)| (name.clone(), breaker.statistics()))
            .collect()
    }

    /// Remove a circuit breaker
    pub fn remove(&self, name: &str) -> Option<CircuitBreaker> {
        let mut breakers = self.breakers.write();
        let removed = breakers.remove(name);
        if removed.is_some() {
            info!("Removed circuit breaker: {}", name);
        }
        removed
    }

    /// Clear all circuit breakers
    pub fn clear(&self) {
        let mut breakers = self.breakers.write();
        breakers.clear();
        info!("Cleared all circuit breakers");
    }

    /// Get count of registered circuit breakers
    pub fn count(&self) -> usize {
        let breakers = self.breakers.read();
        breakers.len()
    }
}

impl Default for CircuitBreakerRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
            reset_timeout: Duration::from_secs(5),
            window_size: 10,
        };

        let breaker = CircuitBreaker::new("test".into(), config);

        assert_eq!(breaker.state(), CircuitState::Closed);

        // Successful call
        let result = breaker.call(async { Ok::<_, DbError>(42) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
            reset_timeout: Duration::from_millis(100),
            window_size: 10,
        };

        let breaker = CircuitBreaker::new("test".into(), config);

        // Generate failures
        for _ in 0..3 {
            let _ = breaker
                .call(async { Err::<(), _>(DbError::Internal("failure".into())) })
                .await;
        }

        assert_eq!(breaker.state(), CircuitState::Open);

        // Next call should be rejected
        let result = breaker.call(async { Ok::<_, DbError>(42) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_recovery() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
            reset_timeout: Duration::from_millis(100),
            window_size: 10,
        };

        let breaker = CircuitBreaker::new("test".into(), config);

        // Open the circuit
        for _ in 0..2 {
            let _ = breaker
                .call(async { Err::<(), _>(DbError::Internal("failure".into())) })
                .await;
        }

        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait for reset timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Next call should transition to half-open
        let _ = breaker.call(async { Ok::<_, DbError>(1) }).await;
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Second success should close the circuit
        let _ = breaker.call(async { Ok::<_, DbError>(2) }).await;
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_fallback() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
            reset_timeout: Duration::from_secs(5),
            window_size: 10,
        };

        let breaker = CircuitBreaker::new("test".into(), config);

        // Open the circuit
        let _ = breaker
            .call(async { Err::<(), _>(DbError::Internal("failure".into())) })
            .await;

        // Use fallback
        let result = breaker
            .call_with_fallback(
                async { Ok::<_, DbError>(42) },
                || Ok(100), // fallback value
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 100);
    }

    #[tokio::test]
    async fn test_circuit_breaker_registry() {
        let registry = CircuitBreakerRegistry::with_defaults();

        let breaker1 = registry.get_or_create("service1");
        let breaker2 = registry.get_or_create("service2");
        let breaker1_again = registry.get_or_create("service1");

        assert_eq!(breaker1.name(), "service1");
        assert_eq!(breaker2.name(), "service2");

        // Should return the same instance
        assert_eq!(breaker1.name(), breaker1_again.name());

        assert_eq!(registry.count(), 2);

        let list = registry.list();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_circuit_breaker_statistics() {
        let breaker = CircuitBreaker::with_defaults("test".into());

        let _ = breaker.call(async { Ok::<_, DbError>(1) }).await;
        let _ = breaker.call(async { Ok::<_, DbError>(2) }).await;
        let _ = breaker
            .call(async { Err::<(), _>(DbError::Internal("failure".into())) })
            .await;

        let stats = breaker.statistics();
        assert_eq!(stats.total_calls, 3);
        assert_eq!(stats.successful_calls, 2);
        assert_eq!(stats.failed_calls, 1);

        let success_rate = breaker.success_rate();
        assert!((success_rate - 0.666).abs() < 0.01);
    }
}
