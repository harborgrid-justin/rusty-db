// # Connection Warmup
//
// Strategies for pre-establishing connections to minimize latency.
// Supports eager, lazy, and on-demand warmup policies.

use super::{NodePool, PoolConfig};
use crate::common::NodeId;
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Warmup strategy for connection pools
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarmupStrategy {
    /// No warmup - connections created on-demand
    None,

    /// Create minimum connections immediately
    Eager,

    /// Create connections lazily as needed
    Lazy,

    /// Create connections based on predicted load
    Predictive,

    /// Custom warmup count
    Custom(usize),
}

impl Default for WarmupStrategy {
    fn default() -> Self {
        WarmupStrategy::Eager
    }
}

/// Manages connection warmup for pools
pub struct WarmupManager {
    /// Default warmup strategy
    strategy: WarmupStrategy,

    /// Per-node warmup overrides
    node_overrides: Arc<RwLock<HashMap<NodeId, WarmupStrategy>>>,

    /// Warmup statistics
    stats: Arc<RwLock<WarmupStats>>,

    /// Background warmup task handle
    background_task: Option<tokio::task::JoinHandle<()>>,
}

impl WarmupManager {
    /// Create a new warmup manager
    pub fn new(strategy: WarmupStrategy) -> Self {
        Self {
            strategy,
            node_overrides: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(WarmupStats::default())),
            background_task: None,
        }
    }

    /// Set warmup strategy for a specific node
    pub async fn set_node_strategy(&self, node_id: NodeId, strategy: WarmupStrategy) {
        let mut overrides = self.node_overrides.write().await;
        overrides.insert(node_id, strategy);
    }

    /// Get warmup strategy for a node
    pub async fn get_node_strategy(&self, node_id: &NodeId) -> WarmupStrategy {
        let overrides = self.node_overrides.read().await;
        overrides.get(node_id).copied().unwrap_or(self.strategy)
    }

    /// Warm up a pool according to its strategy
    pub async fn warmup_pool(&self, pool: &NodePool, config: &PoolConfig) -> Result<()> {
        let node_id = pool.node_id().clone();
        let strategy = self.get_node_strategy(&node_id).await;

        let warmup_count = match strategy {
            WarmupStrategy::None => 0,
            WarmupStrategy::Eager => config.warmup_connections,
            WarmupStrategy::Lazy => config.min_connections,
            WarmupStrategy::Predictive => self.predict_warmup_count(config).await,
            WarmupStrategy::Custom(count) => count,
        };

        if warmup_count == 0 {
            return Ok(());
        }

        let start = Instant::now();

        // Perform warmup
        pool.warmup().await?;

        let duration = start.elapsed();

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_warmups += 1;
        stats.total_connections_warmed += warmup_count as u64;
        stats.total_warmup_time += duration;
        stats.last_warmup_time = Some(Instant::now());

        if warmup_count > 0 {
            let avg_per_connection = duration.as_millis() as f64 / warmup_count as f64;
            stats.avg_warmup_time_per_connection =
                (stats.avg_warmup_time_per_connection + avg_per_connection) / 2.0;
        }

        Ok(())
    }

    /// Predict optimal warmup count based on historical data
    async fn predict_warmup_count(&self, config: &PoolConfig) -> usize {
        // Simple prediction: use average between min and warmup connections
        (config.min_connections + config.warmup_connections) / 2
    }

    /// Warm up multiple pools concurrently
    pub async fn warmup_pools(
        &self,
        pools: Vec<(&NodePool, &PoolConfig)>,
    ) -> Result<Vec<Result<()>>> {
        let mut tasks = Vec::new();

        for (pool, config) in pools {
            let task = self.warmup_pool(pool, config);
            tasks.push(task);
        }

        // Execute all warmups concurrently
        let results = futures::future::join_all(tasks).await;

        Ok(results)
    }

    /// Start background warmup task
    pub fn start_background_warmup(
        &mut self,
        pools: Arc<RwLock<HashMap<NodeId, Arc<NodePool>>>>,
        config: PoolConfig,
        interval: Duration,
    ) {
        let _node_overrides = Arc::clone(&self.node_overrides);
        let _stats = Arc::clone(&self.stats);

        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                let pools_map = pools.read().await;

                for (node_id, pool) in pools_map.iter() {
                    // Check if pool needs warmup
                    let pool_stats = pool.statistics().await;

                    if pool_stats.idle_connections < config.min_connections {
                        // Pool below minimum, warm it up
                        if let Err(e) = pool.warmup().await {
                            eprintln!("Background warmup failed for {}: {}", node_id, e);
                        }
                    }
                }
            }
        });

        self.background_task = Some(handle);
    }

    /// Stop background warmup task
    pub async fn stop_background_warmup(&mut self) {
        if let Some(handle) = self.background_task.take() {
            handle.abort();
        }
    }

    /// Get warmup statistics
    pub async fn statistics(&self) -> WarmupStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Reset statistics
    pub async fn reset_statistics(&self) {
        let mut stats = self.stats.write().await;
        *stats = WarmupStats::default();
    }
}

impl Drop for WarmupManager {
    fn drop(&mut self) {
        if let Some(handle) = self.background_task.take() {
            handle.abort();
        }
    }
}

/// Statistics for connection warmup
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WarmupStats {
    /// Total number of warmup operations
    pub total_warmups: u64,

    /// Total connections created during warmup
    pub total_connections_warmed: u64,

    /// Total time spent on warmup
    pub total_warmup_time: Duration,

    /// Average time to warm up one connection (milliseconds)
    pub avg_warmup_time_per_connection: f64,

    /// Last warmup timestamp (skipped for serde - Instant cannot be serialized)
    #[serde(skip)]
    pub last_warmup_time: Option<Instant>,

    /// Number of failed warmup attempts
    pub failed_warmups: u64,
}

impl WarmupStats {
    /// Calculate average warmup time
    pub fn avg_warmup_time(&self) -> Duration {
        if self.total_warmups > 0 {
            self.total_warmup_time / self.total_warmups as u32
        } else {
            Duration::from_secs(0)
        }
    }

    /// Get time since last warmup
    pub fn time_since_last_warmup(&self) -> Option<Duration> {
        self.last_warmup_time.map(|t| t.elapsed())
    }
}

/// Warmup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarmupConfig {
    /// Warmup strategy
    pub strategy: WarmupStrategy,

    /// Warmup timeout
    pub timeout: Duration,

    /// Maximum concurrent warmup operations
    pub max_concurrent: usize,

    /// Retry failed warmups
    pub retry_on_failure: bool,

    /// Maximum retry attempts
    pub max_retries: usize,

    /// Delay between retries
    pub retry_delay: Duration,

    /// Enable background warmup
    pub enable_background: bool,

    /// Background warmup interval
    pub background_interval: Duration,
}

impl Default for WarmupConfig {
    fn default() -> Self {
        Self {
            strategy: WarmupStrategy::Eager,
            timeout: Duration::from_secs(10),
            max_concurrent: 10,
            retry_on_failure: true,
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
            enable_background: true,
            background_interval: Duration::from_secs(60),
        }
    }
}

/// Warmup coordinator for multiple nodes
pub struct WarmupCoordinator {
    /// Warmup manager
    manager: Arc<WarmupManager>,

    /// Warmup configuration
    config: WarmupConfig,

    /// Nodes to warm up
    nodes: Arc<RwLock<Vec<NodeId>>>,
}

impl WarmupCoordinator {
    /// Create a new warmup coordinator
    pub fn new(config: WarmupConfig) -> Self {
        let manager = Arc::new(WarmupManager::new(config.strategy));

        Self {
            manager,
            config,
            nodes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a node to warm up
    pub async fn add_node(&self, node_id: NodeId) {
        let mut nodes = self.nodes.write().await;
        if !nodes.contains(&node_id) {
            nodes.push(node_id);
        }
    }

    /// Remove a node from warmup
    pub async fn remove_node(&self, node_id: &NodeId) {
        let mut nodes = self.nodes.write().await;
        nodes.retain(|n| n != node_id);
    }

    /// Warm up all registered nodes
    pub async fn warmup_all(
        &self,
        pools: &HashMap<NodeId, Arc<NodePool>>,
        pool_config: &PoolConfig,
    ) -> HashMap<NodeId, Result<()>> {
        let nodes = self.nodes.read().await;
        let mut results = HashMap::new();

        for node_id in nodes.iter() {
            if let Some(pool) = pools.get(node_id) {
                let result = self.warmup_with_retry(pool, pool_config).await;

                results.insert(node_id.clone(), result);
            }
        }

        results
    }

    /// Warm up with retry logic
    async fn warmup_with_retry(&self, pool: &NodePool, config: &PoolConfig) -> Result<()> {
        let mut attempts = 0;
        let max_attempts = if self.config.retry_on_failure {
            self.config.max_retries + 1
        } else {
            1
        };

        loop {
            attempts += 1;

            match tokio::time::timeout(self.config.timeout, self.manager.warmup_pool(pool, config))
                .await
            {
                Ok(Ok(())) => return Ok(()),
                Ok(Err(e)) if attempts >= max_attempts => return Err(e),
                Err(_) if attempts >= max_attempts => {
                    return Err(DbError::Timeout(format!(
                        "Warmup timeout after {} attempts",
                        attempts
                    )));
                }
                _ => {
                    // Retry
                    tokio::time::sleep(self.config.retry_delay).await;
                }
            }
        }
    }

    /// Get warmup manager
    pub fn manager(&self) -> Arc<WarmupManager> {
        Arc::clone(&self.manager)
    }

    /// Get statistics
    pub async fn statistics(&self) -> WarmupStats {
        self.manager.statistics().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_warmup_strategy_default() {
        assert_eq!(WarmupStrategy::default(), WarmupStrategy::Eager);
    }

    #[tokio::test]
    async fn test_warmup_manager_creation() {
        let manager = WarmupManager::new(WarmupStrategy::Eager);

        let stats = manager.statistics().await;
        assert_eq!(stats.total_warmups, 0);
    }

    #[tokio::test]
    async fn test_node_strategy_override() {
        let manager = WarmupManager::new(WarmupStrategy::Eager);

        manager
            .set_node_strategy("node-1".to_string(), WarmupStrategy::Lazy)
            .await;

        let strategy = manager.get_node_strategy(&"node-1".to_string()).await;
        assert_eq!(strategy, WarmupStrategy::Lazy);

        let default_strategy = manager.get_node_strategy(&"node-2".to_string()).await;
        assert_eq!(default_strategy, WarmupStrategy::Eager);
    }

    #[test]
    fn test_warmup_config_default() {
        let config = WarmupConfig::default();
        assert_eq!(config.strategy, WarmupStrategy::Eager);
        assert!(config.retry_on_failure);
        assert_eq!(config.max_retries, 3);
    }

    #[tokio::test]
    async fn test_warmup_coordinator() {
        let config = WarmupConfig::default();
        let coordinator = WarmupCoordinator::new(config);

        coordinator.add_node("node-1".to_string()).await;
        coordinator.add_node("node-2".to_string()).await;

        let nodes = coordinator.nodes.read().await;
        assert_eq!(nodes.len(), 2);
    }

    #[tokio::test]
    async fn test_warmup_stats() {
        let stats = WarmupStats {
            total_warmups: 10,
            total_warmup_time: Duration::from_secs(100),
            ..Default::default()
        };

        let avg = stats.avg_warmup_time();
        assert_eq!(avg, Duration::from_secs(10));
    }
}
