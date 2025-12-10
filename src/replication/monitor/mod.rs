// # Replication Health Monitoring
//
// This module provides comprehensive health monitoring for replication systems,
// including metrics collection, trend analysis, alerting, and historical tracking.

mod errors;
mod types;
mod config;
mod monitor;
mod monitor_impl;

// Re-export public types
pub use errors::HealthMonitorError;
pub use types::{
    ReplicaHealthStatus, HealthCheckResult, HealthComponents, ComponentHealth,
    HealthIssue, IssueSeverity, HealthMetrics, HealthAlert, AlertStatus,
    HealthHistoryEntry, HealthStatistics, HealthTrend, TrendDirection,
    HealthPrediction,
};
pub use config::{HealthMonitorConfig, HealthThresholds};
pub use monitor::{HealthMonitor, ReplicationHealthMonitor};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replication::ReplicaId;

    #[tokio::test]
    async fn test_health_monitor_creation() {
        let config = HealthMonitorConfig::default();
        let monitor = ReplicationHealthMonitor::new(config).await;
        assert!(monitor.is_ok());
    }

    #[tokio::test]
    async fn test_replica_health_check() {
        let config = HealthMonitorConfig::default();
        let monitor = ReplicationHealthMonitor::new(config).await.unwrap();

        let replica_id = ReplicaId::new("test-replica").unwrap();
        let result = monitor.check_replica_health(&replica_id).await;
        assert!(result.is_ok());

        let health = result.unwrap();
        assert_eq!(health.replica_id, replica_id);
        assert!(health.overall_score >= 0.0 && health.overall_score <= 100.0);
    }

    #[tokio::test]
    async fn test_health_history() {
        let config = HealthMonitorConfig::default();
        let monitor = ReplicationHealthMonitor::new(config).await.unwrap();

        let replica_id = ReplicaId::new("test-replica").unwrap();

        // Perform multiple health checks to build history
        for _ in 0..5 {
            let _ = monitor.check_replica_health(&replica_id).await;
        }

        let history = monitor
            .get_health_history(&replica_id, std::time::Duration::from_secs(3600))
            .await;

        assert!(history.is_ok());
        let entries = history.unwrap();
        assert_eq!(entries.len(), 5);
    }

    #[tokio::test]
    async fn test_health_statistics() {
        let config = HealthMonitorConfig::default();
        let monitor = ReplicationHealthMonitor::new(config).await.unwrap();

        let replica_id = ReplicaId::new("test-replica").unwrap();

        // Perform health checks
        for _ in 0..3 {
            let _ = monitor.check_replica_health(&replica_id).await;
        }

        let stats = monitor.get_statistics(Some(&replica_id)).await;
        assert!(stats.is_ok());

        let stats = stats.unwrap();
        assert_eq!(stats.total_checks, 3);
    }

    #[tokio::test]
    async fn test_health_trend() {
        let config = HealthMonitorConfig::default();
        let monitor = ReplicationHealthMonitor::new(config).await.unwrap();

        let replica_id = ReplicaId::new("test-replica").unwrap();

        // Build some history
        for _ in 0..10 {
            let _ = monitor.check_replica_health(&replica_id).await;
        }

        let trend = monitor.get_health_trend(&replica_id).await;
        assert!(trend.is_ok());

        let trend = trend.unwrap();
        assert_eq!(trend.replica_id, replica_id);
    }

    #[tokio::test]
    async fn test_alert_acknowledgement() {
        let config = HealthMonitorConfig::default();
        let monitor = ReplicationHealthMonitor::new(config).await.unwrap();

        // Try to acknowledge non-existent alert
        let result = monitor.acknowledge_alert("non-existent").await;
        assert!(result.is_err());
    }
}
