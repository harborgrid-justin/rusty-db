// HealthMonitor trait implementation

use async_trait::async_trait;
use std::time::{Duration, SystemTime};

use super::errors::HealthMonitorError;
use super::monitor::{HealthMonitor, ReplicationHealthMonitor};
use super::types::*;
use crate::replication::ReplicaId;

#[async_trait]
impl HealthMonitor for ReplicationHealthMonitor {
    async fn check_replica_health(
        &self,
        replica_id: &ReplicaId,
    ) -> Result<HealthCheckResult, HealthMonitorError> {
        let metrics = self.collect_metrics(replica_id)?;
        let components = self.calculate_component_health(&metrics);
        let overall_score = self.calculate_overall_score(&components);
        let status = self.determine_overall_status(overall_score);
        let issues = self.identify_issues(&components, &metrics);

        let result = HealthCheckResult {
            replica_id: replica_id.clone(),
            status,
            overall_score,
            components,
            timestamp: SystemTime::now(),
            issues,
            metrics,
        };

        self.record_health_history(&result);
        self.update_statistics(replica_id, &result);

        Ok(result)
    }

    async fn check_all_replicas(&self) -> Result<Vec<HealthCheckResult>, HealthMonitorError> {
        // In production, this would query all known replicas
        let replica_ids = vec![
            ReplicaId::new("replica-1").unwrap(),
            ReplicaId::new("replica-2").unwrap(),
        ];

        let mut results = Vec::new();
        for replica_id in replica_ids {
            if let Ok(result) = self.check_replica_health(&replica_id).await {
                results.push(result);
            }
        }

        Ok(results)
    }

    async fn get_health_history(
        &self,
        replica_id: &ReplicaId,
        duration: Duration,
    ) -> Result<Vec<HealthHistoryEntry>, HealthMonitorError> {
        let history = self.history.read();
        let entries =
            history
                .get(replica_id)
                .ok_or_else(|| HealthMonitorError::ReplicaNotFound {
                    replica_id: replica_id.to_string(),
                })?;

        let now = SystemTime::now();
        let cutoff = now - duration;

        Ok(entries
            .iter()
            .filter(|entry| entry.timestamp >= cutoff)
            .cloned()
            .collect())
    }

    async fn get_statistics(
        &self,
        replica_id: Option<&ReplicaId>,
    ) -> Result<HealthStatistics, HealthMonitorError> {
        let statistics = self.statistics.read();

        if let Some(replica_id) = replica_id {
            statistics
                .get(replica_id)
                .cloned()
                .ok_or_else(|| HealthMonitorError::ReplicaNotFound {
                    replica_id: replica_id.to_string(),
                })
        } else {
            // Aggregate statistics across all replicas
            if statistics.is_empty() {
                return Ok(HealthStatistics::default());
            }

            let total_checks: u64 = statistics.values().map(|s| s.total_checks).sum();
            let healthy_count: u64 = statistics.values().map(|s| s.healthy_count).sum();
            let degraded_count: u64 = statistics.values().map(|s| s.degraded_count).sum();
            let unhealthy_count: u64 = statistics.values().map(|s| s.unhealthy_count).sum();
            let critical_count: u64 = statistics.values().map(|s| s.critical_count).sum();

            let average_score = if !statistics.is_empty() {
                statistics.values().map(|s| s.average_score).sum::<f64>() / statistics.len() as f64
            } else {
                0.0
            };

            let average_lag_bytes = if !statistics.is_empty() {
                statistics
                    .values()
                    .map(|s| s.average_lag_bytes)
                    .sum::<u64>()
                    / statistics.len() as u64
            } else {
                0
            };

            let uptime_percentage = if total_checks > 0 {
                (healthy_count + degraded_count) as f64 / total_checks as f64 * 100.0
            } else {
                100.0
            };

            Ok(HealthStatistics {
                total_checks,
                healthy_count,
                degraded_count,
                unhealthy_count,
                critical_count,
                average_score,
                average_lag_bytes,
                average_lag_time: Duration::from_secs(30), // Simplified
                uptime_percentage,
            })
        }
    }

    async fn get_active_alerts(&self) -> Result<Vec<HealthAlert>, HealthMonitorError> {
        let alerts = self.alerts.read();
        Ok(alerts
            .values()
            .filter(|alert| alert.status == AlertStatus::Active)
            .cloned()
            .collect())
    }

    async fn acknowledge_alert(&self, alert_id: &str) -> Result<(), HealthMonitorError> {
        let mut alerts = self.alerts.write();
        if let Some(alert) = alerts.get_mut(alert_id) {
            alert.status = AlertStatus::Acknowledged;
            Ok(())
        } else {
            Err(HealthMonitorError::AlertDeliveryFailed {
                reason: format!("Alert not found: {}", alert_id),
            })
        }
    }

    async fn get_health_trend(
        &self,
        replica_id: &ReplicaId,
    ) -> Result<HealthTrend, HealthMonitorError> {
        let history = self
            .get_health_history(replica_id, self.config.trend_window)
            .await?;

        if history.is_empty() {
            return Ok(HealthTrend {
                replica_id: replica_id.clone(),
                period: self.config.trend_window,
                trend_direction: TrendDirection::Unknown,
                score_change: 0.0,
                lag_trend: TrendDirection::Unknown,
                throughput_trend: TrendDirection::Unknown,
                predictions: Vec::new(),
            });
        }

        // Calculate trends
        let first_score = history.first().map(|e| e.score).unwrap_or(0.0);
        let last_score = history.last().map(|e| e.score).unwrap_or(0.0);
        let score_change = last_score - first_score;

        let trend_direction = if score_change > 5.0 {
            TrendDirection::Improving
        } else if score_change < -5.0 {
            TrendDirection::Degrading
        } else {
            TrendDirection::Stable
        };

        // Calculate lag trend
        let first_lag = history
            .first()
            .map(|e| e.metrics.replication_lag_bytes)
            .unwrap_or(0);
        let last_lag = history
            .last()
            .map(|e| e.metrics.replication_lag_bytes)
            .unwrap_or(0);

        let lag_trend = if last_lag < first_lag && (first_lag - last_lag) > 10 * 1024 * 1024 {
            TrendDirection::Improving
        } else if last_lag > first_lag && (last_lag - first_lag) > 10 * 1024 * 1024 {
            TrendDirection::Degrading
        } else {
            TrendDirection::Stable
        };

        Ok(HealthTrend {
            replica_id: replica_id.clone(),
            period: self.config.trend_window,
            trend_direction,
            score_change,
            lag_trend,
            throughput_trend: TrendDirection::Stable,
            predictions: Vec::new(),
        })
    }
}
