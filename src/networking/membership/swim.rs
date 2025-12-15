// SWIM Protocol for Failure Detection
//
// This module implements the SWIM (Scalable Weakly-consistent Infection-style
// Process Group Membership) protocol for efficient failure detection.
//
// Features:
// - Gossip-based membership propagation
// - Direct and indirect probing for failure detection
// - Suspicion mechanism to reduce false positives
// - Efficient scalability to large clusters

#![allow(dead_code)]
//
// Reference: "SWIM: Scalable Weakly-consistent Infection-style Process Group
// Membership Protocol" (Das et al., 2002)

use crate::common::NodeId;
use crate::error::Result;
use crate::networking::membership::{MembershipEvent, NodeInfo, NodeStatus, SwimConfig};
use rand::prelude::IndexedRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, RwLock};
use tokio::time;

/// SWIM message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwimMessage {
    /// Direct ping to check if node is alive
    Ping { from: NodeId, sequence: u64 },

    /// Response to ping
    Ack { from: NodeId, sequence: u64 },

    /// Request to probe a node indirectly
    PingReq {
        from: NodeId,
        target: NodeId,
        sequence: u64,
    },

    /// Membership update via gossip
    Gossip { updates: Vec<MembershipUpdate> },
}

/// Membership update for gossip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipUpdate {
    pub node_id: NodeId,
    pub status: NodeStatus,
    pub incarnation: u64,
    pub timestamp: SystemTime,
}

/// SWIM membership manager
pub struct SwimMembership {
    /// Local node ID
    node_id: NodeId,

    /// Local node information
    local_node: NodeInfo,

    /// All known members
    members: Arc<RwLock<HashMap<NodeId, MemberInfo>>>,

    /// Configuration
    config: SwimConfig,

    /// Protocol period timer
    protocol_period: Duration,

    /// Suspicion timeout multiplier
    suspicion_multiplier: u32,

    /// Number of nodes to probe indirectly
    indirect_probe_size: usize,

    /// Gossip fanout
    gossip_fanout: usize,

    /// Pending acks for ping requests
    pending_acks: Arc<RwLock<HashMap<u64, PendingAck>>>,

    /// Sequence number for messages
    sequence: Arc<RwLock<u64>>,

    /// Event broadcaster
    event_tx: mpsc::Sender<MembershipEvent>,

    /// Shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,
}

/// Member information in SWIM
#[derive(Debug, Clone)]
struct MemberInfo {
    /// Node information
    node: NodeInfo,

    /// Incarnation number (for suspicion refutation)
    incarnation: u64,

    /// When we last heard from this node
    last_seen: SystemTime,

    /// When node transitioned to suspected state
    suspected_at: Option<SystemTime>,
}

impl MemberInfo {
    fn new(node: NodeInfo) -> Self {
        Self {
            node,
            incarnation: 0,
            last_seen: SystemTime::now(),
            suspected_at: None,
        }
    }

    fn update_status(&mut self, status: NodeStatus, incarnation: u64) {
        if incarnation > self.incarnation {
            self.node.status = status;
            self.incarnation = incarnation;
            self.last_seen = SystemTime::now();

            if status == NodeStatus::Suspected {
                self.suspected_at = Some(SystemTime::now());
            } else {
                self.suspected_at = None;
            }
        }
    }
}

/// Pending acknowledgment
struct PendingAck {
    target: NodeId,
    sent_at: SystemTime,
    timeout: Duration,
}

impl SwimMembership {
    /// Create a new SWIM membership manager
    pub fn new(
        node_id: NodeId,
        local_node: NodeInfo,
        config: SwimConfig,
    ) -> (Self, mpsc::Receiver<MembershipEvent>) {
        let (event_tx, event_rx) = mpsc::channel(1000);

        let swim = Self {
            node_id: node_id.clone(),
            local_node,
            members: Arc::new(RwLock::new(HashMap::new())),
            protocol_period: config.protocol_period,
            suspicion_multiplier: config.suspicion_multiplier,
            indirect_probe_size: config.indirect_probe_size,
            gossip_fanout: config.gossip_fanout,
            config,
            pending_acks: Arc::new(RwLock::new(HashMap::new())),
            sequence: Arc::new(RwLock::new(0)),
            event_tx,
            shutdown_tx: None,
        };

        (swim, event_rx)
    }

    /// Start the SWIM protocol
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Protocol period task - main SWIM loop
        let members = self.members.clone();
        let node_id = self.node_id.clone();
        let protocol_period = self.protocol_period;
        let indirect_probe_size = self.indirect_probe_size;
        let gossip_fanout = self.gossip_fanout;
        let pending_acks = self.pending_acks.clone();
        let sequence = self.sequence.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(protocol_period);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = Self::protocol_tick(
                            &node_id,
                            &members,
                            indirect_probe_size,
                            gossip_fanout,
                            &pending_acks,
                            &sequence,
                        ).await {
                            tracing::error!("SWIM protocol period failed: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });

        // Failure detection task
        let members = self.members.clone();
        let suspicion_multiplier = self.suspicion_multiplier;
        let protocol_period = self.protocol_period;
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                if let Err(e) =
                    Self::check_failures(&members, suspicion_multiplier, protocol_period, &event_tx)
                        .await
                {
                    tracing::error!("Failure detection failed: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Stop the SWIM protocol
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(()).await;
        }
        Ok(())
    }

    /// Main protocol period task
    async fn protocol_tick(
        node_id: &NodeId,
        members: &Arc<RwLock<HashMap<NodeId, MemberInfo>>>,
        _indirect_probe_size: usize,
        gossip_fanout: usize,
        pending_acks: &Arc<RwLock<HashMap<u64, PendingAck>>>,
        sequence: &Arc<RwLock<u64>>,
    ) -> Result<()> {
        // Select a random member to ping
        let target = {
            let members_guard = members.read().await;
            let active_members: Vec<NodeId> = members_guard
                .iter()
                .filter(|(id, info)| {
                    id.as_str() != node_id.as_str() && info.node.status != NodeStatus::Failed
                })
                .map(|(id, _)| id.clone())
                .collect();

            if active_members.is_empty() {
                return Ok(());
            }

            active_members.choose(&mut rand::rng()).cloned()
        };

        if let Some(target_id) = target {
            // Send ping
            let seq = {
                let mut seq_guard = sequence.write().await;
                *seq_guard += 1;
                *seq_guard
            };

            // In a real implementation, we would send the actual ping message
            tracing::trace!(
                from = %node_id,
                to = %target_id,
                sequence = seq,
                "Sending ping"
            );

            // Record pending ack
            let mut pending = pending_acks.write().await;
            pending.insert(
                seq,
                PendingAck {
                    target: target_id.clone(),
                    sent_at: SystemTime::now(),
                    timeout: Duration::from_millis(500),
                },
            );
        }

        // Gossip membership updates
        Self::gossip_updates(node_id, members, gossip_fanout).await?;

        Ok(())
    }

    /// Gossip membership updates to random nodes
    async fn gossip_updates(
        node_id: &NodeId,
        members: &Arc<RwLock<HashMap<NodeId, MemberInfo>>>,
        fanout: usize,
    ) -> Result<()> {
        let members_guard = members.read().await;

        // Select random gossip targets
        let targets: Vec<NodeId> = members_guard
            .keys()
            .filter(|id| id.as_str() != node_id.as_str())
            .take(fanout)
            .cloned()
            .collect();

        // Prepare updates
        let updates: Vec<MembershipUpdate> = members_guard
            .iter()
            .map(|(id, info)| MembershipUpdate {
                node_id: id.clone(),
                status: info.node.status,
                incarnation: info.incarnation,
                timestamp: info.last_seen,
            })
            .collect();

        drop(members_guard);

        // Send gossip to targets
        for target in targets {
            tracing::trace!(
                from = %node_id,
                to = %target,
                update_count = updates.len(),
                "Gossiping membership updates"
            );
            // In a real implementation, we would send the actual gossip message
        }

        Ok(())
    }

    /// Check for node failures
    async fn check_failures(
        members: &Arc<RwLock<HashMap<NodeId, MemberInfo>>>,
        suspicion_multiplier: u32,
        protocol_period: Duration,
        event_tx: &mpsc::Sender<MembershipEvent>,
    ) -> Result<()> {
        let mut members_guard = members.write().await;
        let now = SystemTime::now();
        let suspicion_timeout = protocol_period * suspicion_multiplier;

        let mut to_suspect = Vec::new();
        let mut to_fail = Vec::new();

        for (node_id, info) in members_guard.iter() {
            if info.node.status == NodeStatus::Active {
                // Check if we should suspect this node
                if let Ok(elapsed) = now.duration_since(info.last_seen) {
                    if elapsed > protocol_period * 3 {
                        to_suspect.push(node_id.clone());
                    }
                }
            } else if info.node.status == NodeStatus::Suspected {
                // Check if suspected node should be marked as failed
                if let Some(suspected_at) = info.suspected_at {
                    if let Ok(elapsed) = now.duration_since(suspected_at) {
                        if elapsed > suspicion_timeout {
                            to_fail.push(node_id.clone());
                        }
                    }
                }
            }
        }

        // Apply status changes
        for node_id in to_suspect {
            if let Some(info) = members_guard.get_mut(&node_id) {
                info.update_status(NodeStatus::Suspected, info.incarnation + 1);
                let _ = event_tx
                    .send(MembershipEvent::NodeSuspected {
                        node_id: node_id.clone(),
                    })
                    .await;
                tracing::warn!(node_id = %node_id, "Node suspected");
            }
        }

        for node_id in to_fail {
            if let Some(info) = members_guard.get_mut(&node_id) {
                info.update_status(NodeStatus::Failed, info.incarnation + 1);
                let _ = event_tx
                    .send(MembershipEvent::NodeFailed {
                        node_id: node_id.clone(),
                    })
                    .await;
                tracing::error!(node_id = %node_id, "Node failed");
            }
        }

        Ok(())
    }

    /// Handle incoming SWIM message
    pub async fn handle_message(&self, message: SwimMessage) -> Result<Option<SwimMessage>> {
        match message {
            SwimMessage::Ping {
                from: _from,
                sequence: _sequence,
            } => {
                // Respond with ack
                Ok(Some(SwimMessage::Ack {
                    from: self.node_id.clone(),
                    sequence: _sequence,
                }))
            }

            SwimMessage::Ack { from, sequence } => {
                // Remove from pending acks
                let mut pending = self.pending_acks.write().await;
                pending.remove(&sequence);

                // Update last seen time
                let mut members = self.members.write().await;
                if let Some(info) = members.get_mut(&from) {
                    info.last_seen = SystemTime::now();
                    if info.node.status == NodeStatus::Suspected {
                        info.update_status(NodeStatus::Active, info.incarnation + 1);
                        let _ = self
                            .event_tx
                            .send(MembershipEvent::NodeRecovered {
                                node_id: from.clone(),
                            })
                            .await;
                    }
                }

                Ok(None)
            }

            SwimMessage::PingReq {
                from,
                target,
                sequence: _,
            } => {
                // Forward ping to target
                tracing::trace!(
                    relay = %self.node_id,
                    from = %from,
                    target = %target,
                    "Relaying ping request"
                );
                // In a real implementation, we would ping the target
                Ok(None)
            }

            SwimMessage::Gossip { updates } => {
                // Merge gossip updates
                let mut members = self.members.write().await;
                for update in updates {
                    if let Some(info) = members.get_mut(&update.node_id) {
                        if update.incarnation > info.incarnation {
                            info.update_status(update.status, update.incarnation);
                        }
                    }
                }
                Ok(None)
            }
        }
    }

    /// Add a new member to track
    pub async fn add_member(&self, node: NodeInfo) -> Result<()> {
        let mut members = self.members.write().await;
        let node_id = node.id.clone();
        members.insert(node_id.clone(), MemberInfo::new(node));

        let _ = self
            .event_tx
            .send(MembershipEvent::NodeJoined {
                node_id: node_id.clone(),
                node_info: members.get(&node_id).unwrap().node.clone(),
            })
            .await;

        Ok(())
    }

    /// Get all active members
    pub async fn get_active_members(&self) -> Vec<NodeInfo> {
        let members = self.members.read().await;
        members
            .values()
            .filter(|info| info.node.status == NodeStatus::Active)
            .map(|info| info.node.clone())
            .collect()
    }

    /// Check if a node is alive
    pub async fn is_node_alive(&self, node_id: &NodeId) -> bool {
        let members = self.members.read().await;
        members
            .get(node_id)
            .map(|info| info.node.status == NodeStatus::Active)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::membership::NodeMetadata;
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_swim_creation() {
        let node_info = NodeInfo::new(
            "node1".to_string(),
            "127.0.0.1:7000".parse::<SocketAddr>().unwrap(),
            "127.0.0.1:5432".parse::<SocketAddr>().unwrap(),
            NodeMetadata::default(),
        );

        let config = SwimConfig::default();
        let (swim, _rx) = SwimMembership::new("node1".to_string(), node_info, config);

        assert_eq!(swim.node_id, "node1");
    }

    #[tokio::test]
    async fn test_handle_ping() {
        let node_info = NodeInfo::new(
            "node1".to_string(),
            "127.0.0.1:7000".parse::<SocketAddr>().unwrap(),
            "127.0.0.1:5432".parse::<SocketAddr>().unwrap(),
            NodeMetadata::default(),
        );

        let config = SwimConfig::default();
        let (swim, _rx) = SwimMembership::new("node1".to_string(), node_info, config);

        let message = SwimMessage::Ping {
            from: "node2".to_string(),
            sequence: 1,
        };

        let response = swim.handle_message(message).await.unwrap();
        assert!(response.is_some());

        if let Some(SwimMessage::Ack { from, sequence }) = response {
            assert_eq!(from, "node1");
            assert_eq!(sequence, 1);
        }
    }
}
