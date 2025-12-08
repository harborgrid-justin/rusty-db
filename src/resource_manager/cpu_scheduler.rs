//! CPU Scheduler for Resource Management
//!
//! This module implements CPU time allocation, fair-share scheduling,
//! priority-based round-robin, and runaway query detection.
//!
//! Optimizations:
//! - Per-core resource tracking with cache-line alignment to avoid false sharing
//! - Atomic counters for lock-free updates
//! - Inline hints for hot paths
//! - Cold attributes for error paths

use std::collections::{HashMap, VecDeque, BinaryHeap};
use std::sync::{Arc, RwLock, Mutex};
use std::sync::atomic::{AtomicU64, AtomicU32, AtomicUsize, Ordering as AtomicOrdering};
use std::time::{Duration, Instant, SystemTime};
use std::cmp::Ordering;
use serde::{Deserialize, Serialize};

use crate::error::{DbError, Result};
use super::consumer_groups::{ConsumerGroupId, PriorityLevel};
use super::plans::ResourcePlanId;

/// Per-core resource tracker - avoids global lock contention
/// Cache line aligned to prevent false sharing (64 bytes on most platforms)
#[repr(C, align(64))]
pub struct PerCoreResourceTracker {
    /// CPU time used by this core (nanoseconds)
    cpu_used: AtomicU64,
    /// Memory used by this core (bytes)
    memory_used: AtomicU64,
    /// Pending I/O operations
    io_pending: AtomicU64,
    /// Number of tasks scheduled on this core
    tasks_scheduled: AtomicU64,
    /// Cache line padding to ensure 64-byte alignment
    _padding: [u8; 32],
}

impl PerCoreResourceTracker {
    /// Create a new per-core tracker
    #[inline]
    pub fn new() -> Self {
        Self {
            cpu_used: AtomicU64::new(0),
            memory_used: AtomicU64::new(0),
            io_pending: AtomicU64::new(0),
            tasks_scheduled: AtomicU64::new(0),
            _padding: [0; 32],
        }
    }

    /// Record CPU time usage (lock-free)
    #[inline]
    pub fn add_cpu_time(&self, nanoseconds: u64) {
        self.cpu_used.fetch_add(nanoseconds, AtomicOrdering::Relaxed);
    }

    /// Record memory usage (lock-free)
    #[inline]
    pub fn add_memory(&self, bytes: u64) {
        self.memory_used.fetch_add(bytes, AtomicOrdering::Relaxed);
    }

    /// Increment I/O pending count
    #[inline]
    pub fn inc_io_pending(&self) {
        self.io_pending.fetch_add(1, AtomicOrdering::Relaxed);
    }

    /// Decrement I/O pending count
    #[inline]
    pub fn dec_io_pending(&self) {
        self.io_pending.fetch_sub(1, AtomicOrdering::Relaxed);
    }

    /// Increment task count
    #[inline]
    pub fn inc_tasks(&self) {
        self.tasks_scheduled.fetch_add(1, AtomicOrdering::Relaxed);
    }

    /// Get current CPU usage
    #[inline]
    pub fn cpu_usage(&self) -> u64 {
        self.cpu_used.load(AtomicOrdering::Relaxed)
    }

    /// Get current memory usage
    #[inline]
    pub fn memory_usage(&self) -> u64 {
        self.memory_used.load(AtomicOrdering::Relaxed)
    }

    /// Get pending I/O count
    #[inline]
    pub fn io_pending_count(&self) -> u64 {
        self.io_pending.load(AtomicOrdering::Relaxed)
    }

    /// Get total tasks scheduled
    #[inline]
    pub fn tasks_count(&self) -> u64 {
        self.tasks_scheduled.load(AtomicOrdering::Relaxed)
    }
}

/// Query identifier
pub type QueryId = u64;

/// Thread/Task identifier
pub type TaskId = u64;

/// CPU quantum size in milliseconds
const DEFAULT_QUANTUM_MS: u64 = 100;

/// Runaway query CPU time threshold (in seconds)
const RUNAWAY_THRESHOLD_SECS: u64 = 300;

/// CPU scheduling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchedulingPolicy {
    /// Fair-share scheduling based on shares
    FairShare,
    /// Priority-based round-robin
    PriorityRoundRobin,
    /// Weighted fair queuing
    WeightedFairQueuing,
    /// Completely fair scheduler (CFS-like)
    CompletelyFair,
}

/// Task state in the scheduler
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is ready to run
    Ready,
    /// Task is currently running
    Running,
    /// Task is waiting for I/O
    Waiting,
    /// Task is sleeping
    Sleeping,
    /// Task is throttled due to resource limits
    Throttled,
    /// Task is completed
    Completed,
}

/// Scheduled task information
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// Task identifier
    pub task_id: TaskId,
    /// Associated query ID
    pub query_id: QueryId,
    /// Consumer group
    pub group_id: ConsumerGroupId,
    /// Task priority
    pub priority: PriorityLevel,
    /// Task state
    pub state: TaskState,
    /// CPU time used (nanoseconds)
    pub cpu_time_used: u64,
    /// Number of times scheduled
    pub schedule_count: u64,
    /// Last scheduled time
    pub last_scheduled: Option<Instant>,
    /// Task creation time
    pub created_at: Instant,
    /// Virtual runtime (for CFS)
    pub vruntime: u64,
    /// Time slice remaining (nanoseconds)
    pub time_slice_remaining: u64,
    /// Whether task is a runaway query
    pub is_runaway: bool,
    /// Throttle factor (1.0 = normal, <1.0 = throttled)
    pub throttle_factor: f64,
}

impl ScheduledTask {
    /// Create a new scheduled task
    pub fn new(
        task_id: TaskId,
        query_id: QueryId,
        group_id: ConsumerGroupId,
        priority: PriorityLevel,
    ) -> Self {
        Self {
            task_id,
            query_id,
            group_id,
            priority,
            state: TaskState::Ready,
            cpu_time_used: 0,
            schedule_count: 0,
            last_scheduled: None,
            created_at: Instant::now(),
            vruntime: 0,
            time_slice_remaining: DEFAULT_QUANTUM_MS * 1_000_000, // Convert to nanoseconds
            is_runaway: false,
            throttle_factor: 1.0,
        }
    }

    /// Update CPU time usage
    pub fn add_cpu_time(&mut self, nanoseconds: u64) {
        self.cpu_time_used += nanoseconds;

        // Check for runaway query
        if self.cpu_time_used > RUNAWAY_THRESHOLD_SECS * 1_000_000_000 {
            self.is_runaway = true;
        }
    }

    /// Calculate priority for scheduling
    pub fn effective_priority(&self) -> i32 {
        let base_priority = self.priority.value() as i32;
        let runaway_penalty = if self.is_runaway { 5 } else { 0 };
        base_priority + runaway_penalty
    }
}

/// Ordering for priority queue (min-heap based on vruntime)
impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower vruntime = higher priority
        other.vruntime.cmp(&self.vruntime)
            .then_with(|| self.priority.cmp(&other.priority))
            .then_with(|| self.task_id.cmp(&other.task_id))
    }
}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id
    }
}

impl Eq for ScheduledTask {}

/// CPU group allocation with atomic counters for lock-free updates
#[derive(Debug)]
pub struct GroupAllocation {
    pub group_id: ConsumerGroupId,
    /// CPU shares allocated to this group
    pub shares: u32,
    /// CPU percentage (if using percentage-based allocation)
    pub percentage: Option<u8>,
    /// Current CPU time used (nanoseconds) - atomic for lock-free updates
    pub cpu_time_used: AtomicU64,
    /// Number of active tasks - atomic for lock-free updates
    pub active_tasks: AtomicUsize,
    /// Target vruntime (for fair scheduling)
    pub target_vruntime: AtomicU64,
}

impl GroupAllocation {
    #[inline]
    pub fn new(group_id: ConsumerGroupId, shares: u32, percentage: Option<u8>) -> Self {
        Self {
            group_id,
            shares,
            percentage,
            cpu_time_used: AtomicU64::new(0),
            active_tasks: AtomicUsize::new(0),
            target_vruntime: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn cpu_time(&self) -> u64 {
        self.cpu_time_used.load(AtomicOrdering::Relaxed)
    }

    #[inline]
    pub fn active_task_count(&self) -> usize {
        self.active_tasks.load(AtomicOrdering::Relaxed)
    }

    #[inline]
    pub fn add_cpu_time(&self, nanoseconds: u64) {
        self.cpu_time_used.fetch_add(nanoseconds, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn inc_active_tasks(&self) {
        self.active_tasks.fetch_add(1, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn dec_active_tasks(&self) {
        self.active_tasks.fetch_sub(1, AtomicOrdering::Relaxed);
    }
}

/// CPU scheduler implementation with per-core tracking
pub struct CpuScheduler {
    /// Scheduling policy
    policy: SchedulingPolicy,
    /// All tasks
    tasks: Arc<RwLock<HashMap<TaskId, ScheduledTask>>>,
    /// Ready queue (priority queue for CFS)
    ready_queue: Arc<Mutex<BinaryHeap<ScheduledTask>>>,
    /// Per-group ready queues (for group-based scheduling)
    group_queues: Arc<RwLock<HashMap<ConsumerGroupId, VecDeque<TaskId>>>>,
    /// Group allocations
    group_allocations: Arc<RwLock<HashMap<ConsumerGroupId, GroupAllocation>>>,
    /// Running tasks
    running_tasks: Arc<RwLock<HashSet<TaskId>>>,
    /// Current quantum size (nanoseconds)
    quantum_ns: u64,
    /// Next task ID - atomic for lock-free allocation
    next_task_id: AtomicU64,
    /// Total CPU shares - atomic for lock-free updates
    total_shares: AtomicU32,
    /// Minimum vruntime (for CFS) - atomic for lock-free updates
    min_vruntime: AtomicU64,
    /// Per-core resource trackers to avoid false sharing
    per_core_trackers: Vec<PerCoreResourceTracker>,
    /// Number of CPU cores
    num_cores: usize,
    /// Runaway query detection enabled
    runaway_detection_enabled: bool,
    /// Runaway query throttle factor
    runaway_throttle_factor: f64,
    /// Scheduler statistics with atomic counters
    stats: Arc<SchedulerStats>,
}

use std::collections::HashSet;

/// Scheduler statistics with atomic counters for lock-free updates
#[derive(Debug, Default)]
pub struct SchedulerStats {
    /// Total tasks scheduled
    pub total_scheduled: AtomicU64,
    /// Total context switches
    pub context_switches: AtomicU64,
    /// Total CPU time allocated (nanoseconds)
    pub total_cpu_time: AtomicU64,
    /// Number of runaway queries detected
    pub runaway_queries_detected: AtomicU64,
    /// Number of throttled tasks
    pub throttled_tasks: AtomicU64,
    /// Average wait time (nanoseconds)
    pub avg_wait_time: AtomicU64,
}

impl SchedulerStats {
    #[inline]
    pub fn inc_scheduled(&self) {
        self.total_scheduled.fetch_add(1, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn inc_context_switches(&self) {
        self.context_switches.fetch_add(1, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn add_cpu_time(&self, ns: u64) {
        self.total_cpu_time.fetch_add(ns, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn inc_runaway(&self) {
        self.runaway_queries_detected.fetch_add(1, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn inc_throttled(&self) {
        self.throttled_tasks.fetch_add(1, AtomicOrdering::Relaxed);
    }

    pub fn snapshot(&self) -> SchedulerStatsSnapshot {
        SchedulerStatsSnapshot {
            total_scheduled: self.total_scheduled.load(AtomicOrdering::Relaxed),
            context_switches: self.context_switches.load(AtomicOrdering::Relaxed),
            total_cpu_time: self.total_cpu_time.load(AtomicOrdering::Relaxed),
            runaway_queries_detected: self.runaway_queries_detected.load(AtomicOrdering::Relaxed),
            throttled_tasks: self.throttled_tasks.load(AtomicOrdering::Relaxed),
            avg_wait_time: self.avg_wait_time.load(AtomicOrdering::Relaxed),
        }
    }
}

/// Snapshot of scheduler statistics for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStatsSnapshot {
    pub total_scheduled: u64,
    pub context_switches: u64,
    pub total_cpu_time: u64,
    pub runaway_queries_detected: u64,
    pub throttled_tasks: u64,
    pub avg_wait_time: u64,
}

impl CpuScheduler {
    /// Create a new CPU scheduler with per-core tracking
    pub fn new(policy: SchedulingPolicy) -> Self {
        let num_cores = num_cpus::get();
        let per_core_trackers = (0..num_cores)
            .map(|_| PerCoreResourceTracker::new())
            .collect();

        Self {
            policy,
            tasks: Arc::new(RwLock::new(HashMap::new())),
            ready_queue: Arc::new(Mutex::new(BinaryHeap::new())),
            group_queues: Arc::new(RwLock::new(HashMap::new())),
            group_allocations: Arc::new(RwLock::new(HashMap::new())),
            running_tasks: Arc::new(RwLock::new(HashSet::new())),
            quantum_ns: DEFAULT_QUANTUM_MS * 1_000_000,
            next_task_id: AtomicU64::new(1),
            total_shares: AtomicU32::new(1000),
            min_vruntime: AtomicU64::new(0),
            per_core_trackers,
            num_cores,
            runaway_detection_enabled: true,
            runaway_throttle_factor: 0.5,
            stats: Arc::new(SchedulerStats::default()),
        }
    }

    /// Get current core ID (approximation)
    #[inline]
    fn current_core_id(&self) -> usize {
        // Use thread ID as approximation for core ID
        // In production, use core_affinity crate for accurate core binding
        std::thread::current().id().as_u64().get() as usize % self.num_cores
    }

    /// Get per-core tracker for current core
    #[inline]
    fn current_core_tracker(&self) -> &PerCoreResourceTracker {
        let core_id = self.current_core_id();
        &self.per_core_trackers[core_id]
    }

    /// Aggregate statistics across all cores
    pub fn aggregate_core_stats(&self) -> (u64, u64, u64) {
        let mut total_cpu = 0u64;
        let mut total_memory = 0u64;
        let mut total_io = 0u64;

        for tracker in &self.per_core_trackers {
            total_cpu += tracker.cpu_usage();
            total_memory += tracker.memory_usage();
            total_io += tracker.io_pending_count();
        }

        (total_cpu, total_memory, total_io)
    }

    /// Register a consumer group with CPU allocation
    pub fn register_group(
        &self,
        group_id: ConsumerGroupId,
        shares: u32,
        percentage: Option<u8>,
    ) -> Result<()> {
        let mut allocations = self.group_allocations.write().unwrap();

        if allocations.contains_key(&group_id) {
            return self.group_exists_error(group_id);
        }

        allocations.insert(group_id, GroupAllocation::new(group_id, shares, percentage));

        // Update total shares atomically
        self.total_shares.fetch_add(shares, AtomicOrdering::Relaxed);

        // Create group queue
        let mut queues = self.group_queues.write().unwrap();
        queues.insert(group_id, VecDeque::new());

        Ok(())
    }

    /// Error path for group already exists (cold to optimize hot path)
    #[cold]
    #[inline(never)]
    fn group_exists_error(&self, group_id: ConsumerGroupId) -> Result<()> {
        Err(DbError::AlreadyExists(
            format!("Group {} already registered", group_id)
        ))
    }

    /// Add a new task to the scheduler
    #[inline]
    pub fn add_task(
        &self,
        query_id: QueryId,
        group_id: ConsumerGroupId,
        priority: PriorityLevel,
    ) -> Result<TaskId> {
        // Atomic task ID allocation (lock-free)
        let task_id = self.next_task_id.fetch_add(1, AtomicOrdering::Relaxed);

        let mut task = ScheduledTask::new(task_id, query_id, group_id, priority);

        // Set initial vruntime based on current min_vruntime
        match self.policy {
            SchedulingPolicy::CompletelyFair => {
                let min_vr = self.min_vruntime.load(AtomicOrdering::Relaxed);
                task.vruntime = min_vr;
            }
            _ => {}
        }

        // Add to tasks map
        {
            let mut tasks = self.tasks.write().unwrap();
            tasks.insert(task_id, task.clone());
        }

        // Add to appropriate queue
        match self.policy {
            SchedulingPolicy::CompletelyFair => {
                let mut ready_queue = self.ready_queue.lock().unwrap();
                ready_queue.push(task);
            }
            _ => {
                let mut group_queues = self.group_queues.write().unwrap();
                if let Some(queue) = group_queues.get_mut(&group_id) {
                    queue.push_back(task_id);
                }
            }
        }

        // Update group stats atomically
        {
            let allocations = self.group_allocations.read().unwrap();
            if let Some(alloc) = allocations.get(&group_id) {
                alloc.inc_active_tasks();
            }
        }

        // Update per-core tracker
        self.current_core_tracker().inc_tasks();

        Ok(task_id)
    }

    /// Schedule next task to run (hot path - inline for performance)
    #[inline]
    pub fn schedule_next(&self) -> Option<TaskId> {
        match self.policy {
            SchedulingPolicy::CompletelyFair => self.schedule_cfs(),
            SchedulingPolicy::FairShare => self.schedule_fair_share(),
            SchedulingPolicy::PriorityRoundRobin => self.schedule_priority_rr(),
            SchedulingPolicy::WeightedFairQueuing => self.schedule_wfq(),
        }
    }

    /// Completely Fair Scheduler (CFS) implementation (hot path - inline)
    #[inline]
    fn schedule_cfs(&self) -> Option<TaskId> {
        let mut ready_queue = self.ready_queue.lock().unwrap();

        if let Some(mut task) = ready_queue.pop() {
            task.state = TaskState::Running;
            task.last_scheduled = Some(Instant::now());
            task.schedule_count += 1;

            let task_id = task.task_id;

            // Add to running tasks
            {
                let mut running = self.running_tasks.write().unwrap();
                running.insert(task_id);
            }

            // Update task in map
            {
                let mut tasks = self.tasks.write().unwrap();
                tasks.insert(task_id, task);
            }

            // Update stats atomically (lock-free)
            self.stats.inc_scheduled();
            self.stats.inc_context_switches();

            // Update per-core tracker
            self.current_core_tracker().inc_tasks();

            Some(task_id)
        } else {
            None
        }
    }

    /// Fair-share scheduling implementation (hot path - inline)
    #[inline]
    fn schedule_fair_share(&self) -> Option<TaskId> {
        let allocations = self.group_allocations.read().unwrap();
        let mut group_queues = self.group_queues.write().unwrap();

        // Find group with lowest CPU usage relative to allocation
        let mut best_group: Option<ConsumerGroupId> = None;
        let mut best_ratio = f64::MAX;

        for (group_id, alloc) in allocations.iter() {
            if alloc.active_task_count() == 0 {
                continue;
            }

            if let Some(queue) = group_queues.get(group_id) {
                if queue.is_empty() {
                    continue;
                }

                // Calculate usage ratio (lower is better)
                let ratio = if alloc.shares > 0 {
                    alloc.cpu_time() as f64 / alloc.shares as f64
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
            if let Some(queue) = group_queues.get_mut(&group_id) {
                if let Some(task_id) = queue.pop_front() {
                    self.mark_task_running(task_id);
                    return Some(task_id);
                }
            }
        }

        None
    }

    /// Priority-based round-robin scheduling
    fn schedule_priority_rr(&self) -> Option<TaskId> {
        let group_queues = self.group_queues.read().unwrap();
        let tasks = self.tasks.read().unwrap();

        // Find highest priority ready task
        let mut best_task: Option<TaskId> = None;
        let mut best_priority = u8::MAX;

        for queue in group_queues.values() {
            for &task_id in queue.iter() {
                if let Some(task) = tasks.get(&task_id) {
                    if task.state == TaskState::Ready {
                        let eff_priority = task.effective_priority() as u8;
                        if eff_priority < best_priority {
                            best_priority = eff_priority;
                            best_task = Some(task_id);
                        }
                    }
                }
            }
        }

        if let Some(task_id) = best_task {
            drop(tasks);
            drop(group_queues);
            self.mark_task_running(task_id);
            return Some(task_id);
        }

        None
    }

    /// Weighted Fair Queuing scheduling
    fn schedule_wfq(&self) -> Option<TaskId> {
        // Similar to fair-share but with virtual time tracking
        self.schedule_fair_share()
    }

    /// Mark task as running (hot path - inline)
    #[inline]
    fn mark_task_running(&self, task_id: TaskId) {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(&task_id) {
            task.state = TaskState::Running;
            task.last_scheduled = Some(Instant::now());
            task.schedule_count += 1;
        }

        let mut running = self.running_tasks.write().unwrap();
        running.insert(task_id);

        // Update stats atomically (lock-free)
        self.stats.inc_scheduled();
        self.stats.inc_context_switches();

        // Update per-core tracker
        self.current_core_tracker().inc_tasks();
    }

    /// Update task after execution (hot path - inline)
    #[inline]
    pub fn update_task(&self, task_id: TaskId, cpu_time_ns: u64) -> Result<()> {
        let mut tasks = self.tasks.write().unwrap();
        let task = tasks.get_mut(&task_id)
            .ok_or_else(|| self.task_not_found_error(task_id))?;

        // Apply throttle if runaway
        let actual_cpu_time = if task.is_runaway {
            (cpu_time_ns as f64 * self.runaway_throttle_factor) as u64
        } else {
            cpu_time_ns
        };

        task.add_cpu_time(actual_cpu_time);

        // Update vruntime for CFS
        if self.policy == SchedulingPolicy::CompletelyFair {
            let weight = self.calculate_task_weight(task);
            task.vruntime += (actual_cpu_time as f64 / weight) as u64;
        }

        let group_id = task.group_id;
        drop(tasks);

        // Update group CPU time atomically
        {
            let allocations = self.group_allocations.read().unwrap();
            if let Some(alloc) = allocations.get(&group_id) {
                alloc.add_cpu_time(actual_cpu_time);
            }
        }

        // Update stats atomically (lock-free)
        self.stats.add_cpu_time(actual_cpu_time);

        // Update per-core tracker
        self.current_core_tracker().add_cpu_time(actual_cpu_time);

        Ok(())
    }

    /// Error path for task not found (cold to optimize hot path)
    #[cold]
    #[inline(never)]
    fn task_not_found_error(&self, task_id: TaskId) -> DbError {
        DbError::NotFound(format!("Task {} not found", task_id))
    }

    /// Calculate task weight for CFS (hot path - inline)
    #[inline]
    fn calculate_task_weight(&self, task: &ScheduledTask) -> f64 {
        // Weight based on priority (higher priority = higher weight)
        let base_weight = match task.priority.value() {
            0 => 4.0,
            1 => 3.0,
            2 => 2.5,
            3 => 2.0,
            4 => 1.5,
            5 => 1.2,
            6 => 1.0,
            _ => 0.8,
        };

        // Apply throttle factor
        base_weight * task.throttle_factor
    }

    /// Yield current task (put back in queue)
    pub fn yield_task(&self, task_id: TaskId) -> Result<()> {
        let mut running = self.running_tasks.write().unwrap();
        running.remove(&task_id);

        let mut tasks = self.tasks.write().unwrap();
        let task = tasks.get_mut(&task_id)
            .ok_or_else(|| DbError::NotFound(format!("Task {} not found", task_id)))?;

        task.state = TaskState::Ready;

        match self.policy {
            SchedulingPolicy::CompletelyFair => {
                // Re-insert into ready queue
                let mut ready_queue = self.ready_queue.lock().unwrap();
                ready_queue.push(task.clone());
            }
            _ => {
                // Re-insert into group queue
                let mut group_queues = self.group_queues.write().unwrap();
                if let Some(queue) = group_queues.get_mut(&task.group_id) {
                    queue.push_back(task_id);
                }
            }
        }

        Ok(())
    }

    /// Complete a task
    pub fn complete_task(&self, task_id: TaskId) -> Result<()> {
        let mut running = self.running_tasks.write().unwrap();
        running.remove(&task_id);

        let mut tasks = self.tasks.write().unwrap();
        let task = tasks.get_mut(&task_id)
            .ok_or_else(|| self.task_not_found_error(task_id))?;

        task.state = TaskState::Completed;
        let group_id = task.group_id;
        drop(tasks);

        // Update group stats atomically
        {
            let allocations = self.group_allocations.read().unwrap();
            if let Some(alloc) = allocations.get(&group_id) {
                alloc.dec_active_tasks();
            }
        }

        Ok(())
    }

    /// Detect and throttle runaway queries
    pub fn detect_runaway_queries(&self) -> Vec<TaskId> {
        if !self.runaway_detection_enabled {
            return Vec::new();
        }

        let mut runaway_tasks = Vec::new();
        let mut tasks = self.tasks.write().unwrap();

        for (task_id, task) in tasks.iter_mut() {
            if !task.is_runaway && task.cpu_time_used > RUNAWAY_THRESHOLD_SECS * 1_000_000_000 {
                task.is_runaway = true;
                task.throttle_factor = self.runaway_throttle_factor;
                runaway_tasks.push(*task_id);

                // Update stats atomically (lock-free)
                self.stats.inc_runaway();
                self.stats.inc_throttled();
            }
        }

        runaway_tasks
    }

    /// Get task information
    pub fn get_task(&self, task_id: TaskId) -> Option<ScheduledTask> {
        let tasks = self.tasks.read().unwrap();
        tasks.get(&task_id).cloned()
    }

    /// Get scheduler statistics (snapshot for serialization)
    pub fn get_stats(&self) -> SchedulerStatsSnapshot {
        self.stats.snapshot()
    }

    /// Get group statistics (clone needed due to Atomics not being Clone)
    pub fn get_group_stats(&self, group_id: ConsumerGroupId) -> Option<(ConsumerGroupId, u32, Option<u8>, u64, usize)> {
        let allocations = self.group_allocations.read().unwrap();
        allocations.get(&group_id).map(|alloc| {
            (
                alloc.group_id,
                alloc.shares,
                alloc.percentage,
                alloc.cpu_time(),
                alloc.active_task_count(),
            )
        })
    }

    /// Set quantum size
    pub fn set_quantum(&mut self, quantum_ms: u64) {
        self.quantum_ns = quantum_ms * 1_000_000;
    }

    /// Enable/disable runaway detection
    pub fn set_runaway_detection(&mut self, enabled: bool) {
        self.runaway_detection_enabled = enabled;
    }

    /// Set runaway throttle factor
    pub fn set_runaway_throttle_factor(&mut self, factor: f64) {
        self.runaway_throttle_factor = factor.max(0.1).min(1.0);
    }

    /// Update minimum vruntime (for CFS) - uses atomics
    pub fn update_min_vruntime(&self) {
        if self.policy != SchedulingPolicy::CompletelyFair {
            return;
        }

        let tasks = self.tasks.read().unwrap();
        let running = self.running_tasks.read().unwrap();

        let mut min_vr = u64::MAX;
        for &task_id in running.iter() {
            if let Some(task) = tasks.get(&task_id) {
                min_vr = min_vr.min(task.vruntime);
            }
        }

        if min_vr != u64::MAX {
            // Update atomically (lock-free)
            self.min_vruntime.store(min_vr, AtomicOrdering::Relaxed);
        }
    }

    /// Rebalance group allocations
    pub fn rebalance_groups(&self) -> Result<()> {
        let allocations = self.group_allocations.read().unwrap();
        let total_shares = self.total_shares.load(AtomicOrdering::Relaxed);

        for alloc in allocations.values() {
            // Calculate target vruntime based on shares
            if total_shares > 0 {
                let share_ratio = alloc.shares as f64 / total_shares as f64;
                let target_vruntime = (alloc.cpu_time() as f64 / share_ratio) as u64;
                alloc.target_vruntime.store(target_vruntime, AtomicOrdering::Relaxed);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = CpuScheduler::new(SchedulingPolicy::CompletelyFair);
        assert_eq!(scheduler.policy, SchedulingPolicy::CompletelyFair);
    }

    #[test]
    fn test_add_task() {
        let scheduler = CpuScheduler::new(SchedulingPolicy::CompletelyFair);
        scheduler.register_group(1, 1000, None).unwrap();

        let task_id = scheduler.add_task(1, 1, PriorityLevel::medium()).unwrap();
        assert!(task_id > 0);

        let task = scheduler.get_task(task_id).unwrap();
        assert_eq!(task.state, TaskState::Ready);
    }

    #[test]
    fn test_schedule_next() {
        let scheduler = CpuScheduler::new(SchedulingPolicy::CompletelyFair);
        scheduler.register_group(1, 1000, None).unwrap();

        scheduler.add_task(1, 1, PriorityLevel::medium()).unwrap();
        let scheduled = scheduler.schedule_next();
        assert!(scheduled.is_some());
    }

    #[test]
    fn test_runaway_detection() {
        let scheduler = CpuScheduler::new(SchedulingPolicy::CompletelyFair);
        scheduler.register_group(1, 1000, None).unwrap();

        let task_id = scheduler.add_task(1, 1, PriorityLevel::medium()).unwrap();

        // Simulate lots of CPU time
        scheduler.update_task(task_id, RUNAWAY_THRESHOLD_SECS * 1_000_000_000 + 1).unwrap();

        let runaway = scheduler.detect_runaway_queries();
        assert!(!runaway.is_empty());
    }
}


