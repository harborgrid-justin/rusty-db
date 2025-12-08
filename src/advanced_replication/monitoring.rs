//! # Replication Monitoring
//!
//! Comprehensive monitoring for replication lag, throughput,
//! error rates, and health dashboards.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque, BTreeMap};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use crate::Result;
use crate::error::DbError;

/// Replication lag measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationLag {
    /// Source identifier
    pub source_id: String,
    /// Target identifier
    pub target_id: String,
    /// Lag in milliseconds
    pub lag_ms: u64,
    /// Lag in bytes
    pub lag_bytes: u64,
    /// Lag in transactions
    pub lag_transactions: u64,
    /// Measured at
    pub measured_at: u64,
}

/// Throughput metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    /// Measurement period start
    pub period_start: u64,
    /// Measurement period end
    pub period_end: u64,
    /// Changes replicated
    pub changes: u64,
    /// Bytes replicated
    pub bytes: u64,
    /// Transactions replicated
    pub transactions: u64,
    /// Changes per second
    pub changes_per_second: f64,
    /// Bytes per second
    pub bytes_per_second: f64,
    /// Transactions per second
    pub transactions_per_second: f64,
}

/// Error rate metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRateMetrics {
    /// Measurement period start
    pub period_start: u64,
    /// Measurement period end
    pub period_end: u64,
    /// Total operations
    pub total_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Error rate (0.0-1.0)
    pub error_rate: f64,
    /// Errors by type
    pub errors_by_type: HashMap<String, u64>,
}

/// Conflict rate metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRateMetrics {
    /// Measurement period start
    pub period_start: u64,
    /// Measurement period end
    pub period_end: u64,
    /// Total changes
    pub total_changes: u64,
    /// Conflicts detected
    pub conflicts: u64,
    /// Conflict rate (0.0-1.0)
    pub conflict_rate: f64,
    /// Conflicts by type
    pub conflicts_by_type: HashMap<String, u64>,
}

/// Alert threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThreshold {
    /// Threshold name
    pub name: String,
    /// Metric to monitor
    pub metric: MetricType,
    /// Threshold value
    pub threshold: f64,
    /// Comparison operator
    pub operator: ComparisonOperator,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Enabled
    pub enabled: bool,
}

/// Type of metric to monitor
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetricType {
    LagMs,
    LagBytes,
    LagTransactions,
    ChangesPerSecond,
    BytesPerSecond,
    ErrorRate,
    ConflictRate,
}

/// Comparison operator for thresholds
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    Equal,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// Alert severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Generated alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Alert ID
    pub id: String,
    /// Threshold that triggered this alert
    pub threshold_name: String,
    /// Metric value
    pub metric_value: f64,
    /// Threshold value
    pub threshold_value: f64,
    /// Severity
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// Triggered at
    pub triggered_at: u64,
    /// Acknowledged
    pub acknowledged: bool,
}

/// Health status for a replication channel
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChannelHealth {
    Healthy,
    Warning,
    Critical,
    Offline,
}

/// Replication channel status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStatus {
    /// Channel ID
    pub channel_id: String,
    /// Source
    pub source: String,
    /// Target
    pub target: String,
    /// Health status
    pub health: ChannelHealth,
    /// Current lag
    pub lag: ReplicationLag,
    /// Last activity
    pub last_activity: u64,
    /// Active since
    pub active_since: u64,
}

/// Dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    /// Overall health
    pub overall_health: ChannelHealth,
    /// Channel statuses
    pub channels: Vec<ChannelStatus>,
    /// Recent throughput
    pub throughput: ThroughputMetrics,
    /// Recent error rate
    pub error_rate: ErrorRateMetrics,
    /// Recent conflict rate
    pub conflict_rate: ConflictRateMetrics,
    /// Active alerts
    pub active_alerts: Vec<Alert>,
    /// Generated at
    pub generated_at: u64,
}

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: u64,
    pub value: f64,
}

/// Replication monitor
pub struct ReplicationMonitor {
    /// Replication lag measurements
    lag_measurements: Arc<RwLock<VecDeque<ReplicationLag>>>,
    /// Throughput measurements
    throughput_measurements: Arc<RwLock<VecDeque<ThroughputMetrics>>>,
    /// Error rate measurements
    error_measurements: Arc<RwLock<VecDeque<ErrorRateMetrics>>>,
    /// Conflict rate measurements
    conflict_measurements: Arc<RwLock<VecDeque<ConflictRateMetrics>>>,
    /// Alert thresholds
    thresholds: Arc<RwLock<Vec<AlertThreshold>>>,
    /// Active alerts
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    /// Channel statuses
    channels: Arc<RwLock<HashMap<String, ChannelStatus>>>,
    /// Time series data
    time_series: Arc<RwLock<HashMap<String, VecDeque<TimeSeriesPoint>>>>,
    /// Measurement window (seconds)
    window_seconds: u64,
}

impl ReplicationMonitor {
    /// Create a new replication monitor
    pub fn new(window_seconds: u64) -> Self {
        Self {
            lag_measurements: Arc::new(RwLock::new(VecDeque::new())),
            throughput_measurements: Arc::new(RwLock::new(VecDeque::new())),
            error_measurements: Arc::new(RwLock::new(VecDeque::new())),
            conflict_measurements: Arc::new(RwLock::new(VecDeque::new())),
            thresholds: Arc::new(RwLock::new(Vec::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            channels: Arc::new(RwLock::new(HashMap::new())),
            time_series: Arc::new(RwLock::new(HashMap::new())),
            window_seconds,
        }
    }

    /// Record replication lag
    pub fn record_lag(&self, lag: ReplicationLag) -> Result<()> {
        let mut measurements = self.lag_measurements.write();
        measurements.push_back(lag.clone());

        // Cleanup old measurements
        self.cleanup_old_measurements(&mut measurements, lag.measured_at);

        // Record in time series
        self.record_time_series("lag_ms", lag.measured_at, lag.lag_ms as f64);

        // Check thresholds
        self.check_lag_thresholds(&lag)?;

        Ok(())
    }

    /// Record throughput metrics
    pub fn record_throughput(&self, metrics: ThroughputMetrics) -> Result<()> {
        let mut measurements = self.throughput_measurements.write();
        measurements.push_back(metrics.clone());

        // Cleanup old measurements
        self.cleanup_old_measurements(&mut measurements, metrics.period_end);

        // Record in time series
        self.record_time_series("changes_per_second", metrics.period_end, metrics.changes_per_second);
        self.record_time_series("bytes_per_second", metrics.period_end, metrics.bytes_per_second);

        // Check thresholds
        self.check_throughput_thresholds(&metrics)?;

        Ok(())
    }

    /// Record error rate
    pub fn record_error_rate(&self, metrics: ErrorRateMetrics) -> Result<()> {
        let mut measurements = self.error_measurements.write();
        measurements.push_back(metrics.clone());

        // Cleanup old measurements
        self.cleanup_old_measurements(&mut measurements, metrics.period_end);

        // Record in time series
        self.record_time_series("error_rate", metrics.period_end, metrics.error_rate);

        // Check thresholds
        self.check_error_thresholds(&metrics)?;

        Ok(())
    }

    /// Record conflict rate
    pub fn record_conflict_rate(&self, metrics: ConflictRateMetrics) -> Result<()> {
        let mut measurements = self.conflict_measurements.write();
        measurements.push_back(metrics.clone());

        // Cleanup old measurements
        self.cleanup_old_measurements(&mut measurements, metrics.period_end);

        // Record in time series
        self.record_time_series("conflict_rate", metrics.period_end, metrics.conflict_rate);

        // Check thresholds
        self.check_conflict_thresholds(&metrics)?;

        Ok(())
    }

    /// Cleanup old measurements outside the window
    fn cleanup_old_measurements<T>(&self, measurements: &mut VecDeque<T>, current_time: u64)
    where
        T: HasTimestamp,
    {
        let cutoff = current_time.saturating_sub(self.window_seconds * 1000);

        while let Some(front) = measurements.front() {
            if front.timestamp() < cutoff {
                measurements.pop_front();
            } else {
                break;
            }
        }
    }

    /// Record time series data
    fn record_time_series(&self, metric: &str, timestamp: u64, value: f64) {
        let mut time_series = self.time_series.write();
        let series = time_series.entry(metric.to_string()).or_insert_with(VecDeque::new);

        series.push_back(TimeSeriesPoint { timestamp, value });

        // Keep only last 1000 points
        while series.len() > 1000 {
            series.pop_front();
        }
    }

    /// Add an alert threshold
    pub fn add_threshold(&self, threshold: AlertThreshold) -> Result<()> {
        let mut thresholds = self.thresholds.write();
        thresholds.push(threshold);
        Ok(())
    }

    /// Remove an alert threshold
    pub fn remove_threshold(&self, name: &str) -> Result<()> {
        let mut thresholds = self.thresholds.write();
        thresholds.retain(|t| t.name != name);
        Ok(())
    }

    /// Check lag thresholds
    fn check_lag_thresholds(&self, lag: &ReplicationLag) -> Result<()> {
        let thresholds = self.thresholds.read();

        for threshold in thresholds.iter() {
            if !threshold.enabled {
                continue;
            }

            let metric_value = match threshold.metric {
                MetricType::LagMs => lag.lag_ms as f64,
                MetricType::LagBytes => lag.lag_bytes as f64,
                MetricType::LagTransactions => lag.lag_transactions as f64,
                _ => continue,
            };

            if self.threshold_exceeded(metric_value, threshold.threshold, &threshold.operator) {
                self.trigger_alert(threshold, metric_value)?;
            }
        }

        Ok(())
    }

    /// Check throughput thresholds
    fn check_throughput_thresholds(&self, metrics: &ThroughputMetrics) -> Result<()> {
        let thresholds = self.thresholds.read();

        for threshold in thresholds.iter() {
            if !threshold.enabled {
                continue;
            }

            let metric_value = match threshold.metric {
                MetricType::ChangesPerSecond => metrics.changes_per_second,
                MetricType::BytesPerSecond => metrics.bytes_per_second,
                _ => continue,
            };

            if self.threshold_exceeded(metric_value, threshold.threshold, &threshold.operator) {
                self.trigger_alert(threshold, metric_value)?;
            }
        }

        Ok(())
    }

    /// Check error thresholds
    fn check_error_thresholds(&self, metrics: &ErrorRateMetrics) -> Result<()> {
        let thresholds = self.thresholds.read();

        for threshold in thresholds.iter() {
            if !threshold.enabled {
                continue;
            }

            if threshold.metric == MetricType::ErrorRate {
                if self.threshold_exceeded(metrics.error_rate, threshold.threshold, &threshold.operator) {
                    self.trigger_alert(threshold, metrics.error_rate)?;
                }
            }
        }

        Ok(())
    }

    /// Check conflict thresholds
    fn check_conflict_thresholds(&self, metrics: &ConflictRateMetrics) -> Result<()> {
        let thresholds = self.thresholds.read();

        for threshold in thresholds.iter() {
            if !threshold.enabled {
                continue;
            }

            if threshold.metric == MetricType::ConflictRate {
                if self.threshold_exceeded(metrics.conflict_rate, threshold.threshold, &threshold.operator) {
                    self.trigger_alert(threshold, metrics.conflict_rate)?;
                }
            }
        }

        Ok(())
    }

    /// Check if threshold is exceeded
    fn threshold_exceeded(&self, value: f64, threshold: f64, operator: &ComparisonOperator) -> bool {
        match operator {
            ComparisonOperator::GreaterThan => value > threshold,
            ComparisonOperator::LessThan => value < threshold,
            ComparisonOperator::Equal => (value - threshold).abs() < f64::EPSILON,
            ComparisonOperator::GreaterThanOrEqual => value >= threshold,
            ComparisonOperator::LessThanOrEqual => value <= threshold,
        }
    }

    /// Trigger an alert
    fn trigger_alert(&self, threshold: &AlertThreshold, value: f64) -> Result<()> {
        let alert = Alert {
            id: format!("alert-{}", uuid::Uuid::new_v4()),
            threshold_name: threshold.name.clone(),
            metric_value: value,
            threshold_value: threshold.threshold,
            severity: threshold.severity.clone(),
            message: format!(
                "Threshold '{}' exceeded: {} {} {}",
                threshold.name,
                value,
                match threshold.operator {
                    ComparisonOperator::GreaterThan => ">",
                    ComparisonOperator::LessThan => "<",
                    ComparisonOperator::Equal => "=",
                    ComparisonOperator::GreaterThanOrEqual => ">=",
                    ComparisonOperator::LessThanOrEqual => "<=",
                },
                threshold.threshold
            ),
            triggered_at: Self::current_timestamp(),
            acknowledged: false,
        };

        let mut alerts = self.active_alerts.write();
        alerts.insert(alert.id.clone(), alert);

        Ok(())
    }

    /// Acknowledge an alert
    pub fn acknowledge_alert(&self, alert_id: &str) -> Result<()> {
        let mut alerts = self.active_alerts.write();

        let alert = alerts.get_mut(alert_id)
            .ok_or_else(|| DbError::Replication(
                format!("Alert {} not found", alert_id)
            ))?;

        alert.acknowledged = true;
        Ok(())
    }

    /// Get active alerts
    pub fn get_active_alerts(&self) -> Vec<Alert> {
        self.active_alerts.read().values().cloned().collect()
    }

    /// Update channel status
    pub fn update_channel_status(&self, status: ChannelStatus) -> Result<()> {
        let mut channels = self.channels.write();
        channels.insert(status.channel_id.clone(), status);
        Ok(())
    }

    /// Get channel status
    pub fn get_channel_status(&self, channel_id: &str) -> Option<ChannelStatus> {
        self.channels.read().get(channel_id).cloned()
    }

    /// Get all channel statuses
    pub fn get_all_channel_statuses(&self) -> Vec<ChannelStatus> {
        self.channels.read().values().cloned().collect()
    }

    /// Generate dashboard data
    pub fn generate_dashboard(&self) -> DashboardData {
        let channels = self.get_all_channel_statuses();

        // Determine overall health
        let overall_health = if channels.is_empty() {
            ChannelHealth::Offline
        } else {
            let critical_count = channels.iter().filter(|c| c.health == ChannelHealth::Critical).count();
            let warning_count = channels.iter().filter(|c| c.health == ChannelHealth::Warning).count();

            if critical_count > 0 {
                ChannelHealth::Critical
            } else if warning_count > 0 {
                ChannelHealth::Warning
            } else {
                ChannelHealth::Healthy
            }
        };

        // Get latest metrics
        let throughput = self.throughput_measurements.read()
            .back()
            .cloned()
            .unwrap_or_else(|| ThroughputMetrics {
                period_start: 0,
                period_end: 0,
                changes: 0,
                bytes: 0,
                transactions: 0,
                changes_per_second: 0.0,
                bytes_per_second: 0.0,
                transactions_per_second: 0.0,
            });

        let error_rate = self.error_measurements.read()
            .back()
            .cloned()
            .unwrap_or_else(|| ErrorRateMetrics {
                period_start: 0,
                period_end: 0,
                total_operations: 0,
                failed_operations: 0,
                error_rate: 0.0,
                errors_by_type: HashMap::new(),
            });

        let conflict_rate = self.conflict_measurements.read()
            .back()
            .cloned()
            .unwrap_or_else(|| ConflictRateMetrics {
                period_start: 0,
                period_end: 0,
                total_changes: 0,
                conflicts: 0,
                conflict_rate: 0.0,
                conflicts_by_type: HashMap::new(),
            });

        let active_alerts = self.get_active_alerts();

        DashboardData {
            overall_health,
            channels,
            throughput,
            error_rate,
            conflict_rate,
            active_alerts,
            generated_at: Self::current_timestamp(),
        }
    }

    /// Get time series data
    pub fn get_time_series(&self, metric: &str) -> Vec<TimeSeriesPoint> {
        self.time_series.read()
            .get(metric)
            .map(|series| series.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Current timestamp in milliseconds
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

/// Trait for types with timestamps
trait HasTimestamp {
    fn timestamp(&self) -> u64;
}

impl HasTimestamp for ReplicationLag {
    fn timestamp(&self) -> u64 {
        self.measured_at
    }
}

impl HasTimestamp for ThroughputMetrics {
    fn timestamp(&self) -> u64 {
        self.period_end
    }
}

impl HasTimestamp for ErrorRateMetrics {
    fn timestamp(&self) -> u64 {
        self.period_end
    }
}

impl HasTimestamp for ConflictRateMetrics {
    fn timestamp(&self) -> u64 {
        self.period_end
    }
}

impl Default for ReplicationMonitor {
    fn default() -> Self {
        Self::new(3600) // 1 hour window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_lag() {
        let monitor = ReplicationMonitor::new(3600);

        let lag = ReplicationLag {
            source_id: "source-1".to_string(),
            target_id: "target-1".to_string(),
            lag_ms: 1000,
            lag_bytes: 1024,
            lag_transactions: 10,
            measured_at: 0,
        };

        monitor.record_lag(lag).unwrap();

        let measurements = monitor.lag_measurements.read();
        assert_eq!(measurements.len(), 1);
    }

    #[test]
    fn test_threshold_alert() {
        let monitor = ReplicationMonitor::new(3600);

        let threshold = AlertThreshold {
            name: "high_lag".to_string(),
            metric: MetricType::LagMs,
            threshold: 500.0,
            operator: ComparisonOperator::GreaterThan,
            severity: AlertSeverity::Warning,
            enabled: true,
        };

        monitor.add_threshold(threshold).unwrap();

        let lag = ReplicationLag {
            source_id: "source-1".to_string(),
            target_id: "target-1".to_string(),
            lag_ms: 1000,
            lag_bytes: 1024,
            lag_transactions: 10,
            measured_at: 0,
        };

        monitor.record_lag(lag).unwrap();

        let alerts = monitor.get_active_alerts();
        assert_eq!(alerts.len(), 1);
    }
}


