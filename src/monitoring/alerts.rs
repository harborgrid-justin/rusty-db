// Performance Alerts
// Threshold-based alerting, anomaly detection, alert routing and escalation

use std::fmt;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Instant, Duration};


/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "INFO"),
            AlertSeverity::Warning => write!(f, "WARNING"),
            AlertSeverity::Error => write!(f, "ERROR"),
            AlertSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Alert state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertState {
    Active,
    Acknowledged,
    Resolved,
    Suppressed,
}

impl fmt::Display for AlertState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertState::Active => write!(f, "ACTIVE"),
            AlertState::Acknowledged => write!(f, "ACKNOWLEDGED"),
            AlertState::Resolved => write!(f, "RESOLVED"),
            AlertState::Suppressed => write!(f, "SUPPRESSED"),
        }
    }
}

/// Alert category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertCategory {
    Performance,
    Availability,
    Capacity,
    Security,
    DataIntegrity,
    Replication,
    Backup,
    Configuration,
}

impl fmt::Display for AlertCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertCategory::Performance => write!(f, "Performance"),
            AlertCategory::Availability => write!(f, "Availability"),
            AlertCategory::Capacity => write!(f, "Capacity"),
            AlertCategory::Security => write!(f, "Security"),
            AlertCategory::DataIntegrity => write!(f, "Data Integrity"),
            AlertCategory::Replication => write!(f, "Replication"),
            AlertCategory::Backup => write!(f, "Backup"),
            AlertCategory::Configuration => write!(f, "Configuration"),
        }
    }
}

/// Alert definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: u64,
    pub name: String,
    pub category: AlertCategory,
    pub severity: AlertSeverity,
    pub state: AlertState,
    pub message: String,
    pub details: HashMap<String, String>,
    pub triggered_at: SystemTime,
    pub acknowledged_at: Option<SystemTime>,
    pub resolved_at: Option<SystemTime>,
    pub acknowledged_by: Option<String>,
    pub escalation_level: u8,
    pub occurrence_count: u64,
}

impl Alert {
    pub fn new(
        id: u64,
        name: impl Into<String>,
        category: AlertCategory,
        severity: AlertSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            category,
            severity,
            state: AlertState::Active,
            message: message.into(),
            details: HashMap::new(),
            triggered_at: SystemTime::now(),
            acknowledged_at: None,
            resolved_at: None,
            acknowledged_by: None,
            escalation_level: 0,
            occurrence_count: 1,
        }
    }

    pub fn add_detail(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.details.insert(key.into(), value.into());
    }

    pub fn acknowledge(&mut self, user: impl Into<String>) {
        self.state = AlertState::Acknowledged;
        self.acknowledged_at = Some(SystemTime::now());
        self.acknowledged_by = Some(user.into());
    }

    pub fn resolve(&mut self) {
        self.state = AlertState::Resolved;
        self.resolved_at = Some(SystemTime::now());
    }

    pub fn suppress(&mut self) {
        self.state = AlertState::Suppressed;
    }

    pub fn escalate(&mut self) {
        self.escalation_level += 1;
    }

    pub fn increment_occurrence(&mut self) {
        self.occurrence_count += 1;
    }

    pub fn is_active(&self) -> bool {
        self.state == AlertState::Active
    }

    pub fn duration(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.triggered_at)
            .unwrap_or(Duration::ZERO)
    }
}

/// Threshold-based alert rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdRule {
    pub name: String,
    pub metric_name: String,
    pub threshold: f64,
    pub comparison: ComparisonOperator,
    pub duration: Duration,
    pub severity: AlertSeverity,
    pub category: AlertCategory,
    pub enabled: bool,
    pub cooldown_period: Duration,
}

impl ThresholdRule {
    pub fn new(
        name: impl Into<String>,
        metric_name: impl Into<String>,
        threshold: f64,
        comparison: ComparisonOperator,
        severity: AlertSeverity,
    ) -> Self {
        Self {
            name: name.into(),
            metric_name: metric_name.into(),
            threshold,
            comparison,
            duration: Duration::from_secs(60),
            severity,
            category: AlertCategory::Performance,
            enabled: true,
            cooldown_period: Duration::from_secs(300),
        }
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn with_category(mut self, category: AlertCategory) -> Self {
        self.category = category;
        self
    }

    pub fn with_cooldown(mut self, cooldown: Duration) -> Self {
        self.cooldown_period = cooldown;
        self
    }

    pub fn evaluate(&self, value: f64) -> bool {
        match self.comparison {
            ComparisonOperator::GreaterThan => value > self.threshold,
            ComparisonOperator::GreaterThanOrEqual => value >= self.threshold,
            ComparisonOperator::LessThan => value < self.threshold,
            ComparisonOperator::LessThanOrEqual => value <= self.threshold,
            ComparisonOperator::Equal => (value - self.threshold).abs() < f64::EPSILON,
            ComparisonOperator::NotEqual => (value - self.threshold).abs() >= f64::EPSILON,
        }
    }
}

/// Comparison operators for threshold rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

/// Anomaly detection algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyDetectionAlgorithm {
    StandardDeviation,
    InterquartileRange,
    MovingAverage,
    ExponentialSmoothing,
}

/// Anomaly detection rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyRule {
    pub name: String,
    pub metric_name: String,
    pub algorithm: AnomalyDetectionAlgorithm,
    pub sensitivity: f64,
    pub window_size: usize,
    pub severity: AlertSeverity,
    pub category: AlertCategory,
    pub enabled: bool,
}

impl AnomalyRule {
    pub fn new(
        name: impl Into<String>,
        metric_name: impl Into<String>,
        algorithm: AnomalyDetectionAlgorithm,
        sensitivity: f64,
    ) -> Self {
        Self {
            name: name.into(),
            metric_name: metric_name.into(),
            algorithm,
            sensitivity,
            window_size: 100,
            severity: AlertSeverity::Warning,
            category: AlertCategory::Performance,
            enabled: true,
        }
    }

    pub fn detect_anomaly(&self, history: &[f64], current_value: f64) -> bool {
        if history.is_empty() {
            return false;
        }

        match self.algorithm {
            AnomalyDetectionAlgorithm::StandardDeviation => {
                self.detect_stddev(history, current_value)
            }
            AnomalyDetectionAlgorithm::InterquartileRange => {
                self.detect_iqr(history, current_value)
            }
            AnomalyDetectionAlgorithm::MovingAverage => {
                self.detect_moving_average(history, current_value)
            }
            AnomalyDetectionAlgorithm::ExponentialSmoothing => {
                self.detect_exponential_smoothing(history, current_value)
            }
        }
    }

    fn detect_stddev(&self, history: &[f64], current_value: f64) -> bool {
        let mean = history.iter().sum::<f64>() / history.len() as f64;
        let variance = history.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / history.len() as f64;
        let stddev = variance.sqrt();

        let z_score = (current_value - mean).abs() / stddev;
        z_score > self.sensitivity
    }

    fn detect_iqr(&self, history: &[f64], current_value: f64) -> bool {
        let mut sorted = history.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let q1_idx = sorted.len() / 4;
        let q3_idx = 3 * sorted.len() / 4;

        if q1_idx >= sorted.len() || q3_idx >= sorted.len() {
            return false;
        }

        let q1 = sorted[q1_idx];
        let q3 = sorted[q3_idx];
        let iqr = q3 - q1;

        let lower_bound = q1 - (self.sensitivity * iqr);
        let upper_bound = q3 + (self.sensitivity * iqr);

        current_value < lower_bound || current_value > upper_bound
    }

    fn detect_moving_average(&self, history: &[f64], current_value: f64) -> bool {
        let window_size = self.window_size.min(history.len());
        let recent = &history[history.len().saturating_sub(window_size)..];
        let avg = recent.iter().sum::<f64>() / recent.len() as f64;

        (current_value - avg).abs() > (avg * self.sensitivity)
    }

    fn detect_exponential_smoothing(&self, history: &[f64], current_value: f64) -> bool {
        if history.is_empty() {
            return false;
        }

        let alpha = 0.3; // Smoothing factor
        let mut forecast = history[0];

        for &value in history {
            forecast = alpha * value + (1.0 - alpha) * forecast;
        }

        (current_value - forecast).abs() > (forecast * self.sensitivity)
    }
}

/// Alert manager
pub struct AlertManager {
    alerts: Arc<RwLock<HashMap<u64, Alert>>>,
    alert_history: Arc<RwLock<VecDeque<Alert>>>,
    threshold_rules: Arc<RwLock<HashMap<String, ThresholdRule>>>,
    anomaly_rules: Arc<RwLock<HashMap<String, AnomalyRule>>>,
    metric_history: Arc<RwLock<HashMap<String, VecDeque<(SystemTime, f64)>>>>,
    last_alert_id: Arc<RwLock<u64>>,
    last_trigger_time: Arc<RwLock<HashMap<String, SystemTime>>>,
    max_history: usize,
    max_metric_history: usize,
}

impl AlertManager {
    pub fn new(max_history: usize, max_metric_history: usize) -> Self {
        Self {
            alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            threshold_rules: Arc::new(RwLock::new(HashMap::new())),
            anomaly_rules: Arc::new(RwLock::new(HashMap::new())),
            metric_history: Arc::new(RwLock::new(HashMap::new())),
            last_alert_id: Arc::new(RwLock::new(0)),
            last_trigger_time: Arc::new(RwLock::new(HashMap::new())),
            max_history,
            max_metric_history,
        }
    }

    pub fn add_threshold_rule(&self, rule: ThresholdRule) {
        self.threshold_rules.write().insert(rule.name.clone(), rule);
    }

    pub fn add_anomaly_rule(&self, rule: AnomalyRule) {
        self.anomaly_rules.write().insert(rule.name.clone(), rule);
    }

    pub fn remove_threshold_rule(&self, name: &str) -> bool {
        self.threshold_rules.write().remove(name).is_some()
    }

    pub fn remove_anomaly_rule(&self, name: &str) -> bool {
        self.anomaly_rules.write().remove(name).is_some()
    }

    pub fn record_metric(&self, metric_name: impl Into<String>, value: f64) {
        let metric_name = metric_name.into();
        let timestamp = SystemTime::now();

        let mut history = self.metric_history.write();
        let entries = history.entry(metric_name.clone()).or_insert_with(VecDeque::new);

        if entries.len() >= self.max_metric_history {
            entries.pop_front();
        }

        entries.push_back((timestamp, value));
        drop(history);

        // Evaluate threshold rules
        self.evaluate_threshold_rules(&metric_name, value);

        // Evaluate anomaly rules
        self.evaluate_anomaly_rules(&metric_name, value);
    }

    fn evaluate_threshold_rules(&self, metric_name: &str, value: f64) {
        let rules = self.threshold_rules.read();

        for rule in rules.values() {
            if !rule.enabled || rule.metric_name != metric_name {
                continue;
            }

            // Check cooldown period
            if let Some(last_trigger) = self.last_trigger_time.read().get(&rule.name) {
                if let Ok(elapsed) = SystemTime::now().duration_since(*last_trigger) {
                    if elapsed < rule.cooldown_period {
                        continue;
                    }
                }
            }

            if rule.evaluate(value) {
                self.trigger_alert(
                    rule.name.clone(),
                    rule.category,
                    rule.severity,
                    format!("Threshold exceeded: {} = {:.2} (threshold: {:.2})",
                        metric_name, value, rule.threshold),
                );

                self.last_trigger_time.write().insert(rule.name.clone(), SystemTime::now());
            }
        }
    }

    fn evaluate_anomaly_rules(&self, metric_name: &str, value: f64) {
        let rules = self.anomaly_rules.read();

        for rule in rules.values() {
            if !rule.enabled || rule.metric_name != metric_name {
                continue;
            }

            // Get historical values
            let history = self.metric_history.read();
            if let Some(entries) = history.get(metric_name) {
                let values: Vec<f64> = entries.iter().map(|(_, v)| *v).collect();

                if rule.detect_anomaly(&values, value) {
                    self.trigger_alert(
                        format!("{}_anomaly", rule.name),
                        rule.category,
                        rule.severity,
                        format!("Anomaly detected in {}: {:.2}", metric_name, value),
                    );
                }
            }
        }
    }

    pub fn trigger_alert(
        &self,
        name: impl Into<String>,
        category: AlertCategory,
        severity: AlertSeverity,
        message: impl Into<String>,
    ) -> u64 {
        let name = name.into();
        let mut last_id = self.last_alert_id.write();
        *last_id += 1;
        let alert_id = *last_id;
        drop(last_id);

        let alert = Alert::new(alert_id, name, category, severity, message);

        self.alerts.write().insert(alert_id, alert.clone());

        let mut history = self.alert_history.write();
        if history.len() >= self.max_history {
            history.pop_front();
        }
        history.push_back(alert);

        alert_id
    }

    pub fn acknowledge_alert(&self, alert_id: u64, user: impl Into<String>) -> bool {
        if let Some(alert) = self.alerts.write().get_mut(&alert_id) {
            alert.acknowledge(user);
            true
        } else {
            false
        }
    }

    pub fn resolve_alert(&self, alert_id: u64) -> bool {
        if let Some(alert) = self.alerts.write().get_mut(&alert_id) {
            alert.resolve();
            true
        } else {
            false
        }
    }

    pub fn suppress_alert(&self, alert_id: u64) -> bool {
        if let Some(alert) = self.alerts.write().get_mut(&alert_id) {
            alert.suppress();
            true
        } else {
            false
        }
    }

    pub fn get_alert(&self, alert_id: u64) -> Option<Alert> {
        self.alerts.read().get(&alert_id).cloned()
    }

    pub fn get_active_alerts(&self) -> Vec<Alert> {
        self.alerts
            .read()
            .values()
            .filter(|a| a.is_active())
            .cloned()
            .collect()
    }

    pub fn get_alerts_by_severity(&self, severity: AlertSeverity) -> Vec<Alert> {
        self.alerts
            .read()
            .values()
            .filter(|a| a.severity == severity && a.is_active())
            .cloned()
            .collect()
    }

    pub fn get_alerts_by_category(&self, category: AlertCategory) -> Vec<Alert> {
        self.alerts
            .read()
            .values()
            .filter(|a| a.category == category && a.is_active())
            .cloned()
            .collect()
    }

    pub fn get_alert_history(&self, limit: usize) -> Vec<Alert> {
        self.alert_history
            .read()
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn clear_resolved_alerts(&self) {
        self.alerts.write().retain(|_, alert| alert.state != AlertState::Resolved);
    }

    pub fn get_alert_count(&self) -> usize {
        self.alerts.read().len()
    }

    pub fn get_active_alert_count(&self) -> usize {
        self.alerts.read().values().filter(|a| a.is_active()).count()
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new(10000, 1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_creation() {
        let alert = Alert::new(
            1,
            "test_alert",
            AlertCategory::Performance,
            AlertSeverity::Warning,
            "Test alert message",
        );

        assert_eq!(alert.id, 1);
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert!(alert.is_active());
    }

    #[test]
    fn test_alert_acknowledgment() {
        let mut alert = Alert::new(
            1,
            "test_alert",
            AlertCategory::Performance,
            AlertSeverity::Warning,
            "Test alert message",
        );

        alert.acknowledge("admin");
        assert_eq!(alert.state, AlertState::Acknowledged);
        assert_eq!(alert.acknowledged_by, Some("admin".to_string()));
    }

    #[test]
    fn test_threshold_rule() {
        let rule = ThresholdRule::new(
            "cpu_high",
            "cpu_usage",
            80.0,
            ComparisonOperator::GreaterThan,
            AlertSeverity::Warning,
        );

        assert!(rule.evaluate(85.0));
        assert!(!rule.evaluate(75.0));
    }

    #[test]
    fn test_anomaly_detection_stddev() {
        let rule = AnomalyRule::new(
            "latency_anomaly",
            "query_latency",
            AnomalyDetectionAlgorithm::StandardDeviation,
            3.0,
        );

        let history = vec![10.0, 11.0, 10.5, 10.2, 10.8, 10.3];
        assert!(!rule.detect_anomaly(&history, 10.6));
        assert!(rule.detect_anomaly(&history, 50.0));
    }

    #[test]
    fn test_alert_manager() {
        let manager = AlertManager::new(100, 100);

        let rule = ThresholdRule::new(
            "cpu_high",
            "cpu_usage",
            80.0,
            ComparisonOperator::GreaterThan,
            AlertSeverity::Warning,
        );

        manager.add_threshold_rule(rule);
        manager.record_metric("cpu_usage", 85.0);

        assert!(manager.get_active_alert_count() > 0);
    }
}


