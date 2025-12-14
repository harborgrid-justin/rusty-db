// Membership List Management for RustyDB
//
// Maintains a consistent view of cluster membership across all nodes.
// Provides efficient membership tracking, delta updates, and snapshots.
//
// # Features
//
// - Consistent member ordering
// - Membership snapshots
// - Delta updates for efficiency
// - Member health tracking
// - Version vectors for causality tracking

use super::{NodeInfo, NodeStatus};
use crate::common::NodeId;
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

/// Version vector for tracking causality
#[derive(Debug, Clone, Default, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct VersionVector {
    /// Map of node ID to version number
    versions: HashMap<NodeId, u64>,
}

impl VersionVector {
    /// Create a new version vector
    pub fn new() -> Self {
        Self {
            versions: HashMap::new(),
        }
    }

    /// Increment the version for a node
    pub fn increment(&mut self, node_id: &NodeId) {
        let version = self.versions.entry(node_id.clone()).or_insert(0);
        *version += 1;
    }

    /// Get the version for a node
    pub fn get(&self, node_id: &NodeId) -> u64 {
        self.versions.get(node_id).copied().unwrap_or(0)
    }

    /// Merge another version vector (take max of each version)
    pub fn merge(&mut self, other: &VersionVector) {
        for (node_id, version) in &other.versions {
            let current = self.versions.entry(node_id.clone()).or_insert(0);
            *current = (*current).max(*version);
        }
    }

    /// Check if this vector dominates another (all versions >= other's versions)
    pub fn dominates(&self, other: &VersionVector) -> bool {
        other
            .versions
            .iter()
            .all(|(node_id, version)| self.get(node_id) >= *version)
    }
}

/// Member entry in the membership list
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct Member {
    /// Node information
    pub info: NodeInfo,

    /// When this member was first seen
    pub joined_at: u64,

    /// When this member was last updated
    pub updated_at: u64,

    /// Member version (for conflict resolution)
    pub version: u64,

    /// Member health score (0-100)
    pub health: u8,
}

impl Member {
    /// Create a new member
    pub fn new(info: NodeInfo) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        Self {
            info,
            joined_at: now,
            updated_at: now,
            version: 0,
            health: 100,
        }
    }

    /// Update member with new info
    pub fn update(&mut self, info: NodeInfo) {
        self.info = info;
        self.version += 1;
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
    }

    /// Update health score
    pub fn update_health(&mut self, health: u8) {
        self.health = health.min(100);
    }
}

/// Delta update for membership changes
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum MembershipDelta {
    /// Member added
    Added(Member),

    /// Member updated
    Updated(Member),

    /// Member removed
    Removed(NodeId),
}

/// Snapshot of membership list at a point in time
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct MembershipSnapshot {
    /// All members in the snapshot
    pub members: Vec<Member>,

    /// Version vector at snapshot time
    pub version: VersionVector,

    /// Snapshot timestamp
    pub timestamp: u64,
}

impl MembershipSnapshot {
    /// Get member count
    pub fn count(&self) -> usize {
        self.members.len()
    }

    /// Get alive member count
    pub fn alive_count(&self) -> usize {
        self.members
            .iter()
            .filter(|m| m.info.status == NodeStatus::Alive)
            .count()
    }
}

/// Membership list manager
pub struct MembershipList {
    /// Members indexed by node ID (BTreeMap for consistent ordering)
    members: BTreeMap<NodeId, Member>,

    /// Version vector for causality tracking
    version: VersionVector,

    /// Local node ID
    local_node_id: Option<NodeId>,
}

impl MembershipList {
    /// Create a new membership list
    pub fn new() -> Self {
        Self {
            members: BTreeMap::new(),
            version: VersionVector::new(),
            local_node_id: None,
        }
    }

    /// Set the local node ID
    pub fn set_local_node(&mut self, node_id: NodeId) {
        self.local_node_id = Some(node_id);
    }

    /// Add or update a member
    pub fn add_or_update(&mut self, info: NodeInfo) -> Option<MembershipDelta> {
        let node_id = info.id.clone();

        if let Some(member) = self.members.get_mut(&node_id) {
            // Update existing member
            let old_version = member.version;
            member.update(info.clone());
            self.version.increment(&node_id);

            if member.version > old_version {
                Some(MembershipDelta::Updated(member.clone()))
            } else {
                None
            }
        } else {
            // Add new member
            let member = Member::new(info);
            self.members.insert(node_id.clone(), member.clone());
            self.version.increment(&node_id);

            Some(MembershipDelta::Added(member))
        }
    }

    /// Remove a member
    pub fn remove(&mut self, node_id: &NodeId) -> Option<MembershipDelta> {
        if self.members.remove(node_id).is_some() {
            self.version.increment(node_id);
            Some(MembershipDelta::Removed(node_id.clone()))
        } else {
            None
        }
    }

    /// Get a member by ID
    pub fn get(&self, node_id: &NodeId) -> Option<&Member> {
        self.members.get(node_id)
    }

    /// Get all members
    pub fn all(&self) -> Vec<&Member> {
        self.members.values().collect()
    }

    /// Get all alive members
    pub fn alive(&self) -> Vec<&Member> {
        self.members
            .values()
            .filter(|m| m.info.status == NodeStatus::Alive)
            .collect()
    }

    /// Get member count
    pub fn count(&self) -> usize {
        self.members.len()
    }

    /// Get alive member count
    pub fn alive_count(&self) -> usize {
        self.alive().len()
    }

    /// Update member status
    pub fn update_status(&mut self, node_id: &NodeId, status: NodeStatus) -> Result<()> {
        if let Some(member) = self.members.get_mut(node_id) {
            member.info.status = status;
            member.version += 1;
            self.version.increment(node_id);
            Ok(())
        } else {
            Err(DbError::NotFound(format!("Member not found: {}", node_id)))
        }
    }

    /// Update member health
    pub fn update_health(&mut self, node_id: &NodeId, health: u8) -> Result<()> {
        if let Some(member) = self.members.get_mut(node_id) {
            member.update_health(health);
            self.version.increment(node_id);
            Ok(())
        } else {
            Err(DbError::NotFound(format!("Member not found: {}", node_id)))
        }
    }

    /// Create a snapshot of the current membership
    pub fn snapshot(&self) -> MembershipSnapshot {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        MembershipSnapshot {
            members: self.members.values().cloned().collect(),
            version: self.version.clone(),
            timestamp,
        }
    }

    /// Apply a membership delta
    pub fn apply_delta(&mut self, delta: MembershipDelta) -> Result<()> {
        match delta {
            MembershipDelta::Added(member) => {
                self.members.insert(member.info.id.clone(), member.clone());
                self.version.increment(&member.info.id);
            }

            MembershipDelta::Updated(member) => {
                if let Some(existing) = self.members.get_mut(&member.info.id) {
                    // Only apply if version is newer
                    if member.version > existing.version {
                        *existing = member.clone();
                        self.version.increment(&member.info.id);
                    }
                } else {
                    // Member doesn't exist, add it
                    self.members.insert(member.info.id.clone(), member.clone());
                    self.version.increment(&member.info.id);
                }
            }

            MembershipDelta::Removed(node_id) => {
                self.members.remove(&node_id);
                self.version.increment(&node_id);
            }
        }

        Ok(())
    }

    /// Merge a snapshot into this membership list
    pub fn merge_snapshot(&mut self, snapshot: MembershipSnapshot) -> Vec<MembershipDelta> {
        let mut deltas = Vec::new();

        for member in snapshot.members {
            let node_id = member.info.id.clone();

            if let Some(existing) = self.members.get(&node_id) {
                // Only update if incoming version is newer
                if member.version > existing.version {
                    self.members.insert(node_id.clone(), member.clone());
                    self.version.increment(&node_id);
                    deltas.push(MembershipDelta::Updated(member));
                }
            } else {
                // New member
                self.members.insert(node_id.clone(), member.clone());
                self.version.increment(&node_id);
                deltas.push(MembershipDelta::Added(member));
            }
        }

        // Merge version vectors
        self.version.merge(&snapshot.version);

        deltas
    }

    /// Get the version vector
    pub fn version(&self) -> &VersionVector {
        &self.version
    }

    /// Get members in consistent order (sorted by node ID)
    pub fn ordered_members(&self) -> Vec<&Member> {
        self.members.values().collect()
    }

    /// Clear all members
    pub fn clear(&mut self) {
        self.members.clear();
        self.version = VersionVector::new();
    }
}

impl Default for MembershipList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_vector() {
        let mut v1 = VersionVector::new();
        let mut v2 = VersionVector::new();

        v1.increment(&"node1".to_string());
        v1.increment(&"node1".to_string());
        v1.increment(&"node2".to_string());

        v2.increment(&"node1".to_string());
        v2.increment(&"node3".to_string());

        assert_eq!(v1.get(&"node1".to_string()), 2);
        assert_eq!(v1.get(&"node2".to_string()), 1);
        assert_eq!(v2.get(&"node1".to_string()), 1);
        assert_eq!(v2.get(&"node3".to_string()), 1);

        v1.merge(&v2);

        assert_eq!(v1.get(&"node1".to_string()), 2);
        assert_eq!(v1.get(&"node2".to_string()), 1);
        assert_eq!(v1.get(&"node3".to_string()), 1);
    }

    #[test]
    fn test_membership_list() {
        let mut list = MembershipList::new();

        let node1 = NodeInfo::new("node1".to_string(), "127.0.0.1:7946".parse().unwrap());

        let node2 = NodeInfo::new("node2".to_string(), "127.0.0.1:7947".parse().unwrap());

        // Add members
        let delta1 = list.add_or_update(node1.clone());
        assert!(matches!(delta1, Some(MembershipDelta::Added(_))));

        let delta2 = list.add_or_update(node2.clone());
        assert!(matches!(delta2, Some(MembershipDelta::Added(_))));

        assert_eq!(list.count(), 2);
        assert_eq!(list.alive_count(), 2);

        // Update member
        let mut node1_updated = node1.clone();
        node1_updated
            .metadata
            .insert("key".to_string(), "value".to_string());

        let delta3 = list.add_or_update(node1_updated);
        assert!(matches!(delta3, Some(MembershipDelta::Updated(_))));

        assert_eq!(list.count(), 2);

        // Remove member
        let delta4 = list.remove(&"node1".to_string());
        assert!(matches!(delta4, Some(MembershipDelta::Removed(_))));

        assert_eq!(list.count(), 1);
    }

    #[test]
    fn test_membership_snapshot() {
        let mut list = MembershipList::new();

        let node1 = NodeInfo::new("node1".to_string(), "127.0.0.1:7946".parse().unwrap());

        list.add_or_update(node1);

        let snapshot = list.snapshot();

        assert_eq!(snapshot.count(), 1);
        assert_eq!(snapshot.alive_count(), 1);
    }
}
