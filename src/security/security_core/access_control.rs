// # Access Control
//
// Security policy engine and defense orchestration for access control and threat management.

use std::collections::{HashSet, HashMap};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::Result;

use super::super::IntegratedSecurityManager;
use super::super::rbac::RoleId;
use super::common::*;

// ============================================================================
// Security Policy Engine
// ============================================================================

pub struct SecurityPolicyEngine {
    policies: Arc<RwLock<HashMap<PolicyId, SecurityPolicy>>>,
    decision_cache: Arc<RwLock<HashMap<String, PolicyDecision>>>,
    stats: Arc<RwLock<PolicyEngineStatistics>>,
    attribute_providers: Arc<RwLock<Vec<Box<dyn AttributeProvider + Send + Sync>>>>,
}

pub type PolicyId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub id: PolicyId,
    pub name: String,
    pub policy_type: PolicyType,
    pub target: String,
    pub rules: Vec<PolicyRule>,
    pub priority: i32,
    pub enabled: bool,
    pub effect: PolicyEffect,
    pub conditions: Vec<PolicyCondition>,
    pub metadata: HashMap<String, String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyType {
    AccessControl,
    DataProtection,
    Audit,
    Encryption,
    Compliance,
    ThreatResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub name: String,
    pub subject: String,
    pub action: String,
    pub resource: String,
    pub effect: PolicyEffect,
    pub conditions: Vec<PolicyCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyEffect {
    Allow,
    Deny,
    Audit,
    Challenge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCondition {
    pub attribute: String,
    pub operator: ConditionOperator,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains,
    NotContains,
    In,
    NotIn,
    Matches,
}

#[derive(Debug, Clone)]
pub struct PolicyDecision {
    pub decision: PolicyEffect,
    pub applicable_policies: Vec<PolicyId>,
    pub reason: String,
    pub obligations: Vec<String>,
    pub timestamp: i64,
}

pub trait AttributeProvider {
    fn get_attribute(&self, name: &str, context: &EvaluationContext) -> Option<String>;
}

#[derive(Debug, Clone)]
pub struct EvaluationContext {
    pub user_id: String,
    pub roles: HashSet<RoleId>,
    pub resource: String,
    pub action: String,
    pub ip_address: Option<String>,
    pub timestamp: i64,
    pub session_attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyEngineStatistics {
    pub total_evaluations: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub allow_decisions: u64,
    pub deny_decisions: u64,
    pub avg_evaluation_time_us: f64,
}

impl SecurityPolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            decision_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(PolicyEngineStatistics::default())),
            attribute_providers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn add_policy(&self, policy: SecurityPolicy) -> Result<()> {
        let mut policies = self.policies.write();
        policies.insert(policy.id.clone(), policy);
        Ok(())
    }

    pub fn evaluate(&self, context: &EvaluationContext) -> Result<PolicyDecision> {
        let start = current_timestamp_micros();

        let cache_key = format!("{}:{}:{}", context.user_id, context.resource, context.action);
        {
            let cache = self.decision_cache.read();
            if let Some(decision) = cache.get(&cache_key) {
                if current_timestamp() - decision.timestamp < 300 {
                    let mut stats = self.stats.write();
                    stats.cache_hits += 1;
                    return Ok(decision.clone());
                }
            }
        }

        let mut stats = self.stats.write();
        stats.cache_misses += 1;
        drop(stats);

        let policies = self.policies.read();
        let mut applicable_policies = Vec::new();
        let mut final_effect = PolicyEffect::Deny;
        let mut reasons = Vec::new();

        let mut sorted_policies: Vec<_> = policies.values().collect();
        sorted_policies.sort_by_key(|p| std::cmp::Reverse(p.priority));

        for policy in sorted_policies {
            if !policy.enabled {
                continue;
            }

            if self.policy_applies(policy, context) {
                applicable_policies.push(policy.id.clone());

                for rule in &policy.rules {
                    if self.rule_matches(rule, context)? {
                        if self.evaluate_conditions(&rule.conditions, context)? {
                            if rule.effect == PolicyEffect::Allow {
                                final_effect = PolicyEffect::Allow;
                                reasons.push(format!("Allowed by policy '{}' rule '{}'", policy.name, rule.name));
                            } else if rule.effect == PolicyEffect::Deny {
                                final_effect = PolicyEffect::Deny;
                                reasons.push(format!("Denied by policy '{}' rule '{}'", policy.name, rule.name));
                                break;
                            }
                        }
                    }
                }
            }
        }

        let decision = PolicyDecision {
            decision: final_effect.clone(),
            applicable_policies,
            reason: reasons.join("; "),
            obligations: Vec::new(),
            timestamp: current_timestamp(),
        };

        {
            let mut cache = self.decision_cache.write();
            cache.insert(cache_key, decision.clone());
        }

        let elapsed = current_timestamp_micros() - start;
        let mut stats = self.stats.write();
        stats.total_evaluations += 1;
        match final_effect {
            PolicyEffect::Allow => stats.allow_decisions += 1,
            PolicyEffect::Deny => stats.deny_decisions += 1,
            _ => {}
        }
        stats.avg_evaluation_time_us =
            (stats.avg_evaluation_time_us * (stats.total_evaluations - 1) as f64 + elapsed as f64)
            / stats.total_evaluations as f64;

        Ok(decision)
    }

    fn policy_applies(&self, policy: &SecurityPolicy, context: &EvaluationContext) -> bool {
        glob_match(&policy.target, &context.resource)
    }

    fn rule_matches(&self, rule: &PolicyRule, context: &EvaluationContext) -> Result<bool> {
        let subject_match = glob_match(&rule.subject, &context.user_id) ||
                           context.roles.iter().any(|r| glob_match(&rule.subject, r));

        let action_match = glob_match(&rule.action, &context.action);

        let resource_match = glob_match(&rule.resource, &context.resource);

        Ok(subject_match && action_match && resource_match)
    }

    fn evaluate_conditions(&self, conditions: &[PolicyCondition], context: &EvaluationContext) -> Result<bool> {
        for condition in conditions {
            let attr_value = match condition.attribute.as_str() {
                "user_id" => Some(context.user_id.clone()),
                "action" => Some(context.action.clone()),
                "resource" => Some(context.resource.clone()),
                "timestamp" => Some(context.timestamp.to_string()),
                _ => context.session_attributes.get(&condition.attribute).cloned(),
            };

            if let Some(value) = attr_value {
                if !self.evaluate_condition(condition, &value)? {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn evaluate_condition(&self, condition: &PolicyCondition, value: &str) -> Result<bool> {
        match condition.operator {
            ConditionOperator::Equals => Ok(value == condition.value),
            ConditionOperator::NotEquals => Ok(value != condition.value),
            ConditionOperator::Contains => Ok(value.contains(&condition.value)),
            ConditionOperator::NotContains => Ok(!value.contains(&condition.value)),
            ConditionOperator::GreaterThan => {
                let v1: f64 = value.parse().unwrap_or(0.0);
                let v2: f64 = condition.value.parse().unwrap_or(0.0);
                Ok(v1 > v2)
            }
            ConditionOperator::LessThan => {
                let v1: f64 = value.parse().unwrap_or(0.0);
                let v2: f64 = condition.value.parse().unwrap_or(0.0);
                Ok(v1 < v2)
            }
            _ => Ok(false),
        }
    }

    pub fn get_statistics(&self) -> PolicyEngineStatistics {
        self.stats.read().clone()
    }
}

// ============================================================================
// Defense Orchestrator
// ============================================================================

pub struct DefenseOrchestrator {
    security_manager: Arc<IntegratedSecurityManager>,
    defense_status: Arc<RwLock<HashMap<DefenseLayer, LayerStatus>>>,
    threat_level: Arc<RwLock<ThreatLevel>>,
    effectiveness: Arc<RwLock<HashMap<DefenseLayer, f64>>>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum DefenseLayer {
    Authentication,
    Authorization,
    Encryption,
    Audit,
    NetworkSecurity,
    DataProtection,
    ThreatDetection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStatus {
    pub active: bool,
    pub health: f64,
    pub last_check: i64,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ThreatLevel {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl DefenseOrchestrator {
    pub fn new(security_manager: Arc<IntegratedSecurityManager>) -> Self {
        let mut defense_status = HashMap::new();

        for layer in &[
            DefenseLayer::Authentication,
            DefenseLayer::Authorization,
            DefenseLayer::Encryption,
            DefenseLayer::Audit,
            DefenseLayer::NetworkSecurity,
            DefenseLayer::DataProtection,
            DefenseLayer::ThreatDetection,
        ] {
            defense_status.insert(layer.clone(), LayerStatus {
                active: true,
                health: 1.0,
                last_check: current_timestamp(),
                issues: Vec::new(),
            });
        }

        Self {
            security_manager,
            defense_status: Arc::new(RwLock::new(defense_status)),
            threat_level: Arc::new(RwLock::new(ThreatLevel::Low)),
            effectiveness: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn validate_coverage(&self) -> DefenseCoverageReport {
        let status = self.defense_status.read();
        let mut gaps = Vec::new();
        let mut total_health = 0.0;
        let mut active_layers = 0;

        for (layer, layer_status) in status.iter() {
            if !layer_status.active {
                gaps.push(format!("Layer {:?} is inactive", layer));
            } else {
                active_layers += 1;
                total_health += layer_status.health;

                if layer_status.health < 0.7 {
                    gaps.push(format!("Layer {:?} health is low: {:.2}", layer, layer_status.health));
                }
            }
        }

        let coverage_score = if active_layers > 0 {
            total_health / active_layers as f64
        } else {
            0.0
        };

        DefenseCoverageReport {
            coverage_score,
            active_layers,
            total_layers: status.len(),
            gaps,
            timestamp: current_timestamp(),
        }
    }

    pub fn adapt_defenses(&self, new_threat_level: ThreatLevel) -> Result<()> {
        let mut threat_level = self.threat_level.write();
        *threat_level = new_threat_level;

        match new_threat_level {
            ThreatLevel::Critical => {
                self.enable_all_defenses()?;
                self.tighten_authentication()?;
            }
            ThreatLevel::High => {
                self.enable_all_defenses()?;
            }
            ThreatLevel::Medium => {
            }
            ThreatLevel::Low => {
            }
        }

        Ok(())
    }

    fn enable_all_defenses(&self) -> Result<()> {
        let mut status = self.defense_status.write();
        for layer_status in status.values_mut() {
            layer_status.active = true;
        }
        Ok(())
    }

    fn tighten_authentication(&self) -> Result<()> {
        Ok(())
    }

    pub fn get_effectiveness_score(&self) -> f64 {
        let effectiveness = self.effectiveness.read();
        if effectiveness.is_empty() {
            return 1.0;
        }

        let sum: f64 = effectiveness.values().sum();
        sum / effectiveness.len() as f64
    }

    pub fn get_threat_level(&self) -> ThreatLevel {
        *self.threat_level.read()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenseCoverageReport {
    pub coverage_score: f64,
    pub active_layers: usize,
    pub total_layers: usize,
    pub gaps: Vec<String>,
    pub timestamp: i64,
}
