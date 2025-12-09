// Resource isolation mechanisms for multi-tenant environments
// Implements memory, I/O, CPU, and network isolation using Rust ownership and resource governors

use tokio::time::sleep;
use std::fmt;
use std::collections::VecDeque;
use std::time::Duration;
use std::time::Instant;
use std::collections::{HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime};

/// Isolation error types
#[derive(Debug, Clone)]
pub enum IsolationError {
    ResourceExhausted(String),
    AllocationFailed(String),
    InvalidConfiguration(String),
    ThrottlingActive(String),
    QuotaExceeded(String),
    LockContentionTimeout(String),
}

impl std::fmt::Display for IsolationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsolationError::ResourceExhausted(msg) => write!(f, "Resource exhausted: {}", msg),
            IsolationError::AllocationFailed(msg) => write!(f, "Allocation failed: {}", msg),
            IsolationError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            IsolationError::ThrottlingActive(msg) => write!(f, "Throttling active: {}", msg),
            IsolationError::QuotaExceeded(msg) => write!(f, "Quota exceeded: {}", msg),
            IsolationError::LockContentionTimeout(msg) => write!(f, "Lock contention timeout: {}", msg),
        }
    }
}

impl std::error::Error for IsolationError {}

pub type IsolationResult<T> = Result<T, IsolationError>;

/// Memory isolation using Rust ownership and resource tracking
pub struct MemoryIsolator {
    tenant_allocations: Arc<RwLock<HashMap<String, TenantMemoryAllocation>>>,
    global_memory_limit: u64,
    global_allocated: Arc<RwLock<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantMemoryAllocation {
    pub tenant_id: String,
    pub allocated_bytes: u64,
    pub quota_bytes: u64,
    pub peak_usage_bytes: u64,
    pub allocation_count: u64,
    pub deallocation_count: u64,
    pub oom_count: u64,
    pub last_allocation: SystemTime,
}

impl MemoryIsolator {
    pub fn new(global_memory_limit_mb: u64) -> Self {
        Self {
            tenant_allocations: Arc::new(RwLock::new(HashMap::new())),
            global_memory_limit: global_memory_limit_mb * 1024 * 1024,
            global_allocated: Arc::new(RwLock::new(0)),
        }
    }

    /// Allocate memory for a tenant
    pub async fn allocate(
        &self,
        tenant_id: &str,
        size_bytes: u64,
    ) -> IsolationResult<MemoryAllocation> {
        let mut allocations = self.tenant_allocations.write().await;
        let mut global_allocated = self.global_allocated.write().await;

        // Get or create tenant allocation record
        let tenant_alloc = allocations.entry(tenant_id.to_string())
            .or_insert_with(|| TenantMemoryAllocation {
                tenant_id: tenant_id.to_string(),
                allocated_bytes: 0,
                quota_bytes: 1024 * 1024 * 1024, // Default 1GB
                peak_usage_bytes: 0,
                allocation_count: 0,
                deallocation_count: 0,
                oom_count: 0,
                last_allocation: SystemTime::now(),
            });

        // Check tenant quota
        if tenant_alloc.allocated_bytes + size_bytes > tenant_alloc.quota_bytes {
            tenant_alloc.oom_count += 1;
            return Err(IsolationError::QuotaExceeded(
                format!("Tenant {} memory quota exceeded", tenant_id)
            ));
        }

        // Check global limit
        if *global_allocated + size_bytes > self.global_memory_limit {
            return Err(IsolationError::ResourceExhausted(
                "Global memory limit reached".to_string()
            ));
        }

        // Perform allocation
        tenant_alloc.allocated_bytes += size_bytes;
        tenant_alloc.allocation_count += 1;
        tenant_alloc.last_allocation = SystemTime::now();

        if tenant_alloc.allocated_bytes > tenant_alloc.peak_usage_bytes {
            tenant_alloc.peak_usage_bytes = tenant_alloc.allocated_bytes;
        }

        *global_allocated += size_bytes;

        Ok(MemoryAllocation {
            tenant_id: tenant_id.to_string(),
            size_bytes,
            allocation_time: Instant::now(),
        })
    }

    /// Deallocate memory for a tenant
    pub async fn deallocate(&self, allocation: MemoryAllocation) -> IsolationResult<()> {
        let mut allocations = self.tenant_allocations.write().await;
        let mut global_allocated = self.global_allocated.write().await;

        if let Some(tenant_alloc) = allocations.get_mut(&allocation.tenant_id) {
            tenant_alloc.allocated_bytes = tenant_alloc.allocated_bytes.saturating_sub(allocation.size_bytes);
            tenant_alloc.deallocation_count += 1;
        }

        *global_allocated = global_allocated.saturating_sub(allocation.size_bytes);

        Ok(())
    }

    /// Set memory quota for tenant
    pub async fn set_quota(&self, tenant_id: &str, quota_bytes: u64) -> IsolationResult<()> {
        let mut allocations = self.tenant_allocations.write().await;

        let tenant_alloc = allocations.entry(tenant_id.to_string())
            .or_insert_with(|| TenantMemoryAllocation {
                tenant_id: tenant_id.to_string(),
                allocated_bytes: 0,
                quota_bytes: 0,
                peak_usage_bytes: 0,
                allocation_count: 0,
                deallocation_count: 0,
                oom_count: 0,
                last_allocation: SystemTime::now(),
            });

        tenant_alloc.quota_bytes = quota_bytes;

        Ok(())
    }

    /// Get tenant memory statistics
    pub async fn get_tenant_stats(&self, tenant_id: &str) -> Option<TenantMemoryAllocation> {
        let allocations = self.tenant_allocations.read().await;
        allocations.get(tenant_id).cloned()
    }

    /// Get global memory statistics
    pub async fn get_global_stats(&self) -> MemoryGlobalStats {
        let global_allocated = *self.global_allocated.read().await;
        let allocations = self.tenant_allocations.read().await;

        MemoryGlobalStats {
            total_allocated_bytes: global_allocated,
            total_limit_bytes: self.global_memory_limit,
            utilization_percent: (global_allocated as f64 / self.global_memory_limit as f64) * 100.0,
            tenant_count: allocations.len(),
        }
    }
}

/// Memory allocation handle
pub struct MemoryAllocation {
    tenant_id: String,
    size_bytes: u64,
    allocation_time: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGlobalStats {
    pub total_allocated_bytes: u64,
    pub total_limit_bytes: u64,
    pub utilization_percent: f64,
    pub tenant_count: usize,
}

/// I/O bandwidth allocation using token bucket algorithm
pub struct IoBandwidthAllocator {
    tenant_buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
}

#[derive(Debug, Clone)]
pub struct TokenBucket {
    pub tenant_id: String,
    pub capacity: u64,           // Maximum tokens (bytes)
    pub tokens: u64,              // Current tokens
    pub refill_rate_per_sec: u64, // Tokens added per second
    pub last_refill: Instant,
}

impl TokenBucket {
    pub fn new(tenant_id: String, bandwidth_mbps: u32) -> Self {
        let bytes_per_sec = (bandwidth_mbps as u64) * 1024 * 1024;
        Self {
            tenant_id,
            capacity: bytes_per_sec * 2, // 2 second burst
            tokens: bytes_per_sec * 2,
            refill_rate_per_sec: bytes_per_sec,
            last_refill: Instant::now(),
        }
    }

    pub fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        let new_tokens = (self.refill_rate_per_sec as f64 * elapsed) as u64;
        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = now;
    }

    pub fn consume(&mut self, bytes: u64) -> bool {
        self.refill();

        if self.tokens >= bytes {
            self.tokens -= bytes;
            true
        } else {
            false
        }
    }

    pub fn available(&mut self) -> u64 {
        self.refill();
        self.tokens
    }
}

impl IoBandwidthAllocator {
    pub fn new() -> Self {
        Self {
            tenant_buckets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Configure I/O bandwidth for tenant
    pub async fn configure_tenant(&self, tenant_id: String, bandwidth_mbps: u32) {
        let mut buckets = self.tenant_buckets.write().await;
        buckets.insert(tenant_id.clone(), TokenBucket::new(tenant_id, bandwidth_mbps));
    }

    /// Request I/O bandwidth
    pub async fn request_bandwidth(
        &self,
        tenant_id: &str,
        bytes: u64,
    ) -> IsolationResult<()> {
        let mut buckets = self.tenant_buckets.write().await;

        let bucket = buckets.get_mut(tenant_id)
            .ok_or_else(|| IsolationError::InvalidConfiguration(
                format!("Tenant {} not configured", tenant_id)
            ))?;

        if bucket.consume(bytes) {
            Ok(())
        } else {
            Err(IsolationError::ThrottlingActive(
                format!("Bandwidth limit reached for tenant {}", tenant_id)
            ))
        }
    }

    /// Wait for bandwidth availability
    pub async fn wait_for_bandwidth(
        &self,
        tenant_id: &str,
        bytes: u64,
        timeout: Duration,
    ) -> IsolationResult<()> {
        let start = Instant::now();

        loop {
            match self.request_bandwidth(tenant_id, bytes).await {
                Ok(()) => return Ok(()),
                Err(_) => {
                    if start.elapsed() > timeout {
                        return Err(IsolationError::ThrottlingActive(
                            "Bandwidth wait timeout".to_string()
                        ));
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        }
    }

    /// Get available bandwidth
    pub async fn get_available(&self, tenant_id: &str) -> Option<u64> {
        let mut buckets = self.tenant_buckets.write().await;
        buckets.get_mut(tenant_id).map(|b| b.available())
    }
}

impl Default for IoBandwidthAllocator {
    fn default() -> Self {
        Self::new()
    }
}

/// CPU scheduler using fair share scheduling
pub struct CpuScheduler {
    tenant_shares: Arc<RwLock<HashMap<String, CpuShares>>>,
    total_shares: Arc<RwLock<u64>>,
    scheduling_history: Arc<RwLock<VecDeque<SchedulingEvent>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuShares {
    pub tenant_id: String,
    pub shares: u64,
    pub min_percent: u32,
    pub max_percent: u32,
    pub used_cpu_ns: u64,
    pub throttled_ns: u64,
    pub last_scheduled: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingEvent {
    pub tenant_id: String,
    pub timestamp: SystemTime,
    pub cpu_time_ns: u64,
    pub was_throttled: bool,
}

impl CpuScheduler {
    pub fn new() -> Self {
        Self {
            tenant_shares: Arc::new(RwLock::new(HashMap::new())),
            total_shares: Arc::new(RwLock::new(0)),
            scheduling_history: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Configure CPU shares for tenant
    pub async fn configure_tenant(
        &self,
        tenant_id: String,
        shares: u64,
        min_percent: u32,
        max_percent: u32,
    ) -> IsolationResult<()> {
        if min_percent > 100 || max_percent > 100 || min_percent > max_percent {
            return Err(IsolationError::InvalidConfiguration(
                "Invalid CPU percentage configuration".to_string()
            ));
        }

        let mut tenant_shares = self.tenant_shares.write().await;
        let mut total_shares = self.total_shares.write().await;

        // Remove old shares if exists
        if let Some(old) = tenant_shares.get(&tenant_id) {
            *total_shares -= old.shares;
        }

        let cpu_shares = CpuShares {
            tenant_id: tenant_id.clone(),
            shares,
            min_percent,
            max_percent,
            used_cpu_ns: 0,
            throttled_ns: 0,
            last_scheduled: SystemTime::now(),
        };

        tenant_shares.insert(tenant_id, cpu_shares);
        *total_shares += shares;

        Ok(())
    }

    /// Calculate fair CPU allocation
    pub async fn calculate_allocation(&self, tenant_id: &str) -> Option<f64> {
        let tenant_shares = self.tenant_shares.read().await;
        let total_shares = *self.total_shares.read().await;

        if total_shares == 0 {
            return None;
        }

        let shares = tenant_shares.get(tenant_id)?;
        let fair_percent = (shares.shares as f64 / total_shares as f64) * 100.0;

        // Apply min/max constraints
        let allocation = fair_percent.max(shares.min_percent as f64)
                                     .min(shares.max_percent as f64);

        Some(allocation)
    }

    /// Check if tenant should be throttled
    pub async fn should_throttle(&self, tenant_id: &str, requested_ns: u64) -> bool {
        let allocation = match self.calculate_allocation(tenant_id).await {
            Some(a) => a,
            None => return true,
        };

        let tenant_shares = self.tenant_shares.read().await;
        let shares = match tenant_shares.get(tenant_id) {
            Some(s) => s,
            None => return true,
        };

        // Simple throttling: check if max percent would be exceeded
        let total_cpu_ns = shares.used_cpu_ns + requested_ns;
        let elapsed = SystemTime::now()
            .duration_since(shares.last_scheduled)
            .unwrap_or(Duration::from_secs(1));

        let usage_percent = (total_cpu_ns as f64 / elapsed.as_nanos() as f64) * 100.0;

        usage_percent > shares.max_percent as f64
    }

    /// Record CPU usage
    pub async fn record_usage(&self, tenant_id: &str, cpu_time_ns: u64, was_throttled: bool) {
        let mut tenant_shares = self.tenant_shares.write().await;

        if let Some(shares) = tenant_shares.get_mut(tenant_id) {
            shares.used_cpu_ns += cpu_time_ns;
            if was_throttled {
                shares.throttled_ns += cpu_time_ns;
            }
            shares.last_scheduled = SystemTime::now();
        }

        drop(tenant_shares);

        // Record in history
        let mut history = self.scheduling_history.write().await;
        history.push_back(SchedulingEvent {
            tenant_id: tenant_id.to_string(),
            timestamp: SystemTime::now(),
            cpu_time_ns,
            was_throttled,
        });

        // Keep last 10000 events
        if history.len() > 10000 {
            history.pop_front();
        }
    }

    /// Get tenant CPU statistics
    pub async fn get_tenant_stats(&self, tenant_id: &str) -> Option<CpuStats> {
        let tenant_shares = self.tenant_shares.read().await;
        let shares = tenant_shares.get(tenant_id)?;

        let elapsed = SystemTime::now()
            .duration_since(shares.last_scheduled)
            .unwrap_or(Duration::from_secs(1));

        Some(CpuStats {
            tenant_id: tenant_id.to_string(),
            used_cpu_ns: shares.used_cpu_ns,
            throttled_ns: shares.throttled_ns,
            throttle_ratio: shares.throttled_ns as f64 / shares.used_cpu_ns.max(1) as f64,
            allocation_percent: self.calculate_allocation(tenant_id).await.unwrap_or(0.0),
        })
    }
}

impl Default for CpuScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuStats {
    pub tenant_id: String,
    pub used_cpu_ns: u64,
    pub throttled_ns: u64,
    pub throttle_ratio: f64,
    pub allocation_percent: f64,
}

/// Network isolation with tenant-specific ports and bandwidth
pub struct NetworkIsolator {
    tenant_ports: Arc<RwLock<HashMap<String, NetworkConfig>>>,
    port_allocator: Arc<RwLock<PortAllocator>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub tenant_id: String,
    pub assigned_port: u16,
    pub max_bandwidth_mbps: u32,
    pub max_connections: u32,
    pub current_connections: u32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

struct PortAllocator {
    base_port: u16,
    max_port: u16,
    allocated_ports: HashMap<u16, String>,
}

impl PortAllocator {
    fn new(base_port: u16, max_port: u16) -> Self {
        Self {
            base_port,
            max_port,
            allocated_ports: HashMap::new(),
        }
    }

    fn allocate(&mut self, tenant_id: &str) -> Option<u16> {
        for port in self.base_port..=self.max_port {
            if !self.allocated_ports.contains_key(&port) {
                self.allocated_ports.insert(port, tenant_id.to_string());
                return Some(port);
            }
        }
        None
    }

    fn deallocate(&mut self, port: u16) {
        self.allocated_ports.remove(&port);
    }
}

impl NetworkIsolator {
    pub fn new(base_port: u16, max_port: u16) -> Self {
        Self {
            tenant_ports: Arc::new(RwLock::new(HashMap::new())),
            port_allocator: Arc::new(RwLock::new(PortAllocator::new(base_port, max_port))),
        }
    }

    /// Allocate network resources for tenant
    pub async fn allocate_tenant(
        &self,
        tenant_id: String,
        max_bandwidth_mbps: u32,
        max_connections: u32,
    ) -> IsolationResult<u16> {
        let mut allocator = self.port_allocator.write().await;
        let port = allocator.allocate(&tenant_id)
            .ok_or_else(|| IsolationError::ResourceExhausted("No ports available".to_string()))?;

        drop(allocator);

        let config = NetworkConfig {
            tenant_id: tenant_id.clone(),
            assigned_port: port,
            max_bandwidth_mbps,
            max_connections,
            current_connections: 0,
            bytes_sent: 0,
            bytes_received: 0,
        };

        let mut tenant_ports = self.tenant_ports.write().await;
        tenant_ports.insert(tenant_id, config);

        Ok(port)
    }

    /// Check if tenant can accept new connection
    pub async fn can_accept_connection(&self, tenant_id: &str) -> IsolationResult<bool> {
        let tenant_ports = self.tenant_ports.read().await;
        let config = tenant_ports.get(tenant_id)
            .ok_or_else(|| IsolationError::InvalidConfiguration(
                format!("Tenant {} not configured", tenant_id)
            ))?;

        Ok(config.current_connections < config.max_connections)
    }

    /// Increment connection count
    pub async fn increment_connections(&self, tenant_id: &str) -> IsolationResult<()> {
        let mut tenant_ports = self.tenant_ports.write().await;
        let config = tenant_ports.get_mut(tenant_id)
            .ok_or_else(|| IsolationError::InvalidConfiguration(
                format!("Tenant {} not configured", tenant_id)
            ))?;

        if config.current_connections >= config.max_connections {
            return Err(IsolationError::ResourceExhausted("Max connections reached".to_string()));
        }

        config.current_connections += 1;
        Ok(())
    }

    /// Decrement connection count
    pub async fn decrement_connections(&self, tenant_id: &str) -> IsolationResult<()> {
        let mut tenant_ports = self.tenant_ports.write().await;
        if let Some(config) = tenant_ports.get_mut(tenant_id) {
            config.current_connections = config.current_connections.saturating_sub(1);
        }
        Ok(())
    }

    /// Record network traffic
    pub async fn record_traffic(&self, tenant_id: &str, bytes_sent: u64, bytes_received: u64) {
        let mut tenant_ports = self.tenant_ports.write().await;
        if let Some(config) = tenant_ports.get_mut(tenant_id) {
            config.bytes_sent += bytes_sent;
            config.bytes_received += bytes_received;
        }
    }

    /// Get network statistics
    pub async fn get_stats(&self, tenant_id: &str) -> Option<NetworkConfig> {
        let tenant_ports = self.tenant_ports.read().await;
        tenant_ports.get(tenant_id).cloned()
    }
}

/// Lock contention isolation to prevent one tenant from blocking others
pub struct LockContentionIsolator {
    tenant_locks: Arc<RwLock<HashMap<String, TenantLockStats>>>,
    max_wait_time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantLockStats {
    pub tenant_id: String,
    pub locks_acquired: u64,
    pub locks_waited: u64,
    pub total_wait_time_ms: u64,
    pub max_wait_time_ms: u64,
    pub lock_timeouts: u64,
}

impl LockContentionIsolator {
    pub fn new(max_wait_time: Duration) -> Self {
        Self {
            tenant_locks: Arc::new(RwLock::new(HashMap::new())),
            max_wait_time,
        }
    }

    /// Acquire lock with timeout to prevent indefinite blocking
    pub async fn acquire_lock(
        &self,
        tenant_id: &str,
        resource_id: &str,
    ) -> IsolationResult<LockHandle> {
        let start = Instant::now();

        // Simulate lock acquisition with timeout
        let wait_time = Duration::from_millis(10);
        tokio::time::sleep(wait_time).await;

        let elapsed = start.elapsed();

        if elapsed > self.max_wait_time {
            self.record_timeout(tenant_id).await;
            return Err(IsolationError::LockContentionTimeout(
                format!("Lock timeout for tenant {} on resource {}", tenant_id, resource_id)
            ));
        }

        self.record_acquisition(tenant_id, elapsed).await;

        Ok(LockHandle {
            tenant_id: tenant_id.to_string(),
            resource_id: resource_id.to_string(),
            acquired_at: Instant::now(),
        })
    }

    async fn record_acquisition(&self, tenant_id: &str, wait_time: Duration) {
        let mut tenant_locks = self.tenant_locks.write().await;
        let stats = tenant_locks.entry(tenant_id.to_string())
            .or_insert_with(|| TenantLockStats {
                tenant_id: tenant_id.to_string(),
                locks_acquired: 0,
                locks_waited: 0,
                total_wait_time_ms: 0,
                max_wait_time_ms: 0,
                lock_timeouts: 0,
            });

        stats.locks_acquired += 1;

        if wait_time.as_millis() > 0 {
            stats.locks_waited += 1;
            let wait_ms = wait_time.as_millis() as u64;
            stats.total_wait_time_ms += wait_ms;
            stats.max_wait_time_ms = stats.max_wait_time_ms.max(wait_ms);
        }
    }

    async fn record_timeout(&self, tenant_id: &str) {
        let mut tenant_locks = self.tenant_locks.write().await;
        let stats = tenant_locks.entry(tenant_id.to_string())
            .or_insert_with(|| TenantLockStats {
                tenant_id: tenant_id.to_string(),
                locks_acquired: 0,
                locks_waited: 0,
                total_wait_time_ms: 0,
                max_wait_time_ms: 0,
                lock_timeouts: 0,
            });

        stats.lock_timeouts += 1;
    }

    /// Get lock statistics
    pub async fn get_stats(&self, tenant_id: &str) -> Option<TenantLockStats> {
        let tenant_locks = self.tenant_locks.read().await;
        tenant_locks.get(tenant_id).cloned()
    }
}

pub struct LockHandle {
    tenant_id: String,
    resource_id: String,
    acquired_at: Instant,
}

/// Buffer pool partitioning per tenant
pub struct BufferPoolPartitioner {
    tenant_partitions: Arc<RwLock<HashMap<String, BufferPartition>>>,
    total_buffer_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferPartition {
    pub tenant_id: String,
    pub allocated_bytes: u64,
    pub quota_bytes: u64,
    pub cached_pages: u64,
    pub dirty_pages: u64,
    pub hit_count: u64,
    pub miss_count: u64,
    pub eviction_count: u64,
}

impl BufferPoolPartitioner {
    pub fn new(total_buffer_size_mb: u64) -> Self {
        Self {
            tenant_partitions: Arc::new(RwLock::new(HashMap::new())),
            total_buffer_size: total_buffer_size_mb * 1024 * 1024,
        }
    }

    /// Allocate buffer partition for tenant
    pub async fn allocate_partition(
        &self,
        tenant_id: String,
        quota_bytes: u64,
    ) -> IsolationResult<()> {
        let mut partitions = self.tenant_partitions.write().await;

        let total_allocated: u64 = partitions.values()
            .map(|p| p.quota_bytes)
            .sum();

        if total_allocated + quota_bytes > self.total_buffer_size {
            return Err(IsolationError::ResourceExhausted(
                "Buffer pool exhausted".to_string()
            ));
        }

        let partition = BufferPartition {
            tenant_id: tenant_id.clone(),
            allocated_bytes: 0,
            quota_bytes,
            cached_pages: 0,
            dirty_pages: 0,
            hit_count: 0,
            miss_count: 0,
            eviction_count: 0,
        };

        partitions.insert(tenant_id, partition);

        Ok(())
    }

    /// Cache page for tenant
    pub async fn cache_page(&self, tenant_id: &str, page_size: u64) -> IsolationResult<()> {
        let mut partitions = self.tenant_partitions.write().await;

        let partition = partitions.get_mut(tenant_id)
            .ok_or_else(|| IsolationError::InvalidConfiguration(
                format!("Tenant {} not configured", tenant_id)
            ))?;

        if partition.allocated_bytes + page_size > partition.quota_bytes {
            // Evict pages
            partition.eviction_count += 1;
            partition.allocated_bytes = partition.allocated_bytes.saturating_sub(page_size);
            partition.cached_pages = partition.cached_pages.saturating_sub(1);
        }

        partition.allocated_bytes += page_size;
        partition.cached_pages += 1;

        Ok(())
    }

    /// Record cache hit
    pub async fn record_hit(&self, tenant_id: &str) {
        let mut partitions = self.tenant_partitions.write().await;
        if let Some(partition) = partitions.get_mut(tenant_id) {
            partition.hit_count += 1;
        }
    }

    /// Record cache miss
    pub async fn record_miss(&self, tenant_id: &str) {
        let mut partitions = self.tenant_partitions.write().await;
        if let Some(partition) = partitions.get_mut(tenant_id) {
            partition.miss_count += 1;
        }
    }

    /// Get buffer statistics
    pub async fn get_stats(&self, tenant_id: &str) -> Option<BufferStats> {
        let partitions = self.tenant_partitions.read().await;
        let partition = partitions.get(tenant_id)?;

        let total_accesses = partition.hit_count + partition.miss_count;
        let hit_ratio = if total_accesses > 0 {
            partition.hit_count as f64 / total_accesses as f64
        } else {
            0.0
        };

        Some(BufferStats {
            tenant_id: tenant_id.to_string(),
            allocated_bytes: partition.allocated_bytes,
            quota_bytes: partition.quota_bytes,
            utilization_percent: (partition.allocated_bytes as f64 / partition.quota_bytes as f64) * 100.0,
            cached_pages: partition.cached_pages,
            hit_ratio,
            eviction_count: partition.eviction_count,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferStats {
    pub tenant_id: String,
    pub allocated_bytes: u64,
    pub quota_bytes: u64,
    pub utilization_percent: f64,
    pub cached_pages: u64,
    pub hit_ratio: f64,
    pub eviction_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_isolation() {
        let isolator = MemoryIsolator::new(1024); // 1GB

        isolator.set_quota("tenant1", 100 * 1024 * 1024).await.unwrap();

        let alloc = isolator.allocate("tenant1", 50 * 1024 * 1024).await;
        assert!(alloc.is_ok());

        let stats = isolator.get_tenant_stats("tenant1").await.unwrap();
        assert_eq!(stats.allocated_bytes, 50 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_io_bandwidth() {
        let allocator = IoBandwidthAllocator::new();
        allocator.configure_tenant("tenant1".to_string(), 100).await;

        let result = allocator.request_bandwidth("tenant1", 1024 * 1024).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cpu_scheduler() {
        let scheduler = CpuScheduler::new();

        let result = scheduler.configure_tenant(
            "tenant1".to_string(),
            1000,
            10,
            50,
        ).await;

        assert!(result.is_ok());

        let allocation = scheduler.calculate_allocation("tenant1").await;
        assert!(allocation.is_some());
    }

    #[tokio::test]
    async fn test_network_isolation() {
        let isolator = NetworkIsolator::new(10000, 20000);

        let port = isolator.allocate_tenant(
            "tenant1".to_string(),
            100,
            50,
        ).await;

        assert!(port.is_ok());
        let port_value = port.unwrap();
        assert!(port_value >= 10000 && port_value <= 20000);
    }

    #[tokio::test]
    async fn test_buffer_pool_partitioning() {
        let partitioner = BufferPoolPartitioner::new(1024); // 1GB

        let result = partitioner.allocate_partition(
            "tenant1".to_string(),
            100 * 1024 * 1024,
        ).await;

        assert!(result.is_ok());

        let result = partitioner.cache_page("tenant1", 8192).await;
        assert!(result.is_ok());

        partitioner.record_hit("tenant1").await;

        let stats = partitioner.get_stats("tenant1").await;
        assert!(stats.is_some());
    }
}


