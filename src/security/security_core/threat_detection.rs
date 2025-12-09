// # Threat Detection
//
// Event correlation and threat intelligence components for detecting attack patterns.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::Result;
use crate::error::DbError;

use super::common::*;

// ============================================================================
// Security Event Correlator
// ============================================================================

pub struct SecurityEventCorrelator {
    event_windows: Arc<RwLock<HashMap<String, VecDeque<CorrelatedEvent>>>>,
    attack_patterns: Arc<RwLock<Vec<AttackPattern>>>,
    incidents: Arc<RwLock<Vec<SecurityIncident>>>,
    correlation_rules: Arc<RwLock<Vec<CorrelationRule>>>,
    stats: Arc<RwLock<CorrelatorStatistics>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelatedEvent {
    pub event_type: String,
    pub user_id: String,
    pub source_ip: Option<String>,
    pub resource: Option<String>,
    pub timestamp: i64,
    pub severity: EventSeverity,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventSeverity {
    Info = 1,
    Low = 2,
    Medium = 3,
    High = 4,
    Critical = 5,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackPattern {
    pub id: String,
    pub name: String,
    pub technique_id: String,
    pub description: String,
    pub indicators: Vec<EventIndicator>,
    pub severity: EventSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventIndicator {
    pub event_type: String,
    pub conditions: HashMap<String, String>,
    pub time_window_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIncident {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: EventSeverity,
    pub attack_pattern: Option<String>,
    pub affected_users: HashSet<String>,
    pub affected_resources: HashSet<String>,
    pub events: Vec<CorrelatedEvent>,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: IncidentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IncidentStatus {
    New,
    Investigating,
    Confirmed,
    Mitigated,
    Resolved,
    FalsePositive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationRule {
    pub id: String,
    pub name: String,
    pub event_types: Vec<String>,
    pub time_window_seconds: i64,
    pub threshold: usize,
    pub severity: EventSeverity,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrelatorStatistics {
    pub total_events: u64,
    pub incidents_created: u64,
    pub false_positives: u64,
    pub true_positives: u64,
    pub avg_correlation_time_ms: f64,
}

impl SecurityEventCorrelator {
    pub fn new() -> Self {
        let mut attack_patterns = Vec::new();

        attack_patterns.push(AttackPattern {
            id: "T1110".to_string(),
            name: "Brute Force".to_string(),
            technique_id: "T1110".to_string(),
            description: "Multiple failed login attempts".to_string(),
            indicators: vec![EventIndicator {
                event_type: "failed_login".to_string(),
                conditions: HashMap::new(),
                time_window_seconds: 300,
            }],
            severity: EventSeverity::High,
        });

        attack_patterns.push(AttackPattern {
            id: "T1078".to_string(),
            name: "Valid Accounts".to_string(),
            technique_id: "T1078".to_string(),
            description: "Unusual access patterns from valid account".to_string(),
            indicators: vec![],
            severity: EventSeverity::Medium,
        });

        attack_patterns.push(AttackPattern {
            id: "T1068".to_string(),
            name: "Privilege Escalation".to_string(),
            technique_id: "T1068".to_string(),
            description: "Attempt to elevate privileges".to_string(),
            indicators: vec![],
            severity: EventSeverity::Critical,
        });

        Self {
            event_windows: Arc::new(RwLock::new(HashMap::new())),
            attack_patterns: Arc::new(RwLock::new(attack_patterns)),
            incidents: Arc::new(RwLock::new(Vec::new())),
            correlation_rules: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(CorrelatorStatistics::default())),
        }
    }

    pub fn add_event(&self, event: CorrelatedEvent) -> Result<()> {
        let user_id = event.user_id.clone();
        let mut windows = self.event_windows.write();

        let user_events = windows.entry(user_id).or_insert_with(VecDeque::new);
        user_events.push_back(event.clone());

        let cutoff = current_timestamp() - 3600;
        while let Some(front) = user_events.front() {
            if front.timestamp < cutoff {
                user_events.pop_front();
            } else {
                break;
            }
        }

        let mut stats = self.stats.write();
        stats.total_events += 1;

        drop(stats);
        drop(windows);
        self.analyze_patterns(&event)?;

        Ok(())
    }

    fn analyze_patterns(&self, trigger_event: &CorrelatedEvent) -> Result<()> {
        let windows = self.event_windows.read();
        let patterns = self.attack_patterns.read();

        if let Some(user_events) = windows.get(&trigger_event.user_id) {
            let failed_logins = user_events.iter()
                .filter(|e| e.event_type == "failed_login")
                .count();

            if failed_logins >= 5 {
                self.create_incident(
                    "Brute Force Attack Detected".to_string(),
                    format!("User {} has {} failed login attempts", trigger_event.user_id, failed_logins),
                    EventSeverity::High,
                    Some("T1110".to_string()),
                    vec![trigger_event.user_id.clone()],
                )?;
            }

            let access_count = user_events.iter()
                .filter(|e| e.event_type == "data_access")
                .count();

            if access_count > 100 {
                self.create_incident(
                    "Unusual Data Access Pattern".to_string(),
                    format!("User {} accessed {} resources in short time", trigger_event.user_id, access_count),
                    EventSeverity::Medium,
                    Some("T1078".to_string()),
                    vec![trigger_event.user_id.clone()],
                )?;
            }
        }

        Ok(())
    }

    fn create_incident(
        &self,
        title: String,
        description: String,
        severity: EventSeverity,
        attack_pattern: Option<String>,
        affected_users: Vec<String>,
    ) -> Result<()> {
        let incident = SecurityIncident {
            id: format!("INC_{}", generate_id()),
            title,
            description,
            severity,
            attack_pattern,
            affected_users: affected_users.into_iter().collect(),
            affected_resources: HashSet::new(),
            events: Vec::new(),
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            status: IncidentStatus::New,
        };

        let mut incidents = self.incidents.write();
        incidents.push(incident);

        let mut stats = self.stats.write();
        stats.incidents_created += 1;

        Ok(())
    }

    pub fn get_active_incidents(&self) -> Vec<SecurityIncident> {
        let incidents = self.incidents.read();
        incidents.iter()
            .filter(|i| i.status != IncidentStatus::Resolved && i.status != IncidentStatus::FalsePositive)
            .cloned()
            .collect()
    }

    pub fn get_statistics(&self) -> CorrelatorStatistics {
        self.stats.read().clone()
    }
}

// ============================================================================
// Threat Intelligence
// ============================================================================

pub struct ThreatIntelligence {
    iocs: Arc<RwLock<HashMap<String, IndicatorOfCompromise>>>,
    threat_actors: Arc<RwLock<HashMap<String, ThreatActor>>>,
    vulnerabilities: Arc<RwLock<HashMap<String, Vulnerability>>>,
    ip_reputation: Arc<RwLock<HashMap<String, ReputationScore>>>,
    stats: Arc<RwLock<ThreatIntelligenceStatistics>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorOfCompromise {
    pub id: String,
    pub ioc_type: IocType,
    pub value: String,
    pub severity: EventSeverity,
    pub description: String,
    pub source: String,
    pub first_seen: i64,
    pub last_seen: i64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IocType {
    IpAddress,
    Domain,
    FileHash,
    Email,
    UserAgent,
    SqlPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatActor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub sophistication: ThreatSophistication,
    pub motivations: Vec<String>,
    pub techniques: Vec<String>,
    pub associated_iocs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThreatSophistication {
    ScriptKiddie,
    Intermediate,
    Advanced,
    Expert,
    NationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub cve_id: String,
    pub description: String,
    pub cvss_score: f64,
    pub severity: EventSeverity,
    pub affected_components: Vec<String>,
    pub published_date: i64,
    pub patched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationScore {
    pub ip_address: String,
    pub score: f64,
    pub category: ReputationCategory,
    pub last_updated: i64,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReputationCategory {
    Trusted,
    Unknown,
    Suspicious,
    Malicious,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThreatIntelligenceStatistics {
    pub total_iocs: usize,
    pub active_threats: usize,
    pub vulnerabilities_tracked: usize,
    pub ip_lookups: u64,
    pub threat_matches: u64,
}

impl ThreatIntelligence {
    pub fn new() -> Self {
        Self {
            iocs: Arc::new(RwLock::new(HashMap::new())),
            threat_actors: Arc::new(RwLock::new(HashMap::new())),
            vulnerabilities: Arc::new(RwLock::new(HashMap::new())),
            ip_reputation: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ThreatIntelligenceStatistics::default())),
        }
    }

    pub fn add_ioc(&self, ioc: IndicatorOfCompromise) -> Result<()> {
        let mut iocs = self.iocs.write();
        iocs.insert(ioc.id.clone(), ioc);
        Ok(())
    }

    pub fn check_ioc(&self, ioc_type: IocType, value: &str) -> Option<IndicatorOfCompromise> {
        let iocs = self.iocs.read();
        iocs.values()
            .find(|ioc| ioc.ioc_type == ioc_type && ioc.value == value)
            .cloned()
    }

    pub fn get_ip_reputation(&self, ip: &str) -> ReputationScore {
        let mut stats = self.stats.write();
        stats.ip_lookups += 1;
        drop(stats);

        let reputation = self.ip_reputation.read();
        reputation.get(ip).cloned().unwrap_or_else(|| {
            ReputationScore {
                ip_address: ip.to_string(),
                score: 0.5,
                category: ReputationCategory::Unknown,
                last_updated: current_timestamp(),
                sources: Vec::new(),
            }
        })
    }

    pub fn calculate_threat_score(&self, ip: Option<&str>, user_id: &str) -> f64 {
        let mut score = 0.0;

        if let Some(ip_addr) = ip {
            let rep = self.get_ip_reputation(ip_addr);
            score += (1.0 - rep.score) * 0.5;
        }

        let iocs = self.iocs.read();
        let matching_iocs = iocs.values()
            .filter(|ioc| {
                match &ioc.ioc_type {
                    IocType::IpAddress => ip.map(|i| i == ioc.value).unwrap_or(false),
                    _ => false,
                }
            })
            .count();

        if matching_iocs > 0 {
            score += 0.5;
        }

        score.min(1.0)
    }

    pub fn get_statistics(&self) -> ThreatIntelligenceStatistics {
        let mut stats = self.stats.read().clone();
        stats.total_iocs = self.iocs.read().len();
        stats.vulnerabilities_tracked = self.vulnerabilities.read().len();
        stats
    }
}
