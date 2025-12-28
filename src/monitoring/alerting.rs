// Advanced Alerting System
// Alert rule definitions, threshold-based alerts, anomaly-based alerts,
// alert routing and escalation, alert suppression and deduplication

use crate::error::{DbError, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use super::alerts::{Alert, AlertCategory, AlertSeverity, AlertState};

// SAFETY: Maximum alert rules and routing configs to prevent OOM
const MAX_ALERT_RULES: usize = 10_000;
const MAX_ROUTING_CONFIGS: usize = 1_000;
const MAX_SUPPRESSION_RULES: usize = 1_000;
const MAX_ALERT_HISTORY_PER_RULE: usize = 1_000;

// Alert evaluation interval
const DEFAULT_EVALUATION_INTERVAL: Duration = Duration::from_secs(10);

// Alert routing destination
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertDestination {
    Email(String),
    Slack(String),        // Channel or webhook URL
    PagerDuty(String),    // Integration key
    Webhook(String),      // HTTP endpoint
    Console,              // Log to console
    Database,             // Store in database
}

// Alert routing rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRoutingRule {
    pub id: String,
    pub name: String,
    pub severity_filter: Option<AlertSeverity>,
    pub category_filter: Option<AlertCategory>,
    pub tag_filter: HashMap<String, String>,
    pub destinations: Vec<AlertDestination>,
    pub enabled: bool,
}

impl AlertRoutingRule {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            severity_filter: None,
            category_filter: None,
            tag_filter: HashMap::new(),
            destinations: Vec::new(),
            enabled: true,
        }
    }

    pub fn with_severity(mut self, severity: AlertSeverity) -> Self {
        self.severity_filter = Some(severity);
        self
    }

    pub fn with_category(mut self, category: AlertCategory) -> Self {
        self.category_filter = Some(category);
        self
    }

    pub fn with_destination(mut self, destination: AlertDestination) -> Self {
        self.destinations.push(destination);
        self
    }

    pub fn matches(&self, alert: &Alert) -> bool {
        if !self.enabled {
            return false;
        }

        if let Some(severity) = self.severity_filter {
            if alert.severity < severity {
                return false;
            }
        }

        if let Some(category) = self.category_filter {
            if alert.category != category {
                return false;
            }
        }

        // Check tag filters
        for (key, value) in &self.tag_filter {
            if alert.details.get(key) != Some(value) {
                return false;
            }
        }

        true
    }
}

// Alert escalation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPolicy {
    pub id: String,
    pub name: String,
    pub levels: Vec<EscalationLevel>,
    pub enabled: bool,
}

impl EscalationPolicy {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            levels: Vec::new(),
            enabled: true,
        }
    }

    pub fn add_level(&mut self, level: EscalationLevel) {
        self.levels.push(level);
    }

    pub fn get_level(&self, level_index: usize) -> Option<&EscalationLevel> {
        self.levels.get(level_index)
    }
}

// Escalation level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationLevel {
    pub level: u8,
    pub delay: Duration,
    pub destinations: Vec<AlertDestination>,
}

impl EscalationLevel {
    pub fn new(level: u8, delay: Duration) -> Self {
        Self {
            level,
            delay,
            destinations: Vec::new(),
        }
    }

    pub fn with_destination(mut self, destination: AlertDestination) -> Self {
        self.destinations.push(destination);
        self
    }
}

// Alert suppression rule (prevent alert storms)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppressionRule {
    pub id: String,
    pub name: String,
    pub metric_pattern: String,  // Regex or glob pattern
    pub time_window: Duration,
    pub max_alerts: usize,
    pub enabled: bool,
}

impl SuppressionRule {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        metric_pattern: impl Into<String>,
        time_window: Duration,
        max_alerts: usize,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            metric_pattern: metric_pattern.into(),
            time_window,
            max_alerts,
            enabled: true,
        }
    }

    pub fn should_suppress(&self, metric_name: &str, recent_alert_count: usize) -> bool {
        if !self.enabled {
            return false;
        }

        // Simple pattern matching (can be enhanced with regex)
        let matches = metric_name.contains(&self.metric_pattern);

        matches && recent_alert_count >= self.max_alerts
    }
}

// Alert deduplication key
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct DeduplicationKey {
    rule_id: String,
    fingerprint: String,
}

// Alert instance tracking for deduplication
#[derive(Debug, Clone)]
struct AlertInstance {
    alert_id: u64,
    first_seen: SystemTime,
    last_seen: SystemTime,
    occurrence_count: usize,
}

// Advanced alerting engine
pub struct AlertingEngine {
    routing_rules: Arc<RwLock<HashMap<String, AlertRoutingRule>>>,
    escalation_policies: Arc<RwLock<HashMap<String, EscalationPolicy>>>,
    suppression_rules: Arc<RwLock<HashMap<String, SuppressionRule>>>,

    // Deduplication tracking
    active_alerts: Arc<RwLock<HashMap<DeduplicationKey, AlertInstance>>>,

    // Alert history for suppression tracking
    alert_history: Arc<RwLock<HashMap<String, VecDeque<SystemTime>>>>,

    // Statistics
    stats: Arc<RwLock<AlertingStats>>,

    evaluation_interval: Duration,
}

impl AlertingEngine {
    pub fn new() -> Self {
        Self {
            routing_rules: Arc::new(RwLock::new(HashMap::new())),
            escalation_policies: Arc::new(RwLock::new(HashMap::new())),
            suppression_rules: Arc::new(RwLock::new(HashMap::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(AlertingStats::default())),
            evaluation_interval: DEFAULT_EVALUATION_INTERVAL,
        }
    }

    pub fn with_evaluation_interval(mut self, interval: Duration) -> Self {
        self.evaluation_interval = interval;
        self
    }

    // Add routing rule
    pub fn add_routing_rule(&self, rule: AlertRoutingRule) -> Result<()> {
        let mut rules = self.routing_rules.write();

        if rules.len() >= MAX_ROUTING_CONFIGS {
            return Err(DbError::LimitExceeded(
                "Maximum number of routing rules reached".to_string(),
            ));
        }

        rules.insert(rule.id.clone(), rule);
        Ok(())
    }

    // Remove routing rule
    pub fn remove_routing_rule(&self, rule_id: &str) -> Result<()> {
        self.routing_rules.write().remove(rule_id);
        Ok(())
    }

    // Get routing rule
    pub fn get_routing_rule(&self, rule_id: &str) -> Option<AlertRoutingRule> {
        self.routing_rules.read().get(rule_id).cloned()
    }

    // List all routing rules
    pub fn list_routing_rules(&self) -> Vec<AlertRoutingRule> {
        self.routing_rules.read().values().cloned().collect()
    }

    // Add escalation policy
    pub fn add_escalation_policy(&self, policy: EscalationPolicy) -> Result<()> {
        let mut policies = self.escalation_policies.write();

        if policies.len() >= MAX_ROUTING_CONFIGS {
            return Err(DbError::LimitExceeded(
                "Maximum number of escalation policies reached".to_string(),
            ));
        }

        policies.insert(policy.id.clone(), policy);
        Ok(())
    }

    // Remove escalation policy
    pub fn remove_escalation_policy(&self, policy_id: &str) -> Result<()> {
        self.escalation_policies.write().remove(policy_id);
        Ok(())
    }

    // Add suppression rule
    pub fn add_suppression_rule(&self, rule: SuppressionRule) -> Result<()> {
        let mut rules = self.suppression_rules.write();

        if rules.len() >= MAX_SUPPRESSION_RULES {
            return Err(DbError::LimitExceeded(
                "Maximum number of suppression rules reached".to_string(),
            ));
        }

        rules.insert(rule.id.clone(), rule);
        Ok(())
    }

    // Process an alert through the alerting pipeline
    pub fn process_alert(&self, mut alert: Alert) -> Result<ProcessedAlert> {
        let mut stats = self.stats.write();
        stats.total_alerts_processed += 1;

        // Step 1: Check for deduplication
        let dedup_key = self.generate_deduplication_key(&alert);

        if let Some(instance) = self.active_alerts.write().get_mut(&dedup_key) {
            instance.last_seen = SystemTime::now();
            instance.occurrence_count += 1;
            stats.alerts_deduplicated += 1;

            return Ok(ProcessedAlert {
                alert,
                deduplicated: true,
                suppressed: false,
                routed_to: Vec::new(),
                escalation_level: 0,
            });
        }

        // Step 2: Check for suppression
        if self.should_suppress_alert(&alert)? {
            alert.state = AlertState::Suppressed;
            stats.alerts_suppressed += 1;

            return Ok(ProcessedAlert {
                alert,
                deduplicated: false,
                suppressed: true,
                routed_to: Vec::new(),
                escalation_level: 0,
            });
        }

        // Step 3: Route alert to destinations
        let destinations = self.route_alert(&alert);
        stats.alerts_routed += destinations.len();

        // Step 4: Track alert instance
        self.active_alerts.write().insert(
            dedup_key,
            AlertInstance {
                alert_id: alert.id,
                first_seen: alert.triggered_at,
                last_seen: SystemTime::now(),
                occurrence_count: 1,
            },
        );

        // Update history
        self.update_alert_history(&alert);

        Ok(ProcessedAlert {
            alert,
            deduplicated: false,
            suppressed: false,
            routed_to: destinations,
            escalation_level: 0,
        })
    }

    // Generate deduplication key for an alert
    fn generate_deduplication_key(&self, alert: &Alert) -> DeduplicationKey {
        // Create fingerprint from alert attributes
        let fingerprint = format!(
            "{}:{}:{}",
            alert.name,
            alert.category,
            alert.details.get("metric_name").unwrap_or(&"unknown".to_string())
        );

        DeduplicationKey {
            rule_id: alert.name.clone(),
            fingerprint,
        }
    }

    // Check if alert should be suppressed
    fn should_suppress_alert(&self, alert: &Alert) -> Result<bool> {
        let rules = self.suppression_rules.read();

        for rule in rules.values() {
            if !rule.enabled {
                continue;
            }

            let metric_name = alert.details.get("metric_name")
                .map(|s| s.as_str())
                .unwrap_or("");

            // Count recent alerts
            let recent_count = self.count_recent_alerts(&alert.name, rule.time_window);

            if rule.should_suppress(metric_name, recent_count) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    // Count recent alerts for a rule
    fn count_recent_alerts(&self, rule_name: &str, window: Duration) -> usize {
        let history = self.alert_history.read();

        if let Some(timestamps) = history.get(rule_name) {
            let cutoff = SystemTime::now() - window;
            timestamps.iter()
                .filter(|&&ts| ts >= cutoff)
                .count()
        } else {
            0
        }
    }

    // Route alert to appropriate destinations
    fn route_alert(&self, alert: &Alert) -> Vec<AlertDestination> {
        let rules = self.routing_rules.read();
        let mut destinations = Vec::new();

        for rule in rules.values() {
            if rule.matches(alert) {
                destinations.extend(rule.destinations.clone());
            }
        }

        destinations
    }

    // Update alert history for suppression tracking
    fn update_alert_history(&self, alert: &Alert) {
        let mut history = self.alert_history.write();

        let timestamps = history.entry(alert.name.clone())
            .or_insert_with(VecDeque::new);

        timestamps.push_back(SystemTime::now());

        // Keep only recent history
        while timestamps.len() > MAX_ALERT_HISTORY_PER_RULE {
            timestamps.pop_front();
        }
    }

    // Acknowledge an alert (stops escalation)
    pub fn acknowledge_alert(&self, alert_id: u64) -> Result<()> {
        // Remove from active alerts to stop deduplication
        let mut active = self.active_alerts.write();
        active.retain(|_, instance| instance.alert_id != alert_id);
        Ok(())
    }

    // Resolve an alert
    pub fn resolve_alert(&self, alert_id: u64) -> Result<()> {
        let mut active = self.active_alerts.write();
        active.retain(|_, instance| instance.alert_id != alert_id);
        Ok(())
    }

    // Clean up old alert instances
    pub fn cleanup_old_alerts(&self, max_age: Duration) {
        let cutoff = SystemTime::now() - max_age;
        let mut active = self.active_alerts.write();

        active.retain(|_, instance| instance.last_seen >= cutoff);
    }

    // Get alerting statistics
    pub fn get_stats(&self) -> AlertingStats {
        self.stats.read().clone()
    }

    // Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.write() = AlertingStats::default();
    }

    // Get active alert count
    pub fn get_active_alert_count(&self) -> usize {
        self.active_alerts.read().len()
    }
}

impl Default for AlertingEngine {
    fn default() -> Self {
        Self::new()
    }
}

// Processed alert result
#[derive(Debug, Clone)]
pub struct ProcessedAlert {
    pub alert: Alert,
    pub deduplicated: bool,
    pub suppressed: bool,
    pub routed_to: Vec<AlertDestination>,
    pub escalation_level: u8,
}

// Alerting statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlertingStats {
    pub total_alerts_processed: u64,
    pub alerts_routed: usize,
    pub alerts_suppressed: u64,
    pub alerts_deduplicated: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_rule() {
        let rule = AlertRoutingRule::new("rule1", "Critical Email")
            .with_severity(AlertSeverity::Critical)
            .with_category(AlertCategory::Performance)
            .with_destination(AlertDestination::Email("admin@example.com".to_string()));

        assert_eq!(rule.id, "rule1");
        assert_eq!(rule.severity_filter, Some(AlertSeverity::Critical));
        assert_eq!(rule.destinations.len(), 1);
    }

    #[test]
    fn test_escalation_policy() {
        let mut policy = EscalationPolicy::new("policy1", "Standard Escalation");

        let level1 = EscalationLevel::new(1, Duration::from_secs(300))
            .with_destination(AlertDestination::Email("oncall@example.com".to_string()));

        policy.add_level(level1);

        assert_eq!(policy.levels.len(), 1);
        assert_eq!(policy.get_level(0).unwrap().level, 1);
    }

    #[test]
    fn test_suppression_rule() {
        let rule = SuppressionRule::new(
            "supp1",
            "CPU Spike Suppression",
            "cpu_",
            Duration::from_secs(60),
            5,
        );

        assert!(rule.should_suppress("cpu_usage_high", 6));
        assert!(!rule.should_suppress("cpu_usage_high", 3));
        assert!(!rule.should_suppress("memory_usage_high", 6));
    }

    #[test]
    fn test_alerting_engine() {
        let engine = AlertingEngine::new();

        // Add routing rule
        let rule = AlertRoutingRule::new("rule1", "Test Rule")
            .with_destination(AlertDestination::Console);

        assert!(engine.add_routing_rule(rule).is_ok());

        // List rules
        let rules = engine.list_routing_rules();
        assert_eq!(rules.len(), 1);

        // Get stats
        let stats = engine.get_stats();
        assert_eq!(stats.total_alerts_processed, 0);
    }
}
