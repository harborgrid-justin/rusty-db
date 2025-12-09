/// Resource Management and Quotas
/// 
/// This module provides enterprise resource management:
/// - Memory quotas and limits
/// - CPU usage tracking
/// - Disk I/O throttling
/// - Connection limits and priorities
/// - Query timeout management
/// - Resource pools and allocation

use tokio::time::sleep;
use std::time::SystemTime;
use std::time::Instant;
use crate::Result;
use crate::error::DbError;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration};

/// Resource manager for the database
pub struct ResourceManager {
    /// Memory manager
    memory_manager: Arc<MemoryManager>,
    /// CPU manager
    cpu_manager: Arc<CpuManager>,
    /// Disk I/O manager
    io_manager: Arc<IoManager>,
    /// Connection manager
    connection_manager: Arc<ConnectionManager>,
    /// Query timeout manager
    timeout_manager: Arc<QueryTimeoutManager>,
}

impl ResourceManager {
    pub fn new(config: ResourceConfig) -> Self {
        Self {
            memory_manager: Arc::new(MemoryManager::new(config.max_memory_bytes)),
            cpu_manager: Arc::new(CpuManager::new(config.max_cpu_percent)),
            io_manager: Arc::new(IoManager::new(config.max_io_bytes_per_sec)),
            connection_manager: Arc::new(ConnectionManager::new(config.max_connections)),
            timeout_manager: Arc::new(QueryTimeoutManager::new(config.default_query_timeout)),
        }
    }
    
    /// Allocate resources for a query
    pub fn allocate_query_resources(&self, query_id: String, estimated_memory: u64) -> Result<ResourceAllocation> {
        // Check memory availability
        self.memory_manager.allocate(query_id.clone(), estimated_memory)?;
        
        // Register query with timeout
        self.timeout_manager.register_query(query_id.clone())?;
        
        Ok(ResourceAllocation {
            query_id,
            allocated_memory: estimated_memory,
            start_time: Instant::now(),
        })
    }
    
    /// Release resources for a query
    pub fn release_query_resources(&self, allocation: &ResourceAllocation) {
        self.memory_manager.release(&allocation.query_id, allocation.allocated_memory);
        self.timeout_manager.complete_query(&allocation.query_id);
    }
    
    /// Check if query has timed out
    pub fn check_timeout(&self, query_id: &str) -> bool {
        self.timeout_manager.is_timed_out(query_id)
    }
    
    /// Get resource usage statistics
    pub fn get_stats(&self) -> ResourceStats {
        ResourceStats {
            memory_used: self.memory_manager.used(),
            memory_total: self.memory_manager.total(),
            cpu_usage: self.cpu_manager.usage(),
            io_bytes_per_sec: self.io_manager.current_rate(),
            active_connections: self.connection_manager.active_count(),
        }
    }
}

/// Resource configuration
#[derive(Debug, Clone)]
pub struct ResourceConfig {
    pub max_memory_bytes: u64,
    pub max_cpu_percent: u8,
    pub max_io_bytes_per_sec: u64,
    pub max_connections: usize,
    pub default_query_timeout: Duration,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 1024 * 1024 * 1024 * 4, // 4 GB
            max_cpu_percent: 80,
            max_io_bytes_per_sec: 100 * 1024 * 1024, // 100 MB/s
            max_connections: 1000,
            default_query_timeout: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Resource allocation record
pub struct ResourceAllocation {
    pub query_id: String,
    pub allocated_memory: u64,
    pub start_time: Instant,
}

/// Resource usage statistics
#[derive(Debug, Clone)]
pub struct ResourceStats {
    pub memory_used: u64,
    pub memory_total: u64,
    pub cpu_usage: u8,
    pub io_bytes_per_sec: u64,
    pub active_connections: usize,
}

/// Memory manager
pub struct MemoryManager {
    /// Total memory limit
    total_bytes: u64,
    /// Currently used memory
    used_bytes: Arc<RwLock<u64>>,
    /// Memory allocations by query
    allocations: Arc<RwLock<HashMap<String, u64>>>,
}

impl MemoryManager {
    pub fn new(total_bytes: u64) -> Self {
        Self {
            total_bytes,
            used_bytes: Arc::new(RwLock::new(0)),
            allocations: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Allocate memory for a query
    pub fn allocate(&self, query_id: String, bytes: u64) -> Result<()> {
        let mut used = self.used_bytes.write();
        
        if *used + bytes > self.total_bytes {
            return Err(DbError::InvalidOperation(format!(
                "Insufficient memory: requested {}, available {}",
                bytes,
                self.total_bytes - *used
            )));
        }
        
        *used += bytes;
        self.allocations.write().insert(query_id, bytes);
        
        Ok(())
    }
    
    /// Release allocated memory
    pub fn release(&self, query_id: &str, bytes: u64) {
        let mut used = self.used_bytes.write();
        *used = used.saturating_sub(bytes);
        self.allocations.write().remove(query_id);
    }
    
    pub fn used(&self) -> u64 {
        *self.used_bytes.read()
    }
    
    pub fn total(&self) -> u64 {
        self.total_bytes
    }
    
    pub fn available(&self) -> u64 {
        self.total_bytes - *self.used_bytes.read()
    }
}

/// CPU manager
pub struct CpuManager {
    max_percent: u8,
    current_usage: Arc<RwLock<u8>>,
}

impl CpuManager {
    pub fn new(max_percent: u8) -> Self {
        Self {
            max_percent,
            current_usage: Arc::new(RwLock::new(0)),
        }
    }
    
    pub fn usage(&self) -> u8 {
        *self.current_usage.read()
    }
    
    pub fn update_usage(&self, usage: u8) {
        *self.current_usage.write() = usage;
    }
    
    pub fn is_overloaded(&self) -> bool {
        *self.current_usage.read() > self.max_percent
    }
}

/// Disk I/O manager with throttling
pub struct IoManager {
    max_bytes_per_sec: u64,
    current_rate: Arc<RwLock<u64>>,
    last_update: Arc<RwLock<Instant>>,
    bytes_this_second: Arc<RwLock<u64>>,
}

impl IoManager {
    pub fn new(max_bytes_per_sec: u64) -> Self {
        Self {
            max_bytes_per_sec,
            current_rate: Arc::new(RwLock::new(0)),
            last_update: Arc::new(RwLock::new(Instant::now())),
            bytes_this_second: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Check if I/O operation is allowed (throttling)
    pub fn request_io(&self, bytes: u64) -> Result<()> {
        let mut last_update = self.last_update.write();
        let mut bytes_this_second = self.bytes_this_second.write();
        
        let now = Instant::now();
        if now.duration_since(*last_update) >= Duration::from_secs(1) {
            // Reset counter for new second
            *bytes_this_second = 0;
            *last_update = now;
        }
        
        if *bytes_this_second + bytes > self.max_bytes_per_sec {
            return Err(DbError::InvalidOperation(
                "I/O rate limit exceeded".to_string()
            ));
        }
        
        *bytes_this_second += bytes;
        *self.current_rate.write() = *bytes_this_second;
        
        Ok(())
    }
    
    pub fn current_rate(&self) -> u64 {
        *self.current_rate.read()
    }
}

/// Connection manager with priorities
pub struct ConnectionManager {
    max_connections: usize,
    active_connections: Arc<RwLock<HashMap<String, ConnectionInfo>>>,
}

impl ConnectionManager {
    pub fn new(max_connections: usize) -> Self {
        Self {
            max_connections,
            active_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Accept a new connection
    pub fn accept_connection(&self, conn_id: String, priority: ConnectionPriority) -> Result<()> {
        let mut conns = self.active_connections.write();
        
        if conns.len() >= self.max_connections {
            // Try to evict low-priority connection
            if !self.try_evict_low_priority(&mut conns, priority) {
                return Err(DbError::InvalidOperation(
                    "Maximum connections reached".to_string()
                ));
            }
        }
        
        conns.insert(
            conn_id.clone(),
            ConnectionInfo {
                id: conn_id,
                priority,
                connected_at: SystemTime::now(),
            },
        );
        
        Ok(())
    }
    
    /// Release a connection
    pub fn release_connection(&self, conn_id: &str) {
        self.active_connections.write().remove(conn_id);
    }
    
    pub fn active_count(&self) -> usize {
        self.active_connections.read().len()
    }
    
    fn try_evict_low_priority(
        &self,
        conns: &mut HashMap<String, ConnectionInfo>,
        new_priority: ConnectionPriority,
    ) -> bool {
        // Find lowest priority connection
        let mut lowest: Option<(String, ConnectionPriority)> = None;
        
        for (id, info) in conns.iter() {
            if info.priority < new_priority {
                if lowest.is_none() || info.priority < lowest.as_ref().unwrap().1 {
                    lowest = Some((id.clone(), info.priority));
                }
            }
        }
        
        if let Some((id, _)) = lowest {
            conns.remove(&id);
            true
        } else {
            false
        }
    }
}

/// Connection information
#[derive(Debug, Clone)]
struct ConnectionInfo {
    id: String,
    priority: ConnectionPriority,
    connected_at: SystemTime,
}

/// Connection priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConnectionPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Query timeout manager
pub struct QueryTimeoutManager {
    default_timeout: Duration,
    query_timeouts: Arc<RwLock<HashMap<String, QueryTimeout>>>,
}

impl QueryTimeoutManager {
    pub fn new(default_timeout: Duration) -> Self {
        Self {
            default_timeout,
            query_timeouts: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a query with timeout
    pub fn register_query(&self, query_id: String) -> Result<()> {
        self.register_query_with_timeout(query_id, self.default_timeout)
    }
    
    /// Register query with custom timeout
    pub fn register_query_with_timeout(&self, query_id: String, timeout: Duration) -> Result<()> {
        let mut timeouts = self.query_timeouts.write();
        
        timeouts.insert(
            query_id,
            QueryTimeout {
                start_time: Instant::now(),
                timeout,
            },
        );
        
        Ok(())
    }
    
    /// Check if query has timed out
    pub fn is_timed_out(&self, query_id: &str) -> bool {
        let timeouts = self.query_timeouts.read();
        
        if let Some(query_timeout) = timeouts.get(query_id) {
            query_timeout.start_time.elapsed() > query_timeout.timeout
        } else {
            false
        }
    }
    
    /// Complete query (remove from timeout tracking)
    pub fn complete_query(&self, query_id: &str) {
        self.query_timeouts.write().remove(query_id);
    }
}

/// Query timeout tracking
#[derive(Debug, Clone)]
struct QueryTimeout {
    start_time: Instant,
    timeout: Duration,
}

/// Resource pool for managing reusable resources
pub struct ResourcePool<T> {
    available: Arc<RwLock<Vec<T>>>,
    in_use: Arc<RwLock<usize>>,
    max_size: usize,
}

impl<T> ResourcePool<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            available: Arc::new(RwLock::new(Vec::new())),
            in_use: Arc::new(RwLock::new(0)),
            max_size,
        }
    }
    
    /// Acquire resource from pool
    pub fn acquire(&self) -> Option<T> {
        let mut available = self.available.write();
        if let Some(resource) = available.pop() {
            *self.in_use.write() += 1;
            Some(resource)
        } else {
            None
        }
    }
    
    /// Return resource to pool
    pub fn release(&self, resource: T) {
        let mut available = self.available.write();
        let mut in_use = self.in_use.write();
        
        if *in_use > 0 {
            *in_use -= 1;
        }
        
        if available.len() < self.max_size {
            available.push(resource);
        }
    }
    
    pub fn size(&self) -> usize {
        self.available.read().len() + *self.in_use.read()
    }
}

/// Quota manager for user/database limits
pub struct QuotaManager {
    quotas: Arc<RwLock<HashMap<String, Quota>>>,
}

impl QuotaManager {
    pub fn new() -> Self {
        Self {
            quotas: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Set quota for a user
    pub fn set_quota(&self, user: String, quota: Quota) {
        self.quotas.write().insert(user, quota);
    }
    
    /// Check if operation is within quota
    pub fn check_quota(&self, user: &str, operation: QuotaOperation) -> Result<()> {
        let quotas = self.quotas.read();
        
        if let Some(quota) = quotas.get(user) {
            match operation {
                QuotaOperation::Query => {
                    if quota.queries_per_hour > 0 {
                        // Would check query count in production
                        Ok(())
                    } else {
                        Err(DbError::InvalidOperation("Query quota exceeded".to_string()))
                    }
                }
                QuotaOperation::Storage(bytes) => {
                    if bytes <= quota.max_storage_bytes {
                        Ok(())
                    } else {
                        Err(DbError::InvalidOperation("Storage quota exceeded".to_string()))
                    }
                }
                QuotaOperation::Connection => {
                    if quota.max_connections > 0 {
                        Ok(())
                    } else {
                        Err(DbError::InvalidOperation("Connection quota exceeded".to_string()))
                    }
                }
            }
        } else {
            Ok(()) // No quota set, allow operation
        }
    }
}

/// User quota definition
#[derive(Debug, Clone)]
pub struct Quota {
    pub max_storage_bytes: u64,
    pub queries_per_hour: u32,
    pub max_connections: usize,
}

/// Quota operation types
pub enum QuotaOperation {
    Query,
    Storage(u64),
    Connection,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_manager() {
        let manager = MemoryManager::new(1000);
        
        assert!(manager.allocate("query1".to_string(), 300).is_ok());
        assert_eq!(manager.used(), 300);
        assert_eq!(manager.available(), 700);
        
        assert!(manager.allocate("query2".to_string(), 500).is_ok());
        assert_eq!(manager.used(), 800);
        
        // Should fail - not enough memory
        assert!(manager.allocate("query3".to_string(), 300).is_err());
        
        // Release memory
        manager.release("query1", 300);
        assert_eq!(manager.used(), 500);
        
        // Now should succeed
        assert!(manager.allocate("query3".to_string(), 300).is_ok());
    }
    
    #[test]
    fn test_io_throttling() {
        let manager = IoManager::new(1000);
        
        assert!(manager.request_io(500).is_ok());
        assert!(manager.request_io(400).is_ok());
        
        // Should fail - exceeds limit
        assert!(manager.request_io(200).is_err());
    }
    
    #[test]
    fn test_connection_manager() {
        let manager = ConnectionManager::new(3);
        
        assert!(manager.accept_connection("conn1".to_string(), ConnectionPriority::Normal).is_ok());
        assert!(manager.accept_connection("conn2".to_string(), ConnectionPriority::High).is_ok());
        assert!(manager.accept_connection("conn3".to_string(), ConnectionPriority::Low).is_ok());
        
        assert_eq!(manager.active_count(), 3);
        
        // Should evict low priority connection
        assert!(manager.accept_connection("conn4".to_string(), ConnectionPriority::High).is_ok());
        assert_eq!(manager.active_count(), 3);
    }
    
    #[test]
    fn test_query_timeout() {
        let manager = QueryTimeoutManager::new(Duration::from_millis(100));
        
        manager.register_query("query1".to_string()).unwrap();
        assert!(!manager.is_timed_out("query1"));
        
        std::thread::sleep(Duration::from_millis(150));
        assert!(manager.is_timed_out("query1"));
    }
    
    #[test]
    fn test_resource_pool() {
        let pool: ResourcePool<i32> = ResourcePool::new(5);
        
        pool.release(1);
        pool.release(2);
        
        assert_eq!(pool.size(), 2);
        
        let item = pool.acquire();
        assert!(item.is_some());
        assert_eq!(pool.size(), 2); // Still 2 total (1 in use, 1 available)
    }
    
    #[test]
    fn test_quota_manager() {
        let manager = QuotaManager::new();
        
        manager.set_quota(
            "user1".to_string(),
            Quota {
                max_storage_bytes: 1000,
                queries_per_hour: 100,
                max_connections: 5,
            },
        );
        
        assert!(manager.check_quota("user1", QuotaOperation::Storage(500)).is_ok());
        assert!(manager.check_quota("user1", QuotaOperation::Storage(1500)).is_err());
    }
}


