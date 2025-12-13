// WebSocket Management Types
//
// Request and response types for WebSocket management endpoints

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use super::super::types::SessionId;

// ============================================================================
// WebSocket Server Status
// ============================================================================

/// Overall WebSocket server status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WebSocketStatus {
    /// Server status: healthy, degraded, unhealthy
    pub status: String,
    /// Total number of active WebSocket connections
    pub active_connections: usize,
    /// Total number of connections since server start
    pub total_connections_lifetime: u64,
    /// Total messages sent since server start
    pub messages_sent: u64,
    /// Total messages received since server start
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Number of active subscriptions
    pub active_subscriptions: usize,
    /// Server uptime in seconds
    pub uptime_seconds: u64,
    /// Maximum concurrent connections allowed
    pub max_connections: usize,
    /// Average message latency in milliseconds
    pub avg_message_latency_ms: f64,
    /// Connection error count
    pub error_count: u64,
}

// ============================================================================
// Connection Information
// ============================================================================

/// WebSocket connection information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConnectionInfo {
    /// Unique connection identifier
    pub connection_id: String,
    /// Remote client address
    pub remote_address: String,
    /// Connection protocol (ws or wss)
    pub protocol: String,
    /// Connection state: connected, closing, closed
    pub state: String,
    /// Associated session ID (if authenticated)
    pub session_id: Option<SessionId>,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// Timestamp when connection was established (Unix epoch)
    pub connected_at: i64,
    /// Messages sent on this connection
    pub messages_sent: u64,
    /// Messages received on this connection
    pub messages_received: u64,
    /// Bytes sent on this connection
    pub bytes_sent: u64,
    /// Bytes received on this connection
    pub bytes_received: u64,
    /// Last activity timestamp (Unix epoch)
    pub last_activity: i64,
    /// Active subscriptions on this connection
    pub subscriptions: Vec<String>,
    /// Client user agent (if available)
    pub user_agent: Option<String>,
    /// Connection metadata
    pub metadata: HashMap<String, String>,
}

/// List of WebSocket connections
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ConnectionList {
    /// Array of connection information
    pub connections: Vec<ConnectionInfo>,
    /// Total number of connections
    pub total: usize,
    /// Pagination offset
    pub offset: usize,
    /// Pagination limit
    pub limit: usize,
}

// ============================================================================
// Subscription Information
// ============================================================================

/// WebSocket subscription information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionInfo {
    /// Unique subscription identifier
    pub subscription_id: String,
    /// Connection ID this subscription belongs to
    pub connection_id: String,
    /// Subscription type (query, table, metrics, etc.)
    pub subscription_type: String,
    /// Subscription target (table name, metric name, etc.)
    pub target: String,
    /// Subscription filters (SQL WHERE clause, metric filters, etc.)
    pub filters: Option<String>,
    /// Timestamp when subscription was created (Unix epoch)
    pub created_at: i64,
    /// Number of messages sent for this subscription
    pub messages_sent: u64,
    /// Last message sent timestamp (Unix epoch)
    pub last_message_at: Option<i64>,
    /// Subscription status: active, paused, error
    pub status: String,
    /// Subscription configuration
    pub config: HashMap<String, serde_json::Value>,
}

/// List of WebSocket subscriptions
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionList {
    /// Array of subscription information
    pub subscriptions: Vec<SubscriptionInfo>,
    /// Total number of subscriptions
    pub total: usize,
    /// Pagination offset
    pub offset: usize,
    /// Pagination limit
    pub limit: usize,
}

/// Create subscription request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateSubscriptionRequest {
    /// Connection ID to create subscription on
    pub connection_id: String,
    /// Subscription type (query, table, metrics, logs, etc.)
    pub subscription_type: String,
    /// Subscription target (table name, metric name, etc.)
    pub target: String,
    /// Optional filters (SQL WHERE clause, metric filters, etc.)
    pub filters: Option<String>,
    /// Optional configuration parameters
    pub config: Option<HashMap<String, serde_json::Value>>,
}

/// Create subscription response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateSubscriptionResponse {
    /// Created subscription ID
    pub subscription_id: String,
    /// Subscription information
    pub subscription: SubscriptionInfo,
    /// Success message
    pub message: String,
}

// ============================================================================
// Broadcast & Messaging
// ============================================================================

/// Broadcast message to connections request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BroadcastRequest {
    /// Message to broadcast
    pub message: serde_json::Value,
    /// Message type/event name
    pub event: String,
    /// Target connections (empty = all connections)
    pub target_connections: Option<Vec<String>>,
    /// Filter by subscription type
    pub subscription_type: Option<String>,
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Additional metadata
    pub metadata: Option<HashMap<String, String>>,
}

/// Broadcast response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BroadcastResponse {
    /// Number of connections message was sent to
    pub sent_to_connections: usize,
    /// Connection IDs that received the message
    pub connection_ids: Vec<String>,
    /// Success message
    pub message: String,
    /// Timestamp of broadcast (Unix epoch)
    pub broadcast_at: i64,
}

// ============================================================================
// Disconnect & Control
// ============================================================================

/// Force disconnect request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DisconnectRequest {
    /// Reason for disconnection
    pub reason: Option<String>,
    /// Close code (1000 = normal, 1001 = going away, etc.)
    pub close_code: Option<u16>,
}

/// Disconnect response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DisconnectResponse {
    /// Connection ID that was disconnected
    pub connection_id: String,
    /// Success message
    pub message: String,
    /// Timestamp of disconnect (Unix epoch)
    pub disconnected_at: i64,
}

/// Delete subscription response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteSubscriptionResponse {
    /// Subscription ID that was deleted
    pub subscription_id: String,
    /// Success message
    pub message: String,
    /// Timestamp of deletion (Unix epoch)
    pub deleted_at: i64,
}
