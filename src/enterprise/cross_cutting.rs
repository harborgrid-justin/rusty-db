// # Cross-Cutting Concerns
//
// Provides unified infrastructure for distributed tracing, error handling and recovery,
// circuit breakers, rate limiting, and request context propagation across all subsystems.
//
// ## Features
//
// - **Distributed Tracing**: Correlation IDs and span tracking across service boundaries
// - **Error Handling**: Unified error handling with automatic recovery strategies
// - **Circuit Breaker**: Prevent cascading failures in distributed systems
// - **Rate Limiting**: Token bucket and sliding window rate limiting
// - **Request Context**: Propagate request metadata across async boundaries
// - **Retry Logic**: Configurable retry with exponential backoff
// - **Bulkhead Pattern**: Resource isolation to prevent resource exhaustion
//
// ## Example
//
// ```rust,no_run
// use rusty_db::enterprise::cross_cutting::{
//     RequestContext, CircuitBreaker, RateLimiter, TracingContext
// };
//
// #[tokio::main]
// async fn main() {
//     // Create request context
//     let ctx = RequestContext::new()
//         .with_user("user123")
//         .with_trace_id("trace-xyz");
//
//     // Use circuit breaker
//     let breaker = CircuitBreaker::new("external_api", 5, 60);
//     let result = breaker.call(async {
//         // Make external call
//         Ok::<_, rusty_db::DbError>(42)
//     }).await;
//
//     // Rate limiting
//     let limiter = RateLimiter::new(100, 60); // 100 req/min
//     if limiter.allow("user123").await {
//         // Process request
//     }
// }
// ```

use std::fmt;
use std::time::Instant;
use std::sync::Mutex;
use std::time::SystemTime;
use std::collections::{HashMap};
use std::sync::Arc;
use std::time::{Duration};
use std::future::Future;
use tokio::sync::{RwLock, Semaphore};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::{Result, DbError};

// ============================================================================
// Distributed Tracing
// ============================================================================

// Tracing span representing a unit of work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    // Unique span identifier
    pub span_id: String,
    // Parent span ID (if any)
    pub parent_span_id: Option<String>,
    // Trace ID for correlation
    pub trace_id: String,
    // Span name/operation
    pub name: String,
    // Start time
    pub start_time: SystemTime,
    // End time (if completed)
    pub end_time: Option<SystemTime>,
    // Span tags/attributes
    pub tags: HashMap<String, String>,
    // Span events/logs
    pub events: Vec<SpanEvent>,
}

// Event within a span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    // Event name
    pub name: String,
    // Event timestamp
    pub timestamp: SystemTime,
    // Event attributes
    pub attributes: HashMap<String, String>,
}

impl Span {
    // Create a new root span
    pub fn new(name: impl Into<String>) -> Self {
        let trace_id = Uuid::new_v4().to_string();
        Self {
            span_id: Uuid::new_v4().to_string(),
            parent_span_id: None,
            trace_id,
            name: name.into(),
            start_time: SystemTime::now(),
            end_time: None,
            tags: HashMap::new(),
            events: Vec::new(),
        }
    }

    // Create a child span
    pub fn child(&self, name: impl Into<String>) -> Self {
        Self {
            span_id: Uuid::new_v4().to_string(),
            parent_span_id: Some(self.span_id.clone()),
            trace_id: self.trace_id.clone(),
            name: name.into(),
            start_time: SystemTime::now(),
            end_time: None,
            tags: HashMap::new(),
            events: Vec::new(),
        }
    }

    // Add a tag to the span
    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    // Add an event to the span
    pub fn event(&mut self, name: impl Into<String>, attributes: HashMap<String, String>) {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp: SystemTime::now(),
            attributes,
        });
    }

    // Complete the span
    pub fn finish(&mut self) {
        self.end_time = Some(SystemTime::now());
    }

    // Get span duration
    pub fn duration(&self) -> Option<Duration> {
        if let Some(end) = self.end_time {
            end.duration_since(self.start_time).ok()
        } else {
            None
        }
    }
}

// Tracing context for distributed tracing
#[derive(Debug, Clone)]
pub struct TracingContext {
    // Current span stack
    spans: Arc<RwLock<Vec<Span>>>,
    // Completed spans
    completed: Arc<RwLock<Vec<Span>>>,
}

impl TracingContext {
    // Create a new tracing context
    pub fn new() -> Self {
        Self {
            spans: Arc::new(RwLock::new(Vec::new())),
            completed: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // Start a new span
    pub async fn start_span(&self, name: impl Into<String>) -> String {
        let mut spans = self.spans.write().await;

        let span = if let Some(parent) = spans.last() {
            parent.child(name)
        } else {
            Span::new(name)
        };

        let span_id = span.span_id.clone();
        spans.push(span);
        span_id
    }

    // End the current span
    pub async fn end_span(&self) {
        let mut spans = self.spans.write().await;
        if let Some(mut span) = spans.pop() {
            span.finish();
            let mut completed = self.completed.write().await;
            completed.push(span);
        }
    }

    // Add a tag to current span
    pub async fn tag(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut spans = self.spans.write().await;
        if let Some(span) = spans.last_mut() {
            span.tags.insert(key.into(), value.into());
        }
    }

    // Add an event to current span
    pub async fn event(&self, name: impl Into<String>, attributes: HashMap<String, String>) {
        let mut spans = self.spans.write().await;
        if let Some(span) = spans.last_mut() {
            span.event(name, attributes);
        }
    }

    // Get current trace ID
    pub async fn trace_id(&self) -> Option<String> {
        let spans = self.spans.read().await;
        spans.first().map(|s| s.trace_id.clone())
    }

    // Get all completed spans
    pub async fn get_completed_spans(&self) -> Vec<Span> {
        let completed = self.completed.read().await;
        completed.clone()
    }
}

impl Default for TracingContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Request Context
// ============================================================================

// Request context for propagating metadata
#[derive(Debug, Clone)]
pub struct RequestContext {
    // Request ID
    pub request_id: String,
    // Trace ID
    pub trace_id: String,
    // User ID (if authenticated)
    pub user_id: Option<String>,
    // Session ID
    pub session_id: Option<String>,
    // Request timestamp
    pub timestamp: SystemTime,
    // Custom metadata
    pub metadata: HashMap<String, String>,
    // Tracing context
    pub tracing: TracingContext,
}

impl RequestContext {
    // Create a new request context
    pub fn new() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            trace_id: Uuid::new_v4().to_string(),
            user_id: None,
            session_id: None,
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
            tracing: TracingContext::new(),
        }
    }

    // Set user ID
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    // Set trace ID
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = trace_id.into();
        self
    }

    // Set session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    // Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Circuit Breaker
// ============================================================================

// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    // Circuit is closed, requests pass through
    Closed,
    // Circuit is open, requests are rejected
    Open,
    // Circuit is half-open, testing if service recovered
    HalfOpen,
}

// Circuit breaker implementation
pub struct CircuitBreaker {
    // Circuit name
    name: String,
    // Current state
    state: Arc<RwLock<CircuitState>>,
    // Failure count
    failure_count: Arc<Mutex<u32>>,
    // Failure threshold before opening
    failure_threshold: u32,
    // Timeout before attempting recovery (seconds)
    timeout: u64,
    // Last failure time
    last_failure: Arc<Mutex<Option<Instant>>>,
    // Success count in half-open state
    success_count: Arc<Mutex<u32>>,
    // Required successes to close circuit
    success_threshold: u32,
}

impl CircuitBreaker {
    // Create a new circuit breaker
    pub fn new(name: impl Into<String>, failure_threshold: u32, timeout_secs: u64) -> Self {
        Self {
            name: name.into(),
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(Mutex::new(0)),
            failure_threshold,
            timeout: timeout_secs,
            last_failure: Arc::new(Mutex::new(None)),
            success_count: Arc::new(Mutex::new(0)),
            success_threshold: 3,
        }
    }

    // Execute a function through the circuit breaker
    pub async fn call<F, T, E>(&self, f: F) -> Result<T>
    where
        F: Future<Output = std::result::Result<T, E>>,
        E: fmt::Display,
    {
        // Check if we should attempt the call
        let should_attempt = self.should_attempt().await;

        if !should_attempt {
            return Err(DbError::Internal(format!(
                "Circuit breaker '{}' is open",
                self.name
            )));
        }

        // Execute the function
        match f.await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(DbError::Internal(format!("Circuit breaker call failed: {}", e)))
            }
        }
    }

    // Check if we should attempt the call
    async fn should_attempt(&self) -> bool {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                let last_failure = self.last_failure.lock().unwrap();
                if let Some(last) = *last_failure {
                    if last.elapsed().as_secs() >= self.timeout {
                        // Transition to half-open
                        drop(last_failure);
                        let mut state = self.state.write().await;
                        *state = CircuitState::HalfOpen;
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    // Handle successful call
    async fn on_success(&self) {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => {
                // Reset failure count
                let mut count = self.failure_count.lock().unwrap();
                *count = 0;
            }
            CircuitState::HalfOpen => {
                // Increment success count
                let mut success = self.success_count.lock().unwrap();
                *success += 1;

                if *success >= self.success_threshold {
                    // Close the circuit
                    drop(success);
                    let mut state = self.state.write().await;
                    *state = CircuitState::Closed;

                    let mut failure_count = self.failure_count.lock().unwrap();
                    *failure_count = 0;

                    let mut success_count = self.success_count.lock().unwrap();
                    *success_count = 0;
                }
            }
            CircuitState::Open => {}
        }
    }

    // Handle failed call
    async fn on_failure(&self) {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => {
                let mut count = self.failure_count.lock().unwrap();
                *count += 1;

                if *count >= self.failure_threshold {
                    // Open the circuit
                    drop(count);
                    let mut state = self.state.write().await;
                    *state = CircuitState::Open;

                    let mut last_failure = self.last_failure.lock().unwrap();
                    *last_failure = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                // Failed during testing, reopen circuit
                let mut state = self.state.write().await;
                *state = CircuitState::Open;

                let mut last_failure = self.last_failure.lock().unwrap();
                *last_failure = Some(Instant::now());

                let mut success_count = self.success_count.lock().unwrap();
                *success_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    // Get current state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }

    // Reset circuit breaker
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed;

        let mut failure_count = self.failure_count.lock().unwrap();
        *failure_count = 0;

        let mut success_count = self.success_count.lock().unwrap();
        *success_count = 0;
    }
}

// ============================================================================
// Rate Limiter
// ============================================================================

// Token bucket for rate limiting
struct TokenBucket {
    // Maximum tokens
    capacity: u32,
    // Current tokens
    tokens: f64,
    // Refill rate (tokens per second)
    refill_rate: f64,
    // Last refill time
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: u32, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity as f64);
        self.last_refill = now;
    }

    fn consume(&mut self, tokens: u32) -> bool {
        self.refill();

        if self.tokens >= tokens as f64 {
            self.tokens -= tokens as f64;
            true
        } else {
            false
        }
    }
}

// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    // Per-key token buckets
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
    // Bucket capacity
    capacity: u32,
    // Refill rate (tokens per second)
    refill_rate: f64,
}

impl RateLimiter {
    // Create a new rate limiter
    // - capacity: Maximum requests
    // - window_secs: Time window in seconds
    pub fn new(capacity: u32, window_secs: u64) -> Self {
        let refill_rate = capacity as f64 / window_secs as f64;

        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            capacity,
            refill_rate,
        }
    }

    // Check if a request is allowed
    pub async fn allow(&self, key: impl Into<String>) -> bool {
        self.allow_n(key, 1).await
    }

    // Check if N requests are allowed
    pub async fn allow_n(&self, key: impl Into<String>, n: u32) -> bool {
        let key = key.into();
        let mut buckets = self.buckets.lock().unwrap();

        let bucket = buckets
            .entry(key)
            .or_insert_with(|| TokenBucket::new(self.capacity, self.refill_rate));

        bucket.consume(n)
    }

    // Reset rate limit for a key
    pub async fn reset(&self, key: &str) {
        let mut buckets = self.buckets.lock().unwrap();
        buckets.remove(key);
    }
}

// ============================================================================
// Retry Logic
// ============================================================================

// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    // Maximum retry attempts
    pub max_attempts: u32,
    // Initial delay
    pub initial_delay: Duration,
    // Maximum delay
    pub max_delay: Duration,
    // Backoff multiplier
    pub multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
        }
    }
}

// Execute a function with retry logic
pub async fn retry_with_backoff<F, Fut, T, E>(
    policy: &RetryPolicy,
    mut f: F,
) -> std::result::Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = std::result::Result<T, E>>,
{
    let mut attempts = 0;
    let mut delay = policy.initial_delay;

    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempts += 1;
                if attempts >= policy.max_attempts {
                    return Err(e);
                }

                tokio::time::sleep(delay).await;

                delay = Duration::from_secs_f64(
                    (delay.as_secs_f64() * policy.multiplier).min(policy.max_delay.as_secs_f64())
                );
            }
        }
    }
}

// ============================================================================
// Bulkhead Pattern
// ============================================================================

// Bulkhead for resource isolation
#[allow(dead_code)]
pub struct Bulkhead {
    // Semaphore for limiting concurrent operations
    semaphore: Arc<Semaphore>,
    // Maximum concurrent operations
    max_concurrent: usize,
}

impl Bulkhead {
    // Create a new bulkhead
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    // Execute a function with bulkhead protection
    pub async fn execute<F, T>(&self, f: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        let _permit = self.semaphore.acquire().await
            .map_err(|e| DbError::Internal(format!("Bulkhead acquire failed: {}", e)))?;

        f.await
    }

    // Get available permits
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
}

// ============================================================================
// Error Recovery
// ============================================================================

// Recovery strategy for errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    // Retry the operation
    Retry,
    // Fail fast and propagate error
    FailFast,
    // Use fallback value/behavior
    Fallback,
    // Ignore the error
    Ignore,
}

// Error handler with recovery strategies
pub struct ErrorHandler {
    // Retry policy
    retry_policy: RetryPolicy,
}

impl ErrorHandler {
    // Create a new error handler
    pub fn new(retry_policy: RetryPolicy) -> Self {
        Self { retry_policy }
    }

    // Handle an error with the given strategy
    pub async fn handle<F, Fut, T>(
        &self,
        strategy: RecoveryStrategy,
        mut operation: F,
        fallback: Option<T>,
    ) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T>>,
        T: Clone,
    {
        match strategy {
            RecoveryStrategy::Retry => {
                retry_with_backoff(&self.retry_policy, || operation()).await
            }
            RecoveryStrategy::FailFast => operation().await,
            RecoveryStrategy::Fallback => {
                match operation().await {
                    Ok(v) => Ok(v),
                    Err(_) => {
                        fallback.ok_or_else(|| {
                            DbError::Internal("No fallback value provided".to_string())
                        })
                    }
                }
            }
            RecoveryStrategy::Ignore => {
                match operation().await {
                    Ok(v) => Ok(v),
                    Err(_) => {
                        fallback.ok_or_else(|| {
                            DbError::Internal("No fallback value for ignored error".to_string())
                        })
                    }
                }
            }
        }
    }
}

impl Default for ErrorHandler {
    fn default() -> Self {
        Self::new(RetryPolicy::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new("test", 2, 1);

        // Should be closed initially
        assert_eq!(breaker.state().await, CircuitState::Closed);

        // Simulate failures
        let _ = breaker.call(async { Err::<(), _>("error") }).await;
        let _ = breaker.call(async { Err::<(), _>("error") }).await;

        // Should be open now
        assert_eq!(breaker.state().await, CircuitState::Open);

        // Should reject requests
        assert!(breaker.call(async { Ok::<_, String>(42) }).await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(2, 60);

        // First two should succeed
        assert!(limiter.allow("user1").await);
        assert!(limiter.allow("user1").await);

        // Third should fail
        assert!(!limiter.allow("user1").await);

        // Different user should succeed
        assert!(limiter.allow("user2").await);
    }

    #[tokio::test]
    async fn test_tracing_context() {
        let ctx = TracingContext::new();

        let span1 = ctx.start_span("operation1").await;
        ctx.tag("key", "value").await;

        let span2 = ctx.start_span("operation2").await;
        ctx.event("checkpoint", HashMap::new()).await;

        ctx.end_span().await; // End operation2
        ctx.end_span().await; // End operation1

        let completed = ctx.get_completed_spans().await;
        assert_eq!(completed.len(), 2);
    }

    #[tokio::test]
    async fn test_bulkhead() {
        let bulkhead = Bulkhead::new(2);

        assert_eq!(bulkhead.available_permits(), 2);

        let result = bulkhead.execute(async {
            Ok::<_, DbError>(42)
        }).await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(bulkhead.available_permits(), 2);
    }
}
