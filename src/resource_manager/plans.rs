// Resource Plans for Workload Management
//
// This module implements Oracle-like resource plans with directives, sub-plans,
// time-based switching, and maintenance windows.

use std::time::SystemTime;
use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::time::{Duration};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Timelike, Weekday, NaiveTime};

use crate::error::Result;
use super::consumer_groups::ConsumerGroupId;

/// Resource plan identifier
pub type ResourcePlanId = u64;

/// Directive identifier
pub type DirectiveId = u64;

/// Resource plan definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePlan {
    /// Unique identifier
    pub id: ResourcePlanId,
    /// Plan name
    pub name: String,
    /// Plan description
    pub description: Option<String>,
    /// Whether this is a top-level plan
    pub is_top_plan: bool,
    /// Parent plan ID (for sub-plans)
    pub parent_plan_id: Option<ResourcePlanId>,
    /// CPU management method
    pub cpu_method: CpuManagementMethod,
    /// Parallel execution management
    pub parallel_execution_managed: bool,
    /// Active session pool management
    pub active_session_pool_managed: bool,
    /// Maximum utilization limit (percentage)
    pub max_utilization_limit: Option<u8>,
    /// Whether the plan is enabled
    pub is_enabled: bool,
    /// Plan status
    pub status: PlanStatus,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last modified timestamp
    pub modified_at: SystemTime,
}

/// CPU management method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CpuManagementMethod {
    /// Emphasis on CPU allocation
    Emphasis,
    /// Ratio-based allocation
    Ratio,
    /// Shares-based allocation (modern approach)
    Shares,
}

/// Plan status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    /// Plan is active and enforcing resources
    Active,
    /// Plan is inactive
    Inactive,
    /// Plan is in maintenance mode
    Maintenance,
}

/// Resource plan directive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePlanDirective {
    /// Directive identifier
    pub id: DirectiveId,
    /// Plan this directive belongs to
    pub plan_id: ResourcePlanId,
    /// Consumer group this directive applies to
    pub group_id: ConsumerGroupId,
    /// CPU allocation percentage (for EMPHASIS method)
    pub cpu_pct: Option<u8>,
    /// CPU shares (for SHARES method)
    pub cpu_shares: Option<u32>,
    /// Parallel degree limit
    pub parallel_degree_limit: Option<u32>,
    /// Maximum parallel servers
    pub parallel_server_limit: Option<u32>,
    /// Active session pool size
    pub active_sess_pool_p1: Option<u32>,
    /// Queue timeout for active session pool
    pub queueing_p1: Option<Duration>,
    /// Maximum estimated execution time
    pub max_est_exec_time: Option<Duration>,
    /// Maximum idle time
    pub max_idle_time: Option<Duration>,
    /// Maximum idle blocker time
    pub max_idle_blocker_time: Option<Duration>,
    /// Switch group after condition
    pub switch_group: Option<ConsumerGroupId>,
    /// Switch time (CPU time threshold for switching)
    pub switch_time: Option<Duration>,
    /// Switch estimate (estimated execution time threshold)
    pub switch_estimate: bool,
    /// Switch for call (switch back after call completes)
    pub switch_for_call: bool,
    /// Undo pool limit
    pub undo_pool: Option<u64>,
    /// Maximum undo generation rate
    pub max_undo_rate: Option<u64>,
    /// Sub-plan to use for this group
    pub sub_plan_id: Option<ResourcePlanId>,
    /// Directive priority (for evaluation order)
    pub priority: u32,
}

impl ResourcePlanDirective {
    /// Create a new directive
    pub fn new(
        id: DirectiveId,
        planid: ResourcePlanId,
        groupid: ConsumerGroupId,
    ) -> Self {
        Self {
            id,
            plan_id,
            group_id,
            cpu_pct: None,
            cpu_shares: None,
            parallel_degree_limit: None,
            parallel_server_limit: None,
            active_sess_pool_p1: None,
            queueing_p1: None,
            max_est_exec_time: None,
            max_idle_time: None,
            max_idle_blocker_time: None,
            switch_group: None,
            switch_time: None,
            switch_estimate: false,
            switch_for_call: false,
            undo_pool: None,
            max_undo_rate: None,
            sub_plan_id: None,
            priority: 0,
        }
    }
}

/// Time-based plan switching schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSchedule {
    /// Schedule identifier
    pub id: u64,
    /// Schedule name
    pub name: String,
    /// Day of week (0=Sunday, 6=Saturday, None=all days)
    pub day_of_week: Option<u8>,
    /// Start time
    pub start_time: NaiveTime,
    /// End time
    pub end_time: NaiveTime,
    /// Resource plan to activate
    pub plan_id: ResourcePlanId,
    /// Schedule priority (lower = higher priority)
    pub priority: u32,
    /// Whether this schedule is enabled
    pub is_enabled: bool,
}

impl PlanSchedule {
    /// Check if this schedule is active at the given time
    pub fn is_active_at(&self, time: DateTime<Utc>) -> bool {
        if !self.is_enabled {
            return false;
        }

        // Check day of week
        if let Some(dow) = self.day_of_week {
            let current_dow = time.weekday().num_days_from_sunday() as u8;
            if current_dow != dow {
                return false;
            }
        }

        // Check time range
        let current_time = NaiveTime::from_hms_opt(
            time.hour(),
            time.minute(),
            time.second()
        ).unwrap();

        if self.start_time <= self.end_time {
            // Normal range (e.g., 9:00 to 17:00)
            current_time >= self.start_time && current_time < self.end_time
        } else {
            // Overnight range (e.g., 22:00 to 6:00)
            current_time >= self.start_time || current_time < self.end_time
        }
    }
}

/// Maintenance window definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceWindow {
    /// Window identifier
    pub id: u64,
    /// Window name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Day of week
    pub day_of_week: u8,
    /// Start time
    pub start_time: NaiveTime,
    /// Duration
    pub duration: Duration,
    /// Resource plan to use during maintenance
    pub maintenance_plan_id: ResourcePlanId,
    /// Whether to allow user sessions during maintenance
    pub allow_user_sessions: bool,
    /// Maximum concurrent user sessions
    pub max_user_sessions: Option<u32>,
    /// Whether this window is enabled
    pub is_enabled: bool,
}

/// Resource plan manager
pub struct ResourcePlanManager {
    /// All resource plans
    plans: Arc<RwLock<HashMap<ResourcePlanId, ResourcePlan>>>,
    /// Plans indexed by name
    plans_by_name: Arc<RwLock<HashMap<String, ResourcePlanId>>>,
    /// Plan directives
    directives: Arc<RwLock<HashMap<ResourcePlanId, Vec<ResourcePlanDirective>>>>,
    /// Plan schedules
    schedules: Arc<RwLock<Vec<PlanSchedule>>>,
    /// Maintenance windows
    maintenance_windows: Arc<RwLock<Vec<MaintenanceWindow>>>,
    /// Currently active plan
    active_plan_id: Arc<RwLock<Option<ResourcePlanId>>>,
    /// Default plan
    default_plan_id: ResourcePlanId,
    /// Next plan ID
    next_plan_id: Arc<RwLock<ResourcePlanId>>,
    /// Next directive ID
    next_directive_id: Arc<RwLock<DirectiveId>>,
    /// Next schedule ID
    next_schedule_id: Arc<RwLock<u64>>,
    /// Next window ID
    next_window_id: Arc<RwLock<u64>>,
}

impl ResourcePlanManager {
    /// Create a new resource plan manager
    pub fn new() -> Result<Self> {
        let mut manager = Self {
            plans: Arc::new(RwLock::new(HashMap::new())),
            plans_by_name: Arc::new(RwLock::new(HashMap::new())),
            directives: Arc::new(RwLock::new(HashMap::new())),
            schedules: Arc::new(RwLock::new(Vec::new())),
            maintenance_windows: Arc::new(RwLock::new(Vec::new())),
            active_plan_id: Arc::new(RwLock::new(None)),
            default_plan_id: 1,
            next_plan_id: Arc::new(RwLock::new(100)),
            next_directive_id: Arc::new(RwLock::new(1)),
            next_schedule_id: Arc::new(RwLock::new(1)),
            next_window_id: Arc::new(RwLock::new(1)),
        };

        // Create default system plans
        manager.create_system_plans()?;
        Ok(manager)
    }

    /// Create default system resource plans
    fn create_system_plans(&mut self) -> Result<()> {
        // DEFAULT_PLAN - Balanced resource allocation
        let default_plan = ResourcePlan {
            id: 1,
            name: "DEFAULT_PLAN".to_string(),
            description: Some("Default balanced resource plan".to_string()),
            is_top_plan: true,
            parent_plan_id: None,
            cpu_method: CpuManagementMethod::Shares,
            parallel_execution_managed: true,
            active_session_pool_managed: true,
            max_utilization_limit: Some(100),
            is_enabled: true,
            status: PlanStatus::Active,
            created_at: SystemTime::now(),
            modified_at: SystemTime::now(),
        };
        self.register_plan(default_plan)?;

        // DAYTIME_PLAN - Plan for business hours
        let daytime_plan = ResourcePlan {
            id: 2,
            name: "DAYTIME_PLAN".to_string(),
            description: Some("Resource plan for business hours".to_string()),
            is_top_plan: true,
            parent_plan_id: None,
            cpu_method: CpuManagementMethod::Shares,
            parallel_execution_managed: true,
            active_session_pool_managed: true,
            max_utilization_limit: Some(90),
            is_enabled: true,
            status: PlanStatus::Inactive,
            created_at: SystemTime::now(),
            modified_at: SystemTime::now(),
        };
        self.register_plan(daytime_plan)?;

        // NIGHTTIME_PLAN - Plan for off-hours
        let nighttime_plan = ResourcePlan {
            id: 3,
            name: "NIGHTTIME_PLAN".to_string(),
            description: Some("Resource plan for off-hours batch processing".to_string()),
            is_top_plan: true,
            parent_plan_id: None,
            cpu_method: CpuManagementMethod::Shares,
            parallel_execution_managed: true,
            active_session_pool_managed: false,
            max_utilization_limit: Some(100),
            is_enabled: true,
            status: PlanStatus::Inactive,
            created_at: SystemTime::now(),
            modified_at: SystemTime::now(),
        };
        self.register_plan(nighttime_plan)?;

        // MAINTENANCE_PLAN - Plan for maintenance windows
        let maint_plan = ResourcePlan {
            id: 4,
            name: "MAINTENANCE_PLAN".to_string(),
            description: Some("Resource plan for maintenance operations".to_string()),
            is_top_plan: true,
            parent_plan_id: None,
            cpu_method: CpuManagementMethod::Shares,
            parallel_execution_managed: false,
            active_session_pool_managed: false,
            max_utilization_limit: Some(100),
            is_enabled: true,
            status: PlanStatus::Inactive,
            created_at: SystemTime::now(),
            modified_at: SystemTime::now(),
        };
        self.register_plan(maint_plan)?;

        // Activate default plan
        *self.active_plan_id.write().unwrap() = Some(1);

        Ok(())
    }

    /// Register a resource plan
    fn register_plan(&mut self, plan: ResourcePlan) -> Result<()> {
        let id = plan.id;
        let name = plan.name.clone();

        let mut plans = self.plans.write().unwrap();
        let mut plans_by_name = self.plans_by_name.write().unwrap();

        if plans.contains_key(&id) {
            return Err(DbError::AlreadyExists(
                format!("Resource plan with ID {} already exists", id)
            ))));
        }

        if plans_by_name.contains_key(&name) {
            return Err(DbError::AlreadyExists(
                format!("Resource plan {} already exists", name)
            ))));
        }

        plans_by_name.insert(name, id);
        plans.insert(id, plan);
        Ok(())
    }

    /// Create a new resource plan
    pub fn create_plan(
        &self,
        name: String,
        cpu_method: CpuManagementMethod,
        parent_plan_id: Option<ResourcePlanId>,
    ) -> Result<ResourcePlanId> {
        let id = {
            let mut next_id = self.next_plan_id.write().unwrap();
            let current = *next_id;
            *next_id += 1;
            current
        };

        // If parent plan specified, verify it exists
        let is_top_plan = if let Some(parent_id) = parent_plan_id {
            let plans = self.plans.read().unwrap();
            if !plans.contains_key(&parent_id) {
                return Err(DbError::NotFound(
                    format!("Parent plan {} not found", parent_id)
                ))));
            }
            false
        } else {
            true
        };

        let plan = ResourcePlan {
            id,
            name: name.clone(),
            description: None,
            is_top_plan,
            parent_plan_id,
            cpu_method,
            parallel_execution_managed: false,
            active_session_pool_managed: false,
            max_utilization_limit: None,
            is_enabled: true,
            status: PlanStatus::Inactive,
            created_at: SystemTime::now(),
            modified_at: SystemTime::now(),
        };

        let mut plans = self.plans.write().unwrap();
        let mut plans_by_name = self.plans_by_name.write().unwrap();

        if plans_by_name.contains_key(&name) {
            return Err(DbError::AlreadyExists(
                format!("Resource plan {} already exists", name)
            ))));
        }

        plans_by_name.insert(name, id);
        plans.insert(id, plan);

        Ok(id)
    }

    /// Get resource plan by ID
    pub fn get_plan(&self, plan_id: ResourcePlanId) -> Result<ResourcePlan> {
        let plans = self.plans.read().unwrap();
        plans.get(&plan_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(
                format!("Resource plan {} not found", plan_id)
            ))
    }

    /// Get resource plan by name
    pub fn get_plan_by_name(&self, name: &str) -> Result<ResourcePlan> {
        let plans_by_name = self.plans_by_name.read().unwrap()));
        let plan_id = plans_by_name.get(name)
            .ok_or_else(|| DbError::NotFound(
                format!("Resource plan {} not found", name)
            ))?);
        self.get_plan(*plan_id)
    }

    /// Update resource plan
    pub fn update_plan<F>(&self, plan_id: ResourcePlanId, update_fn: F) -> Result<()>
    where
        F: FnOnce(&mut ResourcePlan),
    {
        let mut plans = self.plans.write().unwrap();
        let plan = plans.get_mut(&plan_id)
            .ok_or_else(|| DbError::NotFound(
                format!("Resource plan {} not found", plan_id)
            ))?);

        update_fn(plan);
        plan.modified_at = SystemTime::now();
        Ok(())
    }

    /// Delete a resource plan
    pub fn delete_plan(&self, plan_id: ResourcePlanId) -> Result<()> {
        // Don't allow deleting system plans
        if plan_id < 100 {
            return Err(DbError::PermissionDenied(
                "Cannot delete system resource plans".to_string()
            ));
        }

        // Don't allow deleting active plan
        {
            let active_id = self.active_plan_id.read().unwrap();
            if *active_id == Some(plan_id) {
                return Err(DbError::Conflict(
                    "Cannot delete active resource plan".to_string()
                ));
            }
        }

        let mut plans = self.plans.write().unwrap();
        let mut plans_by_name = self.plans_by_name.write().unwrap();

        let plan = plans.remove(&plan_id)
            .ok_or_else(|| DbError::NotFound(
                format!("Resource plan {} not found", plan_id)
            ))?);

        plans_by_name.remove(&plan.name);

        // Remove directives
        let mut directives = self.directives.write().unwrap();
        directives.remove(&plan_id);

        Ok(())
    }

    /// Create a plan directive
    pub fn create_directive(
        &self,
        planid: ResourcePlanId,
        groupid: ConsumerGroupId,
    ) -> Result<DirectiveId> {
        // Verify plan exists
        self.get_plan(plan_id)?;

        let id = {
            let mut next_id = self.next_directive_id.write().unwrap();
            let current = *next_id;
            *next_id += 1;
            current
        };

        let directive = ResourcePlanDirective::new(id, plan_id, group_id);

        let mut directives = self.directives.write().unwrap();
        let plan_directives = directives.entry(plan_id).or_insert_with(Vec::new);
        plan_directives.push(directive);

        Ok(id)
    }

    /// Update a directive
    pub fn update_directive<F>(
        &self,
        planid: ResourcePlanId,
        directiveid: DirectiveId,
        update_fn: F,
    )> Result<()>
    where
        F: FnOnce(&mut ResourcePlanDirective),
    {
        let mut directives = self.directives.write().unwrap();
        let plan_directives = directives.get_mut(&plan_id)
            .ok_or_else(|| DbError::NotFound(
                format!("No directives for plan {}", plan_id)
            ))?);

        let directive = plan_directives.iter_mut()
            .find(|d| d.id == directive_id)
            .ok_or_else(|| DbError::NotFound(
                format!("Directive {} not found", directive_id)
            ))?);

        update_fn(directive);
        Ok(())
    }

    /// Get directives for a plan
    pub fn get_plan_directives(&self, plan_id: ResourcePlanId) -> Vec<ResourcePlanDirective> {
        let directives = self.directives.read().unwrap();
        directives.get(&plan_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Activate a resource plan
    pub fn activate_plan(&self, plan_id: ResourcePlanId) -> Result<()> {
        // Verify plan exists
        let plan = self.get_plan(plan_id)?;
        if !plan.is_enabled {
            return Err(DbError::Configuration(
                format!("Cannot activate disabled plan {}", plan.name)
            ))));
        }

        // Deactivate current plan
        if let Some(current_id) = *self.active_plan_id.read().unwrap() {
            self.update_plan(current_id, |p| {
                p.status = PlanStatus::Inactive;
            })?;
        }

        // Activate new plan
        self.update_plan(plan_id, |p| {
            p.status = PlanStatus::Active;
        })?;

        *self.active_plan_id.write().unwrap() = Some(plan_id);

        Ok(())
    }

    /// Deactivate current plan and revert to default
    pub fn deactivate_plan(&self) -> Result<()> {
        if let Some(current_id) = *self.active_plan_id.read().unwrap() {
            self.update_plan(current_id, |p| {
                p.status = PlanStatus::Inactive;
            })?;
        }

        // Activate default plan
        self.activate_plan(self.default_plan_id)
    }

    /// Get currently active plan
    pub fn get_active_plan(&self) -> Option<ResourcePlanId> {
        *self.active_plan_id.read().unwrap()
    }

    /// Add a plan schedule
    pub fn add_schedule(
        &self,
        name: String,
        day_of_week: Option<u8>,
        start_time: NaiveTime,
        end_time: NaiveTime,
        planid: ResourcePlanId,
        priority: u32,
    ) -> Result<u64> {
        // Verify plan exists
        self.get_plan(plan_id)?;

        let id = {
            let mut next_id = self.next_schedule_id.write().unwrap();
            let current = *next_id;
            *next_id += 1;
            current
        };

        let schedule = PlanSchedule {
            id,
            name,
            day_of_week,
            start_time,
            end_time,
            plan_id,
            priority,
            is_enabled: true,
        };

        let mut schedules = self.schedules.write().unwrap();
        schedules.push(schedule);
        schedules.sort_by_key(|s| s.priority);

        Ok(id)
    }

    /// Check schedules and switch plan if needed
    pub fn check_and_switch_plan(&self) -> Result<bool> {
        let now = Utc::now();
        let schedules = self.schedules.read().unwrap();

        // Find highest priority active schedule
        for schedule in schedules.iter() {
            if schedule.is_active_at(now) {
                let current_plan = self.get_active_plan();
                if current_plan != Some(schedule.plan_id) {
                    self.activate_plan(schedule.plan_id)?;
                    return Ok(true);
                }
                return Ok(false);
            }
        }

        // No active schedule, use default
        let current_plan = self.get_active_plan();
        if current_plan != Some(self.default_plan_id) {
            self.activate_plan(self.default_plan_id)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Add a maintenance window
    pub fn add_maintenance_window(
        &self,
        name: String,
        day_of_week: u8,
        start_time: NaiveTime,
        duration: Duration,
        maintenance_plan_id: ResourcePlanId,
    ) -> Result<u64> {
        // Verify plan exists
        self.get_plan(maintenance_plan_id)?;

        let id = {
            let mut next_id = self.next_window_id.write().unwrap();
            let current = *next_id;
            *next_id += 1;
            current
        };

        let window = MaintenanceWindow {
            id,
            name,
            description: None,
            day_of_week,
            start_time,
            duration,
            maintenance_plan_id,
            allow_user_sessions: false,
            max_user_sessions: None,
            is_enabled: true,
        };

        let mut windows = self.maintenance_windows.write().unwrap();
        windows.push(window);

        Ok(id)
    }

    /// Check if currently in maintenance window
    pub fn is_in_maintenance_window(&self) -> Option<MaintenanceWindow> {
        let now = Utc::now();
        let windows = self.maintenance_windows.read().unwrap();

        for window in windows.iter() {
            if !window.is_enabled {
                continue;
            }

            let current_dow = now.weekday().num_days_from_sunday() as u8;
            if current_dow != window.day_of_week {
                continue;
            }

            let current_time = NaiveTime::from_hms_opt(
                now.hour(),
                now.minute(),
                now.second()
            ).unwrap();

            let end_time = {
                let total_seconds = window.start_time.num_seconds_from_midnight() as u64
                    + window.duration.as_secs();
                NaiveTime::from_num_seconds_from_midnight_opt(
                    (total_seconds % 86400) as u32,
                    0
                ).unwrap()
            };

            if current_time >= window.start_time && current_time < end_time {
                return Some(window.clone());
            }
        }

        None
    }

    /// List all resource plans
    pub fn list_plans(&self) -> Vec<ResourcePlan> {
        let plans = self.plans.read().unwrap();
        plans.values().cloned().collect()
    }

    /// Validate plan directives
    pub fn validate_plan(&self, plan_id: ResourcePlanId) -> Result<Vec<String>> {
        let plan = self.get_plan(plan_id)?;
        let directives = self.get_plan_directives(plan_id);
        let mut warnings = Vec::new();

        // Validate CPU allocation totals
        match plan.cpu_method {
            CpuManagementMethod::Emphasis | CpuManagementMethod::Ratio => {
                let total_pct: u32 = directives.iter()
                    .filter_map(|d| d.cpu_pct.map(|p| p as u32))
                    .sum();

                if total_pct > 100 {
                    warnings.push(format!(
                        "Total CPU allocation exceeds 100%: {}%",
                        total_pct
                    ))));
                } else if total_pct < 100 {
                    warnings.push(format!(
                        "Total CPU allocation is less than 100%: {}%",
                        total_pct
                    ))));
                }
            }
            CpuManagementMethod::Shares => {
                // Shares don't need to total to a specific value
                if directives.iter().all(|d| d.cpu_shares.is_none()) {
                    warnings.push("No CPU shares defined for any directive".to_string());
                }
            }
        }

        // Check for directives with sub-plans
        for directive in &directives {
            if let Some(sub_plan_id) = directive.sub_plan_id {
                let sub_plan = self.get_plan(sub_plan_id)?;
                if sub_plan.is_top_plan {
                    warnings.push(format!(
                        "Sub-plan {} should not be a top-level plan",
                        sub_plan.name
                    ))));
                }
            }
        }

        Ok(warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_creation() {
        let manager = ResourcePlanManager::new().unwrap();
        let plan_id = manager.create_plan(
            "TEST_PLAN".to_string(),
            CpuManagementMethod::Shares,
            None,
        ).unwrap();

        let plan = manager.get_plan(plan_id).unwrap();
        assert_eq!(plan.name, "TEST_PLAN");
        assert!(plan.is_top_plan);
    }

    #[test]
    fn test_plan_activation() {
        let manager = ResourcePlanManager::new().unwrap();
        let plan_id = manager.create_plan(
            "TEST_PLAN".to_string(),
            CpuManagementMethod::Shares,
            None,
        ).unwrap();

        manager.activate_plan(plan_id).unwrap();
        assert_eq!(manager.get_active_plan(), Some(plan_id));
    }

    #[test]
    fn test_schedule_check() {
        let manager = ResourcePlanManager::new().unwrap();

        // Create a schedule that should be active right now
        let now = Utc::now();
        let start_time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        let end_time = NaiveTime::from_hms_opt(23, 59, 59).unwrap();

        let plan_id = manager.create_plan(
            "TEST_PLAN".to_string(),
            CpuManagementMethod::Shares,
            None,
        ).unwrap();

        manager.add_schedule(
            "TEST_SCHEDULE".to_string(),
            None, // All days
            start_time,
            end_time,
            plan_id,
            1,
        ).unwrap();

        let switched = manager.check_and_switch_plan().unwrap();
        assert!(switched || manager.get_active_plan() == Some(plan_id));
    }
}
