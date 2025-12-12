// RPC framework for cluster communication
//
// This module provides a high-level RPC abstraction for making remote procedure calls
// between cluster nodes with type-safe request/response patterns.

use crate::error::{DbError, Result};
use crate::networking::routing::router::MessageRouter;
use crate::networking::routing::serialization::ClusterMessage;
use crate::networking::types::{MessagePriority, NodeId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Request trait for RPC calls
#[async_trait]
pub trait Request: Send + Sync + Serialize + for<'de> Deserialize<'de> {
    /// Associated response type
    type Response: Send + Sync + Serialize + for<'de> Deserialize<'de>;

    /// Convert to cluster message
    fn to_cluster_message(self) -> ClusterMessage;

    /// Extract response from cluster message
    fn from_cluster_message(message: ClusterMessage) -> Result<Self::Response>;

    /// Get the request name for logging
    fn request_name(&self) -> &'static str;
}

/// RPC client for making remote procedure calls
pub struct RpcClient {
    /// Message router
    router: Arc<MessageRouter>,
    /// Default timeout for RPC calls
    default_timeout: Duration,
    /// Default priority for RPC calls
    default_priority: MessagePriority,
}

impl RpcClient {
    /// Create a new RPC client
    pub fn new(router: Arc<MessageRouter>) -> Self {
        Self {
            router,
            default_timeout: Duration::from_secs(30),
            default_priority: MessagePriority::Normal,
        }
    }

    /// Create with custom defaults
    pub fn with_defaults(
        router: Arc<MessageRouter>,
        default_timeout: Duration,
        default_priority: MessagePriority,
    ) -> Self {
        Self {
            router,
            default_timeout,
            default_priority,
        }
    }

    /// Make an RPC call to a specific node
    pub async fn call<R: Request>(
        &self,
        node: NodeId,
        request: R,
    ) -> Result<R::Response> {
        self.call_with_options(node, request, None, None).await
    }

    /// Make an RPC call with custom timeout
    pub async fn call_with_timeout<R: Request>(
        &self,
        node: NodeId,
        request: R,
        timeout: Duration,
    ) -> Result<R::Response> {
        self.call_with_options(node, request, Some(timeout), None).await
    }

    /// Make an RPC call with custom priority
    pub async fn call_with_priority<R: Request>(
        &self,
        node: NodeId,
        request: R,
        priority: MessagePriority,
    ) -> Result<R::Response> {
        self.call_with_options(node, request, None, Some(priority)).await
    }

    /// Make an RPC call with full options
    pub async fn call_with_options<R: Request>(
        &self,
        node: NodeId,
        request: R,
        timeout: Option<Duration>,
        priority: Option<MessagePriority>,
    ) -> Result<R::Response> {
        let timeout = timeout.unwrap_or(self.default_timeout);
        let priority = priority.unwrap_or(self.default_priority);

        // Convert request to cluster message
        let message = request.to_cluster_message();

        // Send request and wait for response
        let response_message = self
            .router
            .send_request(node, message, priority, Some(timeout))
            .await?;

        // Convert response message to typed response
        R::from_cluster_message(response_message)
    }

    /// Make an RPC call with automatic retry
    pub async fn call_with_retry<R: Request + Clone>(
        &self,
        node: NodeId,
        request: R,
        max_retries: u32,
    ) -> Result<R::Response> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts <= max_retries {
            // Clone the request for retry
            let message = request.clone().to_cluster_message();

            match self
                .router
                .send_request(
                    node.clone(),
                    message,
                    self.default_priority,
                    Some(self.default_timeout),
                )
                .await
            {
                Ok(response_message) => {
                    return R::from_cluster_message(response_message);
                }
                Err(e) => {
                    last_error = Some(e);
                    attempts += 1;

                    if attempts <= max_retries {
                        // Exponential backoff
                        let backoff = Duration::from_millis(100 * 2u64.pow(attempts));
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| DbError::Network("RPC call failed".to_string())))
    }
}

/// RPC server for handling incoming RPC calls
pub struct RpcServer {
    /// Message router (for registration)
    router: Arc<MessageRouter>,
}

impl RpcServer {
    /// Create a new RPC server
    pub fn new(router: Arc<MessageRouter>) -> Self {
        Self { router }
    }

    /// Register an RPC handler
    pub fn register_handler<H: RpcHandler + 'static>(&self, handler: H) {
        self.router.register_handler(handler);
    }

    /// Get the underlying router
    pub fn router(&self) -> &Arc<MessageRouter> {
        &self.router
    }
}

/// RPC handler trait
#[async_trait]
pub trait RpcHandler: crate::networking::routing::router::MessageHandler {
    /// Handle type information
    fn handler_name(&self) -> &'static str;
}

// ============================================================================
// Example RPC Request/Response Types
// ============================================================================

/// Ping request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingRequest {
    pub timestamp: u64,
}

/// Ping response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResponse {
    pub timestamp: u64,
    pub node_id: NodeId,
}

#[async_trait]
impl Request for PingRequest {
    type Response = PingResponse;

    fn to_cluster_message(self) -> ClusterMessage {
        ClusterMessage::Heartbeat(crate::networking::routing::serialization::HeartbeatMessage {
            node_id: NodeId::new("ping"),
            timestamp: self.timestamp,
            sequence: 0,
        })
    }

    fn from_cluster_message(message: ClusterMessage) -> Result<Self::Response> {
        match message {
            ClusterMessage::HeartbeatAck(ack) => Ok(PingResponse {
                timestamp: ack.timestamp,
                node_id: ack.node_id,
            }),
            _ => Err(DbError::Serialization(
                "Invalid response type for PingRequest".to_string(),
            )),
        }
    }

    fn request_name(&self) -> &'static str {
        "Ping"
    }
}

/// Data read request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataReadRequest {
    pub shard_id: u32,
    pub key: Vec<u8>,
}

/// Data read response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataReadResponse {
    pub value: Option<Vec<u8>>,
    pub version: u64,
}

#[async_trait]
impl Request for DataReadRequest {
    type Response = DataReadResponse;

    fn to_cluster_message(self) -> ClusterMessage {
        ClusterMessage::DataRequest(crate::networking::routing::serialization::DataRequest {
            request_id: crate::networking::routing::serialization::RequestId::new(),
            shard_id: self.shard_id,
            key: self.key,
            read_consistency: crate::networking::routing::serialization::ReadConsistency::Quorum,
        })
    }

    fn from_cluster_message(message: ClusterMessage) -> Result<Self::Response> {
        match message {
            ClusterMessage::DataResponse(response) => Ok(DataReadResponse {
                value: response.value,
                version: response.version,
            }),
            _ => Err(DbError::Serialization(
                "Invalid response type for DataReadRequest".to_string(),
            )),
        }
    }

    fn request_name(&self) -> &'static str {
        "DataRead"
    }
}

/// Data write request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataWriteRequest {
    pub shard_id: u32,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

/// Data write response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataWriteResponse {
    pub success: bool,
    pub version: u64,
}

#[async_trait]
impl Request for DataWriteRequest {
    type Response = DataWriteResponse;

    fn to_cluster_message(self) -> ClusterMessage {
        ClusterMessage::DataWrite(crate::networking::routing::serialization::DataWrite {
            request_id: crate::networking::routing::serialization::RequestId::new(),
            shard_id: self.shard_id,
            key: self.key,
            value: self.value,
            write_consistency: crate::networking::routing::serialization::WriteConsistency::Quorum,
        })
    }

    fn from_cluster_message(message: ClusterMessage) -> Result<Self::Response> {
        match message {
            ClusterMessage::DataWriteAck(ack) => Ok(DataWriteResponse {
                success: ack.success,
                version: ack.version,
            }),
            _ => Err(DbError::Serialization(
                "Invalid response type for DataWriteRequest".to_string(),
            )),
        }
    }

    fn request_name(&self) -> &'static str {
        "DataWrite"
    }
}

/// Query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRpcRequest {
    pub query: String,
    pub params: Vec<Vec<u8>>,
}

/// Query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRpcResponse {
    pub rows: Vec<Vec<u8>>,
    pub row_count: usize,
}

#[async_trait]
impl Request for QueryRpcRequest {
    type Response = QueryRpcResponse;

    fn to_cluster_message(self) -> ClusterMessage {
        ClusterMessage::QueryRequest(crate::networking::routing::serialization::QueryRequest {
            request_id: crate::networking::routing::serialization::RequestId::new(),
            query: self.query,
            params: self.params,
            timeout_ms: 30000,
        })
    }

    fn from_cluster_message(message: ClusterMessage) -> Result<Self::Response> {
        match message {
            ClusterMessage::QueryResponse(response) => Ok(QueryRpcResponse {
                rows: response.rows,
                row_count: response.row_count,
            }),
            _ => Err(DbError::Serialization(
                "Invalid response type for QueryRpcRequest".to_string(),
            )),
        }
    }

    fn request_name(&self) -> &'static str {
        "Query"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::routing::table::RoutingTable;

    #[tokio::test]
    async fn test_rpc_client_creation() {
        let table = RoutingTable::new();
        let router = Arc::new(MessageRouter::new(table));
        let client = RpcClient::new(router);

        assert_eq!(client.default_timeout, Duration::from_secs(30));
        assert_eq!(client.default_priority, MessagePriority::Normal);
    }

    #[test]
    fn test_ping_request_conversion() {
        let request = PingRequest { timestamp: 12345 };
        let message = request.to_cluster_message();

        match message {
            ClusterMessage::Heartbeat(hb) => {
                assert_eq!(hb.timestamp, 12345);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_data_read_request() {
        let request = DataReadRequest {
            shard_id: 1,
            key: vec![1, 2, 3],
        };
        let message = request.to_cluster_message();

        match message {
            ClusterMessage::DataRequest(req) => {
                assert_eq!(req.shard_id, 1);
                assert_eq!(req.key, vec![1, 2, 3]);
            }
            _ => panic!("Wrong message type"),
        }
    }
}
