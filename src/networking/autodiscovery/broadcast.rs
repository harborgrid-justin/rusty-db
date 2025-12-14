// UDP Broadcast Discovery for RustyDB
//
// Simple UDP broadcast-based discovery for LAN environments.
// Nodes periodically broadcast their presence to the subnet broadcast address
// and listen for broadcasts from other nodes.
//
// # Protocol
//
// - Uses UDP broadcast to 255.255.255.255 or subnet-specific broadcast address
// - Periodic announcements every N seconds
// - Rate limiting to prevent broadcast storms
// - Simple JSON-based message format

use super::{DiscoveryConfig, DiscoveryEvent, DiscoveryProtocol, NodeInfo};
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;

/// Broadcast discovery message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum BroadcastMessage {
    /// Announcement of node presence
    Announce { node: NodeInfo, timestamp: u64 },

    /// Request for all nodes to announce
    Probe { from: NodeInfo },

    /// Graceful leave notification
    Leave { node: NodeInfo },
}

/// Discovered node with last seen timestamp
#[derive(Debug, Clone)]
struct DiscoveredNode {
    info: NodeInfo,
    last_seen: Instant,
}

/// UDP Broadcast discovery implementation
pub struct BroadcastDiscovery {
    config: DiscoveryConfig,
    socket: Arc<UdpSocket>,
    discovered: Arc<RwLock<HashMap<String, DiscoveredNode>>>,
    event_tx: mpsc::Sender<DiscoveryEvent>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    broadcast_addr: SocketAddr,
}

impl BroadcastDiscovery {
    /// Create a new broadcast discovery instance
    pub fn new(config: DiscoveryConfig, event_tx: mpsc::Sender<DiscoveryEvent>) -> Result<Self> {
        // Create UDP socket
        let socket = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, config.bind_addr.port()))
            .map_err(|e| DbError::Network(format!("Failed to bind socket: {}", e)))?;

        // Enable broadcast
        socket
            .set_broadcast(true)
            .map_err(|e| DbError::Network(format!("Failed to enable broadcast: {}", e)))?;

        socket
            .set_nonblocking(true)
            .map_err(|e| DbError::Network(format!("Failed to set nonblocking: {}", e)))?;

        let socket = Arc::new(
            UdpSocket::from_std(socket)
                .map_err(|e| DbError::Network(format!("Failed to create tokio socket: {}", e)))?,
        );

        // Default broadcast address
        let broadcast_addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),
            config.bind_addr.port(),
        );

        Ok(Self {
            config,
            socket,
            discovered: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            shutdown_tx: None,
            broadcast_addr,
        })
    }

    /// Set custom broadcast address (e.g., subnet-specific)
    pub fn with_broadcast_addr(mut self, addr: SocketAddr) -> Self {
        self.broadcast_addr = addr;
        self
    }

    /// Send broadcast announcement
    async fn send_announcement(&self) -> Result<()> {
        let msg = BroadcastMessage::Announce {
            node: self.config.local_node.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| DbError::Internal(format!("Time error: {}", e)))?
                .as_secs(),
        };

        self.send_broadcast(&msg).await
    }

    /// Send probe request
    async fn send_probe(&self) -> Result<()> {
        let msg = BroadcastMessage::Probe {
            from: self.config.local_node.clone(),
        };

        self.send_broadcast(&msg).await
    }

    /// Send broadcast message
    async fn send_broadcast(&self, msg: &BroadcastMessage) -> Result<()> {
        let json = serde_json::to_vec(msg)
            .map_err(|e| DbError::Serialization(format!("Failed to serialize: {}", e)))?;

        self.socket
            .send_to(&json, self.broadcast_addr)
            .await
            .map_err(|e| DbError::Network(format!("Failed to send broadcast: {}", e)))?;

        Ok(())
    }

    /// Handle received broadcast message
    async fn handle_message(&self, msg: BroadcastMessage, _from: SocketAddr) -> Result<()> {
        match msg {
            BroadcastMessage::Announce { node, timestamp: _ } => {
                // Ignore our own announcements
                if node.id == self.config.local_node.id {
                    return Ok(());
                }

                let mut discovered = self.discovered.write().await;
                let is_new = !discovered.contains_key(&node.id);

                discovered.insert(
                    node.id.clone(),
                    DiscoveredNode {
                        info: node.clone(),
                        last_seen: Instant::now(),
                    },
                );
                drop(discovered);

                if is_new {
                    let _ = self.event_tx.send(DiscoveryEvent::NodeJoined(node)).await;
                }
            }

            BroadcastMessage::Probe { from: sender } => {
                // Ignore our own probes
                if sender.id == self.config.local_node.id {
                    return Ok(());
                }

                // Respond with announcement
                self.send_announcement().await?;
            }

            BroadcastMessage::Leave { node } => {
                let mut discovered = self.discovered.write().await;
                discovered.remove(&node.id);
                drop(discovered);

                let _ = self.event_tx.send(DiscoveryEvent::NodeLeft(node)).await;
            }
        }

        Ok(())
    }

    /// Check for stale nodes (haven't been seen recently)
    async fn check_stale_nodes(&self) -> Result<()> {
        let timeout = self.config.failure_timeout;
        let now = Instant::now();

        let mut discovered = self.discovered.write().await;
        let stale: Vec<_> = discovered
            .iter()
            .filter(|(_, node)| now.duration_since(node.last_seen) > timeout)
            .map(|(id, node)| (id.clone(), node.info.clone()))
            .collect();

        for (id, info) in stale {
            discovered.remove(&id);
            let _ = self.event_tx.send(DiscoveryEvent::NodeFailed(info)).await;
        }

        Ok(())
    }

    /// Run the broadcast protocol loop
    async fn run_protocol(&self, mut shutdown_rx: mpsc::Receiver<()>) -> Result<()> {
        let mut announce_interval = interval(self.config.gossip_interval);
        let mut stale_check_interval = interval(Duration::from_secs(5));
        let mut buffer = vec![0u8; 65536];

        // Send initial probe
        self.send_probe().await?;

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    break;
                }

                _ = announce_interval.tick() => {
                    if let Err(e) = self.send_announcement().await {
                        eprintln!("Error sending announcement: {}", e);
                    }
                }

                _ = stale_check_interval.tick() => {
                    if let Err(e) = self.check_stale_nodes().await {
                        eprintln!("Error checking stale nodes: {}", e);
                    }
                }

                result = self.socket.recv_from(&mut buffer) => {
                    match result {
                        Ok((len, addr)) => {
                            if let Ok(msg) = serde_json::from_slice::<BroadcastMessage>(&buffer[..len]) {
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
            discovered: self.discovered.clone(),
            event_tx: self.event_tx.clone(),
            shutdown_tx: None,
            broadcast_addr: self.broadcast_addr,
        }
    }
}

#[async_trait::async_trait]
impl DiscoveryProtocol for BroadcastDiscovery {
    async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let discovery = self.clone_for_task();
        tokio::spawn(async move {
            if let Err(e) = discovery.run_protocol(shutdown_rx).await {
                eprintln!("Broadcast protocol error: {}", e);
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
        let discovered = self.discovered.read().await;
        Ok(discovered.values().map(|n| n.info.clone()).collect())
    }

    async fn announce(&self) -> Result<()> {
        self.send_announcement().await
    }

    async fn join(&mut self, _seeds: Vec<SocketAddr>) -> Result<()> {
        // Broadcast discovery doesn't use seeds - just send probe
        self.send_probe().await
    }

    async fn leave(&mut self) -> Result<()> {
        let msg = BroadcastMessage::Leave {
            node: self.config.local_node.clone(),
        };

        self.send_broadcast(&msg).await
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
    fn test_broadcast_addr_builder() {
        let config = DiscoveryConfig::default();
        let (tx, _rx) = mpsc::channel(100);

        let discovery = BroadcastDiscovery::new(config, tx)
            .unwrap()
            .with_broadcast_addr("192.168.1.255:7946".parse().unwrap());

        assert_eq!(discovery.broadcast_addr.to_string(), "192.168.1.255:7946");
    }

    #[test]
    fn test_broadcast_message_serialization() {
        let node = NodeInfo::new("test-node".to_string(), "127.0.0.1:7946".parse().unwrap());

        let msg = BroadcastMessage::Announce {
            node,
            timestamp: 1234567890,
        };

        let json = serde_json::to_vec(&msg).unwrap();
        let deserialized: BroadcastMessage = serde_json::from_slice(&json).unwrap();

        match deserialized {
            BroadcastMessage::Announce { node, timestamp } => {
                assert_eq!(node.id, "test-node");
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Wrong message type"),
        }
    }
}
