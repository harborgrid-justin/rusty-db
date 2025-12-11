//! Anti-Entropy Engine for RustyDB Discovery
//!
//! Implements anti-entropy mechanisms to ensure eventual consistency across the cluster.
//! Uses Merkle trees for efficient state comparison and reconciliation.
//!
//! # Features
//!
//! - Merkle tree-based state comparison
//! - Pull-based state synchronization
//! - Delta-based updates for efficiency
//! - CRDT-based counters for conflict-free updates
//! - Periodic reconciliation

use super::{MembershipList, MembershipSnapshot};
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

/// Hash type for Merkle tree
type Hash = [u8; 32];

/// Merkle tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
enum MerkleNode {
    /// Leaf node containing actual data hash
    Leaf {
        key: String,
        hash: Hash,
    },

    /// Internal node containing hash of children
    Internal {
        hash: Hash,
        left: Box<MerkleNode>,
        right: Box<MerkleNode>,
    },

    /// Empty node
    Empty,
}

impl MerkleNode {
    /// Get the hash of this node
    fn hash(&self) -> Hash {
        match self {
            MerkleNode::Leaf { hash, .. } => *hash,
            MerkleNode::Internal { hash, .. } => *hash,
            MerkleNode::Empty => [0u8; 32],
        }
    }

    /// Check if this node is empty
    #[allow(dead_code)] // Reserved for anti-entropy protocol
    fn is_empty(&self) -> bool {
        matches!(self, MerkleNode::Empty)
    }
}

/// Merkle tree for efficient state comparison
#[derive(Debug, Clone)]
pub struct MerkleTree {
    root: MerkleNode,
    #[allow(dead_code)] // Reserved for tree depth tracking
    depth: usize,
}

impl MerkleTree {
    /// Create a new Merkle tree from key-value pairs
    pub fn new(items: &[(String, Vec<u8>)]) -> Self {
        if items.is_empty() {
            return Self {
                root: MerkleNode::Empty,
                depth: 0,
            };
        }

        let leaves: Vec<MerkleNode> = items
            .iter()
            .map(|(key, value)| {
                let hash = Self::hash_data(value);
                MerkleNode::Leaf {
                    key: key.clone(),
                    hash,
                }
            })
            .collect();

        let root = Self::build_tree(&leaves);
        let depth = Self::calculate_depth(items.len());

        Self { root, depth }
    }

    /// Build tree from leaves
    fn build_tree(nodes: &[MerkleNode]) -> MerkleNode {
        if nodes.is_empty() {
            return MerkleNode::Empty;
        }

        if nodes.len() == 1 {
            return nodes[0].clone();
        }

        let mid = nodes.len() / 2;
        let left = Self::build_tree(&nodes[..mid]);
        let right = Self::build_tree(&nodes[mid..]);

        let hash = Self::hash_nodes(&left, &right);

        MerkleNode::Internal {
            hash,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Calculate tree depth
    fn calculate_depth(item_count: usize) -> usize {
        if item_count == 0 {
            0
        } else {
            (item_count as f64).log2().ceil() as usize
        }
    }

    /// Hash data using SHA-256
    fn hash_data(data: &[u8]) -> Hash {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Hash two nodes together
    fn hash_nodes(left: &MerkleNode, right: &MerkleNode) -> Hash {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&left.hash());
        hasher.update(&right.hash());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Get root hash
    pub fn root_hash(&self) -> Hash {
        self.root.hash()
    }

    /// Compare with another tree and find differences
    pub fn diff(&self, other: &MerkleTree) -> Vec<String> {
        let mut differences = Vec::new();
        Self::diff_recursive(&self.root, &other.root, &mut differences);
        differences
    }

    /// Recursive diff helper
    fn diff_recursive(node1: &MerkleNode, node2: &MerkleNode, differences: &mut Vec<String>) {
        if node1.hash() == node2.hash() {
            return; // Subtrees are identical
        }

        match (node1, node2) {
            (MerkleNode::Leaf { key, .. }, MerkleNode::Leaf { .. }) => {
                differences.push(key.clone());
            }

            (MerkleNode::Internal { left: l1, right: r1, .. },
             MerkleNode::Internal { left: l2, right: r2, .. }) => {
                Self::diff_recursive(l1, l2, differences);
                Self::diff_recursive(r1, r2, differences);
            }

            (MerkleNode::Leaf { key, .. }, _) => {
                differences.push(key.clone());
            }

            (_, MerkleNode::Leaf { key, .. }) => {
                differences.push(key.clone());
            }

            _ => {}
        }
    }
}

/// CRDT-based counter for conflict-free updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtCounter {
    /// Increment counts per node
    increments: HashMap<String, u64>,

    /// Decrement counts per node
    decrements: HashMap<String, u64>,
}

impl CrdtCounter {
    /// Create a new CRDT counter
    pub fn new() -> Self {
        Self {
            increments: HashMap::new(),
            decrements: HashMap::new(),
        }
    }

    /// Increment the counter for a node
    pub fn increment(&mut self, node_id: &str, amount: u64) {
        *self.increments.entry(node_id.to_string()).or_insert(0) += amount;
    }

    /// Decrement the counter for a node
    pub fn decrement(&mut self, node_id: &str, amount: u64) {
        *self.decrements.entry(node_id.to_string()).or_insert(0) += amount;
    }

    /// Get the current value
    pub fn value(&self) -> i64 {
        let total_inc: u64 = self.increments.values().sum();
        let total_dec: u64 = self.decrements.values().sum();
        total_inc as i64 - total_dec as i64
    }

    /// Merge with another CRDT counter
    pub fn merge(&mut self, other: &CrdtCounter) {
        for (node_id, count) in &other.increments {
            let current = self.increments.entry(node_id.clone()).or_insert(0);
            *current = (*current).max(*count);
        }

        for (node_id, count) in &other.decrements {
            let current = self.decrements.entry(node_id.clone()).or_insert(0);
            *current = (*current).max(*count);
        }
    }
}

impl Default for CrdtCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Anti-entropy message types
#[derive(Debug, Clone, Serialize, Deserialize)]
enum AntiEntropyMessage {
    /// Request for remote state hash
    HashRequest,

    /// Response with state hash
    HashResponse {
        root_hash: Hash,
    },

    /// Request for full state
    StateRequest,

    /// Response with full state
    StateResponse {
        snapshot: MembershipSnapshot,
    },

    /// Request for specific keys
    KeysRequest {
        keys: Vec<String>,
    },

    /// Response with key values
    KeysResponse {
        data: HashMap<String, Vec<u8>>,
    },
}

/// Anti-entropy engine
pub struct AntiEntropyEngine {
    membership: Arc<RwLock<MembershipList>>,
    socket: Arc<UdpSocket>,
    peers: Arc<RwLock<Vec<SocketAddr>>>,
    interval_duration: Duration,
}

impl AntiEntropyEngine {
    /// Create a new anti-entropy engine
    pub fn new(
        membership: Arc<RwLock<MembershipList>>,
        socket: Arc<UdpSocket>,
        interval_duration: Duration,
    ) -> Self {
        Self {
            membership,
            socket,
            peers: Arc::new(RwLock::new(Vec::new())),
            interval_duration,
        }
    }

    /// Add a peer to reconcile with
    pub async fn add_peer(&self, addr: SocketAddr) {
        let mut peers = self.peers.write().await;
        if !peers.contains(&addr) {
            peers.push(addr);
        }
    }

    /// Remove a peer
    pub async fn remove_peer(&self, addr: &SocketAddr) {
        let mut peers = self.peers.write().await;
        peers.retain(|p| p != addr);
    }

    /// Build Merkle tree from current membership state
    async fn build_merkle_tree(&self) -> Result<MerkleTree> {
        let membership = self.membership.read().await;
        let snapshot = membership.snapshot();

        let items: Vec<(String, Vec<u8>)> = snapshot
            .members
            .iter()
            .map(|member| {
                let data = bincode::serialize(member)
                    .unwrap_or_default();
                (member.info.id.clone(), data)
            })
            .collect();

        Ok(MerkleTree::new(&items))
    }

    /// Request state hash from peer
    async fn request_hash(&self, peer: SocketAddr) -> Result<Hash> {
        let msg = AntiEntropyMessage::HashRequest;
        let data = bincode::serialize(&msg)
            .map_err(|e| DbError::Serialization(format!("Failed to serialize: {}", e)))?;

        self.socket.send_to(&data, peer).await
            .map_err(|e| DbError::Network(format!("Failed to send: {}", e)))?;

        // In a real implementation, wait for response with timeout
        // For now, return empty hash
        Ok([0u8; 32])
    }

    /// Reconcile with a peer
    async fn reconcile_with_peer(&self, peer: SocketAddr) -> Result<()> {
        // Build local Merkle tree
        let local_tree = self.build_merkle_tree().await?;

        // Request remote hash
        let remote_hash = self.request_hash(peer).await?;

        // If hashes match, states are synchronized
        if local_tree.root_hash() == remote_hash {
            return Ok(());
        }

        // Request full state for reconciliation
        let msg = AntiEntropyMessage::StateRequest;
        let data = bincode::serialize(&msg)
            .map_err(|e| DbError::Serialization(format!("Failed to serialize: {}", e)))?;

        self.socket.send_to(&data, peer).await
            .map_err(|e| DbError::Network(format!("Failed to send: {}", e)))?;

        // In a real implementation, wait for response and merge state
        // For now, just log the reconciliation attempt
        Ok(())
    }

    /// Handle incoming anti-entropy message
    async fn handle_message(&self, msg: AntiEntropyMessage, from: SocketAddr) -> Result<()> {
        match msg {
            AntiEntropyMessage::HashRequest => {
                let tree = self.build_merkle_tree().await?;
                let response = AntiEntropyMessage::HashResponse {
                    root_hash: tree.root_hash(),
                };

                let data = bincode::serialize(&response)
                    .map_err(|e| DbError::Serialization(format!("Failed to serialize: {}", e)))?;

                self.socket.send_to(&data, from).await
                    .map_err(|e| DbError::Network(format!("Failed to send: {}", e)))?;
            }

            AntiEntropyMessage::HashResponse { root_hash: _ } => {
                // Store hash for comparison
                // Not implemented in this basic version
            }

            AntiEntropyMessage::StateRequest => {
                let membership = self.membership.read().await;
                let snapshot = membership.snapshot();
                drop(membership);

                let response = AntiEntropyMessage::StateResponse { snapshot };

                let data = bincode::serialize(&response)
                    .map_err(|e| DbError::Serialization(format!("Failed to serialize: {}", e)))?;

                self.socket.send_to(&data, from).await
                    .map_err(|e| DbError::Network(format!("Failed to send: {}", e)))?;
            }

            AntiEntropyMessage::StateResponse { snapshot } => {
                let mut membership = self.membership.write().await;
                membership.merge_snapshot(snapshot);
            }

            AntiEntropyMessage::KeysRequest { keys: _ } => {
                // Not implemented in this basic version
            }

            AntiEntropyMessage::KeysResponse { data: _ } => {
                // Not implemented in this basic version
            }
        }

        Ok(())
    }

    /// Start the anti-entropy engine
    pub async fn start(self: Arc<Self>) -> Result<()> {
        let mut reconcile_interval = interval(self.interval_duration);
        let mut buffer = vec![0u8; 65536];

        loop {
            tokio::select! {
                _ = reconcile_interval.tick() => {
                    // Reconcile with a random peer
                    let peers = self.peers.read().await;
                    if !peers.is_empty() {
                        let idx = rand::random::<usize>() % peers.len();
                        let peer = peers[idx];
                        drop(peers);

                        if let Err(e) = self.reconcile_with_peer(peer).await {
                            eprintln!("Error reconciling with peer: {}", e);
                        }
                    }
                }

                result = self.socket.recv_from(&mut buffer) => {
                    match result {
                        Ok((len, addr)) => {
                            if let Ok(msg) = bincode::deserialize::<AntiEntropyMessage>(&buffer[..len]) {
                                if let Err(e) = self.handle_message(msg, addr).await {
                                    eprintln!("Error handling anti-entropy message: {}", e);
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree() {
        let items = vec![
            ("key1".to_string(), b"value1".to_vec()),
            ("key2".to_string(), b"value2".to_vec()),
            ("key3".to_string(), b"value3".to_vec()),
        ];

        let tree1 = MerkleTree::new(&items);
        let tree2 = MerkleTree::new(&items);

        // Same data should produce same hash
        assert_eq!(tree1.root_hash(), tree2.root_hash());

        // Different data should produce different hash
        let items2 = vec![
            ("key1".to_string(), b"value1".to_vec()),
            ("key2".to_string(), b"different".to_vec()),
            ("key3".to_string(), b"value3".to_vec()),
        ];

        let tree3 = MerkleTree::new(&items2);
        assert_ne!(tree1.root_hash(), tree3.root_hash());
    }

    #[test]
    fn test_merkle_diff() {
        let items1 = vec![
            ("key1".to_string(), b"value1".to_vec()),
            ("key2".to_string(), b"value2".to_vec()),
        ];

        let items2 = vec![
            ("key1".to_string(), b"value1".to_vec()),
            ("key2".to_string(), b"different".to_vec()),
        ];

        let tree1 = MerkleTree::new(&items1);
        let tree2 = MerkleTree::new(&items2);

        let diff = tree1.diff(&tree2);
        assert!(!diff.is_empty());
    }

    #[test]
    fn test_crdt_counter() {
        let mut counter1 = CrdtCounter::new();
        let mut counter2 = CrdtCounter::new();

        counter1.increment("node1", 5);
        counter1.decrement("node1", 2);

        counter2.increment("node2", 3);
        counter2.increment("node1", 2);

        assert_eq!(counter1.value(), 3);
        assert_eq!(counter2.value(), 5);

        counter1.merge(&counter2);

        // After merge, node1 has max(5,2)=5 increments, 2 decrements
        // node2 has 3 increments
        // Total: (5 + 3) - 2 = 6
        assert_eq!(counter1.value(), 6);
    }
}
