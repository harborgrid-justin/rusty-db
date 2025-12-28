// Data Quality Monitor
// Tracks quality scores, trends, and generates alerts

use crate::error::{DbError, Result};
use crate::quality::quality_rules::{RuleViolation, RuleSeverity};
use crate::quality::quality_profiler::ColumnProfile;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, Duration};

/// Quality dimension categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QualityDimension {
    Completeness,
    Accuracy,
    Consistency,
    Validity,
    Uniqueness,
    Timeliness,
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
    Unknown,
}

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    Critical,
    Warning,
    Info,
}

/// Quality score for a table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScore {
    pub table_name: String,
    pub overall_score: f64,
    pub dimension_scores: HashMap<QualityDimension, f64>,
    pub total_violations: usize,
    pub critical_violations: usize,
    pub timestamp: SystemTime,
}

impl QualityScore {
    pub fn new(table_name: String) -> Self {
        Self {
            table_name,
            overall_score: 100.0,
            dimension_scores: HashMap::new(),
            total_violations: 0,
            critical_violations: 0,
            timestamp: SystemTime::now(),
        }
    }
}

/// Quality trend data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityTrendPoint {
    pub timestamp: SystemTime,
    pub score: f64,
    pub violation_count: usize,
}

/// Quality trend information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityTrend {
    pub table_name: String,
    pub dimension: Option<QualityDimension>,
    pub direction: TrendDirection,
    pub data_points: Vec<QualityTrendPoint>,
    pub change_rate: f64,
    pub period: Duration,
}

/// Quality alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAlert {
    pub id: String,
    pub table_name: String,
    pub alert_type: String,
    pub level: AlertLevel,
    pub message: String,
    pub details: HashMap<String, String>,
    pub timestamp: SystemTime,
    pub acknowledged: bool,
}

/// Quality metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub total_tables_monitored: usize,
    pub tables_below_threshold: Vec<String>,
    pub average_quality_score: f64,
    pub total_violations_24h: usize,
    pub critical_violations_24h: usize,
    pub active_alerts: usize,
    pub timestamp: SystemTime,
}

/// Dashboard data for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub metrics: QualityMetrics,
    pub top_issues: Vec<RuleViolation>,
    pub recent_trends: HashMap<String, QualityTrend>,
    pub active_alerts: Vec<QualityAlert>,
    pub score_distribution: HashMap<String, f64>,
}

/// Quality monitor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub quality_threshold: f64,
    pub alert_threshold: f64,
    pub trend_window_size: usize,
    pub max_trend_history: usize,
    pub enable_auto_alerts: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            quality_threshold: 95.0,
            alert_threshold: 90.0,
            trend_window_size: 10,
            max_trend_history: 100,
            enable_auto_alerts: true,
        }
    }
}

/// Main quality monitor
pub struct QualityMonitor {
    config: MonitorConfig,
    score_history: HashMap<String, VecDeque<QualityScore>>,
    alerts: HashMap<String, QualityAlert>,
    next_alert_id: u64,
}

impl QualityMonitor {
    pub fn new() -> Self {
        Self {
            config: MonitorConfig::default(),
            score_history: HashMap::new(),
            alerts: HashMap::new(),
            next_alert_id: 1,
        }
    }

    pub fn with_config(config: MonitorConfig) -> Self {
        Self {
            config,
            score_history: HashMap::new(),
            alerts: HashMap::new(),
            next_alert_id: 1,
        }
    }

    /// Calculate quality score for a table
    pub fn calculate_quality_score(
        &mut self,
        table_name: &str,
        violations: &[RuleViolation],
        profiles: &[ColumnProfile],
    ) -> Result<QualityScore> {
        let mut score = QualityScore::new(table_name.to_string());

        // Count violations by severity
        let mut critical_count = 0;
        let mut high_count = 0;
        let mut medium_count = 0;
        let mut low_count = 0;

        for violation in violations {
            match violation.severity {
                RuleSeverity::Critical => critical_count += 1,
                RuleSeverity::High => high_count += 1,
                RuleSeverity::Medium => medium_count += 1,
                RuleSeverity::Low => low_count += 1,
                RuleSeverity::Info => {}
            }
        }

        score.total_violations = violations.len();
        score.critical_violations = critical_count;

        // Calculate dimension scores
        score.dimension_scores.insert(
            QualityDimension::Completeness,
            self.calculate_completeness_score(profiles),
        );

        score.dimension_scores.insert(
            QualityDimension::Accuracy,
            self.calculate_accuracy_score(violations),
        );

        score.dimension_scores.insert(
            QualityDimension::Consistency,
            self.calculate_consistency_score(violations),
        );

        score.dimension_scores.insert(
            QualityDimension::Validity,
            self.calculate_validity_score(violations),
        );

        score.dimension_scores.insert(
            QualityDimension::Uniqueness,
            self.calculate_uniqueness_score(violations),
        );

        score.dimension_scores.insert(
            QualityDimension::Timeliness,
            100.0, // Default to perfect score for timeliness
        );

        // Calculate overall score (weighted average)
        let weights: HashMap<QualityDimension, f64> = [
            (QualityDimension::Completeness, 0.25),
            (QualityDimension::Accuracy, 0.25),
            (QualityDimension::Consistency, 0.15),
            (QualityDimension::Validity, 0.20),
            (QualityDimension::Uniqueness, 0.10),
            (QualityDimension::Timeliness, 0.05),
        ].iter().cloned().collect();

        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for (dimension, &weight) in &weights {
            if let Some(&dim_score) = score.dimension_scores.get(dimension) {
                weighted_sum += dim_score * weight;
                total_weight += weight;
            }
        }

        score.overall_score = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            100.0
        };

        // Apply violation penalties
        let violation_penalty = (critical_count * 10 + high_count * 5 + medium_count * 2 + low_count) as f64;
        score.overall_score = (score.overall_score - violation_penalty).max(0.0);

        // Store score in history
        self.record_score(score.clone())?;

        // Generate alerts if needed
        if self.config.enable_auto_alerts {
            self.check_and_generate_alerts(&score)?;
        }

        Ok(score)
    }

    /// Calculate completeness score from profiles
    fn calculate_completeness_score(&self, profiles: &[ColumnProfile]) -> f64 {
        if profiles.is_empty() {
            return 100.0;
        }

        let mut total_completeness = 0.0;
        for profile in profiles {
            let null_percent = profile.distribution.null_percentage;
            let completeness = 100.0 - null_percent;
            total_completeness += completeness;
        }

        total_completeness / profiles.len() as f64
    }

    /// Calculate accuracy score based on violations
    fn calculate_accuracy_score(&self, violations: &[RuleViolation]) -> f64 {
        let format_violations = violations.iter()
            .filter(|v| v.rule_name.contains("Format") || v.rule_name.contains("Pattern"))
            .count();

        let penalty = (format_violations as f64) * 5.0;
        (100.0 - penalty).max(0.0)
    }

    /// Calculate consistency score
    fn calculate_consistency_score(&self, violations: &[RuleViolation]) -> f64 {
        let consistency_violations = violations.iter()
            .filter(|v| v.rule_name.contains("Consistency") || v.rule_name.contains("Reference"))
            .count();

        let penalty = (consistency_violations as f64) * 5.0;
        (100.0 - penalty).max(0.0)
    }

    /// Calculate validity score
    fn calculate_validity_score(&self, violations: &[RuleViolation]) -> f64 {
        let validity_violations = violations.iter()
            .filter(|v| v.rule_name.contains("Range") || v.rule_name.contains("Valid"))
            .count();

        let penalty = (validity_violations as f64) * 5.0;
        (100.0 - penalty).max(0.0)
    }

    /// Calculate uniqueness score
    fn calculate_uniqueness_score(&self, violations: &[RuleViolation]) -> f64 {
        let uniqueness_violations = violations.iter()
            .filter(|v| v.rule_name.contains("Unique") || v.rule_name.contains("Duplicate"))
            .count();

        let penalty = (uniqueness_violations as f64) * 5.0;
        (100.0 - penalty).max(0.0)
    }

    /// Record quality score in history
    fn record_score(&mut self, score: QualityScore) -> Result<()> {
        let history = self.score_history
            .entry(score.table_name.clone())
            .or_insert_with(VecDeque::new);

        history.push_back(score);

        // Limit history size
        while history.len() > self.config.max_trend_history {
            history.pop_front();
        }

        Ok(())
    }

    /// Get quality trend for a table
    pub fn get_quality_trend(
        &self,
        table_name: &str,
        dimension: Option<QualityDimension>,
    ) -> Result<QualityTrend> {
        let history = self.score_history
            .get(table_name)
            .ok_or_else(|| DbError::NotFound(format!("No history for table: {}", table_name)))?;

        if history.is_empty() {
            return Ok(QualityTrend {
                table_name: table_name.to_string(),
                dimension,
                direction: TrendDirection::Unknown,
                data_points: Vec::new(),
                change_rate: 0.0,
                period: Duration::from_secs(0),
            });
        }

        // Collect data points
        let mut data_points = Vec::new();
        for score in history.iter().rev().take(self.config.trend_window_size) {
            let point_score = match dimension {
                Some(dim) => *score.dimension_scores.get(&dim).unwrap_or(&100.0),
                None => score.overall_score,
            };

            data_points.push(QualityTrendPoint {
                timestamp: score.timestamp,
                score: point_score,
                violation_count: score.total_violations,
            });
        }

        data_points.reverse();

        // Calculate trend direction and change rate
        let (direction, change_rate) = self.calculate_trend_direction(&data_points);

        // Calculate period
        let period = if let (Some(first), Some(last)) = (data_points.first(), data_points.last()) {
            last.timestamp.duration_since(first.timestamp).unwrap_or(Duration::from_secs(0))
        } else {
            Duration::from_secs(0)
        };

        Ok(QualityTrend {
            table_name: table_name.to_string(),
            dimension,
            direction,
            data_points,
            change_rate,
            period,
        })
    }

    /// Calculate trend direction from data points
    fn calculate_trend_direction(&self, points: &[QualityTrendPoint]) -> (TrendDirection, f64) {
        if points.len() < 2 {
            return (TrendDirection::Unknown, 0.0);
        }

        // Simple linear regression to detect trend
        let n = points.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;

        for (i, point) in points.iter().enumerate() {
            let x = i as f64;
            let y = point.score;
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);

        let direction = if slope > 0.5 {
            TrendDirection::Improving
        } else if slope < -0.5 {
            TrendDirection::Degrading
        } else {
            TrendDirection::Stable
        };

        (direction, slope)
    }

    /// Check thresholds and generate alerts
    fn check_and_generate_alerts(&mut self, score: &QualityScore) -> Result<()> {
        // Check overall score threshold
        if score.overall_score < self.config.alert_threshold {
            let level = if score.overall_score < self.config.quality_threshold - 10.0 {
                AlertLevel::Critical
            } else {
                AlertLevel::Warning
            };

            self.create_alert(
                &score.table_name,
                "Quality Score Below Threshold",
                level,
                format!(
                    "Quality score {:.2}% is below threshold {:.2}%",
                    score.overall_score, self.config.alert_threshold
                ),
                HashMap::new(),
            )?;
        }

        // Check critical violations
        if score.critical_violations > 0 {
            self.create_alert(
                &score.table_name,
                "Critical Violations Detected",
                AlertLevel::Critical,
                format!("Found {} critical violations", score.critical_violations),
                HashMap::new(),
            )?;
        }

        Ok(())
    }

    /// Create a new alert
    fn create_alert(
        &mut self,
        table_name: &str,
        alert_type: &str,
        level: AlertLevel,
        message: String,
        details: HashMap<String, String>,
    ) -> Result<()> {
        let alert_id = format!("alert-{}", self.next_alert_id);
        self.next_alert_id += 1;

        let alert = QualityAlert {
            id: alert_id.clone(),
            table_name: table_name.to_string(),
            alert_type: alert_type.to_string(),
            level,
            message,
            details,
            timestamp: SystemTime::now(),
            acknowledged: false,
        };

        self.alerts.insert(alert_id, alert);
        Ok(())
    }

    /// Get all active alerts
    pub fn get_active_alerts(&self) -> Vec<QualityAlert> {
        self.alerts.values()
            .filter(|a| !a.acknowledged)
            .cloned()
            .collect()
    }

    /// Acknowledge an alert
    pub fn acknowledge_alert(&mut self, alert_id: &str) -> Result<()> {
        if let Some(alert) = self.alerts.get_mut(alert_id) {
            alert.acknowledged = true;
            Ok(())
        } else {
            Err(DbError::NotFound(format!("Alert not found: {}", alert_id)))
        }
    }

    /// Get dashboard data
    pub fn get_dashboard_data(&self) -> Result<DashboardData> {
        let metrics = self.calculate_metrics()?;
        let active_alerts = self.get_active_alerts();

        // Get recent trends for all monitored tables
        let mut recent_trends = HashMap::new();
        for table_name in self.score_history.keys() {
            if let Ok(trend) = self.get_quality_trend(table_name, None) {
                recent_trends.insert(table_name.clone(), trend);
            }
        }

        // Calculate score distribution
        let mut score_distribution = HashMap::new();
        for (table_name, history) in &self.score_history {
            if let Some(latest) = history.back() {
                score_distribution.insert(table_name.clone(), latest.overall_score);
            }
        }

        Ok(DashboardData {
            metrics,
            top_issues: Vec::new(), // Would be populated from recent violations
            recent_trends,
            active_alerts,
            score_distribution,
        })
    }

    /// Calculate quality metrics
    fn calculate_metrics(&self) -> Result<QualityMetrics> {
        let total_tables = self.score_history.len();
        let mut tables_below_threshold = Vec::new();
        let mut total_score = 0.0;
        let mut total_violations = 0;
        let mut critical_violations = 0;

        let now = SystemTime::now();
        let cutoff = now - Duration::from_secs(24 * 3600);

        for (table_name, history) in &self.score_history {
            if let Some(latest) = history.back() {
                if latest.overall_score < self.config.quality_threshold {
                    tables_below_threshold.push(table_name.clone());
                }
                total_score += latest.overall_score;

                // Count violations in last 24 hours
                if latest.timestamp >= cutoff {
                    total_violations += latest.total_violations;
                    critical_violations += latest.critical_violations;
                }
            }
        }

        let average_score = if total_tables > 0 {
            total_score / total_tables as f64
        } else {
            100.0
        };

        let active_alert_count = self.get_active_alerts().len();

        Ok(QualityMetrics {
            total_tables_monitored: total_tables,
            tables_below_threshold,
            average_quality_score: average_score,
            total_violations_24h: total_violations,
            critical_violations_24h: critical_violations,
            active_alerts: active_alert_count,
            timestamp: now,
        })
    }
}

impl Default for QualityMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_creation() {
        let monitor = QualityMonitor::new();
        assert_eq!(monitor.config.quality_threshold, 95.0);
        assert_eq!(monitor.score_history.len(), 0);
    }

    #[test]
    fn test_quality_score_creation() {
        let score = QualityScore::new("test_table".to_string());
        assert_eq!(score.table_name, "test_table");
        assert_eq!(score.overall_score, 100.0);
    }

    #[test]
    fn test_trend_direction() {
        let monitor = QualityMonitor::new();
        let points = vec![
            QualityTrendPoint {
                timestamp: SystemTime::now(),
                score: 90.0,
                violation_count: 5,
            },
            QualityTrendPoint {
                timestamp: SystemTime::now(),
                score: 92.0,
                violation_count: 3,
            },
            QualityTrendPoint {
                timestamp: SystemTime::now(),
                score: 94.0,
                violation_count: 2,
            },
        ];
        let (direction, _) = monitor.calculate_trend_direction(&points);
        assert_eq!(direction, TrendDirection::Improving);
    }
}
