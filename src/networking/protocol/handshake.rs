// Connection handshake protocol
//
// This module handles the initial handshake between peers when establishing
// a new P2P connection. The handshake includes:
// - Protocol version negotiation
// - Node identification
// - Capability advertisement
// - Authentication preparation

use crate::common::NodeId;
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rand::random;

/// Protocol version supported by this node
pub const SUPPORTED_PROTOCOL_VERSION: u16 = 1;

/// Minimum protocol version this node can communicate with
pub const MIN_PROTOCOL_VERSION: u16 = 1;

/// Handshake request sent by the initiating peer
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct HandshakeRequest {
    /// Protocol version this node supports
    pub protocol_version: u16,

    /// Minimum protocol version this node can work with
    pub min_protocol_version: u16,

    /// Node identifier
    pub node_id: NodeId,

    /// Node capabilities
    pub capabilities: NodeCapabilities,

    /// Cluster name (for validation)
    pub cluster_name: String,

    /// Timestamp of handshake request
    pub timestamp: u64,

    /// Optional authentication token
    pub auth_token: Option<String>,

    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl HandshakeRequest {
    /// Create a new handshake request
    pub fn new(node_id: NodeId, cluster_name: String) -> Self {
        Self {
            protocol_version: SUPPORTED_PROTOCOL_VERSION,
            min_protocol_version: MIN_PROTOCOL_VERSION,
            node_id,
            capabilities: NodeCapabilities::default(),
            cluster_name,
            timestamp: current_timestamp(),
            auth_token: None,
            metadata: HashMap::new(),
        }
    }

    /// Set capabilities
    pub fn with_capabilities(mut self, capabilities: NodeCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Set authentication token
    pub fn with_auth_token(mut self, token: String) -> Self {
        self.auth_token = Some(token);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Validate the handshake request
    pub fn validate(&self, expected_cluster: &str) -> Result<()> {
        // Check cluster name
        if self.cluster_name != expected_cluster {
            return Err(DbError::Network(format!(
                "Cluster name mismatch: expected '{}', got '{}'",
                expected_cluster, self.cluster_name
            )));
        }

        // Check protocol version compatibility
        if self.protocol_version < MIN_PROTOCOL_VERSION {
            return Err(DbError::Network(format!(
                "Unsupported protocol version: {} (minimum: {})",
                self.protocol_version, MIN_PROTOCOL_VERSION
            )));
        }

        // Check timestamp (prevent replay attacks)
        let now = current_timestamp();
        let time_diff = if now > self.timestamp {
            now - self.timestamp
        } else {
            self.timestamp - now
        };

        // Allow 5 minutes clock skew
        if time_diff > 300_000 {
            return Err(DbError::Network(format!(
                "Handshake timestamp too old or too far in future: {} ms difference",
                time_diff
            )));
        }

        Ok(())
    }
}

/// Handshake response sent by the accepting peer
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct HandshakeResponse {
    /// Whether handshake was accepted
    pub accepted: bool,

    /// Protocol version to use for this connection
    pub negotiated_version: u16,

    /// Node identifier of responding node
    pub node_id: NodeId,

    /// Capabilities of responding node
    pub capabilities: NodeCapabilities,

    /// Timestamp of response
    pub timestamp: u64,

    /// Error message if rejected
    pub error_message: Option<String>,

    /// Session identifier for this connection
    pub session_id: Option<String>,

    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl HandshakeResponse {
    /// Create an accepted handshake response
    pub fn accepted(
        node_id: NodeId,
        negotiated_version: u16,
        capabilities: NodeCapabilities,
    ) -> Self {
        Self {
            accepted: true,
            negotiated_version,
            node_id,
            capabilities,
            timestamp: current_timestamp(),
            error_message: None,
            session_id: Some(generate_session_id()),
            metadata: HashMap::new(),
        }
    }

    /// Create a rejected handshake response
    pub fn rejected(error_message: String) -> Self {
        Self {
            accepted: false,
            negotiated_version: 0,
            node_id: String::new(),
            capabilities: NodeCapabilities::default(),
            timestamp: current_timestamp(),
            error_message: Some(error_message),
            session_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Node capabilities advertised during handshake
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeCapabilities {
    /// Supports query execution
    pub supports_query: bool,

    /// Supports replication
    pub supports_replication: bool,

    /// Supports consensus (Raft)
    pub supports_consensus: bool,

    /// Supports data transfer
    pub supports_data_transfer: bool,

    /// Maximum concurrent streams
    pub max_concurrent_streams: u32,

    /// Maximum message size this node can handle
    pub max_message_size: u32,

    /// Supported compression algorithms
    pub compression_algorithms: Vec<String>,

    /// Custom capabilities
    pub custom: HashMap<String, String>,
}

impl Default for NodeCapabilities {
    fn default() -> Self {
        Self {
            supports_query: true,
            supports_replication: true,
            supports_consensus: true,
            supports_data_transfer: true,
            max_concurrent_streams: 100,
            max_message_size: 16 * 1024 * 1024, // 16 MB
            compression_algorithms: vec!["lz4".to_string(), "zstd".to_string()],
            custom: HashMap::new(),
        }
    }
}

impl NodeCapabilities {
    /// Create minimal capabilities
    pub fn minimal() -> Self {
        Self {
            supports_query: false,
            supports_replication: false,
            supports_consensus: false,
            supports_data_transfer: true,
            max_concurrent_streams: 10,
            max_message_size: 1024 * 1024, // 1 MB
            compression_algorithms: vec![],
            custom: HashMap::new(),
        }
    }

    /// Check if capabilities are compatible with another node
    pub fn is_compatible_with(&self, other: &NodeCapabilities) -> bool {
        // Check if there's at least one common compression algorithm
        let has_common_compression = if self.compression_algorithms.is_empty()
            || other.compression_algorithms.is_empty()
        {
            true // No compression required
        } else {
            self.compression_algorithms
                .iter()
                .any(|algo| other.compression_algorithms.contains(algo))
        };

        has_common_compression
    }

    /// Get common capabilities between two nodes
    pub fn common_capabilities(&self, other: &NodeCapabilities) -> NodeCapabilities {
        let common_compressions: Vec<String> = self
            .compression_algorithms
            .iter()
            .filter(|algo| other.compression_algorithms.contains(algo))
            .cloned()
            .collect();

        NodeCapabilities {
            supports_query: self.supports_query && other.supports_query,
            supports_replication: self.supports_replication && other.supports_replication,
            supports_consensus: self.supports_consensus && other.supports_consensus,
            supports_data_transfer: self.supports_data_transfer && other.supports_data_transfer,
            max_concurrent_streams: std::cmp::min(
                self.max_concurrent_streams,
                other.max_concurrent_streams,
            ),
            max_message_size: std::cmp::min(self.max_message_size, other.max_message_size),
            compression_algorithms: common_compressions,
            custom: HashMap::new(),
        }
    }
}

/// Handshake manager
pub struct Handshake {
    node_id: NodeId,
    cluster_name: String,
    capabilities: NodeCapabilities,
}

impl Handshake {
    /// Create a new handshake manager
    pub fn new(node_id: NodeId, cluster_name: String, capabilities: NodeCapabilities) -> Self {
        Self {
            node_id,
            cluster_name,
            capabilities,
        }
    }

    /// Create a handshake request
    pub fn create_request(&self) -> HandshakeRequest {
        HandshakeRequest::new(self.node_id.clone(), self.cluster_name.clone())
            .with_capabilities(self.capabilities.clone())
    }

    /// Process a handshake request and create a response
    pub fn process_request(&self, request: &HandshakeRequest) -> Result<HandshakeResponse> {
        // Validate request
        if let Err(e) = request.validate(&self.cluster_name) {
            return Ok(HandshakeResponse::rejected(e.to_string()));
        }

        // Check protocol version compatibility
        let negotiated_version = std::cmp::min(
            SUPPORTED_PROTOCOL_VERSION,
            request.protocol_version,
        );

        if negotiated_version < MIN_PROTOCOL_VERSION {
            return Ok(HandshakeResponse::rejected(format!(
                "Protocol version incompatible: negotiated {}, minimum required {}",
                negotiated_version, MIN_PROTOCOL_VERSION
            )));
        }

        // Check capability compatibility
        if !self.capabilities.is_compatible_with(&request.capabilities) {
            return Ok(HandshakeResponse::rejected(
                "Incompatible node capabilities".to_string(),
            ));
        }

        // Create accepted response
        let common_caps = self.capabilities.common_capabilities(&request.capabilities);
        Ok(HandshakeResponse::accepted(
            self.node_id.clone(),
            negotiated_version,
            common_caps,
        ))
    }

    /// Validate a handshake response
    pub fn validate_response(&self, response: &HandshakeResponse) -> Result<()> {
        if !response.accepted {
            return Err(DbError::Network(format!(
                "Handshake rejected: {}",
                response.error_message.as_deref().unwrap_or("unknown reason")
            )));
        }

        if response.negotiated_version < MIN_PROTOCOL_VERSION {
            return Err(DbError::Network(format!(
                "Negotiated protocol version {} is below minimum {}",
                response.negotiated_version, MIN_PROTOCOL_VERSION
            )));
        }

        Ok(())
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Generate a random session ID
fn generate_session_id() -> String {
    let random_bytes: [u8; 16] = random();
    hex::encode(random_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handshake_request_creation() {
        let request = HandshakeRequest::new("node1".to_string(), "test-cluster".to_string());

        assert_eq!(request.node_id, "node1");
        assert_eq!(request.cluster_name, "test-cluster");
        assert_eq!(request.protocol_version, SUPPORTED_PROTOCOL_VERSION);
    }

    #[test]
    fn test_handshake_request_validation() {
        let request = HandshakeRequest::new("node1".to_string(), "test-cluster".to_string());

        // Should succeed with matching cluster
        assert!(request.validate("test-cluster").is_ok());

        // Should fail with mismatched cluster
        assert!(request.validate("other-cluster").is_err());
    }

    #[test]
    fn test_handshake_response_accepted() {
        let response = HandshakeResponse::accepted(
            "node2".to_string(),
            1,
            NodeCapabilities::default(),
        );

        assert!(response.accepted);
        assert_eq!(response.node_id, "node2");
        assert!(response.session_id.is_some());
        assert!(response.error_message.is_none());
    }

    #[test]
    fn test_handshake_response_rejected() {
        let response = HandshakeResponse::rejected("Test error".to_string());

        assert!(!response.accepted);
        assert_eq!(response.error_message, Some("Test error".to_string()));
        assert!(response.session_id.is_none());
    }

    #[test]
    fn test_node_capabilities_compatibility() {
        let cap1 = NodeCapabilities::default();
        let cap2 = NodeCapabilities::default();

        assert!(cap1.is_compatible_with(&cap2));

        let cap3 = NodeCapabilities {
            compression_algorithms: vec!["custom".to_string()],
            ..NodeCapabilities::default()
        };

        // Should still be compatible (has common algorithm "lz4")
        assert!(cap1.is_compatible_with(&cap2));
    }

    #[test]
    fn test_common_capabilities() {
        let cap1 = NodeCapabilities {
            supports_query: true,
            supports_replication: true,
            max_concurrent_streams: 100,
            max_message_size: 1000,
            compression_algorithms: vec!["lz4".to_string(), "zstd".to_string()],
            ..NodeCapabilities::default()
        };

        let cap2 = NodeCapabilities {
            supports_query: true,
            supports_replication: false,
            max_concurrent_streams: 50,
            max_message_size: 2000,
            compression_algorithms: vec!["lz4".to_string()],
            ..NodeCapabilities::default()
        };

        let common = cap1.common_capabilities(&cap2);

        assert!(common.supports_query);
        assert!(!common.supports_replication);
        assert_eq!(common.max_concurrent_streams, 50);
        assert_eq!(common.max_message_size, 1000);
        assert_eq!(common.compression_algorithms, vec!["lz4".to_string()]);
    }

    #[test]
    fn test_handshake_process_request() {
        let handshake = Handshake::new(
            "node1".to_string(),
            "test-cluster".to_string(),
            NodeCapabilities::default(),
        );

        let request = HandshakeRequest::new("node2".to_string(), "test-cluster".to_string());

        let response = handshake.process_request(&request).unwrap();
        assert!(response.accepted);
        assert_eq!(response.node_id, "node1");
    }

    #[test]
    fn test_handshake_process_request_wrong_cluster() {
        let handshake = Handshake::new(
            "node1".to_string(),
            "cluster-a".to_string(),
            NodeCapabilities::default(),
        );

        let request = HandshakeRequest::new("node2".to_string(), "cluster-b".to_string());

        let response = handshake.process_request(&request).unwrap();
        assert!(!response.accepted);
        assert!(response.error_message.is_some());
    }

    #[test]
    fn test_session_id_generation() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();

        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 32); // 16 bytes = 32 hex chars
    }
}
