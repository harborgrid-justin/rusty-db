// # WebSocket Protocol Support
//
// This module provides protocol implementations for WebSocket communication in RustyDB.
//
// ## Supported Protocols
//
// - **JSON-RPC 2.0**: Standard JSON-RPC over WebSocket
// - **RustyDB Protocol**: Custom high-performance binary protocol
// - **GraphQL over WebSocket**: GraphQL subscriptions
//
// ## Protocol Negotiation
//
// Clients can negotiate protocol version during WebSocket handshake using
// the `Sec-WebSocket-Protocol` header.
//
// ## Usage
//
// ```rust
// use rusty_db::websocket::protocol::{Protocol, ProtocolHandler};
//
// let protocol = Protocol::JsonRpc;
// let handler = protocol.create_handler();
// ```

use super::message::WebSocketMessage;
use crate::error::{DbError, Result};

use serde::{Deserialize, Serialize};

// ============================================================================
// Protocol Types
// ============================================================================

/// Supported WebSocket protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    /// JSON-RPC 2.0 protocol
    JsonRpc,

    /// Custom RustyDB binary protocol
    RustyDb,

    /// GraphQL over WebSocket
    GraphQL,

    /// Raw text/binary (no specific protocol)
    Raw,
}

impl Protocol {
    /// Parse protocol from string (e.g., from WebSocket subprotocol header)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "jsonrpc" | "json-rpc" | "jsonrpc-2.0" => Some(Protocol::JsonRpc),
            "rustydb" | "rusty-db" => Some(Protocol::RustyDb),
            "graphql" | "graphql-ws" | "graphql-transport-ws" => Some(Protocol::GraphQL),
            "raw" => Some(Protocol::Raw),
            _ => None,
        }
    }

    /// Get protocol name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::JsonRpc => "jsonrpc-2.0",
            Protocol::RustyDb => "rustydb",
            Protocol::GraphQL => "graphql-transport-ws",
            Protocol::Raw => "raw",
        }
    }

    /// Create a protocol handler for this protocol
    pub fn create_handler(&self) -> Box<dyn ProtocolHandler + Send + Sync> {
        match self {
            Protocol::JsonRpc => Box::new(JsonRpcHandler::new()),
            Protocol::RustyDb => Box::new(RustyDbHandler::new()),
            Protocol::GraphQL => Box::new(GraphQLHandler::new()),
            Protocol::Raw => Box::new(RawHandler::new()),
        }
    }
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::JsonRpc
    }
}

// ============================================================================
// Protocol Handler Trait
// ============================================================================

/// Protocol handler trait for processing messages
pub trait ProtocolHandler {
    /// Handle an incoming message and generate a response
    fn handle_message(&self, msg: &WebSocketMessage) -> Result<Option<WebSocketMessage>>;

    /// Get protocol version
    fn protocol_version(&self) -> &str;

    /// Validate a message according to protocol rules
    fn validate_message(&self, msg: &WebSocketMessage) -> Result<()>;
}

// ============================================================================
// JSON-RPC 2.0 Protocol
// ============================================================================

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (must be "2.0")
    pub jsonrpc: String,

    /// Request method
    pub method: String,

    /// Request parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,

    /// Request ID (optional for notifications)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (must be "2.0")
    pub jsonrpc: String,

    /// Result (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    /// Error (present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,

    /// Request ID
    pub id: serde_json::Value,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,

    /// Error message
    pub message: String,

    /// Additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// Standard error codes
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    /// Create a new error
    pub fn new(code: i32, message: String) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }

    /// Create a parse error
    pub fn parse_error() -> Self {
        Self::new(Self::PARSE_ERROR, "Parse error".to_string())
    }

    /// Create an invalid request error
    pub fn invalid_request() -> Self {
        Self::new(Self::INVALID_REQUEST, "Invalid request".to_string())
    }

    /// Create a method not found error
    pub fn method_not_found() -> Self {
        Self::new(Self::METHOD_NOT_FOUND, "Method not found".to_string())
    }

    /// Create an internal error
    pub fn internal_error(msg: String) -> Self {
        Self::new(Self::INTERNAL_ERROR, msg)
    }
}

/// JSON-RPC protocol handler
pub struct JsonRpcHandler;

impl JsonRpcHandler {
    /// Create a new JSON-RPC handler
    pub fn new() -> Self {
        Self
    }

    /// Process a JSON-RPC request
    fn process_request(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Handle different methods
        match req.method.as_str() {
            "query" => self.handle_query(req),
            "execute" => self.handle_execute(req),
            "begin_transaction" => self.handle_begin_transaction(req),
            "commit" => self.handle_commit(req),
            "rollback" => self.handle_rollback(req),
            _ => Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError::method_not_found()),
                id: req.id.unwrap_or(serde_json::Value::Null),
            }),
        }
    }

    fn handle_query(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // This is a placeholder - actual implementation would execute the query
        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "status": "ok",
                "method": "query",
                "message": "Query method called"
            })),
            error: None,
            id: req.id.unwrap_or(serde_json::Value::Null),
        })
    }

    fn handle_execute(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse> {
        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "status": "ok",
                "method": "execute"
            })),
            error: None,
            id: req.id.unwrap_or(serde_json::Value::Null),
        })
    }

    fn handle_begin_transaction(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse> {
        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "transaction_id": uuid::Uuid::new_v4().to_string()
            })),
            error: None,
            id: req.id.unwrap_or(serde_json::Value::Null),
        })
    }

    fn handle_commit(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse> {
        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({"status": "committed"})),
            error: None,
            id: req.id.unwrap_or(serde_json::Value::Null),
        })
    }

    fn handle_rollback(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse> {
        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({"status": "rolled_back"})),
            error: None,
            id: req.id.unwrap_or(serde_json::Value::Null),
        })
    }
}

impl Default for JsonRpcHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolHandler for JsonRpcHandler {
    fn handle_message(&self, msg: &WebSocketMessage) -> Result<Option<WebSocketMessage>> {
        match msg {
            WebSocketMessage::Text(text) => {
                // Parse JSON-RPC request
                let request: JsonRpcRequest =
                    serde_json::from_str(text).map_err(|e| DbError::ParseError(e.to_string()))?;

                // Process request
                let response = self.process_request(request)?;

                // Serialize response
                let response_json = serde_json::to_string(&response)
                    .map_err(|e| DbError::Serialization(e.to_string()))?;

                Ok(Some(WebSocketMessage::text(response_json)))
            }
            _ => Err(DbError::InvalidInput(
                "JSON-RPC requires text messages".to_string(),
            )),
        }
    }

    fn protocol_version(&self) -> &str {
        "2.0"
    }

    fn validate_message(&self, msg: &WebSocketMessage) -> Result<()> {
        match msg {
            WebSocketMessage::Text(text) => {
                // Validate JSON-RPC structure
                let _: JsonRpcRequest = serde_json::from_str(text)
                    .map_err(|e| DbError::Validation(format!("Invalid JSON-RPC: {}", e)))?;
                Ok(())
            }
            _ => Err(DbError::Validation(
                "JSON-RPC requires text messages".to_string(),
            )),
        }
    }
}

// ============================================================================
// RustyDB Custom Protocol
// ============================================================================

/// RustyDB custom protocol message
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct RustyDbMessage {
    /// Message version
    pub version: u8,

    /// Message type
    pub msg_type: RustyDbMessageType,

    /// Message ID for correlation
    pub id: String,

    /// Message payload
    pub payload: Vec<u8>,
}

/// RustyDB message types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum RustyDbMessageType {
    Query = 1,
    QueryResult = 2,
    Execute = 3,
    ExecuteResult = 4,
    BeginTxn = 5,
    Commit = 6,
    Rollback = 7,
    Subscribe = 8,
    Unsubscribe = 9,
    Event = 10,
    Error = 255,
}

/// RustyDB protocol handler
pub struct RustyDbHandler;

impl RustyDbHandler {
    /// Create a new RustyDB handler
    pub fn new() -> Self {
        Self
    }
}

impl Default for RustyDbHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolHandler for RustyDbHandler {
    fn handle_message(&self, msg: &WebSocketMessage) -> Result<Option<WebSocketMessage>> {
        match msg {
            WebSocketMessage::Binary(data) => {
                // Decode binary message
                let decoded: RustyDbMessage =
                    bincode::decode_from_slice(data, bincode::config::standard())
                        .map_err(|e| DbError::Serialization(e.to_string()))?
                        .0;

                // Process based on message type
                // This is a placeholder - actual implementation would route to handlers
                let response = RustyDbMessage {
                    version: decoded.version,
                    msg_type: RustyDbMessageType::QueryResult,
                    id: decoded.id,
                    payload: vec![],
                };

                // Encode response
                let encoded = bincode::encode_to_vec(&response, bincode::config::standard())
                    .map_err(|e| DbError::Serialization(e.to_string()))?;

                Ok(Some(WebSocketMessage::binary(encoded)))
            }
            _ => Err(DbError::InvalidInput(
                "RustyDB protocol requires binary messages".to_string(),
            )),
        }
    }

    fn protocol_version(&self) -> &str {
        "1.0"
    }

    fn validate_message(&self, msg: &WebSocketMessage) -> Result<()> {
        match msg {
            WebSocketMessage::Binary(data) => {
                // Validate message structure
                let _: RustyDbMessage =
                    bincode::decode_from_slice(data, bincode::config::standard())
                        .map_err(|e| {
                            DbError::Validation(format!("Invalid RustyDB message: {}", e))
                        })?
                        .0;
                Ok(())
            }
            _ => Err(DbError::Validation(
                "RustyDB protocol requires binary messages".to_string(),
            )),
        }
    }
}

// ============================================================================
// GraphQL Protocol
// ============================================================================

/// GraphQL over WebSocket handler
pub struct GraphQLHandler;

impl GraphQLHandler {
    /// Create a new GraphQL handler
    pub fn new() -> Self {
        Self
    }
}

impl Default for GraphQLHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolHandler for GraphQLHandler {
    fn handle_message(&self, _msg: &WebSocketMessage) -> Result<Option<WebSocketMessage>> {
        // GraphQL subscriptions will be implemented by Agent 6
        // This is a placeholder
        Ok(None)
    }

    fn protocol_version(&self) -> &str {
        "graphql-transport-ws"
    }

    fn validate_message(&self, _msg: &WebSocketMessage) -> Result<()> {
        // Validation will be implemented with full GraphQL support
        Ok(())
    }
}

// ============================================================================
// Raw Protocol (No Processing)
// ============================================================================

/// Raw protocol handler (pass-through)
pub struct RawHandler;

impl RawHandler {
    /// Create a new raw handler
    pub fn new() -> Self {
        Self
    }
}

impl Default for RawHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolHandler for RawHandler {
    fn handle_message(&self, msg: &WebSocketMessage) -> Result<Option<WebSocketMessage>> {
        // Echo the message back
        Ok(Some(msg.clone()))
    }

    fn protocol_version(&self) -> &str {
        "raw"
    }

    fn validate_message(&self, _msg: &WebSocketMessage) -> Result<()> {
        // All messages are valid in raw mode
        Ok(())
    }
}

// ============================================================================
// Protocol Negotiation
// ============================================================================

/// Protocol negotiation helper
pub struct ProtocolNegotiator {
    /// Supported protocols in order of preference
    supported: Vec<Protocol>,
}

impl ProtocolNegotiator {
    /// Create a new negotiator with default protocols
    pub fn new() -> Self {
        Self {
            supported: vec![
                Protocol::RustyDb,
                Protocol::JsonRpc,
                Protocol::GraphQL,
                Protocol::Raw,
            ],
        }
    }

    /// Create a negotiator with custom protocol list
    pub fn with_protocols(protocols: Vec<Protocol>) -> Self {
        Self {
            supported: protocols,
        }
    }

    /// Negotiate protocol from client request
    pub fn negotiate(&self, requested: &[&str]) -> Option<Protocol> {
        for req in requested {
            if let Some(protocol) = Protocol::from_str(req) {
                if self.supported.contains(&protocol) {
                    return Some(protocol);
                }
            }
        }
        None
    }

    /// Get list of supported protocol names
    pub fn supported_protocols(&self) -> Vec<&'static str> {
        self.supported.iter().map(|p| p.as_str()).collect()
    }
}

impl Default for ProtocolNegotiator {
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
    fn test_protocol_from_str() {
        assert_eq!(Protocol::from_str("jsonrpc"), Some(Protocol::JsonRpc));
        assert_eq!(Protocol::from_str("rustydb"), Some(Protocol::RustyDb));
        assert_eq!(
            Protocol::from_str("graphql-transport-ws"),
            Some(Protocol::GraphQL)
        );
        assert_eq!(Protocol::from_str("invalid"), None);
    }

    #[test]
    fn test_protocol_as_str() {
        assert_eq!(Protocol::JsonRpc.as_str(), "jsonrpc-2.0");
        assert_eq!(Protocol::RustyDb.as_str(), "rustydb");
    }

    #[test]
    fn test_json_rpc_request() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "query".to_string(),
            params: Some(serde_json::json!({"sql": "SELECT * FROM users"})),
            id: Some(serde_json::json!(1)),
        };

        let json = serde_json::to_string(&req).unwrap();
        let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.method, "query");
        assert_eq!(parsed.jsonrpc, "2.0");
    }

    #[test]
    fn test_protocol_negotiation() {
        let negotiator = ProtocolNegotiator::new();

        let result = negotiator.negotiate(&["graphql", "jsonrpc"]);
        assert!(result.is_some());

        let result = negotiator.negotiate(&["unknown"]);
        assert!(result.is_none());
    }

    #[test]
    fn test_json_rpc_handler() {
        let handler = JsonRpcHandler::new();
        let msg = WebSocketMessage::text(r#"{"jsonrpc":"2.0","method":"query","id":1}"#);

        let result = handler.handle_message(&msg);
        assert!(result.is_ok());
    }
}
