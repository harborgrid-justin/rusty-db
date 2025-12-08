/// Load Balancing and Connection Management
///
/// This module provides intelligent load balancing across cluster nodes.
/// Features include:
/// - Connection pooling with automatic scaling
/// - Read replica selection with health awareness
/// - Latency-based routing for optimal performance
/// - Circuit breaker pattern for fault tolerance
/// - Sticky sessions for cache affinity
/// - Request rate limiting
///
/// Load balancing strategies:
/// - Round-robin
/// - Least connections
/// - Weighted round-robin
/// - Latency-based
/// - Locality-aware

use crate::error::DbError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::time::{Duration};
use tokio::sync::Semaphore;

/// Backend node identifier
pub type BackendId = String;

/// Connection identifier
pub type ConnectionId = u64;

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadBalanceStrategy {
    /// Round-robin across all nodes
    RoundRobin,
    /// Select node with least active connections
    LeastConnections,
    /// Weighted round-robin based on node capacity
    WeightedRoundRobin,
    /// Select node with lowest latency
    LatencyBased,
    /// Prefer local/nearby nodes
    LocalityAware,
    /// Random selection
    Random,
}

/// Backend node status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendStatus {
    /// Node is healthy and accepting connections
    Healthy,
    /// Node is degraded but still functional
    Degraded,
    /// Node is temporarily unavailable (circuit open)
    CircuitOpen,
    /// Node is permanently down
    Down,
}

/// Backend node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backend {
    /// Backend ID
    pub id: BackendId,
    /// Address
    pub address: String,
    /// Port
    pub port: u16,
    /// Node role (primary, replica, etc.)
    pub role: NodeRole,
    /// Status
    pub status: BackendStatus,
    /// Weight for weighted load balancing
    pub weight: u32,
    /// Current active connections
    pub active_connections: usize,
    /// Maximum connections
    pub max_connections: usize,
    /// Average latency (milliseconds)
    pub avg_latency_ms: f64,
    /// Request success rate (0.0 - 1.0)
    pub success_rate: f64,
    /// Last health check
    pub last_health_check: SystemTime,
    /// Geographic region
    pub region: String,
    /// Availability zone
    pub zone: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRole {
    Primary,
    Replica,
    ReadOnly,
    Standby,
}

impl Backend {
    pub fn new(id: BackendId, address: String, port: u16, role: NodeRole) -> Self {
        Self {
            id,
            address,
            port,
            role,
            status: BackendStatus::Healthy,
            weight: 100,
            active_connections: 0,
            max_connections: 1000,
            avg_latency_ms: 0.0,
            success_rate: 1.0,
            last_health_check: SystemTime::now(),
            region: "default".to_string(),
            zone: "default".to_string(),
        }
    }

    /// Check if backend can accept new connections
    pub fn can_accept_connection(&self) -> bool {
        self.status == BackendStatus::Healthy
            && self.active_connections < self.max_connections
    }

    /// Get utilization ratio
    pub fn utilization(&self) -> f64 {
        if self.max_connections == 0 {
            0.0
        } else {
            (self.active_connections as f64) / (self.max_connections as f64)
        }
    }
}

/// Connection pool for a backend
struct ConnectionPool {
    backend_id: BackendId,
    available: Arc<RwLock<VecDeque<Connection>>>,
    active: Arc<RwLock<HashMap<ConnectionId, Connection>>>,
    semaphore: Arc<Semaphore>,
    min_size: usize,
    max_size: usize,
}

/// Connection to a backend
#[derive(Debug, Clone)]
struct Connection {
    id: ConnectionId,
    backend_id: BackendId,
    created_at: SystemTime,
    last_used: SystemTime,
    request_count: usize,
}

impl Connection {
    fn new(id: ConnectionId, backend_id: BackendId) -> Self {
        Self {
            id,
            backend_id,
            created_at: SystemTime::now(),
            last_used: SystemTime::now(),
            request_count: 0,
        }
    }

    fn is_expired(&self, max_lifetime: Duration) -> bool {
        self.created_at.elapsed().unwrap_or(Duration::ZERO) > max_lifetime
    }
}

impl ConnectionPool {
    fn new(backend_id: BackendId, min_size: usize, max_size: usize) -> Self {
        Self {
            backend_id,
            available: Arc::new(RwLock::new(VecDeque::new())),
            active: Arc::new(RwLock::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(max_size)),
            min_size,
            max_size,
        }
    }

    async fn acquire(&self, next_conn_id: &mut ConnectionId) -> std::result::Result<Connection, DbError> {
        // Wait for available slot
        let _permit = self.semaphore.acquire().await
            .map_err(|_| DbError::Internal("Failed to acquire connection".into()))?;

        // Try to get from available pool
        let mut available = self.available.write().unwrap();
        if let Some(mut conn) = available.pop_front() {
            conn.last_used = SystemTime::now();
            let conn_id = conn.id;
            drop(available);
            self.active.write().unwrap().insert(conn_id, conn.clone());
            return Ok(conn);
        }
        drop(available);

        // Create new connection
        let conn = Connection::new(*next_conn_id, self.backend_id.clone());
        *next_conn_id += 1;
        self.active.write().unwrap().insert(conn.id, conn.clone());

        Ok(conn)
    }

    fn release(&self, conn_id: ConnectionId) {
        let mut active = self.active.write().unwrap();
        if let Some(conn) = active.remove(&conn_id) {
            drop(active);
            self.available.write().unwrap().push_back(conn);
        }
    }

    fn remove(&self, conn_id: ConnectionId) {
        self.active.write().unwrap().remove(&conn_id);
    }

    fn active_count(&self) -> usize {
        self.active.read().unwrap().len()
    }
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failures detected, rejecting requests
    HalfOpen, // Testing if backend recovered
}

/// Circuit breaker for a backend
struct CircuitBreaker {
    state: RwLock<CircuitState>,
    failure_count: RwLock<usize>,
    last_failure: RwLock<Option<SystemTime>>,
    failure_threshold: usize,
    timeout: Duration,
    half_open_requests: RwLock<usize>,
    half_open_max: usize,
}

impl CircuitBreaker {
    fn new(failure_threshold: usize, timeout: Duration) -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed),
            failure_count: RwLock::new(0),
            last_failure: RwLock::new(None),
            failure_threshold,
            timeout,
            half_open_requests: RwLock::new(0),
            half_open_max: 3,
        }
    }

    fn can_attempt(&self) -> bool {
        let _state = *self.state.read().unwrap();

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout elapsed
                if let Some(last_failure) = *self.last_failure.read().unwrap() {
                    if last_failure.elapsed().unwrap_or(Duration::ZERO) > self.timeout {
                        // Transition to half-open
                        *self.state.write().unwrap() = CircuitState::HalfOpen;
                        *self.half_open_requests.write().unwrap() = 0;
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => {
                let mut half_open = self.half_open_requests.write().unwrap();
                if *half_open < self.half_open_max {
                    *half_open += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn record_success(&self) {
        let _state = *self.state.read().unwrap();

        match state {
            CircuitState::Closed => {
                *self.failure_count.write().unwrap() = 0;
            }
            CircuitState::HalfOpen => {
                // Success in half-open state - close circuit
                *self.state.write().unwrap() = CircuitState::Closed;
                *self.failure_count.write().unwrap() = 0;
            }
            _ => {}
        }
    }

    fn record_failure(&self) {
        let mut failures = self.failure_count.write().unwrap();
        *failures += 1;

        if *failures >= self.failure_threshold {
            *self.state.write().unwrap() = CircuitState::Open;
            *self.last_failure.write().unwrap() = Some(SystemTime::now());
        }
    }

    fn get_state(&self) -> CircuitState {
        *self.state.read().unwrap()
    }
}

/// Load balancer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    /// Load balancing strategy
    pub strategy: LoadBalanceStrategy,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Health check timeout
    pub health_check_timeout: Duration,
    /// Connection pool min size
    pub pool_min_size: usize,
    /// Connection pool max size
    pub pool_max_size: usize,
    /// Connection max lifetime
    pub connection_max_lifetime: Duration,
    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: usize,
    /// Circuit breaker timeout
    pub circuit_breaker_timeout: Duration,
    /// Enable sticky sessions
    pub enable_sticky_sessions: bool,
    /// Session timeout
    pub session_timeout: Duration,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalanceStrategy::LeastConnections,
            health_check_interval: Duration::from_secs(10),
            health_check_timeout: Duration::from_secs(5),
            pool_min_size: 10,
            pool_max_size: 100,
            connection_max_lifetime: Duration::from_secs(3600),
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(60),
            enable_sticky_sessions: false,
            session_timeout: Duration::from_secs(300),
        }
    }
}

/// Session affinity tracking
struct Session {
    session_id: String,
    backend_id: BackendId,
    created_at: SystemTime,
    last_accessed: SystemTime,
}

impl Session {
    fn is_expired(&self, timeout: Duration) -> bool {
        self.last_accessed.elapsed().unwrap_or(Duration::ZERO) > timeout
    }
}

/// Main load balancer
pub struct LoadBalancer {
    /// Configuration
    config: LoadBalancerConfig,
    /// All backend nodes
    backends: Arc<RwLock<HashMap<BackendId, Backend>>>,
    /// Connection pools
    pools: Arc<RwLock<HashMap<BackendId, ConnectionPool>>>,
    /// Circuit breakers
    circuit_breakers: Arc<RwLock<HashMap<BackendId, CircuitBreaker>>>,
    /// Round-robin counter
    round_robin_index: Arc<RwLock<usize>>,
    /// Next connection ID
    next_conn_id: Arc<RwLock<ConnectionId>>,
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl LoadBalancer {
    pub fn new(config: LoadBalancerConfig) -> Self {
        Self {
            config,
            backends: Arc::new(RwLock::new(HashMap::new())),
            pools: Arc::new(RwLock::new(HashMap::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            round_robin_index: Arc::new(RwLock::new(0)),
            next_conn_id: Arc::new(RwLock::new(0)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a backend to the load balancer
    pub fn add_backend(&self, backend: Backend) -> std::result::Result<(), DbError> {
        let backend_id = backend.id.clone();

        // Add backend
        self.backends.write().unwrap().insert(backend_id.clone(), backend);

        // Create connection pool
        let pool = ConnectionPool::new(
            backend_id.clone(),
            self.config.pool_min_size,
            self.config.pool_max_size,
        );
        self.pools.write().unwrap().insert(backend_id.clone(), pool);

        // Create circuit breaker
        let breaker = CircuitBreaker::new(
            self.config.circuit_breaker_threshold,
            self.config.circuit_breaker_timeout,
        );
        self.circuit_breakers.write().unwrap().insert(backend_id, breaker);

        Ok(())
    }

    /// Remove a backend
    pub fn remove_backend(&self, backend_id: &str) -> std::result::Result<(), DbError> {
        self.backends.write().unwrap().remove(backend_id);
        self.pools.write().unwrap().remove(backend_id);
        self.circuit_breakers.write().unwrap().remove(backend_id);
        Ok(())
    }

    /// Select backend based on strategy
    pub fn select_backend(&self, session_id: Option<&str>) -> std::result::Result<BackendId, DbError> {
        // Check for session affinity
        if self.config.enable_sticky_sessions {
            if let Some(sid) = session_id {
                let sessions = self.sessions.read().unwrap();
                if let Some(session) = sessions.get(sid) {
                    if !session.is_expired(self.config.session_timeout) {
                        return Ok(session.backend_id.clone());
                    }
                }
            }
        }

        // Select based on strategy
        let backend_id = match self.config.strategy {
            LoadBalanceStrategy::RoundRobin => self.select_round_robin()?,
            LoadBalanceStrategy::LeastConnections => self.select_least_connections()?,
            LoadBalanceStrategy::WeightedRoundRobin => self.select_weighted_round_robin()?,
            LoadBalanceStrategy::LatencyBased => self.select_latency_based()?,
            LoadBalanceStrategy::LocalityAware => self.select_locality_aware()?,
            LoadBalanceStrategy::Random => self.select_random()?,
        };

        // Create session if enabled
        if self.config.enable_sticky_sessions {
            if let Some(sid) = session_id {
                let session = Session {
                    session_id: sid.to_string(),
                    backend_id: backend_id.clone(),
                    created_at: SystemTime::now(),
                    last_accessed: SystemTime::now(),
                };
                self.sessions.write().unwrap().insert(sid.to_string(), session);
            }
        }

        Ok(backend_id)
    }

    /// Round-robin selection
    fn select_round_robin(&self) -> std::result::Result<BackendId, DbError> {
        let backends = self.backends.read().unwrap();
        let available: Vec<_> = backends
            .values()
            .filter(|b| self.is_backend_available(&b.id))
            .collect();

        if available.is_empty() {
            return Err(DbError::Internal("No available backends".into()));
        }

        let mut index = self.round_robin_index.write().unwrap();
        let selected = &available[*index % available.len()];
        *index = (*index + 1) % available.len();

        Ok(selected.id.clone())
    }

    /// Least connections selection
    fn select_least_connections(&self) -> std::result::Result<BackendId, DbError> {
        let backends = self.backends.read().unwrap();
        let pools = self.pools.read().unwrap();

        let selected = backends
            .values()
            .filter(|b| self.is_backend_available(&b.id))
            .min_by_key(|b| {
                pools.get(&b.id).map(|p| p.active_count()).unwrap_or(usize::MAX)
            })
            .ok_or_else(|| DbError::Internal("No available backends".into()))?;

        Ok(selected.id.clone())
    }

    /// Weighted round-robin selection
    fn select_weighted_round_robin(&self) -> std::result::Result<BackendId, DbError> {
        let backends = self.backends.read().unwrap();
        let available: Vec<_> = backends
            .values()
            .filter(|b| self.is_backend_available(&b.id))
            .collect();

        if available.is_empty() {
            return Err(DbError::Internal("No available backends".into()));
        }

        // Calculate total weight
        let total_weight: u32 = available.iter().map(|b| b.weight).sum();

        // Generate weighted random selection
        let mut rng = rand::thread_rng();
        let mut random = rand::Rng::gen_range(&mut rng, 0..total_weight);

        for backend in &available {
            if random < backend.weight {
                return Ok(backend.id.clone());
            }
            random -= backend.weight;
        }

        Ok(available[0].id.clone())
    }

    /// Latency-based selection
    fn select_latency_based(&self) -> std::result::Result<BackendId, DbError> {
        let backends = self.backends.read().unwrap();

        let selected = backends
            .values()
            .filter(|b| self.is_backend_available(&b.id))
            .min_by(|a, b| {
                a.avg_latency_ms
                    .partial_cmp(&b.avg_latency_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| DbError::Internal("No available backends".into()))?;

        Ok(selected.id.clone())
    }

    /// Locality-aware selection
    fn select_locality_aware(&self) -> std::result::Result<BackendId, DbError> {
        let backends = self.backends.read().unwrap();

        // Prefer local region, then zone
        let local_region = "default"; // Would get from config
        let local_zone = "default";

        let selected = backends
            .values()
            .filter(|b| self.is_backend_available(&b.id))
            .max_by_key(|b| {
                let mut score = 0;
                if b.region == local_region {
                    score += 100;
                }
                if b.zone == local_zone {
                    score += 10;
                }
                score
            })
            .ok_or_else(|| DbError::Internal("No available backends".into()))?;

        Ok(selected.id.clone())
    }

    /// Random selection
    fn select_random(&self) -> std::result::Result<BackendId, DbError> {
        use rand::seq::SliceRandom;

        let backends = self.backends.read().unwrap();
        let available: Vec<_> = backends
            .values()
            .filter(|b| self.is_backend_available(&b.id))
            .collect();

        if available.is_empty() {
            return Err(DbError::Internal("No available backends".into()));
        }

        let mut rng = rand::thread_rng();
        let selected = available.choose(&mut rng).unwrap();

        Ok(selected.id.clone())
    }

    /// Check if backend is available
    fn is_backend_available(&self, backend_id: &str) -> bool {
        let backends = self.backends.read().unwrap();
        let circuit_breakers = self.circuit_breakers.read().unwrap();

        if let Some(backend) = backends.get(backend_id) {
            if !backend.can_accept_connection() {
                return false;
            }

            if let Some(breaker) = circuit_breakers.get(backend_id) {
                return breaker.can_attempt();
            }
        }

        false
    }

    /// Acquire a connection
    pub async fn acquire_connection(&self, session_id: Option<&str>) -> std::result::Result<(BackendId, ConnectionId), DbError> {
        let backend_id = self.select_backend(session_id)?;

        let pools = self.pools.read().unwrap();
        let pool = pools.get(&backend_id)
            .ok_or_else(|| DbError::Internal("Pool not found".into()))?;

        let mut next_id = self.next_conn_id.write().unwrap();
        let conn = pool.acquire(&mut next_id).await?;

        // Update backend active connections
        let mut backends = self.backends.write().unwrap();
        if let Some(backend) = backends.get_mut(&backend_id) {
            backend.active_connections += 1;
        }

        Ok((backend_id, conn.id))
    }

    /// Release a connection
    pub fn release_connection(&self, backend_id: &str, conn_id: ConnectionId) {
        let pools = self.pools.read().unwrap();
        if let Some(pool) = pools.get(backend_id) {
            pool.release(conn_id);

            // Update backend active connections
            let mut backends = self.backends.write().unwrap();
            if let Some(backend) = backends.get_mut(backend_id) {
                backend.active_connections = backend.active_connections.saturating_sub(1);
            }
        }
    }

    /// Record request result for circuit breaker and metrics
    pub fn record_request_result(&self, backend_id: &str, success: bool, latency_ms: f64) {
        // Update circuit breaker
        let circuit_breakers = self.circuit_breakers.read().unwrap();
        if let Some(breaker) = circuit_breakers.get(backend_id) {
            if success {
                breaker.record_success();
            } else {
                breaker.record_failure();
            }
        }

        // Update backend metrics
        let mut backends = self.backends.write().unwrap();
        if let Some(backend) = backends.get_mut(backend_id) {
            // Update latency (exponential moving average)
            let alpha = 0.3;
            backend.avg_latency_ms = alpha * latency_ms + (1.0 - alpha) * backend.avg_latency_ms;

            // Update success rate
            backend.success_rate = alpha * (if success { 1.0 } else { 0.0 })
                + (1.0 - alpha) * backend.success_rate;

            // Update status based on circuit breaker
            if let Some(breaker) = circuit_breakers.get(backend_id) {
                backend.status = match breaker.get_state() {
                    CircuitState::Closed => BackendStatus::Healthy,
                    CircuitState::HalfOpen => BackendStatus::Degraded,
                    CircuitState::Open => BackendStatus::CircuitOpen,
                };
            }
        }
    }

    /// Get all backends
    pub fn get_backends(&self) -> Vec<Backend> {
        self.backends.read().unwrap().values().cloned().collect()
    }

    /// Get backend by ID
    pub fn get_backend(&self, backend_id: &str) -> Option<Backend> {
        self.backends.read().unwrap().get(backend_id).cloned()
    }

    /// Clean up expired sessions
    pub fn cleanup_sessions(&self) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.retain(|_, session| !session.is_expired(self.config.session_timeout));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let backend = Backend::new("node1".into(), "127.0.0.1".into(), 8080, NodeRole::Primary);
        assert_eq!(backend.id, "node1");
        assert!(backend.can_accept_connection());
    }

    #[test]
    fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(10));
        assert!(breaker.can_attempt());

        // Record failures
        breaker.record_failure();
        breaker.record_failure();
        breaker.record_failure();

        // Should be open now
        assert_eq!(breaker.get_state(), CircuitState::Open);
        assert!(!breaker.can_attempt());
    }

    #[test]
    fn test_load_balancer_creation() {
        let config = LoadBalancerConfig::default();
        let lb = LoadBalancer::new(config);

        let backend = Backend::new("node1".into(), "127.0.0.1".into(), 8080, NodeRole::Primary);
        lb.add_backend(backend).unwrap();

        assert_eq!(lb.get_backends().len(), 1);
    }
}


