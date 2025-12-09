// Health monitoring configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Health monitor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMonitorConfig {
    /// Health check interval
    pub check_interval: Duration,
    /// History retention duration
    pub history_retention: Duration,
    /// Maximum history entries per replica
    pub max_history_entries: usize,
    /// Enable automatic alerts
    pub enable_alerts: bool,
    /// Alert cool-down period
    pub alert_cooldown: Duration,
    /// Thresholds for health scoring
    pub thresholds: HealthThresholds,
    /// Enable trend analysis
    pub enable_trend_analysis: bool,
    /// Trend analysis window
    pub trend_window: Duration,
    /// Enable predictions
    pub enable_predictions: bool,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            history_retention: Duration::from_secs(86400 * 7), // 7 days
            max_history_entries: 10000,
            enable_alerts: true,
            alert_cooldown: Duration::from_secs(300), // 5 minutes
            thresholds: HealthThresholds::default(),
            enable_trend_analysis: true,
            trend_window: Duration::from_secs(3600), // 1 hour
            enable_predictions: false,
        }
    }
}

/// Health scoring thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthThresholds {
    /// Lag thresholds
    pub lag_bytes_warning: u64,
    pub lag_bytes_critical: u64,
    pub lag_time_warning: Duration,
    pub lag_time_critical: Duration,
    /// Throughput thresholds
    pub throughput_min_warning: f64,
    pub throughput_min_critical: f64,
    /// Error rate thresholds
    pub error_rate_warning: f64,
    pub error_rate_critical: f64,
    /// Resource usage thresholds
    pub cpu_usage_warning: f64,
    pub cpu_usage_critical: f64,
    pub memory_usage_warning: u64,
    pub memory_usage_critical: u64,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            lag_bytes_warning: 100 * 1024 * 1024, // 100MB
            lag_bytes_critical: 500 * 1024 * 1024, // 500MB
            lag_time_warning: Duration::from_secs(60),
            lag_time_critical: Duration::from_secs(300),
            throughput_min_warning: 1000.0, // bytes/sec
            throughput_min_critical: 100.0,
            error_rate_warning: 0.01, // 1%
            error_rate_critical: 0.05, // 5%
            cpu_usage_warning: 80.0,
            cpu_usage_critical: 95.0,
            memory_usage_warning: 8 * 1024 * 1024 * 1024, // 8GB
            memory_usage_critical: 15 * 1024 * 1024 * 1024, // 15GB
        }
    }
}
