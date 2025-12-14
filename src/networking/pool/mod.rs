// # Connection Pool Module for Distributed Nodes
//
// Enterprise-grade connection pooling and multiplexing for node-to-node communication
// in RustyDB's distributed architecture. This module provides efficient resource management,
// connection reuse, and stream multiplexing for cluster operations.
//
// ## Features
//
// - **Connection Pooling**: Per-node connection pools with configurable limits
// - **Stream Multiplexing**: Multiple logical streams over single connections (yamux-style)
// - **Resource Management**: Automatic scaling, eviction, and warmup
// - **Flow Control**: Per-stream backpressure and priority scheduling
// - **Metrics & Monitoring**: Comprehensive pool statistics and health monitoring
//
// ## Architecture
//
// ```text
//                    ┌─────────────────┐
//                    │  PoolManager    │
//                    │  (All Nodes)    │
//                    └────────┬────────┘
//                             │
//              ┌──────────────┼──────────────┐
//              │              │              │
//        ┌─────▼─────┐  ┌────▼──────┐  ┌───▼──────┐
//        │ NodePool  │  │ NodePool  │  │ NodePool │
//        │ (Node A)  │  │ (Node B)  │  │ (Node C) │
//        └─────┬─────┘  └────┬──────┘  └───┬──────┘
//              │              │              │
//        ┌─────▼─────┐  ┌────▼──────┐  ┌───▼──────┐
//        │Multiplexed│  │Multiplexed│  │Multiplexed│
//        │Connection │  │Connection │  │Connection │
//        └───────────┘  └───────────┘  └───────────┘
//              │              │              │
//        [Stream 1-N]   [Stream 1-N]   [Stream 1-N]
// ```
//
// ## Usage Example
//
// ```rust,no_run
// use rusty_db::networking::pool::{PoolManager, PoolConfig};
// use std::time::Duration;
//
// #[tokio::main]
// async fn main() -> rusty_db::error::Result<()> {
//     let config = PoolConfig {
//         min_connections: 2,
//         max_connections: 10,
//         idle_timeout: Duration::from_secs(300),
//         max_lifetime: Duration::from_secs(3600),
//         acquire_timeout: Duration::from_secs(5),
//         enable_multiplexing: true,
//         max_streams_per_connection: 100,
//         warmup_connections: 3,
//     };
//
//     let manager = PoolManager::new(config);
//
//     // Acquire connection to a specific node
//     let conn = manager.acquire("node-1").await?;
//
//     // Open a multiplexed stream
//     let stream = conn.open_stream().await?;
//
//     // Use the stream for communication
//     // stream.send(message).await?;
//
//     Ok(())
// }
// ```

use crate::common::NodeId;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub mod channel;
pub mod eviction;
pub mod manager;
pub mod metrics;
pub mod multiplexing;
pub mod node_pool;
pub mod warmup;

pub use channel::{ChannelPool, ChannelRequest, RequestChannel};
pub use eviction::{EvictionManager, EvictionPolicy};
pub use manager::{PoolManager, PooledConnection};
pub use metrics::{ConnectionMetrics, PoolMetrics, StreamMetrics};
pub use multiplexing::{MultiplexedConnection, Stream, StreamId, StreamPriority};
pub use node_pool::{ConnectionState, NodeConnection, NodePool};
pub use warmup::{WarmupManager, WarmupStrategy};

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for connection pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum number of connections to maintain per node
    pub min_connections: usize,

    /// Maximum number of connections allowed per node
    pub max_connections: usize,

    /// Duration after which idle connections are evicted
    pub idle_timeout: Duration,

    /// Maximum lifetime of a connection before forced recycling
    pub max_lifetime: Duration,

    /// Timeout for acquiring a connection from the pool
    pub acquire_timeout: Duration,

    /// Enable stream multiplexing over connections
    pub enable_multiplexing: bool,

    /// Maximum number of concurrent streams per multiplexed connection
    pub max_streams_per_connection: usize,

    /// Number of connections to warm up eagerly on node discovery
    pub warmup_connections: usize,

    /// Enable background connection health checks
    pub enable_health_checks: bool,

    /// Interval for background health checks
    pub health_check_interval: Duration,

    /// Enable automatic connection scaling
    pub enable_auto_scaling: bool,

    /// Threshold for triggering scale-up (utilization percentage)
    pub scale_up_threshold: f64,

    /// Threshold for triggering scale-down (utilization percentage)
    pub scale_down_threshold: f64,

    /// Enable connection reuse (keep-alive)
    pub enable_keep_alive: bool,

    /// TCP keep-alive interval
    pub keep_alive_interval: Duration,

    /// Maximum number of retries for failed connection attempts
    pub max_retry_attempts: usize,

    /// Backoff duration between retry attempts
    pub retry_backoff: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 2,
            max_connections: 20,
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(3600),
            acquire_timeout: Duration::from_secs(5),
            enable_multiplexing: true,
            max_streams_per_connection: 100,
            warmup_connections: 3,
            enable_health_checks: true,
            health_check_interval: Duration::from_secs(30),
            enable_auto_scaling: true,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.2,
            enable_keep_alive: true,
            keep_alive_interval: Duration::from_secs(60),
            max_retry_attempts: 3,
            retry_backoff: Duration::from_millis(100),
        }
    }
}

impl PoolConfig {
    /// Validate configuration parameters
    pub fn validate(&self) -> Result<()> {
        if self.min_connections > self.max_connections {
            return Err(crate::error::DbError::Configuration(
                "min_connections cannot exceed max_connections".to_string(),
            ));
        }

        if self.warmup_connections > self.max_connections {
            return Err(crate::error::DbError::Configuration(
                "warmup_connections cannot exceed max_connections".to_string(),
            ));
        }

        if self.scale_up_threshold <= self.scale_down_threshold {
            return Err(crate::error::DbError::Configuration(
                "scale_up_threshold must be greater than scale_down_threshold".to_string(),
            ));
        }

        if self.max_streams_per_connection == 0 && self.enable_multiplexing {
            return Err(crate::error::DbError::Configuration(
                "max_streams_per_connection must be > 0 when multiplexing is enabled".to_string(),
            ));
        }

        Ok(())
    }

    /// Create a builder for PoolConfig
    pub fn builder() -> PoolConfigBuilder {
        PoolConfigBuilder::default()
    }
}

/// Builder for PoolConfig
#[derive(Debug, Default)]
pub struct PoolConfigBuilder {
    config: PoolConfig,
}

impl PoolConfigBuilder {
    /// Set minimum connections
    pub fn min_connections(mut self, min: usize) -> Self {
        self.config.min_connections = min;
        self
    }

    /// Set maximum connections
    pub fn max_connections(mut self, max: usize) -> Self {
        self.config.max_connections = max;
        self
    }

    /// Set idle timeout
    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.config.idle_timeout = timeout;
        self
    }

    /// Set maximum lifetime
    pub fn max_lifetime(mut self, lifetime: Duration) -> Self {
        self.config.max_lifetime = lifetime;
        self
    }

    /// Set acquire timeout
    pub fn acquire_timeout(mut self, timeout: Duration) -> Self {
        self.config.acquire_timeout = timeout;
        self
    }

    /// Enable or disable multiplexing
    pub fn enable_multiplexing(mut self, enable: bool) -> Self {
        self.config.enable_multiplexing = enable;
        self
    }

    /// Set maximum streams per connection
    pub fn max_streams_per_connection(mut self, max: usize) -> Self {
        self.config.max_streams_per_connection = max;
        self
    }

    /// Set warmup connections count
    pub fn warmup_connections(mut self, count: usize) -> Self {
        self.config.warmup_connections = count;
        self
    }

    /// Build the configuration
    pub fn build(self) -> Result<PoolConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

// ============================================================================
// Pool Events
// ============================================================================

/// Events emitted by the connection pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PoolEvent {
    /// Connection created to a node
    ConnectionCreated { node_id: NodeId, connection_id: u64 },

    /// Connection closed
    ConnectionClosed {
        node_id: NodeId,
        connection_id: u64,
        reason: String,
    },

    /// Connection acquired from pool
    ConnectionAcquired {
        node_id: NodeId,
        connection_id: u64,
        wait_time_ms: u64,
    },

    /// Connection returned to pool
    ConnectionReleased {
        node_id: NodeId,
        connection_id: u64,
        usage_duration_ms: u64,
    },

    /// Pool scaled up
    PoolScaledUp {
        node_id: NodeId,
        old_size: usize,
        new_size: usize,
    },

    /// Pool scaled down
    PoolScaledDown {
        node_id: NodeId,
        old_size: usize,
        new_size: usize,
    },

    /// Connection health check failed
    HealthCheckFailed {
        node_id: NodeId,
        connection_id: u64,
        error: String,
    },

    /// Stream opened on multiplexed connection
    StreamOpened {
        node_id: NodeId,
        connection_id: u64,
        stream_id: u32,
    },

    /// Stream closed
    StreamClosed {
        node_id: NodeId,
        connection_id: u64,
        stream_id: u32,
        bytes_sent: u64,
        bytes_received: u64,
    },

    /// Pool reached maximum capacity
    PoolExhausted {
        node_id: NodeId,
        pending_requests: usize,
    },
}

/// Listener for pool events
pub trait PoolEventListener: Send + Sync {
    /// Handle a pool event
    fn on_event(&self, event: PoolEvent);
}

// ============================================================================
// Connection Trait
// ============================================================================

/// Trait for network connections used by the pool
#[async_trait::async_trait]
pub trait Connection: Send + Sync {
    /// Check if the connection is healthy
    async fn is_healthy(&self) -> bool;

    /// Close the connection
    async fn close(&mut self) -> Result<()>;

    /// Get the connection's unique identifier
    fn connection_id(&self) -> u64;

    /// Get the remote node ID
    fn node_id(&self) -> &NodeId;

    /// Get connection statistics
    fn stats(&self) -> ConnectionStats;
}

/// Statistics for a single connection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionStats {
    /// Total bytes sent
    pub bytes_sent: u64,

    /// Total bytes received
    pub bytes_received: u64,

    /// Number of requests processed
    pub requests_processed: u64,

    /// Number of errors encountered
    pub errors: u64,

    /// Connection uptime in seconds
    pub uptime_secs: u64,

    /// Number of active streams (for multiplexed connections)
    pub active_streams: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_validation() {
        let mut config = PoolConfig::default();
        assert!(config.validate().is_ok());

        // Test min > max
        config.min_connections = 50;
        config.max_connections = 20;
        assert!(config.validate().is_err());

        // Test warmup > max
        config.min_connections = 2;
        config.max_connections = 20;
        config.warmup_connections = 50;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_pool_config_builder() {
        let config = PoolConfig::builder()
            .min_connections(5)
            .max_connections(50)
            .idle_timeout(Duration::from_secs(600))
            .build();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.max_connections, 50);
    }

    #[test]
    fn test_pool_config_builder_validation() {
        let config = PoolConfig::builder()
            .min_connections(100)
            .max_connections(50)
            .build();

        assert!(config.is_err());
    }
}
