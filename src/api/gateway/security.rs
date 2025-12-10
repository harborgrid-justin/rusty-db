// Gateway Module
//
// Part of the API Gateway and Security system for RustyDB

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::{Arc};
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use uuid::Uuid;

use crate::error::DbError;
use super::types::*;

// ============================================================================
// Security Features - Request Validation, Threat Detection
// ============================================================================

// Security filter
pub struct SecurityFilter {
    // Request validator
    validator: Arc<RequestValidator>,
    // Threat detector
    threat_detector: Arc<ThreatDetector>,
    // IP filter
    ip_filter: Arc<IpFilter>,
    // CSRF manager
    csrf_manager: Arc<CsrfManager>,
}

// Request validator
pub struct RequestValidator {
    // Maximum path length
    max_path_length: usize,
    // Maximum header size
    max_header_size: usize,
    // Allowed content types
    allowed_content_types: HashSet<String>,
}

// Threat detector
pub struct ThreatDetector {
    // SQL injection patterns
    sql_injection_patterns: Vec<regex::Regex>,
    // XSS patterns
    xss_patterns: Vec<regex::Regex>,
    // Path traversal patterns
    path_traversal_patterns: Vec<regex::Regex>,
    // Suspicious patterns
    #[allow(dead_code)]
    suspicious_patterns: Vec<regex::Regex>,
}

// IP filter
pub struct IpFilter {
    // Whitelist
    whitelist: Arc<RwLock<HashSet<IpAddr>>>,
    // Blacklist
    blacklist: Arc<RwLock<HashSet<IpAddr>>>,
    // Mode
    mode: IpFilterMode,
}

// IP filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpFilterMode {
    // Allow all except blacklisted
    Blacklist,
    // Deny all except whitelisted
    Whitelist,
    // No filtering
    None,
}

// CSRF manager
pub struct CsrfManager {
    // CSRF tokens
    tokens: Arc<RwLock<HashMap<String, CsrfToken>>>,
    // Token timeout (seconds)
    token_timeout: u64,
}

// CSRF token
#[derive(Debug, Clone)]
pub struct CsrfToken {
    // Token value
    pub value: String,
    // Created at
    pub created_at: Instant,
    // User session ID
    pub session_id: String,
}

impl SecurityFilter {
    // Create new security filter
    pub fn new() -> Self {
        Self {
            validator: Arc::new(RequestValidator::new()),
            threat_detector: Arc::new(ThreatDetector::new()),
            ip_filter: Arc::new(IpFilter::new(IpFilterMode::None)),
            csrf_manager: Arc::new(CsrfManager::new()),
        }
    }

    // Validate request
    pub fn validate_request(&self, request: &ApiRequest) -> Result<(), DbError> {
        // IP filtering
        self.ip_filter.check_ip(request.client_ip)?;

        // Request validation
        self.validator.validate(request)?;

        // Threat detection
        self.threat_detector.detect_threats(request)?;

        Ok(())
    }

    // Get IP filter
    pub fn get_ip_filter(&self) -> Arc<IpFilter> {
        Arc::clone(&self.ip_filter)
    }

    // Get CSRF manager
    pub fn get_csrf_manager(&self) -> Arc<CsrfManager> {
        Arc::clone(&self.csrf_manager)
    }
}

impl RequestValidator {
    fn new() -> Self {
        let mut allowed_content_types = HashSet::new();
        allowed_content_types.insert("application/json".to_string());
        allowed_content_types.insert("application/x-www-form-urlencoded".to_string());
        allowed_content_types.insert("multipart/form-data".to_string());
        allowed_content_types.insert("text/plain".to_string());

        Self {
            max_path_length: 2048,
            max_header_size: 8192,
            allowed_content_types,
        }
    }

    // Validate request
    fn validate(&self, request: &ApiRequest) -> Result<(), DbError> {
        // Check path length
        if request.path.len() > self.max_path_length {
            return Err(DbError::InvalidOperation("Path too long".to_string()));
        }

        // Check header sizes
        for (key, value) in &request.headers {
            if key.len() + value.len() > self.max_header_size {
                return Err(DbError::InvalidOperation("Header too large".to_string()));
            }
        }

        // Validate content type
        if let Some(content_type) = request.headers.get("Content-Type") {
            let ct = content_type.split(';').next().unwrap_or("");
            if !self.allowed_content_types.contains(ct) {
                return Err(DbError::InvalidOperation("Invalid content type".to_string()));
            }
        }

        Ok(())
    }
}

impl ThreatDetector {
    fn new() -> Self {
        let sql_injection_patterns = vec![
            regex::Regex::new(r"(?i)(union|select|insert|update|delete|drop|create|alter)\s").unwrap(),
            regex::Regex::new(r"(?i)(--|\|{2}|/\*|\*/)").unwrap(),
            regex::Regex::new(r"(?i)(xp_|sp_)").unwrap(),
        ];

        let xss_patterns = vec![
            regex::Regex::new(r"(?i)<script").unwrap(),
            regex::Regex::new(r"(?i)javascript:").unwrap(),
            regex::Regex::new(r"(?i)onerror\s*=").unwrap(),
            regex::Regex::new(r"(?i)onload\s*=").unwrap(),
        ];

        let path_traversal_patterns = vec![
            regex::Regex::new(r"\.\./").unwrap(),
            regex::Regex::new(r"\.\.\\").unwrap(),
        ];

        let suspicious_patterns = vec![
            regex::Regex::new(r"(?i)(cmd|exec|eval|system)").unwrap(),
        ];

        Self {
            sql_injection_patterns,
            xss_patterns,
            path_traversal_patterns,
            suspicious_patterns,
        }
    }

    // Detect threats in request
    fn detect_threats(&self, request: &ApiRequest) -> Result<(), DbError> {
        // Check path
        self.check_path_traversal(&request.path)?;

        // Check query parameters
        for (_, value) in &request.query_params {
            self.check_sql_injection(value)?;
            self.check_xss(value)?;
        }

        // Check headers
        for (_, value) in &request.headers {
            self.check_xss(value)?;
        }

        // Check body (if text)
        if let Ok(body_str) = std::str::from_utf8(&request.body) {
            self.check_sql_injection(body_str)?;
            self.check_xss(body_str)?;
        }

        Ok(())
    }

    // Check for SQL injection
    pub(crate) fn check_sql_injection(&self, input: &str) -> Result<(), DbError> {
        for pattern in &self.sql_injection_patterns {
            if pattern.is_match(input) {
                return Err(DbError::InvalidOperation("Potential SQL injection detected".to_string()));
            }
        }
        Ok(())
    }

    // Check for XSS
    pub(crate) fn check_xss(&self, input: &str) -> Result<(), DbError> {
        for pattern in &self.xss_patterns {
            if pattern.is_match(input) {
                return Err(DbError::InvalidOperation("Potential XSS attack detected".to_string()));
            }
        }
        Ok(())
    }

    // Check for path traversal
    fn check_path_traversal(&self, input: &str) -> Result<(), DbError> {
        for pattern in &self.path_traversal_patterns {
            if pattern.is_match(input) {
                return Err(DbError::InvalidOperation("Path traversal attempt detected".to_string()));
            }
        }
        Ok(())
    }
}

impl IpFilter {
    fn new(mode: IpFilterMode) -> Self {
        Self {
            whitelist: Arc::new(RwLock::new(HashSet::new())),
            blacklist: Arc::new(RwLock::new(HashSet::new())),
            mode,
        }
    }

    // Check IP address
    pub(crate) fn check_ip(&self, ip: IpAddr) -> Result<(), DbError> {
        match self.mode {
            IpFilterMode::None => Ok(()),
            IpFilterMode::Blacklist => {
                let blacklist = self.blacklist.read();
                if blacklist.contains(&ip) {
                    Err(DbError::InvalidOperation("IP address blacklisted".to_string()))
                } else {
                    Ok(())
                }
            },
            IpFilterMode::Whitelist => {
                let whitelist = self.whitelist.read();
                if whitelist.contains(&ip) {
                    Ok(())
                } else {
                    Err(DbError::InvalidOperation("IP address not whitelisted".to_string()))
                }
            },
        }
    }

    // Add to whitelist
    pub fn add_to_whitelist(&self, ip: IpAddr) {
        let mut whitelist = self.whitelist.write();
        whitelist.insert(ip);
    }

    // Remove from whitelist
    pub fn remove_from_whitelist(&self, ip: IpAddr) -> bool {
        let mut whitelist = self.whitelist.write();
        whitelist.remove(&ip)
    }

    // Add to blacklist
    pub fn add_to_blacklist(&self, ip: IpAddr) {
        let mut blacklist = self.blacklist.write();
        blacklist.insert(ip);
    }

    // Remove from blacklist
    pub fn remove_from_blacklist(&self, ip: IpAddr) -> bool {
        let mut blacklist = self.blacklist.write();
        blacklist.remove(&ip)
    }

    // Set filter mode
    pub fn set_mode(&mut self, mode: IpFilterMode) {
        self.mode = mode;
    }
}

impl CsrfManager {
    fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            token_timeout: 3600,
        }
    }

    // Generate CSRF token
    pub fn generate_token(&self, session_id: String) -> String {
        let token_value = Uuid::new_v4().to_string();

        let token = CsrfToken {
            value: token_value.clone(),
            created_at: Instant::now(),
            session_id,
        };

        let mut tokens = self.tokens.write();
        tokens.insert(token_value.clone(), token);

        token_value
    }

    // Validate CSRF token
    pub fn validate_token(&self, token_value: &str, session_id: &str) -> bool {
        let tokens = self.tokens.read();

        if let Some(token) = tokens.get(token_value) {
            // Check session ID
            if token.session_id != session_id {
                return false;
            }

            // Check expiration
            let age = Instant::now().duration_since(token.created_at);
            if age.as_secs() > self.token_timeout {
                return false;
            }

            true
        } else {
            false
        }
    }

    // Cleanup expired tokens
    pub fn cleanup_expired_tokens(&self) {
        let mut tokens = self.tokens.write();
        let timeout = Duration::from_secs(self.token_timeout);

        tokens.retain(|_, token| {
            Instant::now().duration_since(token.created_at) < timeout
        });
    }
}
