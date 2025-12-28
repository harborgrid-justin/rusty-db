// # Compliance Rules Engine
//
// Comprehensive compliance rules engine for regulatory frameworks including GDPR, HIPAA,
// SOC2, and PCI-DSS. Provides rule definitions, validation, and enforcement capabilities.

use crate::error::{DbError, Result};
use crate::common::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

// ============================================================================
// Compliance Framework Types
// ============================================================================

/// Supported compliance frameworks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceFramework {
    /// General Data Protection Regulation (EU)
    GDPR,
    /// Health Insurance Portability and Accountability Act (US)
    HIPAA,
    /// Service Organization Control 2
    SOC2,
    /// Payment Card Industry Data Security Standard
    PCIDSS,
    /// Custom framework
    Custom,
}

impl ComplianceFramework {
    pub fn as_str(&self) -> &str {
        match self {
            ComplianceFramework::GDPR => "GDPR",
            ComplianceFramework::HIPAA => "HIPAA",
            ComplianceFramework::SOC2 => "SOC2",
            ComplianceFramework::PCIDSS => "PCI-DSS",
            ComplianceFramework::Custom => "Custom",
        }
    }
}

/// Severity level of compliance rule
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RuleSeverity {
    /// Critical violation, immediate action required
    Critical,
    /// High priority violation
    High,
    /// Medium priority violation
    Medium,
    /// Low priority violation
    Low,
    /// Informational
    Info,
}

/// Rule category
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuleCategory {
    /// Data encryption at rest
    EncryptionAtRest,
    /// Data encryption in transit
    EncryptionInTransit,
    /// Access control and authentication
    AccessControl,
    /// Audit logging and monitoring
    AuditLogging,
    /// Data retention and deletion
    DataRetention,
    /// Data privacy and masking
    DataPrivacy,
    /// Backup and recovery
    BackupRecovery,
    /// Network security
    NetworkSecurity,
    /// Vulnerability management
    VulnerabilityManagement,
    /// Incident response
    IncidentResponse,
    /// Custom category
    Custom(String),
}

// ============================================================================
// Compliance Rule Definition
// ============================================================================

/// Compliance rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRule {
    /// Unique rule identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Compliance framework
    pub framework: ComplianceFramework,
    /// Rule category
    pub category: RuleCategory,
    /// Severity level
    pub severity: RuleSeverity,
    /// Whether rule is enabled
    pub enabled: bool,
    /// Rule requirements
    pub requirements: Vec<RuleRequirement>,
    /// Remediation guidance
    pub remediation: String,
    /// Regulatory reference
    pub reference: String,
    /// Created timestamp
    pub created_at: SystemTime,
    /// Updated timestamp
    pub updated_at: SystemTime,
}

impl ComplianceRule {
    pub fn new(
        id: String,
        name: String,
        framework: ComplianceFramework,
        category: RuleCategory,
        severity: RuleSeverity,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            name,
            description: String::new(),
            framework,
            category,
            severity,
            enabled: true,
            requirements: Vec::new(),
            remediation: String::new(),
            reference: String::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    pub fn with_remediation(mut self, remediation: String) -> Self {
        self.remediation = remediation;
        self
    }

    pub fn with_reference(mut self, reference: String) -> Self {
        self.reference = reference;
        self
    }

    pub fn with_requirement(mut self, requirement: RuleRequirement) -> Self {
        self.requirements.push(requirement);
        self
    }
}

/// Rule requirement definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleRequirement {
    /// Requirement ID
    pub id: String,
    /// Requirement description
    pub description: String,
    /// Validation type
    pub validation_type: ValidationType,
    /// Expected value
    pub expected_value: Option<String>,
    /// Operator for comparison
    pub operator: ComparisonOperator,
}

/// Validation type for rule requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationType {
    /// Check if encryption is enabled on table
    TableEncryption,
    /// Check if column is encrypted
    ColumnEncryption,
    /// Check if audit logging is enabled
    AuditEnabled,
    /// Check if access control is configured
    AccessControlConfigured,
    /// Check data retention policy
    DataRetentionPolicy,
    /// Check if backups are configured
    BackupConfigured,
    /// Check password policy strength
    PasswordPolicy,
    /// Check if MFA is enabled
    MfaEnabled,
    /// Check SSL/TLS configuration
    SslTlsConfigured,
    /// Check vulnerability scan status
    VulnerabilityScanStatus,
    /// Custom validation query
    CustomQuery(String),
}

/// Comparison operator for validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    NotContains,
    Exists,
    NotExists,
}

// ============================================================================
// Compliance Rules Engine
// ============================================================================

/// Compliance rules engine
pub struct ComplianceRulesEngine {
    /// All compliance rules indexed by ID
    rules: Arc<RwLock<HashMap<String, ComplianceRule>>>,
    /// Rules indexed by framework
    framework_index: Arc<RwLock<HashMap<ComplianceFramework, Vec<String>>>>,
    /// Rules indexed by category
    category_index: Arc<RwLock<HashMap<RuleCategory, Vec<String>>>>,
    /// Custom rules registry
    custom_rules: Arc<RwLock<HashMap<String, CustomRuleValidator>>>,
}

/// Custom rule validator function
pub type CustomRuleValidator = Arc<dyn Fn(&ComplianceContext) -> Result<bool> + Send + Sync>;

/// Context for compliance validation
#[derive(Debug, Clone)]
pub struct ComplianceContext {
    /// Database name
    pub database: String,
    /// Schema name
    pub schema: Option<String>,
    /// Table name
    pub table: Option<String>,
    /// Column name
    pub column: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ComplianceRulesEngine {
    /// Create a new compliance rules engine
    pub fn new() -> Self {
        let engine = Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            framework_index: Arc::new(RwLock::new(HashMap::new())),
            category_index: Arc::new(RwLock::new(HashMap::new())),
            custom_rules: Arc::new(RwLock::new(HashMap::new())),
        };

        // Initialize default rules
        engine.initialize_default_rules();
        engine
    }

    /// Initialize default compliance rules
    fn initialize_default_rules(&self) {
        // GDPR Rules
        self.add_rule(
            ComplianceRule::new(
                "GDPR-001".to_string(),
                "Data Encryption at Rest".to_string(),
                ComplianceFramework::GDPR,
                RuleCategory::EncryptionAtRest,
                RuleSeverity::Critical,
            )
            .with_description("Personal data must be encrypted at rest".to_string())
            .with_remediation("Enable transparent data encryption (TDE) for all tables containing personal data".to_string())
            .with_reference("GDPR Article 32(1)(a)".to_string())
            .with_requirement(RuleRequirement {
                id: "REQ-001".to_string(),
                description: "Table encryption enabled".to_string(),
                validation_type: ValidationType::TableEncryption,
                expected_value: Some("true".to_string()),
                operator: ComparisonOperator::Equals,
            }),
        ).ok();

        self.add_rule(
            ComplianceRule::new(
                "GDPR-002".to_string(),
                "Audit Logging Enabled".to_string(),
                ComplianceFramework::GDPR,
                RuleCategory::AuditLogging,
                RuleSeverity::High,
            )
            .with_description("All access to personal data must be logged".to_string())
            .with_remediation("Enable comprehensive audit logging for all tables".to_string())
            .with_reference("GDPR Article 30".to_string())
            .with_requirement(RuleRequirement {
                id: "REQ-002".to_string(),
                description: "Audit logging enabled".to_string(),
                validation_type: ValidationType::AuditEnabled,
                expected_value: Some("true".to_string()),
                operator: ComparisonOperator::Equals,
            }),
        ).ok();

        // HIPAA Rules
        self.add_rule(
            ComplianceRule::new(
                "HIPAA-001".to_string(),
                "PHI Encryption Required".to_string(),
                ComplianceFramework::HIPAA,
                RuleCategory::EncryptionAtRest,
                RuleSeverity::Critical,
            )
            .with_description("Protected Health Information (PHI) must be encrypted".to_string())
            .with_remediation("Enable column-level encryption for PHI fields".to_string())
            .with_reference("HIPAA Security Rule §164.312(a)(2)(iv)".to_string())
            .with_requirement(RuleRequirement {
                id: "REQ-003".to_string(),
                description: "PHI column encryption".to_string(),
                validation_type: ValidationType::ColumnEncryption,
                expected_value: Some("true".to_string()),
                operator: ComparisonOperator::Equals,
            }),
        ).ok();

        self.add_rule(
            ComplianceRule::new(
                "HIPAA-002".to_string(),
                "Access Control Implementation".to_string(),
                ComplianceFramework::HIPAA,
                RuleCategory::AccessControl,
                RuleSeverity::Critical,
            )
            .with_description("Implement role-based access controls for PHI".to_string())
            .with_remediation("Configure RBAC policies for all PHI tables".to_string())
            .with_reference("HIPAA Security Rule §164.312(a)(1)".to_string())
            .with_requirement(RuleRequirement {
                id: "REQ-004".to_string(),
                description: "Access control configured".to_string(),
                validation_type: ValidationType::AccessControlConfigured,
                expected_value: Some("true".to_string()),
                operator: ComparisonOperator::Equals,
            }),
        ).ok();

        // SOC2 Rules
        self.add_rule(
            ComplianceRule::new(
                "SOC2-001".to_string(),
                "Encryption in Transit".to_string(),
                ComplianceFramework::SOC2,
                RuleCategory::EncryptionInTransit,
                RuleSeverity::High,
            )
            .with_description("All data transmission must use TLS 1.2 or higher".to_string())
            .with_remediation("Enable TLS 1.2+ for all database connections".to_string())
            .with_reference("SOC2 CC6.7".to_string())
            .with_requirement(RuleRequirement {
                id: "REQ-005".to_string(),
                description: "TLS configured".to_string(),
                validation_type: ValidationType::SslTlsConfigured,
                expected_value: Some("TLS1.2".to_string()),
                operator: ComparisonOperator::GreaterThanOrEqual,
            }),
        ).ok();

        self.add_rule(
            ComplianceRule::new(
                "SOC2-002".to_string(),
                "Multi-Factor Authentication".to_string(),
                ComplianceFramework::SOC2,
                RuleCategory::AccessControl,
                RuleSeverity::High,
            )
            .with_description("MFA required for administrative access".to_string())
            .with_remediation("Enable MFA for all privileged accounts".to_string())
            .with_reference("SOC2 CC6.1".to_string())
            .with_requirement(RuleRequirement {
                id: "REQ-006".to_string(),
                description: "MFA enabled for admins".to_string(),
                validation_type: ValidationType::MfaEnabled,
                expected_value: Some("true".to_string()),
                operator: ComparisonOperator::Equals,
            }),
        ).ok();

        // PCI-DSS Rules
        self.add_rule(
            ComplianceRule::new(
                "PCI-001".to_string(),
                "Cardholder Data Encryption".to_string(),
                ComplianceFramework::PCIDSS,
                RuleCategory::EncryptionAtRest,
                RuleSeverity::Critical,
            )
            .with_description("Cardholder data must be encrypted using strong cryptography".to_string())
            .with_remediation("Enable AES-256 encryption for all cardholder data".to_string())
            .with_reference("PCI-DSS Requirement 3.4".to_string())
            .with_requirement(RuleRequirement {
                id: "REQ-007".to_string(),
                description: "Cardholder data encrypted".to_string(),
                validation_type: ValidationType::ColumnEncryption,
                expected_value: Some("AES-256".to_string()),
                operator: ComparisonOperator::Equals,
            }),
        ).ok();

        self.add_rule(
            ComplianceRule::new(
                "PCI-002".to_string(),
                "Access Logging and Monitoring".to_string(),
                ComplianceFramework::PCIDSS,
                RuleCategory::AuditLogging,
                RuleSeverity::Critical,
            )
            .with_description("Track and monitor all access to cardholder data".to_string())
            .with_remediation("Enable detailed audit logging for cardholder data access".to_string())
            .with_reference("PCI-DSS Requirement 10.2".to_string())
            .with_requirement(RuleRequirement {
                id: "REQ-008".to_string(),
                description: "Comprehensive audit logging".to_string(),
                validation_type: ValidationType::AuditEnabled,
                expected_value: Some("true".to_string()),
                operator: ComparisonOperator::Equals,
            }),
        ).ok();
    }

    /// Add a compliance rule
    pub fn add_rule(&self, rule: ComplianceRule) -> Result<()> {
        let rule_id = rule.id.clone();
        let framework = rule.framework;
        let category = rule.category.clone();

        // Add to main rules map
        self.rules.write()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?
            .insert(rule_id.clone(), rule);

        // Update framework index
        self.framework_index.write()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?
            .entry(framework)
            .or_insert_with(Vec::new)
            .push(rule_id.clone());

        // Update category index
        self.category_index.write()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?
            .entry(category)
            .or_insert_with(Vec::new)
            .push(rule_id);

        Ok(())
    }

    /// Get a rule by ID
    pub fn get_rule(&self, rule_id: &str) -> Result<Option<ComplianceRule>> {
        let rules = self.rules.read()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?;
        Ok(rules.get(rule_id).cloned())
    }

    /// Get all rules for a framework
    pub fn get_rules_by_framework(&self, framework: ComplianceFramework) -> Result<Vec<ComplianceRule>> {
        let framework_index = self.framework_index.read()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?;
        let rules = self.rules.read()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?;

        if let Some(rule_ids) = framework_index.get(&framework) {
            Ok(rule_ids
                .iter()
                .filter_map(|id| rules.get(id).cloned())
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    /// Get all rules for a category
    pub fn get_rules_by_category(&self, category: &RuleCategory) -> Result<Vec<ComplianceRule>> {
        let category_index = self.category_index.read()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?;
        let rules = self.rules.read()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?;

        if let Some(rule_ids) = category_index.get(category) {
            Ok(rule_ids
                .iter()
                .filter_map(|id| rules.get(id).cloned())
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    /// Get all enabled rules
    pub fn get_enabled_rules(&self) -> Result<Vec<ComplianceRule>> {
        let rules = self.rules.read()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?;
        Ok(rules.values().filter(|r| r.enabled).cloned().collect())
    }

    /// Update a rule
    pub fn update_rule(&self, rule: ComplianceRule) -> Result<()> {
        let rule_id = rule.id.clone();
        let mut rules = self.rules.write()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?;

        if rules.contains_key(&rule_id) {
            rules.insert(rule_id, rule);
            Ok(())
        } else {
            Err(DbError::NotFound(format!("Rule not found: {}", rule_id)))
        }
    }

    /// Delete a rule
    pub fn delete_rule(&self, rule_id: &str) -> Result<()> {
        let mut rules = self.rules.write()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?;

        if let Some(rule) = rules.remove(rule_id) {
            // Remove from indexes
            let framework = rule.framework;
            let category = rule.category;

            self.framework_index.write()
                .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?
                .get_mut(&framework)
                .map(|ids| ids.retain(|id| id != rule_id));

            self.category_index.write()
                .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?
                .get_mut(&category)
                .map(|ids| ids.retain(|id| id != rule_id));

            Ok(())
        } else {
            Err(DbError::NotFound(format!("Rule not found: {}", rule_id)))
        }
    }

    /// Register a custom rule validator
    pub fn register_custom_validator(&self, rule_id: String, validator: CustomRuleValidator) -> Result<()> {
        self.custom_rules.write()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?
            .insert(rule_id, validator);
        Ok(())
    }

    /// Get statistics
    pub fn get_statistics(&self) -> ComplianceRulesStatistics {
        let rules = self.rules.read().unwrap();
        let framework_index = self.framework_index.read().unwrap();

        ComplianceRulesStatistics {
            total_rules: rules.len(),
            enabled_rules: rules.values().filter(|r| r.enabled).count(),
            rules_by_framework: framework_index
                .iter()
                .map(|(f, ids)| (*f, ids.len()))
                .collect(),
            rules_by_severity: rules
                .values()
                .fold(HashMap::new(), |mut acc, rule| {
                    *acc.entry(rule.severity).or_insert(0) += 1;
                    acc
                }),
        }
    }
}

impl Default for ComplianceRulesEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for compliance rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRulesStatistics {
    pub total_rules: usize,
    pub enabled_rules: usize,
    pub rules_by_framework: HashMap<ComplianceFramework, usize>,
    pub rules_by_severity: HashMap<RuleSeverity, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_compliance_rule() {
        let rule = ComplianceRule::new(
            "TEST-001".to_string(),
            "Test Rule".to_string(),
            ComplianceFramework::GDPR,
            RuleCategory::EncryptionAtRest,
            RuleSeverity::High,
        );

        assert_eq!(rule.id, "TEST-001");
        assert_eq!(rule.framework, ComplianceFramework::GDPR);
        assert!(rule.enabled);
    }

    #[test]
    fn test_rules_engine() {
        let engine = ComplianceRulesEngine::new();
        let stats = engine.get_statistics();

        assert!(stats.total_rules > 0);
        assert!(stats.enabled_rules > 0);
    }

    #[test]
    fn test_get_rules_by_framework() {
        let engine = ComplianceRulesEngine::new();
        let gdpr_rules = engine.get_rules_by_framework(ComplianceFramework::GDPR).unwrap();

        assert!(!gdpr_rules.is_empty());
        assert!(gdpr_rules.iter().all(|r| r.framework == ComplianceFramework::GDPR));
    }
}
