// # Health Reporter
//
// Generates health reports, exports metrics, and provides status endpoints
// for monitoring and observability.

use super::aggregator::HealthScore;
use crate::common::{HealthStatus, MetricValue, NodeId};
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Health report for a single node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealthReport {
    /// Node identifier
    pub node_id: NodeId,

    /// Current health status
    pub status: HealthStatus,

    /// Health score
    pub score: HealthScore,

    /// Last check timestamp
    pub last_check: SystemTime,

    /// Time since last healthy state
    pub unhealthy_duration: Option<Duration>,

    /// Active alerts
    pub alerts: Vec<HealthAlert>,

    /// Recent check results
    pub recent_checks: Vec<CheckSummary>,
}

/// Check summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckSummary {
    /// Check type
    pub check_type: String,

    /// Success count
    pub success_count: u64,

    /// Failure count
    pub failure_count: u64,

    /// Average response time
    pub avg_response_time_ms: f64,

    /// Last check time
    pub last_check: SystemTime,
}

/// Health alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    /// Alert severity
    pub severity: AlertSeverity,

    /// Alert message
    pub message: String,

    /// When the alert was triggered
    pub triggered_at: SystemTime,

    /// Alert type
    pub alert_type: AlertType,
}

/// Alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    NodeDown,
    NodeDegraded,
    HighLatency,
    LowAvailability,
    DependencyFailure,
    CascadingFailure,
    Custom(String),
}

/// Complete health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Report generation timestamp
    pub timestamp: SystemTime,

    /// Cluster-wide health status
    pub cluster_status: HealthStatus,

    /// Cluster health score
    pub cluster_score: HealthScore,

    /// Per-node reports
    pub node_reports: HashMap<NodeId, NodeHealthReport>,

    /// Active cluster-wide alerts
    pub cluster_alerts: Vec<HealthAlert>,

    /// Overall statistics
    pub statistics: HealthStatistics,
}

/// Health statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatistics {
    /// Total nodes
    pub total_nodes: usize,

    /// Healthy nodes
    pub healthy_nodes: usize,

    /// Degraded nodes
    pub degraded_nodes: usize,

    /// Unhealthy nodes
    pub unhealthy_nodes: usize,

    /// Average cluster latency
    pub avg_latency_ms: f64,

    /// Cluster availability percentage
    pub availability_percent: f64,

    /// Total checks performed
    pub total_checks: u64,

    /// Total failures
    pub total_failures: u64,
}

/// Health reporter
pub struct HealthReporter {
    /// Historical reports
    report_history: Vec<HealthReport>,

    /// Maximum history size
    max_history_size: usize,

    /// Alert history
    alert_history: Vec<HealthAlert>,

    /// Maximum alert history size
    max_alert_history: usize,

    /// Metrics export enabled
    metrics_enabled: bool,

    /// Alert thresholds
    alert_thresholds: AlertThresholds,
}

/// Alert thresholds configuration
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    /// High latency threshold (milliseconds)
    pub high_latency_ms: f64,

    /// Low availability threshold (percentage)
    pub low_availability_percent: f64,

    /// Degraded score threshold
    pub degraded_score: f64,

    /// Critical score threshold
    pub critical_score: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            high_latency_ms: 1000.0,
            low_availability_percent: 95.0,
            degraded_score: 0.7,
            critical_score: 0.3,
        }
    }
}

impl HealthReporter {
    /// Create a new health reporter
    pub fn new() -> Self {
        Self {
            report_history: Vec::new(),
            max_history_size: 1000,
            alert_history: Vec::new(),
            max_alert_history: 10000,
            metrics_enabled: true,
            alert_thresholds: AlertThresholds::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(max_history_size: usize, alert_thresholds: AlertThresholds) -> Self {
        Self {
            report_history: Vec::new(),
            max_history_size,
            alert_history: Vec::new(),
            max_alert_history: 10000,
            metrics_enabled: true,
            alert_thresholds,
        }
    }

    /// Generate a health report
    pub async fn generate_report(&self) -> Result<HealthReport> {
        // This is a placeholder - in a real implementation, this would
        // gather data from the aggregator and other components
        Ok(HealthReport {
            timestamp: SystemTime::now(),
            cluster_status: HealthStatus::Healthy,
            cluster_score: HealthScore::perfect(),
            node_reports: HashMap::new(),
            cluster_alerts: Vec::new(),
            statistics: HealthStatistics {
                total_nodes: 0,
                healthy_nodes: 0,
                degraded_nodes: 0,
                unhealthy_nodes: 0,
                avg_latency_ms: 0.0,
                availability_percent: 100.0,
                total_checks: 0,
                total_failures: 0,
            },
        })
    }

    /// Add a report to history
    pub fn record_report(&mut self, report: HealthReport) {
        self.report_history.push(report);

        // Trim history
        if self.report_history.len() > self.max_history_size {
            self.report_history.remove(0);
        }
    }

    /// Generate alerts based on health state
    pub fn generate_alerts(&mut self, report: &HealthReport) -> Vec<HealthAlert> {
        let mut alerts = Vec::new();

        // Check cluster-wide metrics
        if report.cluster_score.score < self.alert_thresholds.critical_score {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Critical,
                message: format!(
                    "Cluster health critical: score {:.2}",
                    report.cluster_score.score
                ),
                triggered_at: SystemTime::now(),
                alert_type: AlertType::Custom("cluster_critical".to_string()),
            });
        } else if report.cluster_score.score < self.alert_thresholds.degraded_score {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Warning,
                message: format!(
                    "Cluster health degraded: score {:.2}",
                    report.cluster_score.score
                ),
                triggered_at: SystemTime::now(),
                alert_type: AlertType::Custom("cluster_degraded".to_string()),
            });
        }

        // Check per-node alerts
        for (node_id, node_report) in &report.node_reports {
            // Node down
            if node_report.status == HealthStatus::Unhealthy {
                alerts.push(HealthAlert {
                    severity: AlertSeverity::Error,
                    message: format!("Node {} is unhealthy", node_id),
                    triggered_at: SystemTime::now(),
                    alert_type: AlertType::NodeDown,
                });
            }

            // Node degraded
            if node_report.status == HealthStatus::Degraded {
                alerts.push(HealthAlert {
                    severity: AlertSeverity::Warning,
                    message: format!("Node {} is degraded", node_id),
                    triggered_at: SystemTime::now(),
                    alert_type: AlertType::NodeDegraded,
                });
            }

            // Low availability
            if node_report.score.availability
                < self.alert_thresholds.low_availability_percent / 100.0
            {
                alerts.push(HealthAlert {
                    severity: AlertSeverity::Warning,
                    message: format!(
                        "Node {} has low availability: {:.1}%",
                        node_id,
                        node_report.score.availability * 100.0
                    ),
                    triggered_at: SystemTime::now(),
                    alert_type: AlertType::LowAvailability,
                });
            }
        }

        // Record alerts
        for alert in &alerts {
            self.alert_history.push(alert.clone());
        }

        // Trim alert history
        if self.alert_history.len() > self.max_alert_history {
            let excess = self.alert_history.len() - self.max_alert_history;
            self.alert_history.drain(0..excess);
        }

        alerts
    }

    /// Export metrics in Prometheus format
    pub async fn export_metrics(&self) -> Result<String> {
        if !self.metrics_enabled {
            return Err(DbError::InvalidOperation(
                "Metrics export disabled".to_string(),
            ));
        }

        let latest_report = self
            .report_history
            .last()
            .ok_or_else(|| DbError::NotFound("No reports available".to_string()))?;

        let mut metrics = String::new();

        // Cluster metrics
        metrics.push_str(&format!(
            "# HELP rustydb_cluster_health_score Overall cluster health score (0-1)\n\
             # TYPE rustydb_cluster_health_score gauge\n\
             rustydb_cluster_health_score {}\n\n",
            latest_report.cluster_score.score
        ));

        metrics.push_str(&format!(
            "# HELP rustydb_cluster_availability Cluster availability score (0-1)\n\
             # TYPE rustydb_cluster_availability gauge\n\
             rustydb_cluster_availability {}\n\n",
            latest_report.cluster_score.availability
        ));

        metrics.push_str(&format!(
            "# HELP rustydb_cluster_nodes_total Total number of nodes\n\
             # TYPE rustydb_cluster_nodes_total gauge\n\
             rustydb_cluster_nodes_total {}\n\n",
            latest_report.statistics.total_nodes
        ));

        metrics.push_str(&format!(
            "# HELP rustydb_cluster_nodes_healthy Number of healthy nodes\n\
             # TYPE rustydb_cluster_nodes_healthy gauge\n\
             rustydb_cluster_nodes_healthy {}\n\n",
            latest_report.statistics.healthy_nodes
        ));

        metrics.push_str(&format!(
            "# HELP rustydb_cluster_nodes_unhealthy Number of unhealthy nodes\n\
             # TYPE rustydb_cluster_nodes_unhealthy gauge\n\
             rustydb_cluster_nodes_unhealthy {}\n\n",
            latest_report.statistics.unhealthy_nodes
        ));

        metrics.push_str(&format!(
            "# HELP rustydb_cluster_avg_latency_ms Average cluster latency in milliseconds\n\
             # TYPE rustydb_cluster_avg_latency_ms gauge\n\
             rustydb_cluster_avg_latency_ms {}\n\n",
            latest_report.statistics.avg_latency_ms
        ));

        // Per-node metrics
        for (node_id, node_report) in &latest_report.node_reports {
            metrics.push_str(&format!(
                "rustydb_node_health_score{{node=\"{}\"}} {}\n",
                node_id, node_report.score.score
            ));

            metrics.push_str(&format!(
                "rustydb_node_availability{{node=\"{}\"}} {}\n",
                node_id, node_report.score.availability
            ));
        }

        Ok(metrics)
    }

    /// Export metrics as JSON
    pub async fn export_metrics_json(&self) -> Result<serde_json::Value> {
        let latest_report = self
            .report_history
            .last()
            .ok_or_else(|| DbError::NotFound("No reports available".to_string()))?;

        let mut metrics = HashMap::new();

        // Cluster metrics
        metrics.insert(
            "cluster_health_score".to_string(),
            MetricValue::Gauge(latest_report.cluster_score.score),
        );
        metrics.insert(
            "cluster_availability".to_string(),
            MetricValue::Gauge(latest_report.cluster_score.availability),
        );
        metrics.insert(
            "total_nodes".to_string(),
            MetricValue::Counter(latest_report.statistics.total_nodes as u64),
        );
        metrics.insert(
            "healthy_nodes".to_string(),
            MetricValue::Counter(latest_report.statistics.healthy_nodes as u64),
        );
        metrics.insert(
            "unhealthy_nodes".to_string(),
            MetricValue::Counter(latest_report.statistics.unhealthy_nodes as u64),
        );

        Ok(serde_json::to_value(&metrics)?)
    }

    /// Get recent alerts
    pub fn get_recent_alerts(&self, limit: usize) -> Vec<HealthAlert> {
        let start = if self.alert_history.len() > limit {
            self.alert_history.len() - limit
        } else {
            0
        };

        self.alert_history[start..].to_vec()
    }

    /// Get alerts by severity
    pub fn get_alerts_by_severity(&self, severity: AlertSeverity) -> Vec<HealthAlert> {
        self.alert_history
            .iter()
            .filter(|a| a.severity == severity)
            .cloned()
            .collect()
    }

    /// Clear old alerts
    pub fn clear_old_alerts(&mut self, older_than: Duration) {
        let now = SystemTime::now();
        self.alert_history
            .retain(|alert| match now.duration_since(alert.triggered_at) {
                Ok(age) => age < older_than,
                Err(_) => true,
            });
    }

    /// Enable/disable metrics export
    pub fn set_metrics_enabled(&mut self, enabled: bool) {
        self.metrics_enabled = enabled;
    }

    /// Get report history
    pub fn get_report_history(&self, limit: usize) -> Vec<&HealthReport> {
        let start = if self.report_history.len() > limit {
            self.report_history.len() - limit
        } else {
            0
        };

        self.report_history[start..].iter().collect()
    }
}

impl Default for HealthReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_reporter_creation() {
        let reporter = HealthReporter::new();
        assert!(reporter.metrics_enabled);
    }

    #[tokio::test]
    async fn test_generate_report() {
        let reporter = HealthReporter::new();
        let report = reporter.generate_report().await.unwrap();

        assert_eq!(report.cluster_status, HealthStatus::Healthy);
    }

    #[test]
    fn test_alert_generation() {
        let mut reporter = HealthReporter::new();
        let report = HealthReport {
            timestamp: SystemTime::now(),
            cluster_status: HealthStatus::Unhealthy,
            cluster_score: HealthScore {
                score: 0.2,
                availability: 0.2,
                performance: 0.2,
                reliability: 0.2,
            },
            node_reports: HashMap::new(),
            cluster_alerts: Vec::new(),
            statistics: HealthStatistics {
                total_nodes: 0,
                healthy_nodes: 0,
                degraded_nodes: 0,
                unhealthy_nodes: 0,
                avg_latency_ms: 0.0,
                availability_percent: 20.0,
                total_checks: 0,
                total_failures: 0,
            },
        };

        let alerts = reporter.generate_alerts(&report);
        assert!(!alerts.is_empty());
    }

    #[tokio::test]
    async fn test_export_metrics() {
        let mut reporter = HealthReporter::new();

        // Add a report
        let report = HealthReport {
            timestamp: SystemTime::now(),
            cluster_status: HealthStatus::Healthy,
            cluster_score: HealthScore::perfect(),
            node_reports: HashMap::new(),
            cluster_alerts: Vec::new(),
            statistics: HealthStatistics {
                total_nodes: 3,
                healthy_nodes: 3,
                degraded_nodes: 0,
                unhealthy_nodes: 0,
                avg_latency_ms: 10.0,
                availability_percent: 100.0,
                total_checks: 1000,
                total_failures: 0,
            },
        };

        reporter.record_report(report);

        let metrics = reporter.export_metrics().await.unwrap();
        assert!(metrics.contains("rustydb_cluster_health_score"));
        assert!(metrics.contains("rustydb_cluster_nodes_total"));
    }
}
