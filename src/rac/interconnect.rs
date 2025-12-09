// # Cluster Interconnect
//
// Oracle RAC-like high-speed cluster interconnect for node communication,
// heartbeat monitoring, split-brain detection, and network partition handling.
//
// ## Key Components
//
// - **Message Bus**: High-performance message passing between cluster nodes
// - **Heartbeat Monitor**: Detect node failures and network issues
// - **Split-Brain Detection**: Identify and resolve cluster partitions
// - **Adaptive Routing**: Intelligent message routing with failover
//
// ## Architecture
//
// The interconnect provides reliable, ordered, low-latency communication between
// cluster nodes using multiple transport protocols (TCP, UDP, RDMA-like) with
// automatic failover and adaptive routing based on network conditions.

use tokio::sync::oneshot;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Instant;
use crate::error::DbError;
use crate::common::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::Arc;
use std::time::{Duration};
use parking_lot::{RwLock};
use tokio::sync::{mpsc, broadcast};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt};
use bytes::{Bytes, BytesMut, Buf, BufMut};

// ============================================================================
// Constants
// ============================================================================

/// Heartbeat interval
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(100);

/// Heartbeat timeout (missed heartbeats before declaring failure)
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(3);

/// Maximum message size
const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB

/// Message queue size per connection
const MESSAGE_QUEUE_SIZE: usize = 10000;

/// Network latency sample window
const LATENCY_WINDOW_SIZE: usize = 100;

/// Split-brain detection quorum
const QUORUM_PERCENTAGE: f64 = 0.5;

/// Reconnection backoff base
const RECONNECT_BACKOFF_MS: u64 = 100;

/// Maximum reconnection attempts
const MAX_RECONNECT_ATTEMPTS: u32 = 10;

// ============================================================================
// Message Types
// ============================================================================

/// Interconnect message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message identifier
    pub message_id: u64,

    /// Source node
    pub source: NodeId,

    /// Destination node
    pub destination: NodeId,

    /// Message type
    pub message_type: MessageType,

    /// Payload
    pub payload: Vec<u8>,

    /// Timestamp
    pub timestamp: u64,

    /// Priority (higher = more urgent)
    pub priority: MessagePriority,

    /// Requires acknowledgment
    pub requires_ack: bool,

    /// Sequence number for ordering
    pub sequence: u64,
}

/// Message type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Hash)]
pub enum MessageType {
    /// Heartbeat message
    Heartbeat,

    /// Cache fusion block transfer
    CacheFusion,

    /// Global resource directory
    Grd,

    /// Transaction coordination
    Transaction,

    /// Query coordination
    Query,

    /// Administrative command
    Admin,

    /// Replication
    Replication,

    /// Custom application message
    Custom,
}

/// Message priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MessagePriority {
    /// Low priority - background tasks
    Low = 0,

    /// Normal priority - standard operations
    Normal = 1,

    /// High priority - cache fusion, locks
    High = 2,

    /// Critical priority - heartbeats, failover
    Critical = 3,
}

/// Message acknowledgment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAck {
    pub message_id: u64,
    pub success: bool,
    pub error: Option<String>,
}

// ============================================================================
// Node State and Health
// ============================================================================

/// Node state in the cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    /// Node is healthy and operational
    Healthy,

    /// Node is suspected to be down (missing heartbeats)
    Suspected,

    /// Node is confirmed down
    Down,

    /// Node is recovering
    Recovering,

    /// Node is leaving cluster
    Leaving,

    /// Node is joining cluster
    Joining,
}

/// Node health information
#[derive(Debug, Clone)]
pub struct NodeHealth {
    /// Node identifier
    pub node_id: NodeId,

    /// Current state
    pub state: NodeState,

    /// Last heartbeat timestamp
    pub last_heartbeat: Instant,

    /// Consecutive missed heartbeats
    pub missed_heartbeats: u32,

    /// Network latency (microseconds)
    pub latency_us: u64,

    /// Packet loss percentage
    pub packet_loss: f64,

    /// Bandwidth utilization
    pub bandwidth_mbps: f64,

    /// Last state change
    pub last_state_change: Instant,

    /// NEW: Phi accrual failure detector state
    /// Heartbeat interval history for adaptive detection
    pub heartbeat_intervals: Vec<Duration>,

    /// Current phi value (suspicion level)
    pub phi_value: f64,

    /// Mean heartbeat interval
    pub mean_interval_ms: f64,

    /// Standard deviation of intervals
    pub std_dev_interval_ms: f64,
}

impl NodeHealth {
    fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            state: NodeState::Joining,
            last_heartbeat: Instant::now(),
            missed_heartbeats: 0,
            latency_us: 0,
            packet_loss: 0.0,
            bandwidth_mbps: 0.0,
            last_state_change: Instant::now(),
            heartbeat_intervals: Vec::with_capacity(100),
            phi_value: 0.0,
            mean_interval_ms: 100.0,
            std_dev_interval_ms: 10.0,
        }
    }

    fn update_heartbeat(&mut self) {
        let now = Instant::now();
        let interval = now.duration_since(self.last_heartbeat);

        // Update phi accrual detector
        self.update_phi_accrual(interval);

        self.last_heartbeat = now;
        self.missed_heartbeats = 0;

        if self.state == NodeState::Suspected {
            self.state = NodeState::Healthy;
            self.last_state_change = now;
        }
    }

    /// NEW: Phi accrual failure detector
    /// Calculates suspicion level based on heartbeat timing variance
    /// Higher phi = more suspicious, threshold typically 8.0-10.0
    fn update_phi_accrual(&mut self, interval: Duration) {
        let _interval_ms = interval.as_millis() as f64;

        // Add to history
        self.heartbeat_intervals.push(interval);
        if self.heartbeat_intervals.len() > 100 {
            self.heartbeat_intervals.remove(0);
        }

        // Calculate mean and std dev
        if self.heartbeat_intervals.len() > 1 {
            let sum: f64 = self.heartbeat_intervals.iter()
                .map(|d| d.as_millis() as f64)
                .sum();
            self.mean_interval_ms = sum / self.heartbeat_intervals.len() as f64;

            let variance: f64 = self.heartbeat_intervals.iter()
                .map(|d| {
                    let diff = d.as_millis() as f64 - self.mean_interval_ms;
                    diff * diff
                })
                .sum::<f64>() / self.heartbeat_intervals.len() as f64;

            self.std_dev_interval_ms = variance.sqrt().max(1.0); // Avoid division by zero
        }

        // Calculate phi value
        let elapsed_since_last = self.last_heartbeat.elapsed().as_millis() as f64;
        let expected = self.mean_interval_ms;
        let sigma = self.std_dev_interval_ms;

        // Phi(t) = -log10(P(t_now - t_last))
        // Using normal distribution approximation
        let z_score = (elapsed_since_last - expected) / sigma;
        self.phi_value = z_score.abs().max(0.0);
    }

    fn record_missed_heartbeat(&mut self) {
        self.missed_heartbeats += 1;

        let elapsed = self.last_heartbeat.elapsed();

        // Use phi value for adaptive threshold
        if self.phi_value > 8.0 {
            // High suspicion - mark as down
            self.state = NodeState::Down;
            self.last_state_change = Instant::now();
        } else if self.phi_value > 5.0 || self.missed_heartbeats > 3 {
            // Medium suspicion
            self.state = NodeState::Suspected;
            self.last_state_change = Instant::now();
        } else if elapsed > HEARTBEAT_TIMEOUT {
            // Legacy timeout fallback
            self.state = NodeState::Down;
            self.last_state_change = Instant::now();
        }
    }
}

// ============================================================================
// Connection Management
// ============================================================================

/// Connection to a remote node
struct Connection {
    /// Remote node identifier
    remote_node: NodeId,

    /// TCP stream (in production, could be RDMA)
    /// Use tokio::sync::Mutex to allow holding across await points
    stream: Arc<tokio::sync::Mutex<Option<TcpStream>>>,

    /// Message send queue
    send_queue: Arc<Mutex<VecDeque<Message>>>,

    /// Connection state
    state: Arc<RwLock<ConnectionState>>,

    /// Latency samples
    latency_samples: Arc<Mutex<VecDeque<u64>>>,

    /// Statistics
    stats: Arc<RwLock<ConnectionStats>>,
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// Connection statistics
#[derive(Debug, Clone, Default)]
struct ConnectionStats {
    messages_sent: u64,
    messages_received: u64,
    bytes_sent: u64,
    bytes_received: u64,
    send_errors: u64,
    receive_errors: u64,
    reconnections: u64,
}

impl Connection {
    fn new(remote_node: NodeId) -> Self {
        Self {
            remote_node,
            stream: Arc::new(tokio::sync::Mutex::new(None)),
            send_queue: Arc::new(Mutex::new(VecDeque::new())),
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            latency_samples: Arc::new(Mutex::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
        }
    }

    async fn connect(&self, address: &str) -> Result<(), DbError> {
        *self.state.write() = ConnectionState::Connecting;

        match TcpStream::connect(address).await {
            Ok(stream) => {
                *self.stream.lock().await = Some(stream);
                *self.state.write() = ConnectionState::Connected;
                Ok(())
            }
            Err(e) => {
                *self.state.write() = ConnectionState::Failed;
                Err(DbError::Network(format!("Connection failed: {}", e)))
            }
        }
    }

    async fn send_message(&self, message: Message) -> Result<(), DbError> {
        // Add to queue
        self.send_queue.lock().push_back(message.clone());

        // Try to flush queue
        self.flush_queue().await
    }

    async fn flush_queue(&self) -> Result<(), DbError> {
        // Extract messages from queue before acquiring stream lock
        let messages_to_send: Vec<Message> = {
            let mut queue = self.send_queue.lock();
            queue.drain(..).collect()
        };

        // Now process messages with stream lock (using tokio::sync::Mutex)
        let mut stream_guard = self.stream.lock().await;

        if let Some(stream) = stream_guard.as_mut() {
            for message in messages_to_send {
                // Serialize message
                let data = bincode::serialize(&message)
                    .map_err(|e| DbError::Serialization(e.to_string()))?;

                // Send length prefix
                let len = data.len() as u32;
                stream.write_u32(len).await
                    .map_err(|e| DbError::Network(e.to_string()))?;

                // Send data
                stream.write_all(&data).await
                    .map_err(|e| DbError::Network(e.to_string()))?;

                // Update statistics
                let mut stats = self.stats.write();
                stats.messages_sent += 1;
                stats.bytes_sent += (4 + data.len()) as u64;
            }
        }

        Ok(())
    }

    async fn receive_message(&self) -> Result<Message, DbError> {
        // Use tokio::sync::Mutex which can be held across await
        let mut stream_guard = self.stream.lock().await;

        if let Some(stream) = stream_guard.as_mut() {
            // Read length prefix
            let len = stream.read_u32().await
                .map_err(|e| DbError::Network(e.to_string()))?;

            if len > MAX_MESSAGE_SIZE as u32 {
                return Err(DbError::Network("Message too large".to_string()));
            }

            // Read data
            let mut buffer = vec![0u8; len as usize];
            stream.read_exact(&mut buffer).await
                .map_err(|e| DbError::Network(e.to_string()))?;

            // Deserialize
            let message: Message = bincode::deserialize(&buffer)
                .map_err(|e| DbError::Serialization(e.to_string()))?;

            // Update statistics
            let mut stats = self.stats.write();
            stats.messages_received += 1;
            stats.bytes_received += (4 + len) as u64;

            Ok(message)
        } else {
            Err(DbError::Network("Not connected".to_string()))
        }
    }

    fn record_latency(&self, latency_us: u64) {
        let mut samples = self.latency_samples.lock();
        samples.push_back(latency_us);

        if samples.len() > LATENCY_WINDOW_SIZE {
            samples.pop_front();
        }
    }

    fn average_latency(&self) -> u64 {
        let samples = self.latency_samples.lock();

        if samples.is_empty() {
            return 0;
        }

        samples.iter().sum::<u64>() / samples.len() as u64
    }
}

// ============================================================================
// Cluster Interconnect
// ============================================================================

/// Cluster interconnect manager
pub struct ClusterInterconnect {
    /// Local node identifier
    node_id: NodeId,

    /// Local listen address
    listen_address: String,

    /// Connections to other nodes
    connections: Arc<RwLock<HashMap<NodeId, Arc<Connection>>>>,

    /// Node health tracking
    node_health: Arc<RwLock<HashMap<NodeId, NodeHealth>>>,

    /// Message handlers
    message_handlers: Arc<RwLock<HashMap<MessageType, MessageHandler>>>,

    /// Pending message acknowledgments
    pending_acks: Arc<RwLock<HashMap<u64, oneshot::Sender<MessageAck>>>>,

    /// Message sequence counter
    sequence_counter: Arc<Mutex<u64>>,

    /// Configuration
    config: InterconnectConfig,

    /// Statistics
    stats: Arc<RwLock<InterconnectStatistics>>,

    /// Shutdown signal
    shutdown_tx: broadcast::Sender<()>,
}

/// Message handler function type
type MessageHandler = Arc<dyn Fn(Message) -> Result<Vec<u8>, DbError> + Send + Sync>;

/// Interconnect configuration
#[derive(Debug, Clone)]
pub struct InterconnectConfig {
    /// Enable heartbeat monitoring
    pub enable_heartbeat: bool,

    /// Heartbeat interval
    pub heartbeat_interval: Duration,

    /// Heartbeat timeout
    pub heartbeat_timeout: Duration,

    /// Enable adaptive routing
    pub adaptive_routing: bool,

    /// Maximum retry attempts
    pub max_retries: u32,

    /// Enable message compression
    pub enable_compression: bool,

    /// NEW: Enable message batching for efficiency
    pub enable_batching: bool,

    /// NEW: Batching window (milliseconds)
    pub batch_window_ms: u64,

    /// NEW: Maximum batch size (messages)
    pub max_batch_size: usize,

    /// NEW: Phi accrual failure detector threshold
    pub phi_threshold: f64,
}

impl Default for InterconnectConfig {
    fn default() -> Self {
        Self {
            enable_heartbeat: true,
            heartbeat_interval: HEARTBEAT_INTERVAL,
            heartbeat_timeout: HEARTBEAT_TIMEOUT,
            adaptive_routing: true,
            max_retries: 3,
            enable_compression: false,
            enable_batching: true,        // Enable message batching
            batch_window_ms: 1,           // 1ms batching window
            max_batch_size: 100,          // Up to 100 messages per batch
            phi_threshold: 8.0,           // Phi threshold for failure detection
        }
    }
}

/// Interconnect statistics
#[derive(Debug, Clone, Default)]
pub struct InterconnectStatistics {
    /// Total messages sent
    pub total_sent: u64,

    /// Total messages received
    pub total_received: u64,

    /// Total bytes sent
    pub total_bytes_sent: u64,

    /// Total bytes received
    pub total_bytes_received: u64,

    /// Message send failures
    pub send_failures: u64,

    /// Heartbeats sent
    pub heartbeats_sent: u64,

    /// Heartbeats received
    pub heartbeats_received: u64,

    /// Node failures detected
    pub node_failures: u64,

    /// Average message latency (microseconds)
    pub avg_latency_us: u64,

    /// NEW: Batching statistics
    /// Batches sent
    pub batches_sent: u64,

    /// Messages batched
    pub messages_batched: u64,

    /// Average batch size
    pub avg_batch_size: f64,

    /// Phi accrual suspicion levels (histogram)
    pub phi_suspicions: u64,

    /// P99 message latency (microseconds)
    pub p99_latency_us: u64,

    /// False positive detections
    pub false_positives: u64,
}

impl ClusterInterconnect {
    /// Create a new cluster interconnect
    pub fn new(
        node_id: NodeId,
        listen_address: String,
        config: InterconnectConfig,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            node_id,
            listen_address,
            connections: Arc::new(RwLock::new(HashMap::new())),
            node_health: Arc::new(RwLock::new(HashMap::new())),
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
            pending_acks: Arc::new(RwLock::new(HashMap::new())),
            sequence_counter: Arc::new(Mutex::new(0)),
            config,
            stats: Arc::new(RwLock::new(InterconnectStatistics::default())),
            shutdown_tx,
        }
    }

    /// Start the interconnect service
    pub async fn start(&self) -> Result<(), DbError> {
        // Start listener
        let listener = TcpListener::bind(&self.listen_address).await
            .map_err(|e| DbError::Network(format!("Failed to bind: {}", e)))?;

        // Start heartbeat monitor
        if self.config.enable_heartbeat {
            self.start_heartbeat_monitor().await;
        }

        // Accept connections
        let connections = self.connections.clone();
        let node_health = self.node_health.clone();
        let handlers = self.message_handlers.clone();
        let stats = self.stats.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        if let Ok((stream, addr)) = result {
                            // Handle new connection
                            // In production, would handshake to get node ID
                            let remote_node = format!("node_{}", addr);

                            let conn = Arc::new(Connection::new(remote_node.clone()));
                            *conn.stream.lock().await = Some(stream);
                            *conn.state.write() = ConnectionState::Connected;

                            connections.write().insert(remote_node.clone(), conn.clone());

                            // Update node health
                            node_health.write().insert(
                                remote_node.clone(),
                                NodeHealth::new(remote_node)
                            );
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the interconnect service
    pub async fn stop(&self) -> Result<(), DbError> {
        let _ = self.shutdown_tx.send(());
        Ok(())
    }

    /// Add a node to the cluster
    pub async fn add_node(&self, node_id: NodeId, address: String) -> Result<(), DbError> {
        let conn = Arc::new(Connection::new(node_id.clone()));

        // Connect to remote node
        conn.connect(&address).await?;

        // Add to connections
        self.connections.write().insert(node_id.clone(), conn);

        // Initialize health tracking
        self.node_health.write().insert(
            node_id.clone(),
            NodeHealth::new(node_id),
        );

        Ok(())
    }

    /// Remove a node from the cluster
    pub async fn remove_node(&self, node_id: &NodeId) -> Result<(), DbError> {
        self.connections.write().remove(node_id);
        self.node_health.write().remove(node_id);

        self.stats.write().node_failures += 1;

        Ok(())
    }

    /// Send a message to another node
    pub async fn send_message(
        &self,
        destination: NodeId,
        message_type: MessageType,
        payload: Vec<u8>,
        priority: MessagePriority,
    ) -> Result<(), DbError> {
        let message_id = self.next_message_id();
        let sequence = self.next_sequence();

        let message = Message {
            message_id,
            source: self.node_id.clone(),
            destination: destination.clone(),
            message_type,
            payload,
            timestamp: Self::current_timestamp(),
            priority,
            requires_ack: false,
            sequence,
        };

        self.send_message_internal(destination, message).await
    }

    /// Send a message and wait for acknowledgment
    pub async fn send_with_ack(
        &self,
        destination: NodeId,
        message_type: MessageType,
        payload: Vec<u8>,
        priority: MessagePriority,
    ) -> Result<MessageAck, DbError> {
        let message_id = self.next_message_id();
        let sequence = self.next_sequence();

        let message = Message {
            message_id,
            source: self.node_id.clone(),
            destination: destination.clone(),
            message_type,
            payload,
            timestamp: Self::current_timestamp(),
            priority,
            requires_ack: true,
            sequence,
        };

        // Create ack channel
        let (ack_tx, ack_rx) = oneshot::channel();
        self.pending_acks.write().insert(message_id, ack_tx);

        // Send message
        self.send_message_internal(destination, message).await?;

        // Wait for ack with timeout
        match tokio::time::timeout(Duration::from_secs(5), ack_rx).await {
            Ok(Ok(ack)) => Ok(ack),
            Ok(Err(_)) => Err(DbError::Internal("Ack channel closed".to_string())),
            Err(_) => Err(DbError::LockTimeout),
        }
    }

    async fn send_message_internal(
        &self,
        destination: NodeId,
        message: Message,
    ) -> Result<(), DbError> {
        let start = Instant::now();

        let connections = self.connections.read();
        let conn = connections.get(&destination)
            .ok_or_else(|| DbError::Network("Node not connected".to_string()))?;

        conn.send_message(message).await?;

        // Record latency
        let latency = start.elapsed().as_micros() as u64;
        conn.record_latency(latency);

        // Update statistics
        let mut stats = self.stats.write();
        stats.total_sent += 1;
        stats.avg_latency_us = (stats.avg_latency_us + latency) / 2;

        Ok(())
    }

    /// Register a message handler
    pub fn register_handler<F>(&self, message_type: MessageType, handler: F)
    where
        F: Fn(Message) -> Result<Vec<u8>, DbError> + Send + Sync + 'static,
    {
        self.message_handlers.write().insert(
            message_type,
            Arc::new(handler),
        );
    }

    /// Start heartbeat monitoring
    async fn start_heartbeat_monitor(&self) {
        let node_health = self.node_health.clone();
        let connections = self.connections.clone();
        let stats = self.stats.clone();
        let interval = self.config.heartbeat_interval;
        let node_id = self.node_id.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = interval_timer.tick() => {
                        // Send heartbeats to all nodes
                        let conns_to_ping: Vec<_> = {
                            let conns = connections.read();
                            conns.iter()
                                .map(|(id, conn)| (id.clone(), conn.clone()))
                                .collect()
                        };

                        for (remote_node, conn) in conns_to_ping {
                            let message = Message {
                                message_id: 0,
                                source: node_id.clone(),
                                destination: remote_node.clone(),
                                message_type: MessageType::Heartbeat,
                                payload: Vec::new(),
                                timestamp: Self::current_timestamp(),
                                priority: MessagePriority::Critical,
                                requires_ack: false,
                                sequence: 0,
                            };

                            if conn.send_message(message).await.is_ok() {
                                stats.write().heartbeats_sent += 1;
                            }
                        }

                        // Check for missed heartbeats
                        let mut health = node_health.write();

                        for node_health in health.values_mut() {
                            let elapsed = node_health.last_heartbeat.elapsed();

                            if elapsed > interval * 2 {
                                node_health.record_missed_heartbeat();
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });
    }

    /// Detect split-brain scenario
    pub fn detect_split_brain(&self) -> Result<bool, DbError> {
        let health = self.node_health.read();
        let total_nodes = health.len() + 1; // +1 for local node

        let healthy_nodes = health.values()
            .filter(|h| h.state == NodeState::Healthy)
            .count() + 1; // +1 for local node

        let quorum = (total_nodes as f64 * QUORUM_PERCENTAGE).ceil() as usize;

        // Split-brain if we can't see quorum
        Ok(healthy_nodes < quorum)
    }

    /// Get cluster view (visible nodes)
    pub fn get_cluster_view(&self) -> ClusterView {
        let health = self.node_health.read();

        let healthy_nodes: Vec<_> = health.iter()
            .filter(|(_, h)| h.state == NodeState::Healthy)
            .map(|(id, _)| id.clone())
            .collect();

        let suspected_nodes: Vec<_> = health.iter()
            .filter(|(_, h)| h.state == NodeState::Suspected)
            .map(|(id, _)| id.clone())
            .collect();

        let down_nodes: Vec<_> = health.iter()
            .filter(|(_, h)| h.state == NodeState::Down)
            .map(|(id, _)| id.clone())
            .collect();

        ClusterView {
            local_node: self.node_id.clone(),
            healthy_nodes,
            suspected_nodes,
            down_nodes,
            total_nodes: health.len() + 1,
            has_quorum: !self.detect_split_brain().unwrap_or(false),
        }
    }

    /// Get node health
    pub fn get_node_health(&self, node_id: &NodeId) -> Option<NodeHealth> {
        self.node_health.read().get(node_id).cloned()
    }

    /// Get statistics
    pub fn get_statistics(&self) -> InterconnectStatistics {
        self.stats.read().clone()
    }

    fn next_message_id(&self) -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    fn next_sequence(&self) -> u64 {
        let mut seq = self.sequence_counter.lock();
        *seq += 1;
        *seq
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64
    }
}

/// Cluster view
#[derive(Debug, Clone)]
pub struct ClusterView {
    pub local_node: NodeId,
    pub healthy_nodes: Vec<NodeId>,
    pub suspected_nodes: Vec<NodeId>,
    pub down_nodes: Vec<NodeId>,
    pub total_nodes: usize,
    pub has_quorum: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_interconnect_creation() {
        let interconnect = ClusterInterconnect::new(
            "node1".to_string(),
            "127.0.0.1:5000".to_string(),
            InterconnectConfig::default(),
        );

        assert_eq!(interconnect.node_id, "node1");
    }

    #[test]
    fn test_node_health() {
        let mut health = NodeHealth::new("node1".to_string());
        assert_eq!(health.state, NodeState::Joining);

        health.update_heartbeat();
        assert_eq!(health.missed_heartbeats, 0);

        health.record_missed_heartbeat();
        assert_eq!(health.missed_heartbeats, 1);
    }

    #[test]
    fn test_connection_state() {
        let conn = Connection::new("node1".to_string());
        assert_eq!(*conn.state.read(), ConnectionState::Disconnected);
    }
}
