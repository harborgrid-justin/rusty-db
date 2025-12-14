// Load balancing and traffic management for RustyDB distributed clusters.
//
// This module provides enterprise-grade load balancing with multiple strategies,
// traffic shaping, circuit breakers, and retry policies for optimal cluster performance.

use crate::error::{DbError, Result};
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub mod circuit_breaker;
pub mod retry;
pub mod strategies;
pub mod traffic_shaping;

pub use circuit_breaker::{CircuitBreaker, CircuitState};
pub use retry::{RetryBudget, RetryPolicy, RetryStrategy};
pub use strategies::{
    AdaptiveBalancer, ConsistentHashBalancer, LeastConnectionsBalancer, LoadBalancingStrategy,
    RoundRobinBalancer,
};
pub use traffic_shaping::{RateLimiter, TrafficShaper};

/// Unique identifier for a backend node
pub type NodeId = String;

/// Backend server in the load balancer pool
#[derive(Debug, Clone)]
pub struct Backend {
    /// Unique identifier for this backend
    pub id: NodeId,
    /// Network address of the backend
    pub address: SocketAddr,
    /// Weight for weighted load balancing (higher = more traffic)
    pub weight: u32,
    /// Current health status
    pub healthy: bool,
    /// Number of active connections
    pub active_connections: u32,
    /// Last health check timestamp
    pub last_health_check: Option<Instant>,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,
    /// Current throughput (requests per second)
    pub throughput: f64,
}

impl Backend {
    /// Create a new backend with default values
    pub fn new(id: NodeId, address: SocketAddr) -> Self {
        Self {
            id,
            address,
            weight: 100,
            healthy: true,
            active_connections: 0,
            last_health_check: None,
            avg_response_time_ms: 0.0,
            error_rate: 0.0,
            throughput: 0.0,
        }
    }

    /// Create a weighted backend
    pub fn with_weight(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }

    /// Mark backend as healthy or unhealthy
    pub fn set_healthy(&mut self, healthy: bool) {
        self.healthy = healthy;
        self.last_health_check = Some(Instant::now());
    }

    /// Increment active connection count
    pub fn increment_connections(&mut self) {
        self.active_connections += 1;
    }

    /// Decrement active connection count
    pub fn decrement_connections(&mut self) {
        if self.active_connections > 0 {
            self.active_connections -= 1;
        }
    }

    /// Update performance metrics
    pub fn update_metrics(&mut self, response_time_ms: f64, success: bool) {
        // Exponential moving average for response time
        let alpha = 0.2;
        self.avg_response_time_ms =
            alpha * response_time_ms + (1.0 - alpha) * self.avg_response_time_ms;

        // Update error rate
        if success {
            self.error_rate *= 0.95; // Decay error rate on success
        } else {
            self.error_rate = 0.05 + self.error_rate * 0.95; // Increase on failure
        }
    }

    /// Calculate backend load score (lower is better)
    pub fn load_score(&self) -> f64 {
        if !self.healthy {
            return f64::MAX;
        }

        // Combine multiple factors into a load score
        let connection_factor = self.active_connections as f64;
        let latency_factor = self.avg_response_time_ms / 100.0; // Normalize
        let error_factor = self.error_rate * 1000.0; // Weight errors heavily
        let weight_factor = 100.0 / self.weight as f64; // Inverse of weight

        connection_factor + latency_factor + error_factor + weight_factor
    }
}

/// Context for a load balancing decision
#[derive(Debug, Clone)]
pub struct LoadBalancerContext {
    /// Optional key for consistent hashing
    pub key: Option<String>,
    /// Request priority (higher = more important)
    pub priority: u8,
    /// Client identifier
    pub client_id: Option<String>,
    /// Request metadata
    pub metadata: Vec<(String, String)>,
}

impl Default for LoadBalancerContext {
    fn default() -> Self {
        Self {
            key: None,
            priority: 0,
            client_id: None,
            metadata: Vec::new(),
        }
    }
}

impl LoadBalancerContext {
    /// Create a new context with a routing key
    pub fn with_key(key: impl Into<String>) -> Self {
        Self {
            key: Some(key.into()),
            ..Default::default()
        }
    }

    /// Set priority for this request
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set client ID
    pub fn with_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    /// Add metadata
    pub fn add_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.push((key, value));
        self
    }
}

/// Main load balancer that manages backend selection and traffic shaping
pub struct LoadBalancer {
    /// Backend servers
    backends: Arc<RwLock<Vec<Backend>>>,
    /// Load balancing strategy
    strategy: Arc<dyn LoadBalancingStrategy>,
    /// Traffic shaper for rate limiting and prioritization
    traffic_shaper: Arc<TrafficShaper>,
    /// Circuit breaker for fault tolerance
    circuit_breaker: Arc<RwLock<CircuitBreaker>>,
    /// Retry policy
    retry_policy: RetryPolicy,
}

impl LoadBalancer {
    /// Create a new load balancer with the specified strategy
    pub fn new(strategy: Arc<dyn LoadBalancingStrategy>) -> Self {
        Self {
            backends: Arc::new(RwLock::new(Vec::new())),
            strategy,
            traffic_shaper: Arc::new(TrafficShaper::new()),
            circuit_breaker: Arc::new(RwLock::new(CircuitBreaker::new(5, Duration::from_secs(30)))),
            retry_policy: RetryPolicy::default(),
        }
    }

    /// Set the traffic shaper
    pub fn with_traffic_shaper(mut self, shaper: TrafficShaper) -> Self {
        self.traffic_shaper = Arc::new(shaper);
        self
    }

    /// Set the retry policy
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Add a backend to the pool
    pub async fn add_backend(&self, backend: Backend) -> Result<()> {
        let mut backends = self.backends.write().await;
        backends.push(backend);
        Ok(())
    }

    /// Remove a backend from the pool
    pub async fn remove_backend(&self, node_id: &str) -> Result<()> {
        let mut backends = self.backends.write().await;
        backends.retain(|b| b.id != node_id);
        Ok(())
    }

    /// Update backend health status
    pub async fn update_backend_health(&self, node_id: &str, healthy: bool) -> Result<()> {
        let mut backends = self.backends.write().await;
        if let Some(backend) = backends.iter_mut().find(|b| b.id == node_id) {
            backend.set_healthy(healthy);
            Ok(())
        } else {
            Err(DbError::NotFound(format!("Backend {} not found", node_id)))
        }
    }

    /// Select a backend for the request
    pub async fn select_backend(&self, context: &LoadBalancerContext) -> Result<Backend> {
        // Check circuit breaker
        let circuit_breaker = self.circuit_breaker.read().await;
        if !circuit_breaker.can_attempt().await {
            return Err(DbError::Unavailable("Circuit breaker is open".to_string()));
        }
        drop(circuit_breaker);

        // Apply traffic shaping
        self.traffic_shaper
            .check_rate_limit(&context.client_id.clone().unwrap_or_default())
            .await?;

        // Get healthy backends
        let backends = self.backends.read().await;
        let healthy_backends: Vec<_> = backends.iter().filter(|b| b.healthy).cloned().collect();

        if healthy_backends.is_empty() {
            return Err(DbError::Unavailable(
                "No healthy backends available".to_string(),
            ));
        }

        // Use strategy to select backend
        self.strategy.select(&healthy_backends, context).await
    }

    /// Record the result of a request to update metrics and circuit breaker
    pub async fn record_result(
        &self,
        node_id: &str,
        response_time: Duration,
        success: bool,
    ) -> Result<()> {
        // Update backend metrics
        let mut backends = self.backends.write().await;
        if let Some(backend) = backends.iter_mut().find(|b| b.id == node_id) {
            backend.update_metrics(response_time.as_millis() as f64, success);
        }
        drop(backends);

        // Update circuit breaker
        let circuit_breaker = self.circuit_breaker.write().await;
        if success {
            circuit_breaker.record_success().await;
        } else {
            circuit_breaker.record_failure().await;
        }

        Ok(())
    }

    /// Get statistics about all backends
    pub async fn get_statistics(&self) -> BackendStatistics {
        let backends = self.backends.read().await;
        let total = backends.len();
        let healthy = backends.iter().filter(|b| b.healthy).count();
        let total_connections: u32 = backends.iter().map(|b| b.active_connections).sum();
        let avg_response_time: f64 =
            backends.iter().map(|b| b.avg_response_time_ms).sum::<f64>() / total as f64;

        BackendStatistics {
            total_backends: total,
            healthy_backends: healthy,
            total_connections,
            avg_response_time_ms: avg_response_time,
        }
    }

    /// Get the retry policy
    pub fn retry_policy(&self) -> &RetryPolicy {
        &self.retry_policy
    }
}

/// Statistics about backend pool
#[derive(Debug, Clone)]
pub struct BackendStatistics {
    pub total_backends: usize,
    pub healthy_backends: usize,
    pub total_connections: u32,
    pub avg_response_time_ms: f64,
}

impl fmt::Display for BackendStatistics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Backends: {}/{}, Connections: {}, Avg Response: {:.2}ms",
            self.healthy_backends,
            self.total_backends,
            self.total_connections,
            self.avg_response_time_ms
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let backend = Backend::new("node1".to_string(), addr);
        assert_eq!(backend.id, "node1");
        assert_eq!(backend.weight, 100);
        assert!(backend.healthy);
    }

    #[test]
    fn test_backend_load_score() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mut backend = Backend::new("node1".to_string(), addr);

        let initial_score = backend.load_score();

        backend.active_connections = 10;
        let loaded_score = backend.load_score();

        assert!(loaded_score > initial_score);
    }

    #[test]
    fn test_backend_metrics() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mut backend = Backend::new("node1".to_string(), addr);

        backend.update_metrics(100.0, true);
        assert!(backend.avg_response_time_ms > 0.0);
        assert!(backend.error_rate < 0.1);

        backend.update_metrics(200.0, false);
        assert!(backend.error_rate > 0.0);
    }

    #[test]
    fn test_load_balancer_context() {
        let ctx = LoadBalancerContext::with_key("user123")
            .with_priority(5)
            .with_client_id("client1");

        assert_eq!(ctx.key, Some("user123".to_string()));
        assert_eq!(ctx.priority, 5);
        assert_eq!(ctx.client_id, Some("client1".to_string()));
    }
}
