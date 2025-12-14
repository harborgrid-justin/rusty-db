// TLS configuration and management
//
// This module provides TLS 1.2/1.3 support using rustls for secure communication.

use crate::error::{DbError, Result};
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::path::PathBuf;
use std::sync::Arc;

/// TLS protocol version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsVersion {
    /// TLS 1.2
    Tls12,
    /// TLS 1.3
    Tls13,
}

impl TlsVersion {
    /// Convert to rustls protocol version
    pub fn to_rustls_version(&self) -> &'static rustls::SupportedProtocolVersion {
        match self {
            TlsVersion::Tls12 => &rustls::version::TLS12,
            TlsVersion::Tls13 => &rustls::version::TLS13,
        }
    }
}

/// Cipher suite configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherSuite {
    /// TLS 1.3 AES-256-GCM-SHA384
    Tls13Aes256GcmSha384,
    /// TLS 1.3 AES-128-GCM-SHA256
    Tls13Aes128GcmSha256,
    /// TLS 1.3 ChaCha20-Poly1305-SHA256
    Tls13Chacha20Poly1305Sha256,
    /// TLS 1.2 ECDHE-RSA-AES256-GCM-SHA384
    Tls12EcdheRsaAes256GcmSha384,
    /// TLS 1.2 ECDHE-RSA-AES128-GCM-SHA256
    Tls12EcdheRsaAes128GcmSha256,
}

impl CipherSuite {
    /// Convert to rustls cipher suite
    pub fn to_rustls_suite(&self) -> rustls::SupportedCipherSuite {
        match self {
            CipherSuite::Tls13Aes256GcmSha384 => {
                rustls::crypto::ring::cipher_suite::TLS13_AES_256_GCM_SHA384
            }
            CipherSuite::Tls13Aes128GcmSha256 => {
                rustls::crypto::ring::cipher_suite::TLS13_AES_128_GCM_SHA256
            }
            CipherSuite::Tls13Chacha20Poly1305Sha256 => {
                rustls::crypto::ring::cipher_suite::TLS13_CHACHA20_POLY1305_SHA256
            }
            CipherSuite::Tls12EcdheRsaAes256GcmSha384 => {
                rustls::crypto::ring::cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384
            }
            CipherSuite::Tls12EcdheRsaAes128GcmSha256 => {
                rustls::crypto::ring::cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
            }
        }
    }
}

/// TLS configuration
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to certificate file (PEM format)
    pub cert_path: PathBuf,

    /// Path to private key file (PEM format)
    pub key_path: PathBuf,

    /// Path to CA certificate file (PEM format)
    pub ca_path: Option<PathBuf>,

    /// Verify peer certificate
    pub verify_peer: bool,

    /// Minimum TLS version
    pub min_version: TlsVersion,

    /// Maximum TLS version (None = use latest)
    pub max_version: Option<TlsVersion>,

    /// Allowed cipher suites
    pub cipher_suites: Vec<CipherSuite>,

    /// Server name for SNI
    pub server_name: Option<String>,

    /// Enable ALPN (Application-Layer Protocol Negotiation)
    pub alpn_protocols: Vec<String>,

    /// Enable session resumption
    pub session_resumption: bool,

    /// Session ticket lifetime (seconds)
    pub ticket_lifetime: u32,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: PathBuf::from("./certs/server.crt"),
            key_path: PathBuf::from("./certs/server.key"),
            ca_path: Some(PathBuf::from("./certs/ca.crt")),
            verify_peer: true,
            min_version: TlsVersion::Tls12,
            max_version: Some(TlsVersion::Tls13),
            cipher_suites: vec![
                CipherSuite::Tls13Aes256GcmSha384,
                CipherSuite::Tls13Aes128GcmSha256,
                CipherSuite::Tls13Chacha20Poly1305Sha256,
            ],
            server_name: None,
            alpn_protocols: vec!["rustydb/1.0".to_string()],
            session_resumption: true,
            ticket_lifetime: 86400, // 24 hours
        }
    }
}

impl TlsConfig {
    /// Create a new TLS configuration
    pub fn new(cert_path: PathBuf, key_path: PathBuf) -> Self {
        Self {
            cert_path,
            key_path,
            ..Default::default()
        }
    }

    /// Set CA certificate path
    pub fn with_ca_path(mut self, ca_path: PathBuf) -> Self {
        self.ca_path = Some(ca_path);
        self
    }

    /// Set peer verification
    pub fn with_verify_peer(mut self, verify: bool) -> Self {
        self.verify_peer = verify;
        self
    }

    /// Set minimum TLS version
    pub fn with_min_version(mut self, version: TlsVersion) -> Self {
        self.min_version = version;
        self
    }

    /// Set cipher suites
    pub fn with_cipher_suites(mut self, suites: Vec<CipherSuite>) -> Self {
        self.cipher_suites = suites;
        self
    }

    /// Set server name for SNI
    pub fn with_server_name(mut self, name: String) -> Self {
        self.server_name = Some(name);
        self
    }

    /// Load certificate chain from file
    pub fn load_certs(&self) -> Result<Vec<CertificateDer<'static>>> {
        let cert_file = std::fs::File::open(&self.cert_path)
            .map_err(|e| DbError::Configuration(format!("Failed to open cert file: {}", e)))?;

        let mut reader = std::io::BufReader::new(cert_file);
        let certs = rustls_pemfile::certs(&mut reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| DbError::Configuration(format!("Failed to parse certs: {}", e)))?;

        Ok(certs)
    }

    /// Load private key from file
    pub fn load_private_key(&self) -> Result<PrivateKeyDer<'static>> {
        let key_file = std::fs::File::open(&self.key_path)
            .map_err(|e| DbError::Configuration(format!("Failed to open key file: {}", e)))?;

        let mut reader = std::io::BufReader::new(key_file);

        // Try to parse as PKCS8 first
        let mut keys = rustls_pemfile::pkcs8_private_keys(&mut reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| DbError::Configuration(format!("Failed to parse PKCS8 key: {}", e)))?;

        if !keys.is_empty() {
            return Ok(PrivateKeyDer::Pkcs8(keys.remove(0)));
        }

        // Reset reader
        let key_file = std::fs::File::open(&self.key_path)
            .map_err(|e| DbError::Configuration(format!("Failed to open key file: {}", e)))?;
        let mut reader = std::io::BufReader::new(key_file);

        // Try RSA private key
        let mut keys = rustls_pemfile::rsa_private_keys(&mut reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| DbError::Configuration(format!("Failed to parse RSA key: {}", e)))?;

        if !keys.is_empty() {
            return Ok(PrivateKeyDer::Pkcs1(keys.remove(0)));
        }

        Err(DbError::Configuration(
            "No private key found in key file".to_string(),
        ))
    }

    /// Load CA certificates from file
    pub fn load_ca_certs(&self) -> Result<Vec<CertificateDer<'static>>> {
        if let Some(ca_path) = &self.ca_path {
            let ca_file = std::fs::File::open(ca_path)
                .map_err(|e| DbError::Configuration(format!("Failed to open CA file: {}", e)))?;

            let mut reader = std::io::BufReader::new(ca_file);
            let certs = rustls_pemfile::certs(&mut reader)
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| DbError::Configuration(format!("Failed to parse CA certs: {}", e)))?;

            Ok(certs)
        } else {
            Ok(Vec::new())
        }
    }

    /// Create rustls server configuration
    pub fn build_server_config(&self) -> Result<Arc<rustls::ServerConfig>> {
        let certs = self.load_certs()?;
        let key = self.load_private_key()?;

        let mut config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| DbError::Configuration(format!("Failed to set certificate: {}", e)))?;

        // Set ALPN protocols using helper
        self.configure_alpn(&mut config.alpn_protocols);

        Ok(Arc::new(config))
    }

    /// Create rustls client configuration
    pub fn build_client_config(&self) -> Result<Arc<rustls::ClientConfig>> {
        let mut root_store = rustls::RootCertStore::empty();

        // Load CA certificates
        let ca_certs = self.load_ca_certs()?;
        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| DbError::Configuration(format!("Failed to add CA cert: {}", e)))?;
        }

        let mut config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        // Set ALPN protocols using helper
        self.configure_alpn(&mut config.alpn_protocols);

        Ok(Arc::new(config))
    }

    /// Helper to configure ALPN protocols (eliminates duplication)
    fn configure_alpn(&self, alpn_protocols: &mut Vec<Vec<u8>>) {
        if !self.alpn_protocols.is_empty() {
            *alpn_protocols = self
                .alpn_protocols
                .iter()
                .map(|p| p.as_bytes().to_vec())
                .collect();
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if !self.cert_path.exists() {
            return Err(DbError::Configuration(format!(
                "Certificate file not found: {:?}",
                self.cert_path
            )));
        }

        if !self.key_path.exists() {
            return Err(DbError::Configuration(format!(
                "Key file not found: {:?}",
                self.key_path
            )));
        }

        if let Some(ca_path) = &self.ca_path {
            if !ca_path.exists() {
                return Err(DbError::Configuration(format!(
                    "CA file not found: {:?}",
                    ca_path
                )));
            }
        }

        if self.cipher_suites.is_empty() {
            return Err(DbError::Configuration(
                "At least one cipher suite must be specified".to_string(),
            ));
        }

        Ok(())
    }
}

/// TLS manager for handling TLS connections
pub struct TlsManager {
    /// Configuration
    config: TlsConfig,

    /// Server configuration
    server_config: Arc<rustls::ServerConfig>,

    /// Client configuration
    client_config: Arc<rustls::ClientConfig>,
}

impl TlsManager {
    /// Create a new TLS manager
    pub fn new(config: TlsConfig) -> Result<Self> {
        config.validate()?;

        let server_config = config.build_server_config()?;
        let client_config = config.build_client_config()?;

        Ok(Self {
            config,
            server_config,
            client_config,
        })
    }

    /// Get server configuration
    pub fn server_config(&self) -> Arc<rustls::ServerConfig> {
        self.server_config.clone()
    }

    /// Get client configuration
    pub fn client_config(&self) -> Arc<rustls::ClientConfig> {
        self.client_config.clone()
    }

    /// Get TLS configuration
    pub fn config(&self) -> &TlsConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_version() {
        assert_eq!(
            TlsVersion::Tls12.to_rustls_version(),
            &rustls::version::TLS12
        );
        assert_eq!(
            TlsVersion::Tls13.to_rustls_version(),
            &rustls::version::TLS13
        );
    }

    #[test]
    fn test_tls_config_default() {
        let config = TlsConfig::default();
        assert_eq!(config.min_version, TlsVersion::Tls12);
        assert_eq!(config.max_version, Some(TlsVersion::Tls13));
        assert!(config.verify_peer);
        assert!(!config.cipher_suites.is_empty());
    }

    #[test]
    fn test_tls_config_builder() {
        let config = TlsConfig::new(PathBuf::from("./test.crt"), PathBuf::from("./test.key"))
            .with_verify_peer(false)
            .with_min_version(TlsVersion::Tls13)
            .with_server_name("example.com".to_string());

        assert_eq!(config.cert_path, PathBuf::from("./test.crt"));
        assert_eq!(config.key_path, PathBuf::from("./test.key"));
        assert!(!config.verify_peer);
        assert_eq!(config.min_version, TlsVersion::Tls13);
        assert_eq!(config.server_name, Some("example.com".to_string()));
    }
}
