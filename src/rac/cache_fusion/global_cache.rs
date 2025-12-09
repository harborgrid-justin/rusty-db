// # Global Cache Service
//
// Oracle RAC-like Global Cache Service (GCS) for direct memory-to-memory block transfers
// between cluster instances without requiring disk I/O.
//
// ## Key Components
//
// - **Block Mode Management**: Coordinates data block access modes (Shared, Exclusive, etc.)
// - **Block Transfer Engine**: Zero-copy RDMA-like transfers between nodes
// - **Resource Directory**: Maps resources to master instances
// - **Local Cache Management**: Tracks local cache states and pending requests

use tokio::sync::oneshot;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Instant;
use std::collections::HashSet;
use crate::error::DbError;
use crate::common::{PageId, TransactionId, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::Arc;
use std::time::{Duration};
use parking_lot::{RwLock};
use tokio::sync::{mpsc};

// ============================================================================
// Constants
// ============================================================================

/// Maximum block size for cache fusion transfers (default 8KB)
pub const MAX_BLOCK_SIZE: usize = 8192;

/// Maximum number of concurrent block transfers
pub const MAX_CONCURRENT_TRANSFERS: usize = 1024;

/// Block transfer timeout
pub const TRANSFER_TIMEOUT: Duration = Duration::from_secs(5);

/// GCS message timeout
pub const GCS_MESSAGE_TIMEOUT: Duration = Duration::from_millis(500);

// ============================================================================
// Block Mode and Resource Types
// ============================================================================

/// Cache Fusion block access mode (similar to Oracle's cache coherency modes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockMode {
    /// Null mode - no access rights
    Null,

    /// Shared mode - read-only access, multiple instances can hold
    Shared,

    /// Exclusive mode - write access, only one instance can hold
    Exclusive,

    /// Shared Current - shared read with potential for upgrade
    SharedCurrent,

    /// Exclusive Current - current version for modifications
    ExclusiveCurrent,

    /// Past Image - historical version for consistency
    PastImage,
}

impl BlockMode {
    /// Check if mode allows read access
    pub fn can_read(&self) -> bool {
        !matches!(self, BlockMode::Null)
    }

    /// Check if mode allows write access
    pub fn can_write(&self) -> bool {
        matches!(self, BlockMode::Exclusive | BlockMode::ExclusiveCurrent)
    }

    /// Check if mode is compatible with another mode
    pub fn is_compatible(&self, other: &BlockMode) -> bool {
        match (self, other) {
            (BlockMode::Null, _) | (_, BlockMode::Null) => true,
            (BlockMode::Shared, BlockMode::Shared) => true,
            (BlockMode::SharedCurrent, BlockMode::Shared) => true,
            (BlockMode::Shared, BlockMode::SharedCurrent) => true,
            (BlockMode::PastImage, _) | (_, BlockMode::PastImage) => true,
            _ => false,
        }
    }

    /// Get the downgrade mode when transferring to another instance
    pub fn downgrade_for_transfer(&self) -> BlockMode {
        match self {
            BlockMode::Exclusive | BlockMode::ExclusiveCurrent => BlockMode::Shared,
            _ => *self,
        }
    }

    /// Get priority for block acquisition (higher = more priority)
    pub fn priority(&self) -> u8 {
        match self {
            BlockMode::Null => 0,
            BlockMode::PastImage => 1,
            BlockMode::Shared => 2,
            BlockMode::SharedCurrent => 3,
            BlockMode::Exclusive => 4,
            BlockMode::ExclusiveCurrent => 5,
        }
    }
}

/// Global cache resource identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceId {
    /// Data file identifier
    pub file_id: u32,

    /// Block number within file
    pub block_number: PageId,

    /// Class of resource (data, undo, temp, etc.)
    pub class: ResourceClass,
}

/// Resource class for different types of blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceClass {
    /// Data block
    Data,

    /// Index block
    Index,

    /// Undo block
    Undo,

    /// Temp block
    Temp,

    /// System block
    System,
}

/// Block state in the global cache
#[derive(Debug, Clone)]
pub struct BlockState {
    /// Resource identifier
    pub resource_id: ResourceId,

    /// Current access mode
    pub mode: BlockMode,

    /// Current lock holder instance
    pub holder: Option<NodeId>,

    /// Past image holders for read consistency
    pub past_image_holders: HashSet<NodeId>,

    /// Modification SCN (System Change Number)
    pub scn: u64,

    /// Lock Value Block - carries state information
    pub lvb: LockValueBlock,

    /// Timestamp of last access
    pub last_access: Instant,

    /// Number of transfers for this block
    pub transfer_count: u64,
}

/// Lock Value Block - carries state with lock grants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockValueBlock {
    /// Most recent SCN
    pub current_scn: u64,

    /// Master instance for this resource
    pub master_instance: NodeId,

    /// Dirty flag
    pub is_dirty: bool,

    /// Version number for optimistic locking
    pub version: u64,

    /// Custom metadata
    pub metadata: Vec<u8>,
}

impl Default for LockValueBlock {
    fn default() -> Self {
        Self {
            current_scn: 0,
            master_instance: String::new(),
            is_dirty: false,
            version: 0,
            metadata: Vec::new(),
        }
    }
}

// ============================================================================
// Cache Fusion Messages
// ============================================================================

/// Messages exchanged in the Cache Fusion protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheFusionMessage {
    /// Request block in specific mode
    BlockRequest {
        resource_id: ResourceId,
        requested_mode: BlockMode,
        requestor: NodeId,
        transaction_id: TransactionId,
        force_current: bool,
    },

    /// Grant block access
    BlockGrant {
        resource_id: ResourceId,
        granted_mode: BlockMode,
        block_data: Option<Vec<u8>>,
        lvb: LockValueBlock,
        needs_write_back: bool,
    },

    /// Transfer block from one instance to another
    BlockTransfer {
        resource_id: ResourceId,
        block_data: Vec<u8>,
        source_mode: BlockMode,
        target_mode: BlockMode,
        scn: u64,
    },

    /// Notify block has been modified
    BlockInvalidate {
        resource_id: ResourceId,
        new_scn: u64,
        invalidator: NodeId,
    },

    /// Downgrade block mode
    BlockDowngrade {
        resource_id: ResourceId,
        from_mode: BlockMode,
        to_mode: BlockMode,
    },

    /// Request past image for read consistency
    PastImageRequest {
        resource_id: ResourceId,
        as_of_scn: u64,
        requestor: NodeId,
    },

    /// Deliver past image
    PastImageResponse {
        resource_id: ResourceId,
        block_data: Vec<u8>,
        scn: u64,
    },

    /// Write back dirty block to disk
    WriteBackRequest {
        resource_id: ResourceId,
        coordinator: NodeId,
    },

    /// Confirm write back completed
    WriteBackComplete {
        resource_id: ResourceId,
        success: bool,
    },
}

// ============================================================================
// Global Cache Service (GCS)
// ============================================================================

/// Global Cache Service - coordinates block sharing across cluster instances
pub struct GlobalCacheService {
    /// Local node identifier
    node_id: NodeId,

    /// Global resource directory (resource -> master mapping)
    resource_directory: Arc<RwLock<HashMap<ResourceId, NodeId>>>,

    /// Local block cache states
    local_cache: Arc<RwLock<HashMap<ResourceId, BlockState>>>,

    /// Pending block requests
    pending_requests: Arc<Mutex<HashMap<ResourceId, Vec<BlockRequest>>>>,

    /// Message sender for inter-node communication
    message_tx: mpsc::UnboundedSender<(NodeId, CacheFusionMessage)>,

    /// Message receiver
    message_rx: Arc<Mutex<mpsc::UnboundedReceiver<(NodeId, CacheFusionMessage)>>>,

    /// Block transfer statistics
    stats: Arc<RwLock<GcsStatistics>>,

    /// Configuration
    config: GcsConfig,
}

/// Block request tracking
#[derive(Debug)]
struct BlockRequest {
    resource_id: ResourceId,
    requested_mode: BlockMode,
    requestor: NodeId,
    transaction_id: TransactionId,
    timestamp: Instant,
    response_tx: oneshot::Sender<Result<BlockGrant, DbError>>,
}

/// Block grant response
#[derive(Debug, Clone)]
pub struct BlockGrant {
    pub resource_id: ResourceId,
    pub granted_mode: BlockMode,
    pub block_data: Option<Vec<u8>>,
    pub lvb: LockValueBlock,
    pub needs_write_back: bool,
}

/// GCS configuration
#[derive(Debug, Clone)]
pub struct GcsConfig {
    /// Enable zero-copy transfers
    pub enable_zero_copy: bool,

    /// Enable predictive prefetching
    pub enable_prefetch: bool,

    /// Maximum retry attempts for block requests
    pub max_retries: u32,

    /// Adaptive mode switching threshold
    pub adaptive_threshold: usize,

    /// Message batching window (milliseconds) - NEW: Batch requests for efficiency
    pub batch_window_ms: u64,

    /// Maximum messages per batch - NEW: Limit batch size
    pub batch_size: usize,

    /// Enable work-stealing for parallel operations - NEW
    pub enable_work_stealing: bool,

    /// Speculation threshold for slow operations (std deviations) - NEW
    pub speculation_threshold: f64,
}

impl Default for GcsConfig {
    fn default() -> Self {
        Self {
            enable_zero_copy: true,
            enable_prefetch: true,
            max_retries: 3,
            adaptive_threshold: 100,
            batch_window_ms: 1,          // 1ms batching window for optimal latency/throughput
            batch_size: 64,               // Batch up to 64 requests
            enable_work_stealing: true,   // Enable work stealing for load balancing
            speculation_threshold: 2.0,   // Speculate after 2 standard deviations
        }
    }
}

/// GCS statistics
#[derive(Debug, Default, Clone)]
pub struct GcsStatistics {
    /// Total block requests
    pub total_requests: u64,

    /// Successful block grants
    pub successful_grants: u64,

    /// Failed requests
    pub failed_requests: u64,

    /// Cache hits (local cache)
    pub cache_hits: u64,

    /// Cache misses requiring network transfer
    pub cache_misses: u64,

    /// Total bytes transferred
    pub bytes_transferred: u64,

    /// Average transfer latency
    pub avg_transfer_latency_us: u64,

    /// Number of write-backs to disk
    pub write_backs: u64,

    /// Number of downgrades
    pub downgrades: u64,

    /// Number of past image requests
    pub past_image_requests: u64,

    // Advanced metrics for 100+ node clusters
    /// Batched requests count
    pub batched_requests: u64,

    /// Prefetch hits (predicted correctly)
    pub prefetch_hits: u64,

    /// Prefetch misses (wrong prediction)
    pub prefetch_misses: u64,

    /// Deadlocks detected and resolved
    pub deadlocks_detected: u64,

    /// Work stealing operations
    pub work_steals: u64,

    /// Speculative executions
    pub speculative_executions: u64,

    /// P99 latency in microseconds
    pub p99_latency_us: u64,
}

impl GlobalCacheService {
    /// Create a new Global Cache Service instance
    pub fn new(node_id: NodeId, config: GcsConfig) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        Self {
            node_id,
            resource_directory: Arc::new(RwLock::new(HashMap::new())),
            local_cache: Arc::new(RwLock::new(HashMap::new())),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            message_tx,
            message_rx: Arc::new(Mutex::new(message_rx)),
            stats: Arc::new(RwLock::new(GcsStatistics::default())),
            config,
        }
    }

    /// Request a block in specific mode (main entry point for cache fusion)
    pub async fn request_block(
        &self,
        resource_id: ResourceId,
        mode: BlockMode,
        transaction_id: TransactionId,
        force_current: bool,
    ) -> Result<BlockGrant, DbError> {
        // Update statistics
        self.stats.write().total_requests += 1;

        // Check local cache first
        if let Some(grant) = self.check_local_cache(&resource_id, mode).await? {
            self.stats.write().cache_hits += 1;
            return Ok(grant);
        }

        self.stats.write().cache_misses += 1;

        // Determine master instance for this resource
        let master = self.get_master_instance(&resource_id).await?;

        // If we are the master, handle locally
        if master == self.node_id {
            return self.handle_local_block_request(resource_id, mode, transaction_id).await;
        }

        // Send request to master instance
        let (response_tx, response_rx) = oneshot::channel();

        let request = BlockRequest {
            resource_id: resource_id.clone(),
            requested_mode: mode,
            requestor: self.node_id.clone(),
            transaction_id,
            timestamp: Instant::now(),
            response_tx,
        };

        // Track pending request
        self.pending_requests
            .lock()
            .unwrap()
            .entry(resource_id.clone())
            .or_insert_with(Vec::new)
            .push(request);

        // Send message to master
        let message = CacheFusionMessage::BlockRequest {
            resource_id,
            requested_mode: mode,
            requestor: self.node_id.clone(),
            transaction_id,
            force_current,
        };

        self.message_tx
            .send((master, message))
            .map_err(|_| DbError::Internal("Failed to send block request".to_string()))?;

        // Wait for response with timeout
        match tokio::time::timeout(TRANSFER_TIMEOUT, response_rx).await {
            Ok(Ok(Ok(grant))) => {
                self.stats.write().successful_grants += 1;
                Ok(grant)
            }
            Ok(Ok(Err(e))) => {
                self.stats.write().failed_requests += 1;
                Err(e)
            }
            Ok(Err(_)) => {
                self.stats.write().failed_requests += 1;
                Err(DbError::Internal("Block request channel closed".to_string()))
            }
            Err(_) => {
                self.stats.write().failed_requests += 1;
                Err(DbError::LockTimeout)
            }
        }
    }

    /// Check local cache for block availability
    async fn check_local_cache(
        &self,
        resource_id: &ResourceId,
        requested_mode: BlockMode,
    ) -> Result<Option<BlockGrant>, DbError> {
        let cache = self.local_cache.read();

        if let Some(state) = cache.get(resource_id) {
            // Check if current mode is compatible
            if state.mode.is_compatible(&requested_mode) ||
               state.mode.priority() >= requested_mode.priority() {
                return Ok(Some(BlockGrant {
                    resource_id: resource_id.clone(),
                    granted_mode: state.mode,
                    block_data: None, // Data already in local cache
                    lvb: state.lvb.clone(),
                    needs_write_back: false,
                }));
            }
        }

        Ok(None)
    }

    /// Handle block request locally (when we are the master)
    async fn handle_local_block_request(
        &self,
        resource_id: ResourceId,
        mode: BlockMode,
        transaction_id: TransactionId,
    ) -> Result<BlockGrant, DbError> {
        let mut cache = self.local_cache.write();

        // Get or create block state
        let state = cache.entry(resource_id.clone()).or_insert_with(|| {
            BlockState {
                resource_id: resource_id.clone(),
                mode: BlockMode::Null,
                holder: None,
                past_image_holders: HashSet::new(),
                scn: 0,
                lvb: LockValueBlock::default(),
                last_access: Instant::now(),
                transfer_count: 0,
            }
        });

        // Check if mode upgrade is needed
        if state.mode == BlockMode::Null || mode.priority() > state.mode.priority() {
            // Upgrade mode
            state.mode = mode;
            state.holder = Some(self.node_id.clone());
            state.scn += 1;
            state.lvb.current_scn = state.scn;
            state.last_access = Instant::now();
        }

        Ok(BlockGrant {
            resource_id,
            granted_mode: state.mode,
            block_data: Some(vec![0; MAX_BLOCK_SIZE]), // Simulated block data
            lvb: state.lvb.clone(),
            needs_write_back: state.lvb.is_dirty,
        })
    }

    /// Transfer block to another instance (RDMA-like zero-copy)
    pub async fn transfer_block(
        &self,
        resource_id: ResourceId,
        target_node: NodeId,
        block_data: Vec<u8>,
        source_mode: BlockMode,
        target_mode: BlockMode,
    ) -> Result<(), DbError> {
        let start = Instant::now();

        // Update local cache state
        {
            let mut cache = self.local_cache.write();
            if let Some(state) = cache.get_mut(&resource_id) {
                // Downgrade our mode after transfer
                state.mode = source_mode.downgrade_for_transfer();
                state.transfer_count += 1;
            }
        }

        // Create transfer message
        let message = CacheFusionMessage::BlockTransfer {
            resource_id: resource_id.clone(),
            block_data,
            source_mode,
            target_mode,
            scn: self.local_cache.read()
                .get(&resource_id)
                .map(|s| s.scn)
                .unwrap_or(0),
        };

        // Send zero-copy transfer (in production, this would use RDMA)
        self.message_tx
            .send((target_node, message))
            .map_err(|_| DbError::Internal("Failed to send block transfer".to_string()))?;

        // Update statistics
        let elapsed = start.elapsed().as_micros() as u64;
        let mut stats = self.stats.write();
        stats.bytes_transferred += MAX_BLOCK_SIZE as u64;
        stats.avg_transfer_latency_us =
            (stats.avg_transfer_latency_us + elapsed) / 2;

        Ok(())
    }

    /// Request past image for read consistency
    pub async fn request_past_image(
        &self,
        resource_id: ResourceId,
        as_of_scn: u64,
    ) -> Result<Vec<u8>, DbError> {
        self.stats.write().past_image_requests += 1;

        // Determine which instance has the past image
        let holder = self.find_past_image_holder(&resource_id, as_of_scn).await?;

        // Request past image
        let message = CacheFusionMessage::PastImageRequest {
            resource_id: resource_id.clone(),
            as_of_scn,
            requestor: self.node_id.clone(),
        };

        self.message_tx
            .send((holder, message))
            .map_err(|_| DbError::Internal("Failed to send past image request".to_string()))?;

        // In production, would wait for response
        // For now, return simulated past image
        Ok(vec![0; MAX_BLOCK_SIZE])
    }

    /// Invalidate block across all instances
    pub async fn invalidate_block(&self, resource_id: ResourceId, new_scn: u64) -> Result<(), DbError> {
        let message = CacheFusionMessage::BlockInvalidate {
            resource_id: resource_id.clone(),
            new_scn,
            invalidator: self.node_id.clone(),
        };

        // Broadcast to all instances (simplified - in production would use GRD)
        // For now, just update local cache
        let mut cache = self.local_cache.write();
        if let Some(state) = cache.get_mut(&resource_id) {
            state.scn = new_scn;
            state.lvb.current_scn = new_scn;
        }

        Ok(())
    }

    /// Write back dirty block to disk
    pub async fn write_back_block(&self, resource_id: ResourceId) -> Result<(), DbError> {
        self.stats.write().write_backs += 1;

        let mut cache = self.local_cache.write();
        if let Some(state) = cache.get_mut(&resource_id) {
            // Mark as clean after write-back
            state.lvb.is_dirty = false;

            // In production, would actually write to disk
            // For now, just simulate
        }

        Ok(())
    }

    /// Get master instance for a resource
    async fn get_master_instance(&self, resource_id: &ResourceId) -> Result<NodeId, DbError> {
        let dir = self.resource_directory.read();

        if let Some(master) = dir.get(resource_id) {
            Ok(master.clone())
        } else {
            // Default to local instance if not in directory
            Ok(self.node_id.clone())
        }
    }

    /// Find instance holding past image
    async fn find_past_image_holder(
        &self,
        resource_id: &ResourceId,
        _as_of_scn: u64,
    ) -> Result<NodeId, DbError> {
        let cache = self.local_cache.read();

        if let Some(state) = cache.get(resource_id) {
            if let Some(holder) = &state.holder {
                return Ok(holder.clone());
            }
        }

        Ok(self.node_id.clone())
    }

    /// Get GCS statistics
    pub fn get_statistics(&self) -> GcsStatistics {
        self.stats.read().clone()
    }

    /// Process incoming cache fusion message
    pub async fn process_message(
        &self,
        source: NodeId,
        message: CacheFusionMessage,
    ) -> Result<(), DbError> {
        match message {
            CacheFusionMessage::BlockRequest {
                resource_id,
                requested_mode,
                requestor,
                transaction_id,
                force_current,
            } => {
                self.handle_block_request(
                    resource_id,
                    requested_mode,
                    requestor,
                    transaction_id,
                    force_current,
                ).await
            }

            CacheFusionMessage::BlockTransfer {
                resource_id,
                block_data,
                source_mode,
                target_mode,
                scn,
            } => {
                self.handle_block_transfer(
                    resource_id,
                    block_data,
                    source_mode,
                    target_mode,
                    scn,
                ).await
            }

            CacheFusionMessage::BlockInvalidate {
                resource_id,
                new_scn,
                invalidator,
            } => {
                self.handle_block_invalidate(resource_id, new_scn, invalidator).await
            }

            _ => Ok(()),
        }
    }

    async fn handle_block_request(
        &self,
        resource_id: ResourceId,
        requested_mode: BlockMode,
        requestor: NodeId,
        transaction_id: TransactionId,
        _force_current: bool,
    ) -> Result<(), DbError> {
        // Handle the request and send grant
        let grant = self.handle_local_block_request(
            resource_id,
            requested_mode,
            transaction_id,
        ).await?;

        let message = CacheFusionMessage::BlockGrant {
            resource_id: grant.resource_id,
            granted_mode: grant.granted_mode,
            block_data: grant.block_data,
            lvb: grant.lvb,
            needs_write_back: grant.needs_write_back,
        };

        self.message_tx
            .send((requestor, message))
            .map_err(|_| DbError::Internal("Failed to send block grant".to_string()))?;

        Ok(())
    }

    async fn handle_block_transfer(
        &self,
        resource_id: ResourceId,
        block_data: Vec<u8>,
        _source_mode: BlockMode,
        target_mode: BlockMode,
        scn: u64,
    ) -> Result<(), DbError> {
        let mut cache = self.local_cache.write();

        let state = cache.entry(resource_id.clone()).or_insert_with(|| {
            BlockState {
                resource_id: resource_id.clone(),
                mode: BlockMode::Null,
                holder: None,
                past_image_holders: HashSet::new(),
                scn: 0,
                lvb: LockValueBlock::default(),
                last_access: Instant::now(),
                transfer_count: 0,
            }
        });

        state.mode = target_mode;
        state.holder = Some(self.node_id.clone());
        state.scn = scn;
        state.lvb.current_scn = scn;
        state.last_access = Instant::now();

        Ok(())
    }

    async fn handle_block_invalidate(
        &self,
        resource_id: ResourceId,
        new_scn: u64,
        _invalidator: NodeId,
    ) -> Result<(), DbError> {
        let mut cache = self.local_cache.write();

        if let Some(state) = cache.get_mut(&resource_id) {
            state.scn = new_scn;
            state.lvb.current_scn = new_scn;
            state.mode = BlockMode::Null; // Invalidate local copy
        }

        Ok(())
    }
}
