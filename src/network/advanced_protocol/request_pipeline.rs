// Request Pipeline Module
//
// Request/response pipeline, priority queuing, and request management

use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;
use crate::error::DbError;

// ============================================================================
// Constants - Bounds for Open-Ended Data Structures
// ============================================================================

/// Maximum number of pending requests to prevent unbounded memory growth
/// See: diagrams/06_network_api_flow.md - Issue #5.1
pub const MAX_PENDING_REQUESTS: usize = 10_000;

/// Maximum size of priority queue to prevent DoS attacks
/// See: diagrams/06_network_api_flow.md - Issue #5.2
pub const MAX_PRIORITY_QUEUE_SIZE: usize = 1_000;

// ============================================================================
// Request/Response Types
// ============================================================================

pub type RequestId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone)]
pub struct ProtocolRequest {
    pub id: RequestId,
    pub priority: RequestPriority,
    pub payload: Vec<u8>,
}

impl ProtocolRequest {
    pub fn new(id: RequestId, priority: RequestPriority, payload: Vec<u8>) -> Self {
        Self {
            id,
            priority,
            payload,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProtocolResponse {
    pub request_id: RequestId,
    pub payload: Vec<u8>,
}

impl ProtocolResponse {
    pub fn new(request_id: RequestId, payload: Vec<u8>) -> Self {
        Self {
            request_id,
            payload,
        }
    }
}

// ============================================================================
// Pipeline Management
// ============================================================================

pub struct RequestResponsePipeline {
    pending: HashMap<RequestId, ProtocolRequest>,
    next_id: RequestId,
}

impl RequestResponsePipeline {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn next_request_id(&mut self) -> RequestId {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    /// Add a pending request with backpressure control
    /// Returns error if MAX_PENDING_REQUESTS limit is reached
    pub fn add_pending(&mut self, request: ProtocolRequest) -> Result<(), DbError> {
        if self.pending.len() >= MAX_PENDING_REQUESTS {
            return Err(DbError::Network(format!(
                "Request pipeline full: {} pending requests (max: {})",
                self.pending.len(),
                MAX_PENDING_REQUESTS
            )));
        }
        self.pending.insert(request.id, request);
        Ok(())
    }

    pub fn remove_pending(&mut self, request_id: RequestId) -> Option<ProtocolRequest> {
        self.pending.remove(&request_id)
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

impl Default for RequestResponsePipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct PipelineMetrics {
    pub requests_queued: u64,
    pub requests_completed: u64,
}

impl Default for PipelineMetrics {
    fn default() -> Self {
        Self {
            requests_queued: 0,
            requests_completed: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineStats {
    pub avg_latency_ms: f64,
}

// ============================================================================
// Priority Queue - Using BinaryHeap for O(log n) operations
// ============================================================================

// Wrapper for ProtocolRequest to implement Ord for BinaryHeap (max-heap by priority)
#[derive(Debug, Clone)]
struct PriorityWrapper(ProtocolRequest);

impl PartialEq for PriorityWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0.priority == other.0.priority && self.0.id == other.0.id
    }
}

impl Eq for PriorityWrapper {}

impl PartialOrd for PriorityWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then by request ID (FIFO within priority)
        self.0.priority.cmp(&other.0.priority)
            .then_with(|| other.0.id.cmp(&self.0.id))
    }
}

/// Priority-based request queue with bounded size
/// FIXED: Replaced Vec (O(n log n) sort + O(n) remove) with BinaryHeap (O(log n) operations)
/// See: diagrams/06_network_api_flow.md - Issue #3.1
pub struct PriorityRequestQueue {
    queue: BinaryHeap<PriorityWrapper>,
}

impl PriorityRequestQueue {
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
        }
    }

    /// Enqueue a request with backpressure control
    /// Returns error if MAX_PRIORITY_QUEUE_SIZE limit is reached
    pub fn enqueue(&mut self, request: ProtocolRequest) -> Result<(), DbError> {
        if self.queue.len() >= MAX_PRIORITY_QUEUE_SIZE {
            return Err(DbError::Network(format!(
                "Priority queue full: {} requests (max: {})",
                self.queue.len(),
                MAX_PRIORITY_QUEUE_SIZE
            )));
        }
        self.queue.push(PriorityWrapper(request));
        Ok(())
    }

    /// Dequeue highest priority request - O(log n) operation
    pub fn dequeue(&mut self) -> Option<ProtocolRequest> {
        self.queue.pop().map(|w| w.0)
    }

    pub fn peek(&self) -> Option<&ProtocolRequest> {
        self.queue.peek().map(|w| &w.0)
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl Default for PriorityRequestQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub size: usize,
    pub avg_wait_time_ms: f64,
}
