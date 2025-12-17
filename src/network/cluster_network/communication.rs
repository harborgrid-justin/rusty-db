// Cluster Communication Module
//
// Inter-node messaging, connection pooling, and gossip protocol
//
// TODO: CONSOLIDATION NEEDED - ConnectionPool Implementation #2 of 4
// This NodeConnectionPool duplicates connection pooling logic from src/pool/connection_pool.rs.
// RECOMMENDATION: Either delegate to main pool or implement ConnectionPool<T> trait.
// See src/pool/connection_pool.rs for full consolidation analysis.
// See: diagrams/06_network_api_flow.md - Issue #4.3

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};

use super::NodeId;

// ============================================================================
// Message Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone)]
pub struct ClusterMessage {
    pub from: NodeId,
    pub to: NodeId,
    pub priority: MessagePriority,
    pub payload: Vec<u8>,
}

impl ClusterMessage {
    pub fn new(from: NodeId, to: NodeId, priority: MessagePriority, payload: Vec<u8>) -> Self {
        Self {
            from,
            to,
            priority,
            payload,
        }
    }
}

// ============================================================================
// TLS Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
    pub ca_path: String,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: "./certs/server.crt".to_string(),
            key_path: "./certs/server.key".to_string(),
            ca_path: "./certs/ca.crt".to_string(),
        }
    }
}

// ============================================================================
// Node Connection Pool
// ============================================================================

#[derive(Debug, Clone)]
pub struct NodeConnection {
    pub node_id: NodeId,
    pub address: SocketAddr,
    pub connected_at: SystemTime,
}

pub struct NodeConnectionPool {
    max_connections: usize,
    connections: HashMap<NodeId, Vec<NodeConnection>>,
}

impl NodeConnectionPool {
    pub fn new(max_connections: usize) -> Self {
        Self {
            max_connections,
            connections: HashMap::new(),
        }
    }

    pub fn max_connections(&self) -> usize {
        self.max_connections
    }

    pub fn add_connection(&mut self, connection: NodeConnection) {
        self.connections
            .entry(connection.node_id)
            .or_insert_with(Vec::new)
            .push(connection);
    }

    pub fn get_connections(&self, node_id: NodeId) -> Option<&Vec<NodeConnection>> {
        self.connections.get(&node_id)
    }

    pub fn remove_node_connections(&mut self, node_id: NodeId) -> Option<Vec<NodeConnection>> {
        self.connections.remove(&node_id)
    }

    pub fn total_connections(&self) -> usize {
        self.connections.values().map(|v| v.len()).sum()
    }

    pub fn connection_count_for_node(&self, node_id: NodeId) -> usize {
        self.connections
            .get(&node_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }
}

// ============================================================================
// Communication Metrics
// ============================================================================

#[derive(Debug, Clone)]
pub struct CommunicationMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

impl Default for CommunicationMetrics {
    fn default() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }
}

// ============================================================================
// Gossip Protocol
// ============================================================================

pub struct GossipProtocol {
    fanout: usize,
    messages_gossiped: u64,
}

impl GossipProtocol {
    pub fn new(fanout: usize) -> Self {
        Self {
            fanout,
            messages_gossiped: 0,
        }
    }

    pub fn fanout(&self) -> usize {
        self.fanout
    }

    pub fn set_fanout(&mut self, fanout: usize) {
        self.fanout = fanout;
    }

    pub fn record_gossip(&mut self) {
        self.messages_gossiped += 1;
    }

    pub fn gossip_count(&self) -> u64 {
        self.messages_gossiped
    }
}

// ============================================================================
// Reliable Messaging
// ============================================================================

pub struct ReliableMessaging {
    retry_count: usize,
    timeout: Duration,
    retries_performed: u64,
}

impl ReliableMessaging {
    pub fn new(retry_count: usize, timeout: Duration) -> Self {
        Self {
            retry_count,
            timeout,
            retries_performed: 0,
        }
    }

    pub fn retry_count(&self) -> usize {
        self.retry_count
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn record_retry(&mut self) {
        self.retries_performed += 1;
    }

    pub fn retries_performed(&self) -> u64 {
        self.retries_performed
    }
}

impl Default for ReliableMessaging {
    fn default() -> Self {
        Self::new(3, Duration::from_secs(5))
    }
}
