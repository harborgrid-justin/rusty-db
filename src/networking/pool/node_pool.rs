// # Node Pool
//
// Per-node connection pool implementation with lifecycle management,
// validation, and automatic scaling.

use super::{
    manager::HealthCheckResult, manager::PoolStatistics, Connection, PoolConfig, PooledConnection,
};
use crate::common::NodeId;
use crate::error::{DbError, Result};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock, Semaphore};

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection is being established
    Connecting,

    /// Connection is active and healthy
    Active,

    /// Connection is idle in the pool
    Idle,

    /// Connection is being validated
    Validating,

    /// Connection is closing
    Closing,

    /// Connection is closed
    Closed,
}

/// A managed connection with metadata
struct ManagedConnection {
    /// The actual connection
    connection: Box<dyn Connection>,

    /// Current state
    state: ConnectionState,

    /// Time when connection was created
    created_at: Instant,

    /// Time when last used
    last_used_at: Instant,

    /// Number of times this connection has been used
    usage_count: u64,
}

impl ManagedConnection {
    fn new(connection: Box<dyn Connection>) -> Self {
        let now = Instant::now();
        Self {
            connection,
            state: ConnectionState::Active,
            created_at: now,
            last_used_at: now,
            usage_count: 0,
        }
    }

    fn is_expired(&self, max_lifetime: Duration) -> bool {
        self.created_at.elapsed() > max_lifetime
    }

    fn is_idle_expired(&self, idle_timeout: Duration) -> bool {
        self.state == ConnectionState::Idle && self.last_used_at.elapsed() > idle_timeout
    }

    fn mark_used(&mut self) {
        self.last_used_at = Instant::now();
        self.usage_count += 1;
    }
}

/// Connection pool for a single node
pub struct NodePool {
    /// Node identifier
    node_id: NodeId,

    /// Pool configuration
    config: PoolConfig,

    /// Available connections (idle pool)
    idle_connections: Arc<Mutex<VecDeque<ManagedConnection>>>,

    /// Semaphore to limit concurrent connections
    connection_semaphore: Arc<Semaphore>,

    /// Statistics
    stats: Arc<PoolStatisticsInner>,

    /// Shutdown flag
    shutdown: Arc<RwLock<bool>>,

    /// Connection ID generator
    next_connection_id: Arc<AtomicU64>,
}

impl NodePool {
    /// Get the node ID for this pool
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Create a new node pool
    pub fn new(node_id: NodeId, config: PoolConfig) -> Self {
        let pool = Self {
            node_id,
            connection_semaphore: Arc::new(Semaphore::new(config.max_connections)),
            idle_connections: Arc::new(Mutex::new(VecDeque::new())),
            stats: Arc::new(PoolStatisticsInner::new()),
            shutdown: Arc::new(RwLock::new(false)),
            next_connection_id: Arc::new(AtomicU64::new(1)),
            config,
        };

        // Start background tasks
        pool.start_background_tasks();

        pool
    }

    /// Acquire a connection from the pool
    pub async fn acquire(&self) -> Result<PooledConnection> {
        // Check if pool is shutting down
        if *self.shutdown.read().await {
            return Err(DbError::InvalidState("Pool is shutting down".to_string()));
        }

        let start = Instant::now();

        // Try to get an idle connection first
        if let Some(mut managed) = self.pop_idle_connection().await {
            // Validate the connection
            if managed.connection.is_healthy().await {
                managed.mark_used();
                self.stats.record_acquire(start.elapsed());

                let connection = PooledConnection::new(
                    managed.connection,
                    Arc::new(Self::clone_arc_fields(self)),
                );

                return Ok(connection);
            } else {
                // Connection is unhealthy, close it
                let _ = managed.connection.close().await;
                self.stats
                    .connections_closed
                    .fetch_add(1, Ordering::Relaxed);
            }
        }

        // No idle connection available, create a new one
        // Acquire semaphore permit
        let permit = tokio::time::timeout(
            self.config.acquire_timeout,
            self.connection_semaphore.acquire(),
        )
        .await
        .map_err(|_| {
            DbError::Timeout(format!(
                "Failed to acquire connection to {} within {:?}",
                self.node_id, self.config.acquire_timeout
            ))
        })?
        .map_err(|_| DbError::Internal("Semaphore closed".to_string()))?;

        // Create new connection
        let connection = self.create_connection().await?;

        self.stats.record_acquire(start.elapsed());
        self.stats
            .connections_created
            .fetch_add(1, Ordering::Relaxed);

        // Keep the permit alive by forgetting it (connection counts toward limit)
        permit.forget();

        Ok(PooledConnection::new(
            connection,
            Arc::new(Self::clone_arc_fields(self)),
        ))
    }

    /// Release a connection back to the pool
    pub async fn release(&self, mut connection: Box<dyn Connection>, usage_duration_ms: u64) {
        // Update statistics
        self.stats
            .active_connections
            .fetch_sub(1, Ordering::Relaxed);
        self.stats.record_release(usage_duration_ms);

        // Check if shutting down
        if *self.shutdown.read().await {
            let _ = connection.close().await;
            return;
        }

        // Check if connection is still healthy
        if !connection.is_healthy().await {
            let _ = connection.close().await;
            self.stats
                .connections_closed
                .fetch_add(1, Ordering::Relaxed);
            return;
        }

        // Return to idle pool
        let mut managed = ManagedConnection::new(connection);
        managed.state = ConnectionState::Idle;

        let mut idle = self.idle_connections.lock().await;
        idle.push_back(managed);
    }

    /// Pop an idle connection from the pool
    async fn pop_idle_connection(&self) -> Option<ManagedConnection> {
        let mut idle = self.idle_connections.lock().await;

        while let Some(mut conn) = idle.pop_front() {
            // Check if connection is expired
            if conn.is_expired(self.config.max_lifetime)
                || conn.is_idle_expired(self.config.idle_timeout)
            {
                let _ = conn.connection.close().await;
                self.stats
                    .connections_closed
                    .fetch_add(1, Ordering::Relaxed);
                continue;
            }

            self.stats
                .active_connections
                .fetch_add(1, Ordering::Relaxed);
            return Some(conn);
        }

        None
    }

    /// Create a new connection to the node
    async fn create_connection(&self) -> Result<Box<dyn Connection>> {
        // This is a placeholder - in a real implementation, this would
        // establish a TCP connection to the node

        let connection_id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);

        // Create a mock connection for now
        Ok(Box::new(NodeConnection {
            connection_id,
            node_id: self.node_id.clone(),
            created_at: Instant::now(),
        }))
    }

    /// Clone Arc fields for creating a new NodePool reference
    fn clone_arc_fields(&self) -> Self {
        Self {
            node_id: self.node_id.clone(),
            config: self.config.clone(),
            idle_connections: Arc::clone(&self.idle_connections),
            connection_semaphore: Arc::clone(&self.connection_semaphore),
            stats: Arc::clone(&self.stats),
            shutdown: Arc::clone(&self.shutdown),
            next_connection_id: Arc::clone(&self.next_connection_id),
        }
    }

    /// Get pool statistics
    pub async fn statistics(&self) -> PoolStatistics {
        let idle = self.idle_connections.lock().await;
        let idle_count = idle.len();
        let active_count = self.stats.active_connections.load(Ordering::Relaxed);
        let total = idle_count + active_count;

        PoolStatistics {
            total_connections: total,
            active_connections: active_count,
            idle_connections: idle_count,
            pending_requests: self.stats.pending_requests.load(Ordering::Relaxed),
            connections_created: self.stats.connections_created.load(Ordering::Relaxed),
            connections_closed: self.stats.connections_closed.load(Ordering::Relaxed),
            successful_acquires: self.stats.successful_acquires.load(Ordering::Relaxed),
            failed_acquires: self.stats.failed_acquires.load(Ordering::Relaxed),
            avg_acquire_wait_ms: self.stats.avg_acquire_wait_ms(),
            max_acquire_wait_ms: self.stats.max_acquire_wait_ms.load(Ordering::Relaxed),
            utilization: if self.config.max_connections > 0 {
                active_count as f64 / self.config.max_connections as f64
            } else {
                0.0
            },
            exhaustion_count: self.stats.exhaustion_count.load(Ordering::Relaxed),
        }
    }

    /// Perform health check on all connections
    pub async fn health_check(&self) -> HealthCheckResult {
        let mut healthy = 0;
        let mut unhealthy = 0;

        let idle = self.idle_connections.lock().await;

        for conn in idle.iter() {
            if conn.connection.is_healthy().await {
                healthy += 1;
            } else {
                unhealthy += 1;
            }
        }

        HealthCheckResult {
            healthy: unhealthy == 0,
            healthy_connections: healthy,
            unhealthy_connections: unhealthy,
            error: if unhealthy > 0 {
                Some(format!("{} unhealthy connections", unhealthy))
            } else {
                None
            },
        }
    }

    /// Warm up the pool by creating initial connections
    pub async fn warmup(&self) -> Result<()> {
        let warmup_count = self
            .config
            .warmup_connections
            .min(self.config.max_connections);

        for _ in 0..warmup_count {
            let connection = self.create_connection().await?;
            let mut managed = ManagedConnection::new(connection);
            managed.state = ConnectionState::Idle;

            let mut idle = self.idle_connections.lock().await;
            idle.push_back(managed);

            self.stats
                .connections_created
                .fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Shutdown the pool and close all connections
    pub async fn shutdown(&self) -> Result<()> {
        // Set shutdown flag
        let mut shutdown = self.shutdown.write().await;
        *shutdown = true;
        drop(shutdown);

        // Close all idle connections
        let mut idle = self.idle_connections.lock().await;
        while let Some(mut conn) = idle.pop_front() {
            let _ = conn.connection.close().await;
            self.stats
                .connections_closed
                .fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Start background maintenance tasks
    fn start_background_tasks(&self) {
        if self.config.enable_health_checks {
            self.start_health_check_task();
        }

        self.start_eviction_task();
    }

    /// Start background health check task
    fn start_health_check_task(&self) {
        let idle_connections = Arc::clone(&self.idle_connections);
        let stats = Arc::clone(&self.stats);
        let interval = self.config.health_check_interval;
        let shutdown = Arc::clone(&self.shutdown);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                if *shutdown.read().await {
                    break;
                }

                // Check health of idle connections
                let mut idle = idle_connections.lock().await;
                let mut to_remove = Vec::new();

                for (idx, conn) in idle.iter().enumerate() {
                    if !conn.connection.is_healthy().await {
                        to_remove.push(idx);
                    }
                }

                // Remove unhealthy connections
                for idx in to_remove.into_iter().rev() {
                    if let Some(mut conn) = idle.remove(idx) {
                        let _ = conn.connection.close().await;
                        stats.connections_closed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });
    }

    /// Start background eviction task for idle connections
    fn start_eviction_task(&self) {
        let idle_connections = Arc::clone(&self.idle_connections);
        let stats = Arc::clone(&self.stats);
        let idle_timeout = self.config.idle_timeout;
        let max_lifetime = self.config.max_lifetime;
        let shutdown = Arc::clone(&self.shutdown);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(60));

            loop {
                ticker.tick().await;

                if *shutdown.read().await {
                    break;
                }

                let mut idle = idle_connections.lock().await;
                let mut to_remove = Vec::new();

                for (idx, conn) in idle.iter().enumerate() {
                    if conn.is_expired(max_lifetime) || conn.is_idle_expired(idle_timeout) {
                        to_remove.push(idx);
                    }
                }

                // Remove expired connections
                for idx in to_remove.into_iter().rev() {
                    if let Some(mut conn) = idle.remove(idx) {
                        let _ = conn.connection.close().await;
                        stats.connections_closed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });
    }
}

/// Internal statistics tracking
struct PoolStatisticsInner {
    active_connections: AtomicUsize,
    pending_requests: AtomicUsize,
    connections_created: AtomicU64,
    connections_closed: AtomicU64,
    successful_acquires: AtomicU64,
    failed_acquires: AtomicU64,
    total_acquire_wait_ms: AtomicU64,
    max_acquire_wait_ms: AtomicU64,
    exhaustion_count: AtomicU64,
}

impl PoolStatisticsInner {
    fn new() -> Self {
        Self {
            active_connections: AtomicUsize::new(0),
            pending_requests: AtomicUsize::new(0),
            connections_created: AtomicU64::new(0),
            connections_closed: AtomicU64::new(0),
            successful_acquires: AtomicU64::new(0),
            failed_acquires: AtomicU64::new(0),
            total_acquire_wait_ms: AtomicU64::new(0),
            max_acquire_wait_ms: AtomicU64::new(0),
            exhaustion_count: AtomicU64::new(0),
        }
    }

    fn record_acquire(&self, duration: Duration) {
        let wait_ms = duration.as_millis() as u64;
        self.successful_acquires.fetch_add(1, Ordering::Relaxed);
        self.total_acquire_wait_ms
            .fetch_add(wait_ms, Ordering::Relaxed);

        // Update max
        let mut current_max = self.max_acquire_wait_ms.load(Ordering::Relaxed);
        while wait_ms > current_max {
            match self.max_acquire_wait_ms.compare_exchange(
                current_max,
                wait_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }

    fn record_release(&self, _usage_duration_ms: u64) {
        // Could track usage duration statistics here
    }

    fn avg_acquire_wait_ms(&self) -> f64 {
        let total = self.total_acquire_wait_ms.load(Ordering::Relaxed);
        let count = self.successful_acquires.load(Ordering::Relaxed);

        if count > 0 {
            total as f64 / count as f64
        } else {
            0.0
        }
    }
}

/// Simple node connection implementation
pub struct NodeConnection {
    connection_id: u64,
    node_id: NodeId,
    created_at: Instant,
}

#[async_trait::async_trait]
impl Connection for NodeConnection {
    async fn is_healthy(&self) -> bool {
        // In a real implementation, this would perform a health check
        true
    }

    async fn close(&mut self) -> Result<()> {
        // Close the connection
        Ok(())
    }

    fn connection_id(&self) -> u64 {
        self.connection_id
    }

    fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    fn stats(&self) -> super::ConnectionStats {
        super::ConnectionStats {
            uptime_secs: self.created_at.elapsed().as_secs(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_pool_creation() {
        let config = PoolConfig::default();
        let pool = NodePool::new("test-node".to_string(), config);

        let stats = pool.statistics().await;
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.active_connections, 0);
    }

    #[tokio::test]
    async fn test_pool_warmup() {
        let mut config = PoolConfig::default();
        config.warmup_connections = 5;

        let pool = NodePool::new("test-node".to_string(), config);
        pool.warmup().await.unwrap();

        let stats = pool.statistics().await;
        assert_eq!(stats.idle_connections, 5);
    }
}
