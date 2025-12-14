// # Security Core Module
//
// Comprehensive security core with modular components.

pub mod access_control;
pub mod common;
pub mod manager;
pub mod security_policies;
pub mod threat_detection;

// Re-export main types
pub use threat_detection::{
    AttackPattern, CorrelatedEvent, CorrelationRule, CorrelatorStatistics, EventIndicator,
    EventSeverity, IncidentStatus, IndicatorOfCompromise, IocType, ReputationCategory,
    ReputationScore, SecurityEventCorrelator, SecurityIncident, ThreatActor, ThreatIntelligence,
    ThreatIntelligenceStatistics, ThreatSophistication, Vulnerability,
};

pub use access_control::{
    AttributeProvider, ConditionOperator, DefenseCoverageReport, DefenseLayer, DefenseOrchestrator,
    EvaluationContext, LayerStatus, PolicyCondition, PolicyDecision, PolicyEffect,
    PolicyEngineStatistics, PolicyId, PolicyRule, PolicyType, SecurityPolicy, SecurityPolicyEngine,
    ThreatLevel,
};

pub use security_policies::{
    ComplianceControl, ComplianceEvidence, ComplianceFramework, ComplianceStatus,
    ComplianceSummary, ComplianceValidator, ControlAssessment, MetricValue, PenTestReport,
    PenTestResult, PenTestScenario, PenTestSummary, PenetrationTestHarness, SecurityMetrics,
    SecurityPostureScore, TestCategory, TestStatus, TimeSeriesPoint,
};

pub use manager::{
    DashboardData, DashboardView, ExecutiveSummary, SecurityDashboard, SecurityStatus,
    UnifiedSecurityCore,
};
