// # Audit Vault
//
// Comprehensive audit trail system with fine-grained auditing (FGA),
// tamper-evident blockchain-backed logs, and compliance reporting.
//
// ## Features
//
// - **Fine-Grained Auditing**: Track specific column access and modifications
// - **Unified Audit Trail**: Centralized audit repository
// - **Tamper-Evident**: Blockchain-backed integrity verification
// - **Real-Time Alerts**: Immediate notification of security events
// - **Compliance Reports**: SOX, HIPAA, GDPR, PCI-DSS reporting
//
// ## Audit Record Structure
//
// ```text
// ┌─────────────────────────────────────────────┐
// │  Audit Record                               │
// ├─────────────────────────────────────────────┤
// │  - Timestamp                                │
// │  - User ID                                  │
// │  - Session ID                               │
// │  - Action Type                              │
// │  - Object Name                              │
// │  - SQL Statement                            │
// │  - Result (Success/Failure)                 │
// │  - Client IP                                │
// │  - Previous Hash (Blockchain)               │
// │  - Current Hash                             │
// └─────────────────────────────────────────────┘
// ```

use crate::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use parking_lot::RwLock;
use sha2::{Sha256, Digest};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::Arc;

// Audit action types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditAction {
    // SELECT query
    Select,
    // INSERT operation
    Insert,
    // UPDATE operation
    Update,
    // DELETE operation
    Delete,
    // CREATE DDL
    Create,
    // DROP DDL
    Drop,
    // ALTER DDL
    Alter,
    // GRANT privilege
    Grant,
    // REVOKE privilege
    Revoke,
    // Login attempt
    Login,
    // Logout
    Logout,
    // Failed authentication
    AuthFailure,
    // Security policy change
    SecurityChange,
    // Encryption operation
    Encryption,
    // Decryption operation
    Decryption,
    // Key rotation
    KeyRotation,
    // Custom action
    Custom(String),
}

impl AuditAction {
    // Parse action from string
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "SELECT" => Self::Select,
            "INSERT" => Self::Insert,
            "UPDATE" => Self::Update,
            "DELETE" => Self::Delete,
            "CREATE" => Self::Create,
            "DROP" => Self::Drop,
            "ALTER" => Self::Alter,
            "GRANT" => Self::Grant,
            "REVOKE" => Self::Revoke,
            "LOGIN" => Self::Login,
            "LOGOUT" => Self::Logout,
            "AUTH_FAILURE" => Self::AuthFailure,
            "SECURITY_CHANGE" => Self::SecurityChange,
            "ENCRYPTION" => Self::Encryption,
            "DECRYPTION" => Self::Decryption,
            "KEY_ROTATION" => Self::KeyRotation,
            _ => Self::Custom(s.to_string()),
        }
    }

    // Get severity level
    pub fn severity(&self) -> AuditSeverity {
        match self {
            Self::Select => AuditSeverity::Info,
            Self::Insert | Self::Update | Self::Delete => AuditSeverity::Warning,
            Self::Create | Self::Drop | Self::Alter => AuditSeverity::Warning,
            Self::Grant | Self::Revoke => AuditSeverity::Critical,
            Self::Login | Self::Logout => AuditSeverity::Info,
            Self::AuthFailure => AuditSeverity::Critical,
            Self::SecurityChange => AuditSeverity::Critical,
            Self::Encryption | Self::Decryption => AuditSeverity::Info,
            Self::KeyRotation => AuditSeverity::Warning,
            Self::Custom(_) => AuditSeverity::Info,
        }
    }
}

// Audit severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum AuditSeverity {
    Info,
    Warning,
    Critical,
}

// Audit record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    // Record ID
    pub id: u64,
    // Timestamp (Unix timestamp)
    pub timestamp: i64,
    // User ID
    pub user_id: String,
    // Session ID
    pub session_id: String,
    // Client IP address
    pub client_ip: String,
    // Action performed
    pub action: AuditAction,
    // Object name (table, view, etc.)
    pub object_name: Option<String>,
    // SQL statement or description
    pub statement: Option<String>,
    // Result (Success/Failure)
    pub success: bool,
    // Error message if failed
    pub error_message: Option<String>,
    // Affected rows
    pub rows_affected: Option<u64>,
    // Policy that triggered this audit
    pub policy_name: Option<String>,
    // Additional context
    pub context: HashMap<String, String>,
    // Previous record hash (blockchain)
    pub previous_hash: String,
    // Current record hash
    pub hash: String,
    // Severity level
    pub severity: AuditSeverity,
}

impl AuditRecord {
    // Create a new audit record
    pub fn new(
        id: u64,
        userid: String,
        sessionid: String,
        client_ip: String,
        action: AuditAction,
        previoushash: String,
    ) -> Self {
        let severity = action.severity();
        let timestamp = chrono::Utc::now().timestamp();

        let mut record = Self {
            id,
            timestamp,
            user_id: userid,
            session_id: sessionid,
            client_ip,
            action,
            object_name: None,
            statement: None,
            success: true,
            error_message: None,
            rows_affected: None,
            policy_name: None,
            context: HashMap::new(),
            previous_hash: previoushash,
            hash: String::new(),
            severity,
        };

        // Calculate hash
        record.hash = record.calculate_hash();
        record
    }

    // Calculate hash for tamper detection
    fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.id.to_le_bytes());
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(&self.user_id);
        hasher.update(&self.session_id);
        hasher.update(&self.client_ip);
        hasher.update(format!("{:?}", self.action));
        if let Some(ref obj) = self.object_name {
            hasher.update(obj);
        }
        if let Some(ref stmt) = self.statement {
            hasher.update(stmt);
        }
        hasher.update(&[self.success as u8]);
        hasher.update(&self.previous_hash);

        format!("{:x}", hasher.finalize())
    }

    // Verify hash integrity
    pub fn verify_hash(&self) -> bool {
        let calculated = self.calculate_hash();
        calculated == self.hash
    }
}

// Audit policy for fine-grained auditing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPolicy {
    // Policy name
    pub name: String,
    // Enabled flag
    pub enabled: bool,
    // Actions to audit
    pub actions: Vec<AuditAction>,
    // Object name pattern (regex)
    pub object_pattern: Option<String>,
    // User pattern (regex)
    pub user_pattern: Option<String>,
    // Audit only failures
    pub audit_failures_only: bool,
    // Minimum severity to audit
    pub min_severity: AuditSeverity,
    // Created timestamp
    pub created_at: i64,
}

impl AuditPolicy {
    // Create a new audit policy
    pub fn new(name: String, actions: Vec<AuditAction>) -> Self {
        Self {
            name,
            enabled: true,
            actions,
            object_pattern: None,
            user_pattern: None,
            audit_failures_only: false,
            min_severity: AuditSeverity::Info,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    // Check if action should be audited
    pub fn should_audit(&self, action: &AuditAction, user: &str, object: Option<&str>) -> bool {
        if !self.enabled {
            return false;
        }

        // Check action
        if !self.actions.contains(action) && !self.actions.iter().any(|a| {
            matches!(a, AuditAction::Custom(s) if s == "*")
        }) {
            return false;
        }

        // Check user pattern
        if let Some(ref pattern) = self.user_pattern {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if !regex.is_match(user) {
                    return false;
                }
            }
        }

        // Check object pattern
        if let Some(ref pattern) = self.object_pattern {
            if let Some(obj) = object {
                if let Ok(regex) = regex::Regex::new(pattern) {
                    if !regex.is_match(obj) {
                        return false;
                    }
                }
            } else {
                return false;
            }
        }

        // Check severity
        if action.severity() < self.min_severity {
            return false;
        }

        true
    }
}

// Compliance regulation types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplianceRegulation {
    // Sarbanes-Oxley Act
    SOX,
    // Health Insurance Portability and Accountability Act
    HIPAA,
    // General Data Protection Regulation
    GDPR,
    // Payment Card Industry Data Security Standard
    PCIDSS,
    // California Consumer Privacy Act
    CCPA,
    // Custom regulation
    Custom(String),
}

// Compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    // Regulation type
    pub regulation: ComplianceRegulation,
    // Report start date
    pub start_date: i64,
    // Report end date
    pub end_date: i64,
    // Total audit records
    pub total_records: usize,
    // Records by action type
    pub by_action: HashMap<String, usize>,
    // Failed operations
    pub failed_operations: usize,
    // Security events
    pub security_events: usize,
    // Privileged operations
    pub privileged_operations: usize,
    // Findings and violations
    pub findings: Vec<String>,
    // Generated timestamp
    pub generated_at: i64,
}

// Main Audit Vault
pub struct AuditVault {
    // Storage directory
    data_dir: PathBuf,
    // Audit policies
    policies: RwLock<HashMap<String, AuditPolicy>>,
    // In-memory audit buffer (for performance)
    buffer: RwLock<Vec<AuditRecord>>,
    // Record counter
    record_counter: RwLock<u64>,
    // Last record hash (for blockchain)
    last_hash: RwLock<String>,
    // Retention period in days
    retention_days: u32,
    // Alert subscribers
    alert_subscribers: RwLock<Vec<Arc<dyn AlertSubscriber>>>,
    // Statistics
    stats: RwLock<AuditStats>,
}

// Alert subscriber trait
pub trait AlertSubscriber: Send + Sync {
    // Handle an alert
    fn on_alert(&self, record: &AuditRecord);
}

// Audit statistics
#[derive(Debug, Default)]
struct AuditStats {
    total_records: u64,
    by_action: HashMap<String, u64>,
    failed_operations: u64,
    tamper_attempts: u64,
}

impl AuditVault {
    // Create a new audit vault
    pub fn new<P: AsRef<Path>>(data_dir: P, retention_days: u32) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        fs::create_dir_all(&data_dir)
            .map_err(|e| DbError::IoError(format!("Failed to create audit directory: {}", e)))?;

        Ok(Self {
            data_dir,
            policies: RwLock::new(HashMap::new()),
            buffer: RwLock::new(Vec::new()),
            record_counter: RwLock::new(0),
            last_hash: RwLock::new(String::from("0")),
            retention_days,
            alert_subscribers: RwLock::new(Vec::new()),
            stats: RwLock::new(AuditStats::default()),
        })
    }

    // Create an audit policy
    pub fn create_policy(&mut self, policy: AuditPolicy) -> Result<()> {
        self.policies.write().insert(policy.name.clone(), policy);
        Ok(())
    }

    // Drop an audit policy
    pub fn drop_policy(&mut self, name: &str) -> Result<()> {
        self.policies.write().remove(name)
            .ok_or_else(|| DbError::NotFound(format!("Policy not found: {}", name)))?;
        Ok(())
    }

    // Log an audit record
    pub fn log(
        &mut self,
        user_id: &str,
        session_id: &str,
        client_ip: &str,
        action: AuditAction,
        object_name: Option<String>,
        statement: Option<String>,
        success: bool,
    ) -> Result<()> {
        // Check if any policy applies
        let policies = self.policies.read();
        let applicable = policies.values().any(|p| {
            p.should_audit(&action, user_id, object_name.as_deref())
        });

        if !applicable {
            return Ok(());
        }

        // Generate record ID
        let mut counter = self.record_counter.write();
        *counter += 1;
        let record_id = *counter;
        drop(counter);

        // Get previous hash
        let previous_hash = self.last_hash.read().clone();

        // Create audit record
        let mut record = AuditRecord::new(
            record_id,
            user_id.to_string(),
            session_id.to_string(),
            client_ip.to_string(),
            action.clone(),
            previous_hash,
        );

        record.object_name = object_name;
        record.statement = statement;
        record.success = success;

        // Update hash
        record.hash = record.calculate_hash();

        // Update last hash
        *self.last_hash.write() = record.hash.clone();

        // Update statistics
        let mut stats = self.stats.write();
        stats.total_records += 1;
        *stats.by_action.entry(format!("{:?}", action)).or_insert(0) += 1;
        if !success {
            stats.failed_operations += 1;
        }
        drop(stats);

        // Check for alerts
        if record.severity == AuditSeverity::Critical {
            self.send_alert(&record);
        }

        // Add to buffer
        self.buffer.write().push(record);

        // Flush if buffer is large
        if self.buffer.read().len() >= 100 {
            self.flush()?;
        }

        Ok(())
    }

    // Log a security event
    pub fn log_security_event(
        &mut self,
        user_id: &str,
        event_type: &str,
        description: &str,
    ) -> Result<()> {
        self.log(
            user_id,
            "SYSTEM",
            "127.0.0.1",
            AuditAction::SecurityChange,
            Some(event_type.to_string()),
            Some(description.to_string()),
            true,
        )
    }

    // Flush buffer to disk
    pub fn flush(&self) -> Result<()> {
        let mut buffer = self.buffer.write();
        if buffer.is_empty() {
            return Ok(());
        }

        let log_file = self.data_dir.join("audit.log");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .map_err(|e| DbError::IoError(format!("Failed to open audit log: {}", e)))?;

        for record in buffer.iter() {
            let json = serde_json::to_string(record)
                .map_err(|e| DbError::Serialization(format!("Failed to serialize record: {}", e)))?;
            writeln!(file, "{}", json)
                .map_err(|e| DbError::IoError(format!("Failed to write audit log: {}", e)))?;
        }

        buffer.clear();
        Ok(())
    }

    // Query audit records
    pub fn query(
        &self,
        start_date: i64,
        end_date: i64,
        user_filter: Option<&str>,
        action_filter: Option<AuditAction>,
    ) -> Result<Vec<AuditRecord>> {
        // Flush buffer first
        self.flush()?;

        let log_file = self.data_dir.join("audit.log");
        if !log_file.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&log_file)
            .map_err(|e| DbError::IoError(format!("Failed to read audit log: {}", e)))?;

        let mut records = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let record: AuditRecord = serde_json::from_str(line)
                .map_err(|e| DbError::Serialization(format!("Failed to parse record: {}", e)))?;

            // Apply filters
            if record.timestamp < start_date || record.timestamp > end_date {
                continue;
            }

            if let Some(user) = user_filter {
                if record.user_id != user {
                    continue;
                }
            }

            if let Some(ref action) = action_filter {
                if &record.action != action {
                    continue;
                }
            }

            records.push(record);
        }

        Ok(records)
    }

    // Verify audit trail integrity
    pub fn verify_integrity(&self) -> Result<bool> {
        self.flush()?;

        let log_file = self.data_dir.join("audit.log");
        if !log_file.exists() {
            return Ok(true);
        }

        let content = fs::read_to_string(&log_file)
            .map_err(|e| DbError::IoError(format!("Failed to read audit log: {}", e)))?;

        let mut previous_hash = String::from("0");

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let record: AuditRecord = serde_json::from_str(line)
                .map_err(|e| DbError::Serialization(format!("Failed to parse record: {}", e)))?;

            // Verify hash
            if !record.verify_hash() {
                return Ok(false);
            }

            // Verify blockchain link
            if record.previous_hash != previous_hash {
                return Ok(false);
            }

            previous_hash = record.hash.clone();
        }

        Ok(true)
    }

    // Generate compliance report
    pub fn generate_compliance_report(
        &self,
        regulation: &str,
        start_date: i64,
        end_date: i64,
    ) -> Result<ComplianceReport> {
        let regulation_type = match regulation.to_uppercase().as_str() {
            "SOX" => ComplianceRegulation::SOX,
            "HIPAA" => ComplianceRegulation::HIPAA,
            "GDPR" => ComplianceRegulation::GDPR,
            "PCI-DSS" | "PCIDSS" => ComplianceRegulation::PCIDSS,
            "CCPA" => ComplianceRegulation::CCPA,
            _ => ComplianceRegulation::Custom(regulation.to_string()),
        };

        let records = self.query(start_date, end_date, None, None)?;

        let mut by_action: HashMap<String, usize> = HashMap::new();
        let mut failed_operations = 0;
        let mut security_events = 0;
        let mut privileged_operations = 0;
        let mut findings = Vec::new();

        for record in &records {
            *by_action.entry(format!("{:?}", record.action)).or_insert(0) += 1;

            if !record.success {
                failed_operations += 1;
            }

            if record.severity == AuditSeverity::Critical {
                security_events += 1;
            }

            if matches!(record.action, AuditAction::Grant | AuditAction::Revoke) {
                privileged_operations += 1;
            }
        }

        // Generate regulation-specific findings
        match regulation_type {
            ComplianceRegulation::SOX => {
                if privileged_operations > 0 {
                    findings.push(format!("{} privileged operations detected - review for SOX compliance", privileged_operations));
                }
            }
            ComplianceRegulation::HIPAA => {
                if security_events > 10 {
                    findings.push(format!("{} security events - may indicate HIPAA breach risk", security_events));
                }
            }
            ComplianceRegulation::GDPR => {
                let deletes = by_action.get("Delete").unwrap_or(&0);
                findings.push(format!("{} delete operations - verify right to be forgotten compliance", deletes));
            }
            _ => {}
        }

        Ok(ComplianceReport {
            regulation: regulation_type,
            start_date,
            end_date,
            total_records: records.len(),
            by_action,
            failed_operations,
            security_events,
            privileged_operations,
            findings,
            generated_at: chrono::Utc::now().timestamp(),
        })
    }

    // Subscribe to audit alerts
    pub fn subscribe_alerts(&self, subscriber: Arc<dyn AlertSubscriber>) {
        self.alert_subscribers.write().push(subscriber);
    }

    // Send alert to subscribers
    fn send_alert(&self, record: &AuditRecord) {
        let subscribers = self.alert_subscribers.read();
        for subscriber in subscribers.iter() {
            subscriber.on_alert(record);
        }
    }

    // Get audit statistics
    pub fn get_stats(&self) -> (u64, u64, HashMap<String, u64>) {
        let stats = self.stats.read();
        (stats.total_records, stats.failed_operations, stats.by_action.clone())
    }

    // Purge old audit records based on retention policy
    pub fn purge_old_records(&self) -> Result<usize> {
        self.flush()?;

        let cutoff = chrono::Utc::now().timestamp() - (self.retention_days as i64 * 86400);
        let log_file = self.data_dir.join("audit.log");

        if !log_file.exists() {
            return Ok(0);
        }

        let content = fs::read_to_string(&log_file)
            .map_err(|e| DbError::IoError(format!("Failed to read audit log: {}", e)))?;

        let mut kept_records = Vec::new();
        let mut purged_count = 0;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let record: AuditRecord = serde_json::from_str(line)
                .map_err(|e| DbError::Serialization(format!("Failed to parse record: {}", e)))?;

            if record.timestamp >= cutoff {
                kept_records.push(line.to_string());
            } else {
                purged_count += 1;
            }
        }

        // Write back kept records
        fs::write(&log_file, kept_records.join("\n"))
            .map_err(|e| DbError::IoError(format!("Failed to write audit log: {}", e)))?;

        Ok(purged_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_record_hash() {
        let record = AuditRecord::new(
            1,
            "user1".to_string(),
            "session1".to_string(),
            "127.0.0.1".to_string(),
            AuditAction::Select,
            "0".to_string(),
        );

        assert!(record.verify_hash());
    }

    #[test]
    fn test_audit_policy() {
        let policy = AuditPolicy::new(
            "test_policy".to_string(),
            vec![AuditAction::Select, AuditAction::Update],
        );

        assert!(policy.should_audit(&AuditAction::Select, "user1", None));
        assert!(!policy.should_audit(&AuditAction::Delete, "user1", None));
    }

    #[test]
    fn test_audit_logging() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut vault = AuditVault::new(temp_dir.path(), 365).unwrap();

        // Create policy
        let policy = AuditPolicy::new(
            "select_policy".to_string(),
            vec![AuditAction::Select],
        );
        vault.create_policy(policy).unwrap();

        // Log record
        vault.log(
            "user1",
            "session1",
            "127.0.0.1",
            AuditAction::Select,
            Some("table1".to_string()),
            Some("SELECT * FROM table1".to_string()),
            true,
        ).unwrap();

        vault.flush().unwrap();

        let (total, failed, _) = vault.get_stats();
        assert_eq!(total, 1);
        assert_eq!(failed, 0);
    }

    #[test]
    fn test_integrity_verification() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut vault = AuditVault::new(temp_dir.path(), 365).unwrap();

        let policy = AuditPolicy::new(
            "test_policy".to_string(),
            vec![AuditAction::Select, AuditAction::Insert],
        );
        vault.create_policy(policy).unwrap();

        // Log multiple records
        for i in 0..5 {
            vault.log(
                "user1",
                "session1",
                "127.0.0.1",
                AuditAction::Select,
                Some(format!("table{}", i)),
                None,
                true,
            ).unwrap();
        }

        vault.flush().unwrap();

        // Verify integrity
        assert!(vault.verify_integrity().unwrap());
    }
}
