// # Advanced Security Vault Engine
//
// Oracle-like comprehensive security vault providing enterprise-grade data protection,
// encryption, masking, auditing, and access control capabilities.
//
// ## Overview
//
// The Security Vault Engine provides:
// - **Transparent Data Encryption (TDE)**: Tablespace and column-level encryption
// - **Data Masking**: Static and dynamic masking with format-preserving encryption
// - **Key Management**: Hierarchical key management with envelope encryption
// - **Audit Vault**: Tamper-evident audit trails with blockchain backing
// - **Virtual Private Database (VPD)**: Row and column-level security
// - **Privilege Analysis**: Least privilege analysis and optimization
//
// ## Architecture
//
// ```text
// ┌─────────────────────────────────────────────────────────────┐
// │                    Security Vault Manager                    │
// ├─────────────────────────────────────────────────────────────┤
// │  TDE Engine  │  Masking  │  Keystore  │  Audit  │  VPD      │
// │  (AES-256,   │  (Static/ │  (MEK/DEK  │  (FGA)  │  (RLS)    │
// │   ChaCha20)  │  Dynamic) │  Hierarchy)│         │           │
// └─────────────────────────────────────────────────────────────┘
// ```
//
// ## Usage Example
//
// ```rust,no_run
// use rusty_db::security_vault::{SecurityVaultManager, TdeEngine, KeyStore};
//
// # async fn example() -> rusty_db::Result<()> {
// // Initialize security vault
// let mut vault = SecurityVaultManager::new("/secure/vault".to_string()?;
//
// // Enable TDE for a tablespace
// vault.enable_tablespace_encryption("users_ts", "AES256GCM").await?;
//
// // Configure data masking
// vault.create_masking_policy("mask_ssn", "SSN", "PARTIAL_MASK").await?;
//
// // Set up VPD policy
// vault.create_vpd_policy("customer_data", "dept_id = SYS_CONTEXT('USERENV', 'DEPT')")
//     .await?;
// # Ok(())
// # }
// ```

use crate::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::Mutex as AsyncMutex;

pub mod tde;
pub mod masking;
pub mod keystore;
pub mod audit;
pub mod vpd;
pub mod privileges;

pub use tde::{TdeEngine, EncryptionAlgorithm, TdeConfig};
pub use masking::{MaskingEngine, MaskingPolicy, MaskingType};
pub use keystore::{KeyStore, MasterKey, DataEncryptionKey, KeyVersion};
pub use audit::{AuditVault, AuditRecord, AuditPolicy, ComplianceReport};
pub use vpd::{VpdEngine, VpdPolicy, SecurityPredicate};
pub use privileges::{PrivilegeAnalyzer, PrivilegePath, PrivilegeRecommendation};

/// Security context for the current session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// User identifier
    pub user_id: String,
    /// Session identifier
    pub session_id: String,
    /// IP address of client
    pub client_ip: String,
    /// Authenticated roles
    pub roles: Vec<String>,
    /// Security clearance level (0-10)
    pub clearance_level: u8,
    /// Additional context attributes
    pub attributes: HashMap<String, String>,
    /// Timestamp of context creation
    pub created_at: i64,
}

impl SecurityContext {
    /// Create a new security context
    pub fn new(user_id: String, session_id: String, client_ip: String) -> Self {
        Self {
            user_id,
            session_id,
            client_ip,
            roles: Vec::new(),
            clearance_level: 0,
            attributes: HashMap::new(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Get a context attribute
    pub fn get_attribute(&self, key: &str) -> Option<&String> {
        self.attributes.get(key)
    }

    /// Set a context attribute
    pub fn set_attribute(&mut self, key: String, value: String) {
        self.attributes.insert(key, value);
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if clearance level meets minimum requirement
    pub fn meets_clearance(&self, required: u8) -> bool {
        self.clearance_level >= required
    }
}

/// Encryption statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EncryptionStats {
    /// Total bytes encrypted
    pub bytes_encrypted: u64,
    /// Total bytes decrypted
    pub bytes_decrypted: u64,
    /// Number of encryption operations
    pub encrypt_operations: u64,
    /// Number of decryption operations
    pub decrypt_operations: u64,
    /// Number of key rotations
    pub key_rotations: u64,
    /// Failed operations
    pub failed_operations: u64,
}

/// Audit statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditStats {
    /// Total audit records
    pub total_records: u64,
    /// Records by policy
    pub records_by_policy: HashMap<String, u64>,
    /// Failed audit writes
    pub failed_writes: u64,
    /// Tamper detection alerts
    pub tamper_alerts: u64,
}

/// Security vault configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Vault data directory
    pub data_dir: PathBuf,
    /// Enable TDE by default for new tablespaces
    pub default_tde_enabled: bool,
    /// Default encryption algorithm
    pub default_algorithm: String,
    /// Enable audit vault
    pub audit_enabled: bool,
    /// Audit retention period in days
    pub audit_retention_days: u32,
    /// Enable VPD enforcement
    pub vpd_enabled: bool,
    /// Master key rotation interval in days
    pub key_rotation_days: u32,
    /// Enable HSM integration
    pub hsm_enabled: bool,
    /// HSM configuration
    pub hsm_config: Option<HashMap<String, String>>,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("/var/lib/rustydb/security_vault"),
            default_tde_enabled: true,
            default_algorithm: "AES256GCM".to_string(),
            audit_enabled: true,
            audit_retention_days: 365,
            vpd_enabled: true,
            key_rotation_days: 90,
            hsm_enabled: false,
            hsm_config: None,
        }
    }
}

/// Main Security Vault Manager
///
/// Coordinates all security subsystems including TDE, masking, key management,
/// auditing, and access control.
pub struct SecurityVaultManager {
    /// Configuration
    config: VaultConfig,
    /// TDE engine
    tde_engine: Arc<RwLock<TdeEngine>>,
    /// Masking engine
    masking_engine: Arc<RwLock<MaskingEngine>>,
    /// Key store
    key_store: Arc<AsyncMutex<KeyStore>>,
    /// Audit vault
    audit_vault: Arc<AsyncMutex<AuditVault>>,
    /// VPD engine
    vpd_engine: Arc<RwLock<VpdEngine>>,
    /// Privilege analyzer
    privilege_analyzer: Arc<RwLock<PrivilegeAnalyzer>>,
    /// Encryption statistics
    encryption_stats: Arc<RwLock<EncryptionStats>>,
    /// Audit statistics
    audit_stats: Arc<RwLock<AuditStats>>,
    /// Active security contexts
    active_contexts: Arc<RwLock<HashMap<String, SecurityContext>>>,
}

impl SecurityVaultManager {
    /// Create a new security vault manager
    pub fn new(data_dir: String) -> Result<Self> {
        let config = VaultConfig {
            data_dir: PathBuf::from(data_dir),
            ..Default::default()
        };

        Self::with_config(config)
    }

    /// Create with custom configuration
    pub fn with_config(config: VaultConfig) -> Result<Self> {
        // Create data directory if it doesn't exist
        std::fs::create_dir_all(&config.data_dir)
            .map_err(|e| DbError::IoError(format!("Failed to create vault directory: {}", e)))?;

        // Initialize key store first
        let key_store = KeyStore::new(config.data_dir.join("keystore"))?;

        // Initialize TDE engine
        let tde_engine = TdeEngine::new()?;

        // Initialize masking engine
        let masking_engine = MaskingEngine::new()?;

        // Initialize audit vault
        let audit_vault = AuditVault::new(
            config.data_dir.join("audit"),
            config.audit_retention_days,
        )?;

        // Initialize VPD engine
        let vpd_engine = VpdEngine::new()?;

        // Initialize privilege analyzer
        let privilege_analyzer = PrivilegeAnalyzer::new()?;

        Ok(Self {
            config,
            tde_engine: Arc::new(RwLock::new(tde_engine)),
            masking_engine: Arc::new(RwLock::new(masking_engine)),
            key_store: Arc::new(AsyncMutex::new(key_store)),
            audit_vault: Arc::new(AsyncMutex::new(audit_vault)),
            vpd_engine: Arc::new(RwLock::new(vpd_engine)),
            privilege_analyzer: Arc::new(RwLock::new(privilege_analyzer)),
            encryption_stats: Arc::new(RwLock::new(EncryptionStats::default())),
            audit_stats: Arc::new(RwLock::new(AuditStats::default())),
            active_contexts: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create a security context for a session
    pub fn create_security_context(
        &self,
        user_id: String,
        session_id: String,
        client_ip: String,
    ) -> SecurityContext {
        let context = SecurityContext::new(user_id, session_id.clone(), client_ip);
        self.active_contexts.write().insert(session_id, context.clone());
        context
    }

    /// Get security context for a session
    pub fn get_security_context(&self, session_id: &str) -> Option<SecurityContext> {
        self.active_contexts.read().get(session_id).cloned()
    }

    /// Remove security context
    pub fn remove_security_context(&self, session_id: &str) {
        self.active_contexts.write().remove(session_id);
    }

    /// Enable tablespace encryption
    pub async fn enable_tablespace_encryption(
        &mut self,
        tablespace_name: &str,
        algorithm: &str,
    ) -> Result<()> {
        // Generate data encryption key
        let mut key_store = self.key_store.lock().await;
        let dek = key_store.generate_dek(tablespace_name, algorithm)?;
        drop(key_store);

        // Configure TDE
        let mut tde = self.tde_engine.write();
        tde.enable_tablespace_encryption(tablespace_name, algorithm, &dek)?;

        // Audit the operation
        let mut audit = self.audit_vault.lock().await;
        audit.log_security_event(
            "SYSTEM",
            "ENABLE_TDE",
            &format!("Tablespace: {}, Algorithm: {}", tablespace_name, algorithm),
        )?;

        Ok(())
    }

    /// Enable column-level encryption
    pub async fn enable_column_encryption(
        &mut self,
        table_name: &str,
        column_name: &str,
        algorithm: &str,
    ) -> Result<()> {
        let key_id = format!("{}:{}", table_name, column_name);

        // Generate column encryption key
        let mut key_store = self.key_store.lock().await;
        let dek = key_store.generate_dek(&key_id, algorithm)?;
        drop(key_store);

        // Configure TDE
        let mut tde = self.tde_engine.write();
        tde.enable_column_encryption(table_name, column_name, algorithm, &dek)?;

        Ok(())
    }

    /// Create a data masking policy
    pub async fn create_masking_policy(
        &mut self,
        policy_name: &str,
        column_pattern: &str,
        masking_type: &str,
    ) -> Result<()> {
        let mut masking = self.masking_engine.write();
        masking.create_policy(policy_name, column_pattern, masking_type)?;

        // Audit
        let mut audit = self.audit_vault.lock().await;
        audit.log_security_event(
            "SYSTEM",
            "CREATE_MASKING_POLICY",
            &format!("Policy: {}, Type: {}", policy_name, masking_type),
        )?;

        Ok(())
    }

    /// Create a VPD policy
    pub async fn create_vpd_policy(
        &mut self,
        table_name: &str,
        predicate: &str,
    ) -> Result<()> {
        let mut vpd = self.vpd_engine.write();
        vpd.create_policy(table_name, predicate)?;

        // Audit
        let mut audit = self.audit_vault.lock().await;
        audit.log_security_event(
            "SYSTEM",
            "CREATE_VPD_POLICY",
            &format!("Table: {}, Predicate: {}", table_name, predicate),
        )?;

        Ok(())
    }

    /// Rotate encryption keys
    pub async fn rotate_keys(&mut self) -> Result<usize> {
        let mut key_store = self.key_store.lock().await;
        let rotated = key_store.rotate_expired_deks()?;

        self.encryption_stats.write().key_rotations += rotated as u64;

        // Audit
        let mut audit = self.audit_vault.lock().await;
        audit.log_security_event(
            "SYSTEM",
            "KEY_ROTATION",
            &format!("Rotated {} keys", rotated),
        )?;

        Ok(rotated)
    }

    /// Generate compliance report
    pub async fn generate_compliance_report(
        &self,
        regulation: &str,
        start_date: i64,
        end_date: i64,
    ) -> Result<ComplianceReport> {
        let audit = self.audit_vault.lock().await;
        audit.generate_compliance_report(regulation, start_date, end_date)
    }

    /// Analyze privileges for a user
    pub fn analyze_user_privileges(&self, user_id: &str) -> Result<Vec<PrivilegeRecommendation>> {
        let analyzer = self.privilege_analyzer.read();
        analyzer.analyze_user(user_id)
    }

    /// Get encryption statistics
    pub fn get_encryption_stats(&self) -> EncryptionStats {
        self.encryption_stats.read().clone()
    }

    /// Get audit statistics
    pub fn get_audit_stats(&self) -> AuditStats {
        self.audit_stats.read().clone()
    }

    /// Verify audit integrity
    pub async fn verify_audit_integrity(&self) -> Result<bool> {
        let audit = self.audit_vault.lock().await;
        audit.verify_integrity()
    }

    /// Export security configuration
    pub fn export_config(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.config)
            .map_err(|e| DbError::Serialization(format!("Failed to export config: {}", e)))
    }

    /// Get TDE engine reference
    pub fn tde_engine(&self) -> Arc<RwLock<TdeEngine>> {
        Arc::clone(&self.tde_engine)
    }

    /// Get masking engine reference
    pub fn masking_engine(&self) -> Arc<RwLock<MaskingEngine>> {
        Arc::clone(&self.masking_engine)
    }

    /// Get key store reference
    pub fn key_store(&self) -> Arc<AsyncMutex<KeyStore>> {
        Arc::clone(&self.key_store)
    }

    /// Get audit vault reference
    pub fn audit_vault(&self) -> Arc<AsyncMutex<AuditVault>> {
        Arc::clone(&self.audit_vault)
    }

    /// Get VPD engine reference
    pub fn vpd_engine(&self) -> Arc<RwLock<VpdEngine>> {
        Arc::clone(&self.vpd_engine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_context() {
        let mut ctx = SecurityContext::new(
            "user1".to_string(),
            "session1".to_string(),
            "192.168.1.1".to_string(),
        );

        ctx.roles.push("DBA".to_string());
        ctx.clearance_level = 5;
        ctx.set_attribute("dept".to_string(), "IT".to_string());

        assert!(ctx.has_role("DBA"));
        assert!(!ctx.has_role("READONLY"));
        assert!(ctx.meets_clearance(3));
        assert!(!ctx.meets_clearance(7));
        assert_eq!(ctx.get_attribute("dept").unwrap(), "IT");
    }

    #[tokio::test]
    async fn test_vault_manager_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let vault = SecurityVaultManager::new(temp_dir.path().to_str().unwrap().to_string());
        assert!(vault.is_ok());
    }

    #[tokio::test]
    async fn test_security_context_management() {
        let temp_dir = tempfile::tempdir().unwrap();
        let vault = SecurityVaultManager::new(temp_dir.path().to_str().unwrap().to_string()).unwrap();

        let ctx = vault.create_security_context(
            "user1".to_string(),
            "session1".to_string(),
            "127.0.0.1".to_string(),
        );

        assert_eq!(ctx.user_id, "user1");
        assert_eq!(ctx.session_id, "session1");

        let retrieved = vault.get_security_context("session1");
        assert!(retrieved.is_some());

        vault.remove_security_context("session1");
        let after_remove = vault.get_security_context("session1");
        assert!(after_remove.is_none());
    }
}
