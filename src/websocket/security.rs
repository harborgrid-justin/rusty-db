// # WebSocket Security Module
//
// Provides comprehensive security features for WebSocket connections including
// TLS validation, origin checking, rate limiting, message size limits, and encryption.
//
// ## Features
//
// - TLS/SSL certificate validation
// - Origin validation and CORS support
// - Message size limits to prevent DoS
// - Connection rate limiting per IP address
// - Message encryption for sensitive data
// - Secure default configuration
// - Integration with existing security infrastructure

use crate::error::DbError;
use crate::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// Configuration
// ============================================================================

/// WebSocket security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketSecurityConfig {
    /// Enable TLS/SSL for WebSocket connections
    pub enable_tls: bool,

    /// Require valid TLS certificates
    pub require_valid_cert: bool,

    /// Allowed TLS versions
    pub allowed_tls_versions: Vec<TlsVersion>,

    /// Enable origin validation
    pub enable_origin_validation: bool,

    /// Allowed origins (empty = allow all)
    pub allowed_origins: Vec<String>,

    /// Maximum message size in bytes
    pub max_message_size: usize,

    /// Maximum frame size in bytes
    pub max_frame_size: usize,

    /// Maximum connections per IP address
    pub max_connections_per_ip: usize,

    /// Connection rate limit (connections per minute per IP)
    pub connection_rate_limit: u32,

    /// Enable message encryption for sensitive data
    pub enable_message_encryption: bool,

    /// Message encryption algorithm
    pub encryption_algorithm: EncryptionAlgorithm,

    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,

    /// Idle timeout in seconds (close connection if no activity)
    pub idle_timeout_secs: u64,

    /// Maximum number of concurrent connections
    pub max_total_connections: usize,

    /// Enable ping/pong keepalive
    pub enable_keepalive: bool,

    /// Keepalive interval in seconds
    pub keepalive_interval_secs: u64,

    /// Enable compression
    pub enable_compression: bool,

    /// Maximum compression window size
    pub compression_window_size: u8,

    /// Enable subprotocol validation
    pub enable_subprotocol_validation: bool,

    /// Allowed subprotocols
    pub allowed_subprotocols: Vec<String>,

    /// Enable IP address whitelisting
    pub enable_ip_whitelist: bool,

    /// IP address whitelist (empty = disabled)
    pub ip_whitelist: Vec<IpAddr>,

    /// Enable IP address blacklisting
    pub enable_ip_blacklist: bool,

    /// IP address blacklist
    pub ip_blacklist: Vec<IpAddr>,

    /// Enable audit logging
    pub enable_audit_logging: bool,

    /// Log all messages (warning: high overhead)
    pub log_all_messages: bool,
}

impl Default for WebSocketSecurityConfig {
    fn default() -> Self {
        Self {
            enable_tls: true,
            require_valid_cert: true,
            allowed_tls_versions: vec![TlsVersion::TLS12, TlsVersion::TLS13],
            enable_origin_validation: true,
            allowed_origins: Vec::new(),
            max_message_size: 10 * 1024 * 1024, // 10 MB
            max_frame_size: 1024 * 1024,        // 1 MB
            max_connections_per_ip: 10,
            connection_rate_limit: 60, // 60 connections per minute
            enable_message_encryption: false,
            encryption_algorithm: EncryptionAlgorithm::AES256GCM,
            connection_timeout_secs: 30,
            idle_timeout_secs: 300, // 5 minutes
            max_total_connections: 10000,
            enable_keepalive: true,
            keepalive_interval_secs: 30,
            enable_compression: true,
            compression_window_size: 15,
            enable_subprotocol_validation: true,
            allowed_subprotocols: vec![
                "graphql-ws".to_string(),
                "graphql-transport-ws".to_string(),
            ],
            enable_ip_whitelist: false,
            ip_whitelist: Vec::new(),
            enable_ip_blacklist: true,
            ip_blacklist: Vec::new(),
            enable_audit_logging: true,
            log_all_messages: false,
        }
    }
}

impl WebSocketSecurityConfig {
    /// Create a secure configuration with strict defaults
    pub fn secure() -> Self {
        Self {
            enable_tls: true,
            require_valid_cert: true,
            allowed_tls_versions: vec![TlsVersion::TLS13],
            enable_origin_validation: true,
            max_message_size: 1024 * 1024, // 1 MB
            max_frame_size: 64 * 1024,     // 64 KB
            max_connections_per_ip: 5,
            connection_rate_limit: 30,
            enable_message_encryption: true,
            connection_timeout_secs: 10,
            idle_timeout_secs: 120,
            max_total_connections: 1000,
            log_all_messages: true,
            ..Default::default()
        }
    }

    /// Create a permissive configuration for development
    pub fn permissive() -> Self {
        Self {
            enable_tls: false,
            require_valid_cert: false,
            enable_origin_validation: false,
            max_message_size: 100 * 1024 * 1024, // 100 MB
            max_connections_per_ip: 100,
            connection_rate_limit: 1000,
            enable_message_encryption: false,
            enable_ip_whitelist: false,
            enable_ip_blacklist: false,
            enable_audit_logging: false,
            log_all_messages: false,
            ..Default::default()
        }
    }
}

/// TLS version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TlsVersion {
    TLS10,
    TLS11,
    TLS12,
    TLS13,
}

/// Encryption algorithm for message encryption
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// AES-256 in GCM mode
    AES256GCM,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
    /// No encryption
    None,
}

// ============================================================================
// Connection Tracking
// ============================================================================

/// Information about an active WebSocket connection
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Connection ID
    pub connection_id: String,

    /// Client IP address
    pub ip_address: IpAddr,

    /// Connection established time
    pub connected_at: Instant,

    /// Last activity time
    pub last_activity: Instant,

    /// Origin header value
    pub origin: Option<String>,

    /// Subprotocol negotiated
    pub subprotocol: Option<String>,

    /// User agent
    pub user_agent: Option<String>,

    /// Number of messages sent
    pub messages_sent: u64,

    /// Number of messages received
    pub messages_received: u64,

    /// Total bytes sent
    pub bytes_sent: u64,

    /// Total bytes received
    pub bytes_received: u64,

    /// Authentication status
    pub authenticated: bool,

    /// Authenticated user ID
    pub user_id: Option<String>,

    /// Connection metadata
    pub metadata: HashMap<String, String>,
}

impl ConnectionInfo {
    /// Create new connection info
    pub fn new(connection_id: String, ip_address: IpAddr) -> Self {
        let now = Instant::now();
        Self {
            connection_id,
            ip_address,
            connected_at: now,
            last_activity: now,
            origin: None,
            subprotocol: None,
            user_agent: None,
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            authenticated: false,
            user_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Update last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Check if connection is idle
    pub fn is_idle(&self, idle_timeout: Duration) -> bool {
        self.last_activity.elapsed() > idle_timeout
    }

    /// Get connection duration
    pub fn duration(&self) -> Duration {
        self.connected_at.elapsed()
    }
}

// ============================================================================
// Rate Limiting
// ============================================================================

/// Connection rate limiter for IP addresses
#[derive(Debug)]
struct ConnectionRateLimiter {
    /// Connection attempts per IP (timestamp of each attempt)
    attempts: HashMap<IpAddr, Vec<Instant>>,

    /// Window duration for rate limiting
    window_duration: Duration,

    /// Maximum attempts per window
    max_attempts: u32,
}

impl ConnectionRateLimiter {
    fn new(window_duration: Duration, max_attempts: u32) -> Self {
        Self {
            attempts: HashMap::new(),
            window_duration,
            max_attempts,
        }
    }

    /// Check if IP is allowed to connect
    fn check_rate_limit(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let cutoff = now - self.window_duration;

        // Get or create attempts list for this IP
        let attempts = self.attempts.entry(ip).or_insert_with(Vec::new);

        // Remove old attempts
        attempts.retain(|&timestamp| timestamp > cutoff);

        // Check if under limit
        if attempts.len() < self.max_attempts as usize {
            attempts.push(now);
            true
        } else {
            false
        }
    }

    /// Clean up old entries
    fn cleanup(&mut self) {
        let now = Instant::now();
        let cutoff = now - self.window_duration;

        self.attempts.retain(|_, attempts| {
            attempts.retain(|&timestamp| timestamp > cutoff);
            !attempts.is_empty()
        });
    }

    /// Get current rate for an IP
    fn get_current_rate(&self, ip: &IpAddr) -> u32 {
        self.attempts.get(ip).map(|a| a.len() as u32).unwrap_or(0)
    }
}

// ============================================================================
// WebSocket Security Manager
// ============================================================================

/// Main WebSocket security manager
pub struct WebSocketSecurityManager {
    /// Security configuration
    config: Arc<RwLock<WebSocketSecurityConfig>>,

    /// Active connections
    connections: Arc<RwLock<HashMap<String, ConnectionInfo>>>,

    /// Connections by IP address
    connections_by_ip: Arc<RwLock<HashMap<IpAddr, Vec<String>>>>,

    /// Connection rate limiter
    rate_limiter: Arc<RwLock<ConnectionRateLimiter>>,

    /// Encryption keys (connection_id -> key)
    encryption_keys: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl WebSocketSecurityManager {
    /// Create a new WebSocket security manager
    pub fn new(config: WebSocketSecurityConfig) -> Self {
        let rate_limiter =
            ConnectionRateLimiter::new(Duration::from_secs(60), config.connection_rate_limit);

        Self {
            config: Arc::new(RwLock::new(config)),
            connections: Arc::new(RwLock::new(HashMap::new())),
            connections_by_ip: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(RwLock::new(rate_limiter)),
            encryption_keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Validate origin header
    pub fn validate_origin(&self, origin: Option<&str>) -> Result<bool> {
        let config = self.config.read();

        if !config.enable_origin_validation {
            return Ok(true);
        }

        let Some(origin_value) = origin else {
            return Err(DbError::Security("Missing Origin header".to_string()));
        };

        // If allowed origins is empty, allow all
        if config.allowed_origins.is_empty() {
            return Ok(true);
        }

        // Check if origin is in allowed list
        for allowed in &config.allowed_origins {
            if origin_value == allowed || origin_value.ends_with(allowed) {
                return Ok(true);
            }
        }

        Err(DbError::Security(format!(
            "Origin not allowed: {}",
            origin_value
        )))
    }

    /// Validate subprotocol
    pub fn validate_subprotocol(&self, subprotocol: Option<&str>) -> Result<Option<String>> {
        let config = self.config.read();

        if !config.enable_subprotocol_validation {
            return Ok(subprotocol.map(String::from));
        }

        let Some(sub) = subprotocol else {
            return Ok(None);
        };

        if config.allowed_subprotocols.is_empty()
            || config.allowed_subprotocols.iter().any(|s| s == sub)
        {
            Ok(Some(sub.to_string()))
        } else {
            Err(DbError::Security(format!(
                "Subprotocol not allowed: {}",
                sub
            )))
        }
    }

    /// Check if IP address is allowed to connect
    pub fn check_ip_allowed(&self, ip: IpAddr) -> Result<()> {
        let config = self.config.read();

        // Check blacklist first
        if config.enable_ip_blacklist && config.ip_blacklist.contains(&ip) {
            return Err(DbError::Security(format!("IP address blacklisted: {}", ip)));
        }

        // Check whitelist if enabled
        if config.enable_ip_whitelist {
            if config.ip_whitelist.is_empty() || config.ip_whitelist.contains(&ip) {
                Ok(())
            } else {
                Err(DbError::Security(format!(
                    "IP address not whitelisted: {}",
                    ip
                )))
            }
        } else {
            Ok(())
        }
    }

    /// Check connection rate limit for IP
    pub fn check_rate_limit(&self, ip: IpAddr) -> Result<()> {
        let mut rate_limiter = self.rate_limiter.write();

        if rate_limiter.check_rate_limit(ip) {
            Ok(())
        } else {
            let rate = rate_limiter.get_current_rate(&ip);
            Err(DbError::Security(format!(
                "Rate limit exceeded for IP {}: {} connections/min",
                ip, rate
            )))
        }
    }

    /// Check if new connection is allowed for IP
    pub fn check_connection_limit(&self, ip: IpAddr) -> Result<()> {
        let config = self.config.read();
        let connections_by_ip = self.connections_by_ip.read();

        let current_count = connections_by_ip
            .get(&ip)
            .map(|conns| conns.len())
            .unwrap_or(0);

        if current_count >= config.max_connections_per_ip {
            return Err(DbError::Security(format!(
                "Too many connections from IP {}: {} (max: {})",
                ip, current_count, config.max_connections_per_ip
            )));
        }

        // Check total connection limit
        let total_connections = self.connections.read().len();
        if total_connections >= config.max_total_connections {
            return Err(DbError::Security(format!(
                "Maximum total connections reached: {} (max: {})",
                total_connections, config.max_total_connections
            )));
        }

        Ok(())
    }

    /// Register a new connection
    pub fn register_connection(&self, mut conn_info: ConnectionInfo) -> Result<()> {
        let ip = conn_info.ip_address;
        let conn_id = conn_info.connection_id.clone();

        // Update connection info
        conn_info.update_activity();

        // Add to connections
        let mut connections = self.connections.write();
        connections.insert(conn_id.clone(), conn_info);

        // Add to connections by IP
        let mut connections_by_ip = self.connections_by_ip.write();
        connections_by_ip
            .entry(ip)
            .or_insert_with(Vec::new)
            .push(conn_id);

        Ok(())
    }

    /// Unregister a connection
    pub fn unregister_connection(&self, connection_id: &str) -> Result<()> {
        let mut connections = self.connections.write();

        if let Some(conn_info) = connections.remove(connection_id) {
            let ip = conn_info.ip_address;

            // Remove from connections by IP
            let mut connections_by_ip = self.connections_by_ip.write();
            if let Some(ip_conns) = connections_by_ip.get_mut(&ip) {
                ip_conns.retain(|id| id != connection_id);
                if ip_conns.is_empty() {
                    connections_by_ip.remove(&ip);
                }
            }

            // Remove encryption key
            let mut keys = self.encryption_keys.write();
            keys.remove(connection_id);
        }

        Ok(())
    }

    /// Update connection activity
    pub fn update_activity(&self, connection_id: &str) -> Result<()> {
        let mut connections = self.connections.write();

        if let Some(conn_info) = connections.get_mut(connection_id) {
            conn_info.update_activity();
            Ok(())
        } else {
            Err(DbError::NotFound(format!(
                "Connection not found: {}",
                connection_id
            )))
        }
    }

    /// Check message size
    pub fn check_message_size(&self, size: usize) -> Result<()> {
        let config = self.config.read();

        if size > config.max_message_size {
            return Err(DbError::Security(format!(
                "Message size exceeds limit: {} bytes (max: {} bytes)",
                size, config.max_message_size
            )));
        }

        Ok(())
    }

    /// Check frame size
    pub fn check_frame_size(&self, size: usize) -> Result<()> {
        let config = self.config.read();

        if size > config.max_frame_size {
            return Err(DbError::Security(format!(
                "Frame size exceeds limit: {} bytes (max: {} bytes)",
                size, config.max_frame_size
            )));
        }

        Ok(())
    }

    /// Get connection info
    pub fn get_connection(&self, connection_id: &str) -> Option<ConnectionInfo> {
        self.connections.read().get(connection_id).cloned()
    }

    /// Get all connections
    pub fn get_all_connections(&self) -> Vec<ConnectionInfo> {
        self.connections.read().values().cloned().collect()
    }

    /// Get connections for IP
    pub fn get_connections_for_ip(&self, ip: IpAddr) -> Vec<ConnectionInfo> {
        let connections = self.connections.read();
        let connections_by_ip = self.connections_by_ip.read();

        if let Some(conn_ids) = connections_by_ip.get(&ip) {
            conn_ids
                .iter()
                .filter_map(|id| connections.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Close idle connections
    pub fn close_idle_connections(&self) -> Vec<String> {
        let config = self.config.read();
        let idle_timeout = Duration::from_secs(config.idle_timeout_secs);
        let mut connections = self.connections.write();

        let idle_conns: Vec<String> = connections
            .iter()
            .filter(|(_, info)| info.is_idle(idle_timeout))
            .map(|(id, _)| id.clone())
            .collect();

        for conn_id in &idle_conns {
            connections.remove(conn_id);
        }

        idle_conns
    }

    /// Set encryption key for connection
    pub fn set_encryption_key(&self, connection_id: &str, key: Vec<u8>) {
        let mut keys = self.encryption_keys.write();
        keys.insert(connection_id.to_string(), key);
    }

    /// Get encryption key for connection
    pub fn get_encryption_key(&self, connection_id: &str) -> Option<Vec<u8>> {
        self.encryption_keys.read().get(connection_id).cloned()
    }

    /// Cleanup old rate limiter entries
    pub fn cleanup(&self) {
        let mut rate_limiter = self.rate_limiter.write();
        rate_limiter.cleanup();
    }

    /// Get statistics
    pub fn get_stats(&self) -> WebSocketSecurityStats {
        let connections = self.connections.read();
        let connections_by_ip = self.connections_by_ip.read();

        let total_connections = connections.len();
        let authenticated_connections = connections.values().filter(|c| c.authenticated).count();

        let unique_ips = connections_by_ip.len();

        let total_messages_sent: u64 = connections.values().map(|c| c.messages_sent).sum();
        let total_messages_received: u64 = connections.values().map(|c| c.messages_received).sum();
        let total_bytes_sent: u64 = connections.values().map(|c| c.bytes_sent).sum();
        let total_bytes_received: u64 = connections.values().map(|c| c.bytes_received).sum();

        WebSocketSecurityStats {
            total_connections,
            authenticated_connections,
            unique_ips,
            total_messages_sent,
            total_messages_received,
            total_bytes_sent,
            total_bytes_received,
        }
    }

    /// Update configuration
    pub fn update_config(&self, config: WebSocketSecurityConfig) {
        let mut current_config = self.config.write();
        *current_config = config;
    }

    /// Get configuration
    pub fn get_config(&self) -> WebSocketSecurityConfig {
        self.config.read().clone()
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// WebSocket security statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketSecurityStats {
    pub total_connections: usize,
    pub authenticated_connections: usize,
    pub unique_ips: usize,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_default_config() {
        let config = WebSocketSecurityConfig::default();
        assert!(config.enable_tls);
        assert!(config.enable_origin_validation);
        assert_eq!(config.max_message_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_secure_config() {
        let config = WebSocketSecurityConfig::secure();
        assert!(config.enable_tls);
        assert!(config.require_valid_cert);
        assert_eq!(config.allowed_tls_versions, vec![TlsVersion::TLS13]);
        assert!(config.enable_message_encryption);
    }

    #[test]
    fn test_permissive_config() {
        let config = WebSocketSecurityConfig::permissive();
        assert!(!config.enable_tls);
        assert!(!config.enable_origin_validation);
        assert!(!config.enable_message_encryption);
    }

    #[test]
    fn test_connection_info() {
        let ip = IpAddr::from_str("127.0.0.1").unwrap();
        let info = ConnectionInfo::new("conn-1".to_string(), ip);

        assert_eq!(info.connection_id, "conn-1");
        assert_eq!(info.ip_address, ip);
        assert!(!info.authenticated);
        assert_eq!(info.messages_sent, 0);
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = ConnectionRateLimiter::new(Duration::from_secs(60), 5);
        let ip = IpAddr::from_str("192.168.1.1").unwrap();

        // Should allow first 5 connections
        for _ in 0..5 {
            assert!(limiter.check_rate_limit(ip));
        }

        // Should block 6th connection
        assert!(!limiter.check_rate_limit(ip));
    }

    #[test]
    fn test_security_manager_origin_validation() {
        let mut config = WebSocketSecurityConfig::default();
        config.allowed_origins = vec!["https://example.com".to_string()];

        let manager = WebSocketSecurityManager::new(config);

        assert!(manager.validate_origin(Some("https://example.com")).is_ok());
        assert!(manager.validate_origin(Some("https://evil.com")).is_err());
    }

    #[test]
    fn test_security_manager_ip_blacklist() {
        let mut config = WebSocketSecurityConfig::default();
        let blocked_ip = IpAddr::from_str("10.0.0.1").unwrap();
        config.ip_blacklist = vec![blocked_ip];

        let manager = WebSocketSecurityManager::new(config);

        assert!(manager.check_ip_allowed(blocked_ip).is_err());
        assert!(manager
            .check_ip_allowed(IpAddr::from_str("10.0.0.2").unwrap())
            .is_ok());
    }

    #[test]
    fn test_message_size_validation() {
        let config = WebSocketSecurityConfig::default();
        let manager = WebSocketSecurityManager::new(config);

        assert!(manager.check_message_size(1000).is_ok());
        assert!(manager.check_message_size(100 * 1024 * 1024).is_err());
    }

    #[test]
    fn test_connection_registration() {
        let config = WebSocketSecurityConfig::default();
        let manager = WebSocketSecurityManager::new(config);

        let ip = IpAddr::from_str("192.168.1.1").unwrap();
        let conn_info = ConnectionInfo::new("conn-1".to_string(), ip);

        assert!(manager.register_connection(conn_info).is_ok());
        assert!(manager.get_connection("conn-1").is_some());

        assert!(manager.unregister_connection("conn-1").is_ok());
        assert!(manager.get_connection("conn-1").is_none());
    }
}
