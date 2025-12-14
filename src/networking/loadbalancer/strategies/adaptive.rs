// Adaptive load balancing strategy.
//
// Uses latency, error rates, throughput, and other metrics to intelligently
// select the best backend. Can incorporate machine learning hints for
// predictive load balancing.

use super::{Backend, LoadBalancerContext, LoadBalancingStrategy};
use crate::error::{DbError, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Adaptive load balancing strategy
///
/// Selects backends based on multiple factors:
/// - Response time (latency)
/// - Error rate
/// - Active connections
/// - Throughput
/// - Backend weight
pub struct AdaptiveBalancer {
    /// Configuration for scoring
    config: ScoringConfig,
    /// Historical performance data
    history: Arc<RwLock<PerformanceHistory>>,
    /// Whether to enable predictive mode (uses historical patterns)
    predictive: bool,
}

/// Configuration for scoring backends
#[derive(Debug, Clone)]
pub struct ScoringConfig {
    /// Weight for latency in score calculation (0.0 to 1.0)
    pub latency_weight: f64,
    /// Weight for error rate in score calculation (0.0 to 1.0)
    pub error_weight: f64,
    /// Weight for connection count in score calculation (0.0 to 1.0)
    pub connection_weight: f64,
    /// Weight for throughput in score calculation (0.0 to 1.0)
    pub throughput_weight: f64,
    /// Normalization factor for latency (ms)
    pub latency_norm: f64,
    /// Normalization factor for connections
    pub connection_norm: f64,
    /// Normalization factor for throughput (req/s)
    pub throughput_norm: f64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            latency_weight: 0.4,
            error_weight: 0.3,
            connection_weight: 0.2,
            throughput_weight: 0.1,
            latency_norm: 100.0,     // 100ms baseline
            connection_norm: 100.0,  // 100 connections baseline
            throughput_norm: 1000.0, // 1000 req/s baseline
        }
    }
}

impl AdaptiveBalancer {
    /// Create a new adaptive balancer with default configuration
    pub fn new() -> Self {
        Self {
            config: ScoringConfig::default(),
            history: Arc::new(RwLock::new(PerformanceHistory::new())),
            predictive: false,
        }
    }

    /// Create with custom scoring configuration
    pub fn with_config(mut self, config: ScoringConfig) -> Self {
        self.config = config;
        self
    }

    /// Enable predictive mode
    pub fn with_predictive(mut self) -> Self {
        self.predictive = true;
        self
    }

    /// Calculate a composite score for a backend (lower is better)
    fn calculate_score(&self, backend: &Backend) -> f64 {
        if !backend.healthy {
            return f64::MAX;
        }

        // Normalize metrics
        let latency_score = backend.avg_response_time_ms / self.config.latency_norm;
        let error_score = backend.error_rate; // Already 0.0 to 1.0
        let connection_score = backend.active_connections as f64 / self.config.connection_norm;
        let throughput_score = if backend.throughput > 0.0 {
            self.config.throughput_norm / backend.throughput
        } else {
            1.0
        };

        // Weight factor (inverse - higher weight = lower score)
        let weight_factor = 100.0 / backend.weight as f64;

        // Combine scores with weights
        let score = latency_score * self.config.latency_weight
            + error_score * self.config.error_weight
            + connection_score * self.config.connection_weight
            + throughput_score * self.config.throughput_weight
            + weight_factor;

        score
    }

    /// Calculate a predictive score using historical data
    async fn calculate_predictive_score(&self, backend: &Backend) -> f64 {
        let base_score = self.calculate_score(backend);

        if !self.predictive {
            return base_score;
        }

        let history = self.history.read().await;
        let trend = history.get_trend(&backend.id);

        // Adjust score based on trend
        // Improving performance -> lower score (preferred)
        // Degrading performance -> higher score (avoid)
        base_score * (1.0 + trend * 0.2)
    }

    /// Select the backend with the best score
    async fn select_best(&self, backends: &[Backend]) -> Result<Backend> {
        if backends.is_empty() {
            return Err(DbError::Unavailable("No backends available".to_string()));
        }

        let mut best_backend = None;
        let mut best_score = f64::MAX;

        for backend in backends {
            let score = self.calculate_predictive_score(backend).await;
            if score < best_score {
                best_score = score;
                best_backend = Some(backend.clone());
            }
        }

        best_backend.ok_or_else(|| DbError::Unavailable("No backends available".to_string()))
    }

    /// Update performance history
    pub async fn update_history(
        &self,
        node_id: &str,
        latency_ms: f64,
        #[allow(dead_code)] // Reserved for error rate tracking
        error: bool,
    ) {
        let mut history = self.history.write().await;
        history.record(node_id, latency_ms, error);
    }
}

impl Default for AdaptiveBalancer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LoadBalancingStrategy for AdaptiveBalancer {
    async fn select(
        &self,
        backends: &[Backend],
        _context: &LoadBalancerContext,
    ) -> Result<Backend> {
        self.select_best(backends).await
    }

    fn name(&self) -> &str {
        if self.predictive {
            "adaptive-predictive"
        } else {
            "adaptive"
        }
    }

    async fn reset(&self) {
        let mut history = self.history.write().await;
        history.clear();
    }
}

/// Tracks performance history for backends
struct PerformanceHistory {
    /// Historical samples per backend
    samples: HashMap<String, Vec<PerformanceSample>>,
    /// Maximum samples to keep per backend
    max_samples: usize,
}

impl PerformanceHistory {
    /// Create a new performance history tracker
    fn new() -> Self {
        Self {
            samples: HashMap::new(),
            max_samples: 100,
        }
    }

    /// Record a performance sample
    fn record(&mut self, node_id: &str, latency_ms: f64, error: bool) {
        let sample = PerformanceSample {
            timestamp: Instant::now(),
            latency_ms,
            error,
        };

        let samples = self
            .samples
            .entry(node_id.to_string())
            .or_insert_with(Vec::new);
        samples.push(sample);

        // Keep only recent samples
        if samples.len() > self.max_samples {
            samples.remove(0);
        }
    }

    /// Get performance trend for a backend
    /// Returns -1.0 to 1.0 where:
    /// - Negative = improving
    /// - Zero = stable
    /// - Positive = degrading
    fn get_trend(&self, node_id: &str) -> f64 {
        let samples = match self.samples.get(node_id) {
            Some(s) if s.len() >= 10 => s,
            _ => return 0.0, // Not enough data
        };

        // Split into two halves and compare average latency
        let mid = samples.len() / 2;
        let first_half = &samples[..mid];
        let second_half = &samples[mid..];

        let avg_first: f64 =
            first_half.iter().map(|s| s.latency_ms).sum::<f64>() / first_half.len() as f64;
        let avg_second: f64 =
            second_half.iter().map(|s| s.latency_ms).sum::<f64>() / second_half.len() as f64;

        // Calculate trend (normalized)
        if avg_first > 0.0 {
            (avg_second - avg_first) / avg_first
        } else {
            0.0
        }
    }

    /// Clear all history
    fn clear(&mut self) {
        self.samples.clear();
    }
}

/// A single performance measurement
#[derive(Debug, Clone)]
struct PerformanceSample {
    #[allow(dead_code)] // Reserved for performance tracking
    timestamp: Instant,
    latency_ms: f64,
    #[allow(dead_code)] // Reserved for error rate tracking
    error: bool,
}

/// Latency-aware load balancer
///
/// Simplified adaptive balancer that focuses primarily on latency.
pub struct LatencyAwareBalancer {
    /// Recent latency window
    #[allow(dead_code)] // Reserved for latency window configuration
    window: Duration,
}

impl LatencyAwareBalancer {
    /// Create a new latency-aware balancer
    pub fn new(window: Duration) -> Self {
        Self { window }
    }

    /// Calculate latency score (lower is better)
    fn latency_score(&self, backend: &Backend) -> f64 {
        if !backend.healthy {
            return f64::MAX;
        }

        // Combine average latency with connection count
        let latency = backend.avg_response_time_ms;
        let load_penalty = backend.active_connections as f64 * 0.5;

        latency + load_penalty
    }
}

impl Default for LatencyAwareBalancer {
    fn default() -> Self {
        Self::new(Duration::from_secs(60))
    }
}

#[async_trait]
impl LoadBalancingStrategy for LatencyAwareBalancer {
    async fn select(
        &self,
        backends: &[Backend],
        _context: &LoadBalancerContext,
    ) -> Result<Backend> {
        if backends.is_empty() {
            return Err(DbError::Unavailable("No backends available".to_string()));
        }

        let mut best_backend = None;
        let mut best_score = f64::MAX;

        for backend in backends {
            let score = self.latency_score(backend);
            if score < best_score {
                best_score = score;
                best_backend = Some(backend.clone());
            }
        }

        best_backend.ok_or_else(|| DbError::Unavailable("No backends available".to_string()))
    }

    fn name(&self) -> &str {
        "latency-aware"
    }

    async fn reset(&self) {
        // No state to reset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    fn create_backend(id: &str, latency: f64, error_rate: f64, connections: u32) -> Backend {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mut backend = Backend::new(id.to_string(), addr);
        backend.avg_response_time_ms = latency;
        backend.error_rate = error_rate;
        backend.active_connections = connections;
        backend
    }

    #[tokio::test]
    async fn test_adaptive_score_calculation() {
        let balancer = AdaptiveBalancer::new();
        let backend = create_backend("node1", 50.0, 0.01, 10);

        let score = balancer.calculate_score(&backend);
        assert!(score > 0.0 && score < f64::MAX);
    }

    #[tokio::test]
    async fn test_adaptive_selection() {
        let balancer = AdaptiveBalancer::new();
        let backends = vec![
            create_backend("node0", 200.0, 0.1, 50), // High latency, high error
            create_backend("node1", 50.0, 0.01, 10), // Good performance
            create_backend("node2", 100.0, 0.05, 20), // Medium performance
        ];
        let ctx = LoadBalancerContext::default();

        let backend = balancer.select(&backends, &ctx).await.unwrap();
        assert_eq!(backend.id, "node1"); // Should select best performer
    }

    #[tokio::test]
    async fn test_custom_scoring_config() {
        let config = ScoringConfig {
            latency_weight: 1.0, // Only care about latency
            error_weight: 0.0,
            connection_weight: 0.0,
            throughput_weight: 0.0,
            ..Default::default()
        };

        let balancer = AdaptiveBalancer::new().with_config(config);
        let backends = vec![
            create_backend("node0", 100.0, 0.5, 100), // High error but low latency
            create_backend("node1", 200.0, 0.0, 0),   // No errors but high latency
        ];
        let ctx = LoadBalancerContext::default();

        let backend = balancer.select(&backends, &ctx).await.unwrap();
        assert_eq!(backend.id, "node0"); // Should prefer low latency
    }

    #[tokio::test]
    async fn test_performance_history() {
        let mut history = PerformanceHistory::new();

        // Record improving performance
        for i in 0..20 {
            let latency = 100.0 - (i as f64 * 2.0); // Decreasing latency
            history.record("node1", latency, false);
        }

        let trend = history.get_trend("node1");
        assert!(trend < 0.0, "Trend should be negative (improving)");

        history.clear();
        let trend = history.get_trend("node1");
        assert_eq!(trend, 0.0, "Should have no trend after clear");
    }

    #[tokio::test]
    async fn test_latency_aware_balancer() {
        let balancer = LatencyAwareBalancer::default();
        let backends = vec![
            create_backend("node0", 100.0, 0.0, 10),
            create_backend("node1", 50.0, 0.0, 5), // Lowest latency and load
            create_backend("node2", 75.0, 0.0, 20),
        ];
        let ctx = LoadBalancerContext::default();

        let backend = balancer.select(&backends, &ctx).await.unwrap();
        assert_eq!(backend.id, "node1"); // Should select lowest latency
    }

    #[tokio::test]
    async fn test_unhealthy_backend_excluded() {
        let balancer = AdaptiveBalancer::new();
        let mut backends = vec![
            create_backend("node0", 50.0, 0.0, 10),
            create_backend("node1", 40.0, 0.0, 5),
        ];
        backends[1].healthy = false; // Best backend is unhealthy
        let ctx = LoadBalancerContext::default();

        let backend = balancer.select(&backends, &ctx).await.unwrap();
        assert_eq!(backend.id, "node0"); // Should skip unhealthy backend
    }

    #[tokio::test]
    async fn test_predictive_mode() {
        let balancer = AdaptiveBalancer::new().with_predictive();

        // Record degrading performance for node0
        for i in 0..20 {
            let latency = 50.0 + (i as f64 * 2.0); // Increasing latency
            balancer.update_history("node0", latency, false).await;
        }

        // Record stable performance for node1
        for _ in 0..20 {
            balancer.update_history("node1", 60.0, false).await;
        }

        let backends = vec![
            create_backend("node0", 100.0, 0.0, 10), // Currently worse but degrading
            create_backend("node1", 90.0, 0.0, 10),  // Slightly better and stable
        ];

        let score0 = balancer.calculate_predictive_score(&backends[0]).await;
        let score1 = balancer.calculate_predictive_score(&backends[1]).await;

        // node0 should have worse score due to degrading trend
        assert!(score0 > score1, "Degrading backend should have worse score");
    }
}
