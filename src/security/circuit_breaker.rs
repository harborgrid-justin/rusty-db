//! # Enterprise Circuit Breaker & Resilience Patterns
//!
//! This module provides comprehensive resilience patterns for preventing cascading failures
//! and ensuring graceful degradation in distributed systems.
//!
//! ## Core Components
//!
//! - **CircuitBreaker**: Three-state (Closed/Open/Half-Open) failure isolation
//! - **Bulkhead**: Resource pool isolation to prevent resource exhaustion
//! - **TimeoutManager**: Adaptive timeout calculation based on latency percentiles
//! - **RetryPolicy**: Exponential backoff with jitter for transient failures
//! - **FallbackHandler**: Graceful degradation with cached responses
//! - **CascadePreventor**: Stop failure propagation across system boundaries
//! - **ResilienceMetrics**: Comprehensive tracking of resilience patterns
//! - **LoadShedder**: Priority-based admission control under load
//!
//! ## Architecture
//!
//! The resilience patterns are designed to work together:
//! ```text
//! Request → LoadShedder → CircuitBreaker → Bulkhead → TimeoutManager → RetryPolicy → Operation
//!                                ↓ (on failure)
//!                          FallbackHandler
//! ```

use crate::DbError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};
use parking_lot::RwLock;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use rand::Rng;

// ============================================================================
// Constants
// ============================================================================

/// Default failure threshold before opening circuit
const DEFAULT_FAILURE_THRESHOLD: u64 = 5;

/// Default success threshold before closing circuit
const DEFAULT_SUCCESS_THRESHOLD: u64 = 2;

/// Default timeout before transitioning to half-open
const DEFAULT_TIMEOUT_DURATION: Duration = Duration::from_secs(30);

/// Maximum number of half-open requests
const DEFAULT_HALF_OPEN_MAX_REQUESTS: u32 = 3;

/// Default sliding window size for failure tracking
const DEFAULT_WINDOW_SIZE: usize = 100;

/// Latency percentile window size
const LATENCY_WINDOW_SIZE: usize = 1000;

/// Default base backoff duration
const DEFAULT_BASE_BACKOFF: Duration = Duration::from_millis(100);

/// Default maximum backoff duration
const DEFAULT_MAX_BACKOFF: Duration = Duration::from_secs(60);

/// Default backoff multiplier
const DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;

/// Default maximum retry attempts
const DEFAULT_MAX_RETRIES: u32 = 3;

// ============================================================================
// CircuitBreaker - Three-State Failure Isolation
// ============================================================================

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Circuit is closed - requests flow normally
    Closed,

    /// Circuit is open - requests fail fast
    Open,

    /// Circuit is half-open - testing recovery
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u64,

    /// Failure rate threshold (0.0 - 1.0)
    pub failure_rate_threshold: f64,

    /// Number of successes in half-open before closing
    pub success_threshold: u64,

    /// Duration to wait before transitioning to half-open
    pub timeout: Duration,

    /// Maximum requests allowed in half-open state
    pub half_open_max_requests: u32,

    /// Sliding window size for failure tracking
    pub window_size: usize,

    /// Minimum number of calls before calculating failure rate
    pub minimum_calls: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: DEFAULT_FAILURE_THRESHOLD,
            failure_rate_threshold: 0.5,
            success_threshold: DEFAULT_SUCCESS_THRESHOLD,
            timeout: DEFAULT_TIMEOUT_DURATION,
            half_open_max_requests: DEFAULT_HALF_OPEN_MAX_REQUESTS,
            window_size: DEFAULT_WINDOW_SIZE,
            minimum_calls: 10,
        }
    }
}

/// Call outcome for tracking
#[derive(Debug, Clone, Copy)]
enum CallOutcome {
    Success,
    Failure,
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    /// Circuit identifier
    name: String,

    /// Current state
    state: Arc<RwLock<CircuitState>>,

    /// Configuration
    config: CircuitBreakerConfig,

    /// Failure count
    failure_count: AtomicU64,

    /// Success count in current window
    success_count: AtomicU64,

    /// Consecutive successes in half-open state
    consecutive_successes: AtomicU64,

    /// Half-open request counter
    half_open_requests: AtomicU64,

    /// Last state transition time
    last_transition: Arc<RwLock<Instant>>,

    /// Sliding window of call outcomes
    call_outcomes: Arc<Mutex<VecDeque<CallOutcome>>>,

    /// Metrics
    metrics: Arc<CircuitBreakerMetrics>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(name: String, config: CircuitBreakerConfig) -> Self {
        Self {
            name,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            config,
            failure_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            consecutive_successes: AtomicU64::new(0),
            half_open_requests: AtomicU64::new(0),
            last_transition: Arc::new(RwLock::new(Instant::now())),
            call_outcomes: Arc::new(Mutex::new(VecDeque::new())),
            metrics: Arc::new(CircuitBreakerMetrics::new()),
        }
    }

    /// Execute operation with circuit breaker protection
    pub async fn call<F, T, E>(&self, operation: F) -> std::result::Result<T, E>
    where
        F: std::future::Future<Output = std::result::Result<T, E>>,
        E: From<DbError>,
    {
        // Check if we should allow the call
        if !self.allow_request() {
            self.metrics.rejected_calls.fetch_add(1, Ordering::Relaxed);
            return Err(DbError::CircuitBreakerOpen(self.name.clone()).into());
        }

        // Execute operation
        let start = Instant::now();
        let result = operation.await;
        let duration = start.elapsed();

        // Record outcome
        match &result {
            Ok(_) => self.on_success(duration),
            Err(_) => self.on_failure(duration),
        }

        result
    }

    /// Check if request should be allowed
    fn allow_request(&self) -> bool {
        let state = *self.state.read();

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if we should transition to half-open
                let elapsed = self.last_transition.read().elapsed();
                if elapsed >= self.config.timeout {
                    self.transition_to_half_open();
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited requests in half-open state
                let current = self.half_open_requests.load(Ordering::Relaxed);
                if current < self.config.half_open_max_requests as u64 {
                    self.half_open_requests.fetch_add(1, Ordering::Relaxed);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Handle successful call
    fn on_success(&self, duration: Duration) {
        self.success_count.fetch_add(1, Ordering::Relaxed);
        self.metrics.successful_calls.fetch_add(1, Ordering::Relaxed);
        self.metrics.record_latency(duration);

        // Add to sliding window
        self.record_outcome(CallOutcome::Success);

        let state = *self.state.read();

        match state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                let consecutive = self.consecutive_successes.fetch_add(1, Ordering::Relaxed) + 1;

                if consecutive >= self.config.success_threshold {
                    self.transition_to_closed();
                }
            }
            CircuitState::Open => {
                // Should not happen, but handle gracefully
            }
        }
    }

    /// Handle failed call
    fn on_failure(&self, duration: Duration) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);
        self.metrics.failed_calls.fetch_add(1, Ordering::Relaxed);
        self.metrics.record_latency(duration);

        // Add to sliding window
        self.record_outcome(CallOutcome::Failure);

        let state = *self.state.read();

        match state {
            CircuitState::Closed => {
                // Check if we should open the circuit
                if self.should_open_circuit() {
                    self.transition_to_open();
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open immediately opens circuit
                self.transition_to_open();
            }
            CircuitState::Open => {
                // Already open, nothing to do
            }
        }
    }

    /// Record call outcome in sliding window
    fn record_outcome(&self, outcome: CallOutcome) {
        let mut outcomes = self.call_outcomes.lock();
        outcomes.push_back(outcome);

        if outcomes.len() > self.config.window_size {
            outcomes.pop_front();
        }
    }

    /// Check if circuit should open based on failure rate
    fn should_open_circuit(&self) -> bool {
        let outcomes = self.call_outcomes.lock();
        let total = outcomes.len() as u64;

        // Need minimum number of calls to make decision
        if total < self.config.minimum_calls {
            return false;
        }

        let failures = outcomes.iter()
            .filter(|o| matches!(o, CallOutcome::Failure))
            .count() as u64;

        let failure_rate = failures as f64 / total as f64;

        // Open if failure rate exceeds threshold OR absolute failure count exceeds threshold
        failure_rate >= self.config.failure_rate_threshold
            || failures >= self.config.failure_threshold
    }

    /// Transition to open state
    fn transition_to_open(&self) {
        *self.state.write() = CircuitState::Open;
        *self.last_transition.write() = Instant::now();
        self.consecutive_successes.store(0, Ordering::Relaxed);
        self.half_open_requests.store(0, Ordering::Relaxed);
        self.metrics.state_transitions.fetch_add(1, Ordering::Relaxed);

        tracing::warn!(
            circuit_breaker = %self.name,
            "Circuit breaker opened due to failures"
        );
    }

    /// Transition to half-open state
    fn transition_to_half_open(&self) {
        *self.state.write() = CircuitState::HalfOpen;
        *self.last_transition.write() = Instant::now();
        self.consecutive_successes.store(0, Ordering::Relaxed);
        self.half_open_requests.store(0, Ordering::Relaxed);
        self.metrics.state_transitions.fetch_add(1, Ordering::Relaxed);

        tracing::info!(
            circuit_breaker = %self.name,
            "Circuit breaker transitioning to half-open for recovery testing"
        );
    }

    /// Transition to closed state
    fn transition_to_closed(&self) {
        *self.state.write() = CircuitState::Closed;
        *self.last_transition.write() = Instant::now();
        self.failure_count.store(0, Ordering::Relaxed);
        self.consecutive_successes.store(0, Ordering::Relaxed);
        self.half_open_requests.store(0, Ordering::Relaxed);
        self.metrics.state_transitions.fetch_add(1, Ordering::Relaxed);

        tracing::info!(
            circuit_breaker = %self.name,
            "Circuit breaker closed - normal operation resumed"
        );
    }

    /// Get current circuit state
    pub fn state(&self) -> CircuitState {
        *self.state.read()
    }

    /// Force circuit to open (for testing or manual intervention)
    pub fn force_open(&self) {
        self.transition_to_open();
    }

    /// Force circuit to close (for testing or manual intervention)
    pub fn force_close(&self) {
        self.transition_to_closed();
    }

    /// Get circuit breaker metrics
    pub fn metrics(&self) -> CircuitBreakerMetricsSnapshot {
        self.metrics.snapshot()
    }
}

/// Circuit breaker metrics
pub struct CircuitBreakerMetrics {
    successful_calls: AtomicU64,
    failed_calls: AtomicU64,
    rejected_calls: AtomicU64,
    state_transitions: AtomicU64,
    latencies: Mutex<VecDeque<Duration>>,
}

impl CircuitBreakerMetrics {
    fn new() -> Self {
        Self {
            successful_calls: AtomicU64::new(0),
            failed_calls: AtomicU64::new(0),
            rejected_calls: AtomicU64::new(0),
            state_transitions: AtomicU64::new(0),
            latencies: Mutex::new(VecDeque::new()),
        }
    }

    fn record_latency(&self, duration: Duration) {
        let mut latencies = self.latencies.lock();
        latencies.push_back(duration);

        if latencies.len() > LATENCY_WINDOW_SIZE {
            latencies.pop_front();
        }
    }

    fn snapshot(&self) -> CircuitBreakerMetricsSnapshot {
        let latencies = self.latencies.lock();
        let mut sorted: Vec<_> = latencies.iter().map(|d| d.as_micros() as u64).collect();
        sorted.sort_unstable();

        let p50 = percentile(&sorted, 0.5);
        let p95 = percentile(&sorted, 0.95);
        let p99 = percentile(&sorted, 0.99);

        CircuitBreakerMetricsSnapshot {
            successful_calls: self.successful_calls.load(Ordering::Relaxed),
            failed_calls: self.failed_calls.load(Ordering::Relaxed),
            rejected_calls: self.rejected_calls.load(Ordering::Relaxed),
            state_transitions: self.state_transitions.load(Ordering::Relaxed),
            p50_latency_us: p50,
            p95_latency_us: p95,
            p99_latency_us: p99,
        }
    }
}

/// Circuit breaker metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerMetricsSnapshot {
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub rejected_calls: u64,
    pub state_transitions: u64,
    pub p50_latency_us: u64,
    pub p95_latency_us: u64,
    pub p99_latency_us: u64,
}

// ============================================================================
// Bulkhead - Resource Pool Isolation
// ============================================================================

/// Bulkhead configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadConfig {
    /// Maximum concurrent requests
    pub max_concurrent: usize,

    /// Maximum queue size
    pub max_queue_size: usize,

    /// Timeout for acquiring permit
    pub acquire_timeout: Duration,
}

impl Default for BulkheadConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 100,
            max_queue_size: 1000,
            acquire_timeout: Duration::from_secs(30),
        }
    }
}

/// Bulkhead for resource isolation
pub struct Bulkhead {
    /// Bulkhead name
    name: String,

    /// Semaphore for concurrency control
    semaphore: Arc<Semaphore>,

    /// Configuration
    config: BulkheadConfig,

    /// Queue size counter
    queue_size: AtomicUsize,

    /// Metrics
    metrics: Arc<BulkheadMetrics>,
}

impl Bulkhead {
    /// Create a new bulkhead
    pub fn new(name: String, config: BulkheadConfig) -> Self {
        Self {
            name,
            semaphore: Arc::new(Semaphore::new(config.max_concurrent)),
            config,
            queue_size: AtomicUsize::new(0),
            metrics: Arc::new(BulkheadMetrics::new()),
        }
    }

    /// Execute operation with bulkhead protection
    pub async fn call<F, T, E>(&self, operation: F) -> std::result::Result<T, E>
    where
        F: std::future::Future<Output = std::result::Result<T, E>>,
        E: From<DbError>,
    {
        // Check queue size
        let queue_size = self.queue_size.load(Ordering::Relaxed);
        if queue_size >= self.config.max_queue_size {
            self.metrics.rejected_calls.fetch_add(1, Ordering::Relaxed);
            return Err(DbError::BulkheadFull(self.name.clone()).into());
        }

        // Increment queue size
        self.queue_size.fetch_add(1, Ordering::Relaxed);
        let _guard = QueueGuard::new(&self.queue_size);

        // Acquire permit with timeout
        let permit = match timeout(
            self.config.acquire_timeout,
            self.semaphore.clone().acquire_owned()
        ).await {
            Ok(Ok(permit)) => permit,
            Ok(Err(_)) => {
                self.metrics.rejected_calls.fetch_add(1, Ordering::Relaxed);
                return Err(DbError::BulkheadFull(self.name.clone()).into());
            }
            Err(_) => {
                self.metrics.timeout_calls.fetch_add(1, Ordering::Relaxed);
                return Err(DbError::LockTimeout.into());
            }
        };

        self.metrics.active_calls.fetch_add(1, Ordering::Relaxed);
        let _active_guard = ActiveGuard::new(&self.metrics.active_calls);

        // Execute operation
        let start = Instant::now();
        let result = operation.await;
        let duration = start.elapsed();

        self.metrics.record_call(duration);

        result
    }

    /// Get bulkhead metrics
    pub fn metrics(&self) -> BulkheadMetricsSnapshot {
        self.metrics.snapshot(self.queue_size.load(Ordering::Relaxed))
    }
}

/// Guard to decrement queue size on drop
struct QueueGuard<'a> {
    counter: &'a AtomicUsize,
}

impl<'a> QueueGuard<'a> {
    fn new(counter: &'a AtomicUsize) -> Self {
        Self { counter }
    }
}

impl<'a> Drop for QueueGuard<'a> {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Guard to decrement active calls on drop
struct ActiveGuard<'a> {
    counter: &'a AtomicU64,
}

impl<'a> ActiveGuard<'a> {
    fn new(counter: &'a AtomicU64) -> Self {
        Self { counter }
    }
}

impl<'a> Drop for ActiveGuard<'a> {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Bulkhead metrics
struct BulkheadMetrics {
    active_calls: AtomicU64,
    total_calls: AtomicU64,
    rejected_calls: AtomicU64,
    timeout_calls: AtomicU64,
    total_wait_time: AtomicU64, // microseconds
}

impl BulkheadMetrics {
    fn new() -> Self {
        Self {
            active_calls: AtomicU64::new(0),
            total_calls: AtomicU64::new(0),
            rejected_calls: AtomicU64::new(0),
            timeout_calls: AtomicU64::new(0),
            total_wait_time: AtomicU64::new(0),
        }
    }

    fn record_call(&self, duration: Duration) {
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        self.total_wait_time.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    fn snapshot(&self, current_queue_size: usize) -> BulkheadMetricsSnapshot {
        let total = self.total_calls.load(Ordering::Relaxed);
        let total_wait = self.total_wait_time.load(Ordering::Relaxed);

        BulkheadMetricsSnapshot {
            active_calls: self.active_calls.load(Ordering::Relaxed),
            total_calls: total,
            rejected_calls: self.rejected_calls.load(Ordering::Relaxed),
            timeout_calls: self.timeout_calls.load(Ordering::Relaxed),
            current_queue_size,
            avg_wait_time_us: if total > 0 { total_wait / total } else { 0 },
        }
    }
}

/// Bulkhead metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadMetricsSnapshot {
    pub active_calls: u64,
    pub total_calls: u64,
    pub rejected_calls: u64,
    pub timeout_calls: u64,
    pub current_queue_size: usize,
    pub avg_wait_time_us: u64,
}

// ============================================================================
// TimeoutManager - Adaptive Timeout Calculation
// ============================================================================

/// Timeout manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutManagerConfig {
    /// Base timeout duration
    pub base_timeout: Duration,

    /// Percentile to use for adaptive timeout (0.95 = P95)
    pub percentile: f64,

    /// Multiplier applied to percentile latency
    pub multiplier: f64,

    /// Minimum timeout
    pub min_timeout: Duration,

    /// Maximum timeout
    pub max_timeout: Duration,
}

impl Default for TimeoutManagerConfig {
    fn default() -> Self {
        Self {
            base_timeout: Duration::from_secs(30),
            percentile: 0.95,
            multiplier: 1.5,
            min_timeout: Duration::from_millis(100),
            max_timeout: Duration::from_secs(300),
        }
    }
}

/// Timeout manager for adaptive timeouts
pub struct TimeoutManager {
    /// Configuration
    config: TimeoutManagerConfig,

    /// Latency samples per endpoint
    latencies: Arc<RwLock<HashMap<String, VecDeque<Duration>>>>,

    /// Metrics
    metrics: Arc<RwLock<HashMap<String, TimeoutMetrics>>>,
}

impl TimeoutManager {
    /// Create a new timeout manager
    pub fn new(config: TimeoutManagerConfig) -> Self {
        Self {
            config,
            latencies: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Execute operation with adaptive timeout
    pub async fn call<F, T, E>(&self, endpoint: &str, operation: F) -> std::result::Result<T, E>
    where
        F: std::future::Future<Output = std::result::Result<T, E>>,
        E: From<DbError>,
    {
        let timeout_duration = self.calculate_timeout(endpoint);

        let start = Instant::now();
        let result = match timeout(timeout_duration, operation).await {
            Ok(result) => {
                let duration = start.elapsed();
                self.record_latency(endpoint, duration);
                result
            }
            Err(_) => {
                self.record_timeout(endpoint);
                return Err(DbError::LockTimeout.into());
            }
        };

        result
    }

    /// Calculate adaptive timeout for endpoint
    pub fn calculate_timeout(&self, endpoint: &str) -> Duration {
        let latencies = self.latencies.read();

        if let Some(samples) = latencies.get(endpoint) {
            if samples.len() < 10 {
                // Not enough samples, use base timeout
                return self.config.base_timeout;
            }

            let mut sorted: Vec<_> = samples.iter().map(|d| d.as_micros() as u64).collect();
            sorted.sort_unstable();

            let p_latency = percentile(&sorted, self.config.percentile);
            let adaptive = Duration::from_micros((p_latency as f64 * self.config.multiplier) as u64);

            // Clamp to min/max
            adaptive.clamp(self.config.min_timeout, self.config.max_timeout)
        } else {
            self.config.base_timeout
        }
    }

    /// Record latency sample
    fn record_latency(&self, endpoint: &str, duration: Duration) {
        let mut latencies = self.latencies.write();
        let samples = latencies.entry(endpoint.to_string()).or_insert_with(VecDeque::new);

        samples.push_back(duration);
        if samples.len() > LATENCY_WINDOW_SIZE {
            samples.pop_front();
        }

        // Update metrics
        let mut metrics = self.metrics.write();
        let endpoint_metrics = metrics.entry(endpoint.to_string()).or_insert_with(TimeoutMetrics::new);
        endpoint_metrics.total_calls += 1;
    }

    /// Record timeout
    fn record_timeout(&self, endpoint: &str) {
        let mut metrics = self.metrics.write();
        let endpoint_metrics = metrics.entry(endpoint.to_string()).or_insert_with(TimeoutMetrics::new);
        endpoint_metrics.timeout_count += 1;
    }

    /// Get timeout metrics for endpoint
    pub fn metrics(&self, endpoint: &str) -> Option<TimeoutMetrics> {
        self.metrics.read().get(endpoint).cloned()
    }
}

/// Timeout metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutMetrics {
    pub total_calls: u64,
    pub timeout_count: u64,
}

impl TimeoutMetrics {
    fn new() -> Self {
        Self {
            total_calls: 0,
            timeout_count: 0,
        }
    }
}

// ============================================================================
// RetryPolicy - Exponential Backoff with Jitter
// ============================================================================

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicyConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,

    /// Base backoff duration
    pub base_backoff: Duration,

    /// Maximum backoff duration
    pub max_backoff: Duration,

    /// Backoff multiplier
    pub multiplier: f64,

    /// Add jitter to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryPolicyConfig {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_RETRIES,
            base_backoff: DEFAULT_BASE_BACKOFF,
            max_backoff: DEFAULT_MAX_BACKOFF,
            multiplier: DEFAULT_BACKOFF_MULTIPLIER,
            jitter: true,
        }
    }
}

/// Retry policy with exponential backoff
pub struct RetryPolicy {
    /// Configuration
    config: RetryPolicyConfig,

    /// Metrics
    metrics: Arc<RetryMetrics>,
}

impl RetryPolicy {
    /// Create a new retry policy
    pub fn new(config: RetryPolicyConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(RetryMetrics::new()),
        }
    }

    /// Execute operation with retry
    pub async fn call<F, Fut, T, E>(&self, mut operation: F) -> std::result::Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, E>>,
        E: From<DbError>,
    {
        let mut attempt = 0;

        loop {
            attempt += 1;
            self.metrics.total_attempts.fetch_add(1, Ordering::Relaxed);

            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        self.metrics.successful_retries.fetch_add(1, Ordering::Relaxed);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    if attempt >= self.config.max_attempts {
                        self.metrics.exhausted_retries.fetch_add(1, Ordering::Relaxed);
                        return Err(e);
                    }

                    // Calculate backoff
                    let backoff = self.calculate_backoff(attempt);

                    self.metrics.total_backoff_time.fetch_add(
                        backoff.as_micros() as u64,
                        Ordering::Relaxed
                    );

                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }

    /// Calculate backoff duration for attempt
    fn calculate_backoff(&self, attempt: u32) -> Duration {
        let base_ms = self.config.base_backoff.as_millis() as f64;
        let exponential = base_ms * self.config.multiplier.powi((attempt - 1) as i32);
        let clamped = exponential.min(self.config.max_backoff.as_millis() as f64);

        let backoff = if self.config.jitter {
            // Add jitter: random value between 0.5 and 1.5 times the backoff
            let jitter = rand::thread_rng().gen_range(0.5..=1.5);
            Duration::from_millis((clamped * jitter) as u64)
        } else {
            Duration::from_millis(clamped as u64)
        };

        backoff.clamp(self.config.base_backoff, self.config.max_backoff)
    }

    /// Get retry metrics
    pub fn metrics(&self) -> RetryMetricsSnapshot {
        self.metrics.snapshot()
    }
}

/// Retry metrics
struct RetryMetrics {
    total_attempts: AtomicU64,
    successful_retries: AtomicU64,
    exhausted_retries: AtomicU64,
    total_backoff_time: AtomicU64, // microseconds
}

impl RetryMetrics {
    fn new() -> Self {
        Self {
            total_attempts: AtomicU64::new(0),
            successful_retries: AtomicU64::new(0),
            exhausted_retries: AtomicU64::new(0),
            total_backoff_time: AtomicU64::new(0),
        }
    }

    fn snapshot(&self) -> RetryMetricsSnapshot {
        RetryMetricsSnapshot {
            total_attempts: self.total_attempts.load(Ordering::Relaxed),
            successful_retries: self.successful_retries.load(Ordering::Relaxed),
            exhausted_retries: self.exhausted_retries.load(Ordering::Relaxed),
            total_backoff_time_us: self.total_backoff_time.load(Ordering::Relaxed),
        }
    }
}

/// Retry metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryMetricsSnapshot {
    pub total_attempts: u64,
    pub successful_retries: u64,
    pub exhausted_retries: u64,
    pub total_backoff_time_us: u64,
}

// ============================================================================
// FallbackHandler - Graceful Degradation
// ============================================================================

/// Fallback response
pub enum FallbackResponse<T> {
    /// Cached response
    Cached(T),

    /// Default value
    Default(T),

    /// Empty response
    Empty,

    /// Custom fallback
    Custom(T),
}

/// Fallback handler for graceful degradation
pub struct FallbackHandler<T> {
    /// Cached responses
    cache: Arc<RwLock<HashMap<String, (T, SystemTime)>>>,

    /// Cache TTL
    cache_ttl: Duration,

    /// Default value provider
    default_provider: Option<Arc<dyn Fn() -> T + Send + Sync>>,

    /// Metrics
    metrics: Arc<FallbackMetrics>,
}

impl<T: Clone + Send + Sync> FallbackHandler<T> {
    /// Create a new fallback handler
    pub fn new(cache_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl,
            default_provider: None,
            metrics: Arc::new(FallbackMetrics::new()),
        }
    }

    /// Set default value provider
    pub fn with_default<F>(mut self, provider: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        self.default_provider = Some(Arc::new(provider));
        self
    }

    /// Cache response
    pub fn cache_response(&self, key: String, value: T) {
        let mut cache = self.cache.write();
        cache.insert(key, (value, SystemTime::now()));
    }

    /// Get cached response
    pub fn get_cached(&self, key: &str) -> Option<T> {
        let cache = self.cache.read();

        if let Some((value, cached_at)) = cache.get(key) {
            let age = SystemTime::now().duration_since(*cached_at).unwrap_or_default();

            if age < self.cache_ttl {
                self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Some(value.clone());
            }
        }

        self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Get fallback response
    pub fn get_fallback(&self, key: &str) -> Option<FallbackResponse<T>> {
        // Try cache first
        if let Some(cached) = self.get_cached(key) {
            self.metrics.fallback_used.fetch_add(1, Ordering::Relaxed);
            return Some(FallbackResponse::Cached(cached));
        }

        // Try default
        if let Some(ref provider) = self.default_provider {
            self.metrics.fallback_used.fetch_add(1, Ordering::Relaxed);
            return Some(FallbackResponse::Default(provider()));
        }

        None
    }

    /// Get fallback metrics
    pub fn metrics(&self) -> FallbackMetricsSnapshot {
        self.metrics.snapshot()
    }
}

/// Fallback metrics
struct FallbackMetrics {
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    fallback_used: AtomicU64,
}

impl FallbackMetrics {
    fn new() -> Self {
        Self {
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            fallback_used: AtomicU64::new(0),
        }
    }

    fn snapshot(&self) -> FallbackMetricsSnapshot {
        FallbackMetricsSnapshot {
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            fallback_used: self.fallback_used.load(Ordering::Relaxed),
        }
    }
}

/// Fallback metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackMetricsSnapshot {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub fallback_used: u64,
}

// ============================================================================
// CascadePreventor - Stop Failure Propagation
// ============================================================================

/// Cascade prevention configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadePreventorConfig {
    /// Error rate threshold to trigger prevention
    pub error_rate_threshold: f64,

    /// Window size for error rate calculation
    pub window_size: usize,

    /// Minimum calls before triggering
    pub minimum_calls: u64,

    /// Fast-fail duration after trigger
    pub fast_fail_duration: Duration,
}

impl Default for CascadePreventorConfig {
    fn default() -> Self {
        Self {
            error_rate_threshold: 0.7,
            window_size: 100,
            minimum_calls: 20,
            fast_fail_duration: Duration::from_secs(10),
        }
    }
}

/// Cascade preventor
pub struct CascadePreventor {
    /// Configuration
    config: CascadePreventorConfig,

    /// Recent outcomes
    outcomes: Arc<Mutex<VecDeque<bool>>>, // true = success, false = failure

    /// Fast-fail mode
    fast_fail_until: Arc<RwLock<Option<Instant>>>,

    /// Metrics
    metrics: Arc<CascadeMetrics>,
}

impl CascadePreventor {
    /// Create a new cascade preventor
    pub fn new(config: CascadePreventorConfig) -> Self {
        Self {
            config,
            outcomes: Arc::new(Mutex::new(VecDeque::new())),
            fast_fail_until: Arc::new(RwLock::new(None)),
            metrics: Arc::new(CascadeMetrics::new()),
        }
    }

    /// Check if request should be allowed
    pub fn allow_request(&self) -> bool {
        // Check if in fast-fail mode
        if let Some(until) = *self.fast_fail_until.read() {
            if Instant::now() < until {
                self.metrics.prevented_calls.fetch_add(1, Ordering::Relaxed);
                return false;
            } else {
                // Exit fast-fail mode
                *self.fast_fail_until.write() = None;
            }
        }

        true
    }

    /// Record call outcome
    pub fn record_outcome(&self, success: bool) {
        let mut outcomes = self.outcomes.lock();
        outcomes.push_back(success);

        if outcomes.len() > self.config.window_size {
            outcomes.pop_front();
        }

        // Check if we should trigger fast-fail
        if outcomes.len() >= self.config.minimum_calls as usize {
            let failures = outcomes.iter().filter(|&&s| !s).count();
            let error_rate = failures as f64 / outcomes.len() as f64;

            if error_rate >= self.config.error_rate_threshold {
                // Trigger fast-fail mode
                let until = Instant::now() + self.config.fast_fail_duration;
                *self.fast_fail_until.write() = Some(until);

                self.metrics.cascade_preventions.fetch_add(1, Ordering::Relaxed);

                tracing::warn!(
                    error_rate = %error_rate,
                    threshold = %self.config.error_rate_threshold,
                    "Cascade prevention activated - entering fast-fail mode"
                );
            }
        }
    }

    /// Get cascade prevention metrics
    pub fn metrics(&self) -> CascadeMetricsSnapshot {
        self.metrics.snapshot()
    }
}

/// Cascade prevention metrics
struct CascadeMetrics {
    prevented_calls: AtomicU64,
    cascade_preventions: AtomicU64,
}

impl CascadeMetrics {
    fn new() -> Self {
        Self {
            prevented_calls: AtomicU64::new(0),
            cascade_preventions: AtomicU64::new(0),
        }
    }

    fn snapshot(&self) -> CascadeMetricsSnapshot {
        CascadeMetricsSnapshot {
            prevented_calls: self.prevented_calls.load(Ordering::Relaxed),
            cascade_preventions: self.cascade_preventions.load(Ordering::Relaxed),
        }
    }
}

/// Cascade prevention metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeMetricsSnapshot {
    pub prevented_calls: u64,
    pub cascade_preventions: u64,
}

// ============================================================================
// LoadShedder - Priority-Based Admission Control
// ============================================================================

/// Request priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RequestPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Load shedder configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadShedderConfig {
    /// Queue depth threshold for starting to shed
    pub queue_depth_threshold: usize,

    /// CPU usage threshold (0.0 - 1.0)
    pub cpu_threshold: f64,

    /// Memory usage threshold (0.0 - 1.0)
    pub memory_threshold: f64,

    /// Enable priority-based shedding
    pub priority_based: bool,
}

impl Default for LoadShedderConfig {
    fn default() -> Self {
        Self {
            queue_depth_threshold: 1000,
            cpu_threshold: 0.9,
            memory_threshold: 0.9,
            priority_based: true,
        }
    }
}

/// Load shedder for admission control
pub struct LoadShedder {
    /// Configuration
    config: LoadShedderConfig,

    /// Current queue depth
    queue_depth: AtomicUsize,

    /// Metrics
    metrics: Arc<LoadShedderMetrics>,
}

impl LoadShedder {
    /// Create a new load shedder
    pub fn new(config: LoadShedderConfig) -> Self {
        Self {
            config,
            queue_depth: AtomicUsize::new(0),
            metrics: Arc::new(LoadShedderMetrics::new()),
        }
    }

    /// Check if request should be admitted
    pub fn admit_request(&self, priority: RequestPriority) -> bool {
        let queue_depth = self.queue_depth.load(Ordering::Relaxed);

        // Always admit critical requests
        if priority == RequestPriority::Critical {
            self.metrics.admitted_requests.fetch_add(1, Ordering::Relaxed);
            return true;
        }

        // Check queue depth
        if queue_depth >= self.config.queue_depth_threshold {
            if self.config.priority_based {
                // Shed low priority requests first
                if priority == RequestPriority::Low {
                    self.metrics.shed_requests.fetch_add(1, Ordering::Relaxed);
                    return false;
                }

                // Shed normal priority if severely overloaded
                if queue_depth >= self.config.queue_depth_threshold * 2 && priority == RequestPriority::Normal {
                    self.metrics.shed_requests.fetch_add(1, Ordering::Relaxed);
                    return false;
                }
            } else {
                // Shed all non-critical
                self.metrics.shed_requests.fetch_add(1, Ordering::Relaxed);
                return false;
            }
        }

        self.metrics.admitted_requests.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// Increment queue depth
    pub fn increment_queue_depth(&self) {
        self.queue_depth.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement queue depth
    pub fn decrement_queue_depth(&self) {
        self.queue_depth.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get load shedder metrics
    pub fn metrics(&self) -> LoadShedderMetricsSnapshot {
        self.metrics.snapshot(self.queue_depth.load(Ordering::Relaxed))
    }
}

/// Load shedder metrics
struct LoadShedderMetrics {
    admitted_requests: AtomicU64,
    shed_requests: AtomicU64,
}

impl LoadShedderMetrics {
    fn new() -> Self {
        Self {
            admitted_requests: AtomicU64::new(0),
            shed_requests: AtomicU64::new(0),
        }
    }

    fn snapshot(&self, current_queue_depth: usize) -> LoadShedderMetricsSnapshot {
        LoadShedderMetricsSnapshot {
            admitted_requests: self.admitted_requests.load(Ordering::Relaxed),
            shed_requests: self.shed_requests.load(Ordering::Relaxed),
            current_queue_depth,
        }
    }
}

/// Load shedder metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadShedderMetricsSnapshot {
    pub admitted_requests: u64,
    pub shed_requests: u64,
    pub current_queue_depth: usize,
}

// ============================================================================
// ResilienceMetrics - Comprehensive Tracking
// ============================================================================

/// Comprehensive resilience metrics
pub struct ResilienceMetrics {
    /// Circuit breaker metrics by name
    pub circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreakerMetricsSnapshot>>>,

    /// Bulkhead metrics by name
    pub bulkheads: Arc<RwLock<HashMap<String, BulkheadMetricsSnapshot>>>,

    /// Timeout metrics by endpoint
    pub timeouts: Arc<RwLock<HashMap<String, TimeoutMetrics>>>,

    /// Retry metrics
    pub retry_metrics: Arc<RwLock<RetryMetricsSnapshot>>,

    /// Fallback metrics
    pub fallback_metrics: Arc<RwLock<FallbackMetricsSnapshot>>,

    /// Cascade prevention metrics
    pub cascade_metrics: Arc<RwLock<CascadeMetricsSnapshot>>,

    /// Load shedder metrics
    pub load_shedder_metrics: Arc<RwLock<LoadShedderMetricsSnapshot>>,
}

impl ResilienceMetrics {
    /// Create new resilience metrics
    pub fn new() -> Self {
        Self {
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            bulkheads: Arc::new(RwLock::new(HashMap::new())),
            timeouts: Arc::new(RwLock::new(HashMap::new())),
            retry_metrics: Arc::new(RwLock::new(RetryMetricsSnapshot {
                total_attempts: 0,
                successful_retries: 0,
                exhausted_retries: 0,
                total_backoff_time_us: 0,
            })),
            fallback_metrics: Arc::new(RwLock::new(FallbackMetricsSnapshot {
                cache_hits: 0,
                cache_misses: 0,
                fallback_used: 0,
            })),
            cascade_metrics: Arc::new(RwLock::new(CascadeMetricsSnapshot {
                prevented_calls: 0,
                cascade_preventions: 0,
            })),
            load_shedder_metrics: Arc::new(RwLock::new(LoadShedderMetricsSnapshot {
                admitted_requests: 0,
                shed_requests: 0,
                current_queue_depth: 0,
            })),
        }
    }
}

impl Default for ResilienceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Calculate percentile from sorted values
fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }

    let index = ((sorted.len() as f64 - 1.0) * p) as usize;
    sorted.get(index).copied().unwrap_or(0)
}

// ============================================================================
// Error Extensions
// ============================================================================

impl From<DbError> for String {
    fn from(err: DbError) -> Self {
        format!("{:?}", err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let config = CircuitBreakerConfig::default();
        let cb = CircuitBreaker::new("test".to_string(), config);

        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            minimum_calls: 3,
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test".to_string(), config);

        // Simulate failures
        for _ in 0..3 {
            cb.on_failure(Duration::from_millis(10));
        }

        // Should be open now
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn test_bulkhead_limits_concurrency() {
        let config = BulkheadConfig {
            max_concurrent: 2,
            ..Default::default()
        };
        let bulkhead = Arc::new(Bulkhead::new("test".to_string(), config));

        let b1 = bulkhead.clone();
        let b2 = bulkhead.clone();
        let b3 = bulkhead.clone();

        // Start two concurrent operations
        let handle1 = tokio::spawn(async move {
            b1.call(async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok::<_, DbError>(())
            }).await
        });

        let handle2 = tokio::spawn(async move {
            b2.call(async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok::<_, DbError>(())
            }).await
        });

        // Third should be blocked
        tokio::time::sleep(Duration::from_millis(10)).await;

        let handle3 = tokio::spawn(async move {
            b3.call(async {
                Ok::<_, DbError>(())
            }).await
        });

        handle1.await.unwrap().unwrap();
        handle2.await.unwrap().unwrap();
        handle3.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_retry_policy_with_backoff() {
        let config = RetryPolicyConfig {
            max_attempts: 3,
            base_backoff: Duration::from_millis(10),
            ..Default::default()
        };
        let retry = RetryPolicy::new(config);

        let mut attempt = 0;
        let result = retry.call(|| async {
            attempt += 1;
            if attempt < 3 {
                Err::<(), DbError>(DbError::Network("temporary failure".to_string()))
            } else {
                Ok(())
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(attempt, 3);
    }

    #[test]
    fn test_load_shedder_priority() {
        let config = LoadShedderConfig {
            queue_depth_threshold: 10,
            ..Default::default()
        };
        let shedder = LoadShedder::new(config);

        // Set high queue depth
        for _ in 0..15 {
            shedder.increment_queue_depth();
        }

        // Critical should always be admitted
        assert!(shedder.admit_request(RequestPriority::Critical));

        // Low priority should be shed
        assert!(!shedder.admit_request(RequestPriority::Low));
    }

    #[test]
    fn test_percentile_calculation() {
        let values = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        assert_eq!(percentile(&values, 0.5), 5);
        assert_eq!(percentile(&values, 0.95), 10);
        assert_eq!(percentile(&values, 0.99), 10);
    }
}


