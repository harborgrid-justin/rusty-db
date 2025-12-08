//! # Security Module
//!
//! Comprehensive enterprise-grade security system for RustyDB providing authentication,
//! authorization, encryption, auditing, and mandatory access control.
//!
//! ## Architecture
//!
//! The security module is organized into seven core submodules:
//!
//! ### 1. Role-Based Access Control (RBAC)
//! Hierarchical role definitions with inheritance, dynamic activation, and separation
//! of duties constraints. See [`rbac`] module for details.
//!
//! ### 2. Fine-Grained Access Control (FGAC)
//! Row-level security policies, column-level masking, virtual private database patterns,
//! and predicate injection. See [`fgac`] module for details.
//!
//! ### 3. Encryption Services
//! Transparent Data Encryption (TDE), column-level encryption, key rotation without
//! downtime, and HSM integration. See [`encryption`] module for details.
//!
//! ### 4. Audit System
//! Statement and object-level auditing with fine-grained conditions and tamper
//! protection. See [`audit`] module for details.
//!
//! ### 5. Authentication Framework
//! Password policies, multi-factor authentication, LDAP/AD integration, and OAuth2/OIDC
//! support. See [`authentication`] module for details.
//!
//! ### 6. Privilege Management
//! System and object privileges, GRANT/REVOKE operations with admin option, and
//! privilege inheritance. See [`privileges`] module for details.
//!
//! ### 7. Security Labels
//! Mandatory access control, multi-level security, compartment-based security, and
//! label-based filtering. See [`labels`] module for details.
//!
//! ### 8. Buffer Overflow Protection
//! Comprehensive bounds checking, stack canaries, integer overflow guards, and safe
//! memory operations. See [`bounds_protection`] module for details.
//!
//! ### 9. Secure Garbage Collection
//! Memory sanitization, secure deallocation, cryptographic erasure, and heap spray
//! prevention. See [`secure_gc`] module for details.
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use rusty_db::security::*;
//!
//! # fn example() -> rusty_db::Result<()> {
//! // Create integrated security manager
//! let security = IntegratedSecurityManager::new();
//!
//! // Authenticate user
//! let session = security.authenticate("username", "password")?;
//!
//! // Check permissions
//! let can_select = security.check_permission(
//!     &session.session_id,
//!     "SELECT",
//!     "employees",
//! )?;
//!
//! // Apply row-level security
//! let filtered_rows = security.filter_rows(
//!     &session.session_id,
//!     "employees",
//!     vec!["row1".to_string(), "row2".to_string()],
//! )?;
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use std::sync::Arc;
use crate::Result;
use crate::error::DbError;

// Re-export all submodules
pub mod rbac;
pub mod fgac;
pub mod encryption;
pub mod encryption_engine;
pub mod audit;
pub mod authentication;
pub mod privileges;
pub mod labels;
pub mod bounds_protection;
pub mod secure_gc;
pub mod injection_prevention;
pub mod network_hardening;
pub mod insider_threat;
pub mod auto_recovery;
pub mod memory_hardening;
pub mod circuit_breaker;

// Re-export commonly used types
pub use rbac::{RbacManager, Role, RoleAssignment, SeparationOfDutiesConstraint};
pub use fgac::{FgacManager, RowLevelPolicy, ColumnPolicy, SecurityContext as FgacContext};
pub use encryption::{EncryptionManager, EncryptionKey, TdeConfig, ColumnEncryption};
pub use encryption_engine::{
    EncryptionEngine, KeyManager, SecureKey, SecureKeyMaterial, ColumnEncryptor,
    TransparentEncryption, KeyRotator, EncryptedIndex, SecureKeyStore, CryptoRandom,
    Algorithm, Ciphertext, KeyDerivation,
};
pub use audit::{AuditManager, AuditRecord, AuditAction, AuditPolicy};
pub use authentication::{
    AuthenticationManager, UserAccount, AuthSession, PasswordPolicy, LoginCredentials, LoginResult
};
pub use privileges::{PrivilegeManager, SystemPrivilege, ObjectPrivilege, PrivilegeGrant};
pub use labels::{LabelManager, SecurityLabel, ClassificationLevel, UserClearance};
pub use bounds_protection::{
    BoundsCheckedBuffer, SafeSlice, SafeSliceMut, SafeIndex, OverflowGuard,
    StackCanary, SafeString, ArrayBoundsChecker,
};
pub use secure_gc::{
    SecureDrop, SensitiveData, MemorySanitizer, CryptoErase, SecurePool,
    ReferenceTracker, DelayedSanitizer, HeapGuard,
};
pub use injection_prevention::{
    InjectionPreventionGuard, InputSanitizer, DangerousPatternDetector,
    SQLValidator, ParameterizedQueryBuilder, UnicodeNormalizer,
    EscapeValidator, QueryWhitelister, ThreatDetection, ThreatType,
    Severity, PreparedStatement, Parameter, ParameterValue, ParameterType,
};
pub use network_hardening::{
    NetworkHardeningManager, AdaptiveRateLimiter, ConnectionGuard, DDoSMitigator,
    ProtocolValidator, TLSEnforcer, NetworkAnomalyDetector, IPReputationChecker,
    DDoSAttackType, ViolationType, AnomalyType, NetworkHardeningStats,
};
pub use insider_threat::{
    InsiderThreatManager, ThreatScorer, BehaviorAnalyzer, AnomalyDetector,
    DataExfiltrationGuard, PrivilegeEscalationDetector, QuerySanitizer, ForensicLogger,
    QueryRiskAssessment, ThreatAction, InsiderThreatConfig, ThreatStatistics,
    UserBehaviorBaseline, AnomalyScore, ExfiltrationAttempt, PrivilegeEscalationAttempt,
    ThreatLevel as InsiderThreatLevel,
};
pub use auto_recovery::{
    AutoRecoveryManager, AutoRecoveryConfig, CrashDetector, TransactionRollbackManager,
    CorruptionDetector, DataRepairer, StateSnapshotManager, HealthMonitor, SelfHealer,
};
pub use memory_hardening::{
    SecureBuffer, GuardedMemory, SecureZeroingAllocator, MemoryCanary,
    IsolatedHeap, MemoryHardeningConfig, CanaryCheckFrequency,
    AllocatorStatsSnapshot, IsolatedHeapStatsSnapshot, PAGE_SIZE, CANARY_SIZE,
    SecurityMetrics as MemorySecurityMetrics,
};
pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitState, CircuitBreakerMetrics,
};

/// Integrated security manager combining all security subsystems
pub struct IntegratedSecurityManager {
    /// RBAC manager
    pub rbac: Arc<RbacManager>,
    /// FGAC manager
    pub fgac: Arc<FgacManager>,
    /// Encryption manager
    pub encryption: Arc<EncryptionManager>,
    /// Audit manager
    pub audit: Arc<AuditManager>,
    /// Authentication manager
    pub authentication: Arc<AuthenticationManager>,
    /// Privilege manager
    pub privileges: Arc<PrivilegeManager>,
    /// Label manager
    pub labels: Arc<LabelManager>,
    /// Insider threat manager
    pub insider_threat: Arc<InsiderThreatManager>,
}

impl IntegratedSecurityManager {
    /// Create a new integrated security manager
    pub fn new() -> Self {
        Self {
            rbac: Arc::new(RbacManager::new()),
            fgac: Arc::new(FgacManager::new()),
            encryption: Arc::new(EncryptionManager::new()),
            audit: Arc::new(AuditManager::new()),
            authentication: Arc::new(AuthenticationManager::new()),
            privileges: Arc::new(PrivilegeManager::new()),
            labels: Arc::new(LabelManager::new()),
            insider_threat: Arc::new(InsiderThreatManager::new()),
        }
    }

    /// Authenticate a user and create a session
    pub fn authenticate(&self, username: &str, password: &str) -> Result<AuthSession> {
        let credentials = LoginCredentials {
            username: username.to_string(),
            password: password.to_string(),
            mfa_code: None,
            client_ip: None,
            user_agent: None,
        };

        let result = self.authentication.login(credentials)?;

        match result {
            LoginResult::Success { session } => {
                // Log successful authentication
                self.audit.log_event(
                    username.to_string(),
                    Some(session.session_id.clone()),
                    AuditAction::Login,
                    None,
                    None,
                    true,
                    HashMap::new(),
                )?;

                Ok(session)
            }
            LoginResult::MfaRequired { .. } => {
                Err(DbError::Network("MFA required".to_string()))
            }
            LoginResult::PasswordChangeRequired { .. } => {
                Err(DbError::Network("Password change required".to_string()))
            }
            LoginResult::AccountLocked { .. } => {
                // Log failed login
                self.audit.log_event(
                    username.to_string(),
                    None,
                    AuditAction::FailedLogin,
                    None,
                    None,
                    false,
                    HashMap::new(),
                )?;

                Err(DbError::Network("Account locked".to_string()))
            }
            LoginResult::InvalidCredentials => {
                // Log failed login
                self.audit.log_event(
                    username.to_string(),
                    None,
                    AuditAction::FailedLogin,
                    None,
                    None,
                    false,
                    HashMap::new(),
                )?;

                Err(DbError::Network("Invalid credentials".to_string()))
            }
            LoginResult::AccountDisabled => {
                Err(DbError::Network("Account disabled".to_string()))
            }
        }
    }

    /// Check if user has permission for an operation
    pub fn check_permission(
        &self,
        session_id: &str,
        operation: &str,
        object: &str,
    ) -> Result<bool> {
        // Validate session
        let session = self.authentication.validate_session(session_id)?;

        // Map operation to privilege
        let privilege = match operation.to_uppercase().as_str() {
            "SELECT" => ObjectPrivilege::Select,
            "INSERT" => ObjectPrivilege::Insert,
            "UPDATE" => ObjectPrivilege::Update,
            "DELETE" => ObjectPrivilege::Delete,
            "ALTER" => ObjectPrivilege::Alter,
            _ => return Ok(false),
        };

        // Check privilege
        let result = self.privileges.check_object_privilege(
            &session.user_id,
            &privilege,
            &privileges::PrivilegeObjectType::Table,
            &object.to_string(),
            None,
        );

        Ok(result.has_privilege)
    }

    /// Filter rows based on security policies
    pub fn filter_rows(
        &self,
        session_id: &str,
        table_id: &str,
        row_ids: Vec<String>,
    ) -> Result<Vec<String>> {
        // Validate session
        let session = self.authentication.validate_session(session_id)?;

        // Apply label-based filtering
        let label_filtered = if let Ok(_clearance) = self.labels.get_user_clearance(&session.user_id) {
            self.labels.filter_readable_rows(&session.user_id, table_id, row_ids)?
        } else {
            row_ids
        };

        // Additional FGAC filtering could be applied here

        Ok(label_filtered)
    }

    /// Grant a privilege to a user
    pub fn grant_privilege(
        &self,
        grantor_session_id: &str,
        grantee: &str,
        privilege: SystemPrivilege,
        with_grant_option: bool,
    ) -> Result<String> {
        // Validate grantor session
        let session = self.authentication.validate_session(grantor_session_id)?;

        // Grant the privilege
        let grant_id = self.privileges.grant_system_privilege(
            session.user_id.clone(),
            grantee.to_string(),
            privilege.clone(),
            with_grant_option,
        )?;

        // Audit the grant
        let mut context = HashMap::new();
        context.insert("privilege".to_string(), format!("{:?}", privilege));
        context.insert("grantee".to_string(), grantee.to_string());

        self.audit.log_event(
            session.username.clone(),
            Some(session.session_id.clone()),
            AuditAction::Grant,
            None,
            None,
            true,
            context,
        )?;

        Ok(grant_id)
    }

    /// Encrypt sensitive data
    pub fn encrypt_data(&self, key_id: &str, plaintext: &[u8]) -> Result<Vec<u8>> {
        let ciphertext = self.encryption.encrypt_data(key_id, plaintext)?;

        // Audit encryption operation
        self.audit.log_event(
            "SYSTEM".to_string(),
            None,
            AuditAction::Custom("ENCRYPT_DATA".to_string()),
            None,
            None,
            true,
            HashMap::new(),
        )?;

        Ok(ciphertext)
    }

    /// Decrypt sensitive data
    pub fn decrypt_data(&self, key_id: &str, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let plaintext = self.encryption.decrypt_data(key_id, ciphertext)?;

        // Audit decryption operation
        self.audit.log_event(
            "SYSTEM".to_string(),
            None,
            AuditAction::Custom("DECRYPT_DATA".to_string()),
            None,
            None,
            true,
            HashMap::new(),
        )?;

        Ok(plaintext)
    }

    /// Get security statistics
    pub fn get_statistics(&self) -> SecurityStatistics {
        SecurityStatistics {
            audit_stats: self.audit.get_statistics(),
            encryption_stats: self.encryption.get_statistics(),
            privilege_stats: self.privileges.get_statistics(),
            label_stats: self.labels.get_statistics(),
            fgac_stats: self.fgac.get_statistics(),
            active_sessions: self.authentication.sessions.read().len(),
            total_users: self.authentication.users.read().len(),
            threat_stats: self.insider_threat.get_statistics(),
        }
    }

    /// Assess query for insider threats
    pub fn assess_query_threat(
        &self,
        user_id: &str,
        session_id: Option<String>,
        query: &str,
        tables: Vec<String>,
        estimated_rows: u64,
        client_ip: Option<String>,
        location: Option<String>,
    ) -> Result<QueryRiskAssessment> {
        self.insider_threat.assess_query(
            &user_id.to_string(),
            session_id,
            query,
            tables,
            estimated_rows,
            client_ip,
            location,
        )
    }
}

impl Default for IntegratedSecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined security statistics
#[derive(Debug, Clone)]
pub struct SecurityStatistics {
    pub audit_stats: audit::AuditStatistics,
    pub encryption_stats: encryption::EncryptionStatistics,
    pub privilege_stats: privileges::PrivilegeStatistics,
    pub label_stats: labels::LabelStatistics,
    pub fgac_stats: fgac::FgacStatistics,
    pub active_sessions: usize,
    pub total_users: usize,
    pub threat_stats: insider_threat::ThreatStatistics,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integrated_security() {
        let security = IntegratedSecurityManager::new();

        // Create a user
        let user_id = security.authentication.create_user(
            "testuser".to_string(),
            "TestPassword123!".to_string(),
            Some("test@example.com".to_string()),
        ).unwrap();

        // Update user status to active
        {
            let mut users = security.authentication.users.write();
            if let Some(user) = users.get_mut(&user_id) {
                user.status = authentication::AccountStatus::Active;
            }
        }

        // Test authentication
        let result = security.authenticate("testuser", "TestPassword123!");
        assert!(result.is_ok());
    }

    #[test]
    fn test_privilege_check() {
        let security = IntegratedSecurityManager::new();

        // Grant system privilege
        let _grant_id = security.privileges.grant_system_privilege(
            "SYSTEM".to_string(),
            "user1".to_string(),
            SystemPrivilege::CreateTable,
            false,
        ).unwrap();

        // Check privilege
        let result = security.privileges.check_system_privilege(
            &"user1".to_string(),
            &SystemPrivilege::CreateTable,
        );

        assert!(result.has_privilege);
    }

    #[test]
    fn test_audit_logging() {
        let security = IntegratedSecurityManager::new();

        let id = security.audit.log_event(
            "testuser".to_string(),
            Some("session1".to_string()),
            AuditAction::Select,
            Some("users".to_string()),
            Some(audit::ObjectType::Table),
            true,
            HashMap::new(),
        ).unwrap();

        assert!(id > 0);
    }

    #[test]
    fn test_encryption_key_generation() {
        let security = IntegratedSecurityManager::new();

        // Initialize master key
        let master_key = vec![0u8; 32];
        security.encryption.initialize_master_key(master_key).unwrap();

        // Generate table encryption key
        let key_id = security.encryption.generate_key(
            encryption::KeyType::TableEncryption,
            encryption::EncryptionAlgorithm::Aes256Gcm,
            Some("MASTER_KEY".to_string()),
        ).unwrap();

        assert!(key_id.starts_with("KEY_"));
    }

    #[test]
    fn test_label_based_access() {
        let security = IntegratedSecurityManager::new();

        // Register compartment
        let compartment = labels::Compartment {
            id: "SECRET".to_string(),
            name: "Secret".to_string(),
            description: None,
            parent: None,
        };
        security.labels.register_compartment(compartment).unwrap();

        // Set user clearance
        let mut max_read = SecurityLabel::new(ClassificationLevel::Secret);
        max_read.add_compartment("SECRET".to_string());

        let clearance = UserClearance {
            user_id: "user1".to_string(),
            max_read: max_read.clone(),
            max_write: max_read.clone(),
            current_label: max_read,
            authorized_compartments: HashSet::from(["SECRET".to_string()]),
            authorized_groups: HashSet::new(),
        };

        assert!(security.labels.set_user_clearance(clearance).is_ok());
    }

    #[test]
    fn test_fgac_policy() {
        let security = IntegratedSecurityManager::new();

        let policy = fgac::RowLevelPolicy {
            id: "pol1".to_string(),
            name: "Test Policy".to_string(),
            table_id: "employees".to_string(),
            policy_type: fgac::PolicyType::Permissive,
            predicate: "department = 'Engineering'".to_string(),
            principals: vec![],
            enabled: true,
            priority: 100,
            created_at: 0,
            updated_at: 0,
            description: None,
        };

        assert!(security.fgac.add_row_policy(policy).is_ok());
    }

    #[test]
    fn test_rbac_role_creation() {
        let security = IntegratedSecurityManager::new();

        let role = rbac::Role::new("admin".to_string(), "Administrator".to_string());
        assert!(security.rbac.create_role(role).is_ok());
    }
}
pub mod security_core;
pub use security_core::{
    UnifiedSecurityCore, SecurityPolicyEngine, DefenseOrchestrator,
    SecurityEventCorrelator, ThreatIntelligence, ComplianceValidator,
    SecurityMetrics as CoreSecurityMetrics, PenetrationTestHarness, SecurityDashboard,
    SecurityPolicy, PolicyType, PolicyEffect, PolicyDecision,
    DefenseLayer, ThreatLevel as CoreThreatLevel, DefenseCoverageReport,
    SecurityIncident, IncidentStatus, EventSeverity,
    IndicatorOfCompromise, IocType, ThreatActor,
    ComplianceFramework, ComplianceControl, ComplianceStatus, ComplianceSummary,
    SecurityPostureScore, PenTestReport, PenTestSummary,
    DashboardView, ExecutiveSummary, SecurityStatus,
};
