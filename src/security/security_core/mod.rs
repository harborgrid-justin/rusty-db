// # Security Core Module
//
// Comprehensive security core with modular components.

pub mod common;
pub mod threat_detection;
pub mod access_control;
pub mod security_policies;
pub mod manager;

// Re-export main types
pub use threat_detection::{
    SecurityEventCorrelator, CorrelatedEvent, EventSeverity,
    AttackPattern, EventIndicator, SecurityIncident, IncidentStatus,
    CorrelationRule, CorrelatorStatistics,
    ThreatIntelligence, IndicatorOfCompromise, IocType,
    ThreatActor, ThreatSophistication, Vulnerability,
    ReputationScore, ReputationCategory, ThreatIntelligenceStatistics,
};

pub use access_control::{
    SecurityPolicyEngine, PolicyId, SecurityPolicy, PolicyType,
    PolicyRule, PolicyEffect, PolicyCondition, ConditionOperator,
    PolicyDecision, AttributeProvider, EvaluationContext,
    PolicyEngineStatistics,
    DefenseOrchestrator, DefenseLayer, LayerStatus,
    ThreatLevel, DefenseCoverageReport,
};

pub use security_policies::{
    ComplianceValidator, ComplianceFramework, ComplianceControl,
    ControlAssessment, ComplianceStatus, ComplianceEvidence,
    ComplianceSummary,
    SecurityMetrics, MetricValue, TimeSeriesPoint, SecurityPostureScore,
    PenetrationTestHarness, PenTestResult, TestStatus, PenTestScenario,
    TestCategory, PenTestReport, PenTestSummary,
};

pub use manager::{
    UnifiedSecurityCore, SecurityDashboard, DashboardData,
    DashboardView, ExecutiveSummary, SecurityStatus,
};
