// WebSocket Metrics Collector Module
//
// Integration of WebSocket metrics with RustyDB's monitoring system
// Provides:
// - Prometheus metrics export for WebSocket connections
// - Dashboard data provider for real-time visualization
// - Health check integration
// - Metrics registry integration

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use super::health::{HealthCheckResult};
use super::metrics_core::*;
use super::metrics_registry::MetricsRegistry;
use crate::websocket::metrics::{ConnectionMetrics, ErrorType, MetricsSnapshot, WebSocketMetrics};

// ============================================================================
// WebSocket Metrics Collector
// ============================================================================

/// Main collector that integrates WebSocket metrics with the monitoring system
pub struct WebSocketMetricsCollector {
    /// Core WebSocket metrics
    ws_metrics: Arc<WebSocketMetrics>,

    /// Reference to the global metrics registry
    registry: Arc<MetricsRegistry>,

    /// Cached metric references for performance
    metric_refs: Arc<RwLock<WebSocketMetricRefs>>,

    /// Configuration
    config: CollectorConfig,
}

/// Configuration for the WebSocket metrics collector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorConfig {
    /// Enable Prometheus export
    pub enable_prometheus: bool,

    /// Enable dashboard data collection
    pub enable_dashboard: bool,

    /// Update interval for aggregated metrics (seconds)
    pub update_interval_secs: u64,

    /// Export detailed per-connection metrics
    pub export_per_connection: bool,

    /// Maximum number of error types to track
    pub max_error_types: usize,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            enable_prometheus: true,
            enable_dashboard: true,
            update_interval_secs: 15,
            export_per_connection: false,
            max_error_types: 50,
        }
    }
}

/// Cached references to frequently used metrics
#[allow(dead_code)]
struct WebSocketMetricRefs {
    // Connection metrics
    active_connections: Arc<GaugeMetric>,
    total_connections: Arc<CounterMetric>,
    total_disconnections: Arc<CounterMetric>,

    // Message metrics
    messages_sent: Arc<CounterMetric>,
    messages_received: Arc<CounterMetric>,

    // Data volume metrics
    bytes_sent: Arc<CounterMetric>,
    bytes_received: Arc<CounterMetric>,

    // Subscription metrics
    subscriptions_active: Arc<GaugeMetric>,
    subscriptions_total: Arc<CounterMetric>,

    // Performance metrics
    queue_depth: Arc<GaugeMetric>,
    backpressure_events: Arc<CounterMetric>,

    // Latency histograms
    #[allow(dead_code)]
    connection_duration: Arc<HistogramMetric>,
    #[allow(dead_code)]
    message_latency: Arc<HistogramMetric>,

    // Error counters by type
    errors: HashMap<ErrorType, Arc<CounterMetric>>,
}

impl WebSocketMetricsCollector {
    /// Create a new WebSocket metrics collector
    pub fn new(registry: Arc<MetricsRegistry>) -> Self {
        Self::with_config(registry, CollectorConfig::default())
    }

    /// Create a new WebSocket metrics collector with custom configuration
    pub fn with_config(registry: Arc<MetricsRegistry>, config: CollectorConfig) -> Self {
        let ws_metrics = Arc::new(WebSocketMetrics::new());
        let metric_refs = Arc::new(RwLock::new(Self::initialize_metrics(&registry)));

        Self {
            ws_metrics,
            registry,
            metric_refs,
            config,
        }
    }

    /// Initialize all metric references in the registry
    fn initialize_metrics(registry: &Arc<MetricsRegistry>) -> WebSocketMetricRefs {
        // Define standard histogram buckets for latency (in seconds)
        let latency_buckets = vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ];

        // Define histogram buckets for connection duration (in seconds)
        let duration_buckets = vec![
            1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0, 1800.0, 3600.0, 7200.0,
        ];

        WebSocketMetricRefs {
            active_connections: registry.get_or_create_gauge(
                "websocket_active_connections",
                &[],
                "Number of currently active WebSocket connections",
            ),
            total_connections: registry.get_or_create_counter(
                "websocket_connections_total",
                &[],
                "Total number of WebSocket connections since startup",
            ),
            total_disconnections: registry.get_or_create_counter(
                "websocket_disconnections_total",
                &[],
                "Total number of WebSocket disconnections since startup",
            ),
            messages_sent: registry.get_or_create_counter(
                "websocket_messages_sent_total",
                &[],
                "Total number of messages sent to clients",
            ),
            messages_received: registry.get_or_create_counter(
                "websocket_messages_received_total",
                &[],
                "Total number of messages received from clients",
            ),
            bytes_sent: registry.get_or_create_counter(
                "websocket_bytes_sent_total",
                &[],
                "Total bytes sent to clients",
            ),
            bytes_received: registry.get_or_create_counter(
                "websocket_bytes_received_total",
                &[],
                "Total bytes received from clients",
            ),
            subscriptions_active: registry.get_or_create_gauge(
                "websocket_subscriptions_active",
                &[],
                "Number of currently active subscriptions",
            ),
            subscriptions_total: registry.get_or_create_counter(
                "websocket_subscriptions_total",
                &[],
                "Total number of subscriptions created",
            ),
            queue_depth: registry.get_or_create_gauge(
                "websocket_message_queue_depth",
                &[],
                "Current depth of the outgoing message queue",
            ),
            backpressure_events: registry.get_or_create_counter(
                "websocket_backpressure_events_total",
                &[],
                "Total number of backpressure events",
            ),
            connection_duration: registry.get_or_create_histogram(
                "websocket_connection_duration_seconds",
                &[],
                "Duration of WebSocket connections in seconds",
                duration_buckets,
            ),
            message_latency: registry.get_or_create_histogram(
                "websocket_message_latency_seconds",
                &[],
                "Message processing latency in seconds",
                latency_buckets,
            ),
            errors: HashMap::new(),
        }
    }

    /// Get or create an error counter for a specific error type
    fn get_error_counter(&self, error_type: ErrorType) -> Arc<CounterMetric> {
        let mut refs = self.metric_refs.write();

        refs.errors
            .entry(error_type)
            .or_insert_with(|| {
                self.registry.get_or_create_counter(
                    "websocket_errors_total",
                    &[("type", error_type.as_str())],
                    "Total WebSocket errors by type",
                )
            })
            .clone()
    }

    /// Get the underlying WebSocket metrics
    pub fn metrics(&self) -> Arc<WebSocketMetrics> {
        self.ws_metrics.clone()
    }

    /// Update all metrics in the registry from the WebSocket metrics snapshot
    pub fn update(&self) {
        let snapshot = self.ws_metrics.snapshot();
        let refs = self.metric_refs.read();

        // Update connection metrics
        refs.active_connections
            .set(snapshot.active_connections as f64);

        // For counters, we need to set them to the total value
        // (In a real implementation, we might track deltas instead)
        let current_connections = refs.total_connections.get();
        if snapshot.total_connections > current_connections {
            refs.total_connections
                .inc_by(snapshot.total_connections - current_connections);
        }

        let current_disconnections = refs.total_disconnections.get();
        if snapshot.total_disconnections > current_disconnections {
            refs.total_disconnections
                .inc_by(snapshot.total_disconnections - current_disconnections);
        }

        // Update message metrics
        let current_sent = refs.messages_sent.get();
        if snapshot.messages_sent > current_sent {
            refs.messages_sent
                .inc_by(snapshot.messages_sent - current_sent);
        }

        let current_received = refs.messages_received.get();
        if snapshot.messages_received > current_received {
            refs.messages_received
                .inc_by(snapshot.messages_received - current_received);
        }

        // Update byte metrics
        let current_bytes_sent = refs.bytes_sent.get();
        if snapshot.bytes_sent > current_bytes_sent {
            refs.bytes_sent
                .inc_by(snapshot.bytes_sent - current_bytes_sent);
        }

        let current_bytes_received = refs.bytes_received.get();
        if snapshot.bytes_received > current_bytes_received {
            refs.bytes_received
                .inc_by(snapshot.bytes_received - current_bytes_received);
        }

        // Update subscription metrics
        refs.subscriptions_active
            .set(snapshot.subscriptions_active as f64);

        let current_subs = refs.subscriptions_total.get();
        if snapshot.subscriptions_total > current_subs {
            refs.subscriptions_total
                .inc_by(snapshot.subscriptions_total - current_subs);
        }

        // Update performance metrics
        refs.queue_depth.set(snapshot.message_queue_depth as f64);

        let current_backpressure = refs.backpressure_events.get();
        if snapshot.backpressure_events > current_backpressure {
            refs.backpressure_events
                .inc_by(snapshot.backpressure_events - current_backpressure);
        }

        // Update error metrics
        drop(refs); // Release read lock before acquiring write lock
        for (error_type_str, count) in &snapshot.errors_by_type {
            if let Some(error_type) = Self::parse_error_type(error_type_str) {
                let counter = self.get_error_counter(error_type);
                let current = counter.get();
                if *count > current {
                    counter.inc_by(count - current);
                }
            }
        }
    }

    /// Parse error type from string
    fn parse_error_type(s: &str) -> Option<ErrorType> {
        match s {
            "connection_failed" => Some(ErrorType::ConnectionFailed),
            "handshake_failed" => Some(ErrorType::HandshakeFailed),
            "protocol_error" => Some(ErrorType::ProtocolError),
            "message_parse_error" => Some(ErrorType::MessageParseError),
            "authentication_failed" => Some(ErrorType::AuthenticationFailed),
            "authorization_failed" => Some(ErrorType::AuthorizationFailed),
            "timeout" => Some(ErrorType::Timeout),
            "network_error" => Some(ErrorType::NetworkError),
            "internal_error" => Some(ErrorType::InternalError),
            "rate_limit_exceeded" => Some(ErrorType::RateLimitExceeded),
            "invalid_message" => Some(ErrorType::InvalidMessage),
            "other" => Some(ErrorType::Other),
            _ => None,
        }
    }

    /// Export Prometheus metrics text format
    pub fn export_prometheus(&self) -> String {
        // First update all metrics
        self.update();

        // Let the registry's Prometheus exporter handle the formatting
        // This is a placeholder - in real implementation, we'd use PrometheusExporter
        let snapshot = self.ws_metrics.snapshot();

        let mut output = String::new();

        // Connection metrics
        output.push_str("# HELP websocket_active_connections Number of currently active WebSocket connections\n");
        output.push_str("# TYPE websocket_active_connections gauge\n");
        output.push_str(&format!(
            "websocket_active_connections {}\n",
            snapshot.active_connections
        ));

        output.push_str("# HELP websocket_connections_total Total number of WebSocket connections since startup\n");
        output.push_str("# TYPE websocket_connections_total counter\n");
        output.push_str(&format!(
            "websocket_connections_total {}\n",
            snapshot.total_connections
        ));

        // Message metrics
        output.push_str(
            "# HELP websocket_messages_sent_total Total number of messages sent to clients\n",
        );
        output.push_str("# TYPE websocket_messages_sent_total counter\n");
        output.push_str(&format!(
            "websocket_messages_sent_total {}\n",
            snapshot.messages_sent
        ));

        output.push_str("# HELP websocket_messages_received_total Total number of messages received from clients\n");
        output.push_str("# TYPE websocket_messages_received_total counter\n");
        output.push_str(&format!(
            "websocket_messages_received_total {}\n",
            snapshot.messages_received
        ));

        // Bytes metrics
        output.push_str("# HELP websocket_bytes_sent_total Total bytes sent to clients\n");
        output.push_str("# TYPE websocket_bytes_sent_total counter\n");
        output.push_str(&format!(
            "websocket_bytes_sent_total {}\n",
            snapshot.bytes_sent
        ));

        output
            .push_str("# HELP websocket_bytes_received_total Total bytes received from clients\n");
        output.push_str("# TYPE websocket_bytes_received_total counter\n");
        output.push_str(&format!(
            "websocket_bytes_received_total {}\n",
            snapshot.bytes_received
        ));

        // Subscription metrics
        output.push_str(
            "# HELP websocket_subscriptions_active Number of currently active subscriptions\n",
        );
        output.push_str("# TYPE websocket_subscriptions_active gauge\n");
        output.push_str(&format!(
            "websocket_subscriptions_active {}\n",
            snapshot.subscriptions_active
        ));

        // Performance metrics
        output.push_str(
            "# HELP websocket_message_queue_depth Current depth of the outgoing message queue\n",
        );
        output.push_str("# TYPE websocket_message_queue_depth gauge\n");
        output.push_str(&format!(
            "websocket_message_queue_depth {}\n",
            snapshot.message_queue_depth
        ));

        output.push_str(
            "# HELP websocket_backpressure_events_total Total number of backpressure events\n",
        );
        output.push_str("# TYPE websocket_backpressure_events_total counter\n");
        output.push_str(&format!(
            "websocket_backpressure_events_total {}\n",
            snapshot.backpressure_events
        ));

        // Error metrics
        output.push_str("# HELP websocket_errors_total Total WebSocket errors by type\n");
        output.push_str("# TYPE websocket_errors_total counter\n");
        for (error_type, count) in &snapshot.errors_by_type {
            output.push_str(&format!(
                "websocket_errors_total{{type=\"{}\"}} {}\n",
                error_type, count
            ));
        }

        output
    }

    /// Get dashboard data suitable for visualization
    pub fn dashboard_data(&self) -> WebSocketDashboardData {
        let snapshot = self.ws_metrics.snapshot();
        let connections = if self.config.export_per_connection {
            self.ws_metrics.all_connections()
        } else {
            Vec::new()
        };

        WebSocketDashboardData {
            snapshot,
            connections,
            health: self.health_check(),
        }
    }

    /// Perform health check on WebSocket metrics
    pub fn health_check(&self) -> HealthCheckResult {
        let snapshot = self.ws_metrics.snapshot();

        // Define health criteria
        let max_healthy_queue_depth = 1000;
        let max_degraded_queue_depth = 5000;
        let max_healthy_error_rate = 0.01; // 1%

        // Check queue depth
        if snapshot.message_queue_depth > max_degraded_queue_depth {
            return HealthCheckResult::unhealthy(
                "websocket",
                format!(
                    "Message queue depth critically high: {}",
                    snapshot.message_queue_depth
                ),
            );
        }

        if snapshot.message_queue_depth > max_healthy_queue_depth {
            return HealthCheckResult::degraded(
                "websocket",
                format!(
                    "Message queue depth elevated: {}",
                    snapshot.message_queue_depth
                ),
            );
        }

        // Check error rate
        let total_messages = snapshot.messages_sent + snapshot.messages_received;
        if total_messages > 0 {
            let total_errors: u64 = snapshot.errors_by_type.values().sum();
            let error_rate = total_errors as f64 / total_messages as f64;

            if error_rate > max_healthy_error_rate {
                return HealthCheckResult::degraded(
                    "websocket",
                    format!("Error rate elevated: {:.2}%", error_rate * 100.0),
                );
            }
        }

        // All checks passed
        HealthCheckResult::healthy("websocket")
            .with_detail(
                "active_connections".to_string(),
                serde_json::json!(snapshot.active_connections),
            )
            .with_detail(
                "queue_depth".to_string(),
                serde_json::json!(snapshot.message_queue_depth),
            )
    }

    /// Get real-time streaming data (for WebSocket streaming to dashboards)
    pub fn streaming_data(&self) -> WebSocketStreamingData {
        let snapshot = self.ws_metrics.snapshot();

        WebSocketStreamingData {
            timestamp: snapshot.timestamp,
            active_connections: snapshot.active_connections,
            messages_per_sec: 0.0, // Would need to calculate rate
            bytes_per_sec: 0.0,    // Would need to calculate rate
            avg_latency_ms: snapshot
                .avg_message_latency
                .map(|d| d.as_secs_f64() * 1000.0),
            queue_depth: snapshot.message_queue_depth,
            error_count: snapshot.errors_by_type.values().sum(),
        }
    }
}

// ============================================================================
// Dashboard Data Structures
// ============================================================================

/// Complete dashboard data for WebSocket metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketDashboardData {
    pub snapshot: MetricsSnapshot,
    pub connections: Vec<ConnectionMetrics>,
    pub health: HealthCheckResult,
}

/// Real-time streaming data for live dashboards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketStreamingData {
    pub timestamp: std::time::SystemTime,
    pub active_connections: usize,
    pub messages_per_sec: f64,
    pub bytes_per_sec: f64,
    pub avg_latency_ms: Option<f64>,
    pub queue_depth: usize,
    pub error_count: u64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use crate::api::HealthStatus;
    use super::*;

    #[test]
    fn test_collector_creation() {
        let registry = Arc::new(MetricsRegistry::default());
        let collector = WebSocketMetricsCollector::new(registry);

        assert_eq!(collector.ws_metrics.active_connections(), 0);
    }

    #[test]
    fn test_prometheus_export() {
        let registry = Arc::new(MetricsRegistry::default());
        let collector = WebSocketMetricsCollector::new(registry);

        let output = collector.export_prometheus();
        assert!(output.contains("websocket_active_connections"));
        assert!(output.contains("websocket_connections_total"));
        assert!(output.contains("websocket_messages_sent_total"));
    }

    #[test]
    fn test_health_check() {
        let registry = Arc::new(MetricsRegistry::default());
        let collector = WebSocketMetricsCollector::new(registry);

        let health = collector.health_check();
        assert_eq!(health.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_dashboard_data() {
        let registry = Arc::new(MetricsRegistry::default());
        let collector = WebSocketMetricsCollector::new(registry);

        let dashboard = collector.dashboard_data();
        assert_eq!(dashboard.snapshot.active_connections, 0);
        assert_eq!(dashboard.health.status, HealthStatus::Healthy);
    }
}
