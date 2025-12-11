//! Load balancing strategies for backend selection.
//!
//! This module provides multiple strategies for selecting backends:
//! - Round-robin: Simple rotation through backends
//! - Least connections: Select backend with fewest active connections
//! - Consistent hashing: Key-based routing for cache affinity
//! - Adaptive: ML-based selection using latency and error rates

use crate::error::{DbError, Result};
use async_trait::async_trait;

pub mod adaptive;
pub mod consistent_hash;
pub mod least_conn;
pub mod round_robin;

pub use adaptive::AdaptiveBalancer;
pub use consistent_hash::ConsistentHashBalancer;
pub use least_conn::LeastConnectionsBalancer;
pub use round_robin::RoundRobinBalancer;

use super::{Backend, LoadBalancerContext};

/// Trait for load balancing strategies
#[async_trait]
pub trait LoadBalancingStrategy: Send + Sync {
    /// Select a backend from the available pool
    async fn select(
        &self,
        backends: &[Backend],
        context: &LoadBalancerContext,
    ) -> Result<Backend>;

    /// Get the name of this strategy
    fn name(&self) -> &str;

    /// Reset internal state (e.g., for round-robin counter)
    async fn reset(&self);
}
