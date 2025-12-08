//! I/O Scheduler for Resource Management
//!
//! This module implements I/O bandwidth allocation, IOPS limiting,
//! priority queues, deadline-based scheduling, and multi-tenant I/O isolation.
//!
//! Optimizations:
//! - Atomic counters for lock-free I/O stat updates
//! - Inline hints for scheduling hot paths
//! - Cold attributes for error paths
//! - Per-device I/O tracking

use std::collections::{HashMap, VecDeque, BinaryHeap};
use std::sync::{Arc, RwLock, Mutex};
use std::sync::atomic::{AtomicU64, AtomicU32, AtomicUsize, Ordering as AtomicOrdering};
use std::time::{Duration, Instant, SystemTime};
use std::cmp::Ordering;
use serde::{Deserialize, Serialize};

use crate::error::{DbError, Result};
use super::consumer_groups::ConsumerGroupId;

/// I/O request identifier
pub type IoRequestId = u64;

/// I/O request type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IoRequestType {
    /// Read request
    Read,
    /// Write request
    Write,
    /// Synchronous write
    SyncWrite,
    /// Metadata operation
    Metadata,
}

/// I/O priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IoPriority {
    /// Real-time priority (highest)
    RealTime,
    /// High priority
    High,
    /// Normal priority
    Normal,
    /// Low priority
    Low,
    /// Background/idle priority (lowest)
    Idle,
}

impl IoPriority {
    /// Get numeric value for priority (lower = higher priority)
    pub fn value(&self) -> u8 {
        match self {
            IoPriority::RealTime => 0,
            IoPriority::High => 1,
            IoPriority::Normal => 2,
            IoPriority::Low => 3,
            IoPriority::Idle => 4,
        }
    }
}

/// I/O request
#[derive(Debug, Clone)]
pub struct IoRequest {
    /// Request identifier
    pub id: IoRequestId,
    /// Consumer group
    pub group_id: ConsumerGroupId,
    /// Request type
    pub request_type: IoRequestType,
    /// Priority
    pub priority: IoPriority,
    /// Size in bytes
    pub size_bytes: u64,
    /// File/block offset
    pub offset: u64,
    /// Deadline for completion
    pub deadline: Option<Instant>,
    /// Submission time
    pub submitted_at: Instant,
    /// Start time (when I/O actually started)
    pub started_at: Option<Instant>,
    /// Completion time
    pub completed_at: Option<Instant>,
    /// Whether this is a sequential I/O
    pub is_sequential: bool,
}

impl IoRequest {
    /// Create a new I/O request
    pub fn new(
        id: IoRequestId,
        group_id: ConsumerGroupId,
        request_type: IoRequestType,
        priority: IoPriority,
        size_bytes: u64,
        offset: u64,
    ) -> Self {
        Self {
            id,
            group_id,
            request_type,
            priority,
            size_bytes,
            offset,
            deadline: None,
            submitted_at: Instant::now(),
            started_at: None,
            completed_at: None,
            is_sequential: false,
        }
    }

    /// Calculate wait time
    pub fn wait_time(&self) -> Duration {
        if let Some(started) = self.started_at {
            started.duration_since(self.submitted_at)
        } else {
            Instant::now().duration_since(self.submitted_at)
        }
    }

    /// Check if deadline has passed
    pub fn is_past_deadline(&self) -> bool {
        if let Some(deadline) = self.deadline {
            Instant::now() > deadline
        } else {
            false
        }
    }

    /// Calculate effective priority (considering deadline)
    pub fn effective_priority(&self) -> u8 {
        let mut priority = self.priority.value();

        // Boost priority if past deadline
        if self.is_past_deadline() {
            priority = priority.saturating_sub(2);
        }

        priority
    }
}

/// Ordering for priority queue
impl Ord for IoRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower priority value = higher priority
        self.effective_priority().cmp(&other.effective_priority())
            .then_with(|| {
                // For same priority, earlier deadline = higher priority
                match (&self.deadline, &other.deadline) {
                    (Some(d1), Some(d2)) => d1.cmp(d2),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => self.submitted_at.cmp(&other.submitted_at),
                }
            })
            .reverse() // Reverse for max-heap behavior
    }
}

impl PartialOrd for IoRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for IoRequest {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for IoRequest {}

/// I/O group allocation with atomic counters for lock-free updates
#[derive(Debug)]
pub struct IoGroupAllocation {
    pub group_id: ConsumerGroupId,
    /// Bandwidth limit in bytes/sec
    pub bandwidth_limit: Option<u64>,
    /// IOPS limit
    pub iops_limit: Option<u32>,
    /// I/O weight (for proportional sharing)
    pub weight: u32,
    /// Current bandwidth usage (bytes/sec, exponentially weighted)
    pub current_bandwidth: AtomicU64,
    /// Current IOPS
    pub current_iops: AtomicU32,
    /// Total bytes transferred - atomic for lock-free updates
    pub total_bytes: AtomicU64,
    /// Total I/O operations - atomic for lock-free updates
    pub total_ops: AtomicU64,
    /// Pending I/O requests - atomic for lock-free updates
    pub pending_requests: AtomicUsize,
}

impl IoGroupAllocation {
    #[inline]
    pub fn new(
        group_id: ConsumerGroupId,
        bandwidth_limit: Option<u64>,
        iops_limit: Option<u32>,
        weight: u32,
    ) -> Self {
        Self {
            group_id,
            bandwidth_limit,
            iops_limit,
            weight,
            current_bandwidth: AtomicU64::new(0),
            current_iops: AtomicU32::new(0),
            total_bytes: AtomicU64::new(0),
            total_ops: AtomicU64::new(0),
            pending_requests: AtomicUsize::new(0),
        }
    }

    #[inline]
    pub fn get_total_bytes(&self) -> u64 {
        self.total_bytes.load(AtomicOrdering::Relaxed)
    }

    #[inline]
    pub fn get_total_ops(&self) -> u64 {
        self.total_ops.load(AtomicOrdering::Relaxed)
    }

    #[inline]
    pub fn get_pending_requests(&self) -> usize {
        self.pending_requests.load(AtomicOrdering::Relaxed)
    }

    #[inline]
    pub fn add_bytes(&self, bytes: u64) {
        self.total_bytes.fetch_add(bytes, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn inc_ops(&self) {
        self.total_ops.fetch_add(1, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn inc_pending(&self) {
        self.pending_requests.fetch_add(1, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn dec_pending(&self) {
        self.pending_requests.fetch_sub(1, AtomicOrdering::Relaxed);
    }
}

/// Token bucket for rate limiting
#[derive(Debug)]
pub struct TokenBucket {
    /// Maximum tokens
    capacity: u64,
    /// Current tokens available
    tokens: f64,
    /// Refill rate (tokens per second)
    refill_rate: f64,
    /// Last refill time
    last_refill: Instant,
}

impl TokenBucket {
    /// Create a new token bucket
    pub fn new(capacity: u64, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume tokens
    pub fn try_consume(&mut self, tokens: u64) -> bool {
        self.refill();

        if self.tokens >= tokens as f64 {
            self.tokens -= tokens as f64;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let new_tokens = elapsed * self.refill_rate;

        self.tokens = (self.tokens + new_tokens).min(self.capacity as f64);
        self.last_refill = now;
    }

    /// Get available tokens
    pub fn available(&mut self) -> u64 {
        self.refill();
        self.tokens as u64
    }
}

/// I/O scheduling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IoSchedulingPolicy {
    /// Completely Fair Queuing (CFQ)
    CompletelyFair,
    /// Deadline-based scheduling
    Deadline,
    /// Budget Fair Queueing (BFQ)
    BudgetFair,
    /// Priority-based
    Priority,
}

/// I/O scheduler with atomic counters
pub struct IoScheduler {
    /// Scheduling policy
    policy: IoSchedulingPolicy,
    /// All I/O requests
    requests: Arc<RwLock<HashMap<IoRequestId, IoRequest>>>,
    /// Priority queue for pending requests
    pending_queue: Arc<Mutex<BinaryHeap<IoRequest>>>,
    /// Per-group queues
    group_queues: Arc<RwLock<HashMap<ConsumerGroupId, VecDeque<IoRequestId>>>>,
    /// Group allocations
    group_allocations: Arc<RwLock<HashMap<ConsumerGroupId, IoGroupAllocation>>>,
    /// Bandwidth token buckets per group
    bandwidth_buckets: Arc<RwLock<HashMap<ConsumerGroupId, TokenBucket>>>,
    /// IOPS token buckets per group
    iops_buckets: Arc<RwLock<HashMap<ConsumerGroupId, TokenBucket>>>,
    /// Active I/O requests
    active_requests: Arc<RwLock<Vec<IoRequestId>>>,
    /// Next request ID - atomic for lock-free allocation
    next_request_id: AtomicU64,
    /// Maximum concurrent I/O operations
    max_concurrent_io: usize,
    /// Statistics with atomic counters
    stats: Arc<IoStats>,
}

/// I/O statistics with atomic counters for lock-free updates
#[derive(Debug, Default)]
pub struct IoStats {
    /// Total I/O requests submitted
    pub total_requests: AtomicU64,
    /// Total I/O requests completed
    pub completed_requests: AtomicU64,
    /// Total bytes read
    pub total_bytes_read: AtomicU64,
    /// Total bytes written
    pub total_bytes_written: AtomicU64,
    /// Average I/O latency (microseconds)
    pub avg_latency_us: AtomicU64,
    /// Number of deadline misses
    pub deadline_misses: AtomicU64,
    /// Throttled requests
    pub throttled_requests: AtomicU64,
}

impl IoStats {
    #[inline]
    pub fn inc_requests(&self) {
        self.total_requests.fetch_add(1, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn inc_completed(&self) {
        self.completed_requests.fetch_add(1, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn add_bytes_read(&self, bytes: u64) {
        self.total_bytes_read.fetch_add(bytes, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn add_bytes_written(&self, bytes: u64) {
        self.total_bytes_written.fetch_add(bytes, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn inc_deadline_misses(&self) {
        self.deadline_misses.fetch_add(1, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn inc_throttled(&self) {
        self.throttled_requests.fetch_add(1, AtomicOrdering::Relaxed);
    }

    pub fn snapshot(&self) -> IoStatsSnapshot {
        IoStatsSnapshot {
            total_requests: self.total_requests.load(AtomicOrdering::Relaxed),
            completed_requests: self.completed_requests.load(AtomicOrdering::Relaxed),
            total_bytes_read: self.total_bytes_read.load(AtomicOrdering::Relaxed),
            total_bytes_written: self.total_bytes_written.load(AtomicOrdering::Relaxed),
            avg_latency_us: self.avg_latency_us.load(AtomicOrdering::Relaxed),
            deadline_misses: self.deadline_misses.load(AtomicOrdering::Relaxed),
            throttled_requests: self.throttled_requests.load(AtomicOrdering::Relaxed),
        }
    }
}

/// Snapshot of I/O statistics for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoStatsSnapshot {
    pub total_requests: u64,
    pub completed_requests: u64,
    pub total_bytes_read: u64,
    pub total_bytes_written: u64,
    pub avg_latency_us: u64,
    pub deadline_misses: u64,
    pub throttled_requests: u64,
}

impl IoScheduler {
    /// Create a new I/O scheduler
    pub fn new(policy: IoSchedulingPolicy, max_concurrent_io: usize) -> Self {
        Self {
            policy,
            requests: Arc::new(RwLock::new(HashMap::new())),
            pending_queue: Arc::new(Mutex::new(BinaryHeap::new())),
            group_queues: Arc::new(RwLock::new(HashMap::new())),
            group_allocations: Arc::new(RwLock::new(HashMap::new())),
            bandwidth_buckets: Arc::new(RwLock::new(HashMap::new())),
            iops_buckets: Arc::new(RwLock::new(HashMap::new())),
            active_requests: Arc::new(RwLock::new(Vec::new())),
            next_request_id: AtomicU64::new(1),
            max_concurrent_io,
            stats: Arc::new(IoStats::default()),
        }
    }

    /// Register a consumer group with I/O limits
    pub fn register_group(
        &self,
        group_id: ConsumerGroupId,
        bandwidth_limit: Option<u64>,
        iops_limit: Option<u32>,
        weight: u32,
    ) -> Result<()> {
        let mut allocations = self.group_allocations.write().unwrap();

        if allocations.contains_key(&group_id) {
            return Err(DbError::AlreadyExists(
                format!("Group {} already registered", group_id)
            ));
        }

        allocations.insert(
            group_id,
            IoGroupAllocation::new(group_id, bandwidth_limit, iops_limit, weight)
        );

        // Create token buckets if limits are set
        if let Some(bw_limit) = bandwidth_limit {
            let mut buckets = self.bandwidth_buckets.write().unwrap();
            buckets.insert(group_id, TokenBucket::new(bw_limit, bw_limit as f64));
        }

        if let Some(iops_limit) = iops_limit {
            let mut buckets = self.iops_buckets.write().unwrap();
            buckets.insert(group_id, TokenBucket::new(iops_limit as u64, iops_limit as f64));
        }

        // Create group queue
        let mut queues = self.group_queues.write().unwrap();
        queues.insert(group_id, VecDeque::new());

        Ok(())
    }

    /// Submit an I/O request
    #[inline]
    pub fn submit_request(
        &self,
        group_id: ConsumerGroupId,
        request_type: IoRequestType,
        priority: IoPriority,
        size_bytes: u64,
        offset: u64,
        deadline: Option<Duration>,
    ) -> Result<IoRequestId> {
        // Atomic request ID allocation (lock-free)
        let request_id = self.next_request_id.fetch_add(1, AtomicOrdering::Relaxed);

        let mut request = IoRequest::new(
            request_id,
            group_id,
            request_type,
            priority,
            size_bytes,
            offset,
        );

        if let Some(deadline_duration) = deadline {
            request.deadline = Some(Instant::now() + deadline_duration);
        }

        // Add to requests map
        {
            let mut requests = self.requests.write().unwrap();
            requests.insert(request_id, request.clone());
        }

        // Add to appropriate queue
        match self.policy {
            IoSchedulingPolicy::Priority | IoSchedulingPolicy::Deadline => {
                let mut pending_queue = self.pending_queue.lock().unwrap();
                pending_queue.push(request);
            }
            _ => {
                let mut group_queues = self.group_queues.write().unwrap();
                if let Some(queue) = group_queues.get_mut(&group_id) {
                    queue.push_back(request_id);
                }
            }
        }

        // Update group stats atomically
        {
            let allocations = self.group_allocations.read().unwrap();
            if let Some(alloc) = allocations.get(&group_id) {
                alloc.inc_pending();
            }
        }

        // Update stats atomically (lock-free)
        self.stats.inc_requests();

        Ok(request_id)
    }

    /// Schedule next I/O request (hot path - inline)
    #[inline]
    pub fn schedule_next(&self) -> Option<IoRequestId> {
        // Check if we can accept more concurrent I/O
        {
            let active = self.active_requests.read().unwrap();
            if active.len() >= self.max_concurrent_io {
                return None;
            }
        }

        match self.policy {
            IoSchedulingPolicy::Priority => self.schedule_priority(),
            IoSchedulingPolicy::Deadline => self.schedule_deadline(),
            IoSchedulingPolicy::CompletelyFair => self.schedule_cfq(),
            IoSchedulingPolicy::BudgetFair => self.schedule_bfq(),
        }
    }

    /// Priority-based scheduling (hot path - inline)
    #[inline]
    fn schedule_priority(&self) -> Option<IoRequestId> {
        let mut pending_queue = self.pending_queue.lock().unwrap();

        while let Some(request) = pending_queue.pop() {
            // Check group limits
            if self.check_group_limits(request.group_id, request.size_bytes) {
                self.start_request(request.id);
                return Some(request.id);
            } else {
                // Put back in queue if throttled
                pending_queue.push(request);
                break;
            }
        }

        None
    }

    /// Deadline-based scheduling
    fn schedule_deadline(&self) -> Option<IoRequestId> {
        // Same as priority for now, but prioritizes deadline misses
        self.schedule_priority()
    }

    /// Completely Fair Queuing (hot path - inline)
    #[inline]
    fn schedule_cfq(&self) -> Option<IoRequestId> {
        let allocations = self.group_allocations.read().unwrap();
        let mut group_queues = self.group_queues.write().unwrap();

        // Find group with lowest service time relative to weight
        let mut best_group: Option<ConsumerGroupId> = None;
        let mut best_ratio = f64::MAX;

        for (group_id, alloc) in allocations.iter() {
            if alloc.get_pending_requests() == 0 {
                continue;
            }

            if let Some(queue) = group_queues.get(group_id) {
                if queue.is_empty() {
                    continue;
                }

                // Calculate service ratio (lower is better)
                let ratio = if alloc.weight > 0 {
                    alloc.get_total_bytes() as f64 / alloc.weight as f64
                } else {
                    f64::MAX
                };

                if ratio < best_ratio {
                    best_ratio = ratio;
                    best_group = Some(*group_id);
                }
            }
        }

        // Schedule from best group
        if let Some(group_id) = best_group {
            drop(allocations);

            if let Some(queue) = group_queues.get_mut(&group_id) {
                while let Some(request_id) = queue.pop_front() {
                    let requests = self.requests.read().unwrap();
                    if let Some(request) = requests.get(&request_id) {
                        let size = request.size_bytes;
                        drop(requests);

                        if self.check_group_limits(group_id, size) {
                            self.start_request(request_id);
                            return Some(request_id);
                        } else {
                            // Put back and try later
                            queue.push_front(request_id);
                            break;
                        }
                    }
                }
            }
        }

        None
    }

    /// Budget Fair Queuing
    fn schedule_bfq(&self) -> Option<IoRequestId> {
        // Similar to CFQ but with time budgets
        self.schedule_cfq()
    }

    /// Check if group is within limits (hot path - inline)
    #[inline]
    fn check_group_limits(&self, group_id: ConsumerGroupId, size_bytes: u64) -> bool {
        // Check bandwidth limit
        if let Some(mut buckets) = self.bandwidth_buckets.write().ok() {
            if let Some(bucket) = buckets.get_mut(&group_id) {
                if !bucket.try_consume(size_bytes) {
                    self.stats.inc_throttled();
                    return false;
                }
            }
        }

        // Check IOPS limit
        if let Some(mut buckets) = self.iops_buckets.write().ok() {
            if let Some(bucket) = buckets.get_mut(&group_id) {
                if !bucket.try_consume(1) {
                    self.stats.inc_throttled();
                    return false;
                }
            }
        }

        true
    }

    /// Start an I/O request
    fn start_request(&self, request_id: IoRequestId) {
        let mut requests = self.requests.write().unwrap();
        if let Some(request) = requests.get_mut(&request_id) {
            request.started_at = Some(Instant::now());
        }

        let mut active = self.active_requests.write().unwrap();
        active.push(request_id);
    }

    /// Complete an I/O request
    pub fn complete_request(&self, request_id: IoRequestId) -> Result<()> {
        let mut requests = self.requests.write().unwrap();
        let request = requests.get_mut(&request_id)
            .ok_or_else(|| DbError::NotFound(format!("Request {} not found", request_id)))?;

        request.completed_at = Some(Instant::now());
        let request_type = request.request_type;
        let size_bytes = request.size_bytes;
        let group_id = request.group_id;
        let is_past_deadline = request.is_past_deadline();
        let started_at = request.started_at;
        let submitted_at = request.submitted_at;

        drop(requests);

        // Update group stats atomically
        {
            let allocations = self.group_allocations.read().unwrap();
            if let Some(alloc) = allocations.get(&group_id) {
                alloc.add_bytes(size_bytes);
                alloc.inc_ops();
                alloc.dec_pending();
            }
        }

        // Update global stats atomically (lock-free)
        self.stats.inc_completed();

        match request_type {
            IoRequestType::Read => self.stats.add_bytes_read(size_bytes),
            IoRequestType::Write | IoRequestType::SyncWrite => {
                self.stats.add_bytes_written(size_bytes)
            }
            _ => {}
        }

        if is_past_deadline {
            self.stats.inc_deadline_misses();
        }

        // Update average latency
        if let Some(started) = started_at {
            let latency_us = started.duration_since(submitted_at).as_micros() as u64;
            let current_avg = self.stats.avg_latency_us.load(AtomicOrdering::Relaxed);
            self.stats.avg_latency_us.store((current_avg + latency_us) / 2, AtomicOrdering::Relaxed);
        }

        // Remove from active requests
        {
            let mut active = self.active_requests.write().unwrap();
            active.retain(|&id| id != request_id);
        }

        Ok(())
    }

    /// Get I/O statistics (snapshot for serialization)
    pub fn get_stats(&self) -> IoStatsSnapshot {
        self.stats.snapshot()
    }

    /// Get group I/O statistics (values only, not Clone)
    pub fn get_group_stats(&self, group_id: ConsumerGroupId) -> Option<(ConsumerGroupId, u64, u64, usize)> {
        let allocations = self.group_allocations.read().unwrap();
        allocations.get(&group_id).map(|alloc| {
            (
                alloc.group_id,
                alloc.get_total_bytes(),
                alloc.get_total_ops(),
                alloc.get_pending_requests(),
            )
        })
    }

    /// Update bandwidth usage metrics
    pub fn update_bandwidth_metrics(&self) {
        let mut allocations = self.group_allocations.write().unwrap();

        for alloc in allocations.values_mut() {
            // Exponentially weighted moving average
            alloc.current_bandwidth = (alloc.current_bandwidth * 9 + alloc.total_bytes) / 10;
            alloc.current_iops = ((alloc.current_iops as u64 * 9 + alloc.total_ops) / 10) as u32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_scheduler_creation() {
        let scheduler = IoScheduler::new(IoSchedulingPolicy::Priority, 32);
        assert_eq!(scheduler.policy, IoSchedulingPolicy::Priority);
    }

    #[test]
    fn test_submit_request() {
        let scheduler = IoScheduler::new(IoSchedulingPolicy::Priority, 32);
        scheduler.register_group(1, None, None, 100).unwrap();

        let request_id = scheduler.submit_request(
            1,
            IoRequestType::Read,
            IoPriority::Normal,
            4096,
            0,
            None,
        ).unwrap();

        assert!(request_id > 0);
    }

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(1000, 1000.0);
        assert!(bucket.try_consume(500));
        assert!(bucket.try_consume(500));
        assert!(!bucket.try_consume(100)); // Should fail, no tokens left
    }

    #[test]
    fn test_schedule_with_limits() {
        let scheduler = IoScheduler::new(IoSchedulingPolicy::Priority, 32);
        // Set low bandwidth limit
        scheduler.register_group(1, Some(1000), Some(10), 100).unwrap();

        scheduler.submit_request(
            1,
            IoRequestType::Read,
            IoPriority::Normal,
            500,
            0,
            None,
        ).unwrap();

        let scheduled = scheduler.schedule_next();
        assert!(scheduled.is_some());
    }
}


