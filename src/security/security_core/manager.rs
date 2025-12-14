// # Security Core Manager
//
// Unified security core orchestration and security dashboard.

use crate::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::super::IntegratedSecurityManager;
use super::access_control::*;
use super::security_policies::*;
use super::threat_detection::*;

// ============================================================================
// Unified Security Core
// ============================================================================

pub struct UnifiedSecurityCore {
    #[allow(dead_code)]
    security_manager: Arc<IntegratedSecurityManager>,
    event_correlator: Arc<SecurityEventCorrelator>,
    threat_intelligence: Arc<ThreatIntelligence>,
    policy_engine: Arc<SecurityPolicyEngine>,
    defense_orchestrator: Arc<DefenseOrchestrator>,
    compliance_validator: Arc<ComplianceValidator>,
    security_metrics: Arc<SecurityMetrics>,
    pen_test_harness: Arc<PenetrationTestHarness>,
}

impl UnifiedSecurityCore {
    pub fn new(security_manager: Arc<IntegratedSecurityManager>) -> Self {
        let defense_orchestrator =
            Arc::new(DefenseOrchestrator::new(Arc::clone(&security_manager)));

        Self {
            security_manager: Arc::clone(&security_manager),
            event_correlator: Arc::new(SecurityEventCorrelator::new()),
            threat_intelligence: Arc::new(ThreatIntelligence::new()),
            policy_engine: Arc::new(SecurityPolicyEngine::new()),
            defense_orchestrator,
            compliance_validator: Arc::new(ComplianceValidator::new()),
            security_metrics: Arc::new(SecurityMetrics::new()),
            pen_test_harness: Arc::new(PenetrationTestHarness::new()),
        }
    }

    pub fn record_security_event(&self, event: CorrelatedEvent) -> Result<()> {
        self.event_correlator.add_event(event)?;
        self.security_metrics.record("security_events", 1.0);
        Ok(())
    }

    pub fn check_threat(&self, user_id: &str, ip: Option<&str>) -> Result<f64> {
        let threat_score = self.threat_intelligence.calculate_threat_score(ip, user_id);
        self.security_metrics.record("threat_score", threat_score);
        Ok(threat_score)
    }

    pub fn evaluate_policy(&self, context: &EvaluationContext) -> Result<PolicyDecision> {
        let decision = self.policy_engine.evaluate(context)?;
        self.security_metrics.record("policy_evaluations", 1.0);
        Ok(decision)
    }

    pub fn validate_compliance(&self, framework_id: &str) -> Result<f64> {
        self.compliance_validator
            .calculate_framework_score(framework_id)
    }

    pub fn run_security_tests(&self) -> Result<PenTestReport> {
        self.pen_test_harness.run_all_tests()
    }

    pub fn get_security_posture(&self) -> SecurityPostureScore {
        self.security_metrics.calculate_security_posture()
    }

    pub fn get_event_correlator(&self) -> &Arc<SecurityEventCorrelator> {
        &self.event_correlator
    }

    pub fn get_threat_intelligence(&self) -> &Arc<ThreatIntelligence> {
        &self.threat_intelligence
    }

    pub fn get_policy_engine(&self) -> &Arc<SecurityPolicyEngine> {
        &self.policy_engine
    }

    pub fn get_defense_orchestrator(&self) -> &Arc<DefenseOrchestrator> {
        &self.defense_orchestrator
    }

    pub fn get_compliance_validator(&self) -> &Arc<ComplianceValidator> {
        &self.compliance_validator
    }

    pub fn get_security_metrics(&self) -> &Arc<SecurityMetrics> {
        &self.security_metrics
    }

    pub fn get_pen_test_harness(&self) -> &Arc<PenetrationTestHarness> {
        &self.pen_test_harness
    }
}

// ============================================================================
// Security Dashboard
// ============================================================================

// Type alias for SecurityDashboard
pub type DashboardView = SecurityDashboard;

pub struct SecurityDashboard {
    security_core: Arc<UnifiedSecurityCore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub active_threats: usize,
    pub active_incidents: Vec<SecurityIncident>,
    pub threat_level: ThreatLevel,
    pub security_posture: SecurityPostureScore,
    pub compliance_summary: ComplianceSummary,
    pub recent_events: Vec<CorrelatedEvent>,
    pub defense_coverage: DefenseCoverageReport,
    pub pen_test_summary: PenTestSummary,
    pub correlator_stats: CorrelatorStatistics,
    pub policy_stats: PolicyEngineStatistics,
    pub threat_intel_stats: ThreatIntelligenceStatistics,
}

// Executive summary of security status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveSummary {
    pub overall_security_score: f64,
    pub threat_level: ThreatLevel,
    pub active_incidents: usize,
    pub compliance_score: f64,
    pub recommendations: Vec<String>,
    pub key_metrics: Vec<(String, f64)>,
}

// Security status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityStatus {
    Secure,
    Warning,
    Critical,
    UnderAttack,
}

impl SecurityDashboard {
    pub fn new(security_core: Arc<UnifiedSecurityCore>) -> Self {
        Self { security_core }
    }

    pub fn get_dashboard_data(&self) -> DashboardData {
        let active_incidents = self
            .security_core
            .get_event_correlator()
            .get_active_incidents();
        let threat_level = self
            .security_core
            .get_defense_orchestrator()
            .get_threat_level();
        let security_posture = self.security_core.get_security_posture();
        let compliance_summary = self
            .security_core
            .get_compliance_validator()
            .get_compliance_summary();
        let defense_coverage = self
            .security_core
            .get_defense_orchestrator()
            .validate_coverage();
        let pen_test_summary = self.security_core.get_pen_test_harness().get_test_summary();
        let correlator_stats = self.security_core.get_event_correlator().get_statistics();
        let policy_stats = self.security_core.get_policy_engine().get_statistics();
        let threat_intel_stats = self
            .security_core
            .get_threat_intelligence()
            .get_statistics();

        DashboardData {
            active_threats: active_incidents.len(),
            active_incidents,
            threat_level,
            security_posture,
            compliance_summary,
            recent_events: Vec::new(),
            defense_coverage,
            pen_test_summary,
            correlator_stats,
            policy_stats,
            threat_intel_stats,
        }
    }
}
