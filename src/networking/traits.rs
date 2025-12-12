// Standard traits for the RustyDB networking layer
//
// This module defines the core traits that all networking components must implement.
// These traits provide standard interfaces for transport, service discovery, health
// monitoring, load balancing, and cluster membership management.

use crate::error::Result;
use crate::common::{Component, HealthStatus};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;

use super::types::{
    ClusterMessage, HealthCheckResult, MembershipEvent, NodeAddress, NodeId, NodeInfo,
    SelectionCriteria,
};

// ============================================================================
// NetworkTransport - Low-level message transport
// ============================================================================

/// Trait for network transport implementations
///
/// Responsible for low-level sending and receiving of cluster messages.
/// Implementations handle connection management, serialization, compression,
/// and encryption.
#[async_trait]
pub trait NetworkTransport: Component + Send + Sync {
    /// Send a message to a specific node address
    ///
    /// # Arguments
    /// * `addr` - The network address of the destination node
    /// * `msg` - The cluster message to send
    ///
    /// # Returns
    /// * `Ok(())` if the message was sent successfully
    /// * `Err` if the send failed (connection error, serialization error, etc.)
    async fn send(&self, addr: &NodeAddress, msg: &ClusterMessage) -> Result<()>;

    /// Send a message to a node identified by NodeId
    ///
    /// The transport layer should maintain a mapping of NodeId to NodeAddress.
    ///
    /// # Arguments
    /// * `node_id` - The ID of the destination node
    /// * `msg` - The cluster message to send
    async fn send_to_node(&self, node_id: &NodeId, msg: &ClusterMessage) -> Result<()>;

    /// Receive the next message from the network
    ///
    /// This is a blocking operation that waits for the next message.
    ///
    /// # Returns
    /// * `Ok((source, message))` - The source address and received message
    /// * `Err` if receiving failed
    async fn receive(&self) -> Result<(NodeAddress, ClusterMessage)>;

    /// Broadcast a message to all known nodes
    ///
    /// # Arguments
    /// * `msg` - The cluster message to broadcast
    ///
    /// # Returns
    /// * `Ok(successful_count)` - Number of nodes that successfully received the message
    /// * `Err` if the broadcast failed
    async fn broadcast(&self, msg: &ClusterMessage) -> Result<usize>;

    /// Get the number of active connections
    fn active_connections(&self) -> usize;

    /// Get the local bind address
    fn local_address(&self) -> &NodeAddress;

    /// Close connection to a specific node
    async fn close_connection(&self, node_id: &NodeId) -> Result<()>;

    /// Get statistics for all connections
    fn connection_stats(&self) -> HashMap<NodeId, ConnectionStats>;
}

/// Statistics for a single connection
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    /// Number of messages sent
    pub messages_sent: u64,
    /// Number of messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
}

// ============================================================================
// ServiceDiscovery - Node discovery and registration
// ============================================================================

/// Trait for service discovery implementations
///
/// Responsible for discovering nodes in the cluster and registering the local node.
#[async_trait]
pub trait ServiceDiscovery: Component + Send + Sync {
    /// Register the local node with the service discovery system
    ///
    /// # Arguments
    /// * `node` - Information about the local node
    ///
    /// # Returns
    /// * `Ok(())` if registration succeeded
    /// * `Err` if registration failed
    async fn register_node(&self, node: &NodeInfo) -> Result<()>;

    /// Unregister the local node from the service discovery system
    ///
    /// This should be called during graceful shutdown.
    async fn unregister_node(&self) -> Result<()>;

    /// Discover all available nodes in the cluster
    ///
    /// # Returns
    /// * `Ok(nodes)` - List of discovered nodes
    /// * `Err` if discovery failed
    async fn discover_nodes(&self) -> Result<Vec<NodeInfo>>;

    /// Get information about a specific node
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node to look up
    ///
    /// # Returns
    /// * `Ok(Some(node))` - Node information if found
    /// * `Ok(None)` - Node not found
    /// * `Err` if lookup failed
    async fn get_node(&self, node_id: &NodeId) -> Result<Option<NodeInfo>>;

    /// Watch for changes in the cluster topology
    ///
    /// Returns a channel receiver that will receive membership events.
    ///
    /// # Returns
    /// * `Ok(receiver)` - Channel for receiving membership events
    /// * `Err` if watch setup failed
    async fn watch_changes(&self) -> Result<Receiver<MembershipEvent>>;

    /// Update metadata for the local node
    ///
    /// # Arguments
    /// * `metadata` - Key-value pairs to update
    async fn update_metadata(&self, metadata: HashMap<String, String>) -> Result<()>;
}

// ============================================================================
// HealthMonitor - Health checking and failure detection
// ============================================================================

/// Trait for health monitoring implementations
///
/// Responsible for continuously monitoring the health of cluster nodes and
/// detecting failures.
#[async_trait]
pub trait HealthMonitor: Component + Send + Sync {
    /// Check the health of a specific node
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node to check
    ///
    /// # Returns
    /// * `Ok(result)` - Health check result
    /// * `Err` if the health check couldn't be performed
    async fn check_health(&self, node_id: &NodeId) -> Result<HealthCheckResult>;

    /// Check the health of all known nodes
    ///
    /// # Returns
    /// * `Ok(results)` - Map of node IDs to health check results
    async fn check_all_nodes(&self) -> Result<HashMap<NodeId, HealthCheckResult>>;

    /// Get the current health status of a node
    ///
    /// This returns cached status without performing a new check.
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node
    ///
    /// # Returns
    /// * `Some(status)` - Cached health status
    /// * `None` - No health information available
    fn get_node_health(&self, node_id: &NodeId) -> Option<HealthStatus>;

    /// Get list of unhealthy nodes
    ///
    /// # Returns
    /// * List of node IDs that are currently unhealthy
    fn get_unhealthy_nodes(&self) -> Vec<NodeId>;

    /// Get list of healthy nodes
    ///
    /// # Returns
    /// * List of node IDs that are currently healthy
    fn get_healthy_nodes(&self) -> Vec<NodeId>;

    /// Start continuous health monitoring
    ///
    /// This should spawn a background task that periodically checks all nodes.
    async fn start_monitoring(&self) -> Result<()>;

    /// Stop continuous health monitoring
    async fn stop_monitoring(&self) -> Result<()>;

    /// Subscribe to health change events
    ///
    /// # Returns
    /// * `Ok(receiver)` - Channel for receiving health status changes
    async fn subscribe_health_changes(&self) -> Result<Receiver<HealthChangeEvent>>;
}

/// Event representing a change in node health status
#[derive(Debug, Clone)]
pub struct HealthChangeEvent {
    /// Node that changed health status
    pub node_id: NodeId,
    /// Previous health status
    pub previous_status: HealthStatus,
    /// New health status
    pub new_status: HealthStatus,
    /// Timestamp of the change
    pub timestamp: std::time::SystemTime,
}

// ============================================================================
// LoadBalancer - Traffic distribution across nodes
// ============================================================================

/// Trait for load balancer implementations
///
/// Responsible for selecting the best node for routing requests based on
/// various strategies (round-robin, least connections, etc.).
pub trait LoadBalancer: Component + Send + Sync {
    /// Select a node based on the given criteria
    ///
    /// # Arguments
    /// * `criteria` - Selection criteria (routing key, capabilities, etc.)
    ///
    /// # Returns
    /// * `Some(node_id)` - Selected node ID
    /// * `None` - No suitable node found
    fn select_node(&self, criteria: &SelectionCriteria) -> Option<NodeId>;

    /// Select multiple nodes (for operations requiring replication)
    ///
    /// # Arguments
    /// * `criteria` - Selection criteria
    /// * `count` - Number of nodes to select
    ///
    /// # Returns
    /// * `Vec<NodeId>` - List of selected node IDs (may be less than count)
    fn select_nodes(&self, criteria: &SelectionCriteria, count: usize) -> Vec<NodeId>;

    /// Update the weight of a node (for weighted load balancing)
    ///
    /// # Arguments
    /// * `node_id` - The node to update
    /// * `weight` - New weight (higher = more traffic)
    fn update_node_weight(&mut self, node_id: &NodeId, weight: f64);

    /// Update weights for multiple nodes
    ///
    /// # Arguments
    /// * `weights` - Map of node IDs to weights
    fn update_weights(&mut self, weights: HashMap<NodeId, f64>);

    /// Mark a node as unavailable (should not receive traffic)
    ///
    /// # Arguments
    /// * `node_id` - The node to mark as unavailable
    fn mark_node_unavailable(&mut self, node_id: &NodeId);

    /// Mark a node as available (can receive traffic)
    ///
    /// # Arguments
    /// * `node_id` - The node to mark as available
    fn mark_node_available(&mut self, node_id: &NodeId);

    /// Get the current load distribution across nodes
    ///
    /// # Returns
    /// * Map of node IDs to their current load (number of active requests)
    fn get_load_distribution(&self) -> HashMap<NodeId, usize>;

    /// Record the completion of a request to a node
    ///
    /// This is used for load tracking in strategies like least connections.
    ///
    /// # Arguments
    /// * `node_id` - The node that handled the request
    fn record_request_completion(&mut self, node_id: &NodeId);
}

// ============================================================================
// ClusterMembership - Cluster state management
// ============================================================================

/// Trait for cluster membership management
///
/// Responsible for maintaining the cluster membership view and handling
/// join/leave operations.
#[async_trait]
pub trait ClusterMembership: Component + Send + Sync {
    /// Get the current list of cluster members
    ///
    /// # Returns
    /// * `Vec<NodeInfo>` - List of all cluster members
    fn get_members(&self) -> Vec<NodeInfo>;

    /// Get information about a specific member
    ///
    /// # Arguments
    /// * `node_id` - The ID of the member
    ///
    /// # Returns
    /// * `Some(node_info)` - Member information if found
    /// * `None` - Member not found
    fn get_member(&self, node_id: &NodeId) -> Option<NodeInfo>;

    /// Update the state of a cluster member
    ///
    /// # Arguments
    /// * `node_id` - The ID of the member to update
    /// * `state` - New state
    fn update_member_state(&mut self, node_id: &NodeId, state: super::types::NodeState);

    /// Join the cluster
    ///
    /// # Arguments
    /// * `local_node` - Information about the local node
    /// * `seed_nodes` - Initial seed nodes to contact
    ///
    /// # Returns
    /// * `Ok(())` if joined successfully
    /// * `Err` if join failed
    async fn join_cluster(&self, local_node: NodeInfo, seed_nodes: Vec<NodeAddress>) -> Result<()>;

    /// Leave the cluster gracefully
    ///
    /// # Returns
    /// * `Ok(())` if left successfully
    /// * `Err` if leave failed
    async fn leave_cluster(&self) -> Result<()>;

    /// Add a member to the cluster
    ///
    /// # Arguments
    /// * `node` - Information about the node to add
    fn add_member(&mut self, node: NodeInfo);

    /// Remove a member from the cluster
    ///
    /// # Arguments
    /// * `node_id` - The ID of the member to remove
    fn remove_member(&mut self, node_id: &NodeId);

    /// Get the local node ID
    fn local_node_id(&self) -> &NodeId;

    /// Get the number of members in the cluster
    fn member_count(&self) -> usize;

    /// Check if a node is a member of the cluster
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node to check
    ///
    /// # Returns
    /// * `true` if the node is a member, `false` otherwise
    fn is_member(&self, node_id: &NodeId) -> bool;

    /// Subscribe to membership change events
    ///
    /// # Returns
    /// * `Ok(receiver)` - Channel for receiving membership events
    async fn subscribe_membership_changes(&self) -> Result<Receiver<MembershipEvent>>;

    /// Perform gossip round to disseminate membership information
    ///
    /// This should be called periodically by the gossip protocol.
    async fn gossip_round(&self) -> Result<()>;
}

// ============================================================================
// MessageHandler - Handle incoming messages
// ============================================================================

/// Trait for handling specific message types
///
/// Components can implement this to handle specific cluster message types.
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Get the message types this handler can handle
    fn message_types(&self) -> Vec<&'static str>;

    /// Handle an incoming message
    ///
    /// # Arguments
    /// * `from` - The node that sent the message
    /// * `message` - The cluster message
    ///
    /// # Returns
    /// * `Ok(())` if the message was handled successfully
    /// * `Err` if handling failed
    async fn handle_message(&self, from: &NodeId, message: &ClusterMessage) -> Result<()>;
}

// ============================================================================
// CircuitBreaker - Fault tolerance
// ============================================================================

/// Trait for circuit breaker implementations
///
/// Prevents cascading failures by stopping requests to failing services.
pub trait CircuitBreaker: Send + Sync {
    /// Check if the circuit is open (requests should be blocked)
    fn is_open(&self) -> bool;

    /// Check if the circuit is closed (requests allowed)
    fn is_closed(&self) -> bool;

    /// Check if the circuit is half-open (testing recovery)
    fn is_half_open(&self) -> bool;

    /// Record a successful operation
    fn record_success(&mut self);

    /// Record a failed operation
    fn record_failure(&mut self);

    /// Reset the circuit breaker to closed state
    fn reset(&mut self);

    /// Get the current state as a string
    fn state(&self) -> &str;
}

// ============================================================================
// ConnectionPool - Connection pooling
// ============================================================================

/// Trait for connection pool implementations
///
/// Manages a pool of connections to remote nodes.
#[async_trait]
pub trait ConnectionPool: Component + Send + Sync {
    /// Get a connection to a specific node
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node to connect to
    ///
    /// # Returns
    /// * `Ok(connection)` - A connection handle
    /// * `Err` if connection failed
    async fn get_connection(&self, node_id: &NodeId) -> Result<Box<dyn Connection>>;

    /// Return a connection to the pool
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node
    /// * `connection` - The connection to return
    async fn return_connection(&self, node_id: &NodeId, connection: Box<dyn Connection>);

    /// Get the number of active connections in the pool
    fn active_connections(&self) -> usize;

    /// Get the number of idle connections in the pool
    fn idle_connections(&self) -> usize;

    /// Close all connections to a specific node
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node
    async fn close_connections(&self, node_id: &NodeId) -> Result<()>;
}

/// Trait representing a single connection
#[async_trait]
pub trait Connection: Send + Sync {
    /// Send a message over this connection
    async fn send(&mut self, message: &ClusterMessage) -> Result<()>;

    /// Receive a message from this connection
    async fn receive(&mut self) -> Result<ClusterMessage>;

    /// Check if the connection is still alive
    fn is_alive(&self) -> bool;

    /// Close the connection
    async fn close(&mut self) -> Result<()>;
}
