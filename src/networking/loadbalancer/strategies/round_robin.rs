//! Round-robin load balancing strategies.
//!
//! Provides simple round-robin, weighted round-robin, and smooth weighted round-robin
//! algorithms for distributing load across backends.

use super::{Backend, LoadBalancerContext, LoadBalancingStrategy};
use crate::error::{DbError, Result};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Simple round-robin load balancer
///
/// Distributes requests evenly across all healthy backends in rotation.
pub struct RoundRobinBalancer {
    current: AtomicUsize,
}

impl RoundRobinBalancer {
    /// Create a new round-robin balancer
    pub fn new() -> Self {
        Self {
            current: AtomicUsize::new(0),
        }
    }
}

impl Default for RoundRobinBalancer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LoadBalancingStrategy for RoundRobinBalancer {
    async fn select(
        &self,
        backends: &[Backend],
        _context: &LoadBalancerContext,
    ) -> Result<Backend> {
        if backends.is_empty() {
            return Err(DbError::Unavailable("No backends available".to_string()));
        }

        let index = self.current.fetch_add(1, Ordering::Relaxed) % backends.len();
        Ok(backends[index].clone())
    }

    fn name(&self) -> &str {
        "round-robin"
    }

    async fn reset(&self) {
        self.current.store(0, Ordering::Relaxed);
    }
}

/// Weighted round-robin load balancer
///
/// Distributes requests based on backend weights. Higher weight backends
/// receive proportionally more traffic.
pub struct WeightedRoundRobinBalancer {
    current: AtomicUsize,
}

impl WeightedRoundRobinBalancer {
    /// Create a new weighted round-robin balancer
    pub fn new() -> Self {
        Self {
            current: AtomicUsize::new(0),
        }
    }

    /// Calculate total weight of all backends
    fn total_weight(backends: &[Backend]) -> u32 {
        backends.iter().map(|b| b.weight).sum()
    }
}

impl Default for WeightedRoundRobinBalancer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LoadBalancingStrategy for WeightedRoundRobinBalancer {
    async fn select(
        &self,
        backends: &[Backend],
        _context: &LoadBalancerContext,
    ) -> Result<Backend> {
        if backends.is_empty() {
            return Err(DbError::Unavailable("No backends available".to_string()));
        }

        let total_weight = Self::total_weight(backends);
        if total_weight == 0 {
            return Err(DbError::Network("All backends have zero weight".to_string()));
        }

        // Get current position in the weighted sequence
        let position = self.current.fetch_add(1, Ordering::Relaxed) % total_weight as usize;

        // Find the backend corresponding to this position
        let mut cumulative = 0u32;
        for backend in backends {
            cumulative += backend.weight;
            if position < cumulative as usize {
                return Ok(backend.clone());
            }
        }

        // Fallback (should not reach here)
        Ok(backends[0].clone())
    }

    fn name(&self) -> &str {
        "weighted-round-robin"
    }

    async fn reset(&self) {
        self.current.store(0, Ordering::Relaxed);
    }
}

/// Backend state for smooth weighted round-robin
#[derive(Debug, Clone)]
struct SmoothBackendState {
    backend: Backend,
    current_weight: i32,
    effective_weight: i32,
}

/// Smooth weighted round-robin load balancer
///
/// Implements the NGINX smooth weighted round-robin algorithm which
/// provides better distribution than simple weighted round-robin.
/// Prevents sending all requests to the highest-weight backend in bursts.
pub struct SmoothWeightedRoundRobinBalancer {
    states: Arc<RwLock<Vec<SmoothBackendState>>>,
}

impl SmoothWeightedRoundRobinBalancer {
    /// Create a new smooth weighted round-robin balancer
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Initialize or update backend states
    async fn update_states(&self, backends: &[Backend]) {
        let mut states = self.states.write().await;

        // Remove states for backends that no longer exist
        states.retain(|s| backends.iter().any(|b| b.id == s.backend.id));

        // Add or update states
        for backend in backends {
            if let Some(state) = states.iter_mut().find(|s| s.backend.id == backend.id) {
                // Update existing state
                state.backend = backend.clone();
                state.effective_weight = backend.weight as i32;
            } else {
                // Add new state
                states.push(SmoothBackendState {
                    backend: backend.clone(),
                    current_weight: 0,
                    effective_weight: backend.weight as i32,
                });
            }
        }
    }

    /// Select backend using smooth weighted round-robin algorithm
    async fn select_smooth(&self) -> Result<Backend> {
        let mut states = self.states.write().await;

        if states.is_empty() {
            return Err(DbError::Unavailable("No backends available".to_string()));
        }

        let total_weight: i32 = states.iter().map(|s| s.effective_weight).sum();

        if total_weight == 0 {
            return Err(DbError::Network("All backends have zero weight".to_string()));
        }

        // Find backend with highest current_weight
        let mut best_idx = 0;
        let mut best_weight = states[0].current_weight;

        for (i, state) in states.iter().enumerate() {
            state.current_weight;
            // Increase current_weight by effective_weight
            let new_weight = state.current_weight + state.effective_weight;
            if new_weight > best_weight {
                best_idx = i;
                best_weight = new_weight;
            }
        }

        // Update weights
        for (i, state) in states.iter_mut().enumerate() {
            state.current_weight += state.effective_weight;
            if i == best_idx {
                state.current_weight -= total_weight;
            }
        }

        Ok(states[best_idx].backend.clone())
    }
}

impl Default for SmoothWeightedRoundRobinBalancer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LoadBalancingStrategy for SmoothWeightedRoundRobinBalancer {
    async fn select(
        &self,
        backends: &[Backend],
        _context: &LoadBalancerContext,
    ) -> Result<Backend> {
        if backends.is_empty() {
            return Err(DbError::Unavailable("No backends available".to_string()));
        }

        // Update states to match current backends
        self.update_states(backends).await;

        // Select using smooth algorithm
        self.select_smooth().await
    }

    fn name(&self) -> &str {
        "smooth-weighted-round-robin"
    }

    async fn reset(&self) {
        let mut states = self.states.write().await;
        for state in states.iter_mut() {
            state.current_weight = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    fn create_test_backends(count: usize) -> Vec<Backend> {
        (0..count)
            .map(|i| {
                let addr: SocketAddr = format!("127.0.0.1:{}", 8080 + i).parse().unwrap();
                Backend::new(format!("node{}", i), addr)
            })
            .collect()
    }

    #[tokio::test]
    async fn test_round_robin() {
        let balancer = RoundRobinBalancer::new();
        let backends = create_test_backends(3);
        let ctx = LoadBalancerContext::default();

        // Should rotate through backends
        let b1 = balancer.select(&backends, &ctx).await.unwrap();
        let b2 = balancer.select(&backends, &ctx).await.unwrap();
        let b3 = balancer.select(&backends, &ctx).await.unwrap();
        let b4 = balancer.select(&backends, &ctx).await.unwrap();

        assert_eq!(b1.id, "node0");
        assert_eq!(b2.id, "node1");
        assert_eq!(b3.id, "node2");
        assert_eq!(b4.id, "node0"); // Wraps around
    }

    #[tokio::test]
    async fn test_weighted_round_robin() {
        let balancer = WeightedRoundRobinBalancer::new();
        let mut backends = create_test_backends(2);
        backends[0].weight = 1;
        backends[1].weight = 2;
        let ctx = LoadBalancerContext::default();

        // node1 should appear twice as often
        let mut counts = std::collections::HashMap::new();
        for _ in 0..30 {
            let backend = balancer.select(&backends, &ctx).await.unwrap();
            *counts.entry(backend.id).or_insert(0) += 1;
        }

        assert_eq!(counts.get("node0"), Some(&10));
        assert_eq!(counts.get("node1"), Some(&20));
    }

    #[tokio::test]
    async fn test_smooth_weighted_round_robin() {
        let balancer = SmoothWeightedRoundRobinBalancer::new();
        let mut backends = create_test_backends(3);
        backends[0].weight = 5;
        backends[1].weight = 1;
        backends[2].weight = 1;
        let ctx = LoadBalancerContext::default();

        let mut selections = Vec::new();
        for _ in 0..7 {
            let backend = balancer.select(&backends, &ctx).await.unwrap();
            selections.push(backend.id);
        }

        // Verify smooth distribution (node0 should appear 5 times but not consecutively)
        let count0 = selections.iter().filter(|&id| id == "node0").count();
        assert_eq!(count0, 5);

        // Check that we don't get 5 consecutive selections of node0
        let mut max_consecutive = 0;
        let mut current_consecutive = 0;
        for id in selections {
            if id == "node0" {
                current_consecutive += 1;
                max_consecutive = max_consecutive.max(current_consecutive);
            } else {
                current_consecutive = 0;
            }
        }
        assert!(max_consecutive < 5, "Should not select same backend 5 times in a row");
    }

    #[tokio::test]
    async fn test_empty_backends() {
        let balancer = RoundRobinBalancer::new();
        let backends = vec![];
        let ctx = LoadBalancerContext::default();

        let result = balancer.select(&backends, &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reset() {
        let balancer = RoundRobinBalancer::new();
        let backends = create_test_backends(3);
        let ctx = LoadBalancerContext::default();

        balancer.select(&backends, &ctx).await.unwrap();
        balancer.select(&backends, &ctx).await.unwrap();

        balancer.reset().await;

        let backend = balancer.select(&backends, &ctx).await.unwrap();
        assert_eq!(backend.id, "node0"); // Should start from beginning
    }
}
