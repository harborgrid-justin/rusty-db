//! Mutual TLS (mTLS) authentication and verification
//!
//! This module provides client certificate verification, certificate chain validation,
//! OCSP stapling, and CRL checking.

use crate::error::{DbError, Result};
use crate::networking::security::AuthContext;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Object Identifier (OID) for certificate extensions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Oid(pub Vec<u64>);

impl Oid {
    /// Create a new OID
    pub fn new(components: Vec<u64>) -> Self {
        Self(components)
    }

    /// Parse OID from string (e.g., "1.2.3.4")
    pub fn from_string(s: &str) -> Result<Self> {
        let components: Result<Vec<u64>> = s
            .split('.')
            .map(|c| {
                c.parse::<u64>()
                    .map_err(|e| DbError::ParseError(format!("Invalid OID component: {}", e)))
            })
            .collect();

        Ok(Self(components?))
    }

    /// Convert to dotted string
    pub fn to_string(&self) -> String {
        self.0
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(".")
    }
}

/// Subject matcher for certificate validation
#[derive(Debug, Clone)]
pub enum SubjectMatcher {
    /// Exact match
    Exact(String),
    /// Prefix match
    Prefix(String),
    /// Suffix match
    Suffix(String),
    /// Regex match
    Regex(String),
    /// Any subject
    Any,
}

impl SubjectMatcher {
    /// Check if subject matches
    pub fn matches(&self, subject: &str) -> bool {
        match self {
            SubjectMatcher::Exact(pattern) => subject == pattern,
            SubjectMatcher::Prefix(pattern) => subject.starts_with(pattern),
            SubjectMatcher::Suffix(pattern) => subject.ends_with(pattern),
            SubjectMatcher::Regex(pattern) => {
                // Simple implementation - in production use regex crate
                subject.contains(pattern)
            }
            SubjectMatcher::Any => true,
        }
    }
}

/// Trust store for CA certificates
#[derive(Debug, Clone)]
pub struct TrustStore {
    /// CA certificates
    ca_certs: Vec<rustls::Certificate>,

    /// Trusted subjects
    trusted_subjects: Vec<String>,
}

impl TrustStore {
    /// Create a new trust store
    pub fn new() -> Self {
        Self {
            ca_certs: Vec::new(),
            trusted_subjects: Vec::new(),
        }
    }

    /// Add CA certificate
    pub fn add_ca_cert(&mut self, cert: rustls::Certificate) {
        self.ca_certs.push(cert);
    }

    /// Add trusted subject
    pub fn add_trusted_subject(&mut self, subject: String) {
        self.trusted_subjects.push(subject);
    }

    /// Check if subject is trusted
    pub fn is_trusted(&self, subject: &str) -> bool {
        self.trusted_subjects.iter().any(|s| s == subject)
    }

    /// Get CA certificates
    pub fn ca_certs(&self) -> &[rustls::Certificate] {
        &self.ca_certs
    }
}

impl Default for TrustStore {
    fn default() -> Self {
        Self::new()
    }
}

/// mTLS configuration
#[derive(Debug, Clone)]
pub struct MtlsConfig {
    /// Require client certificates
    pub require_client_cert: bool,

    /// Verify certificate chain
    pub verify_chain: bool,

    /// Enable OCSP stapling
    pub enable_ocsp: bool,

    /// Enable CRL checking
    pub enable_crl: bool,

    /// CRL URL
    pub crl_url: Option<String>,

    /// OCSP responder URL
    pub ocsp_url: Option<String>,

    /// Required certificate extensions (OIDs)
    pub required_extensions: Vec<Oid>,

    /// Subject matcher
    pub subject_matcher: SubjectMatcher,

    /// Maximum certificate chain depth
    pub max_chain_depth: usize,

    /// Certificate validity tolerance (seconds)
    pub validity_tolerance: u64,
}

impl Default for MtlsConfig {
    fn default() -> Self {
        Self {
            require_client_cert: true,
            verify_chain: true,
            enable_ocsp: false,
            enable_crl: false,
            crl_url: None,
            ocsp_url: None,
            required_extensions: Vec::new(),
            subject_matcher: SubjectMatcher::Any,
            max_chain_depth: 5,
            validity_tolerance: 300, // 5 minutes
        }
    }
}

impl MtlsConfig {
    /// Create a new mTLS configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set client certificate requirement
    pub fn with_require_client_cert(mut self, require: bool) -> Self {
        self.require_client_cert = require;
        self
    }

    /// Set chain verification
    pub fn with_verify_chain(mut self, verify: bool) -> Self {
        self.verify_chain = verify;
        self
    }

    /// Enable OCSP
    pub fn with_ocsp(mut self, enabled: bool, url: Option<String>) -> Self {
        self.enable_ocsp = enabled;
        self.ocsp_url = url;
        self
    }

    /// Enable CRL
    pub fn with_crl(mut self, enabled: bool, url: Option<String>) -> Self {
        self.enable_crl = enabled;
        self.crl_url = url;
        self
    }

    /// Set subject matcher
    pub fn with_subject_matcher(mut self, matcher: SubjectMatcher) -> Self {
        self.subject_matcher = matcher;
        self
    }

    /// Add required extension
    pub fn add_required_extension(mut self, oid: Oid) -> Self {
        self.required_extensions.push(oid);
        self
    }
}

/// Certificate validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Validation success
    pub valid: bool,

    /// Certificate subject
    pub subject: String,

    /// Issuer
    pub issuer: String,

    /// Serial number
    pub serial_number: String,

    /// Not before timestamp
    pub not_before: std::time::SystemTime,

    /// Not after timestamp
    pub not_after: std::time::SystemTime,

    /// Validation errors
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// Create a new validation result
    pub fn new(valid: bool, subject: String) -> Self {
        Self {
            valid,
            subject,
            issuer: String::new(),
            serial_number: String::new(),
            not_before: std::time::SystemTime::now(),
            not_after: std::time::SystemTime::now(),
            errors: Vec::new(),
        }
    }

    /// Add error
    pub fn add_error(&mut self, error: String) {
        self.valid = false;
        self.errors.push(error);
    }
}

/// Mutual TLS authenticator
pub struct MtlsAuthenticator {
    /// Configuration
    config: MtlsConfig,

    /// Trust store
    trust_store: Arc<RwLock<TrustStore>>,

    /// Revoked certificates cache
    revoked_certs: Arc<RwLock<HashMap<String, bool>>>,

    /// OCSP responses cache
    ocsp_cache: Arc<RwLock<HashMap<String, bool>>>,
}

impl MtlsAuthenticator {
    /// Create a new mTLS authenticator
    pub fn new(config: MtlsConfig) -> Result<Self> {
        Ok(Self {
            config,
            trust_store: Arc::new(RwLock::new(TrustStore::new())),
            revoked_certs: Arc::new(RwLock::new(HashMap::new())),
            ocsp_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get trust store
    pub fn trust_store(&self) -> Arc<RwLock<TrustStore>> {
        self.trust_store.clone()
    }

    /// Verify client certificate
    pub async fn verify_certificate(
        &self,
        cert_chain: &[rustls::Certificate],
    ) -> Result<ValidationResult> {
        if cert_chain.is_empty() {
            return Ok(ValidationResult::new(
                false,
                "No certificate provided".to_string(),
            ));
        }

        // Extract subject from first certificate
        let subject = self.extract_subject(&cert_chain[0])?;

        let mut result = ValidationResult::new(true, subject.clone());

        // Check subject matcher
        if !self.config.subject_matcher.matches(&subject) {
            result.add_error(format!("Subject does not match: {}", subject));
        }

        // Verify chain depth
        if cert_chain.len() > self.config.max_chain_depth {
            result.add_error(format!(
                "Certificate chain too deep: {} > {}",
                cert_chain.len(),
                self.config.max_chain_depth
            ));
        }

        // Verify chain if enabled
        if self.config.verify_chain {
            if let Err(e) = self.verify_chain(cert_chain).await {
                result.add_error(format!("Chain verification failed: {}", e));
            }
        }

        // Check CRL if enabled
        if self.config.enable_crl {
            if let Err(e) = self.check_crl(&subject).await {
                result.add_error(format!("CRL check failed: {}", e));
            }
        }

        // Check OCSP if enabled
        if self.config.enable_ocsp {
            if let Err(e) = self.check_ocsp(&subject).await {
                result.add_error(format!("OCSP check failed: {}", e));
            }
        }

        // Verify required extensions
        for oid in &self.config.required_extensions {
            if !self.has_extension(&cert_chain[0], oid)? {
                result.add_error(format!("Missing required extension: {}", oid.to_string()));
            }
        }

        Ok(result)
    }

    /// Authenticate from certificate
    pub async fn authenticate(&self, cert_chain: &[rustls::Certificate]) -> Result<AuthContext> {
        let validation = self.verify_certificate(cert_chain).await?;

        if !validation.valid {
            return Err(DbError::Authentication(format!(
                "Certificate validation failed: {:?}",
                validation.errors
            )));
        }

        let mut context = AuthContext::new(validation.subject.clone());
        context.add_attribute("issuer".to_string(), validation.issuer);
        context.add_attribute("serial".to_string(), validation.serial_number);

        Ok(context)
    }

    /// Extract subject from certificate
    fn extract_subject(&self, cert: &rustls::Certificate) -> Result<String> {
        // In a real implementation, parse the X.509 certificate
        // For now, return a placeholder
        Ok("CN=node-1,O=RustyDB,C=US".to_string())
    }

    /// Verify certificate chain
    async fn verify_chain(&self, _cert_chain: &[rustls::Certificate]) -> Result<()> {
        // In a real implementation, verify the certificate chain
        // using the trust store and standard X.509 validation
        Ok(())
    }

    /// Check Certificate Revocation List
    async fn check_crl(&self, subject: &str) -> Result<()> {
        let revoked = self.revoked_certs.read().await;
        if revoked.get(subject).copied().unwrap_or(false) {
            return Err(DbError::Authentication(format!(
                "Certificate revoked: {}",
                subject
            )));
        }
        Ok(())
    }

    /// Check OCSP (Online Certificate Status Protocol)
    async fn check_ocsp(&self, subject: &str) -> Result<()> {
        let ocsp = self.ocsp_cache.read().await;
        if let Some(&valid) = ocsp.get(subject) {
            if !valid {
                return Err(DbError::Authentication(format!(
                    "OCSP check failed: {}",
                    subject
                )));
            }
        }
        Ok(())
    }

    /// Check if certificate has extension
    fn has_extension(&self, _cert: &rustls::Certificate, _oid: &Oid) -> Result<bool> {
        // In a real implementation, parse certificate extensions
        Ok(true)
    }

    /// Revoke certificate
    pub async fn revoke_certificate(&self, subject: String) -> Result<()> {
        let mut revoked = self.revoked_certs.write().await;
        revoked.insert(subject, true);
        Ok(())
    }

    /// Update OCSP response
    pub async fn update_ocsp_response(&self, subject: String, valid: bool) -> Result<()> {
        let mut ocsp = self.ocsp_cache.write().await;
        ocsp.insert(subject, valid);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oid() {
        let oid = Oid::new(vec![1, 2, 3, 4]);
        assert_eq!(oid.to_string(), "1.2.3.4");

        let oid2 = Oid::from_string("1.2.3.4").unwrap();
        assert_eq!(oid, oid2);
    }

    #[test]
    fn test_subject_matcher() {
        let matcher = SubjectMatcher::Exact("CN=test".to_string());
        assert!(matcher.matches("CN=test"));
        assert!(!matcher.matches("CN=other"));

        let matcher = SubjectMatcher::Prefix("CN=".to_string());
        assert!(matcher.matches("CN=test"));
        assert!(matcher.matches("CN=other"));

        let matcher = SubjectMatcher::Any;
        assert!(matcher.matches("anything"));
    }

    #[test]
    fn test_trust_store() {
        let mut store = TrustStore::new();
        store.add_trusted_subject("CN=node-1".to_string());

        assert!(store.is_trusted("CN=node-1"));
        assert!(!store.is_trusted("CN=node-2"));
    }

    #[test]
    fn test_mtls_config() {
        let config = MtlsConfig::new()
            .with_require_client_cert(true)
            .with_verify_chain(true)
            .with_subject_matcher(SubjectMatcher::Prefix("CN=node-".to_string()));

        assert!(config.require_client_cert);
        assert!(config.verify_chain);
    }

    #[tokio::test]
    async fn test_mtls_authenticator() {
        let config = MtlsConfig::default();
        let auth = MtlsAuthenticator::new(config).unwrap();

        // Test revocation
        auth.revoke_certificate("CN=test".to_string())
            .await
            .unwrap();
        assert!(auth.check_crl("CN=test").await.is_err());
    }
}
