// Cluster Membership Management using SWIM Protocol
//
// This module implements the SWIM (Scalable Weakly-consistent Infection-style
// Process Group Membership) protocol for cluster membership and failure detection.
//
// Features:
// - Gossip-based membership propagation
// - Failure detection with configurable timeouts
// - Suspicion mechanism to reduce false positives
// - Protocol buffers-style node metadata
// - Graceful node addition/removal
// - Split-brain prevention with quorum checks
//
// SWIM provides:
// - Scalability to large clusters
// - Low overhead failure detection
// - Eventually consistent membership view

use std::time::SystemTime;
use crate::error::DbError;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration};

// Member identifier
pub type MemberId = String;

// Incarnation number for suspicion mechanism
pub type Incarnation = u64;

// Member state in the cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum MemberState {
    // Member is alive and healthy
    Alive,
    // Member is suspected of failure
    Suspect,
    // Member has failed
    Failed,
    // Member has left gracefully
    Left,
}

// Member information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    // Unique member ID
    pub id: MemberId,
    // Network address
    pub addr: SocketAddr,
    // Current state
    pub state: MemberState,
    // Incarnation number (for suspicion mechanism)
    pub incarnation: Incarnation,
    // Metadata (tags, version, etc.)
    pub metadata: MemberMetadata,
    // State update timestamp
    pub state_changed_at: SystemTime,
    // Last time we heard from this member
    pub last_seen: SystemTime,
}

impl Member {
    pub fn new(id: MemberId, addr: SocketAddr, metadata: MemberMetadata) -> Self {
        Self {
            id,
            addr,
            state: MemberState::Alive,
            incarnation: 0,
            metadata,
            state_changed_at: SystemTime::now(),
            last_seen: SystemTime::now(),
        }
    }

    // Check if member is active (alive or suspect)
    pub fn is_active(&self) -> bool {
        matches!(self.state, MemberState::Alive | MemberState::Suspect)
    }

    // Check if member has failed or left
    pub fn is_down(&self) -> bool {
        matches!(self.state, MemberState::Failed | MemberState::Left)
    }

    // Refute a suspicion by incrementing incarnation
    pub fn refute_suspicion(&mut self) {
        self.incarnation += 1;
        self.state = MemberState::Alive;
        self.state_changed_at = SystemTime::now();
    }
}

// Member metadata (protocol buffers style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberMetadata {
    // Node version
    pub version: String,
    // Datacenter/zone
    pub datacenter: String,
    // Rack identifier
    pub rack: Option<String>,
    // Custom tags
    pub tags: HashMap<String, String>,
    // Capabilities/features
    pub capabilities: Vec<String>,
    // Role (primary, replica, etc.)
    pub role: String,
}

impl Default for MemberMetadata {
    fn default() -> Self {
        Self {
            version: "1.0.0".into(),
            datacenter: "default".into(),
            rack: None,
            tags: HashMap::new(),
            capabilities: Vec::new(),
            role: "node".into(),
        }
    }
}

// SWIM protocol message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwimMessage {
    // Ping message for direct health check
    Ping {
        from: MemberId,
        sequence: u64,
    },
    // Ack response to ping
    Ack {
        from: MemberId,
        sequence: u64,
    },
    // Indirect ping request (ask another node to ping)
    PingReq {
        from: MemberId,
        target: MemberId,
        sequence: u64,
    },
    // Membership update (gossip)
    Gossip {
        from: MemberId,
        updates: Vec<MembershipUpdate>,
    },
    // Join request
    Join {
        member: Member,
    },
    // Leave notification
    Leave {
        member_id: MemberId,
    },
}

// Membership update for gossip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipUpdate {
    pub member_id: MemberId,
    pub addr: SocketAddr,
    pub state: MemberState,
    pub incarnation: Incarnation,
    pub metadata: MemberMetadata,
}

impl MembershipUpdate {
    pub fn from_member(member: &Member) -> Self {
        Self {
            member_id: member.id.clone(),
            addr: member.addr,
            state: member.state,
            incarnation: member.incarnation,
            metadata: member.metadata.clone(),
        }
    }
}

// SWIM configuration
#[derive(Debug, Clone)]
pub struct SwimConfig {
    // Protocol period (time between rounds)
    pub protocol_period: Duration,
    // Number of members to ping-req on timeout
    pub indirect_ping_nodes: usize,
    // Timeout for ping response
    pub ping_timeout: Duration,
    // Timeout for indirect ping response
    pub indirect_ping_timeout: Duration,
    // Number of members to gossip with per round
    pub gossip_fanout: usize,
    // Suspicion timeout before marking as failed
    pub suspicion_timeout: Duration,
    // Number of gossip rounds to propagate a message
    pub gossip_to_the_dead: usize,
    // Enable split-brain prevention
    pub enable_split_brain_prevention: bool,
    // Minimum cluster size for quorum
    pub min_quorum_size: usize,
}

impl Default for SwimConfig {
    fn default() -> Self {
        Self {
            protocol_period: Duration::from_millis(1000),
            indirect_ping_nodes: 3,
            ping_timeout: Duration::from_millis(500),
            indirect_ping_timeout: Duration::from_millis(1000),
            gossip_fanout: 3,
            suspicion_timeout: Duration::from_secs(5),
            gossip_to_the_dead: 3,
            enable_split_brain_prevention: true,
            min_quorum_size: 2,
        }
    }
}

// Pending ping request tracking
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PendingPing {
    target: MemberId,
    sequence: u64,
    sent_at: SystemTime,
    indirect: bool,
}

// SWIM membership manager
pub struct SwimMembership {
    // Configuration
    config: SwimConfig,
    // This node's ID
    local_id: MemberId,
    // This node's member info
    local_member: Arc<RwLock<Member>>,
    // All cluster members
    members: Arc<RwLock<HashMap<MemberId, Member>>>,
    // Pending pings
    pending_pings: Arc<RwLock<HashMap<u64, PendingPing>>>,
    // Sequence number for messages
    sequence: Arc<RwLock<u64>>,
    // Gossip counters (how many times we've gossiped each update)
    gossip_counters: Arc<RwLock<HashMap<String, usize>>>,
}

impl SwimMembership {
    pub fn new(config: SwimConfig, local_id: MemberId, local_addr: SocketAddr) -> Self {
        let metadata = MemberMetadata::default();
        let local_member = Member::new(local_id.clone(), local_addr, metadata);

        Self {
            config,
            local_id: local_id.clone(),
            local_member: Arc::new(RwLock::new(local_member)),
            members: Arc::new(RwLock::new(HashMap::new())),
            pending_pings: Arc::new(RwLock::new(HashMap::new())),
            sequence: Arc::new(RwLock::new(0)),
            gossip_counters: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Get next sequence number
    fn next_sequence(&self) -> u64 {
        let mut seq = self.sequence.write().unwrap();
        *seq += 1;
        *seq
    }

    // Add a new member to the cluster
    pub fn add_member(&self, member: Member) -> Result<(), DbError> {
        let mut members = self.members.write().unwrap();
        members.insert(member.id.clone(), member);
        Ok(())
    }

    // Remove a member from the cluster
    pub fn remove_member(&self, member_id: &str) -> Result<(), DbError> {
        let mut members = self.members.write().unwrap();
        members.remove(member_id);
        Ok(())
    }

    // Get all active members
    pub fn get_active_members(&self) -> Vec<Member> {
        self.members
            .read()
            .unwrap()
            .values()
            .filter(|m| m.is_active())
            .cloned()
            .collect()
    }

    // Get all members
    pub fn get_all_members(&self) -> Vec<Member> {
        self.members.read().unwrap().values().cloned().collect()
    }

    // Get member by ID
    pub fn get_member(&self, id: &str) -> Option<Member> {
        self.members.read().unwrap().get(id).cloned()
    }

    // Select random member for ping
    pub fn select_ping_target(&self) -> Option<MemberId> {
        let members = self.members.read().unwrap();
        let active: Vec<&Member> = members
            .values()
            .filter(|m| m.is_active() && m.id != self.local_id)
            .collect();

        let mut rng = rand::thread_rng();
        active.choose(&mut rng).map(|m| m.id.clone())
    }

    // Create ping message
    pub fn create_ping(&self, target: MemberId) -> (SwimMessage, u64) {
        let sequence = self.next_sequence();
        let msg = SwimMessage::Ping {
            from: self.local_id.clone(),
            sequence,
        };

        // Track pending ping
        let pending = PendingPing {
            target: target.clone(),
            sequence,
            sent_at: SystemTime::now(),
            indirect: false,
        };
        self.pending_pings.write().unwrap().insert(sequence, pending);

        (msg, sequence)
    }

    // Create indirect ping request
    pub fn create_ping_req(&self, target: MemberId) -> Vec<(MemberId, SwimMessage)> {
        let sequence = self.next_sequence();
        let members = self.members.read().unwrap();

        // Select random members to request indirect ping
        let candidates: Vec<&Member> = members
            .values()
            .filter(|m| m.is_active() && m.id != self.local_id && m.id != target)
            .collect();

        let mut rng = rand::thread_rng();
        let selected: Vec<&Member> = candidates
            .choose_multiple(&mut rng, self.config.indirect_ping_nodes)
            .copied()
            .collect();

        let mut messages = Vec::new();
        for member in selected {
            let msg = SwimMessage::PingReq {
                from: self.local_id.clone(),
                target: target.clone(),
                sequence,
            };
            messages.push((member.id.clone(), msg));
        }

        // Track pending ping-req
        if !messages.is_empty() {
            let pending = PendingPing {
                target: target.clone(),
                sequence,
                sent_at: SystemTime::now(),
                indirect: true,
            };
            self.pending_pings.write().unwrap().insert(sequence, pending);
        }

        messages
    }

    // Handle received ping
    pub fn handle_ping(&self, from: MemberId, sequence: u64) -> SwimMessage {
        // Update last_seen for sender
        if let Some(member) = self.members.write().unwrap().get_mut(&from) {
            member.last_seen = SystemTime::now();
        }

        SwimMessage::Ack {
            from: self.local_id.clone(),
            sequence,
        }
    }

    // Handle received ack
    pub fn handle_ack(&self, from: MemberId, sequence: u64) -> Result<(), DbError> {
        // Remove from pending pings
        self.pending_pings.write().unwrap().remove(&sequence);

        // Update member as alive
        if let Some(member) = self.members.write().unwrap().get_mut(&from) {
            member.last_seen = SystemTime::now();
            if member.state == MemberState::Suspect {
                member.state = MemberState::Alive;
                member.state_changed_at = SystemTime::now();
            }
        }

        Ok(())
    }

    // Handle timeout for ping
    pub fn handle_ping_timeout(&self, sequence: u64) -> Result<Vec<(MemberId, SwimMessage)>, DbError> {
        let pending = {
            let pings = self.pending_pings.read().unwrap();
            pings.get(&sequence).cloned()
        };

        if let Some(pending) = pending {
            if !pending.indirect {
                // Direct ping timed out - try indirect ping
                return Ok(self.create_ping_req(pending.target));
            } else {
                // Indirect ping also timed out - mark as suspect
                self.mark_suspect(&pending.target)?;
                self.pending_pings.write().unwrap().remove(&sequence);
            }
        }

        Ok(Vec::new())
    }

    // Mark member as suspect
    fn mark_suspect(&self, member_id: &str) -> Result<(), DbError> {
        let mut members = self.members.write().unwrap();
        if let Some(member) = members.get_mut(member_id) {
            if member.state == MemberState::Alive {
                member.state = MemberState::Suspect;
                member.state_changed_at = SystemTime::now();

                // Create gossip update
                let update_key = format!("suspect-{}-{}", member_id, member.incarnation);
                self.gossip_counters.write().unwrap().insert(update_key, 0);
            }
        }
        Ok(())
    }

    // Check for suspected members that should be marked as failed
    pub fn check_suspected_members(&self) -> Result<Vec<MemberId>, DbError> {
        let mut failed = Vec::new();
        let mut members = self.members.write().unwrap();

        for (id, member) in members.iter_mut() {
            if member.state == MemberState::Suspect {
                if let Ok(elapsed) = member.state_changed_at.elapsed() {
                    if elapsed >= self.config.suspicion_timeout {
                        member.state = MemberState::Failed;
                        member.state_changed_at = SystemTime::now();
                        failed.push(id.clone());

                        // Create gossip update
                        let update_key = format!("failed-{}", id);
                        self.gossip_counters.write().unwrap().insert(update_key, 0);
                    }
                }
            }
        }

        Ok(failed)
    }

    // Create gossip message
    pub fn create_gossip(&self) -> Vec<(MemberId, SwimMessage)> {
        let members = self.members.read().unwrap();
        let mut counters = self.gossip_counters.write().unwrap();

        // Collect updates to gossip
        let mut updates = Vec::new();
        for member in members.values() {
            let update_key = format!("{}-{}-{:?}", member.id, member.incarnation, member.state);
            let count = counters.entry(update_key).or_insert(0);

            if *count < self.config.gossip_to_the_dead || member.is_active() {
                updates.push(MembershipUpdate::from_member(member));
                *count += 1;
            }
        }

        // Select random members to gossip with
        let active: Vec<&Member> = members
            .values()
            .filter(|m| m.is_active() && m.id != self.local_id)
            .collect();

        let mut rng = rand::thread_rng();
        let targets: Vec<&Member> = active
            .choose_multiple(&mut rng, self.config.gossip_fanout)
            .copied()
            .collect();

        let mut messages = Vec::new();
        for target in targets {
            let msg = SwimMessage::Gossip {
                from: self.local_id.clone(),
                updates: updates.clone(),
            };
            messages.push((target.id.clone(), msg));
        }

        messages
    }

    // Handle received gossip
    pub fn handle_gossip(&self, updates: Vec<MembershipUpdate>) -> Result<(), DbError> {
        let mut members = self.members.write().unwrap();

        for update in updates {
            if update.member_id == self.local_id {
                // Update about us - check if we need to refute
                let local = self.local_member.read().unwrap();
                if update.incarnation > local.incarnation
                    || (update.incarnation == local.incarnation
                        && update.state == MemberState::Suspect)
                {
                    drop(local);
                    // Refute the suspicion
                    self.local_member.write().unwrap().refute_suspicion();
                }
                continue;
            }

            match members.get_mut(&update.member_id) {
                Some(existing) => {
                    // Only update if this is newer information
                    if update.incarnation > existing.incarnation
                        || (update.incarnation == existing.incarnation
                            && self.state_supersedes(update.state, existing.state))
                    {
                        existing.incarnation = update.incarnation;
                        existing.state = update.state;
                        existing.addr = update.addr;
                        existing.metadata = update.metadata;
                        existing.state_changed_at = SystemTime::now();
                    }
                }
                None => {
                    // New member
                    let member = Member {
                        id: update.member_id.clone(),
                        addr: update.addr,
                        state: update.state,
                        incarnation: update.incarnation,
                        metadata: update.metadata,
                        state_changed_at: SystemTime::now(),
                        last_seen: SystemTime::now(),
                    };
                    members.insert(update.member_id, member);
                }
            }
        }

        Ok(())
    }

    // Check if one state supersedes another
    fn state_supersedes(&self, new_state: MemberState, old_state: MemberState) -> bool {
        match (old_state, new_state) {
            (MemberState::Alive, MemberState::Suspect) => true,
            (MemberState::Alive, MemberState::Failed) => true,
            (MemberState::Suspect, MemberState::Failed) => true,
            (MemberState::Suspect, MemberState::Alive) => true,
            _ => false,
        }
    }

    // Handle join request
    pub fn handle_join(&self, member: Member) -> Result<Vec<Member>, DbError> {
        let mut members = self.members.write().unwrap();
        members.insert(member.id.clone(), member);

        // Return current member list to joining node
        Ok(members.values().cloned().collect())
    }

    // Handle leave notification
    pub fn handle_leave(&self, member_id: MemberId) -> Result<(), DbError> {
        let mut members = self.members.write().unwrap();
        if let Some(member) = members.get_mut(&member_id) {
            member.state = MemberState::Left;
            member.state_changed_at = SystemTime::now();
        }
        Ok(())
    }

    // Gracefully leave the cluster
    pub fn leave(&self) -> Vec<(MemberId, SwimMessage)> {
        let members = self.members.read().unwrap();
        let msg = SwimMessage::Leave {
            member_id: self.local_id.clone(),
        };

        members
            .values()
            .filter(|m| m.is_active() && m.id != self.local_id)
            .map(|m| (m.id.clone(), msg.clone()))
            .collect()
    }

    // Check for split-brain scenario
    pub fn check_split_brain(&self) -> bool {
        if !self.config.enable_split_brain_prevention {
            return false;
        }

        let active_count = self.get_active_members().len();
        active_count < self.config.min_quorum_size
    }

    // Get cluster health summary
    pub fn get_health_summary(&self) -> ClusterHealth {
        let members = self.members.read().unwrap();
        let mut alive = 0;
        let mut suspect = 0;
        let mut failed = 0;
        let mut left = 0;

        for member in members.values() {
            match member.state {
                MemberState::Alive => alive += 1,
                MemberState::Suspect => suspect += 1,
                MemberState::Failed => failed += 1,
                MemberState::Left => left += 1,
            }
        }

        ClusterHealth {
            total_members: members.len(),
            alive,
            suspect,
            failed,
            left,
            is_split_brain: self.check_split_brain(),
        }
    }
}

// Cluster health summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterHealth {
    pub total_members: usize,
    pub alive: usize,
    pub suspect: usize,
    pub failed: usize,
    pub left: usize,
    pub is_split_brain: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_member_creation() {
        let addr = SocketAddr::from_str("127.0.0.1:8080").unwrap();
        let member = Member::new("node1".into(), addr, MemberMetadata::default());
        assert_eq!(member.id, "node1");
        assert_eq!(member.state, MemberState::Alive);
        assert!(member.is_active());
    }

    #[test]
    fn test_state_transitions() {
        let addr = SocketAddr::from_str("127.0.0.1:8080").unwrap();
        let mut member = Member::new("node1".into(), addr, MemberMetadata::default());

        member.state = MemberState::Suspect;
        assert!(member.is_active());

        member.state = MemberState::Failed;
        assert!(member.is_down());
    }

    #[test]
    fn test_refute_suspicion() {
        let addr = SocketAddr::from_str("127.0.0.1:8080").unwrap();
        let mut member = Member::new("node1".into(), addr, MemberMetadata::default());

        member.state = MemberState::Suspect;
        let old_incarnation = member.incarnation;

        member.refute_suspicion();
        assert_eq!(member.state, MemberState::Alive);
        assert_eq!(member.incarnation, old_incarnation + 1);
    }
}
