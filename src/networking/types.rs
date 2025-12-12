// Common types for the RustyDB networking layer
//
// This module defines all shared types used across networking components including
// node identification, cluster messages, network configuration, and connection info.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;
use std::time::SystemTime;

// ============================================================================
// Node Identification and Addressing
// ============================================================================

/// Unique identifier for a node in the cluster
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    /// Create a new NodeId
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        NodeId(s)
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        NodeId(s.to_string())
    }
}

/// Network address of a node (host:port)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeAddress {
    pub host: String,
    pub port: u16,
}

impl NodeAddress {
    /// Create a new NodeAddress
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    /// Convert to a socket address string
    pub fn to_socket_addr_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl fmt::Display for NodeAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

impl From<SocketAddr> for NodeAddress {
    fn from(addr: SocketAddr) -> Self {
        Self {
            host: addr.ip().to_string(),
            port: addr.port(),
        }
    }
}

/// Current state of a node in the cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    /// Node is joining the cluster
    Joining,
    /// Node is active and healthy
    Active,
    /// Node is suspected to be unhealthy
    Suspected,
    /// Node is confirmed unhealthy
    Failed,
    /// Node is leaving the cluster gracefully
    Leaving,
    /// Node has left the cluster
    Left,
}

impl fmt::Display for NodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeState::Joining => write!(f, "Joining"),
            NodeState::Active => write!(f, "Active"),
            NodeState::Suspected => write!(f, "Suspected"),
            NodeState::Failed => write!(f, "Failed"),
            NodeState::Leaving => write!(f, "Leaving"),
            NodeState::Left => write!(f, "Left"),
        }
    }
}

/// Information about a node in the cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Unique node identifier
    pub id: NodeId,
    /// Network address
    pub address: NodeAddress,
    /// Current state
    pub state: NodeState,
    /// Node metadata (version, capabilities, etc.)
    pub metadata: HashMap<String, String>,
    /// When the node joined the cluster
    pub joined_at: SystemTime,
    /// Last heartbeat timestamp
    pub last_heartbeat: SystemTime,
}

impl NodeInfo {
    /// Create a new NodeInfo
    pub fn new(id: NodeId, address: NodeAddress) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            address,
            state: NodeState::Joining,
            metadata: HashMap::new(),
            joined_at: now,
            last_heartbeat: now,
        }
    }

    /// Update the last heartbeat timestamp
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = SystemTime::now();
    }

    /// Check if the node is healthy (Active state)
    pub fn is_healthy(&self) -> bool {
        matches!(self.state, NodeState::Active)
    }

    /// Get a metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Set a metadata value
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }
}

/// Information about a peer connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Node ID of the peer
    pub node_id: NodeId,
    /// Address of the peer
    pub address: NodeAddress,
    /// When the connection was established
    pub connected_at: SystemTime,
    /// Number of active connections
    pub active_connections: usize,
    /// Total bytes sent to this peer
    pub bytes_sent: u64,
    /// Total bytes received from this peer
    pub bytes_received: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
}

/// Connection state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// Remote node ID
    pub node_id: NodeId,
    /// Remote address
    pub remote_address: NodeAddress,
    /// Local address
    pub local_address: NodeAddress,
    /// Connection established at
    pub established_at: SystemTime,
    /// Is TLS enabled
    pub tls_enabled: bool,
    /// Is compression enabled
    pub compression_enabled: bool,
    /// Number of messages sent
    pub messages_sent: u64,
    /// Number of messages received
    pub messages_received: u64,
}

// ============================================================================
// Cluster Messages
// ============================================================================

/// Priority level for cluster messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MessagePriority {
    /// Low priority (bulk data transfer)
    Low = 0,
    /// Normal priority (regular operations)
    Normal = 1,
    /// High priority (health checks, metadata)
    High = 2,
    /// Critical priority (cluster coordination)
    Critical = 3,
}

/// Envelope for cluster messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// Unique message ID
    pub message_id: String,
    /// Source node ID
    pub from: NodeId,
    /// Destination node ID
    pub to: NodeId,
    /// Message priority
    pub priority: MessagePriority,
    /// Timestamp when message was created
    pub timestamp: SystemTime,
    /// The actual message
    pub message: ClusterMessage,
}

/// All possible cluster message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterMessage {
    /// Heartbeat message for health checking
    Heartbeat {
        node_id: NodeId,
        timestamp: u64,
        sequence: u64,
    },

    /// Response to heartbeat
    HeartbeatAck {
        node_id: NodeId,
        timestamp: u64,
        sequence: u64,
    },

    /// Request to join the cluster
    JoinRequest {
        node_id: NodeId,
        address: NodeAddress,
        metadata: HashMap<String, String>,
    },

    /// Response to join request
    JoinResponse {
        accepted: bool,
        cluster_nodes: Vec<NodeInfo>,
        reason: Option<String>,
    },

    /// Notification that a node is leaving
    LeaveNotification {
        node_id: NodeId,
        reason: String,
    },

    /// Gossip message for membership dissemination
    Gossip {
        members: Vec<NodeInfo>,
        incarnation: u64,
    },

    /// Query request for distributed query execution
    QueryRequest {
        query_id: String,
        query: String,
        params: Vec<String>,
    },

    /// Query response with results
    QueryResponse {
        query_id: String,
        rows: Vec<Vec<u8>>, // Serialized rows
        error: Option<String>,
    },

    /// Replication log entries
    ReplicationLog {
        log_sequence_number: u64,
        entries: Vec<Vec<u8>>, // Serialized log entries
    },

    /// Acknowledgment of replication
    ReplicationAck {
        log_sequence_number: u64,
        node_id: NodeId,
    },

    /// Transaction prepare message (2PC)
    TransactionPrepare {
        transaction_id: u64,
        operations: Vec<Vec<u8>>,
    },

    /// Transaction commit message (2PC)
    TransactionCommit {
        transaction_id: u64,
    },

    /// Transaction abort message (2PC)
    TransactionAbort {
        transaction_id: u64,
        reason: String,
    },

    /// Request for node metadata
    MetadataRequest {
        keys: Vec<String>,
    },

    /// Response with node metadata
    MetadataResponse {
        metadata: HashMap<String, String>,
    },

    /// Generic data transfer
    DataTransfer {
        transfer_id: String,
        chunk_index: u64,
        total_chunks: u64,
        data: Vec<u8>,
    },

    /// Error message
    Error {
        error_code: String,
        message: String,
    },
}

impl ClusterMessage {
    /// Get the message type as a string
    pub fn message_type(&self) -> &'static str {
        match self {
            ClusterMessage::Heartbeat { .. } => "Heartbeat",
            ClusterMessage::HeartbeatAck { .. } => "HeartbeatAck",
            ClusterMessage::JoinRequest { .. } => "JoinRequest",
            ClusterMessage::JoinResponse { .. } => "JoinResponse",
            ClusterMessage::LeaveNotification { .. } => "LeaveNotification",
            ClusterMessage::Gossip { .. } => "Gossip",
            ClusterMessage::QueryRequest { .. } => "QueryRequest",
            ClusterMessage::QueryResponse { .. } => "QueryResponse",
            ClusterMessage::ReplicationLog { .. } => "ReplicationLog",
            ClusterMessage::ReplicationAck { .. } => "ReplicationAck",
            ClusterMessage::TransactionPrepare { .. } => "TransactionPrepare",
            ClusterMessage::TransactionCommit { .. } => "TransactionCommit",
            ClusterMessage::TransactionAbort { .. } => "TransactionAbort",
            ClusterMessage::MetadataRequest { .. } => "MetadataRequest",
            ClusterMessage::MetadataResponse { .. } => "MetadataResponse",
            ClusterMessage::DataTransfer { .. } => "DataTransfer",
            ClusterMessage::Error { .. } => "Error",
        }
    }
}

// ============================================================================
// Network Configuration
// ============================================================================

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to TLS certificate file
    pub cert_path: String,
    /// Path to TLS private key file
    pub key_path: String,
    /// Path to CA certificate file (for client verification)
    pub ca_cert_path: Option<String>,
    /// Require client certificate verification
    pub verify_client: bool,
    /// Minimum TLS protocol version
    pub min_protocol_version: TlsVersion,
}

/// TLS protocol version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TlsVersion {
    /// TLS 1.2
    Tls12,
    /// TLS 1.3 (recommended)
    Tls13,
}

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    /// No compression
    None,
    /// LZ4 compression (fast)
    Lz4,
    /// Snappy compression (fast)
    Snappy,
    /// Zstandard compression (balanced)
    Zstd,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Address to bind to (e.g., "0.0.0.0:7000")
    pub bind_address: String,

    /// Address to advertise to other nodes (e.g., "node1.example.com:7000")
    pub advertise_address: String,

    /// Maximum number of concurrent connections
    pub max_connections: usize,

    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,

    /// Enable message compression
    pub enable_compression: bool,

    /// Compression type
    pub compression_type: CompressionType,

    /// Minimum message size (in bytes) to trigger compression
    pub compression_threshold_bytes: usize,

    /// TLS configuration (if enabled)
    pub tls_config: Option<TlsConfig>,

    /// Health check configuration
    pub health_check_config: HealthCheckConfig,

    /// Load balancing configuration
    pub load_balancing_config: LoadBalancingConfig,

    /// Service discovery configuration
    pub service_discovery_config: ServiceDiscoveryConfig,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:7000".to_string(),
            advertise_address: "localhost:7000".to_string(),
            max_connections: 1000,
            connection_timeout_ms: 5000,
            enable_compression: true,
            compression_type: CompressionType::Lz4,
            compression_threshold_bytes: 1024,
            tls_config: None,
            health_check_config: HealthCheckConfig::default(),
            load_balancing_config: LoadBalancingConfig::default(),
            service_discovery_config: ServiceDiscoveryConfig::default(),
        }
    }
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Interval between health checks in milliseconds
    pub interval_ms: u64,
    /// Timeout for health check responses in milliseconds
    pub timeout_ms: u64,
    /// Number of consecutive failures before marking node as unhealthy
    pub failure_threshold: usize,
    /// Number of consecutive successes before marking node as healthy again
    pub success_threshold: usize,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval_ms: 1000,
            timeout_ms: 500,
            failure_threshold: 3,
            success_threshold: 2,
        }
    }
}

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    /// Round-robin distribution
    RoundRobin,
    /// Route to node with least connections
    LeastConnections,
    /// Consistent hashing based on key
    ConsistentHashing,
    /// Weighted distribution based on node capabilities
    Weighted,
    /// Prefer local/nearby nodes
    Locality,
}

/// Load balancing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    /// Load balancing strategy
    pub strategy: LoadBalancingStrategy,
    /// Node weights (for weighted strategy)
    pub node_weights: HashMap<String, f64>,
}

impl Default for LoadBalancingConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::RoundRobin,
            node_weights: HashMap::new(),
        }
    }
}

/// Service discovery type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceDiscoveryType {
    /// Static configuration-based discovery
    Static,
    /// DNS-based service discovery
    Dns,
    /// etcd-based service discovery
    Etcd,
    /// Consul-based service discovery
    Consul,
}

/// Service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoveryConfig {
    /// Service discovery type
    pub discovery_type: ServiceDiscoveryType,
    /// Seed nodes for initial cluster discovery
    pub seed_nodes: Vec<String>,
    /// Additional configuration (e.g., etcd endpoints)
    pub additional_config: HashMap<String, String>,
}

impl Default for ServiceDiscoveryConfig {
    fn default() -> Self {
        Self {
            discovery_type: ServiceDiscoveryType::Static,
            seed_nodes: Vec::new(),
            additional_config: HashMap::new(),
        }
    }
}

// ============================================================================
// Selection Criteria for Load Balancing
// ============================================================================

/// Criteria for selecting a node
#[derive(Debug, Clone)]
pub struct SelectionCriteria {
    /// Routing key for consistent hashing
    pub routing_key: Option<String>,
    /// Required capabilities
    pub required_capabilities: Vec<String>,
    /// Exclude these nodes
    pub excluded_nodes: Vec<NodeId>,
    /// Prefer nodes in this datacenter/region
    pub preferred_location: Option<String>,
}

impl Default for SelectionCriteria {
    fn default() -> Self {
        Self {
            routing_key: None,
            required_capabilities: Vec::new(),
            excluded_nodes: Vec::new(),
            preferred_location: None,
        }
    }
}

// ============================================================================
// Membership Events
// ============================================================================

/// Events related to cluster membership changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MembershipEvent {
    /// A new node joined the cluster
    NodeJoined(NodeInfo),
    /// A node left the cluster
    NodeLeft(NodeId),
    /// A node failed
    NodeFailed(NodeId),
    /// A node recovered from failure
    NodeRecovered(NodeId),
    /// Node metadata updated
    NodeUpdated(NodeInfo),
}

impl MembershipEvent {
    /// Get the node ID associated with this event
    pub fn node_id(&self) -> &NodeId {
        match self {
            MembershipEvent::NodeJoined(info) => &info.id,
            MembershipEvent::NodeLeft(id) => id,
            MembershipEvent::NodeFailed(id) => id,
            MembershipEvent::NodeRecovered(id) => id,
            MembershipEvent::NodeUpdated(info) => &info.id,
        }
    }
}

// ============================================================================
// Health Check Results
// ============================================================================

/// Result of a health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Node that was checked
    pub node_id: NodeId,
    /// Whether the node is healthy
    pub healthy: bool,
    /// Latency in milliseconds
    pub latency_ms: f64,
    /// Timestamp of the check
    pub timestamp: SystemTime,
    /// Error message if unhealthy
    pub error_message: Option<String>,
}

// ============================================================================
// Network Statistics
// ============================================================================

/// Network statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkStats {
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Number of active connections
    pub active_connections: usize,
    /// Number of connection errors
    pub connection_errors: u64,
    /// Average message latency in milliseconds
    pub avg_latency_ms: f64,
}

// ============================================================================
// Routing Strategy
// ============================================================================

/// Strategy for routing messages to nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingStrategy {
    /// Direct routing to specific node
    Direct,
    /// Broadcast to all nodes
    Broadcast,
    /// Route to primary node only
    Primary,
    /// Route to primary and replicas
    Quorum,
}
