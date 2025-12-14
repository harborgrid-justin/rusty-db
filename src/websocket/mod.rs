// # WebSocket Module
//
// Real-time WebSocket support for RustyDB with comprehensive connection management,
// protocol support, message handling, security, authentication, and monitoring.
//
// ## Core Modules
//
// - **message**: Message types and serialization/deserialization
// - **protocol**: Protocol handlers (JSON-RPC, RustyDB custom, GraphQL)
// - **connection**: Connection management, pooling, and lifecycle
//
// ## Enterprise Features
//
// - **auth**: Authentication and authorization
// - **security**: Security features and encryption
// - **metrics**: Monitoring and performance metrics

// Core modules
pub mod connection;
pub mod message;
pub mod protocol;

// Enterprise modules
pub mod auth;
pub mod metrics;
pub mod security;

// Re-export core types
pub use message::{MessageCodec, MessageEnvelope, MessageHandler, MessageRouter, WebSocketMessage};

pub use protocol::{
    GraphQLHandler, JsonRpcError, JsonRpcHandler, JsonRpcRequest, JsonRpcResponse, Protocol,
    ProtocolHandler, ProtocolNegotiator, RawHandler, RustyDbHandler, RustyDbMessage,
    RustyDbMessageType,
};

pub use connection::{
    ConnectionMetadata, ConnectionPool, ConnectionState, ConnectionStats, PoolStats,
    WebSocketConnection,
};

// Re-export metrics types
pub use metrics::*;

// Re-export authentication types
pub use auth::{
    ApiKeyAuthenticator, AuthMethod, AuthResult, MultiAuthenticator, PermissionChecker,
    SessionAuthenticator, TokenAuthenticator, WebSocketAuthenticator, WebSocketCredentials,
};

// Re-export security types
pub use security::{
    ConnectionInfo, EncryptionAlgorithm, TlsVersion, WebSocketSecurityConfig,
    WebSocketSecurityManager, WebSocketSecurityStats,
};

// Module constants
pub const DEFAULT_MAX_CONNECTIONS: usize = 1000;
pub const DEFAULT_CONNECTION_TIMEOUT_SECS: u64 = 300;
pub const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 30;

// WebSocket close codes
pub mod close_codes {
    pub const NORMAL: u16 = 1000;
    pub const GOING_AWAY: u16 = 1001;
    pub const PROTOCOL_ERROR: u16 = 1002;
    pub const UNSUPPORTED_DATA: u16 = 1003;
    pub const INVALID_FRAME: u16 = 1007;
    pub const POLICY_VIOLATION: u16 = 1008;
    pub const MESSAGE_TOO_BIG: u16 = 1009;
    pub const INTERNAL_ERROR: u16 = 1011;
}

// Helper functions
pub fn create_default_pool() -> ConnectionPool {
    ConnectionPool::new(DEFAULT_MAX_CONNECTIONS)
}

pub fn create_pool(
    max_connections: usize,
    connection_timeout_secs: u64,
    heartbeat_interval_secs: u64,
) -> ConnectionPool {
    use std::time::Duration;

    ConnectionPool::with_config(
        max_connections,
        Duration::from_secs(connection_timeout_secs),
        Duration::from_secs(heartbeat_interval_secs),
    )
}
