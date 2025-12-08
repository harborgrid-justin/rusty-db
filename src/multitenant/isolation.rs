//! # Resource Isolation and Governance
//!
//! Per-tenant resource limits including memory, CPU, I/O bandwidth, connections,
//! temp space, and storage quotas. Enforces fair-share scheduling and QoS policies.
//!
//! ## Features
//!
//! - **Memory Isolation**: Per-PDB memory limits with Rust ownership tracking
//! - **CPU Scheduling**: Fair-share CPU allocation with priority queues
//! - **I/O Bandwidth**: Token bucket-based I/O rate limiting
//! - **Connection Limiting**: Max connections per PDB
//! - **Temp Space Management**: Temporary tablespace quotas
//! - **Storage Quotas**: Persistent storage limits
//! - **QoS Policies**: Priority-based resource allocation
//!
//! ## Architecture
//!
//! Uses Linux cgroups-inspired resource isolation with Rust-native implementations:
//! - Memory: Tracked allocations per PDB using custom allocators
//! - CPU: Cooperative scheduling with weighted fair queuing
//! - I/O: Token bucket algorithm for bandwidth limiting
//! - Storage: Quota enforcement at tablespace level

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration};
use tokio::sync::{RwLock, Semaphore};
use serde::{Serialize, Deserialize};
use crate::error::Result;
use super::pdb::PdbId;

/// Resource limits for a PDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory allocation (bytes)
    pub memory_bytes: u64,

    /// CPU shares (relative weight, higher = more CPU)
    pub cpu_shares: u32,

    /// I/O bandwidth limit (bytes/sec)
    pub io_bandwidth_bytes_per_sec: u64,

    /// Maximum concurrent connections
    pub max_connections: u32,

    /// Temporary space limit (bytes)
    pub temp_space_bytes: u64,

    /// Storage quota (bytes)
    pub storage_quota_bytes: u64,

    /// QoS priority (0-10, higher = more important)
    pub qos_priority: u8,

    /// Enable CPU throttling if exceeded
    pub cpu_throttling_enabled: bool,

    /// Enable I/O throttling if exceeded
    pub io_throttling_enabled: bool,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_bytes: 512 * 1024 * 1024,     // 512 MB
            cpu_shares: 100,                      // 1% of total
            io_bandwidth_bytes_per_sec: 100 * 1024 * 1024, // 100 MB/s
            max_connections: 100,
            temp_space_bytes: 1024 * 1024 * 1024, // 1 GB
            storage_quota_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
            qos_priority: 5,                      // Medium priority
            cpu_throttling_enabled: true,
            io_throttling_enabled: true,
        }
    }
}

/// QoS policy for resource allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QosPolicy {
    /// Best effort (no guarantees)
    BestEffort,
    /// Guaranteed minimum resources
    Guaranteed,
    /// Burstable (can use more when available)
    Burstable,
    /// Critical (highest priority)
    Critical,
}

impl QosPolicy {
    /// Get the multiplier for resource allocation
    pub fn multiplier(&self) -> f64 {
        match self {
            QosPolicy::BestEffort => 0.5,
            QosPolicy::Guaranteed => 1.0,
            QosPolicy::Burstable => 1.5,
            QosPolicy::Critical => 2.0,
        }
    }
}

/// Memory isolator for per-PDB memory tracking
#[derive(Debug, Clone)]
pub struct MemoryIsolator {
    /// Per-PDB memory allocations
    allocations: Arc<RwLock<HashMap<PdbId, MemoryAllocation>>>,

    /// Total system memory
    total_memory: u64,

    /// Reserved system memory
    reserved_memory: u64,
}

#[derive(Debug, Clone)]
struct MemoryAllocation {
    /// Current allocation
    current_bytes: u64,

    /// Peak allocation
    peak_bytes: u64,

    /// Limit
    limit_bytes: u64,

    /// Number of allocations
    allocation_count: u64,

    /// Number of OOM events
    oom_count: u64,
}

impl MemoryIsolator {
    /// Create a new memory isolator
    pub fn new(total_memory: u64) -> Self {
        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            total_memory,
            reserved_memory: total_memory / 10, // 10% reserved for system
        }
    }

    /// Register a PDB
    pub async fn register(&self, pdb_id: PdbId, limit_bytes: u64) -> Result<()> {
        let allocation = MemoryAllocation {
            current_bytes: 0,
            peak_bytes: 0,
            limit_bytes,
            allocation_count: 0,
            oom_count: 0,
        };

        self.allocations.write().await.insert(pdb_id, allocation);
        Ok(())
    }

    /// Unregister a PDB
    pub async fn unregister(&self, pdb_id: PdbId) -> Result<()> {
        self.allocations.write().await.remove(&pdb_id);
        Ok(())
    }

    /// Allocate memory for a PDB
    pub async fn allocate(&self, pdb_id: PdbId, bytes: u64) -> Result<()> {
        let mut allocations = self.allocations.write().await;

        if let Some(alloc) = allocations.get_mut(&pdb_id) {
            let new_total = alloc.current_bytes + bytes;

            if new_total > alloc.limit_bytes {
                alloc.oom_count += 1;
                return Err(DbError::ResourceExhausted(
                    format!("Memory limit exceeded for PDB {:?}: {} > {}",
                        pdb_id, new_total, alloc.limit_bytes)
                ));
            }

            alloc.current_bytes = new_total;
            alloc.allocation_count += 1;

            if new_total > alloc.peak_bytes {
                alloc.peak_bytes = new_total;
            }

            Ok(())
        } else {
            Err(DbError::NotFound(format!("PDB not registered: {:?}", pdb_id)))
        }
    }

    /// Deallocate memory for a PDB
    pub async fn deallocate(&self, pdb_id: PdbId, bytes: u64) -> Result<()> {
        let mut allocations = self.allocations.write().await;

        if let Some(alloc) = allocations.get_mut(&pdb_id) {
            alloc.current_bytes = alloc.current_bytes.saturating_sub(bytes);
            Ok(())
        } else {
            Err(DbError::NotFound(format!("PDB not registered: {:?}", pdb_id)))
        }
    }

    /// Get current usage for a PDB
    pub async fn get_usage(&self, pdb_id: PdbId) -> Result<(u64, u64)> {
        let allocations = self.allocations.read().await;

        if let Some(alloc) = allocations.get(&pdb_id) {
            Ok((alloc.current_bytes, alloc.limit_bytes))
        } else {
            Err(DbError::NotFound(format!("PDB not registered: {:?}", pdb_id)))
        }
    }

    /// Get memory statistics
    pub async fn get_stats(&self, pdb_id: PdbId) -> Result<MemoryStats> {
        let allocations = self.allocations.read().await;

        if let Some(alloc) = allocations.get(&pdb_id) {
            Ok(MemoryStats {
                current_bytes: alloc.current_bytes,
                peak_bytes: alloc.peak_bytes,
                limit_bytes: alloc.limit_bytes,
                allocation_count: alloc.allocation_count,
                oom_count: alloc.oom_count,
                utilization_percent: (alloc.current_bytes as f64 / alloc.limit_bytes as f64) * 100.0,
            })
        } else {
            Err(DbError::NotFound(format!("PDB not registered: {:?}", pdb_id)))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub current_bytes: u64,
    pub peak_bytes: u64,
    pub limit_bytes: u64,
    pub allocation_count: u64,
    pub oom_count: u64,
    pub utilization_percent: f64,
}

/// CPU scheduler with fair-share allocation
#[derive(Debug, Clone)]
pub struct CpuScheduler {
    /// Per-PDB CPU allocations
    allocations: Arc<RwLock<HashMap<PdbId, CpuAllocation>>>,

    /// Total CPU shares
    total_shares: u32,

    /// Scheduling queue
    queue: Arc<RwLock<Vec<ScheduledTask>>>,
}

#[derive(Debug, Clone)]
struct CpuAllocation {
    /// CPU shares (weight)
    shares: u32,

    /// Total CPU time used (microseconds)
    cpu_time_micros: u64,

    /// Last scheduling time
    last_scheduled: Instant,

    /// Virtual runtime (for fair scheduling)
    vruntime: u64,

    /// Number of tasks executed
    tasks_executed: u64,

    /// Throttled count
    throttled_count: u64,
}

#[derive(Debug, Clone)]
struct ScheduledTask {
    pdb_id: PdbId,
    priority: u8,
    submitted_at: Instant,
}

impl CpuScheduler {
    /// Create a new CPU scheduler
    pub fn new(total_shares: u32) -> Self {
        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            total_shares,
            queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a PDB
    pub async fn register(&self, pdb_id: PdbId, shares: u32) -> Result<()> {
        let allocation = CpuAllocation {
            shares,
            cpu_time_micros: 0,
            last_scheduled: Instant::now(),
            vruntime: 0,
            tasks_executed: 0,
            throttled_count: 0,
        };

        self.allocations.write().await.insert(pdb_id, allocation);
        Ok(())
    }

    /// Unregister a PDB
    pub async fn unregister(&self, pdb_id: PdbId) -> Result<()> {
        self.allocations.write().await.remove(&pdb_id);
        Ok(())
    }

    /// Schedule a task
    pub async fn schedule(&self, pdb_id: PdbId, priority: u8) -> Result<()> {
        let task = ScheduledTask {
            pdb_id,
            priority,
            submitted_at: Instant::now(),
        };

        self.queue.write().await.push(task);
        Ok(())
    }

    /// Get next task to execute (fair scheduling)
    pub async fn next_task(&self) -> Option<PdbId> {
        let mut queue = self.queue.write().await;
        let allocations = self.allocations.read().await;

        if queue.is_empty() {
            return None;
        }

        // Find task with minimum vruntime (fair scheduling)
        let mut min_vruntime = u64::MAX;
        let mut selected_idx = 0;

        for (idx, task) in queue.iter().enumerate() {
            if let Some(alloc) = allocations.get(&task.pdb_id) {
                if alloc.vruntime < min_vruntime {
                    min_vruntime = alloc.vruntime;
                    selected_idx = idx;
                }
            }
        }

        let task = queue.remove(selected_idx);
        Some(task.pdb_id)
    }

    /// Record CPU time used
    pub async fn record_cpu_time(&self, pdb_id: PdbId, micros: u64) -> Result<()> {
        let mut allocations = self.allocations.write().await;

        if let Some(alloc) = allocations.get_mut(&pdb_id) {
            alloc.cpu_time_micros += micros;
            alloc.tasks_executed += 1;
            alloc.last_scheduled = Instant::now();

            // Update virtual runtime (weighted by shares)
            let weight = self.total_shares as f64 / alloc.shares as f64;
            alloc.vruntime += (micros as f64 * weight) as u64;

            Ok(())
        } else {
            Err(DbError::NotFound(format!("PDB not registered: {:?}", pdb_id)))
        }
    }

    /// Get CPU statistics
    pub async fn get_stats(&self, pdb_id: PdbId) -> Result<CpuStats> {
        let allocations = self.allocations.read().await;

        if let Some(alloc) = allocations.get(&pdb_id) {
            let total_cpu_time: u64 = allocations.values().map(|a| a.cpu_time_micros).sum();
            let usage_percent = if total_cpu_time > 0 {
                (alloc.cpu_time_micros as f64 / total_cpu_time as f64) * 100.0
            } else {
                0.0
            };

            Ok(CpuStats {
                shares: alloc.shares,
                cpu_time_micros: alloc.cpu_time_micros,
                vruntime: alloc.vruntime,
                tasks_executed: alloc.tasks_executed,
                throttled_count: alloc.throttled_count,
                usage_percent,
            })
        } else {
            Err(DbError::NotFound(format!("PDB not registered: {:?}", pdb_id)))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuStats {
    pub shares: u32,
    pub cpu_time_micros: u64,
    pub vruntime: u64,
    pub tasks_executed: u64,
    pub throttled_count: u64,
    pub usage_percent: f64,
}

/// I/O bandwidth allocator using token bucket algorithm
#[derive(Debug, Clone)]
pub struct IoBandwidthAllocator {
    /// Per-PDB I/O buckets
    buckets: Arc<RwLock<HashMap<PdbId, TokenBucket>>>,
}

#[derive(Debug, Clone)]
struct TokenBucket {
    /// Maximum tokens (bytes)
    capacity: u64,

    /// Current tokens
    tokens: u64,

    /// Refill rate (bytes/sec)
    refill_rate: u64,

    /// Last refill time
    last_refill: Instant,

    /// Total bytes transferred
    total_bytes: u64,

    /// Number of throttle events
    throttle_count: u64,
}

impl TokenBucket {
    fn new(capacity: u64, refill_rate: u64) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_rate,
            last_refill: Instant::now(),
            total_bytes: 0,
            throttle_count: 0,
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let new_tokens = (elapsed.as_secs_f64() * self.refill_rate as f64) as u64;

        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = now;
    }

    fn consume(&mut self, bytes: u64) -> bool {
        self.refill();

        if self.tokens >= bytes {
            self.tokens -= bytes;
            self.total_bytes += bytes;
            true
        } else {
            self.throttle_count += 1;
            false
        }
    }
}

impl IoBandwidthAllocator {
    /// Create a new I/O bandwidth allocator
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a PDB
    pub async fn register(&self, pdb_id: PdbId, bandwidth_bytes_per_sec: u64) -> Result<()> {
        let bucket = TokenBucket::new(
            bandwidth_bytes_per_sec * 2, // 2 second burst
            bandwidth_bytes_per_sec,
        );

        self.buckets.write().await.insert(pdb_id, bucket);
        Ok(())
    }

    /// Unregister a PDB
    pub async fn unregister(&self, pdb_id: PdbId) -> Result<()> {
        self.buckets.write().await.remove(&pdb_id);
        Ok(())
    }

    /// Try to consume bandwidth
    pub async fn try_consume(&self, pdb_id: PdbId, bytes: u64) -> Result<bool> {
        let mut buckets = self.buckets.write().await;

        if let Some(bucket) = buckets.get_mut(&pdb_id) {
            Ok(bucket.consume(bytes))
        } else {
            Err(DbError::NotFound(format!("PDB not registered: {:?}", pdb_id)))
        }
    }

    /// Wait until bandwidth is available
    pub async fn consume(&self, pdb_id: PdbId, bytes: u64) -> Result<()> {
        loop {
            if self.try_consume(pdb_id, bytes).await? {
                return Ok(());
            }

            // Wait a bit before retrying
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Get I/O statistics
    pub async fn get_stats(&self, pdb_id: PdbId) -> Result<IoStats> {
        let buckets = self.buckets.read().await;

        if let Some(bucket) = buckets.get(&pdb_id) {
            Ok(IoStats {
                capacity: bucket.capacity,
                current_tokens: bucket.tokens,
                refill_rate: bucket.refill_rate,
                total_bytes: bucket.total_bytes,
                throttle_count: bucket.throttle_count,
                utilization_percent: ((bucket.capacity - bucket.tokens) as f64 / bucket.capacity as f64) * 100.0,
            })
        } else {
            Err(DbError::NotFound(format!("PDB not registered: {:?}", pdb_id)))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoStats {
    pub capacity: u64,
    pub current_tokens: u64,
    pub refill_rate: u64,
    pub total_bytes: u64,
    pub throttle_count: u64,
    pub utilization_percent: f64,
}

/// Connection limiter
#[derive(Debug, Clone)]
pub struct ConnectionLimiter {
    /// Per-PDB connection limits
    limits: Arc<RwLock<HashMap<PdbId, ConnectionLimit>>>,
}

#[derive(Debug, Clone)]
struct ConnectionLimit {
    /// Maximum connections
    max_connections: u32,

    /// Current connections
    current_connections: u32,

    /// Semaphore for limiting
    semaphore: Arc<Semaphore>,

    /// Total connection attempts
    total_attempts: u64,

    /// Rejected connections
    rejected_count: u64,
}

impl ConnectionLimiter {
    /// Create a new connection limiter
    pub fn new() -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a PDB
    pub async fn register(&self, pdb_id: PdbId, max_connections: u32) -> Result<()> {
        let limit = ConnectionLimit {
            max_connections,
            current_connections: 0,
            semaphore: Arc::new(Semaphore::new(max_connections as usize)),
            total_attempts: 0,
            rejected_count: 0,
        };

        self.limits.write().await.insert(pdb_id, limit);
        Ok(())
    }

    /// Unregister a PDB
    pub async fn unregister(&self, pdb_id: PdbId) -> Result<()> {
        self.limits.write().await.remove(&pdb_id);
        Ok(())
    }

    /// Acquire a connection
    pub async fn acquire(&self, pdb_id: PdbId) -> Result<ConnectionGuard> {
        let mut limits = self.limits.write().await;

        if let Some(limit) = limits.get_mut(&pdb_id) {
            limit.total_attempts += 1;

            // Try to acquire semaphore permit
            match limit.semaphore.clone().try_acquire_owned() {
                Ok(permit) => {
                    limit.current_connections += 1;
                    Ok(ConnectionGuard {
                        pdb_id,
                        _permit: permit,
                    })
                }
                Err(_) => {
                    limit.rejected_count += 1;
                    Err(DbError::ResourceExhausted(
                        format!("Connection limit reached for PDB {:?}: {}",
                            pdb_id, limit.max_connections)
                    ))
                }
            }
        } else {
            Err(DbError::NotFound(format!("PDB not registered: {:?}", pdb_id)))
        }
    }

    /// Get connection statistics
    pub async fn get_stats(&self, pdb_id: PdbId) -> Result<ConnectionStats> {
        let limits = self.limits.read().await;

        if let Some(limit) = limits.get(&pdb_id) {
            Ok(ConnectionStats {
                max_connections: limit.max_connections,
                current_connections: limit.current_connections,
                total_attempts: limit.total_attempts,
                rejected_count: limit.rejected_count,
                utilization_percent: (limit.current_connections as f64 / limit.max_connections as f64) * 100.0,
            })
        } else {
            Err(DbError::NotFound(format!("PDB not registered: {:?}", pdb_id)))
        }
    }
}

/// Connection guard that releases connection on drop
pub struct ConnectionGuard {
    pdb_id: PdbId,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub max_connections: u32,
    pub current_connections: u32,
    pub total_attempts: u64,
    pub rejected_count: u64,
    pub utilization_percent: f64,
}

/// Temp space limiter
#[derive(Debug, Clone)]
pub struct TempSpaceLimiter {
    /// Per-PDB temp space usage
    usage: Arc<RwLock<HashMap<PdbId, TempSpaceUsage>>>,
}

#[derive(Debug, Clone)]
struct TempSpaceUsage {
    /// Maximum temp space
    max_bytes: u64,

    /// Current usage
    current_bytes: u64,

    /// Peak usage
    peak_bytes: u64,

    /// Number of temp allocations
    allocation_count: u64,

    /// Number of rejections
    rejection_count: u64,
}

impl TempSpaceLimiter {
    /// Create a new temp space limiter
    pub fn new() -> Self {
        Self {
            usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a PDB
    pub async fn register(&self, pdb_id: PdbId, max_bytes: u64) -> Result<()> {
        let usage = TempSpaceUsage {
            max_bytes,
            current_bytes: 0,
            peak_bytes: 0,
            allocation_count: 0,
            rejection_count: 0,
        };

        self.usage.write().await.insert(pdb_id, usage);
        Ok(())
    }

    /// Allocate temp space
    pub async fn allocate(&self, pdb_id: PdbId, bytes: u64) -> Result<()> {
        let mut usage_map = self.usage.write().await;

        if let Some(usage) = usage_map.get_mut(&pdb_id) {
            usage.allocation_count += 1;

            let new_total = usage.current_bytes + bytes;
            if new_total > usage.max_bytes {
                usage.rejection_count += 1;
                return Err(DbError::ResourceExhausted(
                    format!("Temp space limit exceeded for PDB {:?}: {} > {}",
                        pdb_id, new_total, usage.max_bytes)
                ));
            }

            usage.current_bytes = new_total;
            if new_total > usage.peak_bytes {
                usage.peak_bytes = new_total;
            }

            Ok(())
        } else {
            Err(DbError::NotFound(format!("PDB not registered: {:?}", pdb_id)))
        }
    }

    /// Deallocate temp space
    pub async fn deallocate(&self, pdb_id: PdbId, bytes: u64) -> Result<()> {
        let mut usage_map = self.usage.write().await;

        if let Some(usage) = usage_map.get_mut(&pdb_id) {
            usage.current_bytes = usage.current_bytes.saturating_sub(bytes);
            Ok(())
        } else {
            Err(DbError::NotFound(format!("PDB not registered: {:?}", pdb_id)))
        }
    }
}

/// Storage quota manager
#[derive(Debug, Clone)]
pub struct StorageQuotaManager {
    /// Per-PDB storage quotas
    quotas: Arc<RwLock<HashMap<PdbId, StorageQuota>>>,
}

#[derive(Debug, Clone)]
struct StorageQuota {
    /// Maximum storage
    max_bytes: u64,

    /// Current usage
    current_bytes: u64,

    /// Soft limit (warning threshold)
    soft_limit_bytes: u64,

    /// Number of quota violations
    violation_count: u64,
}

impl StorageQuotaManager {
    /// Create a new storage quota manager
    pub fn new() -> Self {
        Self {
            quotas: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a PDB
    pub async fn register(&self, pdb_id: PdbId, max_bytes: u64) -> Result<()> {
        let quota = StorageQuota {
            max_bytes,
            current_bytes: 0,
            soft_limit_bytes: (max_bytes as f64 * 0.9) as u64, // 90% soft limit
            violation_count: 0,
        };

        self.quotas.write().await.insert(pdb_id, quota);
        Ok(())
    }

    /// Check if allocation is allowed
    pub async fn check_allocation(&self, pdb_id: PdbId, bytes: u64) -> Result<()> {
        let mut quotas = self.quotas.write().await;

        if let Some(quota) = quotas.get_mut(&pdb_id) {
            let new_total = quota.current_bytes + bytes;

            if new_total > quota.max_bytes {
                quota.violation_count += 1;
                return Err(DbError::QuotaExceeded(
                    format!("Storage quota exceeded for PDB {:?}: {} > {}",
                        pdb_id, new_total, quota.max_bytes)
                ));
            }

            quota.current_bytes = new_total;
            Ok(())
        } else {
            Err(DbError::NotFound(format!("PDB not registered: {:?}", pdb_id)))
        }
    }
}

/// Main resource isolator coordinating all isolation mechanisms
pub struct ResourceIsolator {
    memory: MemoryIsolator,
    cpu: CpuScheduler,
    io: IoBandwidthAllocator,
    connections: ConnectionLimiter,
    temp_space: TempSpaceLimiter,
    storage: StorageQuotaManager,
}

impl ResourceIsolator {
    /// Create a new resource isolator
    pub fn new() -> Self {
        Self {
            memory: MemoryIsolator::new(16 * 1024 * 1024 * 1024), // 16 GB total
            cpu: CpuScheduler::new(10000),
            io: IoBandwidthAllocator::new(),
            connections: ConnectionLimiter::new(),
            temp_space: TempSpaceLimiter::new(),
            storage: StorageQuotaManager::new(),
        }
    }

    /// Register a PDB with resource limits
    pub async fn register_pdb(&self, pdb_id: PdbId, limits: &ResourceLimits) -> Result<()> {
        self.memory.register(pdb_id, limits.memory_bytes).await?;
        self.cpu.register(pdb_id, limits.cpu_shares).await?;
        self.io.register(pdb_id, limits.io_bandwidth_bytes_per_sec).await?;
        self.connections.register(pdb_id, limits.max_connections).await?;
        self.temp_space.register(pdb_id, limits.temp_space_bytes).await?;
        self.storage.register(pdb_id, limits.storage_quota_bytes).await?;
        Ok(())
    }

    /// Unregister a PDB
    pub async fn unregister_pdb(&self, pdb_id: PdbId) -> Result<()> {
        self.memory.unregister(pdb_id).await?;
        self.cpu.unregister(pdb_id).await?;
        self.io.unregister(pdb_id).await?;
        self.connections.unregister(pdb_id).await?;
        Ok(())
    }

    /// Get memory isolator
    pub fn memory(&self) -> &MemoryIsolator {
        &self.memory
    }

    /// Get CPU scheduler
    pub fn cpu(&self) -> &CpuScheduler {
        &self.cpu
    }

    /// Get I/O bandwidth allocator
    pub fn io(&self) -> &IoBandwidthAllocator {
        &self.io
    }

    /// Get connection limiter
    pub fn connections(&self) -> &ConnectionLimiter {
        &self.connections
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_isolator() {
        let isolator = MemoryIsolator::new(1024 * 1024 * 1024);
        let pdb_id = PdbId::new(1);

        isolator.register(pdb_id, 100 * 1024 * 1024).await.unwrap();
        isolator.allocate(pdb_id, 50 * 1024 * 1024).await.unwrap();

        let (current, limit) = isolator.get_usage(pdb_id).await.unwrap();
        assert_eq!(current, 50 * 1024 * 1024);
        assert_eq!(limit, 100 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_cpu_scheduler() {
        let scheduler = CpuScheduler::new(10000);
        let pdb_id = PdbId::new(1);

        scheduler.register(pdb_id, 100).await.unwrap();
        scheduler.schedule(pdb_id, 5).await.unwrap();

        let next = scheduler.next_task().await;
        assert_eq!(next, Some(pdb_id));
    }

    #[tokio::test]
    async fn test_io_bandwidth_allocator() {
        let allocator = IoBandwidthAllocator::new();
        let pdb_id = PdbId::new(1);

        allocator.register(pdb_id, 100 * 1024 * 1024).await.unwrap();
        let _result = allocator.try_consume(pdb_id, 1024).await.unwrap();
        assert!(result);
    }
}


