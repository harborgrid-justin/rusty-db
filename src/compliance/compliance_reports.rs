// # Compliance Reports
//
// Comprehensive compliance reporting engine for generating compliance reports,
// calculating compliance scores, tracking violations, and providing remediation
// recommendations.

use crate::error::{DbError, Result};
use crate::common::*;
use super::compliance_rules::{ComplianceFramework, RuleSeverity};
use super::compliance_scanner::{
    ComplianceFinding, FindingStatus, ScanResult, ResourceType,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

// ============================================================================
// Report Types
// ============================================================================

/// Type of compliance report
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportType {
    /// Executive summary report
    ExecutiveSummary,
    /// Detailed compliance report
    Detailed,
    /// Framework-specific report
    FrameworkSpecific(ComplianceFramework),
    /// Violation tracking report
    ViolationTracking,
    /// Remediation plan
    RemediationPlan,
    /// Trend analysis
    TrendAnalysis,
}

/// Report format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportFormat {
    /// JSON format
    Json,
    /// HTML format
    Html,
    /// PDF format (placeholder)
    Pdf,
    /// CSV format
    Csv,
}

// ============================================================================
// Compliance Report
// ============================================================================

/// Comprehensive compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    /// Report ID
    pub id: String,
    /// Report type
    pub report_type: ReportType,
    /// Generation timestamp
    pub generated_at: SystemTime,
    /// Report period start
    pub period_start: SystemTime,
    /// Report period end
    pub period_end: SystemTime,
    /// Compliance score
    pub compliance_score: ComplianceScore,
    /// Executive summary
    pub executive_summary: ExecutiveSummary,
    /// Framework scores
    pub framework_scores: HashMap<ComplianceFramework, FrameworkScore>,
    /// Violations by severity
    pub violations_by_severity: HashMap<RuleSeverity, usize>,
    /// Top violations
    pub top_violations: Vec<ViolationSummary>,
    /// Remediation recommendations
    pub recommendations: Vec<RemediationRecommendation>,
    /// Compliance trends
    pub trends: ComplianceTrends,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Overall compliance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceScore {
    /// Overall score (0-100)
    pub overall_score: f64,
    /// Total rules checked
    pub total_rules: usize,
    /// Rules passed
    pub rules_passed: usize,
    /// Rules failed
    pub rules_failed: usize,
    /// Compliance percentage
    pub compliance_percentage: f64,
    /// Risk level
    pub risk_level: RiskLevel,
}

/// Risk level assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Critical risk (0-40%)
    Critical,
    /// High risk (40-60%)
    High,
    /// Medium risk (60-80%)
    Medium,
    /// Low risk (80-95%)
    Low,
    /// Minimal risk (95-100%)
    Minimal,
}

impl RiskLevel {
    pub fn from_score(score: f64) -> Self {
        match score {
            s if s >= 95.0 => RiskLevel::Minimal,
            s if s >= 80.0 => RiskLevel::Low,
            s if s >= 60.0 => RiskLevel::Medium,
            s if s >= 40.0 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }
}

/// Executive summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveSummary {
    /// Overall compliance status
    pub status: ComplianceStatus,
    /// Key findings
    pub key_findings: Vec<String>,
    /// Critical issues count
    pub critical_issues: usize,
    /// High priority issues count
    pub high_priority_issues: usize,
    /// Improvement since last report
    pub improvement_percentage: Option<f64>,
    /// Summary text
    pub summary: String,
}

/// Compliance status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceStatus {
    /// Fully compliant
    Compliant,
    /// Partially compliant
    PartiallyCompliant,
    /// Non-compliant
    NonCompliant,
    /// Under review
    UnderReview,
}

/// Framework-specific score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkScore {
    /// Framework
    pub framework: ComplianceFramework,
    /// Score (0-100)
    pub score: f64,
    /// Total rules
    pub total_rules: usize,
    /// Passed rules
    pub passed_rules: usize,
    /// Failed rules
    pub failed_rules: usize,
    /// Status
    pub status: ComplianceStatus,
}

/// Violation summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationSummary {
    /// Rule ID
    pub rule_id: String,
    /// Rule name
    pub rule_name: String,
    /// Framework
    pub framework: ComplianceFramework,
    /// Severity
    pub severity: RuleSeverity,
    /// Number of occurrences
    pub occurrence_count: usize,
    /// Affected resources
    pub affected_resources: Vec<String>,
    /// First detected
    pub first_detected: SystemTime,
    /// Last detected
    pub last_detected: SystemTime,
    /// Status
    pub status: ViolationStatus,
}

/// Violation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationStatus {
    /// New violation
    New,
    /// Existing violation
    Existing,
    /// Remediated
    Remediated,
    /// Acknowledged
    Acknowledged,
    /// Accepted risk
    AcceptedRisk,
}

/// Remediation recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationRecommendation {
    /// Recommendation ID
    pub id: String,
    /// Priority
    pub priority: RemediationPriority,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Related violations
    pub related_violations: Vec<String>,
    /// Estimated effort
    pub estimated_effort: EffortLevel,
    /// Estimated impact
    pub estimated_impact: ImpactLevel,
    /// Steps
    pub steps: Vec<String>,
}

/// Remediation priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RemediationPriority {
    Critical,
    High,
    Medium,
    Low,
}

/// Effort level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffortLevel {
    /// Less than 1 hour
    Minimal,
    /// 1-8 hours
    Low,
    /// 1-3 days
    Medium,
    /// 3-10 days
    High,
    /// More than 10 days
    VeryHigh,
}

/// Impact level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImpactLevel {
    /// Minimal compliance improvement
    Minimal,
    /// Low compliance improvement
    Low,
    /// Medium compliance improvement
    Medium,
    /// High compliance improvement
    High,
    /// Critical compliance improvement
    Critical,
}

/// Compliance trends over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceTrends {
    /// Score history
    pub score_history: Vec<ScoreDataPoint>,
    /// Violation trends
    pub violation_trends: HashMap<RuleSeverity, Vec<TrendDataPoint>>,
    /// Framework trends
    pub framework_trends: HashMap<ComplianceFramework, Vec<ScoreDataPoint>>,
}

/// Score data point for trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreDataPoint {
    pub timestamp: SystemTime,
    pub score: f64,
}

/// Trend data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDataPoint {
    pub timestamp: SystemTime,
    pub count: usize,
}

// ============================================================================
// Compliance Report Generator
// ============================================================================

/// Compliance report generator
pub struct ComplianceReportGenerator {
    /// Generated reports
    reports: Arc<RwLock<HashMap<String, ComplianceReport>>>,
    /// Violation history
    violation_history: Arc<RwLock<Vec<ViolationRecord>>>,
    /// Score history
    score_history: Arc<RwLock<Vec<ScoreRecord>>>,
}

/// Violation record for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationRecord {
    pub timestamp: SystemTime,
    pub violation_id: String,
    pub rule_id: String,
    pub severity: RuleSeverity,
    pub framework: ComplianceFramework,
    pub resource: String,
    pub status: ViolationStatus,
}

/// Score record for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreRecord {
    pub timestamp: SystemTime,
    pub overall_score: f64,
    pub framework_scores: HashMap<ComplianceFramework, f64>,
}

impl ComplianceReportGenerator {
    /// Create a new compliance report generator
    pub fn new() -> Self {
        Self {
            reports: Arc::new(RwLock::new(HashMap::new())),
            violation_history: Arc::new(RwLock::new(Vec::new())),
            score_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Generate a compliance report from scan results
    pub fn generate_report(
        &self,
        report_type: ReportType,
        scan_results: Vec<ScanResult>,
        period_start: SystemTime,
        period_end: SystemTime,
    ) -> Result<ComplianceReport> {
        let report_id = self.generate_report_id();

        // Calculate compliance score
        let compliance_score = self.calculate_compliance_score(&scan_results)?;

        // Generate executive summary
        let executive_summary = self.generate_executive_summary(&scan_results, &compliance_score)?;

        // Calculate framework scores
        let framework_scores = self.calculate_framework_scores(&scan_results)?;

        // Get violations by severity
        let violations_by_severity = self.get_violations_by_severity(&scan_results);

        // Get top violations
        let top_violations = self.get_top_violations(&scan_results, 10)?;

        // Generate recommendations
        let recommendations = self.generate_recommendations(&scan_results)?;

        // Calculate trends
        let trends = self.calculate_trends(period_start, period_end)?;

        let report = ComplianceReport {
            id: report_id.clone(),
            report_type,
            generated_at: SystemTime::now(),
            period_start,
            period_end,
            compliance_score,
            executive_summary,
            framework_scores,
            violations_by_severity,
            top_violations,
            recommendations,
            trends,
            metadata: HashMap::new(),
        };

        // Store report
        self.reports.write()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?
            .insert(report_id, report.clone());

        // Record score
        self.record_score(&report.compliance_score, &report.framework_scores)?;

        Ok(report)
    }

    /// Calculate overall compliance score
    pub fn calculate_compliance_score(&self, scan_results: &[ScanResult]) -> Result<ComplianceScore> {
        let mut total_rules = 0;
        let mut rules_passed = 0;
        let mut rules_failed = 0;

        for scan in scan_results {
            for finding in &scan.findings {
                total_rules += 1;
                match finding.status {
                    FindingStatus::Pass => rules_passed += 1,
                    FindingStatus::Fail => rules_failed += 1,
                    _ => {}
                }
            }
        }

        let compliance_percentage = if total_rules > 0 {
            (rules_passed as f64 / total_rules as f64) * 100.0
        } else {
            100.0
        };

        let overall_score = compliance_percentage;
        let risk_level = RiskLevel::from_score(overall_score);

        Ok(ComplianceScore {
            overall_score,
            total_rules,
            rules_passed,
            rules_failed,
            compliance_percentage,
            risk_level,
        })
    }

    /// Generate executive summary
    fn generate_executive_summary(
        &self,
        scan_results: &[ScanResult],
        score: &ComplianceScore,
    ) -> Result<ExecutiveSummary> {
        let mut critical_issues = 0;
        let mut high_priority_issues = 0;
        let mut key_findings = Vec::new();

        for scan in scan_results {
            for finding in &scan.findings {
                if finding.status == FindingStatus::Fail {
                    match finding.severity {
                        RuleSeverity::Critical => critical_issues += 1,
                        RuleSeverity::High => high_priority_issues += 1,
                        _ => {}
                    }
                }
            }
        }

        // Generate key findings
        if critical_issues > 0 {
            key_findings.push(format!("{} critical compliance issues require immediate attention", critical_issues));
        }
        if high_priority_issues > 0 {
            key_findings.push(format!("{} high-priority compliance issues identified", high_priority_issues));
        }

        let status = match score.risk_level {
            RiskLevel::Minimal | RiskLevel::Low => ComplianceStatus::Compliant,
            RiskLevel::Medium => ComplianceStatus::PartiallyCompliant,
            _ => ComplianceStatus::NonCompliant,
        };

        let summary = format!(
            "Overall compliance score: {:.1}%. Risk level: {:?}. {} rules checked, {} passed, {} failed.",
            score.compliance_percentage,
            score.risk_level,
            score.total_rules,
            score.rules_passed,
            score.rules_failed
        );

        Ok(ExecutiveSummary {
            status,
            key_findings,
            critical_issues,
            high_priority_issues,
            improvement_percentage: None,
            summary,
        })
    }

    /// Calculate framework-specific scores
    fn calculate_framework_scores(&self, scan_results: &[ScanResult]) -> Result<HashMap<ComplianceFramework, FrameworkScore>> {
        let mut framework_stats: HashMap<ComplianceFramework, (usize, usize, usize)> = HashMap::new();

        for scan in scan_results {
            for finding in &scan.findings {
                let stats = framework_stats.entry(finding.framework).or_insert((0, 0, 0));
                stats.0 += 1; // total
                match finding.status {
                    FindingStatus::Pass => stats.1 += 1, // passed
                    FindingStatus::Fail => stats.2 += 1, // failed
                    _ => {}
                }
            }
        }

        let mut scores = HashMap::new();
        for (framework, (total, passed, failed)) in framework_stats {
            let score = if total > 0 {
                (passed as f64 / total as f64) * 100.0
            } else {
                100.0
            };

            let status = if score >= 80.0 {
                ComplianceStatus::Compliant
            } else if score >= 60.0 {
                ComplianceStatus::PartiallyCompliant
            } else {
                ComplianceStatus::NonCompliant
            };

            scores.insert(framework, FrameworkScore {
                framework,
                score,
                total_rules: total,
                passed_rules: passed,
                failed_rules: failed,
                status,
            });
        }

        Ok(scores)
    }

    /// Get violations grouped by severity
    fn get_violations_by_severity(&self, scan_results: &[ScanResult]) -> HashMap<RuleSeverity, usize> {
        let mut violations = HashMap::new();

        for scan in scan_results {
            for finding in &scan.findings {
                if finding.status == FindingStatus::Fail {
                    *violations.entry(finding.severity).or_insert(0) += 1;
                }
            }
        }

        violations
    }

    /// Get top violations
    fn get_top_violations(&self, scan_results: &[ScanResult], limit: usize) -> Result<Vec<ViolationSummary>> {
        let mut violation_map: HashMap<String, ViolationSummary> = HashMap::new();

        for scan in scan_results {
            for finding in &scan.findings {
                if finding.status == FindingStatus::Fail {
                    let entry = violation_map.entry(finding.rule_id.clone()).or_insert_with(|| {
                        ViolationSummary {
                            rule_id: finding.rule_id.clone(),
                            rule_name: finding.rule_name.clone(),
                            framework: finding.framework,
                            severity: finding.severity,
                            occurrence_count: 0,
                            affected_resources: Vec::new(),
                            first_detected: finding.timestamp,
                            last_detected: finding.timestamp,
                            status: ViolationStatus::New,
                        }
                    });

                    entry.occurrence_count += 1;
                    entry.affected_resources.push(format!("{:?}", finding.resource));
                    if finding.timestamp < entry.first_detected {
                        entry.first_detected = finding.timestamp;
                    }
                    if finding.timestamp > entry.last_detected {
                        entry.last_detected = finding.timestamp;
                    }
                }
            }
        }

        let mut violations: Vec<_> = violation_map.into_values().collect();
        violations.sort_by(|a, b| {
            b.severity.cmp(&a.severity)
                .then_with(|| b.occurrence_count.cmp(&a.occurrence_count))
        });

        Ok(violations.into_iter().take(limit).collect())
    }

    /// Generate remediation recommendations
    fn generate_recommendations(&self, scan_results: &[ScanResult]) -> Result<Vec<RemediationRecommendation>> {
        let mut recommendations = Vec::new();
        let mut recommendation_map: HashMap<String, (RuleSeverity, Vec<String>)> = HashMap::new();

        // Group violations by remediation
        for scan in scan_results {
            for finding in &scan.findings {
                if finding.status == FindingStatus::Fail {
                    let key = finding.remediation.clone();
                    let entry = recommendation_map.entry(key).or_insert((finding.severity, Vec::new()));
                    entry.1.push(finding.id.clone());
                }
            }
        }

        // Generate recommendations
        for (remediation, (severity, violations)) in recommendation_map {
            let priority = match severity {
                RuleSeverity::Critical => RemediationPriority::Critical,
                RuleSeverity::High => RemediationPriority::High,
                RuleSeverity::Medium => RemediationPriority::Medium,
                _ => RemediationPriority::Low,
            };

            recommendations.push(RemediationRecommendation {
                id: format!("REC-{}", recommendations.len() + 1),
                priority,
                title: remediation.clone(),
                description: format!("Remediate {} violations", violations.len()),
                related_violations: violations,
                estimated_effort: EffortLevel::Medium,
                estimated_impact: ImpactLevel::High,
                steps: vec![remediation],
            });
        }

        // Sort by priority
        recommendations.sort_by_key(|r| r.priority);

        Ok(recommendations)
    }

    /// Calculate compliance trends
    fn calculate_trends(&self, _start: SystemTime, _end: SystemTime) -> Result<ComplianceTrends> {
        let score_history = self.score_history.read()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?;

        let score_data: Vec<ScoreDataPoint> = score_history
            .iter()
            .map(|record| ScoreDataPoint {
                timestamp: record.timestamp,
                score: record.overall_score,
            })
            .collect();

        Ok(ComplianceTrends {
            score_history: score_data,
            violation_trends: HashMap::new(),
            framework_trends: HashMap::new(),
        })
    }

    /// Record compliance score for trend tracking
    fn record_score(&self, score: &ComplianceScore, framework_scores: &HashMap<ComplianceFramework, FrameworkScore>) -> Result<()> {
        let record = ScoreRecord {
            timestamp: SystemTime::now(),
            overall_score: score.overall_score,
            framework_scores: framework_scores
                .iter()
                .map(|(f, s)| (*f, s.score))
                .collect(),
        };

        self.score_history.write()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?
            .push(record);

        Ok(())
    }

    /// Get report by ID
    pub fn get_report(&self, report_id: &str) -> Result<ComplianceReport> {
        let reports = self.reports.read()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?;

        reports.get(report_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("Report not found: {}", report_id)))
    }

    fn generate_report_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("REPORT-{}", timestamp)
    }
}

impl Default for ComplianceReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_from_score() {
        assert_eq!(RiskLevel::from_score(96.0), RiskLevel::Minimal);
        assert_eq!(RiskLevel::from_score(85.0), RiskLevel::Low);
        assert_eq!(RiskLevel::from_score(70.0), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_score(50.0), RiskLevel::High);
        assert_eq!(RiskLevel::from_score(30.0), RiskLevel::Critical);
    }

    #[test]
    fn test_report_generator() {
        let generator = ComplianceReportGenerator::new();
        let score = generator.calculate_compliance_score(&[]).unwrap();

        assert_eq!(score.total_rules, 0);
        assert_eq!(score.overall_score, 100.0);
    }
}
