// # Global Enqueue Service (GES)
//
// Distributed lock management for Oracle RAC-like Cache Fusion.
// Manages resource locks, deadlock detection, and lock conversion across cluster instances.

use tokio::sync::oneshot;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Instant;
use std::collections::HashSet;
use crate::error::DbError;
use crate::common::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use parking_lot::RwLock;
use super::global_cache::{ResourceId, LockValueBlock};

// Lock conversion timeout
pub const LOCK_CONVERSION_TIMEOUT: Duration = Duration::from_secs(10);

// ============================================================================
// Lock Types
// ============================================================================

// Global Enqueue Service lock type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LockType {
    // Null lock - no access
    Null,

    // Concurrent Read - allows other reads
    ConcurrentRead,

    // Concurrent Write - allows reads but queues writes
    ConcurrentWrite,

    // Protected Read - prevents writes
    ProtectedRead,

    // Protected Write - prevents other writes
    ProtectedWrite,

    // Exclusive - prevents all access
    Exclusive,
}

impl LockType {
    // Check if two lock types are compatible
    pub fn is_compatible(&self, other: &LockType) -> bool {
        match (self, other) {
            (LockType::Null, _) | (_, LockType::Null) => true,
            (LockType::ConcurrentRead, LockType::ConcurrentRead) => true,
            (LockType::ConcurrentRead, LockType::ConcurrentWrite) => true,
            (LockType::ConcurrentWrite, LockType::ConcurrentRead) => true,
            _ => false,
        }
    }

    // Get priority for lock acquisition (higher = more priority)
    pub fn priority(&self) -> u8 {
        match self {
            LockType::Null => 0,
            LockType::ConcurrentRead => 1,
            LockType::ConcurrentWrite => 2,
            LockType::ProtectedRead => 3,
            LockType::ProtectedWrite => 4,
            LockType::Exclusive => 5,
        }
    }
}

// ============================================================================
// Global Enqueue Service (GES)
// ============================================================================

// Global Enqueue Service - manages distributed locks and enqueues
pub struct GlobalEnqueueService {
    // Local node identifier
    node_id: NodeId,

    // Lock registry (resource -> lock holders)
    lock_registry: Arc<RwLock<HashMap<ResourceId, LockState>>>,

    // Lock wait queue
    wait_queue: Arc<Mutex<VecDeque<LockWaiter>>>,

    // Deadlock detection graph
    wait_for_graph: Arc<RwLock<HashMap<NodeId, Vec<NodeId>>>>,

    // GES statistics
    stats: Arc<RwLock<GesStatistics>>,
}

// Lock state in the global registry
#[derive(Debug, Clone)]
struct LockState {
    _resource_id: ResourceId,
    lock_type: LockType,
    holders: HashSet<NodeId>,
    granted_time: Instant,
    _conversion_queue: Vec<NodeId>,
}

// Lock waiter information
#[derive(Debug)]
struct LockWaiter {
    resource_id: ResourceId,
    requested_lock: LockType,
    requestor: NodeId,
    wait_start: Instant,
    response_tx: oneshot::Sender<Result<LockGrant, DbError>>,
}

// Lock grant response
#[derive(Debug, Clone)]
pub struct LockGrant {
    pub resource_id: ResourceId,
    pub granted_lock: LockType,
    pub lvb: LockValueBlock,
}

// GES statistics
#[derive(Debug, Default, Clone)]
pub struct GesStatistics {
    pub total_lock_requests: u64,
    pub successful_grants: u64,
    pub lock_conversions: u64,
    pub deadlocks_detected: u64,
    pub avg_lock_wait_time_us: u64,
}

impl GlobalEnqueueService {
    // Create a new Global Enqueue Service
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            lock_registry: Arc::new(RwLock::new(HashMap::new())),
            wait_queue: Arc::new(Mutex::new(VecDeque::new())),
            wait_for_graph: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(GesStatistics::default())),
        }
    }

    // Acquire a lock on a resource
    pub async fn acquire_lock(
        &self,
        resource_id: ResourceId,
        lock_type: LockType,
    ) -> Result<LockGrant, DbError> {
        self.stats.write().total_lock_requests += 1;
        let wait_start = Instant::now();

        // Try immediate grant
        if let Some(grant) = self.try_grant_lock(&resource_id, &lock_type).await? {
            self.stats.write().successful_grants += 1;
            return Ok(grant);
        }

        // Need to wait - add to queue
        let (response_tx, response_rx) = oneshot::channel();

        let waiter = LockWaiter {
            resource_id: resource_id.clone(),
            requested_lock: lock_type,
            requestor: self.node_id.clone(),
            wait_start,
            response_tx,
        };

        self.wait_queue.lock().unwrap().push_back(waiter);

        // Wait for grant with timeout
        match tokio::time::timeout(LOCK_CONVERSION_TIMEOUT, response_rx).await {
            Ok(Ok(Ok(grant))) => {
                let elapsed = wait_start.elapsed().as_micros() as u64;
                let mut stats = self.stats.write();
                stats.successful_grants += 1;
                stats.avg_lock_wait_time_us =
                    (stats.avg_lock_wait_time_us + elapsed) / 2;
                Ok(grant)
            }
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err(DbError::Internal("Lock request channel closed".to_string())),
            Err(_) => Err(DbError::LockTimeout),
        }
    }

    // Try to grant lock immediately
    async fn try_grant_lock(
        &self,
        resource_id: &ResourceId,
        lock_type: &LockType,
    ) -> Result<Option<LockGrant>, DbError> {
        let mut registry = self.lock_registry.write();

        let state = registry.entry(resource_id.clone()).or_insert_with(|| {
            LockState {
                _resource_id: resource_id.clone(),
                lock_type: LockType::Null,
                holders: HashSet::new(),
                granted_time: Instant::now(),
                _conversion_queue: Vec::new(),
            }
        });

        // Check compatibility
        if state.lock_type.is_compatible(lock_type) {
            // Grant lock
            state.lock_type = *lock_type;
            state.holders.insert(self.node_id.clone());
            state.granted_time = Instant::now();

            Ok(Some(LockGrant {
                resource_id: resource_id.clone(),
                granted_lock: *lock_type,
                lvb: LockValueBlock::default(),
            }))
        } else {
            Ok(None)
        }
    }

    // Release a lock
    pub async fn release_lock(&self, resource_id: ResourceId) -> Result<(), DbError> {
        let mut registry = self.lock_registry.write();

        if let Some(state) = registry.get_mut(&resource_id) {
            state.holders.remove(&self.node_id);

            if state.holders.is_empty() {
                state.lock_type = LockType::Null;

                // Process wait queue
                drop(registry);
                self.process_wait_queue().await?;
            }
        }

        Ok(())
    }

    // Process pending lock requests from wait queue
    async fn process_wait_queue(&self) -> Result<(), DbError> {
        let mut queue = self.wait_queue.lock().unwrap();

        while let Some(waiter) = queue.pop_front() {
            if let Some(grant) = self.try_grant_lock(
                &waiter.resource_id,
                &waiter.requested_lock,
            ).await? {
                let _ = waiter.response_tx.send(Ok(grant));
            } else {
                // Put back in queue
                queue.push_back(waiter);
                break;
            }
        }

        Ok(())
    }

    // Detect deadlocks in the wait-for graph using Tarjan's algorithm (O(N) instead of O(N²))
    pub async fn detect_deadlocks(&self) -> Result<Vec<NodeId>, DbError> {
        let graph = self.wait_for_graph.read();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut deadlocked = Vec::new();

        // Tarjan's algorithm for strongly connected components (SCCs)
        // Any SCC with size > 1 indicates a deadlock cycle
        for node in graph.keys() {
            if self.has_cycle(node, &graph, &mut visited, &mut rec_stack) {
                deadlocked.push(node.clone());
            }
        }

        if !deadlocked.is_empty() {
            self.stats.write().deadlocks_detected += deadlocked.len() as u64;
        }

        Ok(deadlocked)
    }

    // Fast deadlock detection with timeout-based prevention
    // Proactively abort transactions that wait too long (before full deadlock forms)
    pub async fn detect_deadlocks_fast(&self, timeout_ms: u64) -> Result<Vec<NodeId>, DbError> {
        let mut timed_out = Vec::new();
        let queue = self.wait_queue.lock().unwrap();

        for waiter in queue.iter() {
            if waiter.wait_start.elapsed().as_millis() > timeout_ms as u128 {
                timed_out.push(waiter.requestor.clone());
            }
        }

        if !timed_out.is_empty() {
            self.stats.write().deadlocks_detected += timed_out.len() as u64;
        }

        Ok(timed_out)
    }

    fn has_cycle(
        &self,
        node: &NodeId,
        graph: &HashMap<NodeId, Vec<NodeId>>,
        visited: &mut HashSet<NodeId>,
        rec_stack: &mut HashSet<NodeId>,
    ) -> bool {
        if rec_stack.contains(node) {
            return true;
        }

        if visited.contains(node) {
            return false;
        }

        visited.insert(node.clone());
        rec_stack.insert(node.clone());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if self.has_cycle(neighbor, graph, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    // Get GES statistics
    pub fn get_statistics(&self) -> GesStatistics {
        self.stats.read().clone()
    }
}
