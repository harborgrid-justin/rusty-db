// Least connections load balancing strategy.
//
// Selects the backend with the fewest active connections, with support for
// weighted selection and slow start.

use super::{Backend, LoadBalancerContext, LoadBalancingStrategy};
use crate::error::{DbError, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Least connections load balancer
///
/// Selects the backend with the fewest active connections,
/// which helps distribute load more evenly when requests have varying durations.
pub struct LeastConnectionsBalancer {
    /// Whether to use weighted selection
    weighted: bool,
    /// Slow start configuration
    slow_start: Option<Arc<RwLock<SlowStartTracker>>>,
}

impl LeastConnectionsBalancer {
    /// Create a new least connections balancer
    pub fn new() -> Self {
        Self {
            weighted: false,
            slow_start: None,
        }
    }

    /// Enable weighted least connections
    pub fn with_weights(mut self) -> Self {
        self.weighted = true;
        self
    }

    /// Enable slow start for new backends
    pub fn with_slow_start(mut self, duration: Duration) -> Self {
        self.slow_start = Some(Arc::new(RwLock::new(SlowStartTracker::new(duration))));
        self
    }

    /// Calculate connection ratio for a backend
    fn connection_ratio(&self, backend: &Backend) -> f64 {
        if backend.weight == 0 {
            return f64::MAX;
        }

        if self.weighted {
            // Weighted: connections per unit of weight
            backend.active_connections as f64 / backend.weight as f64
        } else {
            // Simple: just connection count
            backend.active_connections as f64
        }
    }

    /// Select backend with fewest connections (accounting for slow start)
    async fn select_least_loaded(&self, backends: &[Backend]) -> Result<Backend> {
        if backends.is_empty() {
            return Err(DbError::Unavailable("No backends available".to_string()));
        }

        let mut best_backend = None;
        let mut best_ratio = f64::MAX;

        for backend in backends {
            let mut ratio = self.connection_ratio(backend);

            // Apply slow start multiplier if enabled
            if let Some(slow_start) = &self.slow_start {
                let tracker = slow_start.read().await;
                ratio *= tracker.get_multiplier(&backend.id);
            }

            if ratio < best_ratio {
                best_ratio = ratio;
                best_backend = Some(backend.clone());
            }
        }

        best_backend.ok_or_else(|| DbError::Unavailable("No backends available".to_string()))
    }
}

impl Default for LeastConnectionsBalancer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LoadBalancingStrategy for LeastConnectionsBalancer {
    async fn select(
        &self,
        backends: &[Backend],
        _context: &LoadBalancerContext,
    ) -> Result<Backend> {
        let backend = self.select_least_loaded(backends).await?;

        // Register backend for slow start tracking
        if let Some(slow_start) = &self.slow_start {
            let mut tracker = slow_start.write().await;
            tracker.register(&backend.id);
        }

        Ok(backend)
    }

    fn name(&self) -> &str {
        if self.weighted {
            "weighted-least-connections"
        } else {
            "least-connections"
        }
    }

    async fn reset(&self) {
        if let Some(slow_start) = &self.slow_start {
            let mut tracker = slow_start.write().await;
            tracker.reset();
        }
    }
}

/// Tracks slow start state for backends
///
/// Gradually increases traffic to new backends over a configured duration
/// to avoid overwhelming them when they first join the pool.
struct SlowStartTracker {
    /// How long slow start lasts
    duration: Duration,
    /// When each backend was first seen
    start_times: HashMap<String, Instant>,
}

impl SlowStartTracker {
    /// Create a new slow start tracker
    fn new(duration: Duration) -> Self {
        Self {
            duration,
            start_times: HashMap::new(),
        }
    }

    /// Register a backend (records first seen time)
    fn register(&mut self, node_id: &str) {
        self.start_times
            .entry(node_id.to_string())
            .or_insert_with(Instant::now);
    }

    /// Get the slow start multiplier for a backend
    ///
    /// Returns 1.0 for fully ramped up backends, higher values during slow start.
    /// Higher multipliers make the backend appear more loaded, reducing traffic.
    fn get_multiplier(&self, node_id: &str) -> f64 {
        if let Some(&start_time) = self.start_times.get(node_id) {
            let elapsed = start_time.elapsed();
            if elapsed < self.duration {
                // Gradually decrease multiplier from 10.0 to 1.0
                let progress = elapsed.as_secs_f64() / self.duration.as_secs_f64();
                10.0 - (9.0 * progress)
            } else {
                1.0 // Fully ramped up
            }
        } else {
            10.0 // Not yet registered, apply maximum slow start penalty
        }
    }

    /// Reset all slow start state
    fn reset(&mut self) {
        self.start_times.clear();
    }
}

/// Least connections with power of two choices
///
/// Randomly selects two backends and chooses the one with fewer connections.
/// This provides good load distribution with lower overhead than checking all backends.
pub struct PowerOfTwoBalancer {
    weighted: bool,
}

impl PowerOfTwoBalancer {
    /// Create a new power of two balancer
    pub fn new() -> Self {
        Self { weighted: false }
    }

    /// Enable weighted selection
    pub fn with_weights(mut self) -> Self {
        self.weighted = true;
        self
    }

    /// Calculate connection ratio
    fn connection_ratio(&self, backend: &Backend) -> f64 {
        if backend.weight == 0 {
            return f64::MAX;
        }

        if self.weighted {
            backend.active_connections as f64 / backend.weight as f64
        } else {
            backend.active_connections as f64
        }
    }

    /// Select two random backends and return the one with fewer connections
    fn select_power_of_two(&self, backends: &[Backend]) -> Result<Backend> {
        use rand::seq::SliceRandom;

        if backends.is_empty() {
            return Err(DbError::Unavailable("No backends available".to_string()));
        }

        if backends.len() == 1 {
            return Ok(backends[0].clone());
        }

        let mut rng = rand::rng();
        let mut samples: Vec<_> = backends.iter().collect();
        samples.partial_shuffle(&mut rng, 2);

        let b1 = samples[0];
        let b2 = samples[1];

        if self.connection_ratio(b1) <= self.connection_ratio(b2) {
            Ok(b1.clone())
        } else {
            Ok(b2.clone())
        }
    }
}

impl Default for PowerOfTwoBalancer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LoadBalancingStrategy for PowerOfTwoBalancer {
    async fn select(
        &self,
        backends: &[Backend],
        _context: &LoadBalancerContext,
    ) -> Result<Backend> {
        self.select_power_of_two(backends)
    }

    fn name(&self) -> &str {
        if self.weighted {
            "weighted-power-of-two"
        } else {
            "power-of-two"
        }
    }

    async fn reset(&self) {
        // No state to reset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    fn create_backend_with_conns(id: &str, connections: u32, weight: u32) -> Backend {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mut backend = Backend::new(id.to_string(), addr);
        backend.active_connections = connections;
        backend.weight = weight;
        backend
    }

    #[tokio::test]
    async fn test_least_connections() {
        let balancer = LeastConnectionsBalancer::new();
        let backends = vec![
            create_backend_with_conns("node0", 10, 100),
            create_backend_with_conns("node1", 5, 100),
            create_backend_with_conns("node2", 15, 100),
        ];
        let ctx = LoadBalancerContext::default();

        let backend = balancer.select(&backends, &ctx).await.unwrap();
        assert_eq!(backend.id, "node1"); // Fewest connections
    }

    #[tokio::test]
    async fn test_weighted_least_connections() {
        let balancer = LeastConnectionsBalancer::new().with_weights();
        let backends = vec![
            create_backend_with_conns("node0", 10, 100), // Ratio: 0.1
            create_backend_with_conns("node1", 5, 50),   // Ratio: 0.1
            create_backend_with_conns("node2", 10, 200), // Ratio: 0.05 (best)
        ];
        let ctx = LoadBalancerContext::default();

        let backend = balancer.select(&backends, &ctx).await.unwrap();
        assert_eq!(backend.id, "node2"); // Best connection-to-weight ratio
    }

    #[tokio::test]
    async fn test_slow_start() {
        let balancer = LeastConnectionsBalancer::new().with_slow_start(Duration::from_secs(60));

        let backends = vec![
            create_backend_with_conns("node0", 5, 100),
            create_backend_with_conns("node1", 5, 100),
        ];
        let ctx = LoadBalancerContext::default();

        // First selection should register the backend
        let backend = balancer.select(&backends, &ctx).await.unwrap();

        // Verify slow start is tracking
        if let Some(slow_start) = &balancer.slow_start {
            let tracker = slow_start.read().await;
            let multiplier = tracker.get_multiplier(&backend.id);
            assert!(
                multiplier > 1.0,
                "New backend should have slow start penalty"
            );
        }
    }

    #[tokio::test]
    async fn test_power_of_two() {
        let balancer = PowerOfTwoBalancer::new();
        let backends = vec![
            create_backend_with_conns("node0", 100, 100),
            create_backend_with_conns("node1", 1, 100),
            create_backend_with_conns("node2", 100, 100),
        ];
        let ctx = LoadBalancerContext::default();

        // node1 should be selected most of the time due to low connections
        let mut selections = HashMap::new();
        for _ in 0..100 {
            let backend = balancer.select(&backends, &ctx).await.unwrap();
            *selections.entry(backend.id).or_insert(0) += 1;
        }

        // node1 should have significantly more selections
        let node1_count = selections.get("node1").unwrap_or(&0);
        assert!(
            *node1_count > 50,
            "node1 should be selected majority of the time"
        );
    }

    #[tokio::test]
    async fn test_empty_backends() {
        let balancer = LeastConnectionsBalancer::new();
        let backends = vec![];
        let ctx = LoadBalancerContext::default();

        let result = balancer.select(&backends, &ctx).await;
        assert!(result.is_err());
    }
}
