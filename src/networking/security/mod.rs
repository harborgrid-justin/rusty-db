//! Network security module for RustyDB distributed database
//!
//! This module provides enterprise-grade network security features including:
//! - Mutual TLS (mTLS) for authentication
//! - Certificate management and rotation
//! - Node identity verification
//! - Message encryption
//! - Access control lists (ACLs)
//! - Application-level firewall
//!
//! # Example
//!
//! ```no_run
//! use rusty_db::networking::security::{SecurityConfig, SecurityManager};
//!
//! let config = SecurityConfig::default()
//!     .with_mtls_enabled(true)
//!     .with_firewall_enabled(true);
//!
//! let manager = SecurityManager::new(config)?;
//! ```

use crate::error::{DbError, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod tls;
pub mod mtls;
pub mod certificates;
pub mod identity;
pub mod encryption;
pub mod acl;
pub mod firewall;

pub use tls::{TlsConfig, TlsVersion, CipherSuite};
pub use mtls::{MtlsAuthenticator, MtlsConfig};
pub use certificates::{CertificateManager, CertificateConfig};
pub use identity::{NodeIdentity, IdentityProvider};
pub use encryption::{MessageEncryption, EncryptionConfig};
pub use acl::{NetworkAcl, AclRule, Action};
pub use firewall::{ApplicationFirewall, FirewallConfig};

/// Security configuration for the network layer
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable mutual TLS
    pub mtls_enabled: bool,

    /// TLS configuration
    pub tls_config: Option<TlsConfig>,

    /// mTLS configuration
    pub mtls_config: Option<MtlsConfig>,

    /// Certificate configuration
    pub cert_config: Option<CertificateConfig>,

    /// Encryption configuration
    pub encryption_config: Option<EncryptionConfig>,

    /// Enable firewall
    pub firewall_enabled: bool,

    /// Firewall configuration
    pub firewall_config: Option<FirewallConfig>,

    /// Enable ACLs
    pub acl_enabled: bool,

    /// ACL rules file path
    pub acl_rules_path: Option<PathBuf>,

    /// Enable audit logging
    pub audit_enabled: bool,

    /// Audit log path
    pub audit_log_path: Option<PathBuf>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            mtls_enabled: false,
            tls_config: None,
            mtls_config: None,
            cert_config: None,
            encryption_config: None,
            firewall_enabled: false,
            firewall_config: None,
            acl_enabled: false,
            acl_rules_path: None,
            audit_enabled: true,
            audit_log_path: Some(PathBuf::from("./logs/security_audit.log")),
        }
    }
}

impl SecurityConfig {
    /// Create a new security configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable mutual TLS
    pub fn with_mtls_enabled(mut self, enabled: bool) -> Self {
        self.mtls_enabled = enabled;
        self
    }

    /// Set TLS configuration
    pub fn with_tls_config(mut self, config: TlsConfig) -> Self {
        self.tls_config = Some(config);
        self
    }

    /// Set mTLS configuration
    pub fn with_mtls_config(mut self, config: MtlsConfig) -> Self {
        self.mtls_config = Some(config);
        self
    }

    /// Enable firewall
    pub fn with_firewall_enabled(mut self, enabled: bool) -> Self {
        self.firewall_enabled = enabled;
        self
    }

    /// Set firewall configuration
    pub fn with_firewall_config(mut self, config: FirewallConfig) -> Self {
        self.firewall_config = Some(config);
        self
    }

    /// Enable ACLs
    pub fn with_acl_enabled(mut self, enabled: bool) -> Self {
        self.acl_enabled = enabled;
        self
    }

    /// Set ACL rules path
    pub fn with_acl_rules_path(mut self, path: PathBuf) -> Self {
        self.acl_rules_path = Some(path);
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.mtls_enabled {
            if self.tls_config.is_none() {
                return Err(DbError::Configuration(
                    "mTLS enabled but TLS config not provided".to_string()
                ));
            }
            if self.mtls_config.is_none() {
                return Err(DbError::Configuration(
                    "mTLS enabled but mTLS config not provided".to_string()
                ));
            }
        }

        if self.acl_enabled && self.acl_rules_path.is_none() {
            return Err(DbError::Configuration(
                "ACL enabled but rules path not provided".to_string()
            ));
        }

        Ok(())
    }
}

/// Authentication provider trait
pub trait AuthProvider: Send + Sync {
    /// Authenticate a connection
    fn authenticate(&self, credentials: &[u8]) -> Result<AuthContext>;

    /// Validate session token
    fn validate_token(&self, token: &str) -> Result<AuthContext>;
}

/// Authorization provider trait
pub trait AuthzProvider: Send + Sync {
    /// Check if action is authorized
    fn authorize(&self, context: &AuthContext, action: &str, resource: &str) -> Result<bool>;
}

/// Authentication context
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// User or node identifier
    pub principal: String,

    /// Roles
    pub roles: Vec<String>,

    /// Attributes
    pub attributes: std::collections::HashMap<String, String>,

    /// Authentication timestamp
    pub authenticated_at: std::time::SystemTime,

    /// Session expiry
    pub expires_at: Option<std::time::SystemTime>,
}

impl AuthContext {
    /// Create a new authentication context
    pub fn new(principal: String) -> Self {
        Self {
            principal,
            roles: Vec::new(),
            attributes: std::collections::HashMap::new(),
            authenticated_at: std::time::SystemTime::now(),
            expires_at: None,
        }
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            std::time::SystemTime::now() > expires_at
        } else {
            false
        }
    }

    /// Add role
    pub fn add_role(&mut self, role: String) {
        self.roles.push(role);
    }

    /// Add attribute
    pub fn add_attribute(&mut self, key: String, value: String) {
        self.attributes.insert(key, value);
    }
}

/// Security manager that coordinates all security components
pub struct SecurityManager {
    /// Configuration
    config: SecurityConfig,

    /// Certificate manager
    cert_manager: Option<Arc<RwLock<CertificateManager>>>,

    /// mTLS authenticator
    mtls_authenticator: Option<Arc<MtlsAuthenticator>>,

    /// Node identity provider
    identity_provider: Option<Arc<IdentityProvider>>,

    /// Message encryption
    message_encryption: Option<Arc<MessageEncryption>>,

    /// Network ACL
    network_acl: Option<Arc<RwLock<NetworkAcl>>>,

    /// Application firewall
    firewall: Option<Arc<RwLock<ApplicationFirewall>>>,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new(config: SecurityConfig) -> Result<Self> {
        config.validate()?;

        let cert_manager = if config.mtls_enabled {
            if let Some(cert_config) = &config.cert_config {
                Some(Arc::new(RwLock::new(
                    CertificateManager::new(cert_config.clone())?
                )))
            } else {
                None
            }
        } else {
            None
        };

        let mtls_authenticator = if config.mtls_enabled {
            if let Some(mtls_config) = &config.mtls_config {
                Some(Arc::new(MtlsAuthenticator::new(mtls_config.clone())?))
            } else {
                None
            }
        } else {
            None
        };

        let identity_provider = if config.mtls_enabled {
            Some(Arc::new(IdentityProvider::new()?))
        } else {
            None
        };

        let message_encryption = if let Some(enc_config) = &config.encryption_config {
            Some(Arc::new(MessageEncryption::new(enc_config.clone())?))
        } else {
            None
        };

        let network_acl = if config.acl_enabled {
            if let Some(acl_path) = &config.acl_rules_path {
                Some(Arc::new(RwLock::new(NetworkAcl::from_file(acl_path)?)))
            } else {
                None
            }
        } else {
            None
        };

        let firewall = if config.firewall_enabled {
            if let Some(fw_config) = &config.firewall_config {
                Some(Arc::new(RwLock::new(
                    ApplicationFirewall::new(fw_config.clone())?
                )))
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            config,
            cert_manager,
            mtls_authenticator,
            identity_provider,
            message_encryption,
            network_acl,
            firewall,
        })
    }

    /// Get certificate manager
    pub fn cert_manager(&self) -> Option<Arc<RwLock<CertificateManager>>> {
        self.cert_manager.clone()
    }

    /// Get mTLS authenticator
    pub fn mtls_authenticator(&self) -> Option<Arc<MtlsAuthenticator>> {
        self.mtls_authenticator.clone()
    }

    /// Get identity provider
    pub fn identity_provider(&self) -> Option<Arc<IdentityProvider>> {
        self.identity_provider.clone()
    }

    /// Get message encryption
    pub fn message_encryption(&self) -> Option<Arc<MessageEncryption>> {
        self.message_encryption.clone()
    }

    /// Get network ACL
    pub fn network_acl(&self) -> Option<Arc<RwLock<NetworkAcl>>> {
        self.network_acl.clone()
    }

    /// Get firewall
    pub fn firewall(&self) -> Option<Arc<RwLock<ApplicationFirewall>>> {
        self.firewall.clone()
    }

    /// Check if connection is allowed
    pub async fn check_connection(&self, source_ip: std::net::IpAddr) -> Result<bool> {
        // Check ACL
        if let Some(acl) = &self.network_acl {
            let mut acl_guard = acl.write().await;
            if !acl_guard.is_allowed(source_ip)? {
                return Ok(false);
            }
        }

        // Check firewall
        if let Some(firewall) = &self.firewall {
            let mut firewall_guard = firewall.write().await;
            if !firewall_guard.allow_connection(source_ip).await? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Encrypt message
    pub fn encrypt_message(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        if let Some(encryption) = &self.message_encryption {
            encryption.encrypt(plaintext)
        } else {
            Ok(plaintext.to_vec())
        }
    }

    /// Decrypt message
    pub fn decrypt_message(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if let Some(encryption) = &self.message_encryption {
            encryption.decrypt(ciphertext)
        } else {
            Ok(ciphertext.to_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(!config.mtls_enabled);
        assert!(!config.firewall_enabled);
        assert!(!config.acl_enabled);
        assert!(config.audit_enabled);
    }

    #[test]
    fn test_security_config_builder() {
        let config = SecurityConfig::new()
            .with_mtls_enabled(true)
            .with_firewall_enabled(true)
            .with_acl_enabled(true);

        assert!(config.mtls_enabled);
        assert!(config.firewall_enabled);
        assert!(config.acl_enabled);
    }

    #[test]
    fn test_auth_context() {
        let mut ctx = AuthContext::new("node-1".to_string());
        ctx.add_role("admin".to_string());
        ctx.add_attribute("region".to_string(), "us-west-2".to_string());

        assert_eq!(ctx.principal, "node-1");
        assert_eq!(ctx.roles.len(), 1);
        assert_eq!(ctx.attributes.len(), 1);
        assert!(!ctx.is_expired());
    }
}
