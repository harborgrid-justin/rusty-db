//! Memory Manager for Resource Management
//!
//! This module implements PGA memory limits, session memory quotas,
//! automatic memory management, and out-of-memory prevention.

use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::time::{Duration};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use super::consumer_groups::ConsumerGroupId;
use super::session_control::SessionId;

/// Memory pool identifier
pub type MemoryPoolId = u64;

/// Memory region identifier
pub type MemoryRegionId = u64;

/// Memory allocation size in bytes
pub type MemorySize = u64;

/// Memory allocation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllocationStrategy {
    /// Automatic memory management
    Automatic,
    /// Manual memory management
    Manual,
    /// Hybrid approach
    Hybrid,
}

/// Memory pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MemoryPressure {
    /// No pressure, plenty of memory
    None,
    /// Low pressure, can allocate freely
    Low,
    /// Medium pressure, should be cautious
    Medium,
    /// High pressure, need to free memory
    High,
    /// Critical pressure, OOM imminent
    Critical,
}

/// Memory pool type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPoolType {
    /// Shared Global Area (SGA-like)
    SharedGlobal,
    /// Program Global Area (PGA-like) - per-session
    ProgramGlobal,
    /// Sort/Hash area
    WorkArea,
    /// Buffer cache
    BufferCache,
    /// Shared pool for SQL/metadata
    SharedPool,
    /// Large pool for large allocations
    LargePool,
}

/// Memory pool definition
#[derive(Debug, Clone)]
pub struct MemoryPool {
    /// Pool identifier
    pub id: MemoryPoolId,
    /// Pool name
    pub name: String,
    /// Pool type
    pub pool_type: MemoryPoolType,
    /// Maximum size in bytes
    pub max_size: MemorySize,
    /// Current allocated size
    pub allocated_size: MemorySize,
    /// Target size (for automatic management)
    pub target_size: Option<MemorySize>,
    /// Minimum size
    pub min_size: MemorySize,
    /// Whether auto-tuning is enabled
    pub auto_tune: bool,
    /// Creation time
    pub created_at: SystemTime,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(
        id: MemoryPoolId,
        name: String,
        pool_type: MemoryPoolType,
        max_size: MemorySize,
        min_size: MemorySize,
    ) -> Self {
        Self {
            id,
            name,
            pool_type,
            max_size,
            allocated_size: 0,
            target_size: None,
            min_size,
            auto_tune: false,
            created_at: SystemTime::now(),
        }
    }

    /// Check if allocation is possible
    pub fn can_allocate(&self, size: MemorySize) -> bool {
        self.allocated_size + size <= self.max_size
    }

    /// Get available memory
    pub fn available(&self) -> MemorySize {
        self.max_size.saturating_sub(self.allocated_size)
    }

    /// Get usage percentage
    pub fn usage_pct(&self) -> f64 {
        if self.max_size == 0 {
            0.0
        } else {
            (self.allocated_size as f64 / self.max_size as f64) * 100.0
        }
    }
}

/// Session memory quota
#[derive(Debug, Clone)]
pub struct SessionMemoryQuota {
    /// Session identifier
    pub session_id: SessionId,
    /// Consumer group
    pub group_id: ConsumerGroupId,
    /// Maximum PGA memory
    pub max_pga_memory: MemorySize,
    /// Current PGA usage
    pub current_pga_usage: MemorySize,
    /// Maximum work area size
    pub max_work_area: MemorySize,
    /// Current work area usage
    pub current_work_area: MemorySize,
    /// Number of memory allocations
    pub allocation_count: u64,
    /// Peak memory usage
    pub peak_usage: MemorySize,
    /// Last allocation time
    pub last_allocation: Option<Instant>,
}

impl SessionMemoryQuota {
    /// Create a new session quota
    pub fn new(
        session_id: SessionId,
        group_id: ConsumerGroupId,
        max_pga_memory: MemorySize,
        max_work_area: MemorySize,
    ) -> Self {
        Self {
            session_id,
            group_id,
            max_pga_memory,
            current_pga_usage: 0,
            max_work_area,
            current_work_area: 0,
            allocation_count: 0,
            peak_usage: 0,
            last_allocation: None,
        }
    }

    /// Check if can allocate
    pub fn can_allocate_pga(&self, size: MemorySize) -> bool {
        self.current_pga_usage + size <= self.max_pga_memory
    }

    /// Check if can allocate work area
    pub fn can_allocate_work_area(&self, size: MemorySize) -> bool {
        self.current_work_area + size <= self.max_work_area
    }

    /// Allocate memory
    pub fn allocate(&mut self, size: MemorySize, is_work_area: bool) -> Result<()> {
        if is_work_area {
            if !self.can_allocate_work_area(size) {
                return Err(DbError::ResourceExhausted(
                    "Work area memory limit exceeded".to_string()
                ));
            }
            self.current_work_area += size;
        } else {
            if !self.can_allocate_pga(size) {
                return Err(DbError::ResourceExhausted(
                    "PGA memory limit exceeded".to_string()
                ));
            }
            self.current_pga_usage += size;
        }

        self.allocation_count += 1;
        self.last_allocation = Some(Instant::now());
        self.peak_usage = self.peak_usage.max(self.current_pga_usage);

        Ok(())
    }

    /// Deallocate memory
    pub fn deallocate(&mut self, size: MemorySize, is_work_area: bool) {
        if is_work_area {
            self.current_work_area = self.current_work_area.saturating_sub(size);
        } else {
            self.current_pga_usage = self.current_pga_usage.saturating_sub(size);
        }
    }

    /// Get total usage
    pub fn total_usage(&self) -> MemorySize {
        self.current_pga_usage + self.current_work_area
    }
}

/// Group memory limits
#[derive(Debug, Clone)]
pub struct GroupMemoryLimits {
    pub group_id: ConsumerGroupId,
    /// Maximum total memory for all sessions in group
    pub max_group_memory: Option<MemorySize>,
    /// Maximum PGA per session
    pub max_session_pga: Option<MemorySize>,
    /// Current total usage
    pub current_total_usage: MemorySize,
    /// Number of active sessions
    pub active_sessions: usize,
}

/// Memory advisor recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAdvisorRecommendation {
    /// Pool being analyzed
    pub pool_id: MemoryPoolId,
    /// Current size
    pub current_size: MemorySize,
    /// Recommended size
    pub recommended_size: MemorySize,
    /// Estimated benefit
    pub estimated_benefit: f64,
    /// Reason for recommendation
    pub reason: String,
    /// Timestamp
    pub timestamp: SystemTime,
}

/// Memory manager
pub struct MemoryManager {
    /// Memory pools
    pools: Arc<RwLock<HashMap<MemoryPoolId, MemoryPool>>>,
    /// Session quotas
    session_quotas: Arc<RwLock<HashMap<SessionId, SessionMemoryQuota>>>,
    /// Group limits
    group_limits: Arc<RwLock<HashMap<ConsumerGroupId, GroupMemoryLimits>>>,
    /// Allocation strategy
    strategy: AllocationStrategy,
    /// Total system memory
    total_system_memory: MemorySize,
    /// Maximum database memory
    max_db_memory: MemorySize,
    /// Current database memory usage
    current_db_usage: Arc<RwLock<MemorySize>>,
    /// Memory pressure level
    pressure_level: Arc<RwLock<MemoryPressure>>,
    /// Pressure thresholds
    pressure_thresholds: PressureThresholds,
    /// Next pool ID
    next_pool_id: Arc<RwLock<MemoryPoolId>>,
    /// Auto-tuning enabled
    auto_tuning_enabled: bool,
    /// Statistics
    stats: Arc<RwLock<MemoryStats>>,
}

/// Pressure thresholds (percentage of max memory)
#[derive(Debug, Clone)]
struct PressureThresholds {
    low: f64,      // 60%
    medium: f64,   // 75%
    high: f64,     // 85%
    critical: f64, // 95%
}

impl Default for PressureThresholds {
    fn default() -> Self {
        Self {
            low: 0.60,
            medium: 0.75,
            high: 0.85,
            critical: 0.95,
        }
    }
}

/// Memory statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Total allocations
    pub total_allocations: u64,
    /// Total deallocations
    pub total_deallocations: u64,
    /// Failed allocations (OOM)
    pub failed_allocations: u64,
    /// Peak memory usage
    pub peak_usage: MemorySize,
    /// Number of memory pressure events
    pub pressure_events: u64,
    /// Number of automatic adjustments
    pub auto_adjustments: u64,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(
        total_system_memory: MemorySize,
        max_db_memory: MemorySize,
        strategy: AllocationStrategy,
    ) -> Result<Self> {
        let mut manager = Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            session_quotas: Arc::new(RwLock::new(HashMap::new())),
            group_limits: Arc::new(RwLock::new(HashMap::new())),
            strategy,
            total_system_memory,
            max_db_memory,
            current_db_usage: Arc::new(RwLock::new(0)),
            pressure_level: Arc::new(RwLock::new(MemoryPressure::None)),
            pressure_thresholds: PressureThresholds::default(),
            next_pool_id: Arc::new(RwLock::new(1)),
            auto_tuning_enabled: strategy == AllocationStrategy::Automatic,
            stats: Arc::new(RwLock::new(MemoryStats::default())),
        };

        // Create default pools
        manager.create_default_pools()?;
        Ok(manager)
    }

    /// Create default memory pools
    fn create_default_pools(&mut self) -> Result<()> {
        // Calculate default sizes (60% for SGA, 40% for PGA)
        let sga_size = (self.max_db_memory as f64 * 0.60) as MemorySize;
        let pga_size = (self.max_db_memory as f64 * 0.40) as MemorySize;

        // Shared Global Area
        let sga_pool = MemoryPool::new(
            1,
            "SGA".to_string(),
            MemoryPoolType::SharedGlobal,
            sga_size,
            sga_size / 2,
        );
        self.register_pool(sga_pool)?;

        // Buffer Cache (within SGA)
        let buffer_cache = MemoryPool::new(
            2,
            "BUFFER_CACHE".to_string(),
            MemoryPoolType::BufferCache,
            (sga_size as f64 * 0.60) as MemorySize,
            (sga_size as f64 * 0.30) as MemorySize,
        );
        self.register_pool(buffer_cache)?;

        // Shared Pool (within SGA)
        let shared_pool = MemoryPool::new(
            3,
            "SHARED_POOL".to_string(),
            MemoryPoolType::SharedPool,
            (sga_size as f64 * 0.30) as MemorySize,
            (sga_size as f64 * 0.15) as MemorySize,
        );
        self.register_pool(shared_pool)?;

        // Large Pool (within SGA)
        let large_pool = MemoryPool::new(
            4,
            "LARGE_POOL".to_string(),
            MemoryPoolType::LargePool,
            (sga_size as f64 * 0.10) as MemorySize,
            (sga_size as f64 * 0.05) as MemorySize,
        );
        self.register_pool(large_pool)?;

        // Program Global Area
        let pga_pool = MemoryPool::new(
            5,
            "PGA".to_string(),
            MemoryPoolType::ProgramGlobal,
            pga_size,
            pga_size / 4,
        );
        self.register_pool(pga_pool)?;

        Ok(())
    }

    /// Register a memory pool
    fn register_pool(&mut self, pool: MemoryPool) -> Result<()> {
        let mut pools = self.pools.write().unwrap();
        if pools.contains_key(&pool.id) {
            return Err(DbError::AlreadyExists(
                format!("Pool {} already exists", pool.id)
            ));
        }
        pools.insert(pool.id, pool);
        Ok(())
    }

    /// Create a session memory quota
    pub fn create_session_quota(
        &self,
        session_id: SessionId,
        group_id: ConsumerGroupId,
        max_pga_memory: Option<MemorySize>,
        max_work_area: Option<MemorySize>,
    ) -> Result<()> {
        // Get group limits
        let (pga_limit, work_area_limit) = {
            let group_limits = self.group_limits.read().unwrap();
            if let Some(limits) = group_limits.get(&group_id) {
                (
                    limits.max_session_pga,
                    Some(limits.max_session_pga.unwrap_or(100 * 1024 * 1024) / 4),
                )
            } else {
                (None, None)
            }
        };

        // Use provided limits or group limits or defaults
        let max_pga = max_pga_memory
            .or(pga_limit)
            .unwrap_or(100 * 1024 * 1024); // 100 MB default

        let max_work = max_work_area
            .or(work_area_limit)
            .unwrap_or(max_pga / 4);

        let quota = SessionMemoryQuota::new(session_id, group_id, max_pga, max_work);

        let mut quotas = self.session_quotas.write().unwrap();
        quotas.insert(session_id, quota);

        // Update group session count
        let mut group_limits = self.group_limits.write().unwrap();
        if let Some(limits) = group_limits.get_mut(&group_id) {
            limits.active_sessions += 1;
        }

        Ok(())
    }

    /// Remove session quota
    pub fn remove_session_quota(&self, session_id: SessionId) -> Result<()> {
        let mut quotas = self.session_quotas.write().unwrap();
        if let Some(quota) = quotas.remove(&session_id) {
            // Update group session count
            let mut group_limits = self.group_limits.write().unwrap();
            if let Some(limits) = group_limits.get_mut(&quota.group_id) {
                if limits.active_sessions > 0 {
                    limits.active_sessions -= 1;
                }
                limits.current_total_usage = limits
                    .current_total_usage
                    .saturating_sub(quota.total_usage());
            }
        }
        Ok(())
    }

    /// Allocate memory from a pool
    pub fn allocate_from_pool(
        &self,
        pool_id: MemoryPoolId,
        size: MemorySize,
    ) -> Result<()> {
        // Check pressure level
        let pressure = *self.pressure_level.read().unwrap();
        if pressure == MemoryPressure::Critical {
            return Err(DbError::ResourceExhausted(
                "Critical memory pressure, allocation denied".to_string()
            ));
        }

        let mut pools = self.pools.write().unwrap();
        let pool = pools.get_mut(&pool_id)
            .ok_or_else(|| DbError::NotFound(format!("Pool {} not found", pool_id)))?;

        if !pool.can_allocate(size) {
            let mut stats = self.stats.write().unwrap();
            stats.failed_allocations += 1;
            return Err(DbError::ResourceExhausted(
                format!("Pool {} cannot allocate {} bytes", pool.name, size)
            ));
        }

        pool.allocated_size += size;

        // Update global usage
        {
            let mut db_usage = self.current_db_usage.write().unwrap();
            *db_usage += size;
        }

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_allocations += 1;
            let current_usage = *self.current_db_usage.read().unwrap();
            stats.peak_usage = stats.peak_usage.max(current_usage);
        }

        // Update pressure level
        self.update_pressure_level();

        Ok(())
    }

    /// Deallocate memory from a pool
    pub fn deallocate_from_pool(
        &self,
        pool_id: MemoryPoolId,
        size: MemorySize,
    ) -> Result<()> {
        let mut pools = self.pools.write().unwrap();
        let pool = pools.get_mut(&pool_id)
            .ok_or_else(|| DbError::NotFound(format!("Pool {} not found", pool_id)))?;

        pool.allocated_size = pool.allocated_size.saturating_sub(size);

        // Update global usage
        {
            let mut db_usage = self.current_db_usage.write().unwrap();
            *db_usage = db_usage.saturating_sub(size);
        }

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_deallocations += 1;
        }

        // Update pressure level
        self.update_pressure_level();

        Ok(())
    }

    /// Allocate session memory
    pub fn allocate_session_memory(
        &self,
        session_id: SessionId,
        size: MemorySize,
        is_work_area: bool,
    ) -> Result<()> {
        let mut quotas = self.session_quotas.write().unwrap();
        let quota = quotas.get_mut(&session_id)
            .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))?;

        quota.allocate(size, is_work_area)?;

        // Update group usage
        {
            let mut group_limits = self.group_limits.write().unwrap();
            if let Some(limits) = group_limits.get_mut(&quota.group_id) {
                limits.current_total_usage += size;

                // Check group limit
                if let Some(max_group) = limits.max_group_memory {
                    if limits.current_total_usage > max_group {
                        // Rollback
                        quota.deallocate(size, is_work_area);
                        limits.current_total_usage -= size;
                        return Err(DbError::ResourceExhausted(
                            "Group memory limit exceeded".to_string()
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Deallocate session memory
    pub fn deallocate_session_memory(
        &self,
        session_id: SessionId,
        size: MemorySize,
        is_work_area: bool,
    ) -> Result<()> {
        let mut quotas = self.session_quotas.write().unwrap();
        if let Some(quota) = quotas.get_mut(&session_id) {
            quota.deallocate(size, is_work_area);

            // Update group usage
            let mut group_limits = self.group_limits.write().unwrap();
            if let Some(limits) = group_limits.get_mut(&quota.group_id) {
                limits.current_total_usage = limits.current_total_usage.saturating_sub(size);
            }
        }
        Ok(())
    }

    /// Update memory pressure level
    fn update_pressure_level(&self) {
        let current_usage = *self.current_db_usage.read().unwrap();
        let usage_ratio = current_usage as f64 / self.max_db_memory as f64;

        let new_pressure = if usage_ratio >= self.pressure_thresholds.critical {
            MemoryPressure::Critical
        } else if usage_ratio >= self.pressure_thresholds.high {
            MemoryPressure::High
        } else if usage_ratio >= self.pressure_thresholds.medium {
            MemoryPressure::Medium
        } else if usage_ratio >= self.pressure_thresholds.low {
            MemoryPressure::Low
        } else {
            MemoryPressure::None
        };

        let old_pressure = *self.pressure_level.read().unwrap();
        if new_pressure != old_pressure {
            *self.pressure_level.write().unwrap() = new_pressure;

            if new_pressure > old_pressure {
                let mut stats = self.stats.write().unwrap();
                stats.pressure_events += 1;
            }
        }
    }

    /// Get current memory pressure
    pub fn get_pressure_level(&self) -> MemoryPressure {
        *self.pressure_level.read().unwrap()
    }

    /// Register group memory limits
    pub fn register_group_limits(
        &self,
        group_id: ConsumerGroupId,
        max_group_memory: Option<MemorySize>,
        max_session_pga: Option<MemorySize>,
    ) -> Result<()> {
        let mut group_limits = self.group_limits.write().unwrap();

        if group_limits.contains_key(&group_id) {
            return Err(DbError::AlreadyExists(
                format!("Group {} limits already exist", group_id)
            ));
        }

        group_limits.insert(group_id, GroupMemoryLimits {
            group_id,
            max_group_memory,
            max_session_pga,
            current_total_usage: 0,
            active_sessions: 0,
        });

        Ok(())
    }

    /// Auto-tune memory pools
    pub fn auto_tune_pools(&self) -> Vec<MemoryAdvisorRecommendation> {
        if !self.auto_tuning_enabled {
            return Vec::new();
        }

        let mut recommendations = Vec::new();
        let pools = self.pools.read().unwrap();

        for pool in pools.values() {
            if !pool.auto_tune {
                continue;
            }

            let usage_pct = pool.usage_pct();

            // Recommend size adjustment based on usage
            let recommended_size = if usage_pct > 90.0 {
                // Increase by 20%
                ((pool.max_size as f64 * 1.2) as MemorySize).min(pool.max_size * 2)
            } else if usage_pct < 50.0 && pool.max_size > pool.min_size {
                // Decrease by 10%
                ((pool.max_size as f64 * 0.9) as MemorySize).max(pool.min_size)
            } else {
                pool.max_size
            };

            if recommended_size != pool.max_size {
                recommendations.push(MemoryAdvisorRecommendation {
                    pool_id: pool.id,
                    current_size: pool.max_size,
                    recommended_size,
                    estimated_benefit: (usage_pct - 75.0).abs() / 25.0,
                    reason: format!(
                        "Pool usage is {:.1}%, recommend {}",
                        usage_pct,
                        if recommended_size > pool.max_size {
                            "increase"
                        } else {
                            "decrease"
                        }
                    ),
                    timestamp: SystemTime::now(),
                });
            }
        }

        recommendations
    }

    /// Get memory statistics
    pub fn get_stats(&self) -> MemoryStats {
        self.stats.read().unwrap().clone()
    }

    /// Get session quota information
    pub fn get_session_quota(&self, session_id: SessionId) -> Option<SessionMemoryQuota> {
        let quotas = self.session_quotas.read().unwrap();
        quotas.get(&session_id).cloned()
    }

    /// Get pool information
    pub fn get_pool(&self, pool_id: MemoryPoolId) -> Option<MemoryPool> {
        let pools = self.pools.read().unwrap();
        pools.get(&pool_id).cloned()
    }

    /// Get all pools
    pub fn list_pools(&self) -> Vec<MemoryPool> {
        let pools = self.pools.read().unwrap();
        pools.values().cloned().collect()
    }

    /// Get current database memory usage
    pub fn get_db_usage(&self) -> MemorySize {
        *self.current_db_usage.read().unwrap()
    }

    /// Get usage percentage
    pub fn get_usage_pct(&self) -> f64 {
        let usage = *self.current_db_usage.read().unwrap();
        (usage as f64 / self.max_db_memory as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_creation() {
        let manager = MemoryManager::new(
            16 * 1024 * 1024 * 1024, // 16 GB system
            8 * 1024 * 1024 * 1024,  // 8 GB for DB
            AllocationStrategy::Automatic,
        ).unwrap();

        assert_eq!(manager.strategy, AllocationStrategy::Automatic);
    }

    #[test]
    fn test_pool_allocation() {
        let manager = MemoryManager::new(
            16 * 1024 * 1024 * 1024,
            8 * 1024 * 1024 * 1024,
            AllocationStrategy::Manual,
        ).unwrap();

        // Allocate from buffer cache
        manager.allocate_from_pool(2, 1024 * 1024).unwrap();

        let pool = manager.get_pool(2).unwrap();
        assert_eq!(pool.allocated_size, 1024 * 1024);
    }

    #[test]
    fn test_session_quota() {
        let manager = MemoryManager::new(
            16 * 1024 * 1024 * 1024,
            8 * 1024 * 1024 * 1024,
            AllocationStrategy::Manual,
        ).unwrap();

        manager.register_group_limits(1, None, Some(200 * 1024 * 1024)).unwrap();
        manager.create_session_quota(1, 1, None, None).unwrap();

        manager.allocate_session_memory(1, 10 * 1024 * 1024, false).unwrap();

        let quota = manager.get_session_quota(1).unwrap();
        assert_eq!(quota.current_pga_usage, 10 * 1024 * 1024);
    }
}


