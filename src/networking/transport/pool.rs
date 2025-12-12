// Connection pool manager for P2P communication
//
// Manages a pool of connections to each peer with:
// - Min/max connections per peer
// - Idle connection cleanup
// - Connection health checking
// - Load-balanced connection selection

use crate::common::NodeId;
use crate::error::{DbError, Result};
use crate::networking::transport::connection::{Connection, TransportType};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Minimum number of connections per peer
    pub min_connections: usize,

    /// Maximum number of connections per peer
    pub max_connections: usize,

    /// Idle timeout before closing a connection
    pub idle_timeout: Duration,

    /// Health check interval
    pub health_check_interval: Duration,

    /// Connection acquisition timeout
    pub acquisition_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 1,
            max_connections: 10,
            idle_timeout: Duration::from_secs(300),
            health_check_interval: Duration::from_secs(30),
            acquisition_timeout: Duration::from_secs(5),
        }
    }
}

/// Connection selection strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionStrategy {
    /// Round-robin selection
    RoundRobin,

    /// Select connection with least messages sent
    LeastLoaded,

    /// Select first available healthy connection
    FirstAvailable,
}

/// Pool of connections to a single peer
struct PeerPool {
    #[allow(dead_code)]
    /// Peer node ID
    peer_id: NodeId,

    /// Active connections
    connections: Vec<Arc<Connection>>,

    /// Next connection index for round-robin
    next_index: usize,

    /// Time of last health check
    last_health_check: std::time::Instant,
}

impl PeerPool {
    fn new(peer_id: NodeId) -> Self {
        Self {
            peer_id,
            connections: Vec::new(),
            next_index: 0,
            last_health_check: std::time::Instant::now(),
        }
    }

    /// Add a connection to the pool
    fn add_connection(&mut self, conn: Arc<Connection>) {
        self.connections.push(conn);
    }

    /// Get a connection using the specified strategy
    async fn get_connection(&mut self, strategy: SelectionStrategy) -> Option<Arc<Connection>> {
        if self.connections.is_empty() {
            return None;
        }

        match strategy {
            SelectionStrategy::RoundRobin => {
                let index = self.next_index % self.connections.len();
                self.next_index = (self.next_index + 1) % self.connections.len();
                Some(Arc::clone(&self.connections[index]))
            }
            SelectionStrategy::LeastLoaded => {
                let mut min_messages = u64::MAX;
                let mut selected = None;

                for conn in &self.connections {
                    if conn.is_healthy().await {
                        let messages = conn.messages_sent();
                        if messages < min_messages {
                            min_messages = messages;
                            selected = Some(Arc::clone(conn));
                        }
                    }
                }

                selected
            }
            SelectionStrategy::FirstAvailable => {
                for conn in &self.connections {
                    if conn.is_healthy().await {
                        return Some(Arc::clone(conn));
                    }
                }
                None
            }
        }
    }

    /// Remove unhealthy connections
    async fn remove_unhealthy(&mut self) {
        self.connections.retain(|conn| {
            let is_healthy = futures::executor::block_on(conn.is_healthy());
            is_healthy
        });
    }

    /// Remove idle connections
    async fn remove_idle(&mut self, idle_timeout: Duration) {
        self.connections.retain(|conn| {
            let should_keep = !futures::executor::block_on(conn.should_close_idle(idle_timeout));
            should_keep
        });
    }

    /// Get number of connections
    fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

/// Connection pool manager
pub struct ConnectionPool {
    config: PoolConfig,
    pools: Arc<RwLock<HashMap<NodeId, PeerPool>>>,
    selection_strategy: SelectionStrategy,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: PoolConfig) -> Self {
        Self {
            config,
            pools: Arc::new(RwLock::new(HashMap::new())),
            selection_strategy: SelectionStrategy::RoundRobin,
        }
    }

    /// Set the connection selection strategy
    pub fn with_strategy(mut self, strategy: SelectionStrategy) -> Self {
        self.selection_strategy = strategy;
        self
    }

    /// Add a connection to the pool
    pub async fn add_connection(&self, peer_id: NodeId, transport_type: TransportType) -> Result<Arc<Connection>> {
        let mut pools = self.pools.write().await;
        let pool = pools.entry(peer_id.clone()).or_insert_with(|| PeerPool::new(peer_id.clone()));

        // Check if we've reached max connections
        if pool.connection_count() >= self.config.max_connections {
            return Err(DbError::LimitExceeded(format!(
                "Maximum connections ({}) reached for peer {}",
                self.config.max_connections, peer_id
            )));
        }

        let conn = Arc::new(Connection::new(peer_id, transport_type));
        pool.add_connection(Arc::clone(&conn));

        tracing::debug!(
            "Added connection to peer {} (total: {})",
            conn.peer_id(),
            pool.connection_count()
        );

        Ok(conn)
    }

    /// Get a connection to a peer
    pub async fn get_connection(&self, peer_id: &NodeId) -> Result<Arc<Connection>> {
        let mut pools = self.pools.write().await;

        let pool = pools
            .get_mut(peer_id)
            .ok_or_else(|| DbError::NotFound(format!("No connection pool for peer {}", peer_id)))?;

        pool.get_connection(self.selection_strategy)
            .await
            .ok_or_else(|| {
                DbError::Unavailable(format!("No healthy connections available for peer {}", peer_id))
            })
    }

    /// Get the number of connections to a peer
    pub async fn connection_count(&self, peer_id: &NodeId) -> usize {
        let pools = self.pools.read().await;
        pools.get(peer_id).map(|p| p.connection_count()).unwrap_or(0)
    }

    /// Remove all connections to a peer
    pub async fn remove_peer(&self, peer_id: &NodeId) -> Result<()> {
        let mut pools = self.pools.write().await;
        pools.remove(peer_id);
        tracing::debug!("Removed all connections for peer {}", peer_id);
        Ok(())
    }

    /// Perform health check on all connections
    pub async fn health_check(&self) -> Result<()> {
        let mut pools = self.pools.write().await;

        for (peer_id, pool) in pools.iter_mut() {
            // Remove unhealthy connections
            pool.remove_unhealthy().await;

            // Remove idle connections (but keep minimum)
            if pool.connection_count() > self.config.min_connections {
                pool.remove_idle(self.config.idle_timeout).await;
            }

            pool.last_health_check = std::time::Instant::now();

            tracing::trace!(
                "Health check for peer {}: {} connections",
                peer_id,
                pool.connection_count()
            );
        }

        Ok(())
    }

    /// Start background health check task
    pub fn start_health_check_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = interval(self.config.health_check_interval);

            loop {
                interval.tick().await;

                if let Err(e) = self.health_check().await {
                    tracing::error!("Health check failed: {}", e);
                }
            }
        })
    }

    /// Get statistics for all pools
    pub async fn get_statistics(&self) -> HashMap<NodeId, PoolStatistics> {
        let pools = self.pools.read().await;
        let mut stats = HashMap::new();

        for (peer_id, pool) in pools.iter() {
            let mut total_sent = 0;
            let mut total_received = 0;
            let mut healthy_count = 0;

            for conn in &pool.connections {
                total_sent += conn.bytes_sent();
                total_received += conn.bytes_received();
                if futures::executor::block_on(conn.is_healthy()) {
                    healthy_count += 1;
                }
            }

            stats.insert(
                peer_id.clone(),
                PoolStatistics {
                    total_connections: pool.connection_count(),
                    healthy_connections: healthy_count,
                    total_bytes_sent: total_sent,
                    total_bytes_received: total_received,
                },
            );
        }

        stats
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStatistics {
    pub total_connections: usize,
    pub healthy_connections: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
}

#[cfg(test)]
mod tests {
    use crate::networking::ConnectionState;
    use super::*;

    #[tokio::test]
    async fn test_connection_pool_creation() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::new(config);
        assert_eq!(pool.connection_count(&"node1".to_string()).await, 0);
    }

    #[tokio::test]
    async fn test_add_connection() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::new(config);

        let peer_id = "node1".to_string();
        let result = pool.add_connection(peer_id.clone(), TransportType::Tcp).await;
        assert!(result.is_ok());
        assert_eq!(pool.connection_count(&peer_id).await, 1);
    }

    #[tokio::test]
    async fn test_max_connections_limit() {
        let mut config = PoolConfig::default();
        config.max_connections = 2;
        let pool = ConnectionPool::new(config);

        let peer_id = "node1".to_string();

        // Add up to max
        pool.add_connection(peer_id.clone(), TransportType::Tcp).await.unwrap();
        pool.add_connection(peer_id.clone(), TransportType::Tcp).await.unwrap();

        // This should fail
        let result = pool.add_connection(peer_id.clone(), TransportType::Tcp).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_connection() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::new(config);

        let peer_id = "node1".to_string();
        let conn = pool.add_connection(peer_id.clone(), TransportType::Tcp).await.unwrap();

        // Set connection as active
        conn.set_state(ConnectionState::Active).await.unwrap();

        let retrieved = pool.get_connection(&peer_id).await;
        assert!(retrieved.is_ok());
    }

    #[tokio::test]
    async fn test_remove_peer() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::new(config);

        let peer_id = "node1".to_string();
        pool.add_connection(peer_id.clone(), TransportType::Tcp).await.unwrap();

        assert_eq!(pool.connection_count(&peer_id).await, 1);

        pool.remove_peer(&peer_id).await.unwrap();
        assert_eq!(pool.connection_count(&peer_id).await, 0);
    }

    #[tokio::test]
    async fn test_selection_strategies() {
        let config = PoolConfig::default();

        // Test round-robin
        let pool = ConnectionPool::new(config.clone()).with_strategy(SelectionStrategy::RoundRobin);
        assert_eq!(pool.selection_strategy, SelectionStrategy::RoundRobin);

        // Test least loaded
        let pool = ConnectionPool::new(config.clone()).with_strategy(SelectionStrategy::LeastLoaded);
        assert_eq!(pool.selection_strategy, SelectionStrategy::LeastLoaded);

        // Test first available
        let pool = ConnectionPool::new(config).with_strategy(SelectionStrategy::FirstAvailable);
        assert_eq!(pool.selection_strategy, SelectionStrategy::FirstAvailable);
    }
}
