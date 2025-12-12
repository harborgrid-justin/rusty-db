// Gossip-based Discovery using SWIM Protocol
//
// Implements the Scalable Weakly-consistent Infection-style Process Group Membership
// (SWIM) protocol for failure detection and membership management.
//
// # Protocol Overview
//
// SWIM uses three main mechanisms:
// - **Ping**: Direct health check to a random member
// - **Ping-Req**: Indirect probe via other members if direct ping fails
// - **Gossip**: Piggyback membership updates on ping messages
//
// # Failure Detection
//
// Nodes can be in three states:
// - Alive: Node is healthy
// - Suspect: Node may have failed (grace period)
// - Dead: Node confirmed failed
//
// # References
//
// - [SWIM Paper](https://www.cs.cornell.edu/projects/Quicksilver/public_pdfs/SWIM.pdf)

use super::{DiscoveryConfig, DiscoveryEvent, DiscoveryProtocol, NodeInfo};
use crate::common::NodeId;
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;

/// Member state in the gossip protocol
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum MemberState {
    /// Node is alive and responding
    Alive {
        /// Incarnation number for conflict resolution
        incarnation: u64,
    },

    /// Node is suspected to be dead
    Suspect {
        /// Incarnation number
        incarnation: u64,

        /// Nodes that confirmed the suspicion
        confirming: Vec<NodeId>,

        /// When the suspicion started
        #[serde(skip)]
        suspected_at: Option<Instant>,
    },

    /// Node is confirmed dead
    Dead {
        /// Incarnation number
        incarnation: u64,
    },
}

impl MemberState {
    /// Get the incarnation number
    pub fn incarnation(&self) -> u64 {
        match self {
            MemberState::Alive { incarnation } => *incarnation,
            MemberState::Suspect { incarnation, .. } => *incarnation,
            MemberState::Dead { incarnation } => *incarnation,
        }
    }

    /// Check if the state is alive
    pub fn is_alive(&self) -> bool {
        matches!(self, MemberState::Alive { .. })
    }
}

/// Gossip message types
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum GossipMessage {
    /// Direct ping to check if node is alive
    Ping {
        from: NodeInfo,
        sequence: u64,
    },

    /// Acknowledgment of ping
    Ack {
        from: NodeInfo,
        sequence: u64,
    },

    /// Request for indirect ping (when direct fails)
    PingReq {
        from: NodeInfo,
        target: SocketAddr,
        sequence: u64,
    },

    /// Membership update (piggybacked on other messages)
    Update {
        node: NodeInfo,
        state: MemberState,
    },

    /// Join request
    Join {
        node: NodeInfo,
    },

    /// Leave notification
    Leave {
        node: NodeInfo,
    },
}

/// Gossip protocol state
struct GossipState {
    /// All known members
    members: HashMap<NodeId, (NodeInfo, MemberState)>,

    /// Incarnation number for this node
    #[allow(dead_code)] // Reserved for incarnation number tracking
    local_incarnation: u64,

    /// Pending acks for pings
    pending_acks: HashMap<u64, Instant>,

    /// Sequence number for messages
    sequence: u64,
}

impl GossipState {
    fn new() -> Self {
        Self {
            members: HashMap::new(),
            local_incarnation: 0,
            pending_acks: HashMap::new(),
            sequence: 0,
        }
    }

    fn next_sequence(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }
}

/// SWIM-based gossip discovery
pub struct GossipDiscovery {
    config: DiscoveryConfig,
    socket: Arc<UdpSocket>,
    state: Arc<RwLock<GossipState>>,
    event_tx: mpsc::Sender<DiscoveryEvent>,
    membership: Arc<RwLock<super::MembershipList>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl GossipDiscovery {
    /// Create a new gossip discovery instance
    pub fn new(
        config: DiscoveryConfig,
        event_tx: mpsc::Sender<DiscoveryEvent>,
        membership: Arc<RwLock<super::MembershipList>>,
    ) -> Result<Self> {
        let socket = std::net::UdpSocket::bind(config.bind_addr)
            .map_err(|e| DbError::Network(format!("Failed to bind socket: {}", e)))?;
        socket.set_nonblocking(true)
            .map_err(|e| DbError::Network(format!("Failed to set nonblocking: {}", e)))?;

        let socket = Arc::new(UdpSocket::from_std(socket)
            .map_err(|e| DbError::Network(format!("Failed to create tokio socket: {}", e)))?);

        Ok(Self {
            config,
            socket,
            state: Arc::new(RwLock::new(GossipState::new())),
            event_tx,
            membership,
            shutdown_tx: None,
        })
    }

    /// Send a ping to a random member
    async fn send_ping(&self) -> Result<()> {
        // Get target node address under read lock
        let target_addr = {
            let state = self.state.read().await;

            // Select random alive member
            let alive_members: Vec<_> = state.members.iter()
                .filter(|(_, (_, s))| s.is_alive())
                .collect();

            if alive_members.is_empty() {
                return Ok(());
            }

            use rand::Rng;
            let idx = rand::rng().random_range(0..alive_members.len());
            let (_, (node, _)) = alive_members[idx];
            node.addr
        };

        // Get sequence number under write lock
        let sequence = {
            let mut state = self.state.write().await;
            let seq = state.next_sequence();
            state.pending_acks.insert(seq, Instant::now());
            seq
        };

        let msg = GossipMessage::Ping {
            from: self.config.local_node.clone(),
            sequence,
        };

        self.send_message(&msg, target_addr).await?;

        Ok(())
    }

    /// Send a ping-req (indirect probe)
    #[allow(dead_code)] // Reserved for indirect ping-req protocol
    async fn send_ping_req(&self, target: SocketAddr) -> Result<()> {
        // Get member addresses under read lock
        let member_addrs: Vec<SocketAddr> = {
            let state = self.state.read().await;

            // Select random alive members for indirect probe
            state.members.iter()
                .filter(|(_, (n, s))| s.is_alive() && n.addr != target)
                .take(3)
                .map(|(_, (node, _))| node.addr)
                .collect()
        };

        if member_addrs.is_empty() {
            return Ok(());
        }

        // Get sequence number under write lock
        let sequence = {
            let mut state = self.state.write().await;
            state.next_sequence()
        };

        let msg = GossipMessage::PingReq {
            from: self.config.local_node.clone(),
            target,
            sequence,
        };

        for addr in member_addrs {
            self.send_message(&msg, addr).await?;
        }

        Ok(())
    }

    /// Handle incoming message
    async fn handle_message(&self, msg: GossipMessage, from: SocketAddr) -> Result<()> {
        match msg {
            GossipMessage::Ping { from: sender, sequence } => {
                // Send ack
                let ack = GossipMessage::Ack {
                    from: self.config.local_node.clone(),
                    sequence,
                };
                self.send_message(&ack, from).await?;

                // Update membership
                self.update_member(sender, MemberState::Alive { incarnation: 0 }).await?;
            }

            GossipMessage::Ack { from: sender, sequence } => {
                // Remove pending ack
                let mut state = self.state.write().await;
                state.pending_acks.remove(&sequence);
                drop(state);

                // Update member as alive
                self.update_member(sender, MemberState::Alive { incarnation: 0 }).await?;
            }

            GossipMessage::PingReq { from: _sender, target, sequence } => {
                // Perform indirect ping on behalf of requester
                let ping = GossipMessage::Ping {
                    from: self.config.local_node.clone(),
                    sequence,
                };
                self.send_message(&ping, target).await?;
            }

            GossipMessage::Update { node, state } => {
                self.update_member(node, state).await?;
            }

            GossipMessage::Join { node } => {
                self.update_member(node.clone(), MemberState::Alive { incarnation: 0 }).await?;

                let _ = self.event_tx.send(DiscoveryEvent::NodeJoined(node)).await;
            }

            GossipMessage::Leave { node } => {
                let mut state = self.state.write().await;
                state.members.remove(&node.id);
                drop(state);

                let _ = self.event_tx.send(DiscoveryEvent::NodeLeft(node)).await;
            }
        }

        Ok(())
    }

    /// Update member state
    async fn update_member(&self, node: NodeInfo, new_state: MemberState) -> Result<()> {
        let mut state = self.state.write().await;

        let should_update = if let Some((_, old_state)) = state.members.get(&node.id) {
            // Only update if incarnation is higher or state changes
            new_state.incarnation() >= old_state.incarnation()
        } else {
            true
        };

        if should_update {
            state.members.insert(node.id.clone(), (node.clone(), new_state));
            drop(state);

            // Update membership list
            let mut membership = self.membership.write().await;
            membership.add_or_update(node.clone());

            Ok(())
        } else {
            Ok(())
        }
    }

    /// Send a gossip message
    async fn send_message(&self, msg: &GossipMessage, addr: SocketAddr) -> Result<()> {
        let bytes = bincode::encode_to_vec(msg, bincode::config::standard())
            .map_err(|e| DbError::Serialization(format!("Failed to serialize message: {}", e)))?;

        self.socket.send_to(&bytes, addr).await
            .map_err(|e| DbError::Network(format!("Failed to send message: {}", e)))?;

        Ok(())
    }

    /// Check for timed out pings and suspect nodes
    async fn check_failures(&self) -> Result<()> {
        let now = Instant::now();
        let timeout = self.config.failure_timeout;

        let mut state = self.state.write().await;

        // Check pending acks
        let timed_out: Vec<_> = state.pending_acks.iter()
            .filter(|(_, sent_at)| now.duration_since(**sent_at) > timeout)
            .map(|(seq, _)| *seq)
            .collect();

        for seq in timed_out {
            state.pending_acks.remove(&seq);
        }

        // Check suspect members
        let suspected: Vec<_> = state.members.iter()
            .filter_map(|(id, (node, s))| {
                if let MemberState::Suspect { suspected_at: Some(at), .. } = s {
                    if now.duration_since(*at) > timeout * 3 {
                        Some((id.clone(), node.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        for (id, node) in suspected {
            state.members.insert(
                id,
                (node.clone(), MemberState::Dead { incarnation: 0 }),
            );

            let _ = self.event_tx.send(DiscoveryEvent::NodeFailed(node)).await;
        }

        Ok(())
    }

    /// Run the gossip protocol loop
    async fn run_protocol(&self, mut shutdown_rx: mpsc::Receiver<()>) -> Result<()> {
        let mut gossip_interval = interval(self.config.gossip_interval);
        let mut buffer = vec![0u8; 65536];

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    break;
                }

                _ = gossip_interval.tick() => {
                    // Send periodic ping
                    if let Err(e) = self.send_ping().await {
                        eprintln!("Error sending ping: {}", e);
                    }

                    // Check for failures
                    if let Err(e) = self.check_failures().await {
                        eprintln!("Error checking failures: {}", e);
                    }
                }

                result = self.socket.recv_from(&mut buffer) => {
                    match result {
                        Ok((len, addr)) => {
                            if let Ok((msg, _)) = bincode::decode_from_slice(&buffer[..len], bincode::config::standard()) {
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
}

#[async_trait::async_trait]
impl DiscoveryProtocol for GossipDiscovery {
    async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let discovery = self.clone_for_task();
        tokio::spawn(async move {
            if let Err(e) = discovery.run_protocol(shutdown_rx).await {
                eprintln!("Gossip protocol error: {}", e);
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
        let state = self.state.read().await;
        Ok(state.members.values()
            .filter(|(_, s)| s.is_alive())
            .map(|(n, _)| n.clone())
            .collect())
    }

    async fn announce(&self) -> Result<()> {
        // Announcement is done via gossip, nothing specific needed
        Ok(())
    }

    async fn join(&mut self, seeds: Vec<SocketAddr>) -> Result<()> {
        let msg = GossipMessage::Join {
            node: self.config.local_node.clone(),
        };

        for seed in seeds {
            self.send_message(&msg, seed).await?;
        }

        Ok(())
    }

    async fn leave(&mut self) -> Result<()> {
        let msg = GossipMessage::Leave {
            node: self.config.local_node.clone(),
        };

        let members = self.members().await?;
        for member in members {
            if let Err(e) = self.send_message(&msg, member.addr).await {
                eprintln!("Error sending leave message: {}", e);
            }
        }

        Ok(())
    }

    fn subscribe(&self) -> mpsc::Receiver<DiscoveryEvent> {
        let (_tx, rx) = mpsc::channel(1000);
        // In a real implementation, we'd store this and forward events
        rx
    }
}

impl GossipDiscovery {
    /// Clone for task spawning
    fn clone_for_task(&self) -> Self {
        Self {
            config: self.config.clone(),
            socket: self.socket.clone(),
            state: self.state.clone(),
            event_tx: self.event_tx.clone(),
            membership: self.membership.clone(),
            shutdown_tx: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_member_state_incarnation() {
        let alive = MemberState::Alive { incarnation: 5 };
        assert_eq!(alive.incarnation(), 5);
        assert!(alive.is_alive());

        let suspect = MemberState::Suspect {
            incarnation: 10,
            confirming: vec![],
            suspected_at: None,
        };
        assert_eq!(suspect.incarnation(), 10);
        assert!(!suspect.is_alive());
    }

    #[tokio::test]
    async fn test_gossip_state() {
        let mut state = GossipState::new();

        let seq1 = state.next_sequence();
        let seq2 = state.next_sequence();

        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);
    }
}
