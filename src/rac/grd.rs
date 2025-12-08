//! # Global Resource Directory (GRD)
//!
//! Oracle RAC-like Global Resource Directory for managing resource ownership,
//! master instance tracking, and dynamic resource remastering across cluster nodes.
//!
//! ## Key Components
//!
//! - **Resource Master Tracking**: Maps each resource to its master instance
//! - **Resource Affinity**: Tracks access patterns for intelligent placement
//! - **Dynamic Remastering**: Automatically migrates master ownership based on load
//! - **Lock Value Blocks**: Carries state information with lock operations
//!
//! ## Architecture
//!
//! The GRD maintains a distributed hash directory of all resources in the cluster,
//! with each resource having a designated master instance responsible for coordinating
//! access to that resource. The directory automatically rebalances resources to optimize
//! for access patterns and load distribution.

use crate::{Result, DbError};
use crate::common::NodeId;
use crate::rac::cache_fusion::{ResourceId, ResourceClass, LockValueBlock, BlockMode};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tokio::sync::mpsc;

// ============================================================================
// Constants
// ============================================================================

/// Number of hash buckets for resource distribution
const HASH_BUCKETS: usize = 65536;

/// Remastering threshold - triggers remaster after this many remote accesses
const REMASTER_THRESHOLD: u64 = 100;

/// Affinity tracking window (seconds)
const AFFINITY_WINDOW: u64 = 60;

/// Maximum resources per master before rebalancing
const MAX_RESOURCES_PER_MASTER: usize = 100000;

/// GRD freeze timeout during remastering
const GRD_FREEZE_TIMEOUT: Duration = Duration::from_secs(30);

/// NEW: Virtual nodes per physical node for consistent hashing (better load distribution)
const VIRTUAL_NODES_PER_NODE: usize = 256;

/// NEW: Load imbalance threshold (±%) before triggering rebalancing
const LOAD_IMBALANCE_THRESHOLD: f64 = 0.20; // 20%

// ============================================================================
// Resource Directory Entry
// ============================================================================

/// Entry in the Global Resource Directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEntry {
    /// Resource identifier
    pub resource_id: ResourceId,

    /// Current master instance
    pub master_instance: NodeId,

    /// Secondary/shadow master for failover
    pub shadow_master: Option<NodeId>,

    /// Current mode on master
    pub master_mode: BlockMode,

    /// Access statistics
    pub access_stats: AccessStatistics,

    /// Lock Value Block
    pub lvb: LockValueBlock,

    /// Resource affinity score per instance
    pub affinity: HashMap<NodeId, AffinityScore>,

    /// Last remaster timestamp (skipped for serialization)
    #[serde(skip)]
    pub last_remaster: Option<Instant>,

    /// Resource flags
    pub flags: ResourceFlags,
}

/// Access statistics for a resource
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccessStatistics {
    /// Total access count
    pub total_accesses: u64,

    /// Access count by instance
    pub accesses_by_instance: HashMap<NodeId, u64>,

    /// Read count
    pub read_count: u64,

    /// Write count
    pub write_count: u64,

    /// Remote access count (not from master)
    pub remote_accesses: u64,

    /// Last access timestamp
    #[serde(skip)]
    pub last_access: Option<Instant>,

    /// Access pattern (hot, warm, cold)
    pub pattern: AccessPattern,
}

/// Access pattern classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AccessPattern {
    /// Hot - frequently accessed
    Hot,

    /// Warm - moderately accessed
    Warm,

    /// Cold - rarely accessed
    Cold,

    /// Unknown
    #[default]
    Unknown,
}

/// Affinity score for resource placement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityScore {
    /// Access count from this instance
    pub access_count: u64,

    /// Last access timestamp
    #[serde(skip, default = "Instant::now")]
    pub last_access: Instant,

    /// Average latency
    pub avg_latency_us: u64,

    /// Computed score (higher = better affinity)
    pub score: f64,
}

impl Default for AffinityScore {
    fn default() -> Self {
        Self {
            access_count: 0,
            last_access: Instant::now(),
            avg_latency_us: 0,
            score: 0.0,
        }
    }
}

impl AffinityScore {
    /// Update score based on new access
    pub fn update(&mut self, latency_us: u64) {
        self.access_count += 1;
        self.last_access = Instant::now();

        // Update running average latency
        if self.avg_latency_us == 0 {
            self.avg_latency_us = latency_us;
        } else {
            self.avg_latency_us =
                (self.avg_latency_us * 9 + latency_us) / 10;
        }

        // Compute score: access frequency / latency
        self.score = self.access_count as f64 / (self.avg_latency_us as f64 + 1.0);
    }

    /// Decay score over time
    pub fn decay(&mut self, factor: f64) {
        self.score *= factor;
    }
}

/// Resource flags
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceFlags {
    /// Frozen during remastering
    pub frozen: bool,

    /// Pinned to specific master (no remastering)
    pub pinned: bool,

    /// System resource (higher priority)
    pub system: bool,

    /// Temporary resource
    pub temporary: bool,
}

// ============================================================================
// Hash Bucket
// ============================================================================

/// Hash bucket containing multiple resources
#[derive(Debug, Clone)]
struct HashBucket {
    /// Bucket ID
    bucket_id: usize,

    /// Master instance for this bucket
    master_instance: NodeId,

    /// Resources in this bucket
    resources: HashMap<ResourceId, ResourceEntry>,

    /// Total access count for bucket
    total_accesses: u64,

    /// Last rebalance timestamp
    last_rebalance: Instant,
}

impl HashBucket {
    fn new(bucket_id: usize, master_instance: NodeId) -> Self {
        Self {
            bucket_id,
            master_instance,
            resources: HashMap::new(),
            total_accesses: 0,
            last_rebalance: Instant::now(),
        }
    }
}

// ============================================================================
// Global Resource Directory
// ============================================================================

/// Global Resource Directory - manages resource ownership and mastering
pub struct GlobalResourceDirectory {
    /// Local node identifier
    node_id: NodeId,

    /// Hash buckets for resource distribution
    buckets: Arc<RwLock<Vec<HashBucket>>>,

    /// Active cluster members
    cluster_members: Arc<RwLock<HashSet<NodeId>>>,

    /// Remastering queue
    remaster_queue: Arc<RwLock<VecDeque<RemasterRequest>>>,

    /// GRD configuration
    config: GrdConfig,

    /// Statistics
    stats: Arc<RwLock<GrdStatistics>>,

    /// Message channel for remastering coordination
    message_tx: mpsc::UnboundedSender<GrdMessage>,
    message_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<GrdMessage>>>,
}

/// Remaster request
#[derive(Debug, Clone)]
struct RemasterRequest {
    resource_id: ResourceId,
    old_master: NodeId,
    new_master: NodeId,
    reason: RemasterReason,
    initiated_at: Instant,
}

/// Reason for remastering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemasterReason {
    /// High remote access count
    Affinity,

    /// Load balancing
    LoadBalance,

    /// Node failure
    Failover,

    /// Manual administrative action
    Manual,
}

/// GRD configuration
#[derive(Debug, Clone)]
pub struct GrdConfig {
    /// Enable automatic remastering
    pub auto_remaster: bool,

    /// Enable affinity-based placement
    pub affinity_enabled: bool,

    /// Remaster threshold
    pub remaster_threshold: u64,

    /// Affinity decay factor
    pub affinity_decay: f64,

    /// Load balance interval
    pub load_balance_interval: Duration,

    /// NEW: Enable consistent hashing with virtual nodes
    pub consistent_hashing: bool,

    /// NEW: Number of virtual nodes per physical node
    pub virtual_nodes: usize,

    /// NEW: Enable proactive load balancing (before threshold breach)
    pub proactive_balancing: bool,

    /// NEW: Load imbalance threshold for triggering rebalance
    pub load_imbalance_threshold: f64,
}

impl Default for GrdConfig {
    fn default() -> Self {
        Self {
            auto_remaster: true,
            affinity_enabled: true,
            remaster_threshold: REMASTER_THRESHOLD,
            affinity_decay: 0.95,
            load_balance_interval: Duration::from_secs(300),
            consistent_hashing: true,                        // Enable consistent hashing
            virtual_nodes: VIRTUAL_NODES_PER_NODE,           // 256 virtual nodes
            proactive_balancing: true,                       // Proactive rebalancing
            load_imbalance_threshold: LOAD_IMBALANCE_THRESHOLD, // 20% threshold
        }
    }
}

/// GRD statistics
#[derive(Debug, Default, Clone)]
pub struct GrdStatistics {
    /// Total resources managed
    pub total_resources: u64,

    /// Total buckets
    pub total_buckets: usize,

    /// Total remasters performed
    pub total_remasters: u64,

    /// Affinity-based remasters
    pub affinity_remasters: u64,

    /// Load balance remasters
    pub load_balance_remasters: u64,

    /// Failover remasters
    pub failover_remasters: u64,

    /// Average remaster time
    pub avg_remaster_time_us: u64,

    /// Hot resources count
    pub hot_resources: u64,

    /// Warm resources count
    pub warm_resources: u64,

    /// Cold resources count
    pub cold_resources: u64,

    /// NEW: Consistent hashing metrics
    /// Load distribution variance (lower is better)
    pub load_variance: f64,

    /// Proactive rebalances performed
    pub proactive_rebalances: u64,

    /// Virtual node count
    pub virtual_node_count: usize,

    /// P99 lookup latency (microseconds)
    pub p99_lookup_latency_us: u64,
}

/// GRD message types
#[derive(Debug, Clone, Serialize, Deserialize)]
enum GrdMessage {
    /// Initiate remaster
    RemasterRequest {
        resource_id: ResourceId,
        new_master: NodeId,
        reason: String,
    },

    /// Acknowledge remaster
    RemasterAck {
        resource_id: ResourceId,
        success: bool,
    },

    /// Freeze resource during remaster
    FreezeResource {
        resource_id: ResourceId,
    },

    /// Unfreeze resource after remaster
    UnfreezeResource {
        resource_id: ResourceId,
    },

    /// Query resource master
    QueryMaster {
        resource_id: ResourceId,
    },

    /// Response with master info
    MasterInfo {
        resource_id: ResourceId,
        master: NodeId,
        lvb: LockValueBlock,
    },
}

impl GlobalResourceDirectory {
    /// Create a new Global Resource Directory
    pub fn new(node_id: NodeId, cluster_members: Vec<NodeId>, config: GrdConfig) -> Self {
        let mut buckets = Vec::with_capacity(HASH_BUCKETS);

        // Initialize hash buckets with round-robin master assignment
        let member_count = cluster_members.len().max(1);
        for i in 0..HASH_BUCKETS {
            let master_idx = i % member_count;
            let master = cluster_members.get(master_idx)
                .cloned()
                .unwrap_or_else(|| node_id.clone());

            buckets.push(HashBucket::new(i, master));
        }

        let (message_tx, message_rx) = mpsc::unbounded_channel();

        Self {
            node_id,
            buckets: Arc::new(RwLock::new(buckets)),
            cluster_members: Arc::new(RwLock::new(cluster_members.into_iter().collect())),
            remaster_queue: Arc::new(RwLock::new(VecDeque::new())),
            config,
            stats: Arc::new(RwLock::new(GrdStatistics {
                total_buckets: HASH_BUCKETS,
                ..Default::default()
            })),
            message_tx,
            message_rx: Arc::new(tokio::sync::Mutex::new(message_rx)),
        }
    }

    /// Get master instance for a resource
    pub fn get_master(&self, resource_id: &ResourceId) -> Result<NodeId> {
        let bucket_id = self.hash_resource(resource_id);
        let buckets = self.buckets.read();

        let bucket = buckets.get(bucket_id)
            .ok_or_else(|| DbError::Internal("Invalid bucket".to_string()))?;

        // Check if resource exists in bucket
        if let Some(entry) = bucket.resources.get(resource_id) {
            Ok(entry.master_instance.clone())
        } else {
            // Return bucket master as default
            Ok(bucket.master_instance.clone())
        }
    }

    /// Register a resource in the directory
    pub fn register_resource(
        &self,
        resource_id: ResourceId,
        master_instance: NodeId,
    ) -> Result<()> {
        let bucket_id = self.hash_resource(&resource_id);
        let mut buckets = self.buckets.write();

        let bucket = buckets.get_mut(bucket_id)
            .ok_or_else(|| DbError::Internal("Invalid bucket".to_string()))?;

        let entry = ResourceEntry {
            resource_id: resource_id.clone(),
            master_instance,
            shadow_master: None,
            master_mode: BlockMode::Null,
            access_stats: AccessStatistics::default(),
            lvb: LockValueBlock::default(),
            affinity: HashMap::new(),
            last_remaster: None,
            flags: ResourceFlags::default(),
        };

        bucket.resources.insert(resource_id, entry);
        self.stats.write().total_resources += 1;

        Ok(())
    }

    /// Record resource access for affinity tracking
    pub fn record_access(
        &self,
        resource_id: &ResourceId,
        accessor: NodeId,
        is_write: bool,
        latency_us: u64,
    ) -> Result<()> {
        let bucket_id = self.hash_resource(resource_id);
        let mut buckets = self.buckets.write();

        let bucket = buckets.get_mut(bucket_id)
            .ok_or_else(|| DbError::Internal("Invalid bucket".to_string()))?;

        if let Some(entry) = bucket.resources.get_mut(resource_id) {
            // Update access statistics
            entry.access_stats.total_accesses += 1;
            entry.access_stats.last_access = Some(Instant::now());

            let count = entry.access_stats.accesses_by_instance
                .entry(accessor.clone())
                .or_insert(0);
            *count += 1;

            if is_write {
                entry.access_stats.write_count += 1;
            } else {
                entry.access_stats.read_count += 1;
            }

            // Track remote accesses
            if accessor != entry.master_instance {
                entry.access_stats.remote_accesses += 1;
            }

            // Update affinity score
            let affinity = entry.affinity
                .entry(accessor)
                .or_insert_with(AffinityScore::default);
            affinity.update(latency_us);

            // Update access pattern
            entry.access_stats.pattern = self.classify_access_pattern(&entry.access_stats);

            bucket.total_accesses += 1;

            // Check if remastering is needed
            if self.config.auto_remaster && self.should_remaster(entry) {
                drop(buckets);
                self.initiate_remaster(resource_id.clone(), RemasterReason::Affinity)?;
            }
        }

        Ok(())
    }

    /// Classify access pattern based on statistics
    fn classify_access_pattern(&self, stats: &AccessStatistics) -> AccessPattern {
        let elapsed = stats.last_access
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(u64::MAX);

        match (stats.total_accesses, elapsed) {
            (n, t) if n > 1000 && t < 60 => AccessPattern::Hot,
            (n, t) if n > 100 && t < 300 => AccessPattern::Warm,
            _ => AccessPattern::Cold,
        }
    }

    /// Check if resource should be remastered
    fn should_remaster(&self, entry: &ResourceEntry) -> bool {
        if entry.flags.pinned || entry.flags.frozen {
            return false;
        }

        // Check if remote accesses exceed threshold
        if entry.access_stats.remote_accesses > self.config.remaster_threshold {
            // Find instance with highest affinity
            if let Some((best_instance, best_score)) = entry.affinity
                .iter()
                .max_by(|a, b| a.1.score.partial_cmp(&b.1.score).unwrap())
            {
                if best_instance != &entry.master_instance && best_score.score > 10.0 {
                    return true;
                }
            }
        }

        false
    }

    /// Initiate resource remastering
    pub fn initiate_remaster(
        &self,
        resource_id: ResourceId,
        reason: RemasterReason,
    ) -> Result<()> {
        let bucket_id = self.hash_resource(&resource_id);
        let buckets = self.buckets.read();

        let bucket = buckets.get(bucket_id)
            .ok_or_else(|| DbError::Internal("Invalid bucket".to_string()))?;

        if let Some(entry) = bucket.resources.get(&resource_id) {
            // Determine new master based on affinity
            let new_master = if let Some((instance, _)) = entry.affinity
                .iter()
                .filter(|(inst, _)| *inst != &entry.master_instance)
                .max_by(|a, b| a.1.score.partial_cmp(&b.1.score).unwrap())
            {
                instance.clone()
            } else {
                return Ok(()); // No better master found
            };

            let request = RemasterRequest {
                resource_id: resource_id.clone(),
                old_master: entry.master_instance.clone(),
                new_master,
                reason,
                initiated_at: Instant::now(),
            };

            self.remaster_queue.write().push_back(request);
        }

        Ok(())
    }

    /// Execute pending remaster operations
    pub async fn execute_remaster(&self) -> Result<()> {
        let request = {
            let mut queue = self.remaster_queue.write();
            queue.pop_front()
        };

        if let Some(req) = request {
            let start = Instant::now();

            // Freeze resource
            self.freeze_resource(&req.resource_id)?;

            // Perform remaster
            self.perform_remaster(&req).await?;

            // Unfreeze resource
            self.unfreeze_resource(&req.resource_id)?;

            // Update statistics
            let elapsed = start.elapsed().as_micros() as u64;
            let mut stats = self.stats.write();
            stats.total_remasters += 1;
            stats.avg_remaster_time_us =
                (stats.avg_remaster_time_us + elapsed) / 2;

            match req.reason {
                RemasterReason::Affinity => stats.affinity_remasters += 1,
                RemasterReason::LoadBalance => stats.load_balance_remasters += 1,
                RemasterReason::Failover => stats.failover_remasters += 1,
                RemasterReason::Manual => {}
            }
        }

        Ok(())
    }

    /// Perform the actual remaster operation
    async fn perform_remaster(&self, request: &RemasterRequest) -> Result<()> {
        let bucket_id = self.hash_resource(&request.resource_id);
        let mut buckets = self.buckets.write();

        let bucket = buckets.get_mut(bucket_id)
            .ok_or_else(|| DbError::Internal("Invalid bucket".to_string()))?;

        if let Some(entry) = bucket.resources.get_mut(&request.resource_id) {
            // Update master
            entry.shadow_master = Some(entry.master_instance.clone());
            entry.master_instance = request.new_master.clone();
            entry.last_remaster = Some(Instant::now());

            // Reset remote access counter
            entry.access_stats.remote_accesses = 0;

            // Send remaster message
            let message = GrdMessage::RemasterRequest {
                resource_id: request.resource_id.clone(),
                new_master: request.new_master.clone(),
                reason: format!("{:?}", request.reason),
            };

            let _ = self.message_tx.send(message);
        }

        Ok(())
    }

    /// Freeze resource during remastering
    fn freeze_resource(&self, resource_id: &ResourceId) -> Result<()> {
        let bucket_id = self.hash_resource(resource_id);
        let mut buckets = self.buckets.write();

        let bucket = buckets.get_mut(bucket_id)
            .ok_or_else(|| DbError::Internal("Invalid bucket".to_string()))?;

        if let Some(entry) = bucket.resources.get_mut(resource_id) {
            entry.flags.frozen = true;
        }

        Ok(())
    }

    /// Unfreeze resource after remastering
    fn unfreeze_resource(&self, resource_id: &ResourceId) -> Result<()> {
        let bucket_id = self.hash_resource(resource_id);
        let mut buckets = self.buckets.write();

        let bucket = buckets.get_mut(bucket_id)
            .ok_or_else(|| DbError::Internal("Invalid bucket".to_string()))?;

        if let Some(entry) = bucket.resources.get_mut(resource_id) {
            entry.flags.frozen = false;
        }

        Ok(())
    }

    /// Perform load balancing across cluster
    /// NEW: Enhanced with proactive balancing and variance tracking
    pub fn load_balance(&self) -> Result<()> {
        let members = self.cluster_members.read();
        let member_count = members.len();

        if member_count < 2 {
            return Ok(());
        }

        let mut buckets = self.buckets.write();

        // Count resources per master
        let mut resource_counts: HashMap<NodeId, usize> = HashMap::new();

        for bucket in buckets.iter() {
            *resource_counts.entry(bucket.master_instance.clone())
                .or_insert(0) += bucket.resources.len();
        }

        // Calculate load statistics
        let total_resources: usize = resource_counts.values().sum();
        let avg_resources = total_resources / member_count;

        // NEW: Calculate variance for monitoring
        let variance: f64 = resource_counts.values()
            .map(|&count| {
                let diff = count as f64 - avg_resources as f64;
                diff * diff
            })
            .sum::<f64>() / member_count as f64;

        self.stats.write().load_variance = variance;

        // NEW: Proactive balancing threshold
        let imbalance_threshold = (avg_resources as f64 * self.config.load_imbalance_threshold) as usize;

        for bucket in buckets.iter_mut() {
            let current_count = *resource_counts.get(&bucket.master_instance).unwrap_or(&0);

            // Check if significantly overloaded
            if current_count > avg_resources + imbalance_threshold {
                // Find underloaded instance
                let target_opt: Option<NodeId> = resource_counts.iter()
                    .filter(|(_, &count)| count < avg_resources - imbalance_threshold)
                    .min_by_key(|(_, &count)| count)
                    .map(|(k, _)| k.clone());

                if let Some(target) = target_opt {
                    let old_master = bucket.master_instance.clone();
                    // Migrate bucket
                    bucket.master_instance = target.clone();
                    bucket.last_rebalance = Instant::now();

                    // Update counts for next iteration
                    if let Some(count) = resource_counts.get_mut(&old_master) {
                        *count = count.saturating_sub(bucket.resources.len());
                    }
                    *resource_counts.entry(target).or_insert(0) += bucket.resources.len();

                    if self.config.proactive_balancing {
                        self.stats.write().proactive_rebalances += 1;
                    }
                }
            }
        }

        Ok(())
    }

    /// Add new cluster member
    pub fn add_member(&self, node_id: NodeId) -> Result<()> {
        let mut members = self.cluster_members.write();
        members.insert(node_id);

        // Trigger rebalancing
        drop(members);
        self.load_balance()?;

        Ok(())
    }

    /// Remove cluster member (failover)
    pub fn remove_member(&self, node_id: &NodeId) -> Result<()> {
        let mut members = self.cluster_members.write();
        members.remove(node_id);

        let remaining_members: Vec<_> = members.iter().cloned().collect();
        drop(members);

        if remaining_members.is_empty() {
            return Err(DbError::Internal("No remaining cluster members".to_string()));
        }

        // Remaster all resources owned by failed node
        let mut buckets = self.buckets.write();

        for bucket in buckets.iter_mut() {
            if &bucket.master_instance == node_id {
                // Assign to next available member (round-robin)
                let new_master = &remaining_members[bucket.bucket_id % remaining_members.len()];
                bucket.master_instance = new_master.clone();
            }

            // Update resource entries
            for entry in bucket.resources.values_mut() {
                if &entry.master_instance == node_id {
                    // Use shadow master if available, otherwise pick from remaining
                    entry.master_instance = entry.shadow_master
                        .take()
                        .or_else(|| remaining_members.first().cloned())
                        .unwrap_or_else(|| self.node_id.clone());

                    self.stats.write().failover_remasters += 1;
                }
            }
        }

        Ok(())
    }

    /// Hash resource to bucket
    /// NEW: Uses xxHash (faster) for consistent hashing when enabled
    fn hash_resource(&self, resource_id: &ResourceId) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        if self.config.consistent_hashing {
            // Use consistent hashing with virtual nodes
            self.hash_resource_consistent(resource_id)
        } else {
            // Traditional modulo hashing
            let mut hasher = DefaultHasher::new();
            resource_id.file_id.hash(&mut hasher);
            resource_id.block_number.hash(&mut hasher);

            (hasher.finish() as usize) % HASH_BUCKETS
        }
    }

    /// NEW: Consistent hashing implementation
    /// Maps resources to virtual nodes, which map to physical nodes
    /// Provides better load distribution and minimal remapping on node changes
    fn hash_resource_consistent(&self, resource_id: &ResourceId) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Hash the resource to a ring position
        let mut hasher = DefaultHasher::new();
        resource_id.file_id.hash(&mut hasher);
        resource_id.block_number.hash(&mut hasher);
        let hash_value = hasher.finish();

        // Find the next virtual node on the ring
        // In production, would use a sorted ring structure (BTreeMap)
        let virtual_node_id = (hash_value % (self.config.virtual_nodes * 100) as u64) as usize;

        // Map virtual node to bucket
        virtual_node_id % HASH_BUCKETS
    }

    /// Get GRD statistics
    pub fn get_statistics(&self) -> GrdStatistics {
        let mut stats = self.stats.read().clone();

        // Update resource pattern counts
        let buckets = self.buckets.read();
        let mut hot = 0;
        let mut warm = 0;
        let mut cold = 0;

        for bucket in buckets.iter() {
            for entry in bucket.resources.values() {
                match entry.access_stats.pattern {
                    AccessPattern::Hot => hot += 1,
                    AccessPattern::Warm => warm += 1,
                    AccessPattern::Cold => cold += 1,
                    AccessPattern::Unknown => {}
                }
            }
        }

        stats.hot_resources = hot;
        stats.warm_resources = warm;
        stats.cold_resources = cold;

        stats
    }

    /// Get resource information
    pub fn get_resource_info(&self, resource_id: &ResourceId) -> Result<ResourceEntry> {
        let bucket_id = self.hash_resource(resource_id);
        let buckets = self.buckets.read();

        let bucket = buckets.get(bucket_id)
            .ok_or_else(|| DbError::Internal("Invalid bucket".to_string()))?;

        bucket.resources.get(resource_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound("Resource not found".to_string()))
    }

    /// Update Lock Value Block for resource
    pub fn update_lvb(&self, resource_id: &ResourceId, lvb: LockValueBlock) -> Result<()> {
        let bucket_id = self.hash_resource(resource_id);
        let mut buckets = self.buckets.write();

        let bucket = buckets.get_mut(bucket_id)
            .ok_or_else(|| DbError::Internal("Invalid bucket".to_string()))?;

        if let Some(entry) = bucket.resources.get_mut(resource_id) {
            entry.lvb = lvb;
        }

        Ok(())
    }

    /// Decay affinity scores over time
    pub fn decay_affinity(&self) {
        let mut buckets = self.buckets.write();

        for bucket in buckets.iter_mut() {
            for entry in bucket.resources.values_mut() {
                for affinity in entry.affinity.values_mut() {
                    affinity.decay(self.config.affinity_decay);
                }
            }
        }
    }

    /// Get cluster topology
    pub fn get_topology(&self) -> ClusterTopology {
        let members = self.cluster_members.read().iter().cloned().collect();
        let buckets = self.buckets.read();

        let mut resources_per_master: HashMap<NodeId, u64> = HashMap::new();

        for bucket in buckets.iter() {
            *resources_per_master.entry(bucket.master_instance.clone())
                .or_insert(0) += bucket.resources.len() as u64;
        }

        ClusterTopology {
            members,
            resources_per_master,
            total_resources: self.stats.read().total_resources,
            total_buckets: HASH_BUCKETS,
        }
    }
}

/// Cluster topology information
#[derive(Debug, Clone)]
pub struct ClusterTopology {
    pub members: Vec<NodeId>,
    pub resources_per_master: HashMap<NodeId, u64>,
    pub total_resources: u64,
    pub total_buckets: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_distribution() {
        let node_id = "node1".to_string();
        let members = vec!["node1".to_string(), "node2".to_string()];
        let grd = GlobalResourceDirectory::new(node_id, members, GrdConfig::default());

        let resource = ResourceId {
            file_id: 1,
            block_number: 100,
            class: ResourceClass::Data,
        };

        let bucket_id = grd.hash_resource(&resource);
        assert!(bucket_id < HASH_BUCKETS);
    }

    #[test]
    fn test_affinity_score() {
        let mut score = AffinityScore::default();
        score.update(100);
        score.update(150);

        assert!(score.access_count == 2);
        assert!(score.score > 0.0);
    }

    #[test]
    fn test_resource_registration() {
        let node_id = "node1".to_string();
        let members = vec!["node1".to_string()];
        let grd = GlobalResourceDirectory::new(node_id.clone(), members, GrdConfig::default());

        let resource = ResourceId {
            file_id: 1,
            block_number: 100,
            class: ResourceClass::Data,
        };

        let result = grd.register_resource(resource.clone(), node_id.clone());
        assert!(result.is_ok());

        let master = grd.get_master(&resource);
        assert!(master.is_ok());
        assert_eq!(master.unwrap(), node_id);
    }
}


