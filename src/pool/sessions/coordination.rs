//! Session pool coordination module
use serde::{Serialize, Deserialize};
use std::time::Duration;

pub struct SessionPool;
pub struct SessionTag;
pub struct SessionAffinity;
pub struct PooledSession;
pub struct SessionSelector;

/// Pool configuration for session pooling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub min_pool_size: usize,
    pub max_pool_size: usize,
    pub initial_pool_size: usize,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Option<Duration>,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_pool_size: 10,
            max_pool_size: 100,
            initial_pool_size: 20,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Some(Duration::from_secs(3600)),
        }
    }
}

/// Pool statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatistics {
    pub active_sessions: usize,
    pub idle_sessions: usize,
    pub total_sessions: usize,
    pub sessions_created: u64,
    pub sessions_closed: u64,
    pub sessions_acquired: u64,
    pub sessions_released: u64,
    pub acquire_failures: u64,
    pub acquire_timeouts: u64,
    pub average_acquire_time: Duration,
}
