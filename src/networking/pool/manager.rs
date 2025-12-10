// # Pool Manager
//
// Manages connection pools for all nodes in the cluster. Provides centralized
// pool management, routing, and lifecycle coordination.

use crate::common::NodeId;
use crate::error::{DbError, Result};
use super::{
    PoolConfig, PoolEvent, PoolEventListener, NodePool, Connection,
    ConnectionStats, PoolMetrics,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

/// Central manager for all node connection pools
pub struct PoolManager {
    /// Configuration for all pools
    config: PoolConfig,

    /// Per-node connection pools
    pools: Arc<RwLock<HashMap<NodeId, Arc<NodePool>>>>,

    /// Global metrics aggregator
    metrics: Arc<PoolMetrics>,

    /// Event listeners
    listeners: Arc<RwLock<Vec<Arc<dyn PoolEventListener>>>>,

    /// Manager creation time
    created_at: Instant,
}

impl PoolManager {
    /// Create a new pool manager
    pub fn new(config: PoolConfig) -> Self {
        Self {
            config,
            pools: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(PoolMetrics::new()),
            listeners: Arc::new(RwLock::new(Vec::new())),
            created_at: Instant::now(),
        }
    }

    /// Acquire a connection to the specified node
    ///
    /// Creates a new pool for the node if one doesn't exist.
    pub async fn acquire(&self, node_id: &str) -> Result<PooledConnection> {
        let pool = self.get_or_create_pool(node_id).await?;

        let start = Instant::now();
        let connection = pool.acquire().await?;
        let wait_time_ms = start.elapsed().as_millis() as u64;

        // Update metrics
        self.metrics.record_acquire(wait_time_ms);

        // Emit event
        self.emit_event(PoolEvent::ConnectionAcquired {
            node_id: node_id.to_string(),
            connection_id: connection.connection_id(),
            wait_time_ms,
        }).await;

        Ok(connection)
    }

    /// Acquire a connection with a custom timeout
    pub async fn acquire_timeout(
        &self,
        node_id: &str,
        timeout: Duration,
    ) -> Result<PooledConnection> {
        tokio::time::timeout(timeout, self.acquire(node_id))
            .await
            .map_err(|_| DbError::Timeout(format!(
                "Failed to acquire connection to {} within {:?}",
                node_id, timeout
            )))?
    }

    /// Get or create a pool for the specified node
    async fn get_or_create_pool(&self, node_id: &str) -> Result<Arc<NodePool>> {
        // Fast path: check if pool exists
        {
            let pools = self.pools.read().await;
            if let Some(pool) = pools.get(node_id) {
                return Ok(Arc::clone(pool));
            }
        }

        // Slow path: create new pool
        let mut pools = self.pools.write().await;

        // Double-check in case another task created it
        if let Some(pool) = pools.get(node_id) {
            return Ok(Arc::clone(pool));
        }

        // Create new pool
        let pool = Arc::new(NodePool::new(
            node_id.to_string(),
            self.config.clone(),
        ));

        pools.insert(node_id.to_string(), Arc::clone(&pool));

        // Emit event
        self.emit_event(PoolEvent::PoolScaledUp {
            node_id: node_id.to_string(),
            old_size: 0,
            new_size: self.config.min_connections,
        }).await;

        Ok(pool)
    }

    /// Remove a pool for a specific node
    ///
    /// This is useful when a node leaves the cluster.
    pub async fn remove_pool(&self, node_id: &str) -> Result<()> {
        let mut pools = self.pools.write().await;

        if let Some(pool) = pools.remove(node_id) {
            pool.shutdown().await?;
        }

        Ok(())
    }

    /// Get pool for a specific node (if it exists)
    pub async fn get_pool(&self, node_id: &str) -> Option<Arc<NodePool>> {
        let pools = self.pools.read().await;
        pools.get(node_id).map(Arc::clone)
    }

    /// Get statistics for all pools
    pub async fn get_all_stats(&self) -> HashMap<NodeId, PoolStatistics> {
        let pools = self.pools.read().await;
        let mut stats = HashMap::new();

        for (node_id, pool) in pools.iter() {
            stats.insert(node_id.clone(), pool.statistics().await);
        }

        stats
    }

    /// Get statistics for a specific node's pool
    pub async fn get_pool_stats(&self, node_id: &str) -> Option<PoolStatistics> {
        let pool = self.get_pool(node_id).await?;
        Some(pool.statistics().await)
    }

    /// Get global metrics
    pub fn global_metrics(&self) -> Arc<PoolMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Add an event listener
    pub async fn add_listener(&self, listener: Arc<dyn PoolEventListener>) {
        let mut listeners = self.listeners.write().await;
        listeners.push(listener);
    }

    /// Emit an event to all listeners
    async fn emit_event(&self, event: PoolEvent) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener.on_event(event.clone());
        }
    }

    /// Shutdown all pools
    pub async fn shutdown(&self) -> Result<()> {
        let mut pools = self.pools.write().await;

        for (node_id, pool) in pools.drain() {
            if let Err(e) = pool.shutdown().await {
                eprintln!("Error shutting down pool for {}: {}", node_id, e);
            }
        }

        Ok(())
    }

    /// Get the number of active pools
    pub async fn pool_count(&self) -> usize {
        let pools = self.pools.read().await;
        pools.len()
    }

    /// Perform health checks on all pools
    pub async fn health_check_all(&self) -> HashMap<NodeId, HealthCheckResult> {
        let pools = self.pools.read().await;
        let mut results = HashMap::new();

        for (node_id, pool) in pools.iter() {
            let result = pool.health_check().await;
            results.insert(node_id.clone(), result);
        }

        results
    }

    /// Get uptime of the manager
    pub fn uptime(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Warm up connections to a specific node
    pub async fn warmup(&self, node_id: &str) -> Result<()> {
        let pool = self.get_or_create_pool(node_id).await?;
        pool.warmup().await
    }

    /// Warm up connections to multiple nodes
    pub async fn warmup_nodes(&self, node_ids: &[String]) -> Result<()> {
        let mut tasks = Vec::new();

        for node_id in node_ids {
            let pool = self.get_or_create_pool(node_id).await?;
            tasks.push(pool.warmup());
        }

        // Wait for all warmup tasks
        for task in tasks {
            task.await?;
        }

        Ok(())
    }
}

/// RAII guard for a pooled connection
///
/// Automatically returns the connection to the pool when dropped.
pub struct PooledConnection {
    /// The underlying connection
    connection: Option<Box<dyn Connection>>,

    /// Pool to return to
    pool: Arc<NodePool>,

    /// Time when connection was acquired
    acquired_at: Instant,

    /// Connection ID for tracking
    connection_id: u64,

    /// Node ID
    node_id: NodeId,
}

impl PooledConnection {
    /// Create a new pooled connection guard
    pub fn new(
        connection: Box<dyn Connection>,
        pool: Arc<NodePool>,
    ) -> Self {
        let connection_id = connection.connection_id();
        let node_id = connection.node_id().clone();

        Self {
            connection: Some(connection),
            pool,
            acquired_at: Instant::now(),
            connection_id,
            node_id,
        }
    }

    /// Get a reference to the underlying connection
    pub fn connection(&self) -> &dyn Connection {
        self.connection.as_ref().unwrap().as_ref()
    }

    /// Get a mutable reference to the underlying connection
    pub fn connection_mut(&mut self) -> &mut dyn Connection {
        self.connection.as_mut().unwrap().as_mut()
    }

    /// Get connection ID
    pub fn connection_id(&self) -> u64 {
        self.connection_id
    }

    /// Get node ID
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Get connection statistics
    pub fn stats(&self) -> ConnectionStats {
        self.connection().stats()
    }

    /// Get time since acquisition
    pub fn acquired_duration(&self) -> Duration {
        self.acquired_at.elapsed()
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            let pool = Arc::clone(&self.pool);
            let usage_duration_ms = self.acquired_at.elapsed().as_millis() as u64;

            // Return connection to pool asynchronously
            tokio::spawn(async move {
                pool.release(connection, usage_duration_ms).await;
            });
        }
    }
}

/// Statistics for a node pool
#[derive(Debug, Clone, Default)]
pub struct PoolStatistics {
    /// Total number of connections (active + idle)
    pub total_connections: usize,

    /// Number of active (in-use) connections
    pub active_connections: usize,

    /// Number of idle connections
    pub idle_connections: usize,

    /// Number of pending acquire requests
    pub pending_requests: usize,

    /// Total connections created since pool creation
    pub connections_created: u64,

    /// Total connections closed since pool creation
    pub connections_closed: u64,

    /// Total successful acquires
    pub successful_acquires: u64,

    /// Total failed acquires
    pub failed_acquires: u64,

    /// Average wait time for acquiring a connection (milliseconds)
    pub avg_acquire_wait_ms: f64,

    /// Maximum wait time recorded (milliseconds)
    pub max_acquire_wait_ms: u64,

    /// Pool utilization (active / max)
    pub utilization: f64,

    /// Number of times the pool was exhausted
    pub exhaustion_count: u64,
}

/// Result of a health check
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Is the pool healthy?
    pub healthy: bool,

    /// Number of healthy connections
    pub healthy_connections: usize,

    /// Number of unhealthy connections
    pub unhealthy_connections: usize,

    /// Error message if unhealthy
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_manager_creation() {
        let config = PoolConfig::default();
        let manager = PoolManager::new(config);

        assert_eq!(manager.pool_count().await, 0);
    }

    #[tokio::test]
    async fn test_pool_manager_multiple_nodes() {
        let config = PoolConfig::default();
        let manager = PoolManager::new(config);

        // Create pools for multiple nodes
        let _pool1 = manager.get_or_create_pool("node-1").await.unwrap();
        let _pool2 = manager.get_or_create_pool("node-2").await.unwrap();

        assert_eq!(manager.pool_count().await, 2);
    }

    #[tokio::test]
    async fn test_pool_removal() {
        let config = PoolConfig::default();
        let manager = PoolManager::new(config);

        let _pool = manager.get_or_create_pool("node-1").await.unwrap();
        assert_eq!(manager.pool_count().await, 1);

        manager.remove_pool("node-1").await.unwrap();
        assert_eq!(manager.pool_count().await, 0);
    }
}
