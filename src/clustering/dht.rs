/// Distributed Hash Table (DHT) Implementation
///
/// This module implements a distributed hash table for data partitioning and routing.
/// Features include:
/// - Consistent hashing with virtual nodes for balanced distribution
/// - Range-based partitioning as an alternative strategy
/// - Automatic rebalancing when nodes join or leave
/// - Hot spot detection and dynamic shard splitting
/// - Replication factor configuration
///
/// The DHT is used for:
/// - Determining which node stores which data
/// - Routing queries to the correct nodes
/// - Rebalancing data across the cluster

use crate::error::DbError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// Hash ring position (0 to 2^64-1)
pub type HashPosition = u64;

/// Node identifier in DHT
pub type DhtNodeId = String;

/// Partition/Shard identifier
pub type ShardId = u64;

/// Key type for data storage
pub type DataKey = Vec<u8>;

/// Hashing strategy for the DHT
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashStrategy {
    /// Consistent hashing with virtual nodes
    ConsistentHash,
    /// Range-based partitioning
    RangeBased,
    /// Rendezvous hashing (HRW - Highest Random Weight)
    RendezvousHash,
}

/// Virtual node in the hash ring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualNode {
    /// Position on the hash ring
    pub position: HashPosition,
    /// Actual node ID this virtual node belongs to
    pub node_id: DhtNodeId,
    /// Virtual node index (0..num_vnodes-1)
    pub vnode_index: u32,
    /// Timestamp when added to ring
    pub added_at: SystemTime,
}

impl VirtualNode {
    pub fn new(position: HashPosition, node_id: DhtNodeId, vnode_index: u32) -> Self {
        Self {
            position,
            node_id,
            vnode_index,
            added_at: SystemTime::now(),
        }
    }
}

/// Range partition definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangePartition {
    /// Partition ID
    pub id: ShardId,
    /// Start of range (inclusive)
    pub start: HashPosition,
    /// End of range (exclusive)
    pub end: HashPosition,
    /// Primary node responsible for this range
    pub primary_node: DhtNodeId,
    /// Replica nodes for this range
    pub replicas: Vec<DhtNodeId>,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last split timestamp (if any)
    pub last_split: Option<SystemTime>,
}

impl RangePartition {
    pub fn new(id: ShardId, start: HashPosition, end: HashPosition, node: DhtNodeId) -> Self {
        Self {
            id,
            start,
            end,
            primary_node: node,
            replicas: Vec::new(),
            created_at: SystemTime::now(),
            last_split: None,
        }
    }

    /// Check if hash falls within this partition
    pub fn contains(&self, hash: HashPosition) -> bool {
        if self.start < self.end {
            hash >= self.start && hash < self.end
        } else {
            // Wraparound case
            hash >= self.start || hash < self.end
        }
    }

    /// Get size of partition
    pub fn size(&self) -> u64 {
        if self.start < self.end {
            self.end - self.start
        } else {
            (u64::MAX - self.start) + self.end
        }
    }
}

/// Hot spot detection metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSpotMetrics {
    /// Shard ID
    pub shard_id: ShardId,
    /// Requests per second
    pub requests_per_second: f64,
    /// Data size in bytes
    pub data_size: u64,
    /// Number of keys
    pub key_count: u64,
    /// Average key size
    pub avg_key_size: u64,
    /// Last measurement time
    pub measured_at: SystemTime,
}

impl HotSpotMetrics {
    pub fn new(shard_id: ShardId) -> Self {
        Self {
            shard_id,
            requests_per_second: 0.0,
            data_size: 0,
            key_count: 0,
            avg_key_size: 0,
            measured_at: SystemTime::now(),
        }
    }

    /// Check if this shard is a hot spot
    pub fn is_hot_spot(&self, threshold_rps: f64, threshold_size: u64) -> bool {
        self.requests_per_second > threshold_rps || self.data_size > threshold_size
    }
}

/// Rebalancing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceOperation {
    /// Operation ID
    pub id: String,
    /// Source shard
    pub source_shard: ShardId,
    /// Target shard (for splits)
    pub target_shard: Option<ShardId>,
    /// Source node
    pub source_node: DhtNodeId,
    /// Target node
    pub target_node: DhtNodeId,
    /// Range to move
    pub hash_range: (HashPosition, HashPosition),
    /// Status
    pub status: RebalanceStatus,
    /// Started at
    pub started_at: SystemTime,
    /// Completed at
    pub completed_at: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RebalanceStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// DHT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtConfig {
    /// Hash strategy to use
    pub strategy: HashStrategy,
    /// Number of virtual nodes per physical node
    pub virtual_nodes_per_node: u32,
    /// Replication factor
    pub replication_factor: usize,
    /// Hot spot detection threshold (requests/sec)
    pub hot_spot_rps_threshold: f64,
    /// Hot spot detection threshold (data size in bytes)
    pub hot_spot_size_threshold: u64,
    /// Enable automatic rebalancing
    pub auto_rebalance: bool,
    /// Minimum time between rebalances
    pub rebalance_interval_secs: u64,
}

impl Default for DhtConfig {
    fn default() -> Self {
        Self {
            strategy: HashStrategy::ConsistentHash,
            virtual_nodes_per_node: 256,
            replication_factor: 3,
            hot_spot_rps_threshold: 10000.0,
            hot_spot_size_threshold: 10 * 1024 * 1024 * 1024, // 10GB
            auto_rebalance: true,
            rebalance_interval_secs: 300, // 5 minutes
        }
    }
}

/// Main DHT structure
pub struct DistributedHashTable {
    /// Configuration
    config: DhtConfig,
    /// Hash ring for consistent hashing (position -> virtual node)
    hash_ring: Arc<RwLock<BTreeMap<HashPosition, VirtualNode>>>,
    /// Range partitions for range-based partitioning
    partitions: Arc<RwLock<Vec<RangePartition>>>,
    /// Node metadata
    nodes: Arc<RwLock<HashMap<DhtNodeId, NodeMetadata>>>,
    /// Hot spot metrics
    metrics: Arc<RwLock<HashMap<ShardId, HotSpotMetrics>>>,
    /// Active rebalancing operations
    rebalance_ops: Arc<RwLock<Vec<RebalanceOperation>>>,
    /// Next shard ID
    next_shard_id: Arc<RwLock<ShardId>>,
}

/// Node metadata in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub id: DhtNodeId,
    pub address: String,
    pub port: u16,
    pub capacity: u64, // Storage capacity in bytes
    pub used: u64,     // Used storage in bytes
    pub vnodes: Vec<u32>, // Virtual node indices
    pub shards: Vec<ShardId>, // Primary shards on this node
    pub joined_at: SystemTime,
}

impl NodeMetadata {
    pub fn new(id: DhtNodeId, address: String, port: u16, capacity: u64) -> Self {
        Self {
            id,
            address,
            port,
            capacity,
            used: 0,
            vnodes: Vec::new(),
            shards: Vec::new(),
            joined_at: SystemTime::now(),
        }
    }

    pub fn available_capacity(&self) -> u64 {
        self.capacity.saturating_sub(self.used)
    }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            (self.used as f64) / (self.capacity as f64)
        }
    }
}

impl DistributedHashTable {
    pub fn new(config: DhtConfig) -> Self {
        Self {
            config,
            hash_ring: Arc::new(RwLock::new(BTreeMap::new())),
            partitions: Arc::new(RwLock::new(Vec::new())),
            nodes: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            rebalance_ops: Arc::new(RwLock::new(Vec::new())),
            next_shard_id: Arc::new(RwLock::new(0)),
        }
    }

    /// Hash a key to a position on the ring
    fn hash_key(&self, key: &[u8]) -> HashPosition {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Hash node ID with virtual node index
    fn hash_vnode(&self, node_id: &str, vnode_index: u32) -> HashPosition {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        node_id.hash(&mut hasher);
        vnode_index.hash(&mut hasher);
        hasher.finish()
    }

    /// Add a node to the DHT
    pub fn add_node(&self, metadata: NodeMetadata) -> Result<(), DbError> {
        let node_id = metadata.id.clone();

        match self.config.strategy {
            HashStrategy::ConsistentHash => {
                self.add_node_consistent_hash(metadata)?;
            }
            HashStrategy::RangeBased => {
                self.add_node_range_based(metadata)?;
            }
            HashStrategy::RendezvousHash => {
                self.add_node_rendezvous(metadata)?;
            }
        }

        // Trigger rebalancing if enabled
        if self.config.auto_rebalance {
            self.trigger_rebalance()?;
        }

        Ok(())
    }

    /// Add node using consistent hashing
    fn add_node_consistent_hash(&self, mut metadata: NodeMetadata) -> Result<(), DbError> {
        let mut ring = self.hash_ring.write().unwrap();
        let node_id = metadata.id.clone();

        // Create virtual nodes
        for _i in 0..self.config.virtual_nodes_per_node {
            let position = self.hash_vnode(&node_id, i);
            let vnode = VirtualNode::new(position, node_id.clone(), i);
            ring.insert(position, vnode);
            metadata.vnodes.push(i);
        }

        // Store node metadata
        let mut nodes = self.nodes.write().unwrap();
        nodes.insert(node_id, metadata);

        Ok(())
    }

    /// Add node using range-based partitioning
    fn add_node_range_based(&self, metadata: NodeMetadata) -> Result<(), DbError> {
        let mut partitions = self.partitions.write().unwrap();
        let mut nodes = self.nodes.write().unwrap();
        let node_id = metadata.id.clone();

        // If this is the first node, create initial partition
        if partitions.is_empty() {
            let mut shard_id = self.next_shard_id.write().unwrap();
            let partition = RangePartition::new(*shard_id, 0, u64::MAX, node_id.clone());
            *shard_id += 1;
            partitions.push(partition);
        } else {
            // Find largest partition and split it
            if let Some(largest_idx) = self.find_largest_partition(&partitions) {
                let mut shard_id = self.next_shard_id.write().unwrap();
                self.split_partition(largest_idx, node_id.clone(), *shard_id, &mut partitions)?;
                *shard_id += 1;
            }
        }

        nodes.insert(node_id, metadata);
        Ok(())
    }

    /// Add node using rendezvous hashing
    fn add_node_rendezvous(&self, metadata: NodeMetadata) -> Result<(), DbError> {
        let mut nodes = self.nodes.write().unwrap();
        nodes.insert(metadata.id.clone(), metadata);
        // Rendezvous hashing doesn't require ring updates
        Ok(())
    }

    /// Remove a node from the DHT
    pub fn remove_node(&self, node_id: &str) -> Result<(), DbError> {
        match self.config.strategy {
            HashStrategy::ConsistentHash => {
                self.remove_node_consistent_hash(node_id)?;
            }
            HashStrategy::RangeBased => {
                self.remove_node_range_based(node_id)?;
            }
            HashStrategy::RendezvousHash => {
                self.remove_node_rendezvous(node_id)?;
            }
        }

        // Remove node metadata
        let mut nodes = self.nodes.write().unwrap();
        nodes.remove(node_id);

        // Trigger rebalancing if enabled
        if self.config.auto_rebalance {
            drop(nodes);
            self.trigger_rebalance()?;
        }

        Ok(())
    }

    /// Remove node from consistent hash ring
    fn remove_node_consistent_hash(&self, node_id: &str) -> Result<(), DbError> {
        let mut ring = self.hash_ring.write().unwrap();

        // Remove all virtual nodes for this node
        ring.retain(|_, vnode| vnode.node_id != node_id);

        Ok(())
    }

    /// Remove node from range-based partitions
    fn remove_node_range_based(&self, node_id: &str) -> Result<(), DbError> {
        let mut partitions = self.partitions.write().unwrap();
        let nodes = self.nodes.read().unwrap();

        // Find partitions owned by this node
        let affected_partitions: Vec<usize> = partitions
            .iter()
            .enumerate()
            .filter(|(_, p)| p.primary_node == node_id)
            .map(|(i, _)| i)
            .collect();

        // Reassign partitions to other nodes
        for idx in affected_partitions {
            if let Some(new_owner) = self.find_least_loaded_node(&nodes, Some(node_id)) {
                partitions[idx].primary_node = new_owner;
            }
        }

        Ok(())
    }

    /// Remove node from rendezvous hash
    fn remove_node_rendezvous(&self, _node_id: &str) -> Result<(), DbError> {
        // Node removal handled by nodes map
        Ok(())
    }

    /// Find which node should handle a key
    pub fn get_node_for_key(&self, key: &[u8]) -> Result<DhtNodeId, DbError> {
        match self.config.strategy {
            HashStrategy::ConsistentHash => self.get_node_consistent_hash(key),
            HashStrategy::RangeBased => self.get_node_range_based(key),
            HashStrategy::RendezvousHash => self.get_node_rendezvous(key),
        }
    }

    /// Get node using consistent hashing
    fn get_node_consistent_hash(&self, key: &[u8]) -> Result<DhtNodeId, DbError> {
        let _hash = self.hash_key(key);
        let ring = self.hash_ring.read().unwrap();

        if ring.is_empty() {
            return Err(DbError::Internal("No nodes in hash ring".into()));
        }

        // Find first node clockwise from hash position
        for (_, vnode) in ring.range(hash..) {
            return Ok(vnode.node_id.clone());
        }

        // Wrap around to first node
        if let Some((_, vnode)) = ring.iter().next() {
            return Ok(vnode.node_id.clone());
        }

        Err(DbError::Internal("No nodes available".into()))
    }

    /// Get node using range-based partitioning
    fn get_node_range_based(&self, key: &[u8]) -> Result<DhtNodeId, DbError> {
        let _hash = self.hash_key(key);
        let partitions = self.partitions.read().unwrap();

        for partition in partitions.iter() {
            if partition.contains(hash) {
                return Ok(partition.primary_node.clone());
            }
        }

        Err(DbError::Internal("No partition found for key".into()))
    }

    /// Get node using rendezvous hashing
    fn get_node_rendezvous(&self, key: &[u8]) -> Result<DhtNodeId, DbError> {
        use std::collections::hash_map::DefaultHasher;

        let nodes = self.nodes.read().unwrap();
        if nodes.is_empty() {
            return Err(DbError::Internal("No nodes available".into()));
        }

        // Calculate weight for each node and pick highest
        let mut best_node = None;
        let mut best_weight = 0u64;

        for node_id in nodes.keys() {
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            node_id.hash(&mut hasher);
            let weight = hasher.finish();

            if weight > best_weight {
                best_weight = weight;
                best_node = Some(node_id.clone());
            }
        }

        best_node.ok_or_else(|| DbError::Internal("No node found".into()))
    }

    /// Get replica nodes for a key
    pub fn get_replica_nodes(&self, key: &[u8]) -> Result<Vec<DhtNodeId>, DbError> {
        let primary = self.get_node_for_key(key)?;
        let mut replicas = vec![primary.clone()];

        match self.config.strategy {
            HashStrategy::ConsistentHash => {
                let _hash = self.hash_key(key);
                let ring = self.hash_ring.read().unwrap();
                let mut seen = HashSet::new();
                seen.insert(primary);

                // Walk clockwise around ring to find unique nodes
                for (_, vnode) in ring.range(hash..) {
                    if seen.len() >= self.config.replication_factor {
                        break;
                    }
                    if !seen.contains(&vnode.node_id) {
                        replicas.push(vnode.node_id.clone());
                        seen.insert(vnode.node_id.clone());
                    }
                }

                // Wrap around if needed
                if seen.len() < self.config.replication_factor {
                    for (_, vnode) in ring.iter() {
                        if seen.len() >= self.config.replication_factor {
                            break;
                        }
                        if !seen.contains(&vnode.node_id) {
                            replicas.push(vnode.node_id.clone());
                            seen.insert(vnode.node_id.clone());
                        }
                    }
                }
            }
            HashStrategy::RangeBased => {
                let _hash = self.hash_key(key);
                let partitions = self.partitions.read().unwrap();

                for partition in partitions.iter() {
                    if partition.contains(hash) {
                        replicas.extend(partition.replicas.iter().cloned());
                        break;
                    }
                }
            }
            HashStrategy::RendezvousHash => {
                use std::collections::hash_map::DefaultHasher;
                let nodes = self.nodes.read().unwrap();
                let mut node_weights: Vec<(DhtNodeId, u64)> = Vec::new();

                for node_id in nodes.keys() {
                    let mut hasher = DefaultHasher::new();
                    key.hash(&mut hasher);
                    node_id.hash(&mut hasher);
                    node_weights.push((node_id.clone(), hasher.finish()));
                }

                node_weights.sort_by_key(|(_, w)| std::cmp::Reverse(*w));
                replicas = node_weights
                    .into_iter()
                    .take(self.config.replication_factor)
                    .map(|(id, _)| id)
                    .collect();
            }
        }

        Ok(replicas)
    }

    /// Update metrics for hot spot detection
    pub fn update_metrics(&self, shard_id: ShardId, rps: f64, data_size: u64, key_count: u64) {
        let mut metrics = self.metrics.write().unwrap();
        let metric = metrics.entry(shard_id).or_insert_with(|| HotSpotMetrics::new(shard_id));

        metric.requests_per_second = rps;
        metric.data_size = data_size;
        metric.key_count = key_count;
        metric.avg_key_size = if key_count > 0 { data_size / key_count } else { 0 };
        metric.measured_at = SystemTime::now();
    }

    /// Detect hot spots
    pub fn detect_hot_spots(&self) -> Vec<ShardId> {
        let metrics = self.metrics.read().unwrap();
        metrics
            .values()
            .filter(|m| m.is_hot_spot(
                self.config.hot_spot_rps_threshold,
                self.config.hot_spot_size_threshold,
            ))
            .map(|m| m.shard_id)
            .collect()
    }

    /// Split a hot partition
    pub fn split_hot_partition(&self, shard_id: ShardId) -> Result<(), DbError> {
        let mut partitions = self.partitions.write().unwrap();
        let nodes = self.nodes.read().unwrap();

        // Find partition to split
        if let Some(idx) = partitions.iter().position(|p| p.id == shard_id) {
            let new_node = self.find_least_loaded_node(&nodes, None)
                .ok_or_else(|| DbError::Internal("No available node for split".into()))?;

            let mut next_shard = self.next_shard_id.write().unwrap();
            self.split_partition(idx, new_node, *next_shard, &mut partitions)?;
            *next_shard += 1;
        }

        Ok(())
    }

    /// Split a partition at an index
    fn split_partition(
        &self,
        idx: usize,
        new_node: DhtNodeId,
        new_shard_id: ShardId,
        partitions: &mut Vec<RangePartition>,
    ) -> Result<(), DbError> {
        let partition = &mut partitions[idx];
        let mid = partition.start + partition.size() / 2;

        // Create new partition for second half
        let new_partition = RangePartition::new(new_shard_id, mid, partition.end, new_node);

        // Update original partition
        partition.end = mid;
        partition.last_split = Some(SystemTime::now());

        partitions.push(new_partition);
        Ok(())
    }

    /// Find largest partition
    fn find_largest_partition(&self, partitions: &[RangePartition]) -> Option<usize> {
        partitions
            .iter()
            .enumerate()
            .max_by_key(|(_, p)| p.size())
            .map(|(i, _)| i)
    }

    /// Find least loaded node
    fn find_least_loaded_node(
        &self,
        nodes: &HashMap<DhtNodeId, NodeMetadata>,
        exclude: Option<&str>,
    ) -> Option<DhtNodeId> {
        nodes
            .values()
            .filter(|n| exclude.map_or(true, |e| n.id != e))
            .min_by(|a, b| a.utilization().partial_cmp(&b.utilization()).unwrap())
            .map(|n| n.id.clone())
    }

    /// Trigger rebalancing
    pub fn trigger_rebalance(&self) -> Result<(), DbError> {
        // Detect hot spots
        let hot_spots = self.detect_hot_spots();

        for shard_id in hot_spots {
            self.split_hot_partition(shard_id)?;
        }

        Ok(())
    }

    /// Get all nodes
    pub fn get_nodes(&self) -> Vec<NodeMetadata> {
        self.nodes.read().unwrap().values().cloned().collect()
    }

    /// Get partition info
    pub fn get_partitions(&self) -> Vec<RangePartition> {
        self.partitions.read().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dht_creation() {
        let config = DhtConfig::default();
        let dht = DistributedHashTable::new(config);
        assert_eq!(dht.get_nodes().len(), 0);
    }

    #[test]
    fn test_add_node_consistent_hash() {
        let config = DhtConfig {
            strategy: HashStrategy::ConsistentHash,
            virtual_nodes_per_node: 10,
            ..Default::default()
        };
        let dht = DistributedHashTable::new(config);

        let metadata = NodeMetadata::new("node1".into(), "127.0.0.1".into(), 8080, 1000000);
        dht.add_node(metadata).unwrap();

        let nodes = dht.get_nodes();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].vnodes.len(), 10);
    }

    #[test]
    fn test_key_routing() {
        let config = DhtConfig::default();
        let dht = DistributedHashTable::new(config);

        let metadata = NodeMetadata::new("node1".into(), "127.0.0.1".into(), 8080, 1000000);
        dht.add_node(metadata).unwrap();

        let key = b"test_key";
        let node = dht.get_node_for_key(key).unwrap();
        assert_eq!(node, "node1");
    }

    #[test]
    fn test_hot_spot_detection() {
        let mut metrics = HotSpotMetrics::new(1);
        metrics.requests_per_second = 15000.0;
        metrics.data_size = 5 * 1024 * 1024 * 1024;

        assert!(metrics.is_hot_spot(10000.0, 10 * 1024 * 1024 * 1024));
    }
}


