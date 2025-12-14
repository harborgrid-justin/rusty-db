// # WebSocket Message Handling
//
// This module provides message types and serialization/deserialization for WebSocket
// communication in RustyDB.
//
// ## Message Types
//
// - **Text**: UTF-8 text messages (typically JSON)
// - **Binary**: Binary protocol messages
// - **Ping/Pong**: Connection keepalive
// - **Close**: Graceful connection closure
//
// ## Usage
//
// ```rust
// use rusty_db::websocket::message::{WebSocketMessage, MessagePayload};
//
// // Create a text message
// let msg = WebSocketMessage::text("Hello, WebSocket!");
//
// // Create a binary message
// let data = vec![1, 2, 3, 4];
// let msg = WebSocketMessage::binary(data);
//
// // Serialize to JSON
// let json = serde_json::to_string(&msg)?;
// ```

use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

/// Query result for WebSocket messages (simplified)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsQueryResult {
    pub rows_affected: u64,
    pub columns: Vec<String>,
    pub data: Vec<Vec<serde_json::Value>>,
}

// ============================================================================
// Message Types
// ============================================================================

/// WebSocket message types supported by RustyDB
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum WebSocketMessage {
    /// Text message (UTF-8 encoded)
    Text(String),

    /// Binary message (raw bytes)
    Binary(Vec<u8>),

    /// Ping message for connection keepalive
    Ping(Vec<u8>),

    /// Pong response to ping
    Pong(Vec<u8>),

    /// Close message with optional reason
    Close { code: u16, reason: String },

    /// Query request message
    Query {
        id: String,
        sql: String,
        params: Option<Vec<serde_json::Value>>,
    },

    /// Query result message
    QueryResult { id: String, result: WsQueryResult },

    /// Error message
    Error {
        id: Option<String>,
        code: String,
        message: String,
    },

    /// Subscription message
    Subscribe {
        id: String,
        topic: String,
        filter: Option<serde_json::Value>,
    },

    /// Unsubscribe message
    Unsubscribe { id: String },

    /// Event notification
    Event {
        topic: String,
        data: serde_json::Value,
    },
}

impl WebSocketMessage {
    /// Create a new text message
    pub fn text<S: Into<String>>(content: S) -> Self {
        WebSocketMessage::Text(content.into())
    }

    /// Create a new binary message
    pub fn binary(data: Vec<u8>) -> Self {
        WebSocketMessage::Binary(data)
    }

    /// Create a ping message
    pub fn ping(data: Vec<u8>) -> Self {
        WebSocketMessage::Ping(data)
    }

    /// Create a pong message
    pub fn pong(data: Vec<u8>) -> Self {
        WebSocketMessage::Pong(data)
    }

    /// Create a close message
    pub fn close(code: u16, reason: String) -> Self {
        WebSocketMessage::Close { code, reason }
    }

    /// Create a query message
    pub fn query<S: Into<String>>(id: S, sql: S, params: Option<Vec<serde_json::Value>>) -> Self {
        WebSocketMessage::Query {
            id: id.into(),
            sql: sql.into(),
            params,
        }
    }

    /// Create a query result message
    pub fn query_result<S: Into<String>>(id: S, result: WsQueryResult) -> Self {
        WebSocketMessage::QueryResult {
            id: id.into(),
            result,
        }
    }

    /// Create an error message
    pub fn error<S: Into<String>>(id: Option<String>, code: S, message: S) -> Self {
        WebSocketMessage::Error {
            id,
            code: code.into(),
            message: message.into(),
        }
    }

    /// Create a subscribe message
    pub fn subscribe<S: Into<String>>(id: S, topic: S, filter: Option<serde_json::Value>) -> Self {
        WebSocketMessage::Subscribe {
            id: id.into(),
            topic: topic.into(),
            filter,
        }
    }

    /// Create an unsubscribe message
    pub fn unsubscribe<S: Into<String>>(id: S) -> Self {
        WebSocketMessage::Unsubscribe { id: id.into() }
    }

    /// Create an event notification message
    pub fn event<S: Into<String>>(topic: S, data: serde_json::Value) -> Self {
        WebSocketMessage::Event {
            topic: topic.into(),
            data,
        }
    }

    /// Check if this is a control message (ping, pong, close)
    pub fn is_control(&self) -> bool {
        matches!(
            self,
            WebSocketMessage::Ping(_) | WebSocketMessage::Pong(_) | WebSocketMessage::Close { .. }
        )
    }

    /// Get message type as string
    pub fn message_type(&self) -> &str {
        match self {
            WebSocketMessage::Text(_) => "text",
            WebSocketMessage::Binary(_) => "binary",
            WebSocketMessage::Ping(_) => "ping",
            WebSocketMessage::Pong(_) => "pong",
            WebSocketMessage::Close { .. } => "close",
            WebSocketMessage::Query { .. } => "query",
            WebSocketMessage::QueryResult { .. } => "query_result",
            WebSocketMessage::Error { .. } => "error",
            WebSocketMessage::Subscribe { .. } => "subscribe",
            WebSocketMessage::Unsubscribe { .. } => "unsubscribe",
            WebSocketMessage::Event { .. } => "event",
        }
    }
}

// ============================================================================
// Message Envelope for Routing and Metadata
// ============================================================================

/// Message envelope with routing and metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// Message ID for correlation
    pub id: String,

    /// Source connection ID
    pub from: Option<String>,

    /// Destination connection ID (None for broadcast)
    pub to: Option<String>,

    /// Timestamp when message was created
    pub timestamp: u64,

    /// Message payload
    pub payload: WebSocketMessage,

    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

impl MessageEnvelope {
    /// Create a new message envelope
    pub fn new(payload: WebSocketMessage) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from: None,
            to: None,
            timestamp,
            payload,
            metadata: None,
        }
    }

    /// Set source connection ID
    pub fn from<S: Into<String>>(mut self, from: S) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Set destination connection ID
    pub fn to<S: Into<String>>(mut self, to: S) -> Self {
        self.to = Some(to.into());
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Check if this is a broadcast message
    pub fn is_broadcast(&self) -> bool {
        self.to.is_none()
    }
}

// ============================================================================
// Message Serialization/Deserialization
// ============================================================================

/// Message codec for serialization/deserialization
pub struct MessageCodec;

impl MessageCodec {
    /// Serialize a WebSocket message to Tungstenite message
    pub fn encode(msg: &WebSocketMessage) -> Result<TungsteniteMessage> {
        match msg {
            WebSocketMessage::Text(text) => Ok(TungsteniteMessage::Text(text.clone().into())),
            WebSocketMessage::Binary(data) => Ok(TungsteniteMessage::Binary(data.clone().into())),
            WebSocketMessage::Ping(data) => Ok(TungsteniteMessage::Ping(data.clone().into())),
            WebSocketMessage::Pong(data) => Ok(TungsteniteMessage::Pong(data.clone().into())),
            WebSocketMessage::Close { code, reason } => Ok(TungsteniteMessage::Close(Some(
                tokio_tungstenite::tungstenite::protocol::CloseFrame {
                    code: (*code).into(),
                    reason: reason.clone().into(),
                },
            ))),
            // For structured messages, serialize to JSON
            _ => {
                let json = serde_json::to_string(msg)
                    .map_err(|e| DbError::Serialization(e.to_string()))?;
                Ok(TungsteniteMessage::Text(json.into()))
            }
        }
    }

    /// Deserialize a Tungstenite message to WebSocket message
    pub fn decode(msg: TungsteniteMessage) -> Result<WebSocketMessage> {
        match msg {
            TungsteniteMessage::Text(text) => {
                let text_str = text.to_string();
                // Try to parse as JSON first
                if let Ok(parsed) = serde_json::from_str::<WebSocketMessage>(&text_str) {
                    Ok(parsed)
                } else {
                    // Fallback to plain text
                    Ok(WebSocketMessage::Text(text_str))
                }
            }
            TungsteniteMessage::Binary(data) => Ok(WebSocketMessage::Binary(data.to_vec())),
            TungsteniteMessage::Ping(data) => Ok(WebSocketMessage::Ping(data.to_vec())),
            TungsteniteMessage::Pong(data) => Ok(WebSocketMessage::Pong(data.to_vec())),
            TungsteniteMessage::Close(frame) => {
                if let Some(frame) = frame {
                    Ok(WebSocketMessage::Close {
                        code: frame.code.into(),
                        reason: frame.reason.to_string(),
                    })
                } else {
                    Ok(WebSocketMessage::Close {
                        code: 1000,
                        reason: String::new(),
                    })
                }
            }
            TungsteniteMessage::Frame(_) => Err(DbError::InvalidInput(
                "Unexpected frame message".to_string(),
            )),
        }
    }

    /// Encode a message envelope
    pub fn encode_envelope(envelope: &MessageEnvelope) -> Result<TungsteniteMessage> {
        let json =
            serde_json::to_string(envelope).map_err(|e| DbError::Serialization(e.to_string()))?;
        Ok(TungsteniteMessage::Text(json.into()))
    }

    /// Decode a message envelope
    pub fn decode_envelope(msg: TungsteniteMessage) -> Result<MessageEnvelope> {
        match msg {
            TungsteniteMessage::Text(text) => {
                serde_json::from_str::<MessageEnvelope>(&text.to_string())
                    .map_err(|e| DbError::Serialization(e.to_string()))
            }
            _ => Err(DbError::InvalidInput(
                "Envelopes must be text messages".to_string(),
            )),
        }
    }
}

// ============================================================================
// Message Router
// ============================================================================

/// Routes messages to appropriate handlers
pub struct MessageRouter {
    /// Registered message handlers by message type
    handlers: std::collections::HashMap<String, Vec<MessageHandler>>,
}

/// Message handler function type
pub type MessageHandler =
    Box<dyn Fn(&WebSocketMessage) -> Result<Option<WebSocketMessage>> + Send + Sync>;

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self {
            handlers: std::collections::HashMap::new(),
        }
    }

    /// Register a handler for a specific message type
    pub fn register<F>(&mut self, message_type: &str, handler: F)
    where
        F: Fn(&WebSocketMessage) -> Result<Option<WebSocketMessage>> + Send + Sync + 'static,
    {
        self.handlers
            .entry(message_type.to_string())
            .or_insert_with(Vec::new)
            .push(Box::new(handler));
    }

    /// Route a message to registered handlers
    pub fn route(&self, msg: &WebSocketMessage) -> Result<Vec<WebSocketMessage>> {
        let msg_type = msg.message_type();
        let mut responses = Vec::new();

        if let Some(handlers) = self.handlers.get(msg_type) {
            for handler in handlers {
                if let Some(response) = handler(msg)? {
                    responses.push(response);
                }
            }
        }

        Ok(responses)
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = WebSocketMessage::text("Hello");
        assert_eq!(msg.message_type(), "text");

        let msg = WebSocketMessage::binary(vec![1, 2, 3]);
        assert_eq!(msg.message_type(), "binary");

        let msg = WebSocketMessage::ping(vec![]);
        assert!(msg.is_control());
    }

    #[test]
    fn test_message_envelope() {
        let payload = WebSocketMessage::text("test");
        let envelope = MessageEnvelope::new(payload).from("conn1").to("conn2");

        assert_eq!(envelope.from, Some("conn1".to_string()));
        assert_eq!(envelope.to, Some("conn2".to_string()));
        assert!(!envelope.is_broadcast());
    }

    #[test]
    fn test_message_codec() {
        let msg = WebSocketMessage::text("hello");
        let encoded = MessageCodec::encode(&msg).unwrap();
        let decoded = MessageCodec::decode(encoded).unwrap();

        match decoded {
            WebSocketMessage::Text(text) => assert_eq!(text, "hello"),
            _ => panic!("Expected text message"),
        }
    }

    #[test]
    fn test_message_router() {
        let mut router = MessageRouter::new();

        router.register("text", |msg| match msg {
            WebSocketMessage::Text(text) => {
                Ok(Some(WebSocketMessage::text(format!("Echo: {}", text))))
            }
            _ => Ok(None),
        });

        let msg = WebSocketMessage::text("hello");
        let responses = router.route(&msg).unwrap();

        assert_eq!(responses.len(), 1);
        match &responses[0] {
            WebSocketMessage::Text(text) => assert_eq!(text, "Echo: hello"),
            _ => panic!("Expected text response"),
        }
    }
}
