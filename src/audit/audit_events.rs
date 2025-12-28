// # Audit Event Types
//
// Defines comprehensive audit event types for SOC2/HIPAA compliance.
// Tracks all security-relevant database operations including DDL, DML, DCL,
// authentication, and configuration changes.

use crate::common::{SessionId, TableId, TransactionId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// Event Categories
// ============================================================================

/// Audit event severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditSeverity {
    /// Informational events (successful operations)
    Info,

    /// Warning events (suspicious but not necessarily malicious)
    Warning,

    /// Error events (failed operations)
    Error,

    /// Critical security events (potential security breaches)
    Critical,
}

/// Audit event categories for compliance classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventCategory {
    /// Data Definition Language (CREATE, ALTER, DROP)
    DDL,

    /// Data Manipulation Language (INSERT, UPDATE, DELETE)
    DML,

    /// Data Control Language (GRANT, REVOKE)
    DCL,

    /// Authentication and session management
    Authentication,

    /// Authorization and access control
    Authorization,

    /// System configuration changes
    Configuration,

    /// Backup and recovery operations
    Backup,

    /// Data access (SELECT queries)
    DataAccess,

    /// Administrative operations
    Administrative,

    /// Security events
    Security,
}

/// Action outcome for compliance tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionOutcome {
    /// Operation completed successfully
    Success,

    /// Operation failed
    Failure,

    /// Operation was denied due to permissions
    Denied,
}

// ============================================================================
// Core Audit Event Structure
// ============================================================================

/// Unique identifier for audit events
pub type AuditEventId = u64;

/// Core audit event with all required compliance fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event identifier
    pub event_id: AuditEventId,

    /// Event timestamp (microseconds since UNIX epoch)
    pub timestamp: i64,

    /// Event category for filtering
    pub category: EventCategory,

    /// Event severity
    pub severity: AuditSeverity,

    /// Action outcome
    pub outcome: ActionOutcome,

    /// Detailed event data
    pub event_type: AuditEventType,

    /// Session identifier (if applicable)
    pub session_id: Option<SessionId>,

    /// Transaction identifier (if applicable)
    pub transaction_id: Option<TransactionId>,

    /// User who performed the action
    pub username: String,

    /// Source IP address
    pub source_ip: Option<IpAddr>,

    /// Database name
    pub database_name: String,

    /// SQL statement (for SQL operations)
    pub sql_statement: Option<String>,

    /// Additional context (key-value pairs)
    pub context: HashMap<String, String>,

    /// Tamper-evident checksum (SHA-256)
    pub checksum: Option<String>,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(
        event_id: AuditEventId,
        category: EventCategory,
        severity: AuditSeverity,
        outcome: ActionOutcome,
        event_type: AuditEventType,
        username: String,
        database_name: String,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as i64;

        Self {
            event_id,
            timestamp,
            category,
            severity,
            outcome,
            event_type,
            session_id: None,
            transaction_id: None,
            username,
            source_ip: None,
            database_name,
            sql_statement: None,
            context: HashMap::new(),
            checksum: None,
        }
    }

    /// Add session information
    pub fn with_session(mut self, session_id: SessionId) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Add transaction information
    pub fn with_transaction(mut self, txn_id: TransactionId) -> Self {
        self.transaction_id = Some(txn_id);
        self
    }

    /// Add source IP address
    pub fn with_source_ip(mut self, ip: IpAddr) -> Self {
        self.source_ip = Some(ip);
        self
    }

    /// Add SQL statement
    pub fn with_sql(mut self, sql: String) -> Self {
        self.sql_statement = Some(sql);
        self
    }

    /// Add context information
    pub fn add_context(mut self, key: String, value: String) -> Self {
        self.context.insert(key, value);
        self
    }

    /// Compute and set tamper-evident checksum
    pub fn compute_checksum(&mut self) {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();

        // Hash all immutable fields
        hasher.update(self.event_id.to_le_bytes());
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(format!("{:?}", self.category).as_bytes());
        hasher.update(format!("{:?}", self.severity).as_bytes());
        hasher.update(format!("{:?}", self.outcome).as_bytes());
        hasher.update(self.username.as_bytes());
        hasher.update(self.database_name.as_bytes());

        if let Some(sql) = &self.sql_statement {
            hasher.update(sql.as_bytes());
        }

        let result = hasher.finalize();
        self.checksum = Some(format!("{:x}", result));
    }

    /// Verify checksum integrity
    pub fn verify_checksum(&self) -> bool {
        if let Some(stored_checksum) = &self.checksum {
            let mut temp_event = self.clone();
            temp_event.checksum = None;
            temp_event.compute_checksum();
            temp_event.checksum.as_ref() == Some(stored_checksum)
        } else {
            false
        }
    }
}

// ============================================================================
// Detailed Event Types
// ============================================================================

/// Specific audit event types with detailed information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    // Authentication Events
    LoginSuccess {
        authentication_method: String,
    },
    LoginFailure {
        reason: String,
        attempts: u32,
    },
    Logout,
    SessionTimeout,

    // DDL Events
    CreateTable {
        table_name: String,
        column_count: usize,
    },
    AlterTable {
        table_name: String,
        alteration_type: String,
    },
    DropTable {
        table_name: String,
    },
    CreateIndex {
        index_name: String,
        table_name: String,
        index_type: String,
    },
    DropIndex {
        index_name: String,
    },
    CreateDatabase {
        database_name: String,
    },
    DropDatabase {
        database_name: String,
    },
    CreateUser {
        new_username: String,
    },
    AlterUser {
        target_username: String,
        change_type: String,
    },
    DropUser {
        target_username: String,
    },

    // DML Events
    Insert {
        table_id: TableId,
        table_name: String,
        row_count: usize,
    },
    Update {
        table_id: TableId,
        table_name: String,
        row_count: usize,
        updated_columns: Vec<String>,
    },
    Delete {
        table_id: TableId,
        table_name: String,
        row_count: usize,
    },
    Truncate {
        table_id: TableId,
        table_name: String,
    },

    // DCL Events
    Grant {
        privilege: String,
        object_type: String,
        object_name: String,
        grantee: String,
    },
    Revoke {
        privilege: String,
        object_type: String,
        object_name: String,
        revokee: String,
    },

    // Data Access Events (SELECT)
    SelectQuery {
        tables_accessed: Vec<String>,
        row_count: usize,
        sensitive_columns: Vec<String>,
    },

    // Configuration Events
    ConfigurationChange {
        parameter: String,
        old_value: String,
        new_value: String,
    },
    SecurityPolicyChange {
        policy_name: String,
        change_description: String,
    },

    // Backup/Recovery Events
    BackupStarted {
        backup_type: String,
        target_location: String,
    },
    BackupCompleted {
        backup_type: String,
        size_bytes: u64,
        duration_ms: u64,
    },
    BackupFailed {
        backup_type: String,
        error_message: String,
    },
    RestoreStarted {
        restore_point: String,
    },
    RestoreCompleted {
        restore_point: String,
        duration_ms: u64,
    },
    RestoreFailed {
        restore_point: String,
        error_message: String,
    },

    // Security Events
    UnauthorizedAccess {
        attempted_resource: String,
        required_privilege: String,
    },
    PermissionDenied {
        resource: String,
        operation: String,
    },
    SuspiciousActivity {
        activity_type: String,
        risk_score: f64,
    },
    DataMaskingApplied {
        table_name: String,
        column_name: String,
        masking_policy: String,
    },
    EncryptionKeyRotation {
        key_id: String,
    },

    // Administrative Events
    DatabaseStartup,
    DatabaseShutdown,
    CheckpointStarted,
    CheckpointCompleted {
        duration_ms: u64,
    },
    VacuumStarted {
        table_name: String,
    },
    VacuumCompleted {
        table_name: String,
        pages_removed: u64,
    },

    // Generic event for extensibility
    Custom {
        event_name: String,
        details: HashMap<String, String>,
    },
}

// ============================================================================
// Helper Functions
// ============================================================================

impl AuditEventType {
    /// Get human-readable description of the event
    pub fn description(&self) -> String {
        match self {
            AuditEventType::LoginSuccess { authentication_method } => {
                format!("User logged in via {}", authentication_method)
            }
            AuditEventType::LoginFailure { reason, attempts } => {
                format!("Login failed: {} (attempt {})", reason, attempts)
            }
            AuditEventType::Logout => "User logged out".to_string(),
            AuditEventType::SessionTimeout => "Session timed out".to_string(),

            AuditEventType::CreateTable { table_name, column_count } => {
                format!("Created table '{}' with {} columns", table_name, column_count)
            }
            AuditEventType::DropTable { table_name } => {
                format!("Dropped table '{}'", table_name)
            }
            AuditEventType::AlterTable { table_name, alteration_type } => {
                format!("Altered table '{}': {}", table_name, alteration_type)
            }

            AuditEventType::Insert { table_name, row_count, .. } => {
                format!("Inserted {} row(s) into '{}'", row_count, table_name)
            }
            AuditEventType::Update { table_name, row_count, .. } => {
                format!("Updated {} row(s) in '{}'", row_count, table_name)
            }
            AuditEventType::Delete { table_name, row_count, .. } => {
                format!("Deleted {} row(s) from '{}'", row_count, table_name)
            }

            AuditEventType::Grant { privilege, object_name, grantee, .. } => {
                format!("Granted {} on '{}' to '{}'", privilege, object_name, grantee)
            }
            AuditEventType::Revoke { privilege, object_name, revokee, .. } => {
                format!("Revoked {} on '{}' from '{}'", privilege, object_name, revokee)
            }

            AuditEventType::SelectQuery { tables_accessed, row_count, .. } => {
                format!("Queried {} table(s), returned {} row(s)",
                    tables_accessed.len(), row_count)
            }

            AuditEventType::UnauthorizedAccess { attempted_resource, .. } => {
                format!("Unauthorized access attempt to '{}'", attempted_resource)
            }

            _ => format!("{:?}", self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(
            1,
            EventCategory::Authentication,
            AuditSeverity::Info,
            ActionOutcome::Success,
            AuditEventType::LoginSuccess {
                authentication_method: "password".to_string(),
            },
            "admin".to_string(),
            "test_db".to_string(),
        );

        assert_eq!(event.event_id, 1);
        assert_eq!(event.username, "admin");
        assert_eq!(event.category, EventCategory::Authentication);
    }

    #[test]
    fn test_checksum_computation() {
        let mut event = AuditEvent::new(
            1,
            EventCategory::DDL,
            AuditSeverity::Info,
            ActionOutcome::Success,
            AuditEventType::CreateTable {
                table_name: "users".to_string(),
                column_count: 5,
            },
            "admin".to_string(),
            "test_db".to_string(),
        );

        event.compute_checksum();
        assert!(event.checksum.is_some());
        assert!(event.verify_checksum());
    }

    #[test]
    fn test_event_builder() {
        let event = AuditEvent::new(
            1,
            EventCategory::DML,
            AuditSeverity::Info,
            ActionOutcome::Success,
            AuditEventType::Insert {
                table_id: 100,
                table_name: "users".to_string(),
                row_count: 1,
            },
            "user1".to_string(),
            "app_db".to_string(),
        )
        .with_session(12345)
        .with_transaction(67890)
        .add_context("client_app".to_string(), "web_ui".to_string());

        assert_eq!(event.session_id, Some(12345));
        assert_eq!(event.transaction_id, Some(67890));
        assert!(event.context.contains_key("client_app"));
    }
}
