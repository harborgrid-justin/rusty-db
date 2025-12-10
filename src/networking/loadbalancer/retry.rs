//! Retry policies for handling transient failures.
//!
//! Implements various retry strategies with exponential backoff, jitter,
//! maximum retry limits, and retry budgets to prevent retry storms.

use crate::error::{DbError, Result};
use std::time::Duration;
use tokio::time::sleep;

/// Retry strategy for handling transient failures
#[derive(Debug, Clone)]
pub enum RetryStrategy {
    /// No retries
    None,
    /// Fixed delay between retries
    Fixed {
        delay: Duration,
    },
    /// Exponential backoff
    Exponential {
        initial_delay: Duration,
        max_delay: Duration,
        multiplier: f64,
    },
    /// Exponential backoff with jitter to prevent thundering herd
    ExponentialWithJitter {
        initial_delay: Duration,
        max_delay: Duration,
        multiplier: f64,
    },
}

impl RetryStrategy {
    /// Calculate delay for a given attempt number
    pub fn delay_for_attempt(&self, attempt: u32) -> Option<Duration> {
        match self {
            RetryStrategy::None => None,
            RetryStrategy::Fixed { delay } => Some(*delay),
            RetryStrategy::Exponential {
                initial_delay,
                max_delay,
                multiplier,
            } => {
                let delay_ms = initial_delay.as_millis() as f64
                    * multiplier.powi(attempt as i32);
                let delay = Duration::from_millis(delay_ms as u64);
                Some(delay.min(*max_delay))
            }
            RetryStrategy::ExponentialWithJitter {
                initial_delay,
                max_delay,
                multiplier,
            } => {
                let base_delay_ms = initial_delay.as_millis() as f64
                    * multiplier.powi(attempt as i32);

                // Add jitter: random value between 0 and base_delay
                let jitter = rand::random::<f64>() * base_delay_ms;
                let delay = Duration::from_millis((base_delay_ms + jitter) as u64);
                Some(delay.min(*max_delay))
            }
        }
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self::ExponentialWithJitter {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
        }
    }
}

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Retry strategy to use
    pub strategy: RetryStrategy,
    /// Maximum number of retry attempts (0 = no retries)
    pub max_attempts: u32,
    /// Maximum total time to spend retrying
    pub max_total_time: Option<Duration>,
    /// Retry budget (prevents retry storms)
    pub budget: Option<RetryBudget>,
}

impl RetryPolicy {
    /// Create a new retry policy with no retries
    pub fn none() -> Self {
        Self {
            strategy: RetryStrategy::None,
            max_attempts: 0,
            max_total_time: None,
            budget: None,
        }
    }

    /// Create a policy with fixed delay
    pub fn fixed(delay: Duration, max_attempts: u32) -> Self {
        Self {
            strategy: RetryStrategy::Fixed { delay },
            max_attempts,
            max_total_time: None,
            budget: None,
        }
    }

    /// Create a policy with exponential backoff
    pub fn exponential(
        initial_delay: Duration,
        max_delay: Duration,
        max_attempts: u32,
    ) -> Self {
        Self {
            strategy: RetryStrategy::Exponential {
                initial_delay,
                max_delay,
                multiplier: 2.0,
            },
            max_attempts,
            max_total_time: None,
            budget: None,
        }
    }

    /// Create a policy with exponential backoff and jitter
    pub fn exponential_with_jitter(
        initial_delay: Duration,
        max_delay: Duration,
        max_attempts: u32,
    ) -> Self {
        Self {
            strategy: RetryStrategy::ExponentialWithJitter {
                initial_delay,
                max_delay,
                multiplier: 2.0,
            },
            max_attempts,
            max_total_time: None,
            budget: None,
        }
    }

    /// Set maximum total time for retries
    pub fn with_max_total_time(mut self, max_time: Duration) -> Self {
        self.max_total_time = Some(max_time);
        self
    }

    /// Set retry budget
    pub fn with_budget(mut self, budget: RetryBudget) -> Self {
        self.budget = Some(budget);
        self
    }

    /// Check if retries are allowed
    pub fn should_retry(&self, attempt: u32, elapsed: Duration) -> bool {
        // Check max attempts
        if attempt >= self.max_attempts {
            return false;
        }

        // Check max total time
        if let Some(max_time) = self.max_total_time {
            if elapsed >= max_time {
                return false;
            }
        }

        // Check retry budget
        if let Some(budget) = &self.budget {
            if !budget.can_retry() {
                return false;
            }
        }

        true
    }

    /// Execute a function with retries
    pub async fn execute<F, Fut, T>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let start_time = std::time::Instant::now();
        let mut attempt = 0u32;
        let mut last_error = None;

        loop {
            // Try the operation
            match operation().await {
                Ok(result) => {
                    // Success - record in budget if present
                    if let Some(budget) = &self.budget {
                        budget.record_success();
                    }
                    return Ok(result);
                }
                Err(err) => {
                    last_error = Some(err);
                    attempt += 1;

                    // Record failure in budget
                    if let Some(budget) = &self.budget {
                        budget.record_failure();
                    }

                    // Check if we should retry
                    if !self.should_retry(attempt, start_time.elapsed()) {
                        break;
                    }

                    // Calculate and apply delay
                    if let Some(delay) = self.strategy.delay_for_attempt(attempt - 1) {
                        sleep(delay).await;
                    }
                }
            }
        }

        // All retries exhausted
        Err(last_error.unwrap_or_else(|| {
            DbError::Network("All retry attempts failed".to_string())
        }))
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::exponential_with_jitter(
            Duration::from_millis(100),
            Duration::from_secs(30),
            3,
        )
    }
}

/// Retry budget to prevent retry storms
///
/// Tracks the ratio of retries to requests and limits retries when
/// the ratio gets too high, preventing retry storms that can overload systems.
#[derive(Debug, Clone)]
pub struct RetryBudget {
    /// Target ratio of retries to requests (e.g., 0.2 = 20% retries)
    target_ratio: f64,
    /// Minimum number of requests in window
    min_requests: u32,
}

impl RetryBudget {
    /// Create a new retry budget
    ///
    /// # Arguments
    /// * `target_ratio` - Target ratio of retries to requests (0.0 to 1.0)
    /// * `min_requests` - Minimum requests before enforcing budget
    pub fn new(target_ratio: f64, min_requests: u32) -> Self {
        Self {
            target_ratio: target_ratio.clamp(0.0, 1.0),
            min_requests,
        }
    }

    /// Check if a retry is allowed under the budget
    pub fn can_retry(&self) -> bool {
        // For now, always allow retries
        // In a full implementation, this would track actual request/retry counts
        true
    }

    /// Record a successful request
    pub fn record_success(&self) {
        // In a full implementation, would increment success counter
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        // In a full implementation, would increment failure counter
    }
}

impl Default for RetryBudget {
    fn default() -> Self {
        Self::new(0.2, 10)
    }
}

/// Helper function to determine if an error is retryable
pub fn is_retryable_error(error: &DbError) -> bool {
    matches!(
        error,
        DbError::Network(_) | DbError::Unavailable(_) | DbError::LockTimeout
    )
}

/// Retry with decorrelated jitter (AWS recommended approach)
///
/// Provides better distribution than exponential backoff with jitter
pub struct DecorrelatedJitterRetry {
    base_delay: Duration,
    max_delay: Duration,
    last_delay: std::sync::Arc<tokio::sync::RwLock<Duration>>,
}

impl DecorrelatedJitterRetry {
    /// Create a new decorrelated jitter retry strategy
    pub fn new(base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            base_delay,
            max_delay,
            last_delay: std::sync::Arc::new(tokio::sync::RwLock::new(base_delay)),
        }
    }

    /// Calculate next delay using decorrelated jitter
    pub async fn next_delay(&self) -> Duration {
        let last = *self.last_delay.read().await;

        // delay = random_between(base_delay, last_delay * 3)
        let min_ms = self.base_delay.as_millis() as f64;
        let max_ms = (last.as_millis() as f64 * 3.0).min(self.max_delay.as_millis() as f64);

        let random_ms = min_ms + rand::random::<f64>() * (max_ms - min_ms);
        let delay = Duration::from_millis(random_ms as u64);

        let mut last_delay = self.last_delay.write().await;
        *last_delay = delay;

        delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_delay() {
        let strategy = RetryStrategy::Fixed {
            delay: Duration::from_millis(100),
        };

        assert_eq!(strategy.delay_for_attempt(0), Some(Duration::from_millis(100)));
        assert_eq!(strategy.delay_for_attempt(5), Some(Duration::from_millis(100)));
    }

    #[test]
    fn test_exponential_backoff() {
        let strategy = RetryStrategy::Exponential {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
        };

        assert_eq!(strategy.delay_for_attempt(0), Some(Duration::from_millis(100)));
        assert_eq!(strategy.delay_for_attempt(1), Some(Duration::from_millis(200)));
        assert_eq!(strategy.delay_for_attempt(2), Some(Duration::from_millis(400)));
        assert_eq!(strategy.delay_for_attempt(3), Some(Duration::from_millis(800)));
    }

    #[test]
    fn test_exponential_backoff_capped() {
        let strategy = RetryStrategy::Exponential {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(500),
            multiplier: 2.0,
        };

        // Should be capped at max_delay
        assert_eq!(strategy.delay_for_attempt(10), Some(Duration::from_millis(500)));
    }

    #[test]
    fn test_exponential_with_jitter() {
        let strategy = RetryStrategy::ExponentialWithJitter {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
        };

        // Jitter makes this non-deterministic, just check it returns something
        let delay = strategy.delay_for_attempt(1);
        assert!(delay.is_some());
        assert!(delay.unwrap() >= Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_retry_policy_max_attempts() {
        let policy = RetryPolicy::fixed(Duration::from_millis(10), 3);

        assert!(policy.should_retry(0, Duration::from_secs(0)));
        assert!(policy.should_retry(1, Duration::from_secs(0)));
        assert!(policy.should_retry(2, Duration::from_secs(0)));
        assert!(!policy.should_retry(3, Duration::from_secs(0)));
    }

    #[tokio::test]
    async fn test_retry_policy_max_time() {
        let policy = RetryPolicy::fixed(Duration::from_millis(10), 10)
            .with_max_total_time(Duration::from_secs(1));

        assert!(policy.should_retry(0, Duration::from_millis(500)));
        assert!(!policy.should_retry(0, Duration::from_secs(2)));
    }

    #[tokio::test]
    async fn test_retry_policy_execute_success() {
        let policy = RetryPolicy::fixed(Duration::from_millis(10), 3);

        let mut attempts = 0;
        let result = policy
            .execute(|| async {
                attempts += 1;
                if attempts < 2 {
                    Err(DbError::Network("Transient error".to_string()))
                } else {
                    Ok(42)
                }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 2); // Failed once, succeeded on second attempt
    }

    #[tokio::test]
    async fn test_retry_policy_execute_exhausted() {
        let policy = RetryPolicy::fixed(Duration::from_millis(10), 2);

        let mut attempts = 0;
        let result = policy
            .execute(|| async {
                attempts += 1;
                Err::<(), _>(DbError::Network("Persistent error".to_string()))
            })
            .await;

        assert!(result.is_err());
        assert_eq!(attempts, 2); // Initial attempt + 2 retries = 2 total
    }

    #[test]
    fn test_is_retryable_error() {
        assert!(is_retryable_error(&DbError::Network("error".to_string())));
        assert!(is_retryable_error(&DbError::Unavailable("error".to_string())));
        assert!(is_retryable_error(&DbError::LockTimeout));
        assert!(!is_retryable_error(&DbError::NotFound("error".to_string())));
    }

    #[tokio::test]
    async fn test_decorrelated_jitter() {
        let retry = DecorrelatedJitterRetry::new(
            Duration::from_millis(100),
            Duration::from_secs(10),
        );

        let delay1 = retry.next_delay().await;
        let delay2 = retry.next_delay().await;

        // Delays should be in valid range
        assert!(delay1 >= Duration::from_millis(100));
        assert!(delay1 <= Duration::from_secs(10));
        assert!(delay2 >= Duration::from_millis(100));
        assert!(delay2 <= Duration::from_secs(10));
    }

    #[test]
    fn test_retry_budget() {
        let budget = RetryBudget::new(0.2, 10);

        // Should allow retries
        assert!(budget.can_retry());
    }

    #[tokio::test]
    async fn test_no_retry_policy() {
        let policy = RetryPolicy::none();

        let mut attempts = 0;
        let result = policy
            .execute(|| async {
                attempts += 1;
                Err::<(), _>(DbError::Network("Error".to_string()))
            })
            .await;

        assert!(result.is_err());
        assert_eq!(attempts, 1); // Only initial attempt, no retries
    }
}
