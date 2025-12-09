//! Connection pool core engine module
//!
//! This module provides the core connection pooling functionality including:
//! - Pool configuration and management
//! - Connection lifecycle and state tracking
//! - Statement and cursor caching
//! - Connection guard implementation

use tokio::time::sleep;
use std::time::SystemTime;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Instant;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, AtomicBool, Ordering};
use std::time::Duration;
use std::collections::HashMap;
use parking_lot::RwLock;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use crate::error::{Result, DbError};

use super::wait_queue::WaitQueue;
use super::partitioning::PoolPartition;
use super::statistics::PoolStatistics;
use super::lifecycle::ConnectionFactory;

/// Errors specific to connection pooling
#[derive(Error, Debug, Clone)]
pub enum PoolError {
    #[error("Pool is closed")]
    PoolClosed,

    #[error("Connection timeout after {0:?}")]
    ConnectionTimeout(Duration),

    #[error("Pool exhausted: {active} active, {max} max")]
    PoolExhausted { active: usize, max: usize },

    #[error("Connection validation failed: {0}")]
    ValidationFailed(String),

    #[error("Connection creation failed: {0}")]
    CreationFailed(String),

    #[error("Connection lifetime exceeded: {current:?} > {max:?}")]
    LifetimeExceeded { current: Duration, max: Duration },

    #[error("Connection lease detected for {0:?}")]
    ConnectionLeak(Duration),

    #[error("Partition not found: {0}")]
    PartitionNotFound(String),

    #[error("Wait queue full: {current} >= {max}")]
    WaitQueueFull { current: usize, max: usize },

    #[error("Starvation detected for waiter {0}")]
    StarvationDetected(u64),

    #[error("Deadlock detected in wait queue")]
    DeadlockDetected,

    #[error("Invalid pool configuration: {0}")]
    InvalidConfig(String),
}

/// Configuration for connection pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum number of connections to maintain
    pub min_size: usize,

    /// Maximum number of connections allowed
    pub max_size: usize,

    /// Initial number of connections to create
    pub initial_size: usize,

    /// Timeout for acquiring a connection
    pub acquire_timeout: Duration,

    /// Maximum lifetime of a connection before forced recycling
    pub max_lifetime: Option<Duration>,

    /// Maximum idle time before connection is closed
    pub idle_timeout: Option<Duration>,

    /// Validate connections before returning from pool
    pub validate_on_acquire: bool,

    /// Validate connections before returning to pool
    pub validate_on_release: bool,

    /// Maximum time to wait for connection validation
    pub validation_timeout: Duration,

    /// Maximum number of waiters in queue
    pub max_wait_queue_size: usize,

    /// Connection creation throttle (max creations per second)
    pub creation_throttle: Option<u64>,

    /// Interval for background maintenance tasks
    pub maintenance_interval: Duration,

    /// Statement cache size per connection
    pub statement_cache_size: usize,

    /// Enable connection leak detection
    pub leak_detection_threshold: Option<Duration>,

    /// Fair queue mode (FIFO vs priority-based)
    pub fair_queue: bool,

    /// Enable pool partitioning
    pub enable_partitioning: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_size: 5,
            max_size: 100,
            initial_size: 10,
            acquire_timeout: Duration::from_secs(30),
            max_lifetime: Some(Duration::from_secs(3600)),
            idle_timeout: Some(Duration::from_secs(600)),
            validate_on_acquire: true,
            validate_on_release: false,
            validation_timeout: Duration::from_secs(5),
            max_wait_queue_size: 1000,
            creation_throttle: Some(10),
            maintenance_interval: Duration::from_secs(30),
            statement_cache_size: 100,
            leak_detection_threshold: Some(Duration::from_secs(300)),
            fair_queue: true,
            enable_partitioning: false,
        }
    }
}

impl PoolConfig {
    /// Validate pool configuration
    pub fn validate(&self) -> std::result::Result<(), PoolError> {
        if self.min_size > self.max_size {
            return Err(PoolError::InvalidConfig(
                format!("min_size ({}) > max_size ({})", self.min_size, self.max_size)
            ));
        }

        if self.initial_size > self.max_size {
            return Err(PoolError::InvalidConfig(
                format!("initial_size ({}) > max_size ({})", self.initial_size, self.max_size)
            ));
        }

        if self.initial_size < self.min_size {
            return Err(PoolError::InvalidConfig(
                format!("initial_size ({}) < min_size ({})", self.initial_size, self.min_size)
            ));
        }

        Ok(())
    }

    /// Create a builder for pool configuration
    pub fn builder() -> PoolConfigBuilder {
        PoolConfigBuilder::default()
    }
}

/// Builder for pool configuration
#[derive(Default)]
pub struct PoolConfigBuilder {
    config: PoolConfig,
}

impl PoolConfigBuilder {
    pub fn min_size(mut self, size: usize) -> Self {
        self.config.min_size = size;
        self
    }

    pub fn max_size(mut self, size: usize) -> Self {
        self.config.max_size = size;
        self
    }

    pub fn initial_size(mut self, size: usize) -> Self {
        self.config.initial_size = size;
        self
    }

    pub fn acquire_timeout(mut self, timeout: Duration) -> Self {
        self.config.acquire_timeout = timeout;
        self
    }

    pub fn max_lifetime(mut self, lifetime: Duration) -> Self {
        self.config.max_lifetime = Some(lifetime);
        self
    }

    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.config.idle_timeout = Some(timeout);
        self
    }

    pub fn statement_cache_size(mut self, size: usize) -> Self {
        self.config.statement_cache_size = size;
        self
    }

    pub fn enable_partitioning(mut self, enable: bool) -> Self {
        self.config.enable_partitioning = enable;
        self
    }

    pub fn build(self) -> std::result::Result<PoolConfig, PoolError> {
        self.config.validate()?;
        Ok(self.config)
    }
}

/// Recycling strategy for connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecyclingStrategy {
    /// Verify connection is still valid, reset state
    Fast,

    /// Full connection reset including session state
    Checked,

    /// Create new connection
    Replace,

    /// Age-based strategy
    Adaptive,
}

/// Connection state for internal tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionState {
    /// Connection is idle and available
    Idle,

    /// Connection is active and in use
    Active,

    /// Connection is being validated
    Validating,

    /// Connection is being recycled
    Recycling,

    /// Connection is closed
    Closed,
}

/// Internal connection wrapper with metadata
pub(crate) struct PooledConnection<C> {
    /// The actual connection
    pub(crate) connection: C,

    /// Unique connection ID
    pub(crate) id: u64,

    /// Current state
    pub(crate) state: ConnectionState,

    /// Time when connection was created
    pub(crate) created_at: Instant,

    /// Time when connection was last used
    pub(crate) last_used_at: Instant,

    /// Time when connection was acquired
    pub(crate) acquired_at: Option<Instant>,

    /// Number of times connection has been borrowed
    pub(crate) borrow_count: u64,

    /// Statement cache for this connection
    pub(crate) statement_cache: StatementCache,

    /// Cursor cache for this connection
    pub(crate) cursor_cache: CursorCache,

    /// Connection-specific metrics
    pub(crate) metrics: ConnectionMetrics,

    /// Owner information for leak detection
    pub(crate) owner: Option<String>,
}

impl<C> PooledConnection<C> {
    pub(crate) fn new(connection: C, id: u64, cache_size: usize) -> Self {
        let now = Instant::now();
        Self {
            connection,
            id,
            state: ConnectionState::Idle,
            created_at: now,
            last_used_at: now,
            acquired_at: None,
            borrow_count: 0,
            statement_cache: StatementCache::new(cache_size),
            cursor_cache: CursorCache::new(cache_size / 2),
            metrics: ConnectionMetrics::default(),
            owner: None,
        }
    }

    pub(crate) fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    pub(crate) fn idle_time(&self) -> Duration {
        self.last_used_at.elapsed()
    }

    pub(crate) fn active_time(&self) -> Option<Duration> {
        self.acquired_at.map(|t| t.elapsed())
    }

    pub(crate) fn is_expired(&self, max_lifetime: Option<Duration>) -> bool {
        if let Some(max) = max_lifetime {
            self.age() > max
        } else {
            false
        }
    }

    pub(crate) fn is_idle_timeout(&self, idle_timeout: Option<Duration>) -> bool {
        if let Some(timeout) = idle_timeout {
            self.idle_time() > timeout
        } else {
            false
        }
    }
}

/// Statement cache for prepared statements
pub(crate) struct StatementCache {
    pub(crate) cache: HashMap<String, CachedStatement>,
    pub(crate) max_size: usize,
    pub(crate) hits: u64,
    pub(crate) misses: u64,
}

impl StatementCache {
    pub(crate) fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            hits: 0,
            misses: 0,
        }
    }

    pub(crate) fn get(&mut self, sql: &str) -> Option<&CachedStatement> {
        if let Some(stmt) = self.cache.get(sql) {
            self.hits += 1;
            Some(stmt)
        } else {
            self.misses += 1;
            None
        }
    }

    pub(crate) fn insert(&mut self, sql: String, statement: CachedStatement) {
        if self.cache.len() >= self.max_size {
            // Simple LRU: remove oldest
            if let Some(key) = self.cache.keys().next().cloned() {
                self.cache.remove(&key);
            }
        }
        self.cache.insert(sql, statement);
    }

    pub(crate) fn clear(&mut self) {
        self.cache.clear();
    }

    pub(crate) fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Cached prepared statement
#[derive(Debug, Clone)]
pub(crate) struct CachedStatement {
    pub(crate) id: u64,
    pub(crate) sql: String,
    pub(crate) created_at: Instant,
    pub(crate) last_used: Instant,
    pub(crate) use_count: u64,
}

/// Cursor cache for open cursors
pub(crate) struct CursorCache {
    pub(crate) cache: HashMap<String, CachedCursor>,
    pub(crate) max_size: usize,
}

impl CursorCache {
    pub(crate) fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
        }
    }

    pub(crate) fn insert(&mut self, name: String, cursor: CachedCursor) {
        if self.cache.len() >= self.max_size {
            if let Some(key) = self.cache.keys().next().cloned() {
                self.cache.remove(&key);
            }
        }
        self.cache.insert(name, cursor);
    }

    pub(crate) fn clear(&mut self) {
        self.cache.clear();
    }
}

/// Cached cursor
#[derive(Debug, Clone)]
pub(crate) struct CachedCursor {
    pub(crate) id: u64,
    pub(crate) query: String,
    pub(crate) created_at: Instant,
}

/// Connection-specific metrics
#[derive(Debug, Default, Clone)]
pub(crate) struct ConnectionMetrics {
    pub(crate) queries_executed: u64,
    pub(crate) transactions_committed: u64,
    pub(crate) transactions_rolled_back: u64,
    pub(crate) bytes_sent: u64,
    pub(crate) bytes_received: u64,
    pub(crate) errors: u64,
}

/// Main connection pool
pub struct ConnectionPool<C> {
    /// Pool configuration
    config: Arc<PoolConfig>,

    /// Available connections
    idle: Arc<Mutex<VecDeque<PooledConnection<C>>>>,

    /// Active connections (tracking only)
    active: Arc<RwLock<HashMap<u64, Instant>>>,

    /// Connection factory
    factory: Arc<dyn ConnectionFactory<C>>,

    /// Next connection ID
    next_id: AtomicU64,

    /// Total connections created
    total_created: AtomicU64,

    /// Total connections destroyed
    total_destroyed: AtomicU64,

    /// Wait queue
    wait_queue: Arc<WaitQueue>,

    /// Pool partitions (if enabled)
    partitions: Arc<RwLock<HashMap<String, PoolPartition<C>>>>,

    /// Pool statistics
    stats: Arc<PoolStatistics>,

    /// Maintenance task handle
    maintenance_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,

    /// Pool state
    closed: AtomicBool,

    /// Creation semaphore for throttling
    creation_semaphore: Arc<Semaphore>,
}

impl<C: Send + Sync + 'static> ConnectionPool<C> {
    /// Create a new connection pool
    pub async fn new(
        config: PoolConfig,
        factory: Arc<dyn ConnectionFactory<C>>,
    ) -> std::result::Result<Self, PoolError> {
        config.validate()?;

        let pool = Self {
            config: Arc::new(config.clone()),
            idle: Arc::new(Mutex::new(VecDeque::new())),
            active: Arc::new(RwLock::new(HashMap::new())),
            factory,
            next_id: AtomicU64::new(1),
            total_created: AtomicU64::new(0),
            total_destroyed: AtomicU64::new(0),
            wait_queue: Arc::new(WaitQueue::new(config.max_wait_queue_size, config.fair_queue)),
            partitions: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(PoolStatistics::new()),
            maintenance_handle: Arc::new(Mutex::new(None)),
            closed: AtomicBool::new(false),
            creation_semaphore: Arc::new(Semaphore::new(config.max_size)),
        };

        // Initialize pool with initial connections
        pool.initialize().await?;

        // Start maintenance task
        pool.start_maintenance_task();

        Ok(pool)
    }

    /// Initialize pool with initial connections
    async fn initialize(&self) -> std::result::Result<(), PoolError> {
        let mut created = 0;
        let target = self.config.initial_size;

        while created < target {
            match self.create_connection().await {
                Ok(conn) => {
                    self.idle.lock().unwrap().push_back(conn);
                    created += 1;
                }
                Err(e) => {
                    tracing::warn!("Failed to create initial connection {}/{}: {}",
                                 created + 1, target, e);
                    // Continue trying to reach min_size at least
                    if created < self.config.min_size {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                    break;
                }
            }
        }

        if created < self.config.min_size {
            return Err(PoolError::CreationFailed(
                format!("Could only create {} of {} minimum connections", created, self.config.min_size)
            ));
        }

        Ok(())
    }

    /// Create a new connection
    async fn create_connection(&self) -> std::result::Result<PooledConnection<C>, PoolError> {
        // Apply throttling if configured
        if let Some(throttle) = self.config.creation_throttle {
            // Simple token bucket implementation
            let delay = Duration::from_millis(1000 / throttle);
            tokio::time::sleep(delay).await;
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        let connection = self.factory.create()
            .await
            .map_err(|e| PoolError::CreationFailed(e.to_string()))?;

        self.total_created.fetch_add(1, Ordering::SeqCst);
        self.stats.record_connection_created();

        Ok(PooledConnection::new(connection, id, self.config.statement_cache_size))
    }

    /// Start background maintenance task
    fn start_maintenance_task(&self) {
        let pool_weak = Arc::downgrade(&self.stats);
        let idle = Arc::clone(&self.idle);
        let active = Arc::clone(&self.active);
        let config = Arc::clone(&self.config);
        let closed = Arc::new(AtomicBool::new(self.closed.load(Ordering::Relaxed)));
        let total_destroyed = Arc::new(AtomicU64::new(self.total_destroyed.load(Ordering::Relaxed)));
        let stats = Arc::clone(&self.stats);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.maintenance_interval);

            loop {
                interval.tick().await;

                if closed.load(Ordering::SeqCst) {
                    break;
                }

                // Check if pool still exists
                if pool_weak.upgrade().is_none() {
                    break;
                }

                // Perform maintenance
                Self::perform_maintenance(
                    &idle,
                    &active,
                    &config,
                    &total_destroyed,
                    &stats,
                ).await;
            }
        });

        *self.maintenance_handle.lock().unwrap() = Some(handle);
    }

    /// Perform maintenance tasks
    async fn perform_maintenance(
        idle: &Arc<Mutex<VecDeque<PooledConnection<C>>>>,
        active: &Arc<RwLock<HashMap<u64, Instant>>>,
        config: &PoolConfig,
        total_destroyed: &AtomicU64,
        stats: &Arc<PoolStatistics>,
    ) {
        let mut idle_conns = idle.lock().unwrap();
        let active_conns = active.read();

        // Remove expired connections
        idle_conns.retain(|conn| {
            let expired = conn.is_expired(config.max_lifetime) ||
                         conn.is_idle_timeout(config.idle_timeout);

            if expired {
                total_destroyed.fetch_add(1, Ordering::SeqCst);
                stats.record_connection_destroyed();
            }

            !expired
        });

        // Detect leaks
        if let Some(leak_threshold) = config.leak_detection_threshold {
            for (&conn_id, &acquired_at) in active_conns.iter() {
                if acquired_at.elapsed() > leak_threshold {
                    tracing::warn!("Potential connection leak detected: connection {} active for {:?}",
                                 conn_id, acquired_at.elapsed());
                    stats.record_leak_detected();
                }
            }
        }

        // Ensure minimum connections
        let total = idle_conns.len() + active_conns.len();
        if total < config.min_size {
            tracing::info!("Pool below minimum size ({} < {}), will create more connections",
                         total, config.min_size);
        }
    }

    /// Acquire a connection from the pool
    pub async fn acquire(&self) -> std::result::Result<PooledConnectionGuard<C>, PoolError> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(PoolError::PoolClosed);
        }

        let start = Instant::now();
        self.stats.record_acquire_attempt();

        // Try to get connection with timeout
        let result = timeout(
            self.config.acquire_timeout,
            self.acquire_inner()
        ).await;

        match result {
            Ok(Ok(guard)) => {
                self.stats.record_acquire_success(start.elapsed());
                Ok(guard)
            }
            Ok(Err(e)) => {
                self.stats.record_acquire_failure();
                Err(e)
            }
            Err(_) => {
                self.stats.record_acquire_timeout();
                Err(PoolError::ConnectionTimeout(self.config.acquire_timeout))
            }
        }
    }

    /// Internal acquire implementation
    async fn acquire_inner(&self) -> std::result::Result<PooledConnectionGuard<C>, PoolError> {
        loop {
            // Try to get idle connection
            if let Some(mut conn) = self.idle.lock().unwrap().pop_front() {
                // Validate if required
                if self.config.validate_on_acquire {
                    if let Err(e) = self.validate_connection(&mut conn).await {
                        tracing::warn!("Connection validation failed: {}", e);
                        self.total_destroyed.fetch_add(1, Ordering::SeqCst);
                        continue;
                    }
                }

                // Mark as active
                conn.state = ConnectionState::Active;
                conn.acquired_at = Some(Instant::now());
                conn.borrow_count += 1;

                let conn_id = conn.id;
                self.active.write().insert(conn_id, Instant::now());

                return Ok(PooledConnectionGuard {
                    connection: Some(conn),
                    pool: PoolHandle {
                        idle: Arc::clone(&self.idle),
                        active: Arc::clone(&self.active),
                        config: Arc::clone(&self.config),
                        stats: Arc::clone(&self.stats),
                    },
                });
            }

            // No idle connections, try to create new one
            let active_count = self.active.read().len();
            if active_count < self.config.max_size {
                // Try to acquire semaphore permit
                if let Ok(permit) = self.creation_semaphore.clone().try_acquire_owned() {
                    return match self.create_connection().await {
                        Ok(mut conn) => {
                            conn.state = ConnectionState::Active;
                            conn.acquired_at = Some(Instant::now());

                            let conn_id = conn.id;
                            self.active.write().insert(conn_id, Instant::now());

                            // Release permit after creation
                            drop(permit);

                            Ok(PooledConnectionGuard {
                                connection: Some(conn),
                                pool: PoolHandle {
                                    idle: Arc::clone(&self.idle),
                                    active: Arc::clone(&self.active),
                                    config: Arc::clone(&self.config),
                                    stats: Arc::clone(&self.stats),
                                },
                            })
                        }
                        Err(e) => {
                            drop(permit);
                            Err(e)
                        }
                    }
                }
            }

            // Pool exhausted, wait in queue
            self.wait_queue.enqueue().await?;
        }
    }

    /// Validate a connection
    async fn validate_connection(&self, conn: &mut PooledConnection<C>) -> std::result::Result<(), PoolError> {
        conn.state = ConnectionState::Validating;

        let result = timeout(
            self.config.validation_timeout,
            self.factory.validate(&conn.connection)
        ).await;

        match result {
            Ok(Ok(true)) => {
                conn.state = ConnectionState::Idle;
                Ok(())
            }
            Ok(Ok(false)) => {
                conn.state = ConnectionState::Closed;
                Err(PoolError::ValidationFailed("Connection is not valid".to_string()))
            }
            Ok(Err(e)) => {
                conn.state = ConnectionState::Closed;
                Err(PoolError::ValidationFailed(e.to_string()))
            }
            Err(_) => {
                conn.state = ConnectionState::Closed;
                Err(PoolError::ValidationFailed("Validation timeout".to_string()))
            }
        }
    }

    /// Get current pool size
    pub fn size(&self) -> usize {
        self.idle.lock().unwrap().len() + self.active.read().len()
    }

    /// Get number of idle connections
    pub fn idle_count(&self) -> usize {
        self.idle.lock().unwrap().len()
    }

    /// Get number of active connections
    pub fn active_count(&self) -> usize {
        self.active.read().len()
    }

    /// Get pool statistics
    pub fn statistics(&self) -> super::statistics::PoolStats {
        self.stats.snapshot()
    }

    /// Close the pool
    pub async fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);

        // Cancel maintenance task
        if let Some(handle) = self.maintenance_handle.lock().unwrap().take() {
            handle.abort();
        }

        // Close all idle connections
        let mut idle = self.idle.lock().unwrap();
        idle.clear();

        // Wait for active connections to be returned
        // In a real implementation, you might want to forcefully close after a timeout
    }
}

/// Handle for returning connections to pool
pub(crate) struct PoolHandle<C> {
    pub(crate) idle: Arc<Mutex<VecDeque<PooledConnection<C>>>>,
    pub(crate) active: Arc<RwLock<HashMap<u64, Instant>>>,
    pub(crate) config: Arc<PoolConfig>,
    pub(crate) stats: Arc<PoolStatistics>,
}

/// Guard for a pooled connection
pub struct PooledConnectionGuard<C> {
    pub(crate) connection: Option<PooledConnection<C>>,
    pub(crate) pool: PoolHandle<C>,
}

impl<C> PooledConnectionGuard<C> {
    /// Get reference to the connection
    pub fn connection(&self) -> &C {
        &self.connection.as_ref().unwrap().connection
    }

    /// Get mutable reference to the connection
    pub fn connection_mut(&mut self) -> &mut C {
        &mut self.connection.as_mut().unwrap().connection
    }

    /// Get connection ID
    pub fn id(&self) -> u64 {
        self.connection.as_ref().unwrap().id
    }

    /// Get connection age
    pub fn age(&self) -> Duration {
        self.connection.as_ref().unwrap().age()
    }
}

impl<C> Drop for PooledConnectionGuard<C> {
    fn drop(&mut self) {
        if let Some(mut conn) = self.connection.take() {
            let conn_id = conn.id;

            // Remove from active
            self.pool.active.write().remove(&conn_id);

            // Reset state
            conn.state = ConnectionState::Idle;
            conn.last_used_at = Instant::now();
            conn.acquired_at = None;

            // Return to idle pool
            self.pool.idle.lock().unwrap().push_back(conn);

            self.pool.stats.record_connection_released();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_validation() {
        let config = PoolConfig {
            min_size: 10,
            max_size: 5,
            ..Default::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_pool_config_builder() {
        let config = PoolConfig::builder()
            .min_size(5)
            .max_size(100)
            .initial_size(10)
            .build();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.min_size, 5);
        assert_eq!(config.max_size, 100);
    }
}
