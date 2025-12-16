// Request Pipeline Module
//
// Request/response pipeline, priority queuing, and request management

use std::collections::HashMap;

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

    pub fn add_pending(&mut self, request: ProtocolRequest) {
        self.pending.insert(request.id, request);
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
// Priority Queue
// ============================================================================

pub struct PriorityRequestQueue {
    queue: Vec<ProtocolRequest>,
}

impl PriorityRequestQueue {
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    pub fn enqueue(&mut self, request: ProtocolRequest) {
        self.queue.push(request);
        // Sort by priority (highest first)
        self.queue.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn dequeue(&mut self) -> Option<ProtocolRequest> {
        if self.queue.is_empty() {
            None
        } else {
            Some(self.queue.remove(0))
        }
    }

    pub fn peek(&self) -> Option<&ProtocolRequest> {
        self.queue.first()
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
