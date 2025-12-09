// Resource Manager
// CPU resource groups, memory allocation limits, I/O bandwidth quotas, query timeout enforcement

use tokio::time::sleep;
use std::fmt;
use std::time::Instant;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration};


/// Resource type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Cpu,
    Memory,
    DiskIO,
    NetworkIO,
    Connections,
    QueryTimeout,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceType::Cpu => write!(f, "CPU"),
            ResourceType::Memory => write!(f, "Memory"),
            ResourceType::DiskIO => write!(f, "Disk I/O"),
            ResourceType::NetworkIO => write!(f, "Network I/O"),
            ResourceType::Connections => write!(f, "Connections"),
            ResourceType::QueryTimeout => write!(f, "Query Timeout"),
        }
    }
}

/// Resource limit definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimit {
    pub resource_type: ResourceType,
    pub max_value: u64,
    pub current_value: u64,
    pub warning_threshold: u64,
    pub enforcement_policy: EnforcementPolicy,
}

impl ResourceLimit {
    pub fn new(resource_type: ResourceType, max_value: u64) -> Self {
        Self {
            resource_type,
            max_value,
            current_value: 0,
            warning_threshold: (max_value as f64 * 0.8) as u64,
            enforcement_policy: EnforcementPolicy::Throttle,
        }
    }

    pub fn with_warning_threshold(mut self, threshold: u64) -> Self {
        self.warning_threshold = threshold;
        self
    }

    pub fn with_enforcement_policy(mut self, policy: EnforcementPolicy) -> Self {
        self.enforcement_policy = policy;
        self
    }

    pub fn is_exceeded(&self) -> bool {
        self.current_value >= self.max_value
    }

    pub fn is_warning(&self) -> bool {
        self.current_value >= self.warning_threshold
    }

    pub fn usage_percentage(&self) -> f64 {
        if self.max_value == 0 {
            0.0
        } else {
            (self.current_value as f64 / self.max_value as f64) * 100.0
        }
    }

    pub fn available(&self) -> u64 {
        self.max_value.saturating_sub(self.current_value)
    }
}

/// Enforcement policy when resource limits are exceeded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementPolicy {
    Allow,      // Log but allow
    Throttle,   // Slow down operations
    Queue,      // Queue requests
    Reject,     // Reject new requests
    Terminate,  // Terminate existing operations
}

/// Resource group for organizing sessions/queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGroup {
    pub name: String,
    pub priority: u8, // 0-255, higher is more priority
    pub limits: HashMap<ResourceType, ResourceLimit>,
    pub active_sessions: Vec<u64>,
    pub total_cpu_time_us: u64,
    pub total_memory_bytes: u64,
    pub total_io_bytes: u64,
    pub created_at: SystemTime,
}

impl ResourceGroup {
    pub fn new(name: impl Into<String>, priority: u8) -> Self {
        Self {
            name: name.into(),
            priority,
            limits: HashMap::new(),
            active_sessions: Vec::new(),
            total_cpu_time_us: 0,
            total_memory_bytes: 0,
            total_io_bytes: 0,
            created_at: SystemTime::now(),
        }
    }

    pub fn add_limit(&mut self, limit: ResourceLimit) {
        self.limits.insert(limit.resource_type, limit);
    }

    pub fn add_session(&mut self, session_id: u64) {
        if !self.active_sessions.contains(&session_id) {
            self.active_sessions.push(session_id);
        }
    }

    pub fn remove_session(&mut self, sessionid: u64) {
        self.active_sessions.retain(|&id| id != session_id);
    }

    pub fn session_count(&self) -> usize {
        self.active_sessions.len()
    }

    pub fn check_limit(&self, resource_type: ResourceType) -> ResourceLimitStatus {
        if let Some(limit) = self.limits.get(&resource_type) {
            if limit.is_exceeded() {
                ResourceLimitStatus::Exceeded(limit.clone())
            } else if limit.is_warning() {
                ResourceLimitStatus::Warning(limit.clone())
            } else {
                ResourceLimitStatus::Ok(limit.clone())
            }
        } else {
            ResourceLimitStatus::NoLimit
        }
    }

    pub fn allocate_resource(&mut self, resource_type: ResourceType, amount: u64) -> bool {
        if let Some(limit) = self.limits.get_mut(&resource_type) {
            if limit.current_value + amount <= limit.max_value {
                limit.current_value += amount;
                true
            } else {
                false
            }
        } else {
            true // No limit, allow allocation
        }
    }

    pub fn release_resource(&mut self, resource_type: ResourceType, amount: u64) {
        if let Some(limit) = self.limits.get_mut(&resource_type) {
            limit.current_value = limit.current_value.saturating_sub(amount);
        }
    }
}

/// Resource limit status
#[derive(Debug, Clone)]
pub enum ResourceLimitStatus {
    Ok(ResourceLimit),
    Warning(ResourceLimit),
    Exceeded(ResourceLimit),
    NoLimit,
}

/// Query resource tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResourceUsage {
    pub query_id: u64,
    pub session_id: u64,
    pub resource_group: String,
    #[serde(skip, default = "Instant::now")]
    pub start_time: Instant,
    #[serde(skip)]
    pub timeout: Option<Duration>,
    pub cpu_time_us: u64,
    pub memory_bytes: u64,
    pub disk_io_bytes: u64,
    pub network_io_bytes: u64,
    pub is_throttled: bool,
}

impl QueryResourceUsage {
    pub fn new(query_id: u64, session_id: u64, resource_group: impl Into<String>) -> Self {
        Self {
            query_id,
            session_id,
            resource_group: resource_group.into(),
            start_time: Instant::now(),
            timeout: None,
            cpu_time_us: 0,
            memory_bytes: 0,
            disk_io_bytes: 0,
            network_io_bytes: 0,
            is_throttled: false,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn is_timeout(&self) -> bool {
        if let Some(timeout) = self.timeout {
            self.elapsed() >= timeout
        } else {
            false
        }
    }

    pub fn record_cpu(&mut self, cpu_time_us: u64) {
        self.cpu_time_us += cpu_time_us;
    }

    pub fn record_memory(&mut self, bytes: u64) {
        self.memory_bytes = bytes;
    }

    pub fn record_disk_io(&mut self, bytes: u64) {
        self.disk_io_bytes += bytes;
    }

    pub fn record_network_io(&mut self, bytes: u64) {
        self.network_io_bytes += bytes;
    }

    pub fn set_throttled(&mut self, throttled: bool) {
        self.is_throttled = throttled;
    }
}

/// Resource manager for enforcing resource limits
pub struct ResourceManager {
    groups: Arc<RwLock<HashMap<String, ResourceGroup>>>,
    session_groups: Arc<RwLock<HashMap<u64, String>>>,
    active_queries: Arc<RwLock<HashMap<u64, QueryResourceUsage>>>,
    default_group: String,
    global_limits: Arc<RwLock<HashMap<ResourceType, ResourceLimit>>>,
}

impl ResourceManager {
    pub fn new() -> Self {
        let mut manager = Self {
            groups: Arc::new(RwLock::new(HashMap::new())),
            session_groups: Arc::new(RwLock::new(HashMap::new())),
            active_queries: Arc::new(RwLock::new(HashMap::new())),
            default_group: "DEFAULT".to_string(),
            global_limits: Arc::new(RwLock::new(HashMap::new())),
        };

        // Create default resource group
        let default = ResourceGroup::new("DEFAULT", 100);
        manager.groups.write().insert("DEFAULT".to_string(), default);

        manager
    }

    pub fn create_group(&self, name: impl Into<String>, priority: u8) {
        let name = name.into();
        let group = ResourceGroup::new(name.clone(), priority);
        self.groups.write().insert(name, group);
    }

    pub fn delete_group(&self, name: &str) -> bool {
        if name == self.default_group {
            return false; // Cannot delete default group
        }

        // Move sessions to default group
        let mut session_groups = self.session_groups.write();
        for (_, group_name) in session_groups.iter_mut() {
            if group_name == name {
                *group_name = self.default_group.clone();
            }
        }
        drop(session_groups);

        self.groups.write().remove(name).is_some()
    }

    pub fn add_group_limit(&self, group_name: &str, limit: ResourceLimit) -> bool {
        if let Some(group) = self.groups.write().get_mut(group_name) {
            group.add_limit(limit);
            true
        } else {
            false
        }
    }

    pub fn set_global_limit(&self, limit: ResourceLimit) {
        self.global_limits.write().insert(limit.resource_type, limit);
    }

    pub fn assign_session_to_group(&self, session_id: u64, group_name: impl Into<String>) -> bool {
        let group_name = group_name.into();

        // Check if group exists
        if !self.groups.read().contains_key(&group_name) {
            return false;
        }

        // Remove from old group if exists
        if let Some(old_group_name) = self.session_groups.read().get(&session_id) {
            if let Some(old_group) = self.groups.write().get_mut(old_group_name) {
                old_group.remove_session(session_id);
            }
        }

        // Add to new group
        if let Some(new_group) = self.groups.write().get_mut(&group_name) {
            new_group.add_session(session_id);
        }

        self.session_groups.write().insert(session_id, group_name);
        true
    }

    pub fn get_session_group(&self, session_id: u64) -> String {
        self.session_groups
            .read()
            .get(&session_id)
            .cloned()
            .unwrap_or_else(|| self.default_group.clone())
    }

    pub fn start_query(
        &self,
        query_id: u64,
        session_id: u64,
        timeout: Option<Duration>,
    ) -> Result<(), String> {
        let group_name = self.get_session_group(session_id);

        // Check connection limit
        let groups = self.groups.read();
        if let Some(group) = groups.get(&group_name) {
            match group.check_limit(ResourceType::Connections) {
                ResourceLimitStatus::Exceeded(limit) => {
                    match limit.enforcement_policy {
                        EnforcementPolicy::Reject => {
                            return Err(format!(
                                "Connection limit exceeded for group {}: {}/{}",
                                group_name, limit.current_value, limit.max_value
                            ))));
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        drop(groups);

        let mut usage = QueryResourceUsage::new(query_id, session_id, group_name.clone());
        if let Some(timeout) = timeout {
            usage = usage.with_timeout(timeout);
        }

        self.active_queries.write().insert(query_id, usage);

        // Allocate connection resource
        if let Some(group) = self.groups.write().get_mut(&group_name) {
            group.allocate_resource(ResourceType::Connections, 1);
        }

        Ok(())
    }

    pub fn end_query(&self, query_id: u64) {
        if let Some(usage) = self.active_queries.write().remove(&query_id) {
            let group_name = usage.resource_group.clone();

            // Release resources
            if let Some(group) = self.groups.write().get_mut(&group_name) {
                group.release_resource(ResourceType::Connections, 1);
                group.release_resource(ResourceType::Memory, usage.memory_bytes);
                group.total_cpu_time_us += usage.cpu_time_us;
                group.total_io_bytes += usage.disk_io_bytes + usage.network_io_bytes;
            }
        }
    }

    pub fn check_query_timeout(&self, query_id: u64) -> bool {
        if let Some(usage) = self.active_queries.read().get(&query_id) {
            usage.is_timeout()
        } else {
            false
        }
    }

    pub fn allocate_memory(
        &self,
        query_id: u64,
        bytes: u64,
    ) -> Result<(), String> {
        let mut active_queries = self.active_queries.write();
        if let Some(usage) = active_queries.get_mut(&query_id) {
            let group_name = usage.resource_group.clone();
            drop(active_queries);

            let mut groups = self.groups.write();
            if let Some(group) = groups.get_mut(&group_name) {
                if group.allocate_resource(ResourceType::Memory, bytes) {
                    let mut active_queries = self.active_queries.write();
                    if let Some(usage) = active_queries.get_mut(&query_id) {
                        usage.record_memory(usage.memory_bytes + bytes);
                    }
                    Ok(())
                } else {
                    let limit = group.limits.get(&ResourceType::Memory).unwrap();
                    Err(format!(
                        "Memory limit exceeded for group {}: {}/{}",
                        group_name, limit.current_value, limit.max_value
                    ))
                }
            } else {
                Ok(())
            }
        } else {
            Err("Query not found".to_string())
        }
    }

    pub fn record_cpu_time(&self, query_id: u64, cpu_time_us: u64) {
        if let Some(usage) = self.active_queries.write().get_mut(&query_id) {
            usage.record_cpu(cpu_time_us)));
        }
    }

    pub fn record_disk_io(&self, query_id: u64, bytes: u64) {
        if let Some(usage) = self.active_queries.write().get_mut(&query_id) {
            usage.record_disk_io(bytes);
        }
    }

    pub fn record_network_io(&self, query_id: u64, bytes: u64) {
        if let Some(usage) = self.active_queries.write().get_mut(&query_id) {
            usage.record_network_io(bytes);
        }
    }

    pub fn get_query_usage(&self, query_id: u64) -> Option<QueryResourceUsage> {
        self.active_queries.read().get(&query_id).cloned()
    }

    pub fn get_active_queries(&self) -> Vec<QueryResourceUsage> {
        self.active_queries.read().values().cloned().collect()
    }

    pub fn get_group_statistics(&self, groupname: &str) -> Option<ResourceGroupStatistics> {
        self.groups.read().get(group_name).map(|group| {
            ResourceGroupStatistics {
                name: group.name.clone(),
                priority: group.priority,
                active_sessions: group.session_count(),
                total_cpu_time_us: group.total_cpu_time_us,
                total_memory_bytes: group.total_memory_bytes,
                total_io_bytes: group.total_io_bytes,
                resource_usage: group
                    .limits
                    .iter()
                    .map(|(rt, limit)| (*rt, limit.usage_percentage()))
                    .collect(),
            }
        })
    }

    pub fn get_all_groups(&self) -> Vec<String> {
        self.groups.read().keys().cloned().collect()
    }

    pub fn get_timeout_queries(&self) -> Vec<QueryResourceUsage> {
        self.active_queries
            .read()
            .values()
            .filter(|usage| usage.is_timeout())
            .cloned()
            .collect()
    }

    pub fn terminate_query(&self, query_id: u64) -> bool {
        self.active_queries.write().remove(&query_id).is_some()
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource group statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGroupStatistics {
    pub name: String,
    pub priority: u8,
    pub active_sessions: usize,
    pub total_cpu_time_us: u64,
    pub total_memory_bytes: u64,
    pub total_io_bytes: u64,
    pub resource_usage: HashMap<ResourceType, f64>,
}

/// Resource allocation planner for predictive resource management
pub struct ResourcePlanner {
    historical_usage: Arc<RwLock<HashMap<String, Vec<ResourceGroupStatistics>>>>,
    max_history: usize,
}

impl ResourcePlanner {
    pub fn new(max_history: usize) -> Self {
        Self {
            historical_usage: Arc::new(RwLock::new(HashMap::new())),
            max_history,
        }
    }

    pub fn record_snapshot(&self, stats: ResourceGroupStatistics) {
        let mut history = self.historical_usage.write();
        let group_history = history.entry(stats.name.clone()).or_insert_with(Vec::new);

        if group_history.len() >= self.max_history {
            group_history.remove(0);
        }

        group_history.push(stats);
    }

    pub fn predict_resource_needs(
        &self,
        group_name: &str,
        resource_type: ResourceType,
    ) -> Option<f64> {
        let history = self.historical_usage.read();
        if let Some(group_history) = history.get(group_name) {
            if group_history.is_empty() {
                return None;
            }

            // Simple moving average prediction
            let sum: f64 = group_history
                .iter()
                .filter_map(|s| s.resource_usage.get(&resource_type))
                .sum();

            let count = group_history
                .iter()
                .filter(|s| s.resource_usage.contains_key(&resource_type))
                .count();

            if count > 0 {
                Some(sum / count as f64)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_peak_usage(
        &self,
        group_name: &str,
        resource_type: ResourceType,
    ) -> Option<f64> {
        let history = self.historical_usage.read();
        if let Some(group_history) = history.get(group_name) {
            group_history
                .iter()
                .filter_map(|s| s.resource_usage.get(&resource_type))
                .cloned()
                .fold(None, |max, v| Some(max.map_or(v, |m| if v > m { v } else { m })))
        } else {
            None
        }
    }

    pub fn get_average_usage(
        &self,
        group_name: &str,
        resource_type: ResourceType,
    ) -> Option<f64> {
        self.predict_resource_needs(group_name, resource_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_limit() {
        let limit = ResourceLimit::new(ResourceType::Memory, 1000);
        assert_eq!(limit.available(), 1000);
        assert!(!limit.is_exceeded());
        assert!(!limit.is_warning());
    }

    #[test]
    fn test_resource_group() {
        let mut group = ResourceGroup::new("test_group", 100);
        let limit = ResourceLimit::new(ResourceType::Memory, 1000);
        group.add_limit(limit);

        assert!(group.allocate_resource(ResourceType::Memory, 500));
        assert_eq!(group.limits.get(&ResourceType::Memory).unwrap().current_value, 500);

        assert!(group.allocate_resource(ResourceType::Memory, 500));
        assert!(!group.allocate_resource(ResourceType::Memory, 1));
    }

    #[test]
    fn test_resource_manager() {
        let manager = ResourceManager::new();
        manager.create_group("high_priority", 200);

        let limit = ResourceLimit::new(ResourceType::Connections, 10);
        manager.add_group_limit("high_priority", limit);

        assert!(manager.assign_session_to_group(1, "high_priority"));

        let result = manager.start_query(1001, 1, Some(Duration::from_secs(30)));
        assert!(result.is_ok());

        let usage = manager.get_query_usage(1001);
        assert!(usage.is_some());

        manager.end_query(1001);
        assert!(manager.get_query_usage(1001).is_none());
    }

    #[test]
    fn test_query_timeout() {
        let mut usage = QueryResourceUsage::new(1, 100, "default")
            .with_timeout(Duration::from_millis(10));

        assert!(!usage.is_timeout());
        std::thread::sleep(Duration::from_millis(20));
        assert!(usage.is_timeout());
    }

    #[test]
    fn test_resource_planner() {
        let planner = ResourcePlanner::new(100);

        let stats = ResourceGroupStatistics {
            name: "test".to_string(),
            priority: 100,
            active_sessions: 5,
            total_cpu_time_us: 1000,
            total_memory_bytes: 500,
            total_io_bytes: 200,
            resource_usage: [(ResourceType::Memory, 50.0)].iter().cloned().collect(),
        };

        planner.record_snapshot(stats);
        let prediction = planner.predict_resource_needs("test", ResourceType::Memory);
        assert_eq!(prediction, Some(50.0));
    }
}


