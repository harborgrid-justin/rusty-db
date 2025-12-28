// # Compliance Scanner
//
// Automated compliance scanning engine that validates database configurations, schemas,
// and data against compliance rules. Supports schema scanning, data classification,
// access control auditing, and encryption verification.

use crate::error::{DbError, Result};
use crate::common::*;
use super::compliance_rules::{
    ComplianceFramework, ComplianceRule, RuleSeverity, ComparisonOperator,
    ValidationType, ComplianceContext,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

// ============================================================================
// Scan Types and Configuration
// ============================================================================

/// Type of compliance scan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanType {
    /// Full compliance scan across all frameworks
    Full,
    /// Schema-only scan
    SchemaOnly,
    /// Data classification scan
    DataClassification,
    /// Access control audit
    AccessControl,
    /// Encryption verification
    Encryption,
    /// Framework-specific scan
    Framework(ComplianceFramework),
}

/// Scan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Type of scan to perform
    pub scan_type: ScanType,
    /// Specific frameworks to scan (None = all)
    pub frameworks: Option<Vec<ComplianceFramework>>,
    /// Database scope
    pub databases: Option<Vec<String>>,
    /// Schema scope
    pub schemas: Option<Vec<String>>,
    /// Table scope
    pub tables: Option<Vec<String>>,
    /// Include disabled rules
    pub include_disabled: bool,
    /// Minimum severity to report
    pub min_severity: RuleSeverity,
    /// Maximum concurrent scans
    pub max_concurrent: usize,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            scan_type: ScanType::Full,
            frameworks: None,
            databases: None,
            schemas: None,
            tables: None,
            include_disabled: false,
            min_severity: RuleSeverity::Info,
            max_concurrent: 10,
        }
    }
}

// ============================================================================
// Scan Result Types
// ============================================================================

/// Result of a compliance scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// Scan ID
    pub scan_id: String,
    /// Scan configuration
    pub config: ScanConfig,
    /// Scan status
    pub status: ScanStatus,
    /// Start time
    pub started_at: SystemTime,
    /// End time
    pub completed_at: Option<SystemTime>,
    /// Findings
    pub findings: Vec<ComplianceFinding>,
    /// Total rules checked
    pub rules_checked: usize,
    /// Total violations found
    pub violations_found: usize,
    /// Scan metadata
    pub metadata: HashMap<String, String>,
}

/// Status of a compliance scan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanStatus {
    /// Scan is pending
    Pending,
    /// Scan is running
    Running,
    /// Scan completed successfully
    Completed,
    /// Scan failed
    Failed,
    /// Scan was cancelled
    Cancelled,
}

/// Compliance finding (violation or pass)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFinding {
    /// Finding ID
    pub id: String,
    /// Rule that was checked
    pub rule_id: String,
    /// Rule name
    pub rule_name: String,
    /// Framework
    pub framework: ComplianceFramework,
    /// Severity
    pub severity: RuleSeverity,
    /// Finding status
    pub status: FindingStatus,
    /// Affected resource
    pub resource: ResourceIdentifier,
    /// Finding details
    pub details: String,
    /// Remediation recommendation
    pub remediation: String,
    /// Timestamp
    pub timestamp: SystemTime,
}

/// Status of a compliance finding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingStatus {
    /// Rule passed
    Pass,
    /// Rule failed (violation)
    Fail,
    /// Rule check skipped
    Skipped,
    /// Error during check
    Error,
}

/// Resource identifier for findings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceIdentifier {
    /// Resource type
    pub resource_type: ResourceType,
    /// Database name
    pub database: String,
    /// Schema name
    pub schema: Option<String>,
    /// Table name
    pub table: Option<String>,
    /// Column name
    pub column: Option<String>,
}

/// Type of database resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceType {
    Database,
    Schema,
    Table,
    Column,
    Index,
    User,
    Role,
    Policy,
}

// ============================================================================
// Compliance Scanner
// ============================================================================

/// Compliance scanner engine
pub struct ComplianceScanner {
    /// Scan results indexed by scan ID
    scans: Arc<RwLock<HashMap<String, ScanResult>>>,
    /// Active scans
    active_scans: Arc<RwLock<HashSet<String>>>,
    /// Schema metadata cache
    schema_cache: Arc<RwLock<HashMap<String, SchemaMetadata>>>,
    /// Encryption status cache
    encryption_cache: Arc<RwLock<HashMap<String, EncryptionStatus>>>,
    /// Access control cache
    access_control_cache: Arc<RwLock<HashMap<String, AccessControlConfig>>>,
}

/// Schema metadata for compliance checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    pub database: String,
    pub schema: String,
    pub tables: Vec<TableMetadata>,
    pub last_updated: SystemTime,
}

/// Table metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMetadata {
    pub name: String,
    pub columns: Vec<ColumnMetadata>,
    pub row_count: u64,
    pub size_bytes: u64,
    pub has_pii: bool,
    pub has_phi: bool,
    pub has_pci: bool,
}

/// Column metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMetadata {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_encrypted: bool,
    pub encryption_algorithm: Option<String>,
    pub data_classification: DataClassification,
}

/// Data classification level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataClassification {
    /// Public data
    Public,
    /// Internal use only
    Internal,
    /// Confidential data
    Confidential,
    /// Restricted/sensitive data
    Restricted,
    /// Personally Identifiable Information
    PII,
    /// Protected Health Information
    PHI,
    /// Payment Card Information
    PCI,
}

/// Encryption status for a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionStatus {
    pub resource: String,
    pub encrypted_at_rest: bool,
    pub encrypted_in_transit: bool,
    pub encryption_algorithm: Option<String>,
    pub key_rotation_enabled: bool,
    pub last_key_rotation: Option<SystemTime>,
}

/// Access control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlConfig {
    pub resource: String,
    pub has_rbac: bool,
    pub has_row_level_security: bool,
    pub has_column_masking: bool,
    pub audit_enabled: bool,
    pub mfa_required: bool,
}

impl ComplianceScanner {
    /// Create a new compliance scanner
    pub fn new() -> Self {
        Self {
            scans: Arc::new(RwLock::new(HashMap::new())),
            active_scans: Arc::new(RwLock::new(HashSet::new())),
            schema_cache: Arc::new(RwLock::new(HashMap::new())),
            encryption_cache: Arc::new(RwLock::new(HashMap::new())),
            access_control_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start a new compliance scan
    pub fn start_scan(&self, config: ScanConfig, rules: Vec<ComplianceRule>) -> Result<String> {
        let scan_id = format!("SCAN-{}", self.generate_scan_id());

        let scan_result = ScanResult {
            scan_id: scan_id.clone(),
            config,
            status: ScanStatus::Running,
            started_at: SystemTime::now(),
            completed_at: None,
            findings: Vec::new(),
            rules_checked: 0,
            violations_found: 0,
            metadata: HashMap::new(),
        };

        // Register scan
        self.scans.write()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?
            .insert(scan_id.clone(), scan_result);

        self.active_scans.write()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?
            .insert(scan_id.clone());

        Ok(scan_id)
    }

    /// Perform schema compliance scan
    pub fn scan_schema(&self, database: &str, schema: &str, rules: &[ComplianceRule]) -> Result<Vec<ComplianceFinding>> {
        let mut findings = Vec::new();

        // Get or refresh schema metadata
        let metadata = self.get_schema_metadata(database, schema)?;

        for rule in rules {
            for table in &metadata.tables {
                for requirement in &rule.requirements {
                    match &requirement.validation_type {
                        ValidationType::TableEncryption => {
                            let finding = self.check_table_encryption(
                                rule,
                                database,
                                schema,
                                &table.name,
                            )?;
                            findings.push(finding);
                        }
                        ValidationType::ColumnEncryption => {
                            for column in &table.columns {
                                let finding = self.check_column_encryption(
                                    rule,
                                    database,
                                    schema,
                                    &table.name,
                                    column,
                                )?;
                                findings.push(finding);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(findings)
    }

    /// Perform data classification scan
    pub fn scan_data_classification(&self, database: &str) -> Result<Vec<ComplianceFinding>> {
        let mut findings = Vec::new();

        // Scan for PII, PHI, PCI data without proper classification
        let metadata = self.get_database_metadata(database)?;

        for schema in metadata {
            for table in &schema.tables {
                // Check if table contains sensitive data but lacks proper controls
                if (table.has_pii || table.has_phi || table.has_pci) && !self.has_encryption(&table.name)? {
                    findings.push(ComplianceFinding {
                        id: format!("FIND-{}", self.generate_finding_id()),
                        rule_id: "DATA-CLASS-001".to_string(),
                        rule_name: "Sensitive Data Protection".to_string(),
                        framework: ComplianceFramework::GDPR,
                        severity: RuleSeverity::Critical,
                        status: FindingStatus::Fail,
                        resource: ResourceIdentifier {
                            resource_type: ResourceType::Table,
                            database: database.to_string(),
                            schema: Some(schema.schema.clone()),
                            table: Some(table.name.clone()),
                            column: None,
                        },
                        details: format!("Table '{}' contains sensitive data but is not encrypted", table.name),
                        remediation: "Enable encryption for tables containing sensitive data".to_string(),
                        timestamp: SystemTime::now(),
                    });
                }
            }
        }

        Ok(findings)
    }

    /// Perform access control audit
    pub fn audit_access_control(&self, database: &str) -> Result<Vec<ComplianceFinding>> {
        let mut findings = Vec::new();

        // Check access control configurations
        let access_configs = self.get_access_control_configs(database)?;

        for (resource, config) in access_configs {
            if !config.has_rbac {
                findings.push(ComplianceFinding {
                    id: format!("FIND-{}", self.generate_finding_id()),
                    rule_id: "ACCESS-001".to_string(),
                    rule_name: "RBAC Not Configured".to_string(),
                    framework: ComplianceFramework::SOC2,
                    severity: RuleSeverity::High,
                    status: FindingStatus::Fail,
                    resource: ResourceIdentifier {
                        resource_type: ResourceType::Table,
                        database: database.to_string(),
                        schema: None,
                        table: Some(resource.clone()),
                        column: None,
                    },
                    details: format!("Resource '{}' does not have RBAC configured", resource),
                    remediation: "Configure role-based access control for this resource".to_string(),
                    timestamp: SystemTime::now(),
                });
            }

            if !config.audit_enabled {
                findings.push(ComplianceFinding {
                    id: format!("FIND-{}", self.generate_finding_id()),
                    rule_id: "AUDIT-001".to_string(),
                    rule_name: "Audit Logging Disabled".to_string(),
                    framework: ComplianceFramework::HIPAA,
                    severity: RuleSeverity::Critical,
                    status: FindingStatus::Fail,
                    resource: ResourceIdentifier {
                        resource_type: ResourceType::Table,
                        database: database.to_string(),
                        schema: None,
                        table: Some(resource.clone()),
                        column: None,
                    },
                    details: format!("Audit logging is not enabled for '{}'", resource),
                    remediation: "Enable comprehensive audit logging".to_string(),
                    timestamp: SystemTime::now(),
                });
            }
        }

        Ok(findings)
    }

    /// Verify encryption compliance
    pub fn verify_encryption(&self, database: &str) -> Result<Vec<ComplianceFinding>> {
        let mut findings = Vec::new();

        let encryption_statuses = self.get_encryption_statuses(database)?;

        for (resource, status) in encryption_statuses {
            if !status.encrypted_at_rest {
                findings.push(ComplianceFinding {
                    id: format!("FIND-{}", self.generate_finding_id()),
                    rule_id: "ENCRYPT-001".to_string(),
                    rule_name: "Encryption at Rest Not Enabled".to_string(),
                    framework: ComplianceFramework::PCIDSS,
                    severity: RuleSeverity::Critical,
                    status: FindingStatus::Fail,
                    resource: ResourceIdentifier {
                        resource_type: ResourceType::Table,
                        database: database.to_string(),
                        schema: None,
                        table: Some(resource.clone()),
                        column: None,
                    },
                    details: format!("Resource '{}' is not encrypted at rest", resource),
                    remediation: "Enable transparent data encryption (TDE)".to_string(),
                    timestamp: SystemTime::now(),
                });
            }

            if !status.key_rotation_enabled {
                findings.push(ComplianceFinding {
                    id: format!("FIND-{}", self.generate_finding_id()),
                    rule_id: "ENCRYPT-002".to_string(),
                    rule_name: "Key Rotation Not Enabled".to_string(),
                    framework: ComplianceFramework::SOC2,
                    severity: RuleSeverity::Medium,
                    status: FindingStatus::Fail,
                    resource: ResourceIdentifier {
                        resource_type: ResourceType::Table,
                        database: database.to_string(),
                        schema: None,
                        table: Some(resource.clone()),
                        column: None,
                    },
                    details: "Encryption key rotation is not enabled".to_string(),
                    remediation: "Enable automatic key rotation".to_string(),
                    timestamp: SystemTime::now(),
                });
            }
        }

        Ok(findings)
    }

    /// Complete a scan
    pub fn complete_scan(&self, scan_id: &str, findings: Vec<ComplianceFinding>) -> Result<()> {
        let mut scans = self.scans.write()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?;

        if let Some(scan) = scans.get_mut(scan_id) {
            scan.status = ScanStatus::Completed;
            scan.completed_at = Some(SystemTime::now());
            scan.violations_found = findings.iter().filter(|f| f.status == FindingStatus::Fail).count();
            scan.findings = findings;

            self.active_scans.write()
                .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?
                .remove(scan_id);

            Ok(())
        } else {
            Err(DbError::NotFound(format!("Scan not found: {}", scan_id)))
        }
    }

    /// Get scan result
    pub fn get_scan_result(&self, scan_id: &str) -> Result<ScanResult> {
        let scans = self.scans.read()
            .map_err(|e| DbError::Internal(format!("Failed to acquire lock: {}", e)))?;

        scans.get(scan_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("Scan not found: {}", scan_id)))
    }

    // Helper methods

    fn check_table_encryption(&self, rule: &ComplianceRule, database: &str, schema: &str, table: &str) -> Result<ComplianceFinding> {
        let is_encrypted = self.has_table_encryption(table)?;

        Ok(ComplianceFinding {
            id: format!("FIND-{}", self.generate_finding_id()),
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            framework: rule.framework,
            severity: rule.severity,
            status: if is_encrypted { FindingStatus::Pass } else { FindingStatus::Fail },
            resource: ResourceIdentifier {
                resource_type: ResourceType::Table,
                database: database.to_string(),
                schema: Some(schema.to_string()),
                table: Some(table.to_string()),
                column: None,
            },
            details: if is_encrypted {
                "Table encryption is enabled".to_string()
            } else {
                "Table encryption is not enabled".to_string()
            },
            remediation: rule.remediation.clone(),
            timestamp: SystemTime::now(),
        })
    }

    fn check_column_encryption(&self, rule: &ComplianceRule, database: &str, schema: &str, table: &str, column: &ColumnMetadata) -> Result<ComplianceFinding> {
        let requires_encryption = matches!(
            column.data_classification,
            DataClassification::PII | DataClassification::PHI | DataClassification::PCI | DataClassification::Restricted
        );

        let status = if requires_encryption && !column.is_encrypted {
            FindingStatus::Fail
        } else {
            FindingStatus::Pass
        };

        Ok(ComplianceFinding {
            id: format!("FIND-{}", self.generate_finding_id()),
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            framework: rule.framework,
            severity: rule.severity,
            status,
            resource: ResourceIdentifier {
                resource_type: ResourceType::Column,
                database: database.to_string(),
                schema: Some(schema.to_string()),
                table: Some(table.to_string()),
                column: Some(column.name.clone()),
            },
            details: if status == FindingStatus::Pass {
                format!("Column '{}' has appropriate encryption", column.name)
            } else {
                format!("Column '{}' contains sensitive data but is not encrypted", column.name)
            },
            remediation: rule.remediation.clone(),
            timestamp: SystemTime::now(),
        })
    }

    fn get_schema_metadata(&self, _database: &str, _schema: &str) -> Result<SchemaMetadata> {
        // Placeholder - would query actual schema metadata
        Ok(SchemaMetadata {
            database: _database.to_string(),
            schema: _schema.to_string(),
            tables: Vec::new(),
            last_updated: SystemTime::now(),
        })
    }

    fn get_database_metadata(&self, _database: &str) -> Result<Vec<SchemaMetadata>> {
        // Placeholder - would query actual database metadata
        Ok(Vec::new())
    }

    fn has_encryption(&self, _table: &str) -> Result<bool> {
        // Placeholder - would check actual encryption status
        Ok(false)
    }

    fn has_table_encryption(&self, _table: &str) -> Result<bool> {
        // Placeholder - would check actual table encryption
        Ok(false)
    }

    fn get_access_control_configs(&self, _database: &str) -> Result<HashMap<String, AccessControlConfig>> {
        // Placeholder - would query actual access control configurations
        Ok(HashMap::new())
    }

    fn get_encryption_statuses(&self, _database: &str) -> Result<HashMap<String, EncryptionStatus>> {
        // Placeholder - would query actual encryption statuses
        Ok(HashMap::new())
    }

    fn generate_scan_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("{}", timestamp)
    }

    fn generate_finding_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros();
        format!("{}", timestamp)
    }
}

impl Default for ComplianceScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_scanner() {
        let scanner = ComplianceScanner::new();
        assert!(scanner.scans.read().unwrap().is_empty());
    }

    #[test]
    fn test_scan_config_default() {
        let config = ScanConfig::default();
        assert_eq!(config.scan_type, ScanType::Full);
        assert!(!config.include_disabled);
    }
}
