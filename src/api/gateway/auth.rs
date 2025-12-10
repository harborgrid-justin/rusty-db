// Gateway Module
//
// Part of the API Gateway and Security system for RustyDB

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::{Arc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};
use rsa::{Pkcs1v15Sign, RsaPublicKey, pkcs1::DecodeRsaPublicKey};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use uuid::Uuid;
use reqwest::Client;
use x509_parser::prelude::*;
use sha1::Sha1;

use crate::error::DbError;
use super::types::*;

// ============================================================================
// Authentication System - JWT, OAuth, API Keys, mTLS
// ============================================================================

// Authentication manager
pub struct AuthenticationManager {
    // JWT validator
    jwt_validator: Arc<JwtValidator>,
    // OAuth provider
    #[allow(dead_code)]
    oauth_provider: Arc<OAuthProvider>,
    // API key store
    api_key_store: Arc<RwLock<ApiKeyStore>>,
    // Session manager
    session_manager: Arc<SessionManager>,
    // mTLS validator
    #[allow(dead_code)]
    mtls_validator: Arc<MtlsValidator>,
    // MFA manager
    #[allow(dead_code)]
    mfa_manager: Arc<MfaManager>,
}

// JWT validator
pub struct JwtValidator {
    // Signing keys (kid -> key)
    signing_keys: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    // Issuer
    issuer: String,
    // Audience
    audience: Vec<String>,
    // Algorithm
    algorithm: JwtAlgorithm,
    // Token expiration tolerance (seconds)
    expiration_tolerance: u64,
}

// JWT algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JwtAlgorithm {
    HS256,
    HS384,
    HS512,
    RS256,
    RS384,
    RS512,
    ES256,
    ES384,
    ES512,
}

// JWT claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    // Issuer
    pub iss: String,
    // Subject
    pub sub: String,
    // Audience
    pub aud: Vec<String>,
    // Expiration time
    pub exp: u64,
    // Not before
    pub nbf: Option<u64>,
    // Issued at
    pub iat: u64,
    // JWT ID
    pub jti: String,
    // Custom claims
    pub custom: HashMap<String, serde_json::Value>,
}

// OAuth 2.0 provider
pub struct OAuthProvider {
    // Provider configuration
    config: Arc<RwLock<OAuthConfig>>,
    // Token cache
    token_cache: Arc<RwLock<HashMap<String, OAuthToken>>>,
}

// OAuth configuration
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    // Authorization endpoint
    pub auth_endpoint: String,
    // Token endpoint
    pub token_endpoint: String,
    // Userinfo endpoint
    pub userinfo_endpoint: Option<String>,
    // Client ID
    pub client_id: String,
    // Client secret
    pub client_secret: String,
    // Redirect URI
    pub redirect_uri: String,
    // Scopes
    pub scopes: Vec<String>,
    // OIDC discovery URL
    pub discovery_url: Option<String>,
}

// OAuth token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    // Access token
    pub access_token: String,
    // Token type
    pub token_type: String,
    // Expires in (seconds)
    pub expires_in: u64,
    // Refresh token
    pub refresh_token: Option<String>,
    // Scope
    pub scope: Option<String>,
    // ID token (OIDC)
    pub id_token: Option<String>,
}

// API key store
pub struct ApiKeyStore {
    // API keys (key -> metadata)
    keys: HashMap<String, ApiKeyMetadata>,
}

// API key metadata
#[derive(Debug, Clone)]
pub struct ApiKeyMetadata {
    // Key ID
    pub key_id: String,
    // User ID
    pub user_id: String,
    // Key hash (never store plain keys)
    pub key_hash: Vec<u8>,
    // Created at
    pub created_at: SystemTime,
    // Expires at
    pub expires_at: Option<SystemTime>,
    // Enabled
    pub enabled: bool,
    // Scopes/permissions
    pub scopes: Vec<String>,
    // Last used
    pub last_used: Option<SystemTime>,
    // Usage count
    pub usage_count: u64,
}

// Session manager
pub struct SessionManager {
    // Active sessions
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    // Session timeout (seconds)
    #[allow(dead_code)]
    session_timeout: u64,
}

// User session
#[derive(Debug, Clone)]
pub struct Session {
    // Session ID
    pub session_id: String,
    // User ID
    pub user_id: String,
    // Username
    pub username: String,
    // Roles
    pub roles: Vec<String>,
    // Permissions
    pub permissions: Vec<String>,
    // Created at
    pub created_at: SystemTime,
    // Last accessed
    pub last_accessed: SystemTime,
    // Expires at
    pub expires_at: SystemTime,
    // IP address
    pub ip_address: IpAddr,
    // User agent
    pub user_agent: Option<String>,
    // Additional attributes
    pub attributes: HashMap<String, String>,
}

// mTLS validator
pub struct MtlsValidator {
    // Trusted CA certificates
    trusted_cas: Arc<RwLock<Vec<Vec<u8>>>>,
    // Certificate revocation list
    crl: Arc<RwLock<HashSet<Vec<u8>>>>,
}

// MFA manager
pub struct MfaManager {
    // TOTP secrets
    totp_secrets: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    // Backup codes
    #[allow(dead_code)]
    backup_codes: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl AuthenticationManager {
    // Create new authentication manager
    pub fn new() -> Self {
        Self {
            jwt_validator: Arc::new(JwtValidator::new("rustydb".to_string())),
            oauth_provider: Arc::new(OAuthProvider::new()),
            api_key_store: Arc::new(RwLock::new(ApiKeyStore::new())),
            session_manager: Arc::new(SessionManager::new()),
            mtls_validator: Arc::new(MtlsValidator::new()),
            mfa_manager: Arc::new(MfaManager::new()),
        }
    }

    // Authenticate request
    pub async fn authenticate(&self, request: &ApiRequest) -> Result<Session, DbError> {
        // Try different authentication methods in order

        // 1. Try JWT bearer token
        if let Some(auth_header) = request.headers.get("Authorization") {
            if auth_header.starts_with("Bearer ") {
                let token = &auth_header[7..];
                if let Ok(claims) = self.jwt_validator.validate(token) {
                    return self.create_session_from_jwt(claims, request);
                }
            }
        }

        // 2. Try API key
        if let Some(api_key) = request.headers.get("X-API-Key") {
            if let Ok(session) = self.authenticate_api_key(api_key, request).await {
                return Ok(session);
            }
        }

        // 3. Try session cookie
        if let Some(cookie) = request.headers.get("Cookie") {
            if let Some(session_id) = self.extract_session_id(cookie) {
                if let Some(session) = self.session_manager.get_session(&session_id) {
                    return Ok(session);
                }
            }
        }

        Err(DbError::InvalidOperation("Authentication failed".to_string()))
    }

    // Create session from JWT claims
    fn create_session_from_jwt(&self, claims: JwtClaims, request: &ApiRequest) -> Result<Session, DbError> {
        let session = Session {
            session_id: Uuid::new_v4().to_string(),
            user_id: claims.sub.clone(),
            username: claims.custom.get("username")
                .and_then(|v| v.as_str())
                .unwrap_or(&claims.sub)
                .to_string(),
            roles: claims.custom.get("roles")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            permissions: Vec::new(),
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            expires_at: UNIX_EPOCH + Duration::from_secs(claims.exp),
            ip_address: request.client_ip,
            user_agent: request.headers.get("User-Agent").cloned(),
            attributes: HashMap::new(),
        };

        Ok(session)
    }

    // Authenticate using API key
    async fn authenticate_api_key(&self, api_key: &str, request: &ApiRequest) -> Result<Session, DbError> {
        let key_store = self.api_key_store.read();

        // Hash the provided key
        let key_hash = Self::hash_api_key(api_key);

        // Find matching key
        for metadata in key_store.keys.values() {
            if metadata.key_hash == key_hash && metadata.enabled {
                // Check expiration
                if let Some(expires_at) = metadata.expires_at {
                    if SystemTime::now() > expires_at {
                        continue;
                    }
                }

                // Create session
                let session = Session {
                    session_id: Uuid::new_v4().to_string(),
                    user_id: metadata.user_id.clone(),
                    username: metadata.user_id.clone(),
                    roles: vec!["api_user".to_string()],
                    permissions: metadata.scopes.clone(),
                    created_at: SystemTime::now(),
                    last_accessed: SystemTime::now(),
                    expires_at: SystemTime::now() + Duration::from_secs(3600),
                    ip_address: request.client_ip,
                    user_agent: request.headers.get("User-Agent").cloned(),
                    attributes: HashMap::new(),
                };

                return Ok(session);
            }
        }

        Err(DbError::InvalidOperation("Invalid API key".to_string()))
    }

    // Extract session ID from cookie
    fn extract_session_id(&self, cookie: &str) -> Option<String> {
        for part in cookie.split(';') {
            let part = part.trim();
            if part.starts_with("session_id=") {
                return Some(part[11..].to_string());
            }
        }
        None
    }

    // Hash API key
    fn hash_api_key(key: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        hasher.finalize().to_vec()
    }

    // Generate new API key
    pub fn generate_api_key(&self, user_id: String, scopes: Vec<String>) -> Result<String, DbError> {
        let key = Uuid::new_v4().to_string();
        let key_hash = Self::hash_api_key(&key);

        let metadata = ApiKeyMetadata {
            key_id: Uuid::new_v4().to_string(),
            user_id,
            key_hash,
            created_at: SystemTime::now(),
            expires_at: None,
            enabled: true,
            scopes,
            last_used: None,
            usage_count: 0,
        };

        let mut key_store = self.api_key_store.write();
        key_store.keys.insert(metadata.key_id.clone(), metadata);

        Ok(key)
    }

    // Revoke API key
    pub fn revoke_api_key(&self, key_id: &str) -> bool {
        let mut key_store = self.api_key_store.write();
        if let Some(metadata) = key_store.keys.get_mut(key_id) {
            metadata.enabled = false;
            true
        } else {
            false
        }
    }

    // List API keys for user
    pub fn list_api_keys(&self, user_id: &str) -> Vec<ApiKeyMetadata> {
        let key_store = self.api_key_store.read();
        key_store.keys.values()
            .filter(|m| m.user_id == user_id)
            .cloned()
            .collect()
    }
}

impl JwtValidator {
    // Create new JWT validator
    pub fn new(issuer: String) -> Self {
        Self {
            signing_keys: Arc::new(RwLock::new(HashMap::new())),
            issuer,
            audience: vec!["rustydb".to_string()],
            algorithm: JwtAlgorithm::HS256,
            expiration_tolerance: 60,
        }
    }

    // Validate JWT token
    pub fn validate(&self, token: &str) -> Result<JwtClaims, DbError> {
        // Parse token
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(DbError::InvalidOperation("Invalid JWT format".to_string()));
        }

        // Decode header
        let header_data = BASE64.decode(parts[0])
            .map_err(|_| DbError::InvalidOperation("Invalid JWT header".to_string()))?;
        let header: serde_json::Value = serde_json::from_slice(&header_data)
            .map_err(|_| DbError::InvalidOperation("Invalid JWT header JSON".to_string()))?;

        let kid = header.get("kid").and_then(|v| v.as_str());

        // Decode payload
        let payload_data = BASE64.decode(parts[1])
            .map_err(|_| DbError::InvalidOperation("Invalid JWT payload".to_string()))?;
        let claims: JwtClaims = serde_json::from_slice(&payload_data)
            .map_err(|_| DbError::InvalidOperation("Invalid JWT claims".to_string()))?;

        // Verify signature
        let signature = BASE64.decode(parts[2])
            .map_err(|_| DbError::InvalidOperation("Invalid JWT signature".to_string()))?;

        let message = format!("{}.{}", parts[0], parts[1]);
        self.verify_signature(&message, &signature, kid)?;

        // Validate claims
        self.validate_claims(&claims)?;

        Ok(claims)
    }

    // Verify signature
    fn verify_signature(&self, message: &str, signature: &[u8], kid: Option<&str>) -> Result<(), DbError> {
        let keys = self.signing_keys.read();

        // If kid is provided, use that key
        if let Some(kid) = kid {
            if let Some(key) = keys.get(kid) {
                return self.verify_with_key(message, signature, key);
            } else {
                return Err(DbError::InvalidOperation(format!("Unknown key ID: {}", kid)));
            }
        }

        // If no kid, try all keys (inefficient but fallback)
        for key in keys.values() {
            if self.verify_with_key(message, signature, key).is_ok() {
                return Ok(());
            }
        }

        Err(DbError::InvalidOperation("Invalid signature".to_string()))
    }

    fn verify_with_key(&self, message: &str, signature: &[u8], key: &[u8]) -> Result<(), DbError> {
        match self.algorithm {
            JwtAlgorithm::HS256 => {
                type HmacSha256 = Hmac<Sha256>;
                let mut mac = HmacSha256::new_from_slice(key)
                    .map_err(|_| DbError::InvalidOperation("Invalid key length".to_string()))?;
                mac.update(message.as_bytes());
                mac.verify_slice(signature)
                    .map_err(|_| DbError::InvalidOperation("Invalid signature".to_string()))
            },
            JwtAlgorithm::RS256 => {
                let pub_key = RsaPublicKey::from_pkcs1_der(key)
                    .map_err(|_| DbError::InvalidOperation("Invalid RSA key".to_string()))?;
                let verifying_key = Pkcs1v15Sign::new::<Sha256>();
                pub_key.verify(verifying_key, message.as_bytes(), signature)
                    .map_err(|_| DbError::InvalidOperation("Invalid signature".to_string()))
            },
            _ => Err(DbError::InvalidOperation("Unsupported algorithm".to_string())),
        }
    }

    // Validate claims
    fn validate_claims(&self, claims: &JwtClaims) -> Result<(), DbError> {
        // Check issuer
        if claims.iss != self.issuer {
            return Err(DbError::InvalidOperation("Invalid issuer".to_string()));
        }

        // Check audience
        if !claims.aud.iter().any(|aud| self.audience.contains(aud)) {
            return Err(DbError::InvalidOperation("Invalid audience".to_string()));
        }

        // Check expiration
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if claims.exp + self.expiration_tolerance < now {
            return Err(DbError::InvalidOperation("Token expired".to_string()));
        }

        // Check not before
        if let Some(nbf) = claims.nbf {
            if nbf > now {
                return Err(DbError::InvalidOperation("Token not yet valid".to_string()));
            }
        }

        Ok(())
    }

    // Add signing key
    pub fn add_signing_key(&self, kid: String, key: Vec<u8>) {
        let mut keys = self.signing_keys.write();
        keys.insert(kid, key);
    }
}

impl OAuthProvider {
    // Create new OAuth provider
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(OAuthConfig::default())),
            token_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Configure OAuth provider
    pub fn configure(&self, config: OAuthConfig) {
        let mut cfg = self.config.write();
        *cfg = config;
    }

    // Exchange authorization code for token
    pub async fn exchange_code(&self, code: &str) -> Result<OAuthToken, DbError> {
        let config = self.config.read();
        let client = Client::new();

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &config.redirect_uri),
            ("client_id", &config.client_id),
            ("client_secret", &config.client_secret),
        ];

        let response = client.post(&config.token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| DbError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DbError::InvalidOperation(format!("OAuth error: {}", response.status())));
        }

        let token: OAuthToken = response.json()
            .await
            .map_err(|e| DbError::InvalidOperation(format!("Failed to parse token: {}", e)))?;

        // Cache token
        let mut cache = self.token_cache.write();
        cache.insert(token.access_token.clone(), token.clone());

        Ok(token)
    }

    // Refresh access token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthToken, DbError> {
        let config = self.config.read();
        let client = Client::new();

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &config.client_id),
            ("client_secret", &config.client_secret),
        ];

        let response = client.post(&config.token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| DbError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DbError::InvalidOperation(format!("OAuth error: {}", response.status())));
        }

        let token: OAuthToken = response.json()
            .await
            .map_err(|e| DbError::InvalidOperation(format!("Failed to parse token: {}", e)))?;

        // Cache token
        let mut cache = self.token_cache.write();
        cache.insert(token.access_token.clone(), token.clone());

        Ok(token)
    }
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            auth_endpoint: String::new(),
            token_endpoint: String::new(),
            userinfo_endpoint: None,
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: String::new(),
            scopes: Vec::new(),
            discovery_url: None,
        }
    }
}

impl ApiKeyStore {
    fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }
}

impl SessionManager {
    fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_timeout: 3600,
        }
    }

    // Get session by ID
    fn get_session(&self, session_id: &str) -> Option<Session> {
        let sessions = self.sessions.read();
        sessions.get(session_id).cloned()
    }

    // Create new session
    pub fn create_session(&self, session: Session) {
        let mut sessions = self.sessions.write();
        sessions.insert(session.session_id.clone(), session);
    }

    // Invalidate session
    pub fn invalidate_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write();
        sessions.remove(session_id).is_some()
    }

    // Cleanup expired sessions
    pub fn cleanup_expired_sessions(&self) {
        let mut sessions = self.sessions.write();
        let now = SystemTime::now();
        sessions.retain(|_, session| session.expires_at > now);
    }
}

impl MtlsValidator {
    fn new() -> Self {
        Self {
            trusted_cas: Arc::new(RwLock::new(Vec::new())),
            crl: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    // Add trusted CA certificate
    pub fn add_trusted_ca(&self, cert: Vec<u8>) {
        let mut cas = self.trusted_cas.write();
        cas.push(cert);
    }

    // Validate client certificate
    pub fn validate_certificate(&self, cert: &[u8]) -> Result<bool, DbError> {
        // Parse certificate
        let (_, x509) = X509Certificate::from_der(cert)
            .map_err(|_| DbError::InvalidOperation("Invalid certificate format".to_string()))?;

        // Check validity period
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        if now < x509.validity().not_before.timestamp() || now > x509.validity().not_after.timestamp() {
            return Ok(false);
        }

        // Check if signed by trusted CA
        let cas = self.trusted_cas.read();
        let mut trusted = false;
        for ca_cert_der in cas.iter() {
            if let Ok((_, ca_cert)) = X509Certificate::from_der(ca_cert_der) {
                if x509.verify_signature(Some(&ca_cert.subject_pki)).is_ok() {
                    trusted = true;
                    break;
                }
            }
        }

        if !trusted {
            return Ok(false);
        }

        // Check CRL
        let serial = x509.serial.to_bytes_be();
        let crl = self.crl.read();
        if crl.contains(&serial) {
            return Ok(false);
        }

        Ok(true)
    }
}

impl MfaManager {
    fn new() -> Self {
        Self {
            totp_secrets: Arc::new(RwLock::new(HashMap::new())),
            backup_codes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Generate TOTP secret for user
    pub fn generate_totp_secret(&self, user_id: String) -> Vec<u8> {
        let mut secret = vec![0u8; 32];
        for i in 0..32 {
            secret[i] = rand::random();
        }

        let mut secrets = self.totp_secrets.write();
        secrets.insert(user_id, secret.clone());

        secret
    }

    // Verify TOTP code
    pub fn verify_totp(&self, user_id: &str, code: &str) -> bool {
        let secrets = self.totp_secrets.read();
        if let Some(secret) = secrets.get(user_id) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let time_step = now / 30;

            // Check current time step and adjacent ones for clock skew
            for i in -1..=1 {
                let t = (time_step as i64 + i) as u64;
                if self.generate_totp(secret, t) == code {
                    return true;
                }
            }
        }
        false
    }

    fn generate_totp(&self, secret: &[u8], time_step: u64) -> String {
        type HmacSha1 = Hmac<Sha1>;
        let mut mac = HmacSha1::new_from_slice(secret)
            .expect("HMAC can take any key length");
        mac.update(&time_step.to_be_bytes());
        let result = mac.finalize().into_bytes();

        let offset = (result[19] & 0xf) as usize;
        let binary = ((result[offset] & 0x7f) as u32) << 24
            | (result[offset + 1] as u32) << 16
            | (result[offset + 2] as u32) << 8
            | (result[offset + 3] as u32);

        let otp = binary % 1_000_000;
        format!("{:06}", otp)
    }
}
