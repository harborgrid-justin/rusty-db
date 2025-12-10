//! Message definitions for cluster communication
//!
//! This module defines all message types that can be sent between nodes in the cluster,
//! including membership, data operations, replication, and coordination messages.

use crate::networking::types::{NodeAddress, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a request
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub String);

impl RequestId {
    /// Generate a new unique request ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Create from a string
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Transaction ID for distributed transactions
pub type TransactionId = u64;

/// Shard ID for data partitioning
pub type ShardId = u32;

/// Sequence number for ordering
pub type SequenceNumber = u64;

/// All possible cluster message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterMessage {
    // ============================================================================
    // Membership Messages
    // ============================================================================
    /// Request to join the cluster
    Join(JoinRequest),

    /// Response to join request
    JoinResponse(JoinResponse),

    /// Notification that a node is leaving
    Leave(LeaveRequest),

    /// Heartbeat message for health checking
    Heartbeat(HeartbeatMessage),

    /// Response to heartbeat
    HeartbeatAck(HeartbeatAck),

    // ============================================================================
    // Data Messages
    // ============================================================================
    /// Request for data
    DataRequest(DataRequest),

    /// Response with data
    DataResponse(DataResponse),

    /// Write data to a node
    DataWrite(DataWrite),

    /// Acknowledge write
    DataWriteAck(DataWriteAck),

    // ============================================================================
    // Replication Messages
    // ============================================================================
    /// Replicate write to replica nodes
    ReplicateWrite(ReplicateWriteRequest),

    /// Acknowledge replication
    ReplicateAck(ReplicateAckResponse),

    /// Sync request for out-of-sync replicas
    SyncRequest(SyncRequest),

    /// Sync response with data
    SyncResponse(SyncResponse),

    // ============================================================================
    // Transaction Coordination (2PC)
    // ============================================================================
    /// Prepare phase of 2PC
    PrepareTransaction(PrepareRequest),

    /// Prepare response
    PrepareResponse(PrepareResponse),

    /// Commit transaction
    CommitTransaction(CommitRequest),

    /// Commit acknowledgment
    CommitAck(CommitAck),

    /// Abort transaction
    AbortTransaction(AbortRequest),

    /// Abort acknowledgment
    AbortAck(AbortAck),

    // ============================================================================
    // Query Messages
    // ============================================================================
    /// Execute a query
    QueryRequest(QueryRequest),

    /// Query results
    QueryResponse(QueryResponse),

    // ============================================================================
    // Metadata Messages
    // ============================================================================
    /// Request metadata
    MetadataRequest(MetadataRequest),

    /// Metadata response
    MetadataResponse(MetadataResponse),

    // ============================================================================
    // Error Messages
    // ============================================================================
    /// Error response
    Error(ErrorResponse),
}

impl ClusterMessage {
    /// Get the message type as a string
    pub fn message_type(&self) -> &'static str {
        match self {
            ClusterMessage::Join(_) => "Join",
            ClusterMessage::JoinResponse(_) => "JoinResponse",
            ClusterMessage::Leave(_) => "Leave",
            ClusterMessage::Heartbeat(_) => "Heartbeat",
            ClusterMessage::HeartbeatAck(_) => "HeartbeatAck",
            ClusterMessage::DataRequest(_) => "DataRequest",
            ClusterMessage::DataResponse(_) => "DataResponse",
            ClusterMessage::DataWrite(_) => "DataWrite",
            ClusterMessage::DataWriteAck(_) => "DataWriteAck",
            ClusterMessage::ReplicateWrite(_) => "ReplicateWrite",
            ClusterMessage::ReplicateAck(_) => "ReplicateAck",
            ClusterMessage::SyncRequest(_) => "SyncRequest",
            ClusterMessage::SyncResponse(_) => "SyncResponse",
            ClusterMessage::PrepareTransaction(_) => "PrepareTransaction",
            ClusterMessage::PrepareResponse(_) => "PrepareResponse",
            ClusterMessage::CommitTransaction(_) => "CommitTransaction",
            ClusterMessage::CommitAck(_) => "CommitAck",
            ClusterMessage::AbortTransaction(_) => "AbortTransaction",
            ClusterMessage::AbortAck(_) => "AbortAck",
            ClusterMessage::QueryRequest(_) => "QueryRequest",
            ClusterMessage::QueryResponse(_) => "QueryResponse",
            ClusterMessage::MetadataRequest(_) => "MetadataRequest",
            ClusterMessage::MetadataResponse(_) => "MetadataResponse",
            ClusterMessage::Error(_) => "Error",
        }
    }
}

// ============================================================================
// Membership Message Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    pub request_id: RequestId,
    pub node_id: NodeId,
    pub address: NodeAddress,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinResponse {
    pub request_id: RequestId,
    pub accepted: bool,
    pub reason: Option<String>,
    pub cluster_view: Vec<NodeId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveRequest {
    pub request_id: RequestId,
    pub node_id: NodeId,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    pub node_id: NodeId,
    pub timestamp: u64,
    pub sequence: SequenceNumber,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatAck {
    pub node_id: NodeId,
    pub timestamp: u64,
    pub sequence: SequenceNumber,
}

// ============================================================================
// Data Message Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRequest {
    pub request_id: RequestId,
    pub shard_id: ShardId,
    pub key: Vec<u8>,
    pub read_consistency: ReadConsistency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadConsistency {
    /// Read from any replica
    Eventual,
    /// Read from majority
    Quorum,
    /// Read from primary only
    Strong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataResponse {
    pub request_id: RequestId,
    pub shard_id: ShardId,
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataWrite {
    pub request_id: RequestId,
    pub shard_id: ShardId,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub write_consistency: WriteConsistency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WriteConsistency {
    /// Write to primary only
    One,
    /// Write to majority
    Quorum,
    /// Write to all replicas
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataWriteAck {
    pub request_id: RequestId,
    pub shard_id: ShardId,
    pub key: Vec<u8>,
    pub version: u64,
    pub success: bool,
}

// ============================================================================
// Replication Message Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicateWriteRequest {
    pub request_id: RequestId,
    pub shard_id: ShardId,
    pub operations: Vec<ReplicationOperation>,
    pub sequence_number: SequenceNumber,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationOperation {
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>, // None means delete
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicateAckResponse {
    pub request_id: RequestId,
    pub shard_id: ShardId,
    pub sequence_number: SequenceNumber,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub request_id: RequestId,
    pub shard_id: ShardId,
    pub from_sequence: SequenceNumber,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub request_id: RequestId,
    pub shard_id: ShardId,
    pub operations: Vec<ReplicationOperation>,
    pub current_sequence: SequenceNumber,
}

// ============================================================================
// Transaction Message Types (2PC)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareRequest {
    pub request_id: RequestId,
    pub transaction_id: TransactionId,
    pub operations: Vec<TransactionOperation>,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOperation {
    pub shard_id: ShardId,
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>, // None means delete
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareResponse {
    pub request_id: RequestId,
    pub transaction_id: TransactionId,
    pub vote: PrepareVote,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrepareVote {
    /// Ready to commit
    Commit,
    /// Must abort
    Abort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRequest {
    pub request_id: RequestId,
    pub transaction_id: TransactionId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitAck {
    pub request_id: RequestId,
    pub transaction_id: TransactionId,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbortRequest {
    pub request_id: RequestId,
    pub transaction_id: TransactionId,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbortAck {
    pub request_id: RequestId,
    pub transaction_id: TransactionId,
}

// ============================================================================
// Query Message Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub request_id: RequestId,
    pub query: String,
    pub params: Vec<Vec<u8>>,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub request_id: RequestId,
    pub rows: Vec<Vec<u8>>,
    pub row_count: usize,
    pub execution_time_ms: u64,
}

// ============================================================================
// Metadata Message Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataRequest {
    pub request_id: RequestId,
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataResponse {
    pub request_id: RequestId,
    pub metadata: HashMap<String, String>,
}

// ============================================================================
// Error Message Type
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub request_id: RequestId,
    pub error_code: String,
    pub message: String,
    pub details: Option<String>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(request_id: RequestId, error_code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            request_id,
            error_code: error_code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Add error details
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}
