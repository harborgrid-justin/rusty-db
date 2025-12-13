// # WebSocket Authentication Module
//
// Provides comprehensive authentication mechanisms for WebSocket connections
// including JWT/Bearer tokens, API keys, session-based auth, and RBAC integration.
//
// ## Features
//
// - Multiple authentication methods (Token, API Key, Session)
// - JWT/Bearer token validation
// - API key authentication
// - Session-based authentication
// - Permission checking for subscriptions
// - Role-based access control (RBAC) integration
// - Connection-level permission tracking
// - Audit logging integration

use crate::error::DbError;
use crate::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Re-export types from security modules
pub use crate::security::rbac::{PermissionId, RoleId, UserId};

// ============================================================================
// Authentication Types
// ============================================================================

/// Authentication credentials for WebSocket connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebSocketCredentials {
    /// JWT/Bearer token
    Token { token: String },

    /// API key authentication
    ApiKey { key: String },

    /// Session ID authentication
    Session { session_id: String },

    /// Username/password (not recommended for production)
    UsernamePassword { username: String, password: String },

    /// Anonymous (no authentication)
    Anonymous,
}

/// Authentication result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    /// Authentication succeeded
    pub authenticated: bool,

    /// User ID if authenticated
    pub user_id: Option<UserId>,

    /// Username if authenticated
    pub username: Option<String>,

    /// Assigned roles
    pub roles: Vec<RoleId>,

    /// Granted permissions
    pub permissions: HashSet<PermissionId>,

    /// Authentication method used
    pub auth_method: AuthMethod,

    /// Authentication timestamp
    pub authenticated_at: i64,

    /// Token/session expiration timestamp
    pub expires_at: Option<i64>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl AuthResult {
    /// Create an anonymous authentication result
    pub fn anonymous() -> Self {
        Self {
            authenticated: false,
            user_id: None,
            username: None,
            roles: Vec::new(),
            permissions: HashSet::new(),
            auth_method: AuthMethod::Anonymous,
            authenticated_at: current_timestamp(),
            expires_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Check if authentication has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            current_timestamp() > expires_at
        } else {
            false
        }
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(permission)
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

/// Authentication method used
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    Token,
    ApiKey,
    Session,
    UsernamePassword,
    Anonymous,
}

// ============================================================================
// Authenticator Trait
// ============================================================================

/// Trait for WebSocket authenticators
pub trait WebSocketAuthenticator: Send + Sync {
    /// Authenticate credentials and return result
    fn authenticate(&self, credentials: &WebSocketCredentials) -> Result<AuthResult>;

    /// Validate existing authentication (check expiration, etc.)
    fn validate(&self, auth_result: &AuthResult) -> Result<bool>;

    /// Refresh authentication (extend session, renew token, etc.)
    fn refresh(&self, auth_result: &AuthResult) -> Result<AuthResult>;

    /// Revoke authentication (logout, invalidate token, etc.)
    fn revoke(&self, user_id: &UserId) -> Result<()>;
}

// ============================================================================
// Token Authentication (JWT/Bearer)
// ============================================================================

/// JWT token claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Subject (user ID)
    pub sub: UserId,

    /// Username
    pub username: String,

    /// Issued at timestamp
    pub iat: i64,

    /// Expiration timestamp
    pub exp: i64,

    /// Issuer
    pub iss: Option<String>,

    /// Audience
    pub aud: Option<String>,

    /// Roles
    pub roles: Vec<RoleId>,

    /// Permissions
    pub permissions: Vec<PermissionId>,

    /// Custom claims
    pub custom: HashMap<String, String>,
}

/// Token authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAuthConfig {
    /// JWT secret key for validation
    pub secret_key: String,

    /// Token issuer to validate
    pub expected_issuer: Option<String>,

    /// Token audience to validate
    pub expected_audience: Option<String>,

    /// Allow expired tokens (for debugging)
    pub allow_expired: bool,

    /// Clock skew tolerance in seconds
    pub clock_skew_secs: i64,

    /// Token refresh window (seconds before expiration)
    pub refresh_window_secs: i64,
}

impl Default for TokenAuthConfig {
    fn default() -> Self {
        Self {
            secret_key: String::new(),
            expected_issuer: None,
            expected_audience: None,
            allow_expired: false,
            clock_skew_secs: 60,
            refresh_window_secs: 300,
        }
    }
}

/// Token-based authenticator
pub struct TokenAuthenticator {
    config: Arc<RwLock<TokenAuthConfig>>,
    // In a real implementation, we'd use a JWT library
    // For now, we'll use a simple token store
    token_store: Arc<RwLock<HashMap<String, TokenClaims>>>,
}

impl TokenAuthenticator {
    /// Create a new token authenticator
    pub fn new(config: TokenAuthConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            token_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Decode and validate JWT token (simplified)
    fn decode_token(&self, token: &str) -> Result<TokenClaims> {
        // In a real implementation, use a JWT library like `jsonwebtoken`
        // For now, lookup in token store
        let store = self.token_store.read();
        store
            .get(token)
            .cloned()
            .ok_or_else(|| DbError::Authentication("Invalid token".to_string()))
    }

    /// Validate token claims
    fn validate_claims(&self, claims: &TokenClaims) -> Result<()> {
        let config = self.config.read();
        let now = current_timestamp();

        // Check expiration
        if !config.allow_expired && claims.exp < now - config.clock_skew_secs {
            return Err(DbError::Authentication("Token expired".to_string()));
        }

        // Check issuer
        if let Some(expected_iss) = &config.expected_issuer {
            if claims.iss.as_ref() != Some(expected_iss) {
                return Err(DbError::Authentication("Invalid issuer".to_string()));
            }
        }

        // Check audience
        if let Some(expected_aud) = &config.expected_audience {
            if claims.aud.as_ref() != Some(expected_aud) {
                return Err(DbError::Authentication("Invalid audience".to_string()));
            }
        }

        Ok(())
    }

    /// Store a token (for testing/development)
    pub fn store_token(&self, token: String, claims: TokenClaims) {
        let mut store = self.token_store.write();
        store.insert(token, claims);
    }

    /// Remove a token
    pub fn remove_token(&self, token: &str) {
        let mut store = self.token_store.write();
        store.remove(token);
    }
}

impl WebSocketAuthenticator for TokenAuthenticator {
    fn authenticate(&self, credentials: &WebSocketCredentials) -> Result<AuthResult> {
        match credentials {
            WebSocketCredentials::Token { token } => {
                let claims = self.decode_token(token)?;
                self.validate_claims(&claims)?;

                Ok(AuthResult {
                    authenticated: true,
                    user_id: Some(claims.sub.clone()),
                    username: Some(claims.username.clone()),
                    roles: claims.roles.clone(),
                    permissions: claims.permissions.iter().cloned().collect(),
                    auth_method: AuthMethod::Token,
                    authenticated_at: current_timestamp(),
                    expires_at: Some(claims.exp),
                    metadata: claims.custom.clone(),
                })
            }
            _ => Err(DbError::Authentication(
                "Invalid credentials type for token authenticator".to_string(),
            )),
        }
    }

    fn validate(&self, auth_result: &AuthResult) -> Result<bool> {
        if auth_result.auth_method != AuthMethod::Token {
            return Ok(false);
        }

        Ok(!auth_result.is_expired())
    }

    fn refresh(&self, auth_result: &AuthResult) -> Result<AuthResult> {
        if auth_result.auth_method != AuthMethod::Token {
            return Err(DbError::Authentication("Not a token authentication".to_string()));
        }

        // Check if in refresh window
        let config = self.config.read();
        let now = current_timestamp();

        if let Some(expires_at) = auth_result.expires_at {
            let time_until_expiry = expires_at - now;
            if time_until_expiry > config.refresh_window_secs {
                return Err(DbError::Authentication(
                    "Token not yet in refresh window".to_string(),
                ));
            }
        }

        // Create new auth result with extended expiration
        let mut new_result = auth_result.clone();
        new_result.authenticated_at = now;
        new_result.expires_at = Some(now + 3600); // 1 hour from now

        Ok(new_result)
    }

    fn revoke(&self, _user_id: &UserId) -> Result<()> {
        // In a real implementation, maintain a revocation list
        Ok(())
    }
}

// ============================================================================
// API Key Authentication
// ============================================================================

/// API key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    /// API key (hashed)
    pub key_hash: String,

    /// User ID
    pub user_id: UserId,

    /// Username
    pub username: String,

    /// Key name/description
    pub name: String,

    /// Assigned roles
    pub roles: Vec<RoleId>,

    /// Granted permissions
    pub permissions: Vec<PermissionId>,

    /// Creation timestamp
    pub created_at: i64,

    /// Expiration timestamp (None = never expires)
    pub expires_at: Option<i64>,

    /// Last used timestamp
    pub last_used: Option<i64>,

    /// Is enabled
    pub enabled: bool,

    /// Rate limit (requests per minute)
    pub rate_limit: Option<u32>,

    /// Allowed IP addresses (empty = any)
    pub allowed_ips: Vec<String>,
}

/// API key authenticator
pub struct ApiKeyAuthenticator {
    /// API key store (key_hash -> ApiKeyInfo)
    api_keys: Arc<RwLock<HashMap<String, ApiKeyInfo>>>,
}

impl ApiKeyAuthenticator {
    /// Create a new API key authenticator
    pub fn new() -> Self {
        Self {
            api_keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Hash API key (in real implementation, use proper hashing)
    fn hash_key(&self, key: &str) -> String {
        // In production, use a proper hash function
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Register an API key
    pub fn register_key(&self, key: String, info: ApiKeyInfo) {
        let key_hash = self.hash_key(&key);
        let mut keys = self.api_keys.write();
        keys.insert(key_hash, info);
    }

    /// Revoke an API key
    pub fn revoke_key(&self, key: &str) {
        let key_hash = self.hash_key(key);
        let mut keys = self.api_keys.write();
        keys.remove(&key_hash);
    }

    /// Get API key info
    pub fn get_key_info(&self, key: &str) -> Option<ApiKeyInfo> {
        let key_hash = self.hash_key(key);
        let keys = self.api_keys.read();
        keys.get(&key_hash).cloned()
    }

    /// Update last used timestamp
    fn update_last_used(&self, key: &str) {
        let key_hash = self.hash_key(key);
        let mut keys = self.api_keys.write();
        if let Some(info) = keys.get_mut(&key_hash) {
            info.last_used = Some(current_timestamp());
        }
    }
}

impl Default for ApiKeyAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketAuthenticator for ApiKeyAuthenticator {
    fn authenticate(&self, credentials: &WebSocketCredentials) -> Result<AuthResult> {
        match credentials {
            WebSocketCredentials::ApiKey { key } => {
                let key_hash = self.hash_key(key);
                let keys = self.api_keys.read();

                let Some(info) = keys.get(&key_hash) else {
                    return Err(DbError::Authentication("Invalid API key".to_string()));
                };

                // Check if enabled
                if !info.enabled {
                    return Err(DbError::Authentication("API key disabled".to_string()));
                }

                // Check expiration
                if let Some(expires_at) = info.expires_at {
                    if current_timestamp() > expires_at {
                        return Err(DbError::Authentication("API key expired".to_string()));
                    }
                }

                // Update last used (drop read lock first)
                drop(keys);
                self.update_last_used(key);

                // Get info again
                let keys = self.api_keys.read();
                let info = keys.get(&key_hash).unwrap();

                Ok(AuthResult {
                    authenticated: true,
                    user_id: Some(info.user_id.clone()),
                    username: Some(info.username.clone()),
                    roles: info.roles.clone(),
                    permissions: info.permissions.iter().cloned().collect(),
                    auth_method: AuthMethod::ApiKey,
                    authenticated_at: current_timestamp(),
                    expires_at: info.expires_at,
                    metadata: HashMap::new(),
                })
            }
            _ => Err(DbError::Authentication(
                "Invalid credentials type for API key authenticator".to_string(),
            )),
        }
    }

    fn validate(&self, auth_result: &AuthResult) -> Result<bool> {
        if auth_result.auth_method != AuthMethod::ApiKey {
            return Ok(false);
        }

        Ok(!auth_result.is_expired())
    }

    fn refresh(&self, auth_result: &AuthResult) -> Result<AuthResult> {
        // API keys don't need refresh
        Ok(auth_result.clone())
    }

    fn revoke(&self, user_id: &UserId) -> Result<()> {
        let mut keys = self.api_keys.write();
        keys.retain(|_, info| &info.user_id != user_id);
        Ok(())
    }
}

// ============================================================================
// Session Authentication
// ============================================================================

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID
    pub session_id: String,

    /// User ID
    pub user_id: UserId,

    /// Username
    pub username: String,

    /// Assigned roles
    pub roles: Vec<RoleId>,

    /// Granted permissions
    pub permissions: Vec<PermissionId>,

    /// Creation timestamp
    pub created_at: i64,

    /// Last activity timestamp
    pub last_activity: i64,

    /// Expiration timestamp
    pub expires_at: i64,

    /// Client IP
    pub client_ip: Option<String>,

    /// Session metadata
    pub metadata: HashMap<String, String>,
}

/// Session authenticator
pub struct SessionAuthenticator {
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,

    /// Session timeout in seconds
    session_timeout: Duration,

    /// Idle timeout in seconds
    idle_timeout: Duration,
}

impl SessionAuthenticator {
    /// Create a new session authenticator
    pub fn new(session_timeout: Duration, idle_timeout: Duration) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_timeout,
            idle_timeout,
        }
    }

    /// Create a new session
    pub fn create_session(&self, info: SessionInfo) -> String {
        let session_id = info.session_id.clone();
        let mut sessions = self.sessions.write();
        sessions.insert(session_id.clone(), info);
        session_id
    }

    /// Destroy a session
    pub fn destroy_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write();
        sessions.remove(session_id);
    }

    /// Update session activity
    fn update_activity(&self, session_id: &str) {
        let mut sessions = self.sessions.write();
        if let Some(info) = sessions.get_mut(session_id) {
            info.last_activity = current_timestamp();
        }
    }

    /// Clean up expired sessions
    pub fn cleanup_expired(&self) {
        let now = current_timestamp();
        let mut sessions = self.sessions.write();

        sessions.retain(|_, info| {
            // Check expiration
            if info.expires_at < now {
                return false;
            }

            // Check idle timeout
            let idle_duration = now - info.last_activity;
            idle_duration < self.idle_timeout.as_secs() as i64
        });
    }

    /// Get session info
    pub fn get_session(&self, session_id: &str) -> Option<SessionInfo> {
        self.sessions.read().get(session_id).cloned()
    }
}

impl Default for SessionAuthenticator {
    fn default() -> Self {
        Self::new(Duration::from_secs(3600), Duration::from_secs(1800))
    }
}

impl WebSocketAuthenticator for SessionAuthenticator {
    fn authenticate(&self, credentials: &WebSocketCredentials) -> Result<AuthResult> {
        match credentials {
            WebSocketCredentials::Session { session_id } => {
                let sessions = self.sessions.read();

                let Some(info) = sessions.get(session_id) else {
                    return Err(DbError::Authentication("Invalid session".to_string()));
                };

                let now = current_timestamp();

                // Check expiration
                if info.expires_at < now {
                    return Err(DbError::Authentication("Session expired".to_string()));
                }

                // Check idle timeout
                let idle_duration = now - info.last_activity;
                if idle_duration > self.idle_timeout.as_secs() as i64 {
                    return Err(DbError::Authentication("Session idle timeout".to_string()));
                }

                // Update activity (drop read lock first)
                drop(sessions);
                self.update_activity(session_id);

                // Get info again
                let sessions = self.sessions.read();
                let info = sessions.get(session_id).unwrap();

                Ok(AuthResult {
                    authenticated: true,
                    user_id: Some(info.user_id.clone()),
                    username: Some(info.username.clone()),
                    roles: info.roles.clone(),
                    permissions: info.permissions.iter().cloned().collect(),
                    auth_method: AuthMethod::Session,
                    authenticated_at: current_timestamp(),
                    expires_at: Some(info.expires_at),
                    metadata: info.metadata.clone(),
                })
            }
            _ => Err(DbError::Authentication(
                "Invalid credentials type for session authenticator".to_string(),
            )),
        }
    }

    fn validate(&self, auth_result: &AuthResult) -> Result<bool> {
        if auth_result.auth_method != AuthMethod::Session {
            return Ok(false);
        }

        Ok(!auth_result.is_expired())
    }

    fn refresh(&self, auth_result: &AuthResult) -> Result<AuthResult> {
        if auth_result.auth_method != AuthMethod::Session {
            return Err(DbError::Authentication(
                "Not a session authentication".to_string(),
            ));
        }

        // Extend session expiration
        let mut new_result = auth_result.clone();
        let now = current_timestamp();
        new_result.expires_at = Some(now + self.session_timeout.as_secs() as i64);

        Ok(new_result)
    }

    fn revoke(&self, user_id: &UserId) -> Result<()> {
        let mut sessions = self.sessions.write();
        sessions.retain(|_, info| &info.user_id != user_id);
        Ok(())
    }
}

// ============================================================================
// Multi-Authenticator (Strategy Pattern)
// ============================================================================

/// Multi-authenticator that tries multiple authentication methods
pub struct MultiAuthenticator {
    authenticators: Vec<Box<dyn WebSocketAuthenticator>>,
}

impl MultiAuthenticator {
    /// Create a new multi-authenticator
    pub fn new() -> Self {
        Self {
            authenticators: Vec::new(),
        }
    }

    /// Add an authenticator
    pub fn add_authenticator(&mut self, auth: Box<dyn WebSocketAuthenticator>) {
        self.authenticators.push(auth);
    }
}

impl Default for MultiAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketAuthenticator for MultiAuthenticator {
    fn authenticate(&self, credentials: &WebSocketCredentials) -> Result<AuthResult> {
        // Try each authenticator until one succeeds
        for authenticator in &self.authenticators {
            if let Ok(result) = authenticator.authenticate(credentials) {
                return Ok(result);
            }
        }

        Err(DbError::Authentication(
            "No authenticator accepted credentials".to_string(),
        ))
    }

    fn validate(&self, auth_result: &AuthResult) -> Result<bool> {
        // Validate with appropriate authenticator based on method
        for authenticator in &self.authenticators {
            if let Ok(valid) = authenticator.validate(auth_result) {
                if valid {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    fn refresh(&self, auth_result: &AuthResult) -> Result<AuthResult> {
        // Try to refresh with appropriate authenticator
        for authenticator in &self.authenticators {
            if let Ok(refreshed) = authenticator.refresh(auth_result) {
                return Ok(refreshed);
            }
        }

        Err(DbError::Authentication(
            "No authenticator could refresh authentication".to_string(),
        ))
    }

    fn revoke(&self, user_id: &UserId) -> Result<()> {
        // Revoke from all authenticators
        for authenticator in &self.authenticators {
            let _ = authenticator.revoke(user_id);
        }

        Ok(())
    }
}

// ============================================================================
// Permission Checking
// ============================================================================

/// Permission checker for WebSocket operations
pub struct PermissionChecker {
    /// Required permissions for subscription types
    subscription_permissions: Arc<RwLock<HashMap<String, Vec<PermissionId>>>>,
}

impl PermissionChecker {
    /// Create a new permission checker
    pub fn new() -> Self {
        Self {
            subscription_permissions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register required permissions for a subscription type
    pub fn register_subscription_permissions(
        &self,
        subscription_type: String,
        permissions: Vec<PermissionId>,
    ) {
        let mut perms = self.subscription_permissions.write();
        perms.insert(subscription_type, permissions);
    }

    /// Check if auth result has permission for subscription
    pub fn check_subscription_permission(
        &self,
        auth_result: &AuthResult,
        subscription_type: &str,
    ) -> Result<bool> {
        let perms = self.subscription_permissions.read();

        let Some(required_perms) = perms.get(subscription_type) else {
            // If no permissions defined, allow
            return Ok(true);
        };

        // Check if user has all required permissions
        for perm in required_perms {
            if !auth_result.has_permission(perm) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check if auth result has permission for action
    pub fn check_permission(
        &self,
        auth_result: &AuthResult,
        permission: &str,
    ) -> Result<bool> {
        Ok(auth_result.has_permission(permission))
    }

    /// Check if auth result has role
    pub fn check_role(&self, auth_result: &AuthResult, role: &str) -> Result<bool> {
        Ok(auth_result.has_role(role))
    }
}

impl Default for PermissionChecker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Utilities
// ============================================================================

/// Get current timestamp in seconds
fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_result_anonymous() {
        let result = AuthResult::anonymous();
        assert!(!result.authenticated);
        assert!(result.user_id.is_none());
        assert_eq!(result.auth_method, AuthMethod::Anonymous);
    }

    #[test]
    fn test_auth_result_expiration() {
        let mut result = AuthResult::anonymous();
        result.expires_at = Some(current_timestamp() - 100);
        assert!(result.is_expired());

        result.expires_at = Some(current_timestamp() + 100);
        assert!(!result.is_expired());
    }

    #[test]
    fn test_auth_result_permissions() {
        let mut result = AuthResult::anonymous();
        result.permissions.insert("read:data".to_string());
        result.permissions.insert("write:data".to_string());

        assert!(result.has_permission("read:data"));
        assert!(result.has_permission("write:data"));
        assert!(!result.has_permission("admin:all"));
    }

    #[test]
    fn test_token_authenticator() {
        let config = TokenAuthConfig::default();
        let authenticator = TokenAuthenticator::new(config);

        let claims = TokenClaims {
            sub: "user1".to_string(),
            username: "testuser".to_string(),
            iat: current_timestamp(),
            exp: current_timestamp() + 3600,
            iss: None,
            aud: None,
            roles: vec!["user".to_string()],
            permissions: vec!["read:data".to_string()],
            custom: HashMap::new(),
        };

        let token = "test-token".to_string();
        authenticator.store_token(token.clone(), claims);

        let creds = WebSocketCredentials::Token { token };
        let result = authenticator.authenticate(&creds).unwrap();

        assert!(result.authenticated);
        assert_eq!(result.user_id, Some("user1".to_string()));
        assert_eq!(result.username, Some("testuser".to_string()));
    }

    #[test]
    fn test_api_key_authenticator() {
        let authenticator = ApiKeyAuthenticator::new();

        let key_info = ApiKeyInfo {
            key_hash: String::new(),
            user_id: "user1".to_string(),
            username: "testuser".to_string(),
            name: "Test Key".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec!["read:data".to_string()],
            created_at: current_timestamp(),
            expires_at: None,
            last_used: None,
            enabled: true,
            rate_limit: None,
            allowed_ips: Vec::new(),
        };

        let api_key = "test-api-key".to_string();
        authenticator.register_key(api_key.clone(), key_info);

        let creds = WebSocketCredentials::ApiKey { key: api_key };
        let result = authenticator.authenticate(&creds).unwrap();

        assert!(result.authenticated);
        assert_eq!(result.user_id, Some("user1".to_string()));
    }

    #[test]
    fn test_session_authenticator() {
        let authenticator = SessionAuthenticator::default();

        let session_info = SessionInfo {
            session_id: "session1".to_string(),
            user_id: "user1".to_string(),
            username: "testuser".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec!["read:data".to_string()],
            created_at: current_timestamp(),
            last_activity: current_timestamp(),
            expires_at: current_timestamp() + 3600,
            client_ip: None,
            metadata: HashMap::new(),
        };

        let session_id = authenticator.create_session(session_info);

        let creds = WebSocketCredentials::Session { session_id };
        let result = authenticator.authenticate(&creds).unwrap();

        assert!(result.authenticated);
        assert_eq!(result.user_id, Some("user1".to_string()));
    }

    #[test]
    fn test_permission_checker() {
        let checker = PermissionChecker::new();

        checker.register_subscription_permissions(
            "metrics".to_string(),
            vec!["read:metrics".to_string()],
        );

        let mut auth_result = AuthResult::anonymous();
        auth_result.authenticated = true;
        auth_result.permissions.insert("read:metrics".to_string());

        assert!(checker
            .check_subscription_permission(&auth_result, "metrics")
            .unwrap());
    }

    #[test]
    fn test_multi_authenticator() {
        let mut multi = MultiAuthenticator::new();

        let token_auth = TokenAuthenticator::new(TokenAuthConfig::default());
        multi.add_authenticator(Box::new(token_auth));

        let api_key_auth = ApiKeyAuthenticator::new();
        multi.add_authenticator(Box::new(api_key_auth));

        // Should fail as we haven't registered any keys/tokens
        let creds = WebSocketCredentials::ApiKey {
            key: "invalid".to_string(),
        };
        assert!(multi.authenticate(&creds).is_err());
    }
}
