// # Enterprise Connection Pooling Engine
//
// This module provides a comprehensive, Oracle-inspired connection pooling system
// with advanced features including elastic sizing, sophisticated wait queue management,
// pool partitioning, and extensive monitoring capabilities.
//
// ## Key Features
//
// - **Elastic Pool Sizing**: Dynamic adjustment between min/max connections
// - **Connection Lifecycle Management**: Factory pattern, state reset, caching
// - **Advanced Wait Queue**: Fair/priority queuing, deadlock detection
// - **Pool Partitioning**: User/application/service-based isolation
// - **Comprehensive Monitoring**: Real-time metrics and leak detection
//
// ## Architecture
//
// The connection pool is designed for high concurrency with minimal contention:
// - Lock-free operations where possible
// - Fine-grained locking for critical sections
// - Background maintenance thread for housekeeping
// - Per-partition statistics for reduced contention

use tokio::time::sleep;
use std::time::SystemTime;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Instant;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, AtomicBool, Ordering};
use std::time::{Duration};
use std::collections::{HashMap};
use parking_lot::{RwLock, Condvar};
use tokio::sync::{Semaphore};
use tokio::time::timeout;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use crate::error::Result;

// ============================================================================
// SECTION 1: POOL CORE ENGINE (700+ lines)
// ============================================================================

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
enum ConnectionState {
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
struct PooledConnection<C> {
    /// The actual connection
    connection: C,

    /// Unique connection ID
    id: u64,

    /// Current state
    state: ConnectionState,

    /// Time when connection was created
    created_at: Instant,

    /// Time when connection was last used
    last_used_at: Instant,

    /// Time when connection was acquired
    acquired_at: Option<Instant>,

    /// Number of times connection has been borrowed
    borrow_count: u64,

    /// Statement cache for this connection
    statement_cache: StatementCache,

    /// Cursor cache for this connection
    cursor_cache: CursorCache,

    /// Connection-specific metrics
    metrics: ConnectionMetrics,

    /// Owner information for leak detection
    owner: Option<String>,
}

impl<C> PooledConnection<C> {
    fn new(connection: C, id: u64, cache_size: usize) -> Self {
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

    fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    fn idle_time(&self) -> Duration {
        self.last_used_at.elapsed()
    }

    fn active_time(&self) -> Option<Duration> {
        self.acquired_at.map(|t| t.elapsed())
    }

    fn is_expired(&self, max_lifetime: Option<Duration>) -> bool {
        if let Some(max) = max_lifetime {
            self.age() > max
        } else {
            false
        }
    }

    fn is_idle_timeout(&self, idle_timeout: Option<Duration>) -> bool {
        if let Some(timeout) = idle_timeout {
            self.idle_time() > timeout
        } else {
            false
        }
    }
}

/// Statement cache for prepared statements
struct StatementCache {
    cache: HashMap<String, CachedStatement>,
    max_size: usize,
    hits: u64,
    misses: u64,
}

impl StatementCache {
    fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            hits: 0,
            misses: 0,
        }
    }

    fn get(&mut self, sql: &str) -> Option<&CachedStatement> {
        if let Some(stmt) = self.cache.get(sql) {
            self.hits += 1;
            Some(stmt)
        } else {
            self.misses += 1;
            None
        }
    }

    fn insert(&mut self, sql: String, statement: CachedStatement) {
        if self.cache.len() >= self.max_size {
            // Simple LRU: remove oldest
            if let Some(key) = self.cache.keys().next().cloned() {
                self.cache.remove(&key);
            }
        }
        self.cache.insert(sql, statement);
    }

    fn clear(&mut self) {
        self.cache.clear();
    }

    fn hit_rate(&self) -> f64 {
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
struct CachedStatement {
    id: u64,
    sql: String,
    created_at: Instant,
    last_used: Instant,
    use_count: u64,
}

/// Cursor cache for open cursors
struct CursorCache {
    cache: HashMap<String, CachedCursor>,
    max_size: usize,
}

impl CursorCache {
    fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
        }
    }

    fn insert(&mut self, name: String, cursor: CachedCursor) {
        if self.cache.len() >= self.max_size {
            if let Some(key) = self.cache.keys().next().cloned() {
                self.cache.remove(&key);
            }
        }
        self.cache.insert(name, cursor);
    }

    fn clear(&mut self) {
        self.cache.clear();
    }
}

/// Cached cursor
#[derive(Debug, Clone)]
struct CachedCursor {
    id: u64,
    query: String,
    created_at: Instant,
}

/// Connection-specific metrics
#[derive(Debug, Default, Clone)]
struct ConnectionMetrics {
    queries_executed: u64,
    transactions_committed: u64,
    transactions_rolled_back: u64,
    bytes_sent: u64,
    bytes_received: u64,
    errors: u64,
}

/// Main connection pool
pub struct ConnectionPool<C> {
    /// Pool configuration
    config: Arc<PoolConfig>,

    /// Available connections
    idle: Arc<Mutex<VecDeque<PooledConnection<C>>>>,

    /// Active connections (tracking only)
    active: Arc<RwLock<HashMap<u64>>>,

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
                    self.idle.lock().push_back(conn);
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

        *self.maintenance_handle.lock() = Some(handle);
    }

    /// Perform maintenance tasks
    async fn perform_maintenance(
        idle: &Arc<Mutex<VecDeque<PooledConnection<C>>>>,
        active: &Arc<RwLock<HashMap<u64>>>,
        config: &PoolConfig,
        total_destroyed: &AtomicU64,
        stats: &Arc<PoolStatistics>,
    ) {
        let mut idle_conns = idle.lock();
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
            for (conn_id, acquired_at) in active_conns.iter() {
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
            if let Some(mut conn) = self.idle.lock().pop_front() {
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
        self.idle.lock().len() + self.active.read().len()
    }

    /// Get number of idle connections
    pub fn idle_count(&self) -> usize {
        self.idle.lock().len()
    }

    /// Get number of active connections
    pub fn active_count(&self) -> usize {
        self.active.read().len()
    }

    /// Get pool statistics
    pub fn statistics(&self) -> PoolStats {
        self.stats.snapshot()
    }

    /// Close the pool
    pub async fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);

        // Cancel maintenance task
        if let Some(handle) = self.maintenance_handle.lock().take() {
            handle.abort();
        }

        // Close all idle connections
        let mut idle = self.idle.lock();
        idle.clear();

        // Wait for active connections to be returned
        // In a real implementation, you might want to forcefully close after a timeout
    }
}

/// Handle for returning connections to pool
struct PoolHandle<C> {
    idle: Arc<Mutex<VecDeque<PooledConnection<C>>>>,
    active: Arc<RwLock<HashMap<u64>>>,
    config: Arc<PoolConfig>,
    stats: Arc<PoolStatistics>,
}

/// Guard for a pooled connection
pub struct PooledConnectionGuard<C> {
    connection: Option<PooledConnection<C>>,
    pool: PoolHandle<C>,
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
            self.pool.idle.lock().push_back(conn);

            self.pool.stats.record_connection_released();
        }
    }
}

// ============================================================================
// SECTION 2: CONNECTION LIFECYCLE (600+ lines)
// ============================================================================

/// Factory trait for creating connections
#[async_trait]
pub trait ConnectionFactory<C>: Send + Sync {
    /// Create a new connection
    async fn create(&self) -> Result<C>;

    /// Validate a connection
    async fn validate(&self, connection: &C) -> Result<bool>;

    /// Reset connection state
    async fn reset(&self, connection: &mut C) -> Result<()>;

    /// Close a connection
    async fn close(&self, connection: C) -> Result<()>;
}

/// Connection aging policy
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AgingPolicy {
    /// Time-based aging (absolute lifetime)
    TimeBased {
        max_lifetime: Duration,
    },

    /// Usage-based aging (number of borrows)
    UsageBased {
        max_borrows: u64,
    },

    /// Combined time and usage aging
    Combined {
        max_lifetime: Duration,
        max_borrows: u64,
    },

    /// Adaptive aging based on error rate
    Adaptive {
        base_lifetime: Duration,
        error_threshold: f64,
    },
}

impl AgingPolicy {
    /// Check if connection should be aged out
    pub fn should_recycle<C>(&self, conn: &PooledConnection<C>) -> bool {
        match self {
            AgingPolicy::TimeBased { max_lifetime } => {
                conn.age() > *max_lifetime
            }
            AgingPolicy::UsageBased { max_borrows } => {
                conn.borrow_count >= *max_borrows
            }
            AgingPolicy::Combined { max_lifetime, max_borrows } => {
                conn.age() > *max_lifetime || conn.borrow_count >= *max_borrows
            }
            AgingPolicy::Adaptive { base_lifetime, error_threshold } => {
                let error_rate = if conn.metrics.queries_executed > 0 {
                    conn.metrics.errors as f64 / conn.metrics.queries_executed as f64
                } else {
                    0.0
                };

                if error_rate > *error_threshold {
                    // Recycle faster if error rate is high
                    conn.age() > (*base_lifetime / 2)
                } else {
                    conn.age() > *base_lifetime
                }
            }
        }
    }
}

/// Connection state reset manager
pub struct StateResetManager {
    /// Whether to reset session variables
    reset_session_vars: bool,

    /// Whether to reset temporary tables
    reset_temp_tables: bool,

    /// Whether to clear prepared statements
    clear_prepared_statements: bool,

    /// Whether to rollback any open transactions
    rollback_transactions: bool,

    /// Custom reset queries
    custom_reset_queries: Vec<String>,
}

impl Default for StateResetManager {
    fn default() -> Self {
        Self {
            reset_session_vars: true,
            reset_temp_tables: true,
            clear_prepared_statements: false,
            rollback_transactions: true,
            custom_reset_queries: Vec::new(),
        }
    }
}

impl StateResetManager {
    /// Create a new state reset manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a custom reset query
    pub fn add_custom_query(&mut self, query: String) {
        self.custom_reset_queries.push(query);
    }

    /// Reset connection state (placeholder - would integrate with actual connection)
    pub async fn reset_state<C>(&self, connection: &mut C) -> Result<()> {
        // In a real implementation, this would execute the necessary SQL
        // commands to reset the connection state

        if self.rollback_transactions {
            // ROLLBACK any open transactions
        }

        if self.reset_session_vars {
            // Reset session variables to defaults
        }

        if self.reset_temp_tables {
            // Drop temporary tables
        }

        if self.clear_prepared_statements {
            // Deallocate prepared statements
        }

        // Execute custom reset queries
        for query in &self.custom_reset_queries {
            // Execute query
        }

        Ok(())
    }
}

/// Connection recycling manager
pub struct RecyclingManager {
    /// Default recycling strategy
    default_strategy: RecyclingStrategy,

    /// Aging policy
    aging_policy: AgingPolicy,

    /// State reset manager
    state_reset: StateResetManager,

    /// Metrics
    recycled_count: AtomicU64,
    replaced_count: AtomicU64,
}

impl RecyclingManager {
    /// Create a new recycling manager
    pub fn new(strategy: RecyclingStrategy, aging_policy: AgingPolicy) -> Self {
        Self {
            default_strategy: strategy,
            aging_policy,
            state_reset: StateResetManager::default(),
            recycled_count: AtomicU64::new(0),
            replaced_count: AtomicU64::new(0),
        }
    }

    /// Determine recycling strategy for a connection
    pub fn determine_strategy<C>(&self, conn: &PooledConnection<C>) -> RecyclingStrategy {
        match self.default_strategy {
            RecyclingStrategy::Adaptive => {
                if self.aging_policy.should_recycle(conn) {
                    RecyclingStrategy::Replace
                } else if conn.borrow_count > 100 {
                    RecyclingStrategy::Checked
                } else {
                    RecyclingStrategy::Fast
                }
            }
            other => other,
        }
    }

    /// Recycle a connection
    pub async fn recycle<C>(&self, conn: &mut PooledConnection<C>) -> Result<()>
    where
        C: Send + Sync,
    {
        let strategy = self.determine_strategy(conn);

        match strategy {
            RecyclingStrategy::Fast => {
                // Quick reset - just clear caches
                conn.statement_cache.clear();
                conn.cursor_cache.clear();
                self.recycled_count.fetch_add(1, Ordering::SeqCst);
            }
            RecyclingStrategy::Checked => {
                // Full state reset
                self.state_reset.reset_state(&mut conn.connection).await?;
                conn.statement_cache.clear();
                conn.cursor_cache.clear();
                self.recycled_count.fetch_add(1, Ordering::SeqCst);
            }
            RecyclingStrategy::Replace => {
                // Connection will be replaced by caller
                self.replaced_count.fetch_add(1, Ordering::SeqCst);
                return Err(DbError::InvalidOperation(
                    "Connection should be replaced".to_string()
                ));
            }
            RecyclingStrategy::Adaptive => {
                // Already resolved in determine_strategy
                unreachable!()
            }
        }

        Ok(())
    }

    /// Get recycling statistics
    pub fn statistics(&self) -> RecyclingStats {
        RecyclingStats {
            recycled_count: self.recycled_count.load(Ordering::SeqCst),
            replaced_count: self.replaced_count.load(Ordering::SeqCst),
        }
    }
}

/// Recycling statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecyclingStats {
    pub recycled_count: u64,
    pub replaced_count: u64,
}

/// Lifetime enforcement manager
pub struct LifetimeEnforcer {
    /// Maximum connection lifetime
    max_lifetime: Option<Duration>,

    /// Maximum idle time
    max_idle_time: Option<Duration>,

    /// Soft lifetime warning threshold
    soft_lifetime_threshold: Option<Duration>,

    /// Metrics
    enforced_count: AtomicU64,
    warnings_issued: AtomicU64,
}

impl LifetimeEnforcer {
    /// Create a new lifetime enforcer
    pub fn new(max_lifetime: Option<Duration>, max_idle_time: Option<Duration>) -> Self {
        let soft_lifetime_threshold = max_lifetime.map(|d| d * 9 / 10);

        Self {
            max_lifetime,
            max_idle_time,
            soft_lifetime_threshold,
            enforced_count: AtomicU64::new(0),
            warnings_issued: AtomicU64::new(0),
        }
    }

    /// Check if connection exceeds lifetime
    pub fn check_lifetime<C>(&self, conn: &PooledConnection<C>) -> LifetimeStatus {
        // Check absolute lifetime
        if let Some(max) = self.max_lifetime {
            let age = conn.age();
            if age > max {
                self.enforced_count.fetch_add(1, Ordering::SeqCst);
                return LifetimeStatus::Exceeded {
                    current: age,
                    max,
                };
            }

            if let Some(threshold) = self.soft_lifetime_threshold {
                if age > threshold {
                    self.warnings_issued.fetch_add(1, Ordering::SeqCst);
                    return LifetimeStatus::NearExpiry {
                        current: age,
                        max,
                    };
                }
            }
        }

        // Check idle time
        if let Some(max_idle) = self.max_idle_time {
            let idle = conn.idle_time();
            if idle > max_idle {
                self.enforced_count.fetch_add(1, Ordering::SeqCst);
                return LifetimeStatus::IdleTimeout {
                    idle_time: idle,
                    max: max_idle,
                };
            }
        }

        LifetimeStatus::Valid
    }

    /// Get enforcement statistics
    pub fn statistics(&self) -> LifetimeEnforcementStats {
        LifetimeEnforcementStats {
            enforced_count: self.enforced_count.load(Ordering::SeqCst),
            warnings_issued: self.warnings_issued.load(Ordering::SeqCst),
        }
    }
}

/// Lifetime status
#[derive(Debug, Clone)]
pub enum LifetimeStatus {
    /// Connection is valid
    Valid,

    /// Connection lifetime exceeded
    Exceeded {
        current: Duration,
        max: Duration,
    },

    /// Connection is near expiry
    NearExpiry {
        current: Duration,
        max: Duration,
    },

    /// Connection idle timeout
    IdleTimeout {
        idle_time: Duration,
        max: Duration,
    },
}

/// Lifetime enforcement statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifetimeEnforcementStats {
    pub enforced_count: u64,
    pub warnings_issued: u64,
}

/// Connection validator
pub struct ConnectionValidator {
    /// Validation query
    validation_query: Option<String>,

    /// Validation timeout
    timeout: Duration,

    /// Fast validation (ping-like)
    fast_validation: bool,

    /// Metrics
    validations_performed: AtomicU64,
    validations_failed: AtomicU64,
}

impl ConnectionValidator {
    /// Create a new validator
    pub fn new(timeout: Duration) -> Self {
        Self {
            validation_query: Some("SELECT 1".to_string()),
            timeout,
            fast_validation: true,
            validations_performed: AtomicU64::new(0),
            validations_failed: AtomicU64::new(0),
        }
    }

    /// Set validation query
    pub fn with_query(mut self, query: String) -> Self {
        self.validation_query = Some(query);
        self
    }

    /// Enable fast validation
    pub fn with_fast_validation(mut self, enabled: bool) -> Self {
        self.fast_validation = enabled;
        self
    }

    /// Validate connection (placeholder)
    pub async fn validate<C>(&self, _connection: &C) -> Result<bool> {
        self.validations_performed.fetch_add(1, Ordering::SeqCst);

        // In a real implementation, execute validation query
        // For now, always return true
        Ok(true)
    }

    /// Get validation statistics
    pub fn statistics(&self) -> ValidationStats {
        let performed = self.validations_performed.load(Ordering::SeqCst);
        let failed = self.validations_failed.load(Ordering::SeqCst);

        ValidationStats {
            validations_performed: performed,
            validations_failed: failed,
            success_rate: if performed > 0 {
                (performed - failed) as f64 / performed as f64
            } else {
                1.0
            },
        }
    }
}

/// Validation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStats {
    pub validations_performed: u64,
    pub validations_failed: u64,
    pub success_rate: f64,
}

// ============================================================================
// SECTION 3: WAIT QUEUE MANAGEMENT (500+ lines)
// ============================================================================

/// Wait queue for connection requests
pub struct WaitQueue {
    /// Queue entries
    entries: Mutex<VecDeque<WaitEntry>>,

    /// Condition variable for notifications
    condvar: Condvar,

    /// Maximum queue size
    max_size: usize,

    /// Fair queue mode
    fair_mode: bool,

    /// Next waiter ID
    next_id: AtomicU64,

    /// Queue statistics
    stats: WaitQueueStats,
}

/// Wait queue entry
struct WaitEntry {
    id: u64,
    enqueued_at: Instant,
    priority: QueuePriority,
    notified: Arc<AtomicBool>,
}

/// Queue priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueuePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for QueuePriority {
    fn default() -> Self {
        QueuePriority::Normal
    }
}

/// Wait queue statistics
#[derive(Default)]
struct WaitQueueStats {
    total_enqueued: AtomicU64,
    total_dequeued: AtomicU64,
    total_timeouts: AtomicU64,
    max_wait_time: AtomicU64, // in microseconds
    total_wait_time: AtomicU64, // in microseconds
}

impl WaitQueue {
    /// Create a new wait queue
    pub fn new(max_size: usize, fair_mode: bool) -> Self {
        Self {
            entries: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            max_size,
            fair_mode,
            next_id: AtomicU64::new(1),
            stats: WaitQueueStats::default(),
        }
    }

    /// Enqueue a waiter
    pub async fn enqueue(&self) -> std::result::Result<(), PoolError> {
        self.enqueue_with_priority(QueuePriority::Normal).await
    }

    /// Enqueue with priority
    pub async fn enqueue_with_priority(&self, priority: QueuePriority) -> std::result::Result<(), PoolError> {
        let mut queue = self.entries.lock();

        if queue.len() >= self.max_size {
            return Err(PoolError::WaitQueueFull {
                current: queue.len(),
                max: self.max_size,
            });
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let entry = WaitEntry {
            id,
            enqueued_at: Instant::now(),
            priority,
            notified: Arc::new(AtomicBool::new(false)),
        };

        if self.fair_mode {
            // FIFO - add to back
            queue.push_back(entry);
        } else {
            // Priority-based - insert based on priority
            let insert_pos = queue.iter().position(|e| e.priority < priority)
                .unwrap_or(queue.len());
            queue.insert(insert_pos, entry);
        }

        self.stats.total_enqueued.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    /// Notify next waiter
    pub fn notify_one(&self) {
        let mut queue = self.entries.lock();

        if let Some(entry) = queue.pop_front() {
            let wait_time = entry.enqueued_at.elapsed();
            self.record_wait_time(wait_time);

            entry.notified.store(true, Ordering::SeqCst);
            self.stats.total_dequeued.fetch_add(1, Ordering::SeqCst);

            self.condvar.notify_one();
        }
    }

    /// Notify all waiters
    pub fn notify_all(&self) {
        let mut queue = self.entries.lock();

        while let Some(entry) = queue.pop_front() {
            let wait_time = entry.enqueued_at.elapsed();
            self.record_wait_time(wait_time);

            entry.notified.store(true, Ordering::SeqCst);
            self.stats.total_dequeued.fetch_add(1, Ordering::SeqCst);
        }

        self.condvar.notify_all();
    }

    /// Get queue position for a waiter
    pub fn queue_position(&self, waiter_id: u64) -> Option<usize> {
        let queue = self.entries.lock();
        queue.iter().position(|e| e.id == waiter_id)
    }

    /// Get current queue length
    pub fn len(&self) -> usize {
        self.entries.lock().len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.entries.lock().is_empty()
    }

    /// Record wait time
    fn record_wait_time(&self, duration: Duration) {
        let micros = duration.as_micros() as u64;
        self.stats.total_wait_time.fetch_add(micros, Ordering::SeqCst);

        // Update max wait time
        let mut current_max = self.stats.max_wait_time.load(Ordering::SeqCst);
        while micros > current_max {
            match self.stats.max_wait_time.compare_exchange(
                current_max,
                micros,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
    }

    /// Get statistics
    pub fn statistics(&self) -> QueueStats {
        let total_enqueued = self.stats.total_enqueued.load(Ordering::SeqCst);
        let total_dequeued = self.stats.total_dequeued.load(Ordering::SeqCst);
        let total_wait_time = self.stats.total_wait_time.load(Ordering::SeqCst);
        let max_wait_time = self.stats.max_wait_time.load(Ordering::SeqCst);

        QueueStats {
            current_size: self.len(),
            total_enqueued,
            total_dequeued,
            total_timeouts: self.stats.total_timeouts.load(Ordering::SeqCst),
            average_wait_time: if total_dequeued > 0 {
                Duration::from_micros(total_wait_time / total_dequeued)
            } else {
                Duration::ZERO
            },
            max_wait_time: Duration::from_micros(max_wait_time),
        }
    }
}

/// Queue statistics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub current_size: usize,
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub total_timeouts: u64,
    pub average_wait_time: Duration,
    pub max_wait_time: Duration,
}

/// Deadlock detector for wait queue
pub struct DeadlockDetector {
    /// Detection enabled
    enabled: bool,

    /// Detection interval
    check_interval: Duration,

    /// Deadlock threshold (how long to wait before considering deadlock)
    deadlock_threshold: Duration,

    /// Detected deadlocks
    deadlocks_detected: AtomicU64,
}

impl DeadlockDetector {
    /// Create a new deadlock detector
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            check_interval: Duration::from_secs(10),
            deadlock_threshold: Duration::from_secs(60),
            deadlocks_detected: AtomicU64::new(0),
        }
    }

    /// Check for deadlocks in the wait queue
    pub fn check_deadlock(&self, queue: &WaitQueue) -> bool {
        if !self.enabled {
            return false;
        }

        let entries = queue.entries.lock();

        // Simple heuristic: if oldest waiter has been waiting too long
        if let Some(oldest) = entries.front() {
            if oldest.enqueued_at.elapsed() > self.deadlock_threshold {
                self.deadlocks_detected.fetch_add(1, Ordering::SeqCst);
                tracing::warn!("Potential deadlock detected: waiter {} waiting for {:?}",
                             oldest.id, oldest.enqueued_at.elapsed());
                return true;
            }
        }

        false
    }

    /// Get statistics
    pub fn statistics(&self) -> DeadlockStats {
        DeadlockStats {
            deadlocks_detected: self.deadlocks_detected.load(Ordering::SeqCst),
        }
    }
}

/// Deadlock statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadlockStats {
    pub deadlocks_detected: u64,
}

/// Starvation prevention system
pub struct StarvationPrevention {
    /// Maximum wait time before boosting priority
    max_wait_time: Duration,

    /// Priority boost increment
    priority_boost: u32,

    /// Boosted waiters
    boosted_count: AtomicU64,
}

impl StarvationPrevention {
    /// Create a new starvation prevention system
    pub fn new(max_wait_time: Duration) -> Self {
        Self {
            max_wait_time,
            priority_boost: 1,
            boosted_count: AtomicU64::new(0),
        }
    }

    /// Check for starvation and boost priority if needed
    pub fn check_and_boost(&self, queue: &WaitQueue) {
        let mut entries = queue.entries.lock();

        for entry in entries.iter_mut() {
            if entry.enqueued_at.elapsed() > self.max_wait_time {
                // Boost priority (simple increment for now)
                if entry.priority < QueuePriority::Critical {
                    entry.priority = match entry.priority {
                        QueuePriority::Low => QueuePriority::Normal,
                        QueuePriority::Normal => QueuePriority::High,
                        QueuePriority::High => QueuePriority::Critical,
                        QueuePriority::Critical => QueuePriority::Critical,
                    };

                    self.boosted_count.fetch_add(1, Ordering::SeqCst);

                    tracing::info!("Boosted priority for waiter {} after {:?}",
                                 entry.id, entry.enqueued_at.elapsed());
                }
            }
        }

        // Re-sort if not in fair mode
        if !queue.fair_mode {
            // Convert to Vec, sort, convert back
            let mut vec: Vec<_> = entries.drain(..).collect();
            vec.sort_by(|a, b| b.priority.cmp(&a.priority));
            entries.extend(vec);
        }
    }

    /// Get statistics
    pub fn statistics(&self) -> StarvationStats {
        StarvationStats {
            boosted_count: self.boosted_count.load(Ordering::SeqCst),
        }
    }
}

/// Starvation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarvationStats {
    pub boosted_count: u64,
}

// ============================================================================
// SECTION 4: POOL PARTITIONING (600+ lines)
// ============================================================================

/// Pool partition for isolation
pub struct PoolPartition<C> {
    /// Partition ID
    id: String,

    /// Partition type
    partition_type: PartitionType,

    /// Dedicated pool for this partition
    pool: Option<Arc<ConnectionPool<C>>>,

    /// Resource limits
    limits: PartitionLimits,

    /// Partition statistics
    stats: PartitionStatistics,

    /// Affinity rules
    affinity: AffinityRules,
}

/// Partition type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartitionType {
    /// User-based partitioning
    User(String),

    /// Application-based partitioning
    Application(String),

    /// Service-based partitioning
    Service(String),

    /// Tenant-based partitioning (multi-tenant isolation)
    Tenant(String),

    /// Resource group partitioning
    ResourceGroup(String),

    /// Custom partitioning
    Custom(String),
}

/// Resource limits for a partition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionLimits {
    /// Maximum connections for this partition
    pub max_connections: usize,

    /// Minimum connections to maintain
    pub min_connections: usize,

    /// Maximum wait queue size
    pub max_wait_queue: usize,

    /// CPU time limit (if applicable)
    pub cpu_limit: Option<Duration>,

    /// Memory limit (if applicable)
    pub memory_limit: Option<usize>,

    /// I/O limit (operations per second)
    pub io_limit: Option<u64>,
}

impl Default for PartitionLimits {
    fn default() -> Self {
        Self {
            max_connections: 50,
            min_connections: 2,
            max_wait_queue: 500,
            cpu_limit: None,
            memory_limit: None,
            io_limit: None,
        }
    }
}

/// Partition statistics
#[derive(Default)]
struct PartitionStatistics {
    connections_acquired: AtomicU64,
    connections_released: AtomicU64,
    wait_timeouts: AtomicU64,
    limit_violations: AtomicU64,
}

impl PartitionStatistics {
    fn snapshot(&self) -> PartitionStats {
        PartitionStats {
            connections_acquired: self.connections_acquired.load(Ordering::SeqCst),
            connections_released: self.connections_released.load(Ordering::SeqCst),
            wait_timeouts: self.wait_timeouts.load(Ordering::SeqCst),
            limit_violations: self.limit_violations.load(Ordering::SeqCst),
        }
    }
}

/// Partition statistics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionStats {
    pub connections_acquired: u64,
    pub connections_released: u64,
    pub wait_timeouts: u64,
    pub limit_violations: u64,
}

/// Affinity rules for routing connections
#[derive(Debug, Clone)]
pub struct AffinityRules {
    /// Preferred partition IDs
    preferred_partitions: Vec<String>,

    /// Fallback partition ID
    fallback_partition: Option<String>,

    /// Sticky sessions (same user always gets same partition)
    sticky_sessions: bool,

    /// Session affinity map
    session_map: Arc<RwLock<HashMap<String, String>>>,
}

impl Default for AffinityRules {
    fn default() -> Self {
        Self {
            preferred_partitions: Vec::new(),
            fallback_partition: None,
            sticky_sessions: false,
            session_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl AffinityRules {
    /// Create new affinity rules
    pub fn new() -> Self {
        Self::default()
    }

    /// Add preferred partition
    pub fn add_preferred(&mut self, partition_id: String) {
        self.preferred_partitions.push(partition_id);
    }

    /// Set fallback partition
    pub fn set_fallback(&mut self, partition_id: String) {
        self.fallback_partition = Some(partition_id);
    }

    /// Enable sticky sessions
    pub fn enable_sticky_sessions(&mut self) {
        self.sticky_sessions = true;
    }

    /// Get partition for session
    pub fn get_partition_for_session(&self, session_id: &str) -> Option<String> {
        if self.sticky_sessions {
            self.session_map.read().get(session_id).cloned()
        } else {
            None
        }
    }

    /// Set partition for session
    pub fn set_partition_for_session(&self, session_id: String, partition_id: String) {
        if self.sticky_sessions {
            self.session_map.write().insert(session_id, partition_id);
        }
    }
}

/// Partition manager
pub struct PartitionManager<C> {
    /// All partitions
    partitions: Arc<RwLock<HashMap<String, Arc<PoolPartition<C>>>>>,

    /// Default partition
    default_partition: Arc<RwLock<Option<String>>>,

    /// Routing strategy
    routing_strategy: RoutingStrategy,

    /// Load balancer
    load_balancer: LoadBalancer,
}

impl<C: Send + Sync + 'static> PartitionManager<C> {
    /// Create a new partition manager
    pub fn new(routing_strategy: RoutingStrategy) -> Self {
        Self {
            partitions: Arc::new(RwLock::new(HashMap::new())),
            default_partition: Arc::new(RwLock::new(None)),
            routing_strategy,
            load_balancer: LoadBalancer::new(),
        }
    }

    /// Create a new partition
    pub fn create_partition(
        &self,
        id: String,
        partition_type: PartitionType,
        limits: PartitionLimits,
    ) -> Arc<PoolPartition<C>> {
        let partition = Arc::new(PoolPartition {
            id: id.clone(),
            partition_type,
            pool: None, // Pool will be created separately
            limits,
            stats: PartitionStatistics::default(),
            affinity: AffinityRules::default(),
        });

        self.partitions.write().insert(id, Arc::clone(&partition));
        partition
    }

    /// Get partition by ID
    pub fn get_partition(&self, id: &str) -> Option<Arc<PoolPartition<C>>> {
        self.partitions.read().get(id).cloned()
    }

    /// Set default partition
    pub fn set_default_partition(&self, id: String) {
        *self.default_partition.write() = Some(id);
    }

    /// Route request to appropriate partition
    pub fn route_request(&self, request: &PartitionRequest) -> Option<String> {
        match &self.routing_strategy {
            RoutingStrategy::UserBased => {
                if let Some(user) = &request.user {
                    // Find or create user partition
                    Some(format!("user_{}", user))
                } else {
                    self.default_partition.read().clone()
                }
            }
            RoutingStrategy::ApplicationBased => {
                if let Some(app) = &request.application {
                    Some(format!("app_{}", app))
                } else {
                    self.default_partition.read().clone()
                }
            }
            RoutingStrategy::ServiceBased => {
                if let Some(service) = &request.service {
                    Some(format!("service_{}", service))
                } else {
                    self.default_partition.read().clone()
                }
            }
            RoutingStrategy::TenantBased => {
                if let Some(tenant) = &request.tenant {
                    Some(format!("tenant_{}", tenant))
                } else {
                    self.default_partition.read().clone()
                }
            }
            RoutingStrategy::LoadBalanced => {
                self.load_balancer.select_partition(&self.partitions.read())
            }
            RoutingStrategy::Custom(func) => {
                func(request)
            }
        }
    }

    /// List all partitions
    pub fn list_partitions(&self) -> Vec<String> {
        self.partitions.read().keys().cloned().collect()
    }

    /// Remove partition
    pub fn remove_partition(&self, id: &str) -> bool {
        self.partitions.write().remove(id).is_some()
    }

    /// Get statistics for all partitions
    pub fn all_statistics(&self) -> HashMap<String, PartitionStats> {
        self.partitions.read()
            .iter()
            .map(|(id, partition)| (id.clone(), partition.stats.snapshot()))
            .collect()
    }
}

/// Routing strategy for partitions
#[derive(Clone)]
pub enum RoutingStrategy {
    /// Route based on user
    UserBased,

    /// Route based on application
    ApplicationBased,

    /// Route based on service
    ServiceBased,

    /// Route based on tenant
    TenantBased,

    /// Load-balanced routing
    LoadBalanced,

    /// Custom routing function
    Custom(Arc<dyn Fn(&PartitionRequest) -> Option<String> + Send + Sync>),
}

/// Partition request information
#[derive(Debug, Clone)]
pub struct PartitionRequest {
    pub user: Option<String>,
    pub application: Option<String>,
    pub service: Option<String>,
    pub tenant: Option<String>,
    pub session_id: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl Default for PartitionRequest {
    fn default() -> Self {
        Self {
            user: None,
            application: None,
            service: None,
            tenant: None,
            session_id: None,
            metadata: HashMap::new(),
        }
    }
}

/// Load balancer for partitions
pub struct LoadBalancer {
    /// Load balancing algorithm
    algorithm: LoadBalancingAlgorithm,

    /// Round-robin counter
    round_robin_counter: AtomicU64,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new() -> Self {
        Self {
            algorithm: LoadBalancingAlgorithm::RoundRobin,
            round_robin_counter: AtomicU64::new(0),
        }
    }

    /// Select partition using load balancing algorithm
    pub fn select_partition<C>(&self, partitions: &HashMap<String, Arc<PoolPartition<C>>>) -> Option<String> {
        if partitions.is_empty() {
            return None;
        }

        match self.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                let keys: Vec<_> = partitions.keys().collect();
                let index = self.round_robin_counter.fetch_add(1, Ordering::SeqCst) as usize % keys.len();
                Some(keys[index].clone())
            }
            LoadBalancingAlgorithm::LeastConnections => {
                // Find partition with least active connections
                partitions.iter()
                    .min_by_key(|(_, p)| {
                        p.stats.connections_acquired.load(Ordering::SeqCst) -
                        p.stats.connections_released.load(Ordering::SeqCst)
                    })
                    .map(|(id, _)| id.clone())
            }
            LoadBalancingAlgorithm::Random => {
                let keys: Vec<_> = partitions.keys().collect();
                use rand::Rng;
                let index = rand::thread_rng().gen_range(0..keys.len());
                Some(keys[index].clone())
            }
        }
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::new()
    }
}

/// Load balancing algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalancingAlgorithm {
    /// Round-robin selection
    RoundRobin,

    /// Select partition with least active connections
    LeastConnections,

    /// Random selection
    Random,
}

// ============================================================================
// SECTION 5: POOL STATISTICS & MONITORING (600+ lines)
// ============================================================================

/// Comprehensive pool statistics
pub struct PoolStatistics {
    // Connection metrics
    connections_created: AtomicU64,
    connections_destroyed: AtomicU64,
    connections_acquired: AtomicU64,
    connections_released: AtomicU64,

    // Timing metrics
    acquire_attempts: AtomicU64,
    acquire_successes: AtomicU64,
    acquire_failures: AtomicU64,
    acquire_timeouts: AtomicU64,
    total_acquire_time: AtomicU64, // in microseconds

    // Wait queue metrics
    queue_additions: AtomicU64,
    queue_removals: AtomicU64,

    // Error metrics
    validation_failures: AtomicU64,
    creation_failures: AtomicU64,

    // Leak detection
    leaks_detected: AtomicU64,

    // Histogram data for wait times
    wait_time_histogram: Arc<RwLock<WaitTimeHistogram>>,

    // Connection usage patterns
    usage_patterns: Arc<RwLock<UsagePatterns>>,

    // Efficiency metrics
    efficiency_metrics: Arc<RwLock<EfficiencyMetrics>>,
}

impl PoolStatistics {
    /// Create new pool statistics
    pub fn new() -> Self {
        Self {
            connections_created: AtomicU64::new(0),
            connections_destroyed: AtomicU64::new(0),
            connections_acquired: AtomicU64::new(0),
            connections_released: AtomicU64::new(0),
            acquire_attempts: AtomicU64::new(0),
            acquire_successes: AtomicU64::new(0),
            acquire_failures: AtomicU64::new(0),
            acquire_timeouts: AtomicU64::new(0),
            total_acquire_time: AtomicU64::new(0),
            queue_additions: AtomicU64::new(0),
            queue_removals: AtomicU64::new(0),
            validation_failures: AtomicU64::new(0),
            creation_failures: AtomicU64::new(0),
            leaks_detected: AtomicU64::new(0),
            wait_time_histogram: Arc::new(RwLock::new(WaitTimeHistogram::new())),
            usage_patterns: Arc::new(RwLock::new(UsagePatterns::new())),
            efficiency_metrics: Arc::new(RwLock::new(EfficiencyMetrics::new())),
        }
    }

    /// Record connection created
    pub fn record_connection_created(&self) {
        self.connections_created.fetch_add(1, Ordering::SeqCst);
    }

    /// Record connection destroyed
    pub fn record_connection_destroyed(&self) {
        self.connections_destroyed.fetch_add(1, Ordering::SeqCst);
    }

    /// Record acquire attempt
    pub fn record_acquire_attempt(&self) {
        self.acquire_attempts.fetch_add(1, Ordering::SeqCst);
    }

    /// Record successful acquire
    pub fn record_acquire_success(&self, duration: Duration) {
        self.acquire_successes.fetch_add(1, Ordering::SeqCst);
        self.connections_acquired.fetch_add(1, Ordering::SeqCst);

        let micros = duration.as_micros() as u64;
        self.total_acquire_time.fetch_add(micros, Ordering::SeqCst);

        self.wait_time_histogram.write().record(duration);
        self.usage_patterns.write().record_acquisition();
    }

    /// Record failed acquire
    pub fn record_acquire_failure(&self) {
        self.acquire_failures.fetch_add(1, Ordering::SeqCst);
    }

    /// Record acquire timeout
    pub fn record_acquire_timeout(&self) {
        self.acquire_timeouts.fetch_add(1, Ordering::SeqCst);
    }

    /// Record connection released
    pub fn record_connection_released(&self) {
        self.connections_released.fetch_add(1, Ordering::SeqCst);
        self.usage_patterns.write().record_release();
    }

    /// Record leak detected
    pub fn record_leak_detected(&self) {
        self.leaks_detected.fetch_add(1, Ordering::SeqCst);
    }

    /// Get statistics snapshot
    pub fn snapshot(&self) -> PoolStats {
        let acquire_attempts = self.acquire_attempts.load(Ordering::SeqCst);
        let acquire_successes = self.acquire_successes.load(Ordering::SeqCst);
        let total_acquire_time = self.total_acquire_time.load(Ordering::SeqCst);

        PoolStats {
            connections_created: self.connections_created.load(Ordering::SeqCst),
            connections_destroyed: self.connections_destroyed.load(Ordering::SeqCst),
            connections_acquired: self.connections_acquired.load(Ordering::SeqCst),
            connections_released: self.connections_released.load(Ordering::SeqCst),
            active_connections: self.connections_acquired.load(Ordering::SeqCst)
                .saturating_sub(self.connections_released.load(Ordering::SeqCst)),
            acquire_attempts,
            acquire_successes,
            acquire_failures: self.acquire_failures.load(Ordering::SeqCst),
            acquire_timeouts: self.acquire_timeouts.load(Ordering::SeqCst),
            average_acquire_time: if acquire_successes > 0 {
                Duration::from_micros(total_acquire_time / acquire_successes)
            } else {
                Duration::ZERO
            },
            success_rate: if acquire_attempts > 0 {
                acquire_successes as f64 / acquire_attempts as f64
            } else {
                1.0
            },
            validation_failures: self.validation_failures.load(Ordering::SeqCst),
            creation_failures: self.creation_failures.load(Ordering::SeqCst),
            leaks_detected: self.leaks_detected.load(Ordering::SeqCst),
            wait_time_histogram: self.wait_time_histogram.read().snapshot(),
            usage_patterns: self.usage_patterns.read().snapshot(),
            efficiency_metrics: self.efficiency_metrics.read().snapshot(),
        }
    }
}

impl Default for PoolStatistics {
    fn default() -> Self {
        Self::new()
    }
}

/// Pool statistics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub connections_created: u64,
    pub connections_destroyed: u64,
    pub connections_acquired: u64,
    pub connections_released: u64,
    pub active_connections: u64,
    pub acquire_attempts: u64,
    pub acquire_successes: u64,
    pub acquire_failures: u64,
    pub acquire_timeouts: u64,
    pub average_acquire_time: Duration,
    pub success_rate: f64,
    pub validation_failures: u64,
    pub creation_failures: u64,
    pub leaks_detected: u64,
    pub wait_time_histogram: HistogramSnapshot,
    pub usage_patterns: UsagePatternsSnapshot,
    pub efficiency_metrics: EfficiencyMetricsSnapshot,
}

/// Wait time histogram
struct WaitTimeHistogram {
    buckets: BTreeMap<u64, u64>, // microseconds -> count
    total_samples: u64,
}

impl WaitTimeHistogram {
    fn new() -> Self {
        Self {
            buckets: BTreeMap::new(),
            total_samples: 0,
        }
    }

    fn record(&mut self, duration: Duration) {
        let micros = duration.as_micros() as u64;

        // Bucket into powers of 2
        let bucket = if micros == 0 {
            0
        } else {
            let log2 = 63 - micros.leading_zeros();
            1u64 << log2
        };

        *self.buckets.entry(bucket).or_insert(0) += 1;
        self.total_samples += 1;
    }

    fn snapshot(&self) -> HistogramSnapshot {
        HistogramSnapshot {
            buckets: self.buckets.clone(),
            total_samples: self.total_samples,
            percentiles: self.calculate_percentiles(),
        }
    }

    fn calculate_percentiles(&self) -> Percentiles {
        if self.total_samples == 0 {
            return Percentiles::default();
        }

        let mut cumulative = 0u64;
        let mut p50 = 0u64;
        let mut p95 = 0u64;
        let mut p99 = 0u64;

        for (&bucket, &count) in &self.buckets {
            cumulative += count;
            let percentile = (cumulative as f64 / self.total_samples as f64) * 100.0;

            if percentile >= 50.0 && p50 == 0 {
                p50 = bucket;
            }
            if percentile >= 95.0 && p95 == 0 {
                p95 = bucket;
            }
            if percentile >= 99.0 && p99 == 0 {
                p99 = bucket;
            }
        }

        Percentiles {
            p50: Duration::from_micros(p50),
            p95: Duration::from_micros(p95),
            p99: Duration::from_micros(p99),
        }
    }
}

/// Histogram snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramSnapshot {
    pub buckets: BTreeMap<u64, u64>,
    pub total_samples: u64,
    pub percentiles: Percentiles,
}

/// Percentile values
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Percentiles {
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
}

/// Connection usage patterns
struct UsagePatterns {
    acquisitions_by_hour: [u64; 24],
    releases_by_hour: [u64; 24],
    peak_hour: usize,
    last_update: Instant,
}

impl UsagePatterns {
    fn new() -> Self {
        Self {
            acquisitions_by_hour: [0; 24],
            releases_by_hour: [0; 24],
            peak_hour: 0,
            last_update: Instant::now(),
        }
    }

    fn record_acquisition(&mut self) {
        let hour = self.current_hour();
        self.acquisitions_by_hour[hour] += 1;
        self.update_peak_hour();
    }

    fn record_release(&mut self) {
        let hour = self.current_hour();
        self.releases_by_hour[hour] += 1;
    }

    fn current_hour(&self) -> usize {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        ((now.as_secs() / 3600) % 24) as usize
    }

    fn update_peak_hour(&mut self) {
        if let Some((hour, _)) = self.acquisitions_by_hour.iter()
            .enumerate()
            .max_by_key(|(_, &count)| count)
        {
            self.peak_hour = hour;
        }
    }

    fn snapshot(&self) -> UsagePatternsSnapshot {
        UsagePatternsSnapshot {
            acquisitions_by_hour: self.acquisitions_by_hour,
            releases_by_hour: self.releases_by_hour,
            peak_hour: self.peak_hour,
        }
    }
}

/// Usage patterns snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePatternsSnapshot {
    pub acquisitions_by_hour: [u64; 24],
    pub releases_by_hour: [u64; 24],
    pub peak_hour: usize,
}

/// Pool efficiency metrics
struct EfficiencyMetrics {
    cache_hit_rate: f64,
    connection_reuse_rate: f64,
    pool_utilization: f64,
    last_calculated: Instant,
}

impl EfficiencyMetrics {
    fn new() -> Self {
        Self {
            cache_hit_rate: 0.0,
            connection_reuse_rate: 0.0,
            pool_utilization: 0.0,
            last_calculated: Instant::now(),
        }
    }

    fn update(&mut self, cache_hits: u64, cache_total: u64, reused: u64, total: u64, utilization: f64) {
        if cache_total > 0 {
            self.cache_hit_rate = cache_hits as f64 / cache_total as f64;
        }

        if total > 0 {
            self.connection_reuse_rate = reused as f64 / total as f64;
        }

        self.pool_utilization = utilization;
        self.last_calculated = Instant::now();
    }

    fn snapshot(&self) -> EfficiencyMetricsSnapshot {
        EfficiencyMetricsSnapshot {
            cache_hit_rate: self.cache_hit_rate,
            connection_reuse_rate: self.connection_reuse_rate,
            pool_utilization: self.pool_utilization,
        }
    }
}

/// Efficiency metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfficiencyMetricsSnapshot {
    pub cache_hit_rate: f64,
    pub connection_reuse_rate: f64,
    pub pool_utilization: f64,
}

/// Real-time dashboard data provider
pub struct DashboardProvider {
    pool_stats: Arc<PoolStatistics>,
    refresh_interval: Duration,
}

impl DashboardProvider {
    /// Create a new dashboard provider
    pub fn new(pool_stats: Arc<PoolStatistics>, refresh_interval: Duration) -> Self {
        Self {
            pool_stats,
            refresh_interval,
        }
    }

    /// Get real-time dashboard data
    pub fn get_dashboard_data(&self) -> DashboardData {
        let stats = self.pool_stats.snapshot();

        DashboardData {
            timestamp: SystemTime::now(),
            active_connections: stats.active_connections,
            total_connections: stats.connections_created - stats.connections_destroyed,
            success_rate: stats.success_rate,
            average_wait_time: stats.average_acquire_time,
            leaks_detected: stats.leaks_detected,
            pool_efficiency: stats.efficiency_metrics.pool_utilization,
            queue_length: 0, // Would need queue reference
            peak_hour: stats.usage_patterns.peak_hour,
        }
    }

    /// Stream dashboard updates
    pub async fn stream_updates(&self) -> impl futures::Stream<Item = DashboardData> {
        let provider = self.clone();
        async_stream::stream! {
            let mut interval = tokio::time::interval(provider.refresh_interval);
            loop {
                interval.tick().await;
                yield provider.get_dashboard_data();
            }
        }
    }
}

impl Clone for DashboardProvider {
    fn clone(&self) -> Self {
        Self {
            pool_stats: Arc::clone(&self.pool_stats),
            refresh_interval: self.refresh_interval,
        }
    }
}

/// Dashboard data snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub timestamp: SystemTime,
    pub active_connections: u64,
    pub total_connections: u64,
    pub success_rate: f64,
    pub average_wait_time: Duration,
    pub leaks_detected: u64,
    pub pool_efficiency: f64,
    pub queue_length: usize,
    pub peak_hour: usize,
}

/// Leak detector
pub struct LeakDetector {
    threshold: Duration,
    check_interval: Duration,
    detected_leaks: Arc<RwLock<Vec<LeakInfo>>>,
}

impl LeakDetector {
    /// Create a new leak detector
    pub fn new(threshold: Duration, check_interval: Duration) -> Self {
        Self {
            threshold,
            check_interval,
            detected_leaks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Check for leaks
    pub fn check_leaks<C>(&self, active: &HashMap<u64>) {
        let now = Instant::now();
        let mut leaks = self.detected_leaks.write();

        for (&conn_id, &acquired_at) in active {
            let active_duration = now - acquired_at;
            if active_duration > self.threshold {
                let leak = LeakInfo {
                    connection_id: conn_id,
                    acquired_at,
                    active_duration,
                    detected_at: now,
                };

                // Only add if not already detected
                if !leaks.iter().any(|l| l.connection_id == conn_id) {
                    tracing::warn!("Connection leak detected: {:?}", leak);
                    leaks.push(leak);
                }
            }
        }
    }

    /// Get detected leaks
    pub fn get_leaks(&self) -> Vec<LeakInfo> {
        self.detected_leaks.read().clone()
    }

    /// Clear leak records
    pub fn clear_leaks(&self) {
        self.detected_leaks.write().clear();
    }
}

/// Leak information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakInfo {
    pub connection_id: u64,
    #[serde(skip, default = "Instant::now")]
    pub acquired_at: Instant,
    pub active_duration: Duration,
    #[serde(skip, default = "Instant::now")]
    pub detected_at: Instant,
}

/// Monitoring exporter for external systems
pub struct MonitoringExporter {
    stats: Arc<PoolStatistics>,
    export_format: ExportFormat,
}

impl MonitoringExporter {
    /// Create a new monitoring exporter
    pub fn new(stats: Arc<PoolStatistics>, format: ExportFormat) -> Self {
        Self {
            stats,
            export_format: format,
        }
    }

    /// Export metrics in configured format
    pub fn export(&self) -> String {
        let stats = self.stats.snapshot();

        match self.export_format {
            ExportFormat::Json => {
                serde_json::to_string_pretty(&stats).unwrap_or_default()
            }
            ExportFormat::Prometheus => {
                self.export_prometheus(&stats)
            }
            ExportFormat::Csv => {
                self.export_csv(&stats)
            }
        }
    }

    fn export_prometheus(&self, stats: &PoolStats) -> String {
        format!(
            "# HELP pool_connections_created Total connections created\n\
             # TYPE pool_connections_created counter\n\
             pool_connections_created {}\n\
             # HELP pool_connections_active Active connections\n\
             # TYPE pool_connections_active gauge\n\
             pool_connections_active {}\n\
             # HELP pool_acquire_success_rate Acquire success rate\n\
             # TYPE pool_acquire_success_rate gauge\n\
             pool_acquire_success_rate {}\n\
             # HELP pool_leaks_detected Detected connection leaks\n\
             # TYPE pool_leaks_detected counter\n\
             pool_leaks_detected {}\n",
            stats.connections_created,
            stats.active_connections,
            stats.success_rate,
            stats.leaks_detected
        )
    }

    fn export_csv(&self, stats: &PoolStats) -> String {
        format!(
            "metric,value\n\
             connections_created,{}\n\
             connections_active,{}\n\
             acquire_success_rate,{}\n\
             leaks_detected,{}\n\
             average_acquire_time_us,{}\n",
            stats.connections_created,
            stats.active_connections,
            stats.success_rate,
            stats.leaks_detected,
            stats.average_acquire_time.as_micros()
        )
    }
}

/// Export format for monitoring data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Prometheus,
    Csv,
}

// ============================================================================
// PUBLIC API FUNCTIONS FOR WEB MANAGEMENT INTERFACE
// ============================================================================

/// Public API for web management interface
pub mod api {
    use super::*;
    use std::time::UNIX_EPOCH;

    /// Get pool configuration
    pub fn get_pool_config<C: Send + Sync + 'static>(pool: &ConnectionPool<C>) -> PoolConfig {
        (*pool.config).clone()
    }

    /// Get pool statistics
    pub fn get_pool_statistics<C: Send + Sync + 'static>(pool: &ConnectionPool<C>) -> PoolStats {
        pool.statistics()
    }

    /// Get current pool size
    pub fn get_pool_size<C: Send + Sync + 'static>(pool: &ConnectionPool<C>) -> PoolSizeInfo {
        PoolSizeInfo {
            total: pool.size(),
            idle: pool.idle_count(),
            active: pool.active_count(),
        }
    }

    /// Get wait queue statistics
    pub fn get_queue_statistics(queue: &WaitQueue) -> QueueStats {
        queue.statistics()
    }

    /// Get all partition statistics
    pub fn get_partition_statistics<C: Send + Sync + 'static>(manager: &PartitionManager<C>) -> HashMap<String, PartitionStats> {
        manager.all_statistics()
    }

    /// List all partitions
    pub fn list_partitions<C: Send + Sync + 'static>(manager: &PartitionManager<C>) -> Vec<String> {
        manager.list_partitions()
    }

    /// Get dashboard data
    pub fn get_dashboard_data(provider: &DashboardProvider) -> DashboardData {
        provider.get_dashboard_data()
    }

    /// Get detected leaks
    pub fn get_detected_leaks(detector: &LeakDetector) -> Vec<LeakInfo> {
        detector.get_leaks()
    }

    /// Export metrics
    pub fn export_metrics(exporter: &MonitoringExporter) -> String {
        exporter.export()
    }
}

/// Pool size information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolSizeInfo {
    pub total: usize,
    pub idle: usize,
    pub active: usize,
}

#[cfg(test)]
mod tests {

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

    #[test]
    fn test_aging_policy() {
        // Test time-based aging
        let policy = AgingPolicy::TimeBased {
            max_lifetime: Duration::from_secs(60),
        };

        // Create a mock connection (would need actual implementation)
        // This is a placeholder for testing the policy logic
    }

    #[test]
    fn test_wait_queue() {
        let queue = WaitQueue::new(100, true);
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_pool_statistics() {
        let stats = PoolStatistics::new();
        stats.record_connection_created();
        stats.record_acquire_success(Duration::from_millis(10));

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.connections_created, 1);
        assert_eq!(snapshot.acquire_successes, 1);
    }
}
