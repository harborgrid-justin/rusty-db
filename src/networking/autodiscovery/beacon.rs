//! Beacon Protocol for RustyDB Discovery
//!
//! Implements a heartbeat-based presence announcement protocol.
//! Nodes periodically send beacons containing their information and metadata.
//!
//! # Features
//!
//! - Periodic heartbeat beacons
//! - Metadata propagation
//! - Leader hints (for leader election integration)
//! - Failure detection via missed beacons
//! - Configurable beacon intervals

use super::{DiscoveryConfig, DiscoveryEvent, DiscoveryProtocol, NodeInfo, NodeStatus};
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;

/// Beacon message containing node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Beacon {
    /// Node information
    pub node: NodeInfo,

    /// Sequence number (monotonically increasing)
    pub sequence: u64,

    /// Timestamp when beacon was sent
    pub timestamp: u64,

    /// Leader hint (node ID that this node thinks is leader)
    pub leader_hint: Option<String>,

    /// Cluster size hint
    pub cluster_size: usize,

    /// Node uptime in seconds
    pub uptime: u64,
}

/// Beacon state for a discovered node
#[derive(Debug, Clone)]
struct BeaconState {
    /// Node information
    info: NodeInfo,

    /// Last received sequence number
    last_sequence: u64,

    /// Last time beacon was received
    last_beacon: Instant,

    /// Number of consecutive missed beacons
    missed_beacons: u32,

    /// Leader hint from this node
    leader_hint: Option<String>,
}

/// Beacon protocol implementation
pub struct BeaconProtocol {
    config: DiscoveryConfig,
    socket: Arc<UdpSocket>,
    states: Arc<RwLock<HashMap<String, BeaconState>>>,
    event_tx: mpsc::Sender<DiscoveryEvent>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    sequence: Arc<RwLock<u64>>,
    start_time: Instant,
}

impl BeaconProtocol {
    /// Create a new beacon protocol instance
    pub fn new(
        config: DiscoveryConfig,
        event_tx: mpsc::Sender<DiscoveryEvent>,
    ) -> Result<Self> {
        // Create UDP socket
        let socket = std::net::UdpSocket::bind(config.bind_addr)
            .map_err(|e| DbError::Network(format!("Failed to bind socket: {}", e)))?;

        socket.set_nonblocking(true)
            .map_err(|e| DbError::Network(format!("Failed to set nonblocking: {}", e)))?;

        // Enable broadcast for subnet-wide beacons
        socket.set_broadcast(true)
            .map_err(|e| DbError::Network(format!("Failed to enable broadcast: {}", e)))?;

        let socket = Arc::new(UdpSocket::from_std(socket)
            .map_err(|e| DbError::Network(format!("Failed to create tokio socket: {}", e)))?);

        Ok(Self {
            config,
            socket,
            states: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            shutdown_tx: None,
            sequence: Arc::new(RwLock::new(0)),
            start_time: Instant::now(),
        })
    }

    /// Get next sequence number
    async fn next_sequence(&self) -> u64 {
        let mut seq = self.sequence.write().await;
        *seq += 1;
        *seq
    }

    /// Send beacon to all known nodes and broadcast address
    async fn send_beacon(&self) -> Result<()> {
        let sequence = self.next_sequence().await;
        let uptime = self.start_time.elapsed().as_secs();

        let states = self.states.read().await;
        let cluster_size = states.len() + 1; // +1 for self

        // Determine leader hint (could integrate with leader election)
        let leader_hint = None; // Placeholder

        drop(states);

        let beacon = Beacon {
            node: self.config.local_node.clone(),
            sequence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| DbError::Internal(format!("Time error: {}", e)))?
                .as_secs(),
            leader_hint,
            cluster_size,
            uptime,
        };

        let data = bincode::serialize(&beacon)
            .map_err(|e| DbError::Serialization(format!("Failed to serialize beacon: {}", e)))?;

        // Send to all known nodes
        let states = self.states.read().await;
        for state in states.values() {
            if let Err(e) = self.socket.send_to(&data, state.info.addr).await {
                eprintln!("Error sending beacon to {}: {}", state.info.addr, e);
            }
        }
        drop(states);

        // Also send to seed nodes
        for seed in &self.config.seed_nodes {
            if let Err(e) = self.socket.send_to(&data, seed).await {
                eprintln!("Error sending beacon to seed {}: {}", seed, e);
            }
        }

        Ok(())
    }

    /// Handle received beacon
    async fn handle_beacon(&self, beacon: Beacon, _from: SocketAddr) -> Result<()> {
        // Ignore our own beacons
        if beacon.node.id == self.config.local_node.id {
            return Ok(());
        }

        let mut states = self.states.write().await;

        if let Some(state) = states.get_mut(&beacon.node.id) {
            // Update existing state
            if beacon.sequence > state.last_sequence {
                state.last_sequence = beacon.sequence;
                state.last_beacon = Instant::now();
                state.missed_beacons = 0;
                state.leader_hint = beacon.leader_hint.clone();

                // Check if node recovered from failure
                if state.info.status != NodeStatus::Alive {
                    state.info.status = NodeStatus::Alive;
                    let _ = self.event_tx.send(
                        DiscoveryEvent::NodeRecovered(state.info.clone())
                    ).await;
                }
            }
        } else {
            // New node discovered
            states.insert(
                beacon.node.id.clone(),
                BeaconState {
                    info: beacon.node.clone(),
                    last_sequence: beacon.sequence,
                    last_beacon: Instant::now(),
                    missed_beacons: 0,
                    leader_hint: beacon.leader_hint.clone(),
                },
            );

            drop(states);

            let _ = self.event_tx.send(DiscoveryEvent::NodeJoined(beacon.node)).await;
        }

        Ok(())
    }

    /// Check for failed nodes (missed beacons)
    async fn check_failures(&self) -> Result<()> {
        let now = Instant::now();
        let beacon_interval = self.config.gossip_interval;
        let max_missed = 3; // Consider failed after 3 missed beacons

        let mut states = self.states.write().await;
        let mut failed = Vec::new();

        for (_id, state) in states.iter_mut() {
            let elapsed = now.duration_since(state.last_beacon);
            let expected_beacons = (elapsed.as_secs() / beacon_interval.as_secs()) as u32;

            if expected_beacons > 0 {
                state.missed_beacons = expected_beacons;

                if state.missed_beacons >= max_missed && state.info.status == NodeStatus::Alive {
                    state.info.status = NodeStatus::Dead;
                    failed.push(state.info.clone());
                }
            }
        }

        drop(states);

        // Send failure events
        for node in failed {
            let _ = self.event_tx.send(DiscoveryEvent::NodeFailed(node)).await;
        }

        Ok(())
    }

    /// Remove dead nodes after extended timeout
    async fn cleanup_dead_nodes(&self) -> Result<()> {
        let now = Instant::now();
        let cleanup_timeout = self.config.failure_timeout * 10; // 10x failure timeout

        let mut states = self.states.write().await;

        states.retain(|_, state| {
            if state.info.status == NodeStatus::Dead {
                now.duration_since(state.last_beacon) < cleanup_timeout
            } else {
                true
            }
        });

        Ok(())
    }

    /// Run the beacon protocol loop
    async fn run_protocol(&self, mut shutdown_rx: mpsc::Receiver<()>) -> Result<()> {
        let mut beacon_interval = interval(self.config.gossip_interval);
        let mut check_interval = interval(self.config.gossip_interval);
        let mut cleanup_interval = interval(Duration::from_secs(60));
        let mut buffer = vec![0u8; 65536];

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    break;
                }

                _ = beacon_interval.tick() => {
                    if let Err(e) = self.send_beacon().await {
                        eprintln!("Error sending beacon: {}", e);
                    }
                }

                _ = check_interval.tick() => {
                    if let Err(e) = self.check_failures().await {
                        eprintln!("Error checking failures: {}", e);
                    }
                }

                _ = cleanup_interval.tick() => {
                    if let Err(e) = self.cleanup_dead_nodes().await {
                        eprintln!("Error cleaning up dead nodes: {}", e);
                    }
                }

                result = self.socket.recv_from(&mut buffer) => {
                    match result {
                        Ok((len, addr)) => {
                            if let Ok(beacon) = bincode::deserialize::<Beacon>(&buffer[..len]) {
                                if let Err(e) = self.handle_beacon(beacon, addr).await {
                                    eprintln!("Error handling beacon: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error receiving beacon: {}", e);
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
            states: self.states.clone(),
            event_tx: self.event_tx.clone(),
            shutdown_tx: None,
            sequence: self.sequence.clone(),
            start_time: self.start_time,
        }
    }
}

#[async_trait::async_trait]
impl DiscoveryProtocol for BeaconProtocol {
    async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let protocol = self.clone_for_task();
        tokio::spawn(async move {
            if let Err(e) = protocol.run_protocol(shutdown_rx).await {
                eprintln!("Beacon protocol error: {}", e);
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
        let states = self.states.read().await;
        Ok(states.values()
            .filter(|s| s.info.status == NodeStatus::Alive)
            .map(|s| s.info.clone())
            .collect())
    }

    async fn announce(&self) -> Result<()> {
        self.send_beacon().await
    }

    async fn join(&mut self, _seeds: Vec<SocketAddr>) -> Result<()> {
        // Seeds are stored in config, beacons will be sent automatically
        self.send_beacon().await
    }

    async fn leave(&mut self) -> Result<()> {
        // Send final beacon with leaving status
        // In a more complete implementation, we'd set a special status
        Ok(())
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
    fn test_beacon_serialization() {
        let node = NodeInfo::new(
            "test-node".to_string(),
            "127.0.0.1:7946".parse().unwrap(),
        );

        let beacon = Beacon {
            node,
            sequence: 42,
            timestamp: 1234567890,
            leader_hint: Some("leader-node".to_string()),
            cluster_size: 5,
            uptime: 3600,
        };

        let bytes = bincode::serialize(&beacon).unwrap();
        let deserialized: Beacon = bincode::deserialize(&bytes).unwrap();

        assert_eq!(deserialized.sequence, 42);
        assert_eq!(deserialized.cluster_size, 5);
        assert_eq!(deserialized.leader_hint, Some("leader-node".to_string()));
    }

    #[tokio::test]
    async fn test_beacon_sequence() {
        let config = DiscoveryConfig::default();
        let (tx, _rx) = mpsc::channel(100);

        let protocol = BeaconProtocol::new(config, tx).unwrap();

        let seq1 = protocol.next_sequence().await;
        let seq2 = protocol.next_sequence().await;
        let seq3 = protocol.next_sequence().await;

        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);
        assert_eq!(seq3, 3);
    }
}
