// Monitoring Module
//
// Part of the comprehensive monitoring system for RustyDB

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::DbError;

// SECTION 4: ALERTING ENGINE (600+ lines)
// ============================================================================

// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    ErrorLevel,
    Critical,
}

// Alert state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertState {
    Pending,
    Firing,
    Resolved,
    Silenced,
    Inhibited,
}

// Alert instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub rule_name: String,
    pub severity: AlertSeverity,
    pub state: AlertState,
    pub message: String,
    pub labels: BTreeMap<String, String>,
    pub annotations: BTreeMap<String, String>,
    pub starts_at: SystemTime,
    pub ends_at: Option<SystemTime>,
    pub value: f64,
    pub fingerprint: String,
}

impl Alert {
    pub fn new(rule_name: String, severity: AlertSeverity, message: String, value: f64) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let fingerprint = format!(
            "{}{}",
            rule_name,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        Self {
            id,
            rule_name,
            severity,
            state: AlertState::Pending,
            message,
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
            starts_at: SystemTime::now(),
            ends_at: None,
            value,
            fingerprint,
        }
    }

    pub fn with_label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }

    pub fn with_annotation(mut self, key: String, value: String) -> Self {
        self.annotations.insert(key, value);
        self
    }
}

// Comparison operator for threshold rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

impl ComparisonOperator {
    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            ComparisonOperator::GreaterThan => value > threshold,
            ComparisonOperator::GreaterThanOrEqual => value >= threshold,
            ComparisonOperator::LessThan => value < threshold,
            ComparisonOperator::LessThanOrEqual => value <= threshold,
            ComparisonOperator::Equal => (value - threshold).abs() < f64::EPSILON,
            ComparisonOperator::NotEqual => (value - threshold).abs() >= f64::EPSILON,
        }
    }
}

// Threshold-based alert rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdAlertRule {
    pub name: String,
    pub metric_name: String,
    pub threshold: f64,
    pub operator: ComparisonOperator,
    pub severity: AlertSeverity,
    pub duration: Duration,
    pub labels: BTreeMap<String, String>,
    pub annotations: BTreeMap<String, String>,
    pub enabled: bool,

    // State tracking
    #[serde(skip)]
    first_triggered: Arc<RwLock<Option<SystemTime>>>,
}

impl ThresholdAlertRule {
    pub fn new(
        name: String,
        metricname: String,
        threshold: f64,
        operator: ComparisonOperator,
        severity: AlertSeverity,
    ) -> Self {
        Self {
            name,
            metric_name: metricname,
            threshold,
            operator,
            severity,
            duration: Duration::from_secs(60),
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
            enabled: true,
            first_triggered: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn evaluate(&self, value: f64) -> Option<Alert> {
        if !self.enabled {
            return None;
        }

        let triggered = self.operator.evaluate(value, self.threshold);

        if triggered {
            let mut first = self.first_triggered.write();
            let trigger_time = first.get_or_insert_with(SystemTime::now);

            // Check if duration threshold met
            if let Ok(elapsed) = SystemTime::now().duration_since(*trigger_time) {
                if elapsed >= self.duration {
                    let message = format!(
                        "{} {} {} (current: {})",
                        self.metric_name,
                        match self.operator {
                            ComparisonOperator::GreaterThan => ">",
                            ComparisonOperator::GreaterThanOrEqual => ">=",
                            ComparisonOperator::LessThan => "<",
                            ComparisonOperator::LessThanOrEqual => "<=",
                            ComparisonOperator::Equal => "==",
                            ComparisonOperator::NotEqual => "!=",
                        },
                        self.threshold,
                        value
                    );

                    let mut alert = Alert::new(self.name.clone(), self.severity, message, value);

                    for (k, v) in &self.labels {
                        alert = alert.with_label(k.clone(), v.clone());
                    }

                    for (k, v) in &self.annotations {
                        alert = alert.with_annotation(k.clone(), v.clone());
                    }

                    alert.state = AlertState::Firing;
                    return Some(alert);
                }
            }
        } else {
            *self.first_triggered.write() = None;
        }

        None
    }
}

// Multi-condition alert rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiConditionAlertRule {
    pub name: String,
    pub conditions: Vec<AlertCondition>,
    pub combine_operator: LogicalOperator,
    pub severity: AlertSeverity,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCondition {
    pub metric_name: String,
    pub threshold: f64,
    pub operator: ComparisonOperator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicalOperator {
    And,
    Or,
}

impl MultiConditionAlertRule {
    pub fn evaluate(&self, metric_values: &HashMap<String, f64>) -> Option<Alert> {
        if !self.enabled {
            return None;
        }

        let results: Vec<bool> = self
            .conditions
            .iter()
            .map(|cond| {
                metric_values
                    .get(&cond.metric_name)
                    .map(|&value| cond.operator.evaluate(value, cond.threshold))
                    .unwrap_or(false)
            })
            .collect();

        let triggered = match self.combine_operator {
            LogicalOperator::And => results.iter().all(|&r| r),
            LogicalOperator::Or => results.iter().any(|&r| r),
        };

        if triggered {
            let message = format!("Multi-condition alert: {}", self.name);
            let mut alert = Alert::new(
                self.name.clone(),
                self.severity,
                message,
                0.0, // No single value for multi-condition
            );
            alert.state = AlertState::Firing;
            Some(alert)
        } else {
            None
        }
    }
}

// Alert routing rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRoute {
    pub name: String,
    pub matchers: Vec<AlertMatcher>,
    pub channels: Vec<String>,
    pub group_by: Vec<String>,
    pub group_wait: Duration,
    pub group_interval: Duration,
    pub repeat_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertMatcher {
    pub label: String,
    pub value: String,
    pub is_regex: bool,
}

impl AlertRoute {
    pub fn matches(&self, alert: &Alert) -> bool {
        if self.matchers.is_empty() {
            return true;
        }

        self.matchers.iter().all(|matcher| {
            alert
                .labels
                .get(&matcher.label)
                .map(|v| {
                    if matcher.is_regex {
                        // Simple string contains for now
                        v.contains(&matcher.value)
                    } else {
                        v == &matcher.value
                    }
                })
                .unwrap_or(false)
        })
    }
}

// Alert silencer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSilence {
    pub id: String,
    pub matchers: Vec<AlertMatcher>,
    pub starts_at: SystemTime,
    pub ends_at: SystemTime,
    pub created_by: String,
    pub comment: String,
}

impl AlertSilence {
    pub fn is_active(&self) -> bool {
        let now = SystemTime::now();
        now >= self.starts_at && now < self.ends_at
    }

    pub fn matches(&self, alert: &Alert) -> bool {
        if !self.is_active() {
            return false;
        }

        self.matchers.iter().all(|matcher| {
            alert
                .labels
                .get(&matcher.label)
                .map(|v| v == &matcher.value)
                .unwrap_or(false)
        })
    }
}

// Alert inhibitor - suppress alerts based on other active alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertInhibitionRule {
    pub source_matchers: Vec<AlertMatcher>,
    pub target_matchers: Vec<AlertMatcher>,
    pub equal_labels: Vec<String>,
}

// Notification channel trait
pub trait NotificationChannel: Send + Sync {
    fn name(&self) -> &str;
    fn send(&self, alert: &Alert) -> Result<(), DbError>;
}

// Webhook notification channel
pub struct WebhookChannel {
    name: String,
    url: String,
    headers: HashMap<String, String>,
}

impl WebhookChannel {
    pub fn new(name: String, url: String) -> Self {
        Self {
            name,
            url,
            headers: HashMap::new(),
        }
    }

    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
}

impl NotificationChannel for WebhookChannel {
    fn name(&self) -> &str {
        &self.name
    }

    fn send(&self, alert: &Alert) -> Result<(), DbError> {
        // In real implementation, would use HTTP client
        println!("Sending alert to webhook {}: {:?}", self.url, alert);
        Ok(())
    }
}

// Email notification channel
pub struct EmailChannel {
    name: String,
    #[allow(dead_code)]
    smtp_server: String,
    from: String,
    to: Vec<String>,
}

impl EmailChannel {
    pub fn new(name: String, smtp_server: String, from: String, to: Vec<String>) -> Self {
        Self {
            name,
            smtp_server,
            from,
            to,
        }
    }
}

impl NotificationChannel for EmailChannel {
    fn name(&self) -> &str {
        &self.name
    }

    fn send(&self, alert: &Alert) -> Result<(), DbError> {
        println!(
            "Sending email alert from {} to {:?}: {}",
            self.from, self.to, alert.message
        );
        Ok(())
    }
}

// Slack notification channel
pub struct SlackChannel {
    name: String,
    #[allow(dead_code)]
    webhook_url: String,
    channel: String,
}

impl SlackChannel {
    pub fn new(name: String, webhook_url: String, channel: String) -> Self {
        Self {
            name,
            webhook_url,
            channel,
        }
    }
}

impl NotificationChannel for SlackChannel {
    fn name(&self) -> &str {
        &self.name
    }

    fn send(&self, alert: &Alert) -> Result<(), DbError> {
        println!("Sending Slack alert to {}: {}", self.channel, alert.message);
        Ok(())
    }
}

// Alert manager
pub struct AlertManager {
    threshold_rules: Arc<RwLock<Vec<ThresholdAlertRule>>>,
    multi_condition_rules: Arc<RwLock<Vec<MultiConditionAlertRule>>>,
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    alert_history: Arc<RwLock<VecDeque<Alert>>>,
    routes: Arc<RwLock<Vec<AlertRoute>>>,
    silences: Arc<RwLock<Vec<AlertSilence>>>,
    #[allow(dead_code)]
    inhibition_rules: Arc<RwLock<Vec<AlertInhibitionRule>>>,
    channels: Arc<RwLock<HashMap<String, Arc<dyn NotificationChannel>>>>,
    max_history: usize,
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            threshold_rules: Arc::new(RwLock::new(Vec::new())),
            multi_condition_rules: Arc::new(RwLock::new(Vec::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(VecDeque::new())),
            routes: Arc::new(RwLock::new(Vec::new())),
            silences: Arc::new(RwLock::new(Vec::new())),
            inhibition_rules: Arc::new(RwLock::new(Vec::new())),
            channels: Arc::new(RwLock::new(HashMap::new())),
            max_history: 10000,
        }
    }

    pub fn add_threshold_rule(&self, rule: ThresholdAlertRule) {
        self.threshold_rules.write().push(rule);
    }

    pub fn add_multi_condition_rule(&self, rule: MultiConditionAlertRule) {
        self.multi_condition_rules.write().push(rule);
    }

    pub fn add_route(&self, route: AlertRoute) {
        self.routes.write().push(route);
    }

    pub fn add_silence(&self, silence: AlertSilence) {
        self.silences.write().push(silence);
    }

    pub fn add_channel(&self, channel: Arc<dyn NotificationChannel>) {
        self.channels
            .write()
            .insert(channel.name().to_string(), channel);
    }

    pub fn evaluate_rules(&self, metrics: &HashMap<String, f64>) {
        // Evaluate threshold rules
        let threshold_rules = self.threshold_rules.read();
        for rule in threshold_rules.iter() {
            if let Some(&value) = metrics.get(&rule.metric_name) {
                if let Some(mut alert) = rule.evaluate(value) {
                    // Check if silenced
                    let silences = self.silences.read();
                    if silences.iter().any(|s| s.matches(&alert)) {
                        alert.state = AlertState::Silenced;
                    }

                    self.fire_alert(alert);
                }
            }
        }

        // Evaluate multi-condition rules
        let multi_rules = self.multi_condition_rules.read();
        for rule in multi_rules.iter() {
            if let Some(alert) = rule.evaluate(metrics) {
                self.fire_alert(alert);
            }
        }
    }

    pub fn fire_alert(&self, alert: Alert) {
        let fingerprint = alert.fingerprint.clone();

        // Add to active alerts
        self.active_alerts
            .write()
            .insert(fingerprint, alert.clone());

        // Add to history
        let mut history = self.alert_history.write();
        history.push_back(alert.clone());
        if history.len() > self.max_history {
            history.pop_front();
        }

        // Route and send notifications
        self.route_alert(&alert);
    }

    fn route_alert(&self, alert: &Alert) {
        let routes = self.routes.read();
        let channels = self.channels.read();

        for route in routes.iter() {
            if route.matches(alert) {
                for channel_name in &route.channels {
                    if let Some(channel) = channels.get(channel_name) {
                        if let Err(e) = channel.send(alert) {
                            eprintln!("Failed to send alert via {}: {:?}", channel_name, e);
                        }
                    }
                }
            }
        }
    }

    pub fn resolve_alert(&self, fingerprint: &str) {
        if let Some(alert) = self.active_alerts.write().remove(fingerprint) {
            let mut resolved = alert.clone();
            resolved.state = AlertState::Resolved;
            resolved.ends_at = Some(SystemTime::now());

            self.alert_history.write().push_back(resolved);
        }
    }

    pub fn get_active_alerts(&self) -> Vec<Alert> {
        self.active_alerts.read().values().cloned().collect()
    }

    pub fn get_alert_history(&self, limit: usize) -> Vec<Alert> {
        let history = self.alert_history.read();
        history.iter().rev().take(limit).cloned().collect()
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
