// Health monitoring implementation

use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::replication::ReplicaId;
use super::config::HealthMonitorConfig;
use super::errors::HealthMonitorError;
use super::types::*;

// Health monitor trait
#[async_trait]
pub trait HealthMonitor: Send + Sync {
    // Check health of a specific replica
    async fn check_replica_health(&self, replica_id: &ReplicaId) -> Result<HealthCheckResult, HealthMonitorError>;

    // Check health of all replicas
    async fn check_all_replicas(&self) -> Result<Vec<HealthCheckResult>, HealthMonitorError>;

    // Get health history for a replica
    async fn get_health_history(
        &self,
        replica_id: &ReplicaId,
        duration: Duration,
    ) -> Result<Vec<HealthHistoryEntry>, HealthMonitorError>;

    // Get health statistics
    async fn get_statistics(&self, replica_id: Option<&ReplicaId>) -> Result<HealthStatistics, HealthMonitorError>;

    // Get active alerts
    async fn get_active_alerts(&self) -> Result<Vec<HealthAlert>, HealthMonitorError>;

    // Acknowledge an alert
    async fn acknowledge_alert(&self, alert_id: &str) -> Result<(), HealthMonitorError>;

    // Get health trend
    async fn get_health_trend(&self, replica_id: &ReplicaId) -> Result<HealthTrend, HealthMonitorError>;
}

// Replication health monitor implementation
pub struct ReplicationHealthMonitor {
    // Configuration
    pub(crate) config: Arc<HealthMonitorConfig>,
    // Health history per replica
    pub(crate) history: Arc<RwLock<HashMap<ReplicaId, VecDeque<HealthHistoryEntry>>>>,
    // Active alerts
    pub(crate) alerts: Arc<RwLock<HashMap<String, HealthAlert>>>,
    // Statistics
    pub(crate) statistics: Arc<RwLock<HashMap<ReplicaId, HealthStatistics>>>,
    // Background task handle
    monitor_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl ReplicationHealthMonitor {
    // Create a new health monitor
    pub async fn new(config: HealthMonitorConfig) -> Result<Self, HealthMonitorError> {
        Self::validate_config(&config)?;

        let monitor = Self {
            config: Arc::new(config),
            history: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(HashMap::new())),
            statistics: Arc::new(RwLock::new(HashMap::new())),
            monitor_handle: Arc::new(RwLock::new(None)),
        };

        monitor.start_background_monitoring().await;

        Ok(monitor)
    }

    fn validate_config(config: &HealthMonitorConfig) -> Result<(), HealthMonitorError> {
        if config.check_interval.as_secs() == 0 {
            return Err(HealthMonitorError::ConfigurationError {
                reason: "Check interval must be greater than 0".to_string(),
            });
        }
        Ok(())
    }

    async fn start_background_monitoring(&self) {
        let config = Arc::clone(&self.config);
        let history: Arc<RwLock<HashMap<ReplicaId, VecDeque<HealthHistoryEntry>>>> = Arc::clone(&self.history);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.check_interval);
            loop {
                interval.tick().await;

                // Cleanup old history entries
                let now = SystemTime::now();
                let mut history = history.write();
                for entries in history.values_mut() {
                    entries.retain(|entry| {
                        now.duration_since(entry.timestamp).unwrap_or_default() < config.history_retention
                    });

                    // Limit number of entries
                    while entries.len() > config.max_history_entries {
                        entries.pop_front();
                    }
                }
            }
        });

        *self.monitor_handle.write() = Some(handle);
    }

    pub(crate) fn collect_metrics(&self, replica_id: &ReplicaId) -> Result<HealthMetrics, HealthMonitorError> {
        // Simplified metric collection - in production this would query actual systems
        Ok(HealthMetrics {
            replication_lag_bytes: 50 * 1024 * 1024, // 50MB
            replication_lag_time: Duration::from_secs(30),
            bytes_per_second: 10_000_000.0, // 10MB/s
            records_per_second: 5000.0,
            error_rate: 0.001,
            connection_count: 5,
            cpu_usage_percent: 45.0,
            memory_usage_bytes: 4 * 1024 * 1024 * 1024, // 4GB
            disk_usage_bytes: 100 * 1024 * 1024 * 1024, // 100GB
        })
    }

    pub(crate) fn calculate_component_health(&self, metrics: &HealthMetrics) -> HealthComponents {
        let thresholds = &self.config.thresholds;

        // Lag component
        let lag_status = if metrics.replication_lag_bytes >= thresholds.lag_bytes_critical {
            ReplicaHealthStatus::Critical
        } else if metrics.replication_lag_bytes >= thresholds.lag_bytes_warning {
            ReplicaHealthStatus::Degraded
        } else {
            ReplicaHealthStatus::Healthy
        };

        let lag_score = 100.0 - (metrics.replication_lag_bytes as f64 / thresholds.lag_bytes_critical as f64 * 100.0).min(100.0);

        // Connection component
        let connection_status = if metrics.connection_count > 0 {
            ReplicaHealthStatus::Healthy
        } else {
            ReplicaHealthStatus::Critical
        };

        // Throughput component
        let throughput_status = if metrics.bytes_per_second < thresholds.throughput_min_critical {
            ReplicaHealthStatus::Critical
        } else if metrics.bytes_per_second < thresholds.throughput_min_warning {
            ReplicaHealthStatus::Degraded
        } else {
            ReplicaHealthStatus::Healthy
        };

        let throughput_score = (metrics.bytes_per_second / thresholds.throughput_min_warning * 100.0).min(100.0);

        // Error rate component
        let error_status = if metrics.error_rate >= thresholds.error_rate_critical {
            ReplicaHealthStatus::Critical
        } else if metrics.error_rate >= thresholds.error_rate_warning {
            ReplicaHealthStatus::Degraded
        } else {
            ReplicaHealthStatus::Healthy
        };

        let error_score = 100.0 - (metrics.error_rate / thresholds.error_rate_critical * 100.0).min(100.0);

        // Resource usage component
        let resource_status = if metrics.cpu_usage_percent >= thresholds.cpu_usage_critical {
            ReplicaHealthStatus::Critical
        } else if metrics.cpu_usage_percent >= thresholds.cpu_usage_warning {
            ReplicaHealthStatus::Degraded
        } else {
            ReplicaHealthStatus::Healthy
        };

        let resource_score = 100.0 - (metrics.cpu_usage_percent / thresholds.cpu_usage_critical * 100.0).min(100.0);

        HealthComponents {
            replication_lag: ComponentHealth {
                status: lag_status,
                score: lag_score,
                value: metrics.replication_lag_bytes as f64,
                threshold: thresholds.lag_bytes_warning as f64,
                unit: "bytes".to_string(),
            },
            connection: ComponentHealth {
                status: connection_status,
                score: 100.0,
                value: metrics.connection_count as f64,
                threshold: 1.0,
                unit: "connections".to_string(),
            },
            throughput: ComponentHealth {
                status: throughput_status,
                score: throughput_score,
                value: metrics.bytes_per_second,
                threshold: thresholds.throughput_min_warning,
                unit: "bytes/sec".to_string(),
            },
            error_rate: ComponentHealth {
                status: error_status,
                score: error_score,
                value: metrics.error_rate,
                threshold: thresholds.error_rate_warning,
                unit: "percentage".to_string(),
            },
            resource_usage: ComponentHealth {
                status: resource_status,
                score: resource_score,
                value: metrics.cpu_usage_percent,
                threshold: thresholds.cpu_usage_warning,
                unit: "percentage".to_string(),
            },
        }
    }

    pub(crate) fn calculate_overall_score(&self, components: &HealthComponents) -> f64 {
        // Weighted average of component scores
        let weights = [0.3, 0.2, 0.2, 0.15, 0.15]; // lag, connection, throughput, error, resource
        let scores = [
            components.replication_lag.score,
            components.connection.score,
            components.throughput.score,
            components.error_rate.score,
            components.resource_usage.score,
        ];

        weights.iter().zip(scores.iter()).map(|(w, s)| w * s).sum()
    }

    pub(crate) fn determine_overall_status(&self, score: f64) -> ReplicaHealthStatus {
        if score >= 90.0 {
            ReplicaHealthStatus::Healthy
        } else if score >= 70.0 {
            ReplicaHealthStatus::Degraded
        } else if score >= 40.0 {
            ReplicaHealthStatus::Unhealthy
        } else {
            ReplicaHealthStatus::Critical
        }
    }

    pub(crate) fn identify_issues(&self, components: &HealthComponents, metrics: &HealthMetrics) -> Vec<HealthIssue> {
        let mut issues = Vec::new();
        let now = SystemTime::now();

        // Check lag
        if components.replication_lag.status == ReplicaHealthStatus::Critical {
            issues.push(HealthIssue {
                severity: IssueSeverity::Critical,
                component: "replication_lag".to_string(),
                description: format!("Critical replication lag: {} bytes", metrics.replication_lag_bytes),
                recommendation: Some("Investigate network, check for slow queries, consider scaling".to_string()),
                detected_at: now,
            });
        } else if components.replication_lag.status == ReplicaHealthStatus::Degraded {
            issues.push(HealthIssue {
                severity: IssueSeverity::Warning,
                component: "replication_lag".to_string(),
                description: format!("High replication lag: {} bytes", metrics.replication_lag_bytes),
                recommendation: Some("Monitor closely, check system load".to_string()),
                detected_at: now,
            });
        }

        // Check error rate
        if components.error_rate.status != ReplicaHealthStatus::Healthy {
            issues.push(HealthIssue {
                severity: if components.error_rate.status == ReplicaHealthStatus::Critical {
                    IssueSeverity::Critical
                } else {
                    IssueSeverity::Warning
                },
                component: "error_rate".to_string(),
                description: format!("Elevated error rate: {:.2}%", metrics.error_rate * 100.0),
                recommendation: Some("Check logs for error patterns".to_string()),
                detected_at: now,
            });
        }

        issues
    }

    pub(crate) fn record_health_history(&self, result: &HealthCheckResult) {
        let mut history = self.history.write();
        let entries = history.entry(result.replica_id.clone()).or_insert_with(VecDeque::new);

        let entry = HealthHistoryEntry {
            timestamp: result.timestamp,
            replica_id: result.replica_id.clone(),
            status: result.status.clone(),
            score: result.overall_score,
            metrics: result.metrics.clone(),
        };

        entries.push_back(entry);

        // Limit size
        while entries.len() > self.config.max_history_entries {
            entries.pop_front();
        }
    }

    pub(crate) fn update_statistics(&self, replica_id: &ReplicaId, result: &HealthCheckResult) {
        let mut statistics = self.statistics.write();
        let stats = statistics.entry(replica_id.clone()).or_insert_with(HealthStatistics::default);

        stats.total_checks += 1;

        match result.status {
            ReplicaHealthStatus::Healthy => stats.healthy_count += 1,
            ReplicaHealthStatus::Degraded => stats.degraded_count += 1,
            ReplicaHealthStatus::Unhealthy => stats.unhealthy_count += 1,
            ReplicaHealthStatus::Critical => stats.critical_count += 1,
            _ => {}
        }

        // Update running averages
        let total = stats.total_checks as f64;
        stats.average_score = (stats.average_score * (total - 1.0) + result.overall_score) / total;
        stats.average_lag_bytes = (stats.average_lag_bytes as f64 * (total - 1.0) + result.metrics.replication_lag_bytes as f64) as u64 / stats.total_checks;

        stats.uptime_percentage = (stats.healthy_count + stats.degraded_count) as f64 / total * 100.0;
    }
}

impl Default for ReplicationHealthMonitor {
    fn default() -> Self {
        futures::executor::block_on(async {
            Self::new(HealthMonitorConfig::default()).await.unwrap()
        })
    }
}
