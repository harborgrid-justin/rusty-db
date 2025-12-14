// Node identity and verification
//
// This module provides node identity verification, SPIFFE/SPIRE integration,
// and JWT tokens for service mesh integration.

use crate::error::{DbError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// Node identity
#[derive(Debug, Clone)]
pub struct NodeIdentity {
    /// Node ID
    pub node_id: String,

    /// Node name
    pub node_name: String,

    /// Region
    pub region: String,

    /// Zone
    pub zone: String,

    /// Role
    pub role: String,

    /// Tags
    pub tags: HashMap<String, String>,

    /// SPIFFE ID
    pub spiffe_id: Option<String>,

    /// Certificate subject
    pub cert_subject: Option<String>,

    /// Creation timestamp
    pub created_at: SystemTime,

    /// Last verified timestamp
    pub last_verified: SystemTime,
}

impl NodeIdentity {
    /// Create a new node identity
    pub fn new(node_id: String, node_name: String) -> Self {
        Self {
            node_id,
            node_name,
            region: String::new(),
            zone: String::new(),
            role: "node".to_string(),
            tags: HashMap::new(),
            spiffe_id: None,
            cert_subject: None,
            created_at: SystemTime::now(),
            last_verified: SystemTime::now(),
        }
    }

    /// Set region and zone
    pub fn with_location(mut self, region: String, zone: String) -> Self {
        self.region = region;
        self.zone = zone;
        self
    }

    /// Set role
    pub fn with_role(mut self, role: String) -> Self {
        self.role = role;
        self
    }

    /// Set SPIFFE ID
    pub fn with_spiffe_id(mut self, spiffe_id: String) -> Self {
        self.spiffe_id = Some(spiffe_id);
        self
    }

    /// Set certificate subject
    pub fn with_cert_subject(mut self, cert_subject: String) -> Self {
        self.cert_subject = Some(cert_subject);
        self
    }

    /// Add tag
    pub fn add_tag(&mut self, key: String, value: String) {
        self.tags.insert(key, value);
    }

    /// Get tag
    pub fn get_tag(&self, key: &str) -> Option<&String> {
        self.tags.get(key)
    }

    /// Update verification timestamp
    pub fn mark_verified(&mut self) {
        self.last_verified = SystemTime::now();
    }

    /// Check if identity needs re-verification
    pub fn needs_verification(&self, interval: Duration) -> bool {
        if let Ok(elapsed) = SystemTime::now().duration_since(self.last_verified) {
            elapsed > interval
        } else {
            true
        }
    }
}

/// SPIFFE identity
#[derive(Debug, Clone)]
pub struct SpiffeId {
    /// Trust domain
    pub trust_domain: String,

    /// Path
    pub path: String,
}

impl SpiffeId {
    /// Create a new SPIFFE ID
    pub fn new(trust_domain: String, path: String) -> Self {
        Self { trust_domain, path }
    }

    /// Parse SPIFFE ID from URI
    pub fn from_uri(uri: &str) -> Result<Self> {
        if !uri.starts_with("spiffe://") {
            return Err(DbError::ParseError(format!("Invalid SPIFFE ID: {}", uri)));
        }

        let rest = &uri[9..]; // Skip "spiffe://"
        let parts: Vec<&str> = rest.splitn(2, '/').collect();

        if parts.len() != 2 {
            return Err(DbError::ParseError(format!(
                "Invalid SPIFFE ID format: {}",
                uri
            )));
        }

        Ok(Self {
            trust_domain: parts[0].to_string(),
            path: format!("/{}", parts[1]),
        })
    }

    /// Convert to URI
    pub fn to_uri(&self) -> String {
        format!("spiffe://{}{}", self.trust_domain, self.path)
    }

    /// Validate SPIFFE ID
    pub fn validate(&self) -> Result<()> {
        if self.trust_domain.is_empty() {
            return Err(DbError::Validation(
                "Trust domain cannot be empty".to_string(),
            ));
        }

        if !self.path.starts_with('/') {
            return Err(DbError::Validation("Path must start with /".to_string()));
        }

        Ok(())
    }
}

/// JWT token for service mesh
#[derive(Debug, Clone)]
pub struct JwtToken {
    /// Token string
    pub token: String,

    /// Subject (node ID)
    pub subject: String,

    /// Issuer
    pub issuer: String,

    /// Audience
    pub audience: Vec<String>,

    /// Issued at
    pub issued_at: SystemTime,

    /// Expiration
    pub expires_at: SystemTime,

    /// Claims
    pub claims: HashMap<String, String>,
}

impl JwtToken {
    /// Create a new JWT token
    pub fn new(subject: String, issuer: String) -> Self {
        let now = SystemTime::now();
        Self {
            token: String::new(),
            subject,
            issuer,
            audience: Vec::new(),
            issued_at: now,
            expires_at: now + Duration::from_secs(3600), // 1 hour default
            claims: HashMap::new(),
        }
    }

    /// Set expiration
    pub fn with_expiration(mut self, expires_at: SystemTime) -> Self {
        self.expires_at = expires_at;
        self
    }

    /// Add audience
    pub fn add_audience(&mut self, audience: String) {
        self.audience.push(audience);
    }

    /// Add claim
    pub fn add_claim(&mut self, key: String, value: String) {
        self.claims.insert(key, value);
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }

    /// Validate token
    pub fn validate(&self) -> Result<()> {
        if self.is_expired() {
            return Err(DbError::Authentication("Token expired".to_string()));
        }

        if self.subject.is_empty() {
            return Err(DbError::Validation("Subject cannot be empty".to_string()));
        }

        Ok(())
    }

    /// Generate token string (placeholder - use jsonwebtoken crate in production)
    pub fn generate(&mut self, _secret: &[u8]) -> Result<()> {
        // In production, use jsonwebtoken crate to generate actual JWT
        self.token = format!(
            "jwt.{}.{}",
            self.subject,
            self.issued_at.elapsed().unwrap_or_default().as_secs()
        );
        Ok(())
    }

    /// Verify token string (placeholder - use jsonwebtoken crate in production)
    pub fn verify(_token: &str, _secret: &[u8]) -> Result<Self> {
        // In production, use jsonwebtoken crate to verify and decode JWT
        Err(DbError::NotImplemented(
            "JWT verification not yet implemented".to_string(),
        ))
    }
}

/// Identity provider
pub struct IdentityProvider {
    /// Registered identities
    identities: Arc<RwLock<HashMap<String, NodeIdentity>>>,

    /// JWT signing key
    jwt_secret: Arc<Vec<u8>>,

    /// SPIFFE trust domain
    spiffe_trust_domain: Option<String>,

    /// Verification interval
    verification_interval: Duration,
}

impl IdentityProvider {
    /// Create a new identity provider
    pub fn new() -> Result<Self> {
        Ok(Self {
            identities: Arc::new(RwLock::new(HashMap::new())),
            jwt_secret: Arc::new(b"secret-key-change-in-production".to_vec()),
            spiffe_trust_domain: None,
            verification_interval: Duration::from_secs(300), // 5 minutes
        })
    }

    /// Set SPIFFE trust domain
    pub fn with_spiffe_trust_domain(mut self, trust_domain: String) -> Self {
        self.spiffe_trust_domain = Some(trust_domain);
        self
    }

    /// Set verification interval
    pub fn with_verification_interval(mut self, interval: Duration) -> Self {
        self.verification_interval = interval;
        self
    }

    /// Set JWT secret
    pub fn with_jwt_secret(mut self, secret: Vec<u8>) -> Self {
        self.jwt_secret = Arc::new(secret);
        self
    }

    /// Register node identity
    pub async fn register(&self, identity: NodeIdentity) -> Result<()> {
        let mut identities = self.identities.write().await;
        identities.insert(identity.node_id.clone(), identity);
        Ok(())
    }

    /// Unregister node identity
    pub async fn unregister(&self, node_id: &str) -> Result<()> {
        let mut identities = self.identities.write().await;
        identities.remove(node_id);
        Ok(())
    }

    /// Get node identity
    pub async fn get_identity(&self, node_id: &str) -> Result<NodeIdentity> {
        let identities = self.identities.read().await;
        identities
            .get(node_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("Node identity not found: {}", node_id)))
    }

    /// Verify node identity
    pub async fn verify(&self, node_id: &str) -> Result<bool> {
        let identities = self.identities.read().await;
        if let Some(identity) = identities.get(node_id) {
            // Check if needs re-verification
            if identity.needs_verification(self.verification_interval) {
                drop(identities);
                let mut identities = self.identities.write().await;
                if let Some(identity) = identities.get_mut(node_id) {
                    identity.mark_verified();
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Generate JWT token for node
    pub async fn generate_jwt_token(&self, node_id: &str) -> Result<JwtToken> {
        let identity = self.get_identity(node_id).await?;

        let mut token = JwtToken::new(identity.node_id.clone(), "rustydb".to_string());
        token.add_claim("node_name".to_string(), identity.node_name);
        token.add_claim("role".to_string(), identity.role);
        token.add_claim("region".to_string(), identity.region);
        token.add_claim("zone".to_string(), identity.zone);

        if let Some(spiffe_id) = identity.spiffe_id {
            token.add_claim("spiffe_id".to_string(), spiffe_id);
        }

        token.generate(&self.jwt_secret)?;

        Ok(token)
    }

    /// Verify JWT token
    pub fn verify_jwt_token(&self, token: &str) -> Result<JwtToken> {
        JwtToken::verify(token, &self.jwt_secret)
    }

    /// Generate SPIFFE ID for node
    pub fn generate_spiffe_id(&self, node_id: &str) -> Result<SpiffeId> {
        let trust_domain = self.spiffe_trust_domain.as_ref().ok_or_else(|| {
            DbError::Configuration("SPIFFE trust domain not configured".to_string())
        })?;

        let spiffe_id = SpiffeId::new(trust_domain.clone(), format!("/node/{}", node_id));

        spiffe_id.validate()?;

        Ok(spiffe_id)
    }

    /// List all identities
    pub async fn list_identities(&self) -> Vec<NodeIdentity> {
        let identities = self.identities.read().await;
        identities.values().cloned().collect()
    }

    /// Get identities by role
    pub async fn get_by_role(&self, role: &str) -> Vec<NodeIdentity> {
        let identities = self.identities.read().await;
        identities
            .values()
            .filter(|id| id.role == role)
            .cloned()
            .collect()
    }

    /// Get identities by region
    pub async fn get_by_region(&self, region: &str) -> Vec<NodeIdentity> {
        let identities = self.identities.read().await;
        identities
            .values()
            .filter(|id| id.region == region)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_identity() {
        let mut identity = NodeIdentity::new("node-1".to_string(), "db-node-1".to_string())
            .with_location("us-west-2".to_string(), "us-west-2a".to_string())
            .with_role("primary".to_string());

        identity.add_tag("env".to_string(), "prod".to_string());

        assert_eq!(identity.node_id, "node-1");
        assert_eq!(identity.region, "us-west-2");
        assert_eq!(identity.role, "primary");
        assert_eq!(identity.get_tag("env"), Some(&"prod".to_string()));
    }

    #[test]
    fn test_spiffe_id() {
        let spiffe = SpiffeId::new("example.com".to_string(), "/node/node-1".to_string());
        assert_eq!(spiffe.to_uri(), "spiffe://example.com/node/node-1");

        let parsed = SpiffeId::from_uri("spiffe://example.com/node/node-1").unwrap();
        assert_eq!(parsed.trust_domain, "example.com");
        assert_eq!(parsed.path, "/node/node-1");
    }

    #[test]
    fn test_jwt_token() {
        let mut token = JwtToken::new("node-1".to_string(), "rustydb".to_string());
        token.add_claim("role".to_string(), "primary".to_string());

        assert!(!token.is_expired());
        assert!(token.validate().is_ok());
    }

    #[tokio::test]
    async fn test_identity_provider() {
        let provider = IdentityProvider::new().unwrap();

        let identity = NodeIdentity::new("node-1".to_string(), "db-node-1".to_string());
        provider.register(identity).await.unwrap();

        let retrieved = provider.get_identity("node-1").await.unwrap();
        assert_eq!(retrieved.node_id, "node-1");

        let verified = provider.verify("node-1").await.unwrap();
        assert!(verified);
    }
}
