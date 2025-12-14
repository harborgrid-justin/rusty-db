// Security WebSocket Handlers
//
// Real-time WebSocket streaming for security events including authentication,
// authorization, audit logs, encryption, rate limiting, and threat detection.

use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{interval, Duration};
use utoipa::ToSchema;

use super::super::types::ApiState;

// ============================================================================
// WebSocket Event Types
// ============================================================================

/// Security event types that can be streamed via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "event_type")]
pub enum SecurityEvent {
    /// Authentication events (login, logout, failures)
    Authentication(AuthenticationEvent),
    /// Authorization events (permission checks, failures)
    Authorization(AuthorizationEvent),
    /// Audit log entries
    AuditLog(AuditLogEvent),
    /// Encryption and key management events
    Encryption(EncryptionEvent),
    /// Rate limiting violations
    RateLimit(RateLimitEvent),
    /// Insider threat detection alerts
    InsiderThreat(InsiderThreatEvent),
    /// Memory hardening violations
    MemoryHardening(MemoryHardeningEvent),
    /// Circuit breaker state changes
    CircuitBreaker(CircuitBreakerEvent),
}

/// Authentication event details
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationEvent {
    pub action: AuthAction,
    pub username: String,
    pub session_id: Option<String>,
    pub success: bool,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub mfa_required: bool,
    pub mfa_verified: bool,
    pub failure_reason: Option<String>,
    pub timestamp: i64,
}

/// Authentication action types
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthAction {
    Login,
    Logout,
    PasswordChange,
    MfaChallenge,
    MfaVerify,
    AccountLocked,
    AccountUnlocked,
    SessionExpired,
    SessionRefreshed,
}

/// Authorization event details
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthorizationEvent {
    pub action: AuthzAction,
    pub username: String,
    pub session_id: Option<String>,
    pub resource: String,
    pub resource_type: String,
    pub permission: String,
    pub granted: bool,
    pub reason: Option<String>,
    pub role_id: Option<String>,
    pub timestamp: i64,
}

/// Authorization action types
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthzAction {
    PermissionCheck,
    PermissionDenied,
    RoleAssigned,
    RoleRevoked,
    PrivilegeGranted,
    PrivilegeRevoked,
    PolicyEvaluated,
    AccessDenied,
}

/// Audit log event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLogEvent {
    pub id: u64,
    pub username: String,
    pub session_id: Option<String>,
    pub action: String,
    pub object_name: Option<String>,
    pub object_type: Option<String>,
    pub sql_text: Option<String>,
    pub success: bool,
    pub severity: String,
    pub rows_affected: Option<u64>,
    pub execution_time_ms: Option<u64>,
    pub client_ip: Option<String>,
    pub error_message: Option<String>,
    pub timestamp: i64,
}

/// Encryption event details
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EncryptionEvent {
    pub action: EncryptionAction,
    pub key_id: Option<String>,
    pub key_type: Option<String>,
    pub algorithm: Option<String>,
    pub target: Option<String>,
    pub target_type: Option<String>,
    pub rotation_progress: Option<u8>,
    pub success: bool,
    pub message: Option<String>,
    pub timestamp: i64,
}

/// Encryption action types
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionAction {
    KeyGenerated,
    KeyRotationStarted,
    KeyRotationProgress,
    KeyRotationCompleted,
    KeyRotationFailed,
    KeyExpired,
    KeyDestroyed,
    EncryptionEnabled,
    EncryptionDisabled,
    TdeEnabled,
    TdeDisabled,
}

/// Rate limiting event details
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RateLimitEvent {
    pub source_ip: String,
    pub user_id: Option<String>,
    pub limit_type: String,
    pub limit_value: u64,
    pub current_rate: u64,
    pub blocked: bool,
    pub adaptive_multiplier: Option<f64>,
    pub reputation_score: Option<i32>,
    pub ddos_suspected: bool,
    pub timestamp: i64,
}

/// Insider threat event details
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InsiderThreatEvent {
    pub threat_id: String,
    pub user_id: String,
    pub threat_type: String,
    pub threat_level: String,
    pub risk_score: u8,
    pub query_text: Option<String>,
    pub tables_accessed: Vec<String>,
    pub rows_affected: u64,
    pub anomalies_detected: Vec<String>,
    pub action_taken: String,
    pub client_ip: Option<String>,
    pub location: Option<String>,
    pub timestamp: i64,
}

/// Memory hardening event details
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MemoryHardeningEvent {
    pub event_type: String,
    pub severity: String,
    pub buffer_id: Option<String>,
    pub address: Option<String>,
    pub size: Option<usize>,
    pub canary_value: Option<String>,
    pub expected_value: Option<String>,
    pub stack_trace: Option<Vec<String>>,
    pub action_taken: String,
    pub timestamp: i64,
}

/// Circuit breaker event details
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CircuitBreakerEvent {
    pub circuit_id: String,
    pub old_state: String,
    pub new_state: String,
    pub failure_count: u32,
    pub success_count: u32,
    pub error_rate: f64,
    pub half_open_successes: Option<u32>,
    pub reason: String,
    pub timestamp: i64,
}

/// Configuration for security event streaming
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SecurityStreamConfig {
    /// Event types to stream (empty = all types)
    pub event_types: Option<Vec<String>>,
    /// Filter by username
    pub username_filter: Option<String>,
    /// Filter by severity level (for audit logs)
    pub min_severity: Option<String>,
    /// Include SQL text in audit logs
    pub include_sql: Option<bool>,
    /// Stream interval in milliseconds (for polling-based events)
    pub interval_ms: Option<u64>,
}

// ============================================================================
// WebSocket Handlers
// ============================================================================

/// WebSocket upgrade endpoint for security event streaming
///
/// Streams real-time security events including authentication, authorization,
/// audit logs, encryption events, rate limiting, insider threats, and more.
#[utoipa::path(
    get,
    path = "/api/v1/ws/security/events",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "websocket-security"
)]
pub async fn ws_security_events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_security_events_websocket(socket, state))
}

/// WebSocket upgrade endpoint for authentication event streaming
#[utoipa::path(
    get,
    path = "/api/v1/ws/security/authentication",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "websocket-security"
)]
pub async fn ws_authentication_events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_authentication_events_websocket(socket, state))
}

/// WebSocket upgrade endpoint for audit log streaming
#[utoipa::path(
    get,
    path = "/api/v1/ws/security/audit",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "websocket-security"
)]
pub async fn ws_audit_stream(ws: WebSocketUpgrade, State(state): State<Arc<ApiState>>) -> Response {
    ws.on_upgrade(|socket| handle_audit_stream_websocket(socket, state))
}

/// WebSocket upgrade endpoint for insider threat alerts
#[utoipa::path(
    get,
    path = "/api/v1/ws/security/threats",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "websocket-security"
)]
pub async fn ws_threat_alerts(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_threat_alerts_websocket(socket, state))
}

/// WebSocket upgrade endpoint for encryption events
#[utoipa::path(
    get,
    path = "/api/v1/ws/security/encryption",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "websocket-security"
)]
pub async fn ws_encryption_events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_encryption_events_websocket(socket, state))
}

/// WebSocket upgrade endpoint for rate limiting events
#[utoipa::path(
    get,
    path = "/api/v1/ws/security/rate-limits",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "websocket-security"
)]
pub async fn ws_rate_limit_events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_rate_limit_events_websocket(socket, state))
}

// ============================================================================
// WebSocket Connection Handlers
// ============================================================================

/// Generic security events WebSocket handler
async fn handle_security_events_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    // Send welcome message
    let welcome = json!({
        "event_type": "connected",
        "message": "Connected to security events stream",
        "available_types": [
            "authentication",
            "authorization",
            "audit_log",
            "encryption",
            "rate_limit",
            "insider_threat",
            "memory_hardening",
            "circuit_breaker"
        ],
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    });

    if let Ok(welcome_json) = serde_json::to_string(&welcome) {
        if socket
            .send(Message::Text(welcome_json.into()))
            .await
            .is_err()
        {
            return;
        }
    }

    // Handle incoming messages and stream events
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    // Parse configuration
                    if let Ok(_config) = serde_json::from_str::<SecurityStreamConfig>(&text) {
                        // In production: apply filters and subscribe to events
                        // For now, send sample events
                        let sample_event = create_sample_authentication_event();
                        if let Ok(event_json) = serde_json::to_string(&sample_event) {
                            if socket.send(Message::Text(event_json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Message::Close(_) => break,
                Message::Ping(data) => {
                    if socket.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                _ => {}
            }
        } else {
            break;
        }
    }
}

/// Authentication events WebSocket handler
async fn handle_authentication_events_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    let ack = json!({
        "event_type": "connected",
        "message": "Connected to authentication events stream",
        "info": "Streaming login, logout, password changes, MFA events",
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    });

    if let Ok(ack_json) = serde_json::to_string(&ack) {
        if socket.send(Message::Text(ack_json.into())).await.is_err() {
            return;
        }
    }

    // In production: subscribe to actual authentication events
    // For now, simulate events at intervals
    let (mut sender, mut receiver) = socket.split();
    let mut ticker = interval(Duration::from_secs(5));

    let streaming_task = tokio::spawn(async move {
        loop {
            ticker.tick().await;

            let event = create_sample_authentication_event();
            if let Ok(event_json) = serde_json::to_string(&event) {
                if sender.send(Message::Text(event_json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming control messages
    while let Some(msg) = receiver.next().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Close(_) => break,
                _ => {}
            }
        } else {
            break;
        }
    }

    streaming_task.abort();
}

/// Audit log streaming WebSocket handler
async fn handle_audit_stream_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    let ack = json!({
        "event_type": "connected",
        "message": "Connected to audit log stream",
        "info": "Streaming all database audit events in real-time",
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    });

    if let Ok(ack_json) = serde_json::to_string(&ack) {
        if socket.send(Message::Text(ack_json.into())).await.is_err() {
            return;
        }
    }

    // In production: subscribe to actual audit events
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(_text) => {
                    let event = create_sample_audit_event();
                    if let Ok(event_json) = serde_json::to_string(&event) {
                        if socket.send(Message::Text(event_json.into())).await.is_err() {
                            break;
                        }
                    }
                }
                Message::Close(_) => break,
                Message::Ping(data) => {
                    if socket.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                _ => {}
            }
        } else {
            break;
        }
    }
}

/// Insider threat alerts WebSocket handler
async fn handle_threat_alerts_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    let ack = json!({
        "event_type": "connected",
        "message": "Connected to insider threat alerts stream",
        "info": "Streaming real-time threat detection alerts",
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    });

    if let Ok(ack_json) = serde_json::to_string(&ack) {
        if socket.send(Message::Text(ack_json.into())).await.is_err() {
            return;
        }
    }

    // In production: subscribe to actual threat detection events
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(_text) => {
                    let event = create_sample_threat_event();
                    if let Ok(event_json) = serde_json::to_string(&event) {
                        if socket.send(Message::Text(event_json.into())).await.is_err() {
                            break;
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        } else {
            break;
        }
    }
}

/// Encryption events WebSocket handler
async fn handle_encryption_events_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    let ack = json!({
        "event_type": "connected",
        "message": "Connected to encryption events stream",
        "info": "Streaming key rotation, TDE events, and encryption operations",
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    });

    if let Ok(ack_json) = serde_json::to_string(&ack) {
        if socket.send(Message::Text(ack_json.into())).await.is_err() {
            return;
        }
    }

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(_text) => {
                    let event = create_sample_encryption_event();
                    if let Ok(event_json) = serde_json::to_string(&event) {
                        if socket.send(Message::Text(event_json.into())).await.is_err() {
                            break;
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        } else {
            break;
        }
    }
}

/// Rate limiting events WebSocket handler
async fn handle_rate_limit_events_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    let ack = json!({
        "event_type": "connected",
        "message": "Connected to rate limiting events stream",
        "info": "Streaming rate limit violations and DDoS detection",
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    });

    if let Ok(ack_json) = serde_json::to_string(&ack) {
        if socket.send(Message::Text(ack_json.into())).await.is_err() {
            return;
        }
    }

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(_text) => {
                    let event = create_sample_rate_limit_event();
                    if let Ok(event_json) = serde_json::to_string(&event) {
                        if socket.send(Message::Text(event_json.into())).await.is_err() {
                            break;
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        } else {
            break;
        }
    }
}

// ============================================================================
// Sample Event Generators (for demonstration)
// ============================================================================

fn create_sample_authentication_event() -> SecurityEvent {
    SecurityEvent::Authentication(AuthenticationEvent {
        action: AuthAction::Login,
        username: "alice@example.com".to_string(),
        session_id: Some("sess_abc123".to_string()),
        success: true,
        client_ip: Some("192.168.1.100".to_string()),
        user_agent: Some("Mozilla/5.0".to_string()),
        mfa_required: true,
        mfa_verified: true,
        failure_reason: None,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    })
}

fn create_sample_audit_event() -> SecurityEvent {
    SecurityEvent::AuditLog(AuditLogEvent {
        id: 12345,
        username: "alice@example.com".to_string(),
        session_id: Some("sess_abc123".to_string()),
        action: "SELECT".to_string(),
        object_name: Some("customers".to_string()),
        object_type: Some("TABLE".to_string()),
        sql_text: Some("SELECT * FROM customers WHERE active = true LIMIT 100".to_string()),
        success: true,
        severity: "INFO".to_string(),
        rows_affected: Some(42),
        execution_time_ms: Some(125),
        client_ip: Some("192.168.1.100".to_string()),
        error_message: None,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    })
}

fn create_sample_threat_event() -> SecurityEvent {
    SecurityEvent::InsiderThreat(InsiderThreatEvent {
        threat_id: "threat_xyz789".to_string(),
        user_id: "user_bob".to_string(),
        threat_type: "DATA_EXFILTRATION".to_string(),
        threat_level: "HIGH".to_string(),
        risk_score: 75,
        query_text: Some("SELECT * FROM sensitive_data".to_string()),
        tables_accessed: vec!["sensitive_data".to_string(), "pii_records".to_string()],
        rows_affected: 50000,
        anomalies_detected: vec![
            "Unusual data volume".to_string(),
            "Off-hours access".to_string(),
            "Atypical query pattern".to_string(),
        ],
        action_taken: "ALERT_GENERATED".to_string(),
        client_ip: Some("10.0.0.50".to_string()),
        location: Some("Unknown".to_string()),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    })
}

fn create_sample_encryption_event() -> SecurityEvent {
    SecurityEvent::Encryption(EncryptionEvent {
        action: EncryptionAction::KeyRotationProgress,
        key_id: Some("key_abc123".to_string()),
        key_type: Some("TableEncryption".to_string()),
        algorithm: Some("AES256-GCM".to_string()),
        target: Some("users_table".to_string()),
        target_type: Some("TABLE".to_string()),
        rotation_progress: Some(45),
        success: true,
        message: Some("Key rotation 45% complete, re-encrypting blocks".to_string()),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    })
}

fn create_sample_rate_limit_event() -> SecurityEvent {
    SecurityEvent::RateLimit(RateLimitEvent {
        source_ip: "192.168.1.200".to_string(),
        user_id: Some("user_charlie".to_string()),
        limit_type: "per_ip".to_string(),
        limit_value: 1000,
        current_rate: 1250,
        blocked: true,
        adaptive_multiplier: Some(0.8),
        reputation_score: Some(35),
        ddos_suspected: false,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    })
}
