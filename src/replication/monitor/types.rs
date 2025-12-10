// Health monitoring core types

use crate::replication::types::ReplicaId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

// Replica health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicaHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Critical,
    Unknown,
}

// Health check result
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub replica_id: ReplicaId,
    pub status: ReplicaHealthStatus,
    pub overall_score: f64,
    pub components: HealthComponents,
    pub timestamp: SystemTime,
    pub issues: Vec<HealthIssue>,
    pub metrics: HealthMetrics,
}

// Individual health components
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthComponents {
    pub replication_lag: ComponentHealth,
    pub connection: ComponentHealth,
    pub throughput: ComponentHealth,
    pub error_rate: ComponentHealth,
    pub resource_usage: ComponentHealth,
}

// Component health status
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: ReplicaHealthStatus,
    pub score: f64,
    pub value: f64,
    pub threshold: f64,
    pub unit: String,
}

// Health issue
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthIssue {
    pub severity: IssueSeverity,
    pub component: String,
    pub description: String,
    pub recommendation: Option<String>,
    pub detected_at: SystemTime,
}

// Issue severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

// Health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub replication_lag_bytes: u64,
    pub replication_lag_time: Duration,
    pub bytes_per_second: f64,
    pub records_per_second: f64,
    pub error_rate: f64,
    pub connection_count: u32,
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub disk_usage_bytes: u64,
}

impl Default for HealthMetrics {
    fn default() -> Self {
        Self {
            replication_lag_bytes: 0,
            replication_lag_time: Duration::ZERO,
            bytes_per_second: 0.0,
            records_per_second: 0.0,
            error_rate: 0.0,
            connection_count: 0,
            cpu_usage_percent: 0.0,
            memory_usage_bytes: 0,
            disk_usage_bytes: 0,
        }
    }
}

// Health alert
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    pub alert_id: String,
    pub replica_id: ReplicaId,
    pub severity: IssueSeverity,
    pub title: String,
    pub description: String,
    pub triggered_at: SystemTime,
    pub resolved_at: Option<SystemTime>,
    pub status: AlertStatus,
}

// Alert status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertStatus {
    Active,
    Acknowledged,
    Resolved,
    Suppressed,
}

// Health history entry
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthHistoryEntry {
    pub timestamp: SystemTime,
    pub replica_id: ReplicaId,
    pub status: ReplicaHealthStatus,
    pub score: f64,
    pub metrics: HealthMetrics,
}

// Aggregated health statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatistics {
    pub total_checks: u64,
    pub healthy_count: u64,
    pub degraded_count: u64,
    pub unhealthy_count: u64,
    pub critical_count: u64,
    pub average_score: f64,
    pub average_lag_bytes: u64,
    pub average_lag_time: Duration,
    pub uptime_percentage: f64,
}

impl Default for HealthStatistics {
    fn default() -> Self {
        Self {
            total_checks: 0,
            healthy_count: 0,
            degraded_count: 0,
            unhealthy_count: 0,
            critical_count: 0,
            average_score: 100.0,
            average_lag_bytes: 0,
            average_lag_time: Duration::ZERO,
            uptime_percentage: 100.0,
        }
    }
}

// Health trend analysis
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthTrend {
    pub replica_id: ReplicaId,
    pub period: Duration,
    pub trend_direction: TrendDirection,
    pub score_change: f64,
    pub lag_trend: TrendDirection,
    pub throughput_trend: TrendDirection,
    pub predictions: Vec<HealthPrediction>,
}

// Trend direction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
    Unknown,
}

// Health prediction
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthPrediction {
    pub metric: String,
    pub predicted_value: f64,
    pub confidence: f64,
    pub time_horizon: Duration,
}
