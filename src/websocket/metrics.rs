// WebSocket Metrics Module
//
// Comprehensive metrics collection for WebSocket connections including:
// - Connection tracking (active, total)
// - Message throughput (sent, received)
// - Data volume (bytes sent/received)
// - Latency tracking (connection duration, message latency)
// - Error tracking by type
// - Subscription management

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

// ============================================================================
// WebSocket Metrics Core Structures
// ============================================================================

/// Main WebSocket metrics aggregator
#[derive(Debug)]
pub struct WebSocketMetrics {
    // Connection metrics
    active_connections: AtomicUsize,
    total_connections: AtomicU64,
    total_disconnections: AtomicU64,

    // Message metrics
    messages_sent: AtomicU64,
    messages_received: AtomicU64,

    // Data volume metrics
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,

    // Error metrics by type
    errors_by_type: Arc<RwLock<HashMap<ErrorType, u64>>>,

    // Subscription metrics
    subscriptions_active: AtomicUsize,
    subscriptions_total: AtomicU64,

    // Latency tracking
    connection_durations: Arc<RwLock<Vec<Duration>>>,
    message_latencies: Arc<RwLock<Vec<Duration>>>,

    // Performance metrics
    message_queue_depth: AtomicUsize,
    backpressure_events: AtomicU64,

    // Connection tracking
    connections: Arc<RwLock<HashMap<ConnectionId, ConnectionMetrics>>>,

    // Configuration
    config: MetricsConfig,
}

/// Configuration for WebSocket metrics collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Maximum number of latency samples to keep
    pub max_latency_samples: usize,

    /// Enable detailed per-connection tracking
    pub track_per_connection: bool,

    /// Enable message payload size histograms
    pub track_message_sizes: bool,

    /// Maximum number of connections to track individually
    pub max_tracked_connections: usize,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            max_latency_samples: 10_000,
            track_per_connection: true,
            track_message_sizes: true,
            max_tracked_connections: 1_000,
        }
    }
}

/// Unique identifier for WebSocket connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub u64);

/// WebSocket error types for categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorType {
    ConnectionFailed,
    HandshakeFailed,
    ProtocolError,
    MessageParseError,
    AuthenticationFailed,
    AuthorizationFailed,
    Timeout,
    NetworkError,
    InternalError,
    RateLimitExceeded,
    InvalidMessage,
    Other,
}

impl ErrorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorType::ConnectionFailed => "connection_failed",
            ErrorType::HandshakeFailed => "handshake_failed",
            ErrorType::ProtocolError => "protocol_error",
            ErrorType::MessageParseError => "message_parse_error",
            ErrorType::AuthenticationFailed => "authentication_failed",
            ErrorType::AuthorizationFailed => "authorization_failed",
            ErrorType::Timeout => "timeout",
            ErrorType::NetworkError => "network_error",
            ErrorType::InternalError => "internal_error",
            ErrorType::RateLimitExceeded => "rate_limit_exceeded",
            ErrorType::InvalidMessage => "invalid_message",
            ErrorType::Other => "other",
        }
    }
}

/// Per-connection metrics tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetrics {
    pub id: ConnectionId,
    pub connected_at: SystemTime,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub errors: u64,
    pub subscriptions: Vec<String>,
    pub last_activity: SystemTime,
    pub remote_addr: String,
    pub user_agent: Option<String>,
}

/// Snapshot of current WebSocket metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: SystemTime,
    pub active_connections: usize,
    pub total_connections: u64,
    pub total_disconnections: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub errors_by_type: HashMap<String, u64>,
    pub subscriptions_active: usize,
    pub subscriptions_total: u64,
    pub message_queue_depth: usize,
    pub backpressure_events: u64,
    pub avg_connection_duration: Option<Duration>,
    pub avg_message_latency: Option<Duration>,
    pub p50_message_latency: Option<Duration>,
    pub p95_message_latency: Option<Duration>,
    pub p99_message_latency: Option<Duration>,
}

// ============================================================================
// WebSocketMetrics Implementation
// ============================================================================

impl WebSocketMetrics {
    /// Create a new WebSocket metrics collector with default configuration
    pub fn new() -> Self {
        Self::with_config(MetricsConfig::default())
    }

    /// Create a new WebSocket metrics collector with custom configuration
    pub fn with_config(config: MetricsConfig) -> Self {
        Self {
            active_connections: AtomicUsize::new(0),
            total_connections: AtomicU64::new(0),
            total_disconnections: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            errors_by_type: Arc::new(RwLock::new(HashMap::new())),
            subscriptions_active: AtomicUsize::new(0),
            subscriptions_total: AtomicU64::new(0),
            connection_durations: Arc::new(RwLock::new(Vec::new())),
            message_latencies: Arc::new(RwLock::new(Vec::new())),
            message_queue_depth: AtomicUsize::new(0),
            backpressure_events: AtomicU64::new(0),
            connections: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    // ========================================================================
    // Connection Tracking
    // ========================================================================

    /// Record a new connection
    pub fn connection_opened(
        &self,
        id: ConnectionId,
        remote_addr: String,
        user_agent: Option<String>,
    ) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        self.total_connections.fetch_add(1, Ordering::Relaxed);

        if self.config.track_per_connection {
            let mut connections = self.connections.write();
            if connections.len() < self.config.max_tracked_connections {
                connections.insert(
                    id,
                    ConnectionMetrics {
                        id,
                        connected_at: SystemTime::now(),
                        messages_sent: 0,
                        messages_received: 0,
                        bytes_sent: 0,
                        bytes_received: 0,
                        errors: 0,
                        subscriptions: Vec::new(),
                        last_activity: SystemTime::now(),
                        remote_addr,
                        user_agent,
                    },
                );
            }
        }
    }

    /// Record a connection closure
    pub fn connection_closed(&self, id: ConnectionId, duration: Duration) {
        let active = self.active_connections.fetch_sub(1, Ordering::Relaxed);
        if active == 0 {
            // Prevent underflow
            self.active_connections.fetch_add(1, Ordering::Relaxed);
        } else {
            self.total_disconnections.fetch_add(1, Ordering::Relaxed);
        }

        // Record connection duration
        let mut durations = self.connection_durations.write();
        durations.push(duration);
        if durations.len() > self.config.max_latency_samples {
            durations.remove(0);
        }

        // Remove from tracking
        if self.config.track_per_connection {
            self.connections.write().remove(&id);
        }
    }

    /// Get the current number of active connections
    pub fn active_connections(&self) -> usize {
        self.active_connections.load(Ordering::Relaxed)
    }

    /// Get the total number of connections since startup
    pub fn total_connections(&self) -> u64 {
        self.total_connections.load(Ordering::Relaxed)
    }

    // ========================================================================
    // Message Tracking
    // ========================================================================

    /// Record a message sent to a client
    pub fn message_sent(&self, id: ConnectionId, size_bytes: usize) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent
            .fetch_add(size_bytes as u64, Ordering::Relaxed);

        if self.config.track_per_connection {
            if let Some(conn) = self.connections.write().get_mut(&id) {
                conn.messages_sent += 1;
                conn.bytes_sent += size_bytes as u64;
                conn.last_activity = SystemTime::now();
            }
        }
    }

    /// Record a message received from a client
    pub fn message_received(&self, id: ConnectionId, size_bytes: usize) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received
            .fetch_add(size_bytes as u64, Ordering::Relaxed);

        if self.config.track_per_connection {
            if let Some(conn) = self.connections.write().get_mut(&id) {
                conn.messages_received += 1;
                conn.bytes_received += size_bytes as u64;
                conn.last_activity = SystemTime::now();
            }
        }
    }

    /// Record message latency (time from send to acknowledgment)
    pub fn record_message_latency(&self, latency: Duration) {
        let mut latencies = self.message_latencies.write();
        latencies.push(latency);
        if latencies.len() > self.config.max_latency_samples {
            latencies.remove(0);
        }
    }

    // ========================================================================
    // Error Tracking
    // ========================================================================

    /// Record an error occurrence
    pub fn record_error(&self, id: Option<ConnectionId>, error_type: ErrorType) {
        let mut errors = self.errors_by_type.write();
        *errors.entry(error_type).or_insert(0) += 1;

        if let Some(conn_id) = id {
            if self.config.track_per_connection {
                if let Some(conn) = self.connections.write().get_mut(&conn_id) {
                    conn.errors += 1;
                }
            }
        }
    }

    /// Get error count by type
    pub fn errors_by_type(&self) -> HashMap<ErrorType, u64> {
        self.errors_by_type.read().clone()
    }

    // ========================================================================
    // Subscription Tracking
    // ========================================================================

    /// Record a new subscription
    pub fn subscription_added(&self, id: ConnectionId, topic: String) {
        self.subscriptions_active.fetch_add(1, Ordering::Relaxed);
        self.subscriptions_total.fetch_add(1, Ordering::Relaxed);

        if self.config.track_per_connection {
            if let Some(conn) = self.connections.write().get_mut(&id) {
                conn.subscriptions.push(topic);
            }
        }
    }

    /// Record subscription removal
    pub fn subscription_removed(&self, id: ConnectionId, topic: &str) {
        let active = self.subscriptions_active.fetch_sub(1, Ordering::Relaxed);
        if active == 0 {
            self.subscriptions_active.fetch_add(1, Ordering::Relaxed);
        }

        if self.config.track_per_connection {
            if let Some(conn) = self.connections.write().get_mut(&id) {
                conn.subscriptions.retain(|t| t != topic);
            }
        }
    }

    /// Get the current number of active subscriptions
    pub fn active_subscriptions(&self) -> usize {
        self.subscriptions_active.load(Ordering::Relaxed)
    }

    // ========================================================================
    // Performance Metrics
    // ========================================================================

    /// Update message queue depth
    pub fn set_queue_depth(&self, depth: usize) {
        self.message_queue_depth.store(depth, Ordering::Relaxed);
    }

    /// Record a backpressure event
    pub fn record_backpressure(&self) {
        self.backpressure_events.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current message queue depth
    pub fn queue_depth(&self) -> usize {
        self.message_queue_depth.load(Ordering::Relaxed)
    }

    // ========================================================================
    // Statistics and Snapshots
    // ========================================================================

    /// Calculate percentile from sorted data
    fn calculate_percentile(sorted_data: &[Duration], percentile: f64) -> Option<Duration> {
        if sorted_data.is_empty() {
            return None;
        }
        let index = ((sorted_data.len() as f64 - 1.0) * percentile) as usize;
        Some(sorted_data[index])
    }

    /// Get average message latency
    pub fn avg_message_latency(&self) -> Option<Duration> {
        let latencies = self.message_latencies.read();
        if latencies.is_empty() {
            return None;
        }
        let sum: Duration = latencies.iter().sum();
        Some(sum / latencies.len() as u32)
    }

    /// Get message latency percentiles
    pub fn message_latency_percentiles(
        &self,
    ) -> (Option<Duration>, Option<Duration>, Option<Duration>) {
        let mut latencies = self.message_latencies.read().clone();
        if latencies.is_empty() {
            return (None, None, None);
        }
        latencies.sort();

        (
            Self::calculate_percentile(&latencies, 0.50),
            Self::calculate_percentile(&latencies, 0.95),
            Self::calculate_percentile(&latencies, 0.99),
        )
    }

    /// Get average connection duration
    pub fn avg_connection_duration(&self) -> Option<Duration> {
        let durations = self.connection_durations.read();
        if durations.is_empty() {
            return None;
        }
        let sum: Duration = durations.iter().sum();
        Some(sum / durations.len() as u32)
    }

    /// Get a snapshot of all current metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        let (p50, p95, p99) = self.message_latency_percentiles();

        MetricsSnapshot {
            timestamp: SystemTime::now(),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_connections: self.total_connections.load(Ordering::Relaxed),
            total_disconnections: self.total_disconnections.load(Ordering::Relaxed),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            errors_by_type: self
                .errors_by_type
                .read()
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), *v))
                .collect(),
            subscriptions_active: self.subscriptions_active.load(Ordering::Relaxed),
            subscriptions_total: self.subscriptions_total.load(Ordering::Relaxed),
            message_queue_depth: self.message_queue_depth.load(Ordering::Relaxed),
            backpressure_events: self.backpressure_events.load(Ordering::Relaxed),
            avg_connection_duration: self.avg_connection_duration(),
            avg_message_latency: self.avg_message_latency(),
            p50_message_latency: p50,
            p95_message_latency: p95,
            p99_message_latency: p99,
        }
    }

    /// Get per-connection metrics
    pub fn connection_metrics(&self, id: ConnectionId) -> Option<ConnectionMetrics> {
        self.connections.read().get(&id).cloned()
    }

    /// Get all tracked connections
    pub fn all_connections(&self) -> Vec<ConnectionMetrics> {
        self.connections.read().values().cloned().collect()
    }

    /// Reset all metrics (useful for testing)
    pub fn reset(&self) {
        self.active_connections.store(0, Ordering::Relaxed);
        self.total_connections.store(0, Ordering::Relaxed);
        self.total_disconnections.store(0, Ordering::Relaxed);
        self.messages_sent.store(0, Ordering::Relaxed);
        self.messages_received.store(0, Ordering::Relaxed);
        self.bytes_sent.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        self.subscriptions_active.store(0, Ordering::Relaxed);
        self.subscriptions_total.store(0, Ordering::Relaxed);
        self.message_queue_depth.store(0, Ordering::Relaxed);
        self.backpressure_events.store(0, Ordering::Relaxed);
        self.errors_by_type.write().clear();
        self.connection_durations.write().clear();
        self.message_latencies.write().clear();
        self.connections.write().clear();
    }
}

impl Default for WebSocketMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Timer Utility for Latency Tracking
// ============================================================================

/// High-precision timer for tracking WebSocket operation latencies
pub struct WebSocketTimer {
    start: Instant,
}

impl WebSocketTimer {
    /// Start a new timer
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Get elapsed duration since timer start
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Get elapsed time in milliseconds
    pub fn elapsed_millis(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    /// Get elapsed time in microseconds
    pub fn elapsed_micros(&self) -> u64 {
        self.start.elapsed().as_micros() as u64
    }
}

impl Default for WebSocketTimer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_connection_tracking() {
        let metrics = WebSocketMetrics::new();

        let conn1 = ConnectionId(1);
        metrics.connection_opened(conn1, "127.0.0.1:8080".to_string(), None);
        assert_eq!(metrics.active_connections(), 1);
        assert_eq!(metrics.total_connections(), 1);

        let conn2 = ConnectionId(2);
        metrics.connection_opened(conn2, "127.0.0.1:8081".to_string(), None);
        assert_eq!(metrics.active_connections(), 2);
        assert_eq!(metrics.total_connections(), 2);

        metrics.connection_closed(conn1, Duration::from_secs(10));
        assert_eq!(metrics.active_connections(), 1);
    }

    #[test]
    fn test_message_tracking() {
        let metrics = WebSocketMetrics::new();
        let conn = ConnectionId(1);

        metrics.connection_opened(conn, "127.0.0.1:8080".to_string(), None);
        metrics.message_sent(conn, 100);
        metrics.message_received(conn, 50);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.messages_sent, 1);
        assert_eq!(snapshot.messages_received, 1);
        assert_eq!(snapshot.bytes_sent, 100);
        assert_eq!(snapshot.bytes_received, 50);
    }

    #[test]
    fn test_error_tracking() {
        let metrics = WebSocketMetrics::new();

        metrics.record_error(None, ErrorType::ConnectionFailed);
        metrics.record_error(None, ErrorType::ConnectionFailed);
        metrics.record_error(None, ErrorType::ProtocolError);

        let errors = metrics.errors_by_type();
        assert_eq!(*errors.get(&ErrorType::ConnectionFailed).unwrap(), 2);
        assert_eq!(*errors.get(&ErrorType::ProtocolError).unwrap(), 1);
    }

    #[test]
    fn test_subscription_tracking() {
        let metrics = WebSocketMetrics::new();
        let conn = ConnectionId(1);

        metrics.connection_opened(conn, "127.0.0.1:8080".to_string(), None);
        metrics.subscription_added(conn, "topic1".to_string());
        metrics.subscription_added(conn, "topic2".to_string());

        assert_eq!(metrics.active_subscriptions(), 2);

        metrics.subscription_removed(conn, "topic1");
        assert_eq!(metrics.active_subscriptions(), 1);
    }

    #[test]
    fn test_latency_tracking() {
        let metrics = WebSocketMetrics::new();

        metrics.record_message_latency(Duration::from_millis(10));
        metrics.record_message_latency(Duration::from_millis(20));
        metrics.record_message_latency(Duration::from_millis(30));

        let avg = metrics.avg_message_latency().unwrap();
        assert_eq!(avg.as_millis(), 20);
    }

    #[test]
    fn test_timer() {
        let timer = WebSocketTimer::new();
        thread::sleep(Duration::from_millis(10));

        let elapsed = timer.elapsed();
        assert!(elapsed >= Duration::from_millis(10));
        assert!(timer.elapsed_millis() >= 10.0);
    }
}
