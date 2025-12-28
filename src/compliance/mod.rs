// # Compliance Module
//
// Enterprise-grade compliance reporting engine for RustyDB providing comprehensive
// compliance validation, scanning, and reporting capabilities for major regulatory
// frameworks including GDPR, HIPAA, SOC2, and PCI-DSS.
//
// ## Architecture
//
// The compliance module is organized into three core submodules:
//
// ### 1. Compliance Rules Engine ([`compliance_rules`])
// Defines and manages compliance rules for multiple regulatory frameworks:
// - **GDPR**: General Data Protection Regulation (EU)
// - **HIPAA**: Health Insurance Portability and Accountability Act (US)
// - **SOC2**: Service Organization Control 2
// - **PCI-DSS**: Payment Card Industry Data Security Standard
// - **Custom**: User-defined compliance frameworks
//
// Features:
// - Rule definitions with severity levels and categories
// - Framework-specific rule sets
// - Custom rule validators
// - Rule requirements and validation types
//
// ### 2. Compliance Scanner ([`compliance_scanner`])
// Automated compliance scanning and validation:
// - Schema compliance scanning
// - Data classification detection
// - Access control auditing
// - Encryption verification
// - Configuration validation
//
// Features:
// - Multiple scan types (Full, Schema, Data Classification, etc.)
// - Configurable scan scope
// - Finding generation and tracking
// - Resource identification
//
// ### 3. Compliance Reports ([`compliance_reports`])
// Comprehensive compliance reporting and analytics:
// - Compliance score calculation
// - Executive summaries
// - Framework-specific scores
// - Violation tracking
// - Remediation recommendations
// - Trend analysis
//
// Features:
// - Multiple report types
// - Risk level assessment
// - Violation history tracking
// - Compliance trends over time
//
// ## Usage Example
//
// ```rust,no_run
// use rusty_db::compliance::*;
//
// # fn example() -> rusty_db::Result<()> {
// // Create compliance engine
// let engine = ComplianceEngine::new();
//
// // Add custom compliance rules
// let rule = ComplianceRule::new(
//     "CUSTOM-001".to_string(),
//     "Data Encryption Required".to_string(),
//     ComplianceFramework::Custom,
//     RuleCategory::EncryptionAtRest,
//     RuleSeverity::Critical,
// );
// engine.rules_engine.add_rule(rule)?;
//
// // Perform compliance scan
// let scan_config = ScanConfig::default();
// let scan_id = engine.scan_database("mydb", scan_config)?;
//
// // Generate compliance report
// let report = engine.generate_report(
//     ReportType::ExecutiveSummary,
//     vec![scan_id],
// )?;
//
// println!("Compliance Score: {:.1}%", report.compliance_score.overall_score);
// println!("Risk Level: {:?}", report.compliance_score.risk_level);
// # Ok(())
// # }
// ```
//
// ## Compliance Frameworks
//
// ### GDPR (General Data Protection Regulation)
// - Data encryption at rest and in transit
// - Audit logging for personal data access
// - Data retention and deletion policies
// - Right to be forgotten implementation
// - Data portability support
//
// ### HIPAA (Health Insurance Portability and Accountability Act)
// - PHI (Protected Health Information) encryption
// - Access control implementation
// - Audit trails for PHI access
// - Breach notification procedures
// - Business associate agreements
//
// ### SOC2 (Service Organization Control 2)
// - Security controls
// - Availability controls
// - Processing integrity
// - Confidentiality
// - Privacy controls
//
// ### PCI-DSS (Payment Card Industry Data Security Standard)
// - Cardholder data encryption
// - Access control and monitoring
// - Vulnerability management
// - Network security
// - Regular security testing
//
// ## Integration
//
// The compliance module integrates with:
// - Security module for access control validation
// - Encryption module for encryption verification
// - Audit module for logging compliance
// - Catalog module for schema metadata

use crate::error::{DbError, Result};
use crate::common::*;
use std::sync::Arc;
use std::time::SystemTime;

// Re-export all submodules
pub mod compliance_rules;
pub mod compliance_scanner;
pub mod compliance_reports;

// Re-export commonly used types from compliance_rules
pub use compliance_rules::{
    ComplianceFramework, ComplianceRule, ComplianceRulesEngine, ComplianceRulesStatistics,
    RuleSeverity, RuleCategory, RuleRequirement, ValidationType, ComparisonOperator,
    CustomRuleValidator, ComplianceContext,
};

// Re-export commonly used types from compliance_scanner
pub use compliance_scanner::{
    ComplianceScanner, ScanConfig, ScanType, ScanResult, ScanStatus,
    ComplianceFinding, FindingStatus, ResourceIdentifier, ResourceType,
    SchemaMetadata, TableMetadata, ColumnMetadata, DataClassification,
    EncryptionStatus, AccessControlConfig,
};

// Re-export commonly used types from compliance_reports
pub use compliance_reports::{
    ComplianceReportGenerator, ComplianceReport, ReportType, ReportFormat,
    ComplianceScore, RiskLevel, ExecutiveSummary, ComplianceStatus,
    FrameworkScore, ViolationSummary, ViolationStatus,
    RemediationRecommendation, RemediationPriority, EffortLevel, ImpactLevel,
    ComplianceTrends, ScoreDataPoint, TrendDataPoint,
};

// ============================================================================
// Integrated Compliance Engine
// ============================================================================

/// Integrated compliance engine combining all compliance subsystems
pub struct ComplianceEngine {
    /// Rules engine
    pub rules_engine: Arc<ComplianceRulesEngine>,
    /// Scanner
    pub scanner: Arc<ComplianceScanner>,
    /// Report generator
    pub report_generator: Arc<ComplianceReportGenerator>,
}

impl ComplianceEngine {
    /// Create a new integrated compliance engine
    pub fn new() -> Self {
        Self {
            rules_engine: Arc::new(ComplianceRulesEngine::new()),
            scanner: Arc::new(ComplianceScanner::new()),
            report_generator: Arc::new(ComplianceReportGenerator::new()),
        }
    }

    /// Perform a full compliance scan on a database
    pub fn scan_database(&self, database: &str, config: ScanConfig) -> Result<String> {
        // Get enabled rules based on configuration
        let rules = if let Some(ref frameworks) = config.frameworks {
            let mut all_rules = Vec::new();
            for framework in frameworks {
                all_rules.extend(self.rules_engine.get_rules_by_framework(*framework)?);
            }
            all_rules
        } else {
            self.rules_engine.get_enabled_rules()?
        };

        // Start scan
        let scan_id = self.scanner.start_scan(config.clone(), rules.clone())?;

        // Perform different scan types based on configuration
        let mut all_findings = Vec::new();

        match config.scan_type {
            ScanType::Full => {
                // Perform all scan types
                all_findings.extend(self.scanner.scan_data_classification(database)?);
                all_findings.extend(self.scanner.audit_access_control(database)?);
                all_findings.extend(self.scanner.verify_encryption(database)?);
            }
            ScanType::SchemaOnly => {
                // Schema scan only
                if let Some(ref schemas) = config.schemas {
                    for schema in schemas {
                        all_findings.extend(self.scanner.scan_schema(database, schema, &rules)?);
                    }
                }
            }
            ScanType::DataClassification => {
                all_findings.extend(self.scanner.scan_data_classification(database)?);
            }
            ScanType::AccessControl => {
                all_findings.extend(self.scanner.audit_access_control(database)?);
            }
            ScanType::Encryption => {
                all_findings.extend(self.scanner.verify_encryption(database)?);
            }
            ScanType::Framework(framework) => {
                let framework_rules = self.rules_engine.get_rules_by_framework(framework)?;
                all_findings.extend(self.scanner.scan_data_classification(database)?);
            }
        }

        // Complete scan with findings
        self.scanner.complete_scan(&scan_id, all_findings)?;

        Ok(scan_id)
    }

    /// Generate a compliance report
    pub fn generate_report(
        &self,
        report_type: ReportType,
        scan_ids: Vec<String>,
    ) -> Result<ComplianceReport> {
        let mut scan_results = Vec::new();

        for scan_id in scan_ids {
            let result = self.scanner.get_scan_result(&scan_id)?;
            scan_results.push(result);
        }

        // Determine report period
        let period_start = scan_results
            .iter()
            .map(|s| s.started_at)
            .min()
            .unwrap_or_else(SystemTime::now);

        let period_end = scan_results
            .iter()
            .filter_map(|s| s.completed_at)
            .max()
            .unwrap_or_else(SystemTime::now);

        self.report_generator.generate_report(
            report_type,
            scan_results,
            period_start,
            period_end,
        )
    }

    /// Get compliance score for a database
    pub fn get_compliance_score(&self, scan_id: &str) -> Result<ComplianceScore> {
        let scan_result = self.scanner.get_scan_result(scan_id)?;
        self.report_generator.calculate_compliance_score(&[scan_result])
    }

    /// Get all rules for a framework
    pub fn get_framework_rules(&self, framework: ComplianceFramework) -> Result<Vec<ComplianceRule>> {
        self.rules_engine.get_rules_by_framework(framework)
    }

    /// Add a custom compliance rule
    pub fn add_custom_rule(&self, rule: ComplianceRule) -> Result<()> {
        self.rules_engine.add_rule(rule)
    }

    /// Get compliance statistics
    pub fn get_statistics(&self) -> ComplianceStatistics {
        ComplianceStatistics {
            rules_stats: self.rules_engine.get_statistics(),
            total_scans: 0, // Would track actual scans
            active_scans: 0, // Would track active scans
            total_reports: 0, // Would track generated reports
        }
    }
}

impl Default for ComplianceEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined compliance statistics
#[derive(Debug, Clone)]
pub struct ComplianceStatistics {
    pub rules_stats: ComplianceRulesStatistics,
    pub total_scans: usize,
    pub active_scans: usize,
    pub total_reports: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_engine_creation() {
        let engine = ComplianceEngine::new();
        let stats = engine.get_statistics();

        assert!(stats.rules_stats.total_rules > 0);
        assert!(stats.rules_stats.enabled_rules > 0);
    }

    #[test]
    fn test_get_framework_rules() {
        let engine = ComplianceEngine::new();

        let gdpr_rules = engine.get_framework_rules(ComplianceFramework::GDPR).unwrap();
        assert!(!gdpr_rules.is_empty());
        assert!(gdpr_rules.iter().all(|r| r.framework == ComplianceFramework::GDPR));

        let hipaa_rules = engine.get_framework_rules(ComplianceFramework::HIPAA).unwrap();
        assert!(!hipaa_rules.is_empty());
        assert!(hipaa_rules.iter().all(|r| r.framework == ComplianceFramework::HIPAA));

        let soc2_rules = engine.get_framework_rules(ComplianceFramework::SOC2).unwrap();
        assert!(!soc2_rules.is_empty());
        assert!(soc2_rules.iter().all(|r| r.framework == ComplianceFramework::SOC2));

        let pci_rules = engine.get_framework_rules(ComplianceFramework::PCIDSS).unwrap();
        assert!(!pci_rules.is_empty());
        assert!(pci_rules.iter().all(|r| r.framework == ComplianceFramework::PCIDSS));
    }

    #[test]
    fn test_add_custom_rule() {
        let engine = ComplianceEngine::new();

        let rule = ComplianceRule::new(
            "CUSTOM-TEST-001".to_string(),
            "Test Custom Rule".to_string(),
            ComplianceFramework::Custom,
            RuleCategory::Custom("Testing".to_string()),
            RuleSeverity::Medium,
        );

        assert!(engine.add_custom_rule(rule).is_ok());
    }

    #[test]
    fn test_compliance_frameworks() {
        assert_eq!(ComplianceFramework::GDPR.as_str(), "GDPR");
        assert_eq!(ComplianceFramework::HIPAA.as_str(), "HIPAA");
        assert_eq!(ComplianceFramework::SOC2.as_str(), "SOC2");
        assert_eq!(ComplianceFramework::PCIDSS.as_str(), "PCI-DSS");
        assert_eq!(ComplianceFramework::Custom.as_str(), "Custom");
    }
}
