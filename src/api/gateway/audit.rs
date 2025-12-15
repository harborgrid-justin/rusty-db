// Gateway Module
//
// Part of the API Gateway and Security system for RustyDB

use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::time::SystemTime;
use uuid::Uuid;

use super::types::*;

// ============================================================================
// Audit Logging
// ============================================================================

// Audit logger
pub struct AuditLogger {
    // Audit events
    events: VecDeque<AuditEvent>,
    // Max events to keep in memory
    max_events: usize,
}

// Audit event
#[derive(Debug, Clone)]
pub struct AuditEvent {
    // Event ID
    pub id: String,
    // Timestamp
    pub timestamp: SystemTime,
    // Event type
    pub event_type: AuditEventType,
    // User ID
    pub user_id: Option<String>,
    // Client IP
    pub client_ip: IpAddr,
    // Request ID
    pub request_id: String,
    // Resource
    pub resource: String,
    // Action
    pub action: String,
    // Result
    pub result: AuditResult,
    // Details
    pub details: HashMap<String, String>,
}

// Audit event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEventType {
    Authentication,
    Authorization,
    DataAccess,
    DataModification,
    AdminAction,
    SecurityEvent,
    ConfigChange,
}

// Audit result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditResult {
    Success,
    Failure,
    Denied,
}

// Security event
#[derive(Debug, Clone)]
pub struct SecurityEvent {
    // Event type
    pub event_type: SecurityEventType,
    // Request ID
    pub request_id: String,
    // Client IP
    pub client_ip: IpAddr,
    // Reason
    pub reason: String,
    // Timestamp
    pub timestamp: SystemTime,
}

// Security event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityEventType {
    AuthenticationFailed,
    AuthorizationFailed,
    RequestBlocked,
    RateLimitExceeded,
    SuspiciousActivity,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            max_events: 10000,
        }
    }

    // Log request
    pub(crate) fn log_request(&mut self, request: &ApiRequest) {
        let event = AuditEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            event_type: AuditEventType::DataAccess,
            user_id: None,
            client_ip: request.client_ip,
            request_id: request.request_id.clone(),
            resource: request.path.clone(),
            action: format!("{:?}", request.method),
            result: AuditResult::Success,
            details: HashMap::new(),
        };

        self.add_event(event);
    }

    // Log security event
    pub(crate) fn log_security_event(&mut self, event: &SecurityEvent) {
        let audit_event = AuditEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: event.timestamp,
            event_type: AuditEventType::SecurityEvent,
            user_id: None,
            client_ip: event.client_ip,
            request_id: event.request_id.clone(),
            resource: String::new(),
            action: format!("{:?}", event.event_type),
            result: AuditResult::Denied,
            details: {
                let mut details = HashMap::new();
                details.insert("reason".to_string(), event.reason.clone());
                details
            },
        };

        self.add_event(audit_event);
    }

    // Add event
    fn add_event(&mut self, event: AuditEvent) {
        if self.events.len() >= self.max_events {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    // Get recent events
    pub fn get_recent_events(&self, count: usize) -> Vec<AuditEvent> {
        self.events.iter().rev().take(count).cloned().collect()
    }

    // Search events
    pub fn search_events(&self, filter: &AuditEventFilter) -> Vec<AuditEvent> {
        self.events
            .iter()
            .filter(|e| filter.matches(e))
            .cloned()
            .collect()
    }
}

// Audit event filter
#[derive(Debug, Default)]
pub struct AuditEventFilter {
    // Filter by event type
    pub event_type: Option<AuditEventType>,
    // Filter by user ID
    pub user_id: Option<String>,
    // Filter by client IP
    pub client_ip: Option<IpAddr>,
    // Filter by time range
    pub time_range: Option<(SystemTime, SystemTime)>,
    // Filter by result
    pub result: Option<AuditResult>,
}

impl AuditEventFilter {
    fn matches(&self, event: &AuditEvent) -> bool {
        if let Some(event_type) = self.event_type {
            if event.event_type != event_type {
                return false;
            }
        }

        if let Some(ref user_id) = self.user_id {
            if event.user_id.as_ref() != Some(user_id) {
                return false;
            }
        }

        if let Some(client_ip) = self.client_ip {
            if event.client_ip != client_ip {
                return false;
            }
        }

        if let Some((start, end)) = self.time_range {
            if event.timestamp < start || event.timestamp > end {
                return false;
            }
        }

        if let Some(result) = self.result {
            if event.result != result {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::gateway::{
        IpFilter, IpFilterMode, RbacManager, Role, SlidingWindow, ThreatDetector, TokenBucket,
    };

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(0, 0.0);

        // Should allow 10 requests initially
        for _ in 0..10 {
            assert!(bucket.consume(1).is_ok());
        }

        // 11th request should fail
        assert!(bucket.consume(1).is_err());
    }

    #[test]
    fn test_sliding_window() {
        let mut window = SlidingWindow::new(0, 0);

        // Should allow 10 requests
        for _ in 0..10 {
            assert!(window.allow_request().is_ok());
        }

        // 11th request should fail
        assert!(window.allow_request().is_err());
    }

    #[test]
    fn test_threat_detection() {
        let detector = ThreatDetector::new();

        // Should detect SQL injection
        assert!(detector.check_sql_injection("' OR '1'='1").is_err());
        assert!(detector
            .check_sql_injection("UNION SELECT * FROM users")
            .is_err());

        // Should detect XSS
        assert!(detector.check_xss("<script>alert('xss')</script>").is_err());
        assert!(detector.check_xss("javascript:alert(1)").is_err());

        // Should allow safe input
        assert!(detector.check_sql_injection("normal text").is_ok());
        assert!(detector.check_xss("normal text").is_ok());
    }

    #[test]
    fn test_ip_filter() {
        let filter = IpFilter::new(IpFilterMode::Blacklist);
        let test_ip = "192.168.1.1".parse().unwrap();

        // Should allow initially
        assert!(filter.check_ip(test_ip).is_ok());

        // Add to blacklist
        filter.add_to_blacklist(test_ip);

        // Should block now
        assert!(filter.check_ip(test_ip).is_err());
    }

    #[test]
    fn test_rbac() {
        let rbac = RbacManager::new();

        // Create role
        let role = Role {
            id: "admin".to_string(),
            name: "Administrator".to_string(),
            description: "Full access".to_string(),
            permissions: vec!["read".to_string(), "write".to_string()],
            parent_roles: Vec::new(),
            created_at: SystemTime::now(),
        };

        rbac.create_role(role);

        // Assign to user
        rbac.assign_role("user1".to_string(), "admin".to_string());

        // Check permissions
        assert!(rbac.has_permission("user1", "read"));
        assert!(rbac.has_permission("user1", "write"));
        assert!(!rbac.has_permission("user1", "delete"));
    }
}
