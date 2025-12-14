// Circuit breaker pattern for fault tolerance.
//
// Prevents cascading failures by temporarily blocking requests to failing backends.
// Implements the classic three-state circuit breaker pattern:
// - Closed: Normal operation, requests pass through
// - Open: Too many failures, requests are blocked
// - Half-Open: Testing if backend has recovered

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed,
    /// Too many failures - requests are blocked
    Open,
    /// Testing recovery - limited requests allowed
    HalfOpen,
}

/// Circuit breaker for preventing cascading failures
pub struct CircuitBreaker {
    /// Current state
    state: Arc<RwLock<CircuitState>>,
    /// Failure count in current window
    failure_count: Arc<RwLock<u32>>,
    /// Success count in half-open state
    success_count: Arc<RwLock<u32>>,
    /// Threshold for opening circuit (uses interior mutability for adaptive adjustment)
    pub(crate) failure_threshold: Arc<RwLock<u32>>,
    /// Time to wait before trying half-open
    timeout: Duration,
    /// When the circuit was opened
    opened_at: Arc<RwLock<Option<Instant>>>,
    /// Number of successes needed in half-open to close
    success_threshold: u32,
    /// Maximum requests allowed in half-open state
    half_open_max_requests: u32,
    /// Current requests in half-open state
    half_open_requests: Arc<RwLock<u32>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    ///
    /// # Arguments
    /// * `failure_threshold` - Number of failures before opening circuit
    /// * `timeout` - Duration to wait before attempting recovery
    pub fn new(failure_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            failure_threshold: Arc::new(RwLock::new(failure_threshold)),
            timeout,
            opened_at: Arc::new(RwLock::new(None)),
            success_threshold: 3,      // Default: need 3 successes to close
            half_open_max_requests: 5, // Allow up to 5 concurrent requests in half-open
            half_open_requests: Arc::new(RwLock::new(0)),
        }
    }

    /// Create with custom success threshold
    pub fn with_success_threshold(mut self, threshold: u32) -> Self {
        self.success_threshold = threshold;
        self
    }

    /// Create with custom half-open max requests
    pub fn with_half_open_max_requests(mut self, max_requests: u32) -> Self {
        self.half_open_max_requests = max_requests;
        self
    }

    /// Check if a request can be attempted
    pub async fn can_attempt(&self) -> bool {
        let mut state = self.state.write().await;

        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                let opened_at = self.opened_at.read().await;
                if let Some(opened_time) = *opened_at {
                    if opened_time.elapsed() >= self.timeout {
                        // Transition to half-open
                        *state = CircuitState::HalfOpen;
                        let mut success_count = self.success_count.write().await;
                        *success_count = 0;
                        let mut half_open_requests = self.half_open_requests.write().await;
                        *half_open_requests = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited requests in half-open state
                let mut half_open_requests = self.half_open_requests.write().await;
                if *half_open_requests < self.half_open_max_requests {
                    *half_open_requests += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Record a successful request
    pub async fn record_success(&self) {
        let mut state = self.state.write().await;

        match *state {
            CircuitState::Closed => {
                // Reset failure count on success
                let mut failure_count = self.failure_count.write().await;
                *failure_count = 0;
            }
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;

                // Decrement half-open requests
                let mut half_open_requests = self.half_open_requests.write().await;
                if *half_open_requests > 0 {
                    *half_open_requests -= 1;
                }

                // Transition to closed if enough successes
                if *success_count >= self.success_threshold {
                    *state = CircuitState::Closed;
                    let mut failure_count = self.failure_count.write().await;
                    *failure_count = 0;
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
            }
        }
    }

    /// Record a failed request
    pub async fn record_failure(&self) {
        let mut state = self.state.write().await;

        match *state {
            CircuitState::Closed => {
                let mut failure_count = self.failure_count.write().await;
                *failure_count += 1;

                // Open circuit if threshold exceeded
                let threshold = *self.failure_threshold.read().await;
                if *failure_count >= threshold {
                    *state = CircuitState::Open;
                    let mut opened_at = self.opened_at.write().await;
                    *opened_at = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open immediately reopens circuit
                *state = CircuitState::Open;
                let mut opened_at = self.opened_at.write().await;
                *opened_at = Some(Instant::now());

                // Reset half-open requests
                let mut half_open_requests = self.half_open_requests.write().await;
                *half_open_requests = 0;
            }
            CircuitState::Open => {
                // Already open, no action needed
            }
        }
    }

    /// Get the current state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// Get failure count
    pub async fn failure_count(&self) -> u32 {
        *self.failure_count.read().await
    }

    /// Get success count (in half-open state)
    pub async fn success_count(&self) -> u32 {
        *self.success_count.read().await
    }

    /// Manually reset the circuit breaker
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed;

        let mut failure_count = self.failure_count.write().await;
        *failure_count = 0;

        let mut success_count = self.success_count.write().await;
        *success_count = 0;

        let mut opened_at = self.opened_at.write().await;
        *opened_at = None;

        let mut half_open_requests = self.half_open_requests.write().await;
        *half_open_requests = 0;
    }

    /// Force the circuit to open
    pub async fn force_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Open;

        let mut opened_at = self.opened_at.write().await;
        *opened_at = Some(Instant::now());
    }

    /// Check if circuit is open
    pub async fn is_open(&self) -> bool {
        let state = self.state.read().await;
        *state == CircuitState::Open
    }

    /// Check if circuit is half-open
    pub async fn is_half_open(&self) -> bool {
        let state = self.state.read().await;
        *state == CircuitState::HalfOpen
    }

    /// Check if circuit is closed
    pub async fn is_closed(&self) -> bool {
        let state = self.state.read().await;
        *state == CircuitState::Closed
    }
}

/// Adaptive circuit breaker that adjusts thresholds based on error rates
pub struct AdaptiveCircuitBreaker {
    /// Base circuit breaker
    base: CircuitBreaker,
    /// Window for calculating error rate
    window_size: Duration,
    /// Recent request results (true = success, false = failure)
    recent_results: Arc<RwLock<Vec<(Instant, bool)>>>,
    /// Target error rate (0.0 to 1.0)
    target_error_rate: f64,
}

impl AdaptiveCircuitBreaker {
    /// Create a new adaptive circuit breaker
    pub fn new(target_error_rate: f64, window_size: Duration, timeout: Duration) -> Self {
        // Start with a reasonable threshold
        let initial_threshold = 10;

        Self {
            base: CircuitBreaker::new(initial_threshold, timeout),
            window_size,
            recent_results: Arc::new(RwLock::new(Vec::new())),
            target_error_rate,
        }
    }

    /// Record a result and update thresholds
    pub async fn record_result(&self, success: bool) {
        // Record in base circuit breaker
        if success {
            self.base.record_success().await;
        } else {
            self.base.record_failure().await;
        }

        // Track in recent results
        let mut results = self.recent_results.write().await;
        results.push((Instant::now(), success));

        // Remove old results outside window
        let cutoff = Instant::now() - self.window_size;
        results.retain(|(timestamp, _)| *timestamp > cutoff);

        // Calculate current error rate
        let error_rate = if !results.is_empty() {
            let failures = results.iter().filter(|(_, success)| !success).count();
            Some(failures as f64 / results.len() as f64)
        } else {
            None
        };

        // Drop results lock before accessing failure_threshold
        drop(results);

        // Adjust threshold based on error rate
        if let Some(error_rate) = error_rate {
            let mut threshold = self.base.failure_threshold.write().await;
            // If error rate is above target, be more sensitive (lower threshold)
            // If error rate is below target, be less sensitive (higher threshold)
            if error_rate > self.target_error_rate * 1.5 {
                // High error rate - make more sensitive
                *threshold = (*threshold * 8 / 10).max(3);
            } else if error_rate < self.target_error_rate * 0.5 {
                // Low error rate - make less sensitive
                *threshold = (*threshold * 12 / 10).min(20);
            }
        }
    }

    /// Check if request can be attempted
    pub async fn can_attempt(&self) -> bool {
        self.base.can_attempt().await
    }

    /// Get current error rate
    pub async fn error_rate(&self) -> f64 {
        let results = self.recent_results.read().await;
        if results.is_empty() {
            return 0.0;
        }

        let failures = results.iter().filter(|(_, success)| !success).count();
        failures as f64 / results.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(5));

        assert!(cb.can_attempt().await);
        assert!(cb.is_closed().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(5));

        // Record failures
        cb.record_failure().await;
        assert!(cb.is_closed().await);

        cb.record_failure().await;
        assert!(cb.is_closed().await);

        cb.record_failure().await;
        assert!(cb.is_open().await);
        assert!(!cb.can_attempt().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(100));

        // Open the circuit
        cb.record_failure().await;
        cb.record_failure().await;
        assert!(cb.is_open().await);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should transition to half-open
        assert!(cb.can_attempt().await);
        assert!(cb.is_half_open().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(100)).with_success_threshold(2);

        // Open the circuit
        cb.record_failure().await;
        cb.record_failure().await;
        assert!(cb.is_open().await);

        // Wait and transition to half-open
        tokio::time::sleep(Duration::from_millis(150)).await;
        cb.can_attempt().await;
        assert!(cb.is_half_open().await);

        // Record successes to close
        cb.record_success().await;
        assert!(cb.is_half_open().await);

        cb.record_success().await;
        assert!(cb.is_closed().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reopens_on_failure() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(100));

        // Open the circuit
        cb.record_failure().await;
        cb.record_failure().await;

        // Wait and transition to half-open
        tokio::time::sleep(Duration::from_millis(150)).await;
        cb.can_attempt().await;
        assert!(cb.is_half_open().await);

        // Failure in half-open reopens circuit
        cb.record_failure().await;
        assert!(cb.is_open().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::new(2, Duration::from_secs(5));

        // Open the circuit
        cb.record_failure().await;
        cb.record_failure().await;
        assert!(cb.is_open().await);

        // Reset
        cb.reset().await;
        assert!(cb.is_closed().await);
        assert_eq!(cb.failure_count().await, 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_force_open() {
        let cb = CircuitBreaker::new(10, Duration::from_secs(5));

        assert!(cb.is_closed().await);

        cb.force_open().await;
        assert!(cb.is_open().await);
        assert!(!cb.can_attempt().await);
    }

    #[tokio::test]
    async fn test_success_resets_failure_count() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(5));

        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.failure_count().await, 2);

        cb.record_success().await;
        assert_eq!(cb.failure_count().await, 0);
        assert!(cb.is_closed().await);
    }

    #[tokio::test]
    async fn test_half_open_max_requests() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(100)).with_half_open_max_requests(2);

        // Open circuit
        cb.record_failure().await;
        cb.record_failure().await;

        // Wait for half-open
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should allow up to max requests
        assert!(cb.can_attempt().await);
        assert!(cb.can_attempt().await);
        assert!(!cb.can_attempt().await); // Third request blocked
    }

    #[tokio::test]
    async fn test_adaptive_circuit_breaker() {
        let cb = AdaptiveCircuitBreaker::new(0.1, Duration::from_secs(60), Duration::from_secs(5));

        // Record mostly successes
        for _ in 0..90 {
            cb.record_result(true).await;
        }
        for _ in 0..10 {
            cb.record_result(false).await;
        }

        let error_rate = cb.error_rate().await;
        assert!((error_rate - 0.1).abs() < 0.01); // Should be ~10%
    }
}
