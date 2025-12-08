//! Consumer Groups for Resource Management
//!
//! This module implements Oracle-like consumer groups for workload classification,
//! user-to-group mapping, dynamic group switching, and priority management.

use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::time::{Duration};
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Consumer group identifier
pub type ConsumerGroupId = u64;

/// User identifier
pub type UserId = u64;

/// Session identifier
pub type SessionId = u64;

/// Priority level for consumer groups (0-7, where 0 is highest priority)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PriorityLevel(u8);

impl PriorityLevel {
    pub fn new(level: u8) -> Result<Self> {
        if level > 7 {
            return Err(DbError::Configuration(
                "Priority level must be between 0 and 7".to_string()
            ));
        }
        Ok(PriorityLevel(level))
    }

    pub fn value(&self) -> u8 {
        self.0
    }

    pub fn high() -> Self {
        PriorityLevel(0)
    }

    pub fn medium() -> Self {
        PriorityLevel(3)
    }

    pub fn low() -> Self {
        PriorityLevel(7)
    }
}

/// Consumer group category
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupCategory {
    /// Interactive user sessions
    Interactive,
    /// Batch processing jobs
    Batch,
    /// Background maintenance tasks
    Maintenance,
    /// Real-time analytics
    Analytics,
    /// System-level operations
    System,
    /// Custom user-defined category
    Custom(String),
}

/// Consumer group definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerGroup {
    /// Unique identifier
    pub id: ConsumerGroupId,
    /// Group name
    pub name: String,
    /// Group description
    pub description: Option<String>,
    /// Priority level
    pub priority: PriorityLevel,
    /// Category
    pub category: GroupCategory,
    /// CPU allocation percentage (0-100)
    pub cpu_allocation_pct: u8,
    /// Maximum sessions allowed in this group
    pub max_sessions: Option<usize>,
    /// Maximum idle time before termination
    pub max_idle_time: Option<Duration>,
    /// Maximum execution time for queries
    pub max_execution_time: Option<Duration>,
    /// Parallel degree of parallelism limit
    pub parallel_degree_limit: Option<u32>,
    /// Memory limit in bytes
    pub memory_limit: Option<u64>,
    /// I/O bandwidth limit in bytes/sec
    pub io_bandwidth_limit: Option<u64>,
    /// IOPS limit
    pub iops_limit: Option<u32>,
    /// Whether this group is active
    pub is_active: bool,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last modified timestamp
    pub modified_at: SystemTime,
    /// Current number of active sessions
    pub current_sessions: usize,
}

impl ConsumerGroup {
    /// Create a new consumer group
    pub fn new(
        id: ConsumerGroupId,
        name: String,
        priority: PriorityLevel,
        category: GroupCategory,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            name,
            description: None,
            priority,
            category,
            cpu_allocation_pct: 0,
            max_sessions: None,
            max_idle_time: None,
            max_execution_time: None,
            parallel_degree_limit: None,
            memory_limit: None,
            io_bandwidth_limit: None,
            iops_limit: None,
            is_active: true,
            created_at: now,
            modified_at: now,
            current_sessions: 0,
        }
    }

    /// Check if the group can accept a new session
    pub fn can_accept_session(&self) -> bool {
        if !self.is_active {
            return false;
        }
        if let Some(max) = self.max_sessions {
            return self.current_sessions < max;
        }
        true
    }

    /// Increment session count
    pub fn increment_sessions(&mut self) -> Result<()> {
        if !self.can_accept_session() {
            return Err(DbError::ResourceExhausted(
                format!("Consumer group {} has reached max sessions", self.name)
            ));
        }
        self.current_sessions += 1;
        Ok(())
    }

    /// Decrement session count
    pub fn decrement_sessions(&mut self) {
        if self.current_sessions > 0 {
            self.current_sessions -= 1;
        }
    }
}

/// Automatic consumer group assignment rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentRule {
    /// Rule identifier
    pub id: u64,
    /// Rule name
    pub name: String,
    /// Rule priority (lower number = higher priority)
    pub priority: u32,
    /// Condition for matching
    pub condition: RuleCondition,
    /// Target consumer group
    pub target_group_id: ConsumerGroupId,
    /// Whether the rule is enabled
    pub is_enabled: bool,
}

/// Rule condition for automatic group assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCondition {
    /// Match by username
    Username(String),
    /// Match by username pattern (regex)
    UsernamePattern(String),
    /// Match by client program name
    ProgramName(String),
    /// Match by client machine
    MachineName(String),
    /// Match by service name
    ServiceName(String),
    /// Match by module name
    ModuleName(String),
    /// Match by time of day
    TimeOfDay { start_hour: u8, end_hour: u8 },
    /// Match by day of week
    DayOfWeek(Vec<u8>), // 0=Sunday, 6=Saturday
    /// Compound condition (AND)
    And(Vec<RuleCondition>),
    /// Compound condition (OR)
    Or(Vec<RuleCondition>),
    /// Negation
    Not(Box<RuleCondition>),
}

impl RuleCondition {
    /// Evaluate the condition against session attributes
    pub fn evaluate(&self, attrs: &SessionAttributes) -> bool {
        match self {
            RuleCondition::Username(name) => attrs.username == *name,
            RuleCondition::UsernamePattern(pattern) => {
                // Simple pattern matching (could be enhanced with regex)
                attrs.username.contains(pattern)
            }
            RuleCondition::ProgramName(name) => {
                attrs.program_name.as_ref().map_or(false, |p| p == name)
            }
            RuleCondition::MachineName(name) => {
                attrs.machine_name.as_ref().map_or(false, |m| m == name)
            }
            RuleCondition::ServiceName(name) => {
                attrs.service_name.as_ref().map_or(false, |s| s == name)
            }
            RuleCondition::ModuleName(name) => {
                attrs.module_name.as_ref().map_or(false, |m| m == name)
            }
            RuleCondition::TimeOfDay { start_hour, end_hour } => {
                let now = SystemTime::now();
                // Simplified time check (would need proper time handling)
                true // Placeholder
            }
            RuleCondition::DayOfWeek(days) => {
                // Simplified day of week check
                true // Placeholder
            }
            RuleCondition::And(conditions) => {
                conditions.iter().all(|c| c.evaluate(attrs))
            }
            RuleCondition::Or(conditions) => {
                conditions.iter().any(|c| c.evaluate(attrs))
            }
            RuleCondition::Not(condition) => !condition.evaluate(attrs),
        }
    }
}

/// Session attributes for rule evaluation
#[derive(Debug, Clone)]
pub struct SessionAttributes {
    pub username: String,
    pub program_name: Option<String>,
    pub machine_name: Option<String>,
    pub service_name: Option<String>,
    pub module_name: Option<String>,
    pub action_name: Option<String>,
}

/// User to consumer group mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserGroupMapping {
    pub user_id: UserId,
    pub username: String,
    pub group_id: ConsumerGroupId,
    pub is_permanent: bool,
    pub created_at: SystemTime,
}

/// Session to consumer group mapping
#[derive(Debug, Clone)]
pub struct SessionGroupMapping {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub group_id: ConsumerGroupId,
    pub previous_group_id: Option<ConsumerGroupId>,
    pub switch_time: SystemTime,
    pub switch_reason: SwitchReason,
}

/// Reason for consumer group switch
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchReason {
    /// Initial assignment
    Initial,
    /// Manual switch by user
    Manual,
    /// Automatic switch by rule
    Automatic,
    /// Administrative override
    Administrative,
    /// Workload-based adjustment
    WorkloadBased,
}

/// Consumer group manager
pub struct ConsumerGroupManager {
    /// All consumer groups
    groups: Arc<RwLock<HashMap<ConsumerGroupId, ConsumerGroup>>>,
    /// Groups indexed by name
    groups_by_name: Arc<RwLock<HashMap<String, ConsumerGroupId>>>,
    /// User to group mappings
    user_mappings: Arc<RwLock<HashMap<UserId, UserGroupMapping>>>,
    /// Session to group mappings
    session_mappings: Arc<RwLock<HashMap<SessionId, SessionGroupMapping>>>,
    /// Assignment rules
    assignment_rules: Arc<RwLock<Vec<AssignmentRule>>>,
    /// Default group for unmapped users
    default_group_id: ConsumerGroupId,
    /// Next group ID
    next_group_id: Arc<RwLock<ConsumerGroupId>>,
    /// Next rule ID
    next_rule_id: Arc<RwLock<u64>>,
}

impl ConsumerGroupManager {
    /// Create a new consumer group manager
    pub fn new() -> Result<Self> {
        let mut manager = Self {
            groups: Arc::new(RwLock::new(HashMap::new())),
            groups_by_name: Arc::new(RwLock::new(HashMap::new())),
            user_mappings: Arc::new(RwLock::new(HashMap::new())),
            session_mappings: Arc::new(RwLock::new(HashMap::new())),
            assignment_rules: Arc::new(RwLock::new(Vec::new())),
            default_group_id: 0,
            next_group_id: Arc::new(RwLock::new(100)),
            next_rule_id: Arc::new(RwLock::new(1)),
        };

        // Create default system groups
        manager.create_system_groups()?;
        Ok(manager)
    }

    /// Create default system consumer groups
    fn create_system_groups(&mut self) -> Result<()> {
        // SYS_GROUP - System operations
        let sys_group = ConsumerGroup::new(
            1,
            "SYS_GROUP".to_string(),
            PriorityLevel::high(),
            GroupCategory::System,
        );
        self.register_group(sys_group)?;

        // INTERACTIVE_GROUP - Interactive users
        let mut interactive_group = ConsumerGroup::new(
            2,
            "INTERACTIVE_GROUP".to_string(),
            PriorityLevel::medium(),
            GroupCategory::Interactive,
        );
        interactive_group.cpu_allocation_pct = 60;
        interactive_group.max_execution_time = Some(Duration::from_secs(300));
        self.register_group(interactive_group)?;

        // BATCH_GROUP - Batch jobs
        let mut batch_group = ConsumerGroup::new(
            3,
            "BATCH_GROUP".to_string(),
            PriorityLevel::low(),
            GroupCategory::Batch,
        );
        batch_group.cpu_allocation_pct = 30;
        self.register_group(batch_group)?;

        // MAINTENANCE_GROUP - Maintenance tasks
        let mut maint_group = ConsumerGroup::new(
            4,
            "MAINTENANCE_GROUP".to_string(),
            PriorityLevel::low(),
            GroupCategory::Maintenance,
        );
        maint_group.cpu_allocation_pct = 10;
        self.register_group(maint_group)?;

        // Set default group
        self.default_group_id = 2; // INTERACTIVE_GROUP

        Ok(())
    }

    /// Register a new consumer group
    fn register_group(&mut self, group: ConsumerGroup) -> Result<()> {
        let id = group.id;
        let name = group.name.clone();

        let mut groups = self.groups.write().unwrap();
        let mut groups_by_name = self.groups_by_name.write().unwrap();

        if groups.contains_key(&id) {
            return Err(DbError::AlreadyExists(
                format!("Consumer group with ID {} already exists", id)
            ));
        }

        if groups_by_name.contains_key(&name) {
            return Err(DbError::AlreadyExists(
                format!("Consumer group with name {} already exists", name)
            ));
        }

        groups_by_name.insert(name, id);
        groups.insert(id, group);
        Ok(())
    }

    /// Create a new consumer group
    pub fn create_group(
        &self,
        name: String,
        priority: PriorityLevel,
        category: GroupCategory,
    ) -> Result<ConsumerGroupId> {
        let id = {
            let mut next_id = self.next_group_id.write().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let group = ConsumerGroup::new(id, name.clone(), priority, category);

        {
            let mut groups = self.groups.write().unwrap();
            let mut groups_by_name = self.groups_by_name.write().unwrap();

            if groups_by_name.contains_key(&name) {
                return Err(DbError::AlreadyExists(
                    format!("Consumer group {} already exists", name)
                ));
            }

            groups_by_name.insert(name, id);
            groups.insert(id, group);
        }

        Ok(id)
    }

    /// Get consumer group by ID
    pub fn get_group(&self, group_id: ConsumerGroupId) -> Result<ConsumerGroup> {
        let groups = self.groups.read().unwrap();
        groups.get(&group_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(
                format!("Consumer group {} not found", group_id)
            ))
    }

    /// Get consumer group by name
    pub fn get_group_by_name(&self, name: &str) -> Result<ConsumerGroup> {
        let groups_by_name = self.groups_by_name.read().unwrap();
        let group_id = groups_by_name.get(name)
            .ok_or_else(|| DbError::NotFound(
                format!("Consumer group {} not found", name)
            ))?;
        self.get_group(*group_id)
    }

    /// Update consumer group configuration
    pub fn update_group<F>(&self, group_id: ConsumerGroupId, update_fn: F) -> Result<()>
    where
        F: FnOnce(&mut ConsumerGroup),
    {
        let mut groups = self.groups.write().unwrap();
        let group = groups.get_mut(&group_id)
            .ok_or_else(|| DbError::NotFound(
                format!("Consumer group {} not found", group_id)
            ))?;

        update_fn(group);
        group.modified_at = SystemTime::now();
        Ok(())
    }

    /// Delete a consumer group
    pub fn delete_group(&self, group_id: ConsumerGroupId) -> Result<()> {
        // Don't allow deleting system groups
        if group_id < 100 {
            return Err(DbError::PermissionDenied(
                "Cannot delete system consumer groups".to_string()
            ));
        }

        let mut groups = self.groups.write().unwrap();
        let mut groups_by_name = self.groups_by_name.write().unwrap();

        let group = groups.remove(&group_id)
            .ok_or_else(|| DbError::NotFound(
                format!("Consumer group {} not found", group_id)
            ))?;

        groups_by_name.remove(&group.name);
        Ok(())
    }

    /// Map a user to a consumer group
    pub fn map_user_to_group(
        &self,
        user_id: UserId,
        username: String,
        group_id: ConsumerGroupId,
        is_permanent: bool,
    ) -> Result<()> {
        // Verify group exists
        self.get_group(group_id)?;

        let mapping = UserGroupMapping {
            user_id,
            username,
            group_id,
            is_permanent,
            created_at: SystemTime::now(),
        };

        let mut user_mappings = self.user_mappings.write().unwrap();
        user_mappings.insert(user_id, mapping);
        Ok(())
    }

    /// Remove user to group mapping
    pub fn unmap_user(&self, user_id: UserId) -> Result<()> {
        let mut user_mappings = self.user_mappings.write().unwrap();
        user_mappings.remove(&user_id)
            .ok_or_else(|| DbError::NotFound(
                format!("User {} mapping not found", user_id)
            ))?;
        Ok(())
    }

    /// Assign session to consumer group (initial assignment)
    pub fn assign_session(
        &self,
        session_id: SessionId,
        user_id: UserId,
        attrs: &SessionAttributes,
    ) -> Result<ConsumerGroupId> {
        // First check user mapping
        let group_id = {
            let user_mappings = self.user_mappings.read().unwrap();
            if let Some(mapping) = user_mappings.get(&user_id) {
                Some(mapping.group_id)
            } else {
                None
            }
        };

        // If no user mapping, apply assignment rules
        let group_id = if let Some(id) = group_id {
            id
        } else {
            self.apply_assignment_rules(attrs)
                .unwrap_or(self.default_group_id)
        };

        // Increment session count for the group
        {
            let mut groups = self.groups.write().unwrap();
            if let Some(group) = groups.get_mut(&group_id) {
                group.increment_sessions()?;
            }
        }

        // Record session mapping
        let mapping = SessionGroupMapping {
            session_id,
            user_id,
            group_id,
            previous_group_id: None,
            switch_time: SystemTime::now(),
            switch_reason: SwitchReason::Initial,
        };

        let mut session_mappings = self.session_mappings.write().unwrap();
        session_mappings.insert(session_id, mapping);

        Ok(group_id)
    }

    /// Switch session to a different consumer group
    pub fn switch_session_group(
        &self,
        session_id: SessionId,
        new_group_id: ConsumerGroupId,
        reason: SwitchReason,
    ) -> Result<()> {
        // Verify new group exists
        self.get_group(new_group_id)?;

        let mut session_mappings = self.session_mappings.write().unwrap();
        let mapping = session_mappings.get_mut(&session_id)
            .ok_or_else(|| DbError::NotFound(
                format!("Session {} not found", session_id)
            ))?;

        let old_group_id = mapping.group_id;
        if old_group_id == new_group_id {
            return Ok(()); // No change needed
        }

        // Update session counts
        {
            let mut groups = self.groups.write().unwrap();

            // Decrement old group
            if let Some(old_group) = groups.get_mut(&old_group_id) {
                old_group.decrement_sessions();
            }

            // Increment new group
            if let Some(new_group) = groups.get_mut(&new_group_id) {
                new_group.increment_sessions()?;
            }
        }

        // Update mapping
        mapping.previous_group_id = Some(old_group_id);
        mapping.group_id = new_group_id;
        mapping.switch_time = SystemTime::now();
        mapping.switch_reason = reason;

        Ok(())
    }

    /// Remove session mapping
    pub fn remove_session(&self, session_id: SessionId) -> Result<()> {
        let mut session_mappings = self.session_mappings.write().unwrap();
        let mapping = session_mappings.remove(&session_id)
            .ok_or_else(|| DbError::NotFound(
                format!("Session {} not found", session_id)
            ))?;

        // Decrement session count
        let mut groups = self.groups.write().unwrap();
        if let Some(group) = groups.get_mut(&mapping.group_id) {
            group.decrement_sessions();
        }

        Ok(())
    }

    /// Add an assignment rule
    pub fn add_assignment_rule(
        &self,
        name: String,
        priority: u32,
        condition: RuleCondition,
        target_group_id: ConsumerGroupId,
    ) -> Result<u64> {
        // Verify target group exists
        self.get_group(target_group_id)?;

        let rule_id = {
            let mut next_id = self.next_rule_id.write().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let rule = AssignmentRule {
            id: rule_id,
            name,
            priority,
            condition,
            target_group_id,
            is_enabled: true,
        };

        let mut rules = self.assignment_rules.write().unwrap();
        rules.push(rule);

        // Sort by priority
        rules.sort_by_key(|r| r.priority);

        Ok(rule_id)
    }

    /// Apply assignment rules to determine consumer group
    fn apply_assignment_rules(&self, attrs: &SessionAttributes) -> Option<ConsumerGroupId> {
        let rules = self.assignment_rules.read().unwrap();

        for rule in rules.iter() {
            if !rule.is_enabled {
                continue;
            }

            if rule.condition.evaluate(attrs) {
                return Some(rule.target_group_id);
            }
        }

        None
    }

    /// Get all consumer groups
    pub fn list_groups(&self) -> Vec<ConsumerGroup> {
        let groups = self.groups.read().unwrap();
        groups.values().cloned().collect()
    }

    /// Get session's current consumer group
    pub fn get_session_group(&self, session_id: SessionId) -> Result<ConsumerGroupId> {
        let session_mappings = self.session_mappings.read().unwrap();
        session_mappings.get(&session_id)
            .map(|m| m.group_id)
            .ok_or_else(|| DbError::NotFound(
                format!("Session {} not found", session_id)
            ))
    }

    /// Get statistics for a consumer group
    pub fn get_group_statistics(&self, group_id: ConsumerGroupId) -> Result<GroupStatistics> {
        let group = self.get_group(group_id)?;

        Ok(GroupStatistics {
            group_id,
            group_name: group.name.clone(),
            current_sessions: group.current_sessions,
            max_sessions: group.max_sessions,
            cpu_allocation_pct: group.cpu_allocation_pct,
            priority: group.priority,
        })
    }
}

/// Consumer group statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupStatistics {
    pub group_id: ConsumerGroupId,
    pub group_name: String,
    pub current_sessions: usize,
    pub max_sessions: Option<usize>,
    pub cpu_allocation_pct: u8,
    pub priority: PriorityLevel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_levels() {
        assert!(PriorityLevel::high() < PriorityLevel::medium());
        assert!(PriorityLevel::medium() < PriorityLevel::low());
    }

    #[test]
    fn test_consumer_group_creation() {
        let manager = ConsumerGroupManager::new().unwrap();
        let group_id = manager.create_group(
            "TEST_GROUP".to_string(),
            PriorityLevel::medium(),
            GroupCategory::Interactive,
        ).unwrap();

        let group = manager.get_group(group_id).unwrap();
        assert_eq!(group.name, "TEST_GROUP");
    }

    #[test]
    fn test_user_mapping() {
        let manager = ConsumerGroupManager::new().unwrap();
        let group_id = manager.create_group(
            "TEST_GROUP".to_string(),
            PriorityLevel::medium(),
            GroupCategory::Interactive,
        ).unwrap();

        manager.map_user_to_group(1, "testuser".to_string(), group_id, true).unwrap();

        let attrs = SessionAttributes {
            username: "testuser".to_string(),
            program_name: None,
            machine_name: None,
            service_name: None,
            module_name: None,
            action_name: None,
        };

        let assigned_group = manager.assign_session(1, 1, &attrs).unwrap();
        assert_eq!(assigned_group, group_id);
    }
}


