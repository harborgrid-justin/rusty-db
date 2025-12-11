//! Serf-Compatible Protocol for RustyDB
//!
//! Implements a protocol compatible with HashiCorp Serf for cluster membership and event propagation.
//! Serf uses a gossip protocol based on SWIM for membership, with additional features for
//! custom events and queries.
//!
//! # Features
//!
//! - SWIM-based membership gossip
//! - Custom event propagation
//! - Query/response mechanism
//! - Tag-based node filtering
//! - User events (fire custom events to cluster)
//!
//! # References
//!
//! - [Serf Documentation](https://www.serf.io/)

use super::{DiscoveryConfig, DiscoveryEvent, DiscoveryProtocol, NodeInfo, NodeStatus};
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;

/// Serf message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SerfMessage {
    /// Join request
    Join {
        node: NodeInfo,
        #[allow(dead_code)] // Reserved for node tags
    tags: HashMap<String, String>,
    },

    /// Leave notification
    Leave {
        node: NodeInfo,
    },

    /// Membership update
    MemberUpdate {
        node: NodeInfo,
        status: NodeStatus,
        incarnation: u64,
    },

    /// Ping for failure detection
    Ping {
        from: NodeInfo,
        sequence: u64,
    },

    /// Ack for ping
    Ack {
        from: NodeInfo,
        sequence: u64,
    },

    /// Custom user event
    UserEvent {
        name: String,
        payload: Vec<u8>,
        coalesce: bool,
    },

    /// Query request
    Query {
        id: String,
        name: String,
        payload: Vec<u8>,
        filter_tags: HashMap<String, String>,
    },

    /// Query response
    QueryResponse {
        id: String,
        payload: Vec<u8>,
    },
}

/// Serf member state
#[derive(Debug, Clone)]
struct SerfMember {
    info: NodeInfo,
    status: NodeStatus,
    incarnation: u64,
    #[allow(dead_code)] // Reserved for node tags
    tags: HashMap<String, String>,
    last_seen: Instant,
}

/// Event handler for user events
pub type EventHandler = Arc<dyn Fn(&str, &[u8]) + Send + Sync>;

/// Query handler for queries
pub type QueryHandler = Arc<dyn Fn(&str, &[u8]) -> Vec<u8> + Send + Sync>;

/// Serf protocol implementation
pub struct SerfProtocol {
    config: DiscoveryConfig,
    socket: Arc<UdpSocket>,
    members: Arc<RwLock<HashMap<String, SerfMember>>>,
    event_tx: mpsc::Sender<DiscoveryEvent>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    sequence: Arc<RwLock<u64>>,
    tags: Arc<RwLock<HashMap<String, String>>>,
    event_handlers: Arc<RwLock<Vec<EventHandler>>>,
    query_handlers: Arc<RwLock<HashMap<String, QueryHandler>>>,
}

impl SerfProtocol {
    /// Create a new Serf protocol instance
    pub fn new(
        config: DiscoveryConfig,
        event_tx: mpsc::Sender<DiscoveryEvent>,
    ) -> Result<Self> {
        // Create UDP socket
        let socket = std::net::UdpSocket::bind(config.bind_addr)
            .map_err(|e| DbError::Network(format!("Failed to bind socket: {}", e)))?;

        socket.set_nonblocking(true)
            .map_err(|e| DbError::Network(format!("Failed to set nonblocking: {}", e)))?;

        let socket = Arc::new(UdpSocket::from_std(socket)
            .map_err(|e| DbError::Network(format!("Failed to create tokio socket: {}", e)))?);

        // Extract tags from node metadata
        let tags = Arc::new(RwLock::new(config.local_node.metadata.clone()));

        Ok(Self {
            config,
            socket,
            members: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            shutdown_tx: None,
            sequence: Arc::new(RwLock::new(0)),
            tags,
            event_handlers: Arc::new(RwLock::new(Vec::new())),
            query_handlers: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Register a handler for user events
    pub async fn register_event_handler(&self, handler: EventHandler) {
        let mut handlers = self.event_handlers.write().await;
        handlers.push(handler);
    }

    /// Register a handler for queries
    pub async fn register_query_handler(&self, name: String, handler: QueryHandler) {
        let mut handlers = self.query_handlers.write().await;
        handlers.insert(name, handler);
    }

    /// Fire a user event to the cluster
    pub async fn fire_event(&self, name: String, payload: Vec<u8>, coalesce: bool) -> Result<()> {
        let msg = SerfMessage::UserEvent {
            name,
            payload,
            coalesce,
        };

        self.broadcast_message(&msg).await
    }

    /// Send a query to the cluster
    pub async fn send_query(
        &self,
        name: String,
        payload: Vec<u8>,
        filter_tags: HashMap<String, String>,
    ) -> Result<String> {
        let query_id = uuid::Uuid::new_v4().to_string();

        let msg = SerfMessage::Query {
            id: query_id.clone(),
            name,
            payload,
            filter_tags,
        };

        self.broadcast_message(&msg).await?;

        Ok(query_id)
    }

    /// Get next sequence number
    async fn next_sequence(&self) -> u64 {
        let mut seq = self.sequence.write().await;
        *seq += 1;
        *seq
    }

    /// Send message to a specific node
    async fn send_message(&self, msg: &SerfMessage, addr: SocketAddr) -> Result<()> {
        let data = bincode::serialize(msg)
            .map_err(|e| DbError::Serialization(format!("Failed to serialize: {}", e)))?;

        self.socket.send_to(&data, addr).await
            .map_err(|e| DbError::Network(format!("Failed to send message: {}", e)))?;

        Ok(())
    }

    /// Broadcast message to all members
    async fn broadcast_message(&self, msg: &SerfMessage) -> Result<()> {
        let members = self.members.read().await;

        for member in members.values() {
            if let Err(e) = self.send_message(msg, member.info.addr).await {
                eprintln!("Error sending to {}: {}", member.info.addr, e);
            }
        }

        Ok(())
    }

    /// Handle incoming message
    async fn handle_message(&self, msg: SerfMessage, from: SocketAddr) -> Result<()> {
        match msg {
            SerfMessage::Join { node, tags } => {
                self.handle_join(node, tags).await?;
            }

            SerfMessage::Leave { node } => {
                self.handle_leave(node).await?;
            }

            SerfMessage::MemberUpdate { node, status, incarnation } => {
                self.handle_member_update(node, status, incarnation).await?;
            }

            SerfMessage::Ping { from: _sender, sequence } => {
                // Send ack
                let ack = SerfMessage::Ack {
                    from: self.config.local_node.clone(),
                    sequence,
                };
                self.send_message(&ack, from).await?;
            }

            SerfMessage::Ack { from: sender, sequence: _ } => {
                // Update member as alive
                let mut members = self.members.write().await;
                if let Some(member) = members.get_mut(&sender.id) {
                    member.status = NodeStatus::Alive;
                    member.last_seen = Instant::now();
                }
            }

            SerfMessage::UserEvent { name, payload, coalesce: _ } => {
                self.handle_user_event(&name, &payload).await?;
            }

            SerfMessage::Query { id, name, payload, filter_tags } => {
                self.handle_query(id, name, payload, filter_tags, from).await?;
            }

            SerfMessage::QueryResponse { id: _, payload: _ } => {
                // Handle query response (store for caller to retrieve)
                // Not implemented in this basic version
            }
        }

        Ok(())
    }

    /// Handle join message
    async fn handle_join(&self, node: NodeInfo, tags: HashMap<String, String>) -> Result<()> {
        if node.id == self.config.local_node.id {
            return Ok(());
        }

        let mut members = self.members.write().await;
        let is_new = !members.contains_key(&node.id);

        members.insert(
            node.id.clone(),
            SerfMember {
                info: node.clone(),
                status: NodeStatus::Alive,
                incarnation: 0,
                tags,
                last_seen: Instant::now(),
            },
        );
        drop(members);

        if is_new {
            let _ = self.event_tx.send(DiscoveryEvent::NodeJoined(node)).await;
        }

        Ok(())
    }

    /// Handle leave message
    async fn handle_leave(&self, node: NodeInfo) -> Result<()> {
        let mut members = self.members.write().await;
        members.remove(&node.id);
        drop(members);

        let _ = self.event_tx.send(DiscoveryEvent::NodeLeft(node)).await;

        Ok(())
    }

    /// Handle member update
    async fn handle_member_update(
        &self,
        node: NodeInfo,
        status: NodeStatus,
        incarnation: u64,
    ) -> Result<()> {
        let mut members = self.members.write().await;

        if let Some(member) = members.get_mut(&node.id) {
            if incarnation >= member.incarnation {
                let old_status = member.status;
                member.status = status;
                member.incarnation = incarnation;
                member.last_seen = Instant::now();

                // Send appropriate event
                if old_status != status {
                    drop(members);
                    match status {
                        NodeStatus::Alive => {
                            let _ = self.event_tx.send(
                                DiscoveryEvent::NodeRecovered(node)
                            ).await;
                        }
                        NodeStatus::Dead => {
                            let _ = self.event_tx.send(
                                DiscoveryEvent::NodeFailed(node)
                            ).await;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle user event
    async fn handle_user_event(&self, name: &str, payload: &[u8]) -> Result<()> {
        let handlers = self.event_handlers.read().await;

        for handler in handlers.iter() {
            handler(name, payload);
        }

        Ok(())
    }

    /// Handle query
    async fn handle_query(
        &self,
        id: String,
        name: String,
        payload: Vec<u8>,
        filter_tags: HashMap<String, String>,
        from: SocketAddr,
    ) -> Result<()> {
        // Check if this node matches the filter tags
        let tags = self.tags.read().await;
        let matches = filter_tags.iter().all(|(k, v)| {
            tags.get(k).map(|val| val == v).unwrap_or(false)
        });
        drop(tags);

        if !matches {
            return Ok(());
        }

        // Execute query handler if registered
        let handlers = self.query_handlers.read().await;
        if let Some(handler) = handlers.get(&name) {
            let response_payload = handler(&name, &payload);
            drop(handlers);

            let response = SerfMessage::QueryResponse {
                id,
                payload: response_payload,
            };

            self.send_message(&response, from).await?;
        }

        Ok(())
    }

    /// Send periodic pings
    async fn send_ping(&self) -> Result<()> {
        let members = self.members.read().await;

        if members.is_empty() {
            return Ok(());
        }

        // Select random member to ping
        let member = members.values()
            .filter(|m| m.status == NodeStatus::Alive)
            .nth(rand::random::<usize>() % members.len().max(1))
            .cloned();

        drop(members);

        if let Some(member) = member {
            let sequence = self.next_sequence().await;
            let msg = SerfMessage::Ping {
                from: self.config.local_node.clone(),
                sequence,
            };

            self.send_message(&msg, member.info.addr).await?;
        }

        Ok(())
    }

    /// Run the Serf protocol loop
    async fn run_protocol(&self, mut shutdown_rx: mpsc::Receiver<()>) -> Result<()> {
        let mut ping_interval = interval(self.config.gossip_interval);
        let mut buffer = vec![0u8; 65536];

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    break;
                }

                _ = ping_interval.tick() => {
                    if let Err(e) = self.send_ping().await {
                        eprintln!("Error sending ping: {}", e);
                    }
                }

                result = self.socket.recv_from(&mut buffer) => {
                    match result {
                        Ok((len, addr)) => {
                            if let Ok(msg) = bincode::deserialize::<SerfMessage>(&buffer[..len]) {
                                if let Err(e) = self.handle_message(msg, addr).await {
                                    eprintln!("Error handling message: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error receiving message: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Clone for task spawning
    fn clone_for_task(&self) -> Self {
        Self {
            config: self.config.clone(),
            socket: self.socket.clone(),
            members: self.members.clone(),
            event_tx: self.event_tx.clone(),
            shutdown_tx: None,
            sequence: self.sequence.clone(),
            tags: self.tags.clone(),
            event_handlers: self.event_handlers.clone(),
            query_handlers: self.query_handlers.clone(),
        }
    }
}

#[async_trait::async_trait]
impl DiscoveryProtocol for SerfProtocol {
    async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let protocol = self.clone_for_task();
        tokio::spawn(async move {
            if let Err(e) = protocol.run_protocol(shutdown_rx).await {
                eprintln!("Serf protocol error: {}", e);
            }
        });

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        Ok(())
    }

    async fn members(&self) -> Result<Vec<NodeInfo>> {
        let members = self.members.read().await;
        Ok(members.values()
            .filter(|m| m.status == NodeStatus::Alive)
            .map(|m| m.info.clone())
            .collect())
    }

    async fn announce(&self) -> Result<()> {
        // Announcement is done via join
        Ok(())
    }

    async fn join(&mut self, seeds: Vec<SocketAddr>) -> Result<()> {
        let tags = self.tags.read().await.clone();

        let msg = SerfMessage::Join {
            node: self.config.local_node.clone(),
            tags,
        };

        for seed in seeds {
            self.send_message(&msg, seed).await?;
        }

        Ok(())
    }

    async fn leave(&mut self) -> Result<()> {
        let msg = SerfMessage::Leave {
            node: self.config.local_node.clone(),
        };

        self.broadcast_message(&msg).await
    }

    fn subscribe(&self) -> mpsc::Receiver<DiscoveryEvent> {
        let (_tx, rx) = mpsc::channel(1000);
        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serf_message_serialization() {
        let node = NodeInfo::new(
            "test-node".to_string(),
            "127.0.0.1:7946".parse().unwrap(),
        );

        let msg = SerfMessage::Join {
            node,
            tags: HashMap::new(),
        };

        let bytes = bincode::serialize(&msg).unwrap();
        let deserialized: SerfMessage = bincode::deserialize(&bytes).unwrap();

        match deserialized {
            SerfMessage::Join { node, .. } => {
                assert_eq!(node.id, "test-node");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[tokio::test]
    async fn test_serf_sequence() {
        let config = DiscoveryConfig::default();
        let (tx, _rx) = mpsc::channel(100);

        let protocol = SerfProtocol::new(config, tx).unwrap();

        let seq1 = protocol.next_sequence().await;
        let seq2 = protocol.next_sequence().await;

        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);
    }
}
