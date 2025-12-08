// Tenant consolidation and workload management
// Implements intelligent tenant placement, rebalancing, and consolidation planning

use std::collections::{HashMap, BTreeMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, Duration};

/// Consolidation error types
#[derive(Debug, Clone)]
pub enum ConsolidationError {
    InsufficientCapacity(String),
    ConflictingAffinityRules(String),
    InvalidPlacement(String),
    RebalancingFailed(String),
    MetricsUnavailable(String),
}

impl std::fmt::Display for ConsolidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsolidationError::InsufficientCapacity(msg) => write!(f, "Insufficient capacity: {}", msg),
            ConsolidationError::ConflictingAffinityRules(msg) => write!(f, "Conflicting affinity rules: {}", msg),
            ConsolidationError::InvalidPlacement(msg) => write!(f, "Invalid placement: {}", msg),
            ConsolidationError::RebalancingFailed(msg) => write!(f, "Rebalancing failed: {}", msg),
            ConsolidationError::MetricsUnavailable(msg) => write!(f, "Metrics unavailable: {}", msg),
        }
    }
}

impl std::error::Error for ConsolidationError {}

pub type ConsolidationResult<T> = Result<T, ConsolidationError>;

/// Workload characteristics for placement decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadProfile {
    pub tenant_id: String,
    pub avg_cpu_percent: f64,
    pub peak_cpu_percent: f64,
    pub avg_memory_mb: u64,
    pub peak_memory_mb: u64,
    pub avg_iops: u32,
    pub peak_iops: u32,
    pub avg_network_mbps: u32,
    pub read_write_ratio: f64,
    pub transaction_rate: f64,
    pub query_complexity: QueryComplexity,
    pub workload_type: WorkloadType,
    pub time_pattern: TimePattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryComplexity {
    Simple,      // Simple queries, low resource usage
    Moderate,    // Moderate complexity
    Complex,     // Complex queries, high resource usage
    Analytical,  // Long-running analytical queries
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkloadType {
    Oltp,        // Online transaction processing
    Olap,        // Online analytical processing
    Mixed,       // Mixed workload
    Batch,       // Batch processing
    Reporting,   // Reporting workload
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimePattern {
    Continuous,   // 24/7 workload
    BusinessHours, // Peak during business hours
    Periodic,     // Periodic spikes
    Sporadic,     // Unpredictable pattern
}

/// Host/node where tenants can be placed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationHost {
    pub host_id: String,
    pub host_name: String,
    pub total_cpu_cores: u32,
    pub total_memory_mb: u64,
    pub total_storage_gb: u64,
    pub total_iops: u32,
    pub allocated_cpu_cores: f64,
    pub allocated_memory_mb: u64,
    pub allocated_storage_gb: u64,
    pub allocated_iops: u32,
    pub tenant_count: u32,
    pub max_tenants: u32,
    pub is_active: bool,
    pub maintenance_mode: bool,
}

impl ConsolidationHost {
    pub fn new(host_id: String, cpu_cores: u32, memory_mb: u64, storage_gb: u64, iops: u32) -> Self {
        Self {
            host_id,
            host_name: format!("host-{}", uuid::Uuid::new_v4()),
            total_cpu_cores: cpu_cores,
            total_memory_mb: memory_mb,
            total_storage_gb: storage_gb,
            total_iops: iops,
            allocated_cpu_cores: 0.0,
            allocated_memory_mb: 0,
            allocated_storage_gb: 0,
            allocated_iops: 0,
            tenant_count: 0,
            max_tenants: 100,
            is_active: true,
            maintenance_mode: false,
        }
    }

    pub fn available_cpu(&self) -> f64 {
        self.total_cpu_cores as f64 - self.allocated_cpu_cores
    }

    pub fn available_memory(&self) -> u64 {
        self.total_memory_mb.saturating_sub(self.allocated_memory_mb)
    }

    pub fn available_storage(&self) -> u64 {
        self.total_storage_gb.saturating_sub(self.allocated_storage_gb)
    }

    pub fn available_iops(&self) -> u32 {
        self.total_iops.saturating_sub(self.allocated_iops)
    }

    pub fn can_accommodate(&self, profile: &WorkloadProfile) -> bool {
        if self.maintenance_mode || !self.is_active || self.tenant_count >= self.max_tenants {
            return false;
        }

        self.available_cpu() >= profile.peak_cpu_percent / 100.0
            && self.available_memory() >= profile.peak_memory_mb
            && self.available_iops() >= profile.peak_iops
    }

    pub fn utilization_score(&self) -> f64 {
        let cpu_util = self.allocated_cpu_cores / self.total_cpu_cores as f64;
        let mem_util = self.allocated_memory_mb as f64 / self.total_memory_mb as f64;
        let iops_util = self.allocated_iops as f64 / self.total_iops as f64;

        (cpu_util + mem_util + iops_util) / 3.0
    }
}

/// Tenant placement on a host
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantPlacement {
    pub tenant_id: String,
    pub host_id: String,
    pub placement_time: SystemTime,
    pub workload_profile: WorkloadProfile,
    pub affinity_score: f64,
}

/// Affinity and anti-affinity rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityRule {
    pub rule_id: String,
    pub rule_type: AffinityType,
    pub tenant_ids: Vec<String>,
    pub description: String,
    pub is_hard: bool, // Hard rule (must enforce) vs soft rule (prefer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AffinityType {
    Affinity,     // Tenants should be co-located
    AntiAffinity, // Tenants should NOT be co-located
}

/// Consolidation plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationPlan {
    pub plan_id: String,
    pub creation_time: SystemTime,
    pub placements: Vec<PlacementAction>,
    pub estimated_savings: f64,
    pub estimated_efficiency: f64,
    pub status: PlanStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementAction {
    pub action_type: ActionType,
    pub tenant_id: String,
    pub source_host: Option<String>,
    pub target_host: String,
    pub estimated_downtime_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    Place,      // Initial placement
    Move,       // Move to different host
    Stay,       // Stay on current host
    Consolidate, // Consolidate with others
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    Draft,
    Analyzing,
    Approved,
    Executing,
    Completed,
    Failed,
}

/// Consolidation metrics and reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationMetrics {
    pub total_hosts: u32,
    pub active_hosts: u32,
    pub total_tenants: u32,
    pub avg_tenants_per_host: f64,
    pub avg_host_utilization: f64,
    pub underutilized_hosts: u32,
    pub overutilized_hosts: u32,
    pub consolidation_ratio: f64,
    pub potential_savings_percent: f64,
    pub measurement_time: SystemTime,
}

/// Main consolidation planner
pub struct ConsolidationPlanner {
    hosts: Arc<RwLock<HashMap<String, ConsolidationHost>>>,
    placements: Arc<RwLock<HashMap<String, TenantPlacement>>>,
    affinity_rules: Arc<RwLock<Vec<AffinityRule>>>,
    workload_profiles: Arc<RwLock<HashMap<String, WorkloadProfile>>>,
    plans: Arc<RwLock<HashMap<String, ConsolidationPlan>>>,
}

impl ConsolidationPlanner {
    pub fn new() -> Self {
        Self {
            hosts: Arc::new(RwLock::new(HashMap::new())),
            placements: Arc::new(RwLock::new(HashMap::new())),
            affinity_rules: Arc::new(RwLock::new(Vec::new())),
            workload_profiles: Arc::new(RwLock::new(HashMap::new())),
            plans: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a host
    pub async fn register_host(&self, host: ConsolidationHost) {
        let mut hosts = self.hosts.write().await;
        hosts.insert(host.host_id.clone(), host);
    }

    /// Update workload profile for a tenant
    pub async fn update_workload_profile(&self, profile: WorkloadProfile) {
        let mut profiles = self.workload_profiles.write().await;
        profiles.insert(profile.tenant_id.clone(), profile);
    }

    /// Add affinity rule
    pub async fn add_affinity_rule(&self, rule: AffinityRule) -> ConsolidationResult<()> {
        // Validate rule doesn't conflict with existing rules
        let rules = self.affinity_rules.read().await;
        for existing in rules.iter() {
            if self.rules_conflict(&rule, existing) {
                return Err(ConsolidationError::ConflictingAffinityRules(
                    format!("Rule conflicts with {}", existing.rule_id)
                ));
            }
        }
        drop(rules);

        let mut rules = self.affinity_rules.write().await;
        rules.push(rule);

        Ok(())
    }

    fn rules_conflict(&self, rule1: &AffinityRule, rule2: &AffinityRule) -> bool {
        // Check if same tenants have conflicting affinity types
        if rule1.rule_type != rule2.rule_type {
            let tenants1: HashSet<_> = rule1.tenant_ids.iter().collect();
            let tenants2: HashSet<_> = rule2.tenant_ids.iter().collect();

            // If they share tenants and have opposite types, they conflict
            let intersection: Vec<_> = tenants1.intersection(&tenants2).collect();
            !intersection.is_empty()
        } else {
            false
        }
    }

    /// Find optimal placement for a tenant
    pub async fn find_placement(
        &self,
        tenant_id: String,
    ) -> ConsolidationResult<String> {
        let profiles = self.workload_profiles.read().await;
        let profile = profiles.get(&tenant_id)
            .ok_or_else(|| ConsolidationError::MetricsUnavailable(
                format!("No workload profile for tenant {}", tenant_id)
            ))?
            .clone();
        drop(profiles);

        let hosts = self.hosts.read().await;
        let affinity_rules = self.affinity_rules.read().await;
        let placements = self.placements.read().await;

        // Score each host
        let mut host_scores: Vec<(String, f64)> = Vec::new();

        for (host_id, host) in hosts.iter() {
            if !host.can_accommodate(&profile) {
                continue;
            }

            let mut score = self.calculate_placement_score(
                host,
                &profile,
                &affinity_rules,
                &placements,
            );

            // Apply affinity rules
            score += self.calculate_affinity_score(
                host_id,
                &tenant_id,
                &affinity_rules,
                &placements,
            );

            host_scores.push((host_id.clone(), score));
        }

        drop(hosts);
        drop(affinity_rules);
        drop(placements);

        // Select host with highest score
        host_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        host_scores.first()
            .map(|(host_id, _)| host_id.clone())
            .ok_or_else(|| ConsolidationError::InsufficientCapacity(
                "No suitable host found".to_string()
            ))
    }

    fn calculate_placement_score(
        &self,
        host: &ConsolidationHost,
        profile: &WorkloadProfile,
        _affinity_rules: &[AffinityRule],
        _placements: &HashMap<String, TenantPlacement>,
    ) -> f64 {
        let mut score = 100.0;

        // Prefer hosts with moderate utilization (avoid both empty and full hosts)
        let utilization = host.utilization_score();
        let optimal_utilization = 0.7;
        let utilization_penalty = ((utilization - optimal_utilization).abs() * 50.0).min(30.0);
        score -= utilization_penalty;

        // Prefer hosts with enough headroom
        let cpu_headroom = host.available_cpu() / host.total_cpu_cores as f64;
        let mem_headroom = host.available_memory() as f64 / host.total_memory_mb as f64;

        if cpu_headroom < 0.2 || mem_headroom < 0.2 {
            score -= 20.0;
        }

        // Workload type affinity (prefer similar workloads together)
        // This would check existing tenants on the host

        score
    }

    fn calculate_affinity_score(
        &self,
        host_id: &str,
        tenant_id: &str,
        affinity_rules: &[AffinityRule],
        placements: &HashMap<String, TenantPlacement>,
    ) -> f64 {
        let mut score = 0.0;

        for rule in affinity_rules {
            if !rule.tenant_ids.contains(&tenant_id.to_string()) {
                continue;
            }

            // Find how many related tenants are on this host
            let related_on_host = rule.tenant_ids.iter()
                .filter(|tid| *tid != tenant_id)
                .filter(|tid| {
                    placements.get(*tid)
                        .map(|p| p.host_id == host_id)
                        .unwrap_or(false)
                })
                .count();

            match rule.rule_type {
                AffinityType::Affinity => {
                    // Prefer hosts with related tenants
                    score += related_on_host as f64 * if rule.is_hard { 50.0 } else { 10.0 };
                }
                AffinityType::AntiAffinity => {
                    // Penalize hosts with related tenants
                    score -= related_on_host as f64 * if rule.is_hard { 100.0 } else { 20.0 };
                }
            }
        }

        score
    }

    /// Place a tenant on a host
    pub async fn place_tenant(
        &self,
        tenant_id: String,
        host_id: Option<String>,
    ) -> ConsolidationResult<String> {
        let target_host = match host_id {
            Some(h) => h,
            None => self.find_placement(tenant_id.clone()).await?,
        };

        let profiles = self.workload_profiles.read().await;
        let profile = profiles.get(&tenant_id)
            .ok_or_else(|| ConsolidationError::MetricsUnavailable(
                format!("No workload profile for tenant {}", tenant_id)
            ))?
            .clone();
        drop(profiles);

        // Update host allocation
        let mut hosts = self.hosts.write().await;
        let host = hosts.get_mut(&target_host)
            .ok_or_else(|| ConsolidationError::InvalidPlacement(
                format!("Host {} not found", target_host)
            ))?;

        if !host.can_accommodate(&profile) {
            return Err(ConsolidationError::InsufficientCapacity(
                format!("Host {} cannot accommodate tenant", target_host)
            ));
        }

        host.allocated_cpu_cores += profile.peak_cpu_percent / 100.0;
        host.allocated_memory_mb += profile.peak_memory_mb;
        host.allocated_iops += profile.peak_iops;
        host.tenant_count += 1;

        drop(hosts);

        // Record placement
        let placement = TenantPlacement {
            tenant_id: tenant_id.clone(),
            host_id: target_host.clone(),
            placement_time: SystemTime::now(),
            workload_profile: profile,
            affinity_score: 0.0,
        };

        let mut placements = self.placements.write().await;
        placements.insert(tenant_id, placement);

        Ok(target_host)
    }

    /// Generate consolidation plan
    pub async fn generate_consolidation_plan(&self) -> ConsolidationResult<ConsolidationPlan> {
        let hosts = self.hosts.read().await;
        let placements = self.placements.read().await;

        let mut actions = Vec::new();
        let mut underutilized_hosts = Vec::new();

        // Identify underutilized hosts
        for (host_id, host) in hosts.iter() {
            let utilization = host.utilization_score();
            if utilization < 0.3 && host.tenant_count > 0 {
                underutilized_hosts.push(host_id.clone());
            }
        }

        // Create consolidation actions
        for host_id in &underutilized_hosts {
            // Find tenants on this host
            let tenants_on_host: Vec<_> = placements.iter()
                .filter(|(_, p)| p.host_id == *host_id)
                .map(|(tid, _)| tid.clone())
                .collect();

            // Try to move them to better hosts
            for tenant_id in tenants_on_host {
                if let Ok(target_host) = self.find_placement(tenant_id.clone()).await {
                    if target_host != *host_id {
                        actions.push(PlacementAction {
                            action_type: ActionType::Move,
                            tenant_id,
                            source_host: Some(host_id.clone()),
                            target_host,
                            estimated_downtime_ms: 5000,
                        });
                    }
                }
            }
        }

        let savings = underutilized_hosts.len() as f64 * 10.0; // Simplified savings calculation

        let plan = ConsolidationPlan {
            plan_id: uuid::Uuid::new_v4().to_string(),
            creation_time: SystemTime::now(),
            placements: actions,
            estimated_savings: savings,
            estimated_efficiency: 0.85,
            status: PlanStatus::Draft,
        };

        let mut plans = self.plans.write().await;
        plans.insert(plan.plan_id.clone(), plan.clone());

        Ok(plan)
    }

    /// Execute automatic rebalancing
    pub async fn execute_rebalancing(&self) -> ConsolidationResult<Vec<String>> {
        let plan = self.generate_consolidation_plan().await?;

        let mut moved_tenants = Vec::new();

        for action in &plan.placements {
            match action.action_type {
                ActionType::Move => {
                    // Remove from source host
                    if let Some(source) = &action.source_host {
                        self.remove_tenant_from_host(&action.tenant_id, source).await?;
                    }

                    // Place on target host
                    self.place_tenant(
                        action.tenant_id.clone(),
                        Some(action.target_host.clone()),
                    ).await?;

                    moved_tenants.push(action.tenant_id.clone());
                }
                _ => {}
            }
        }

        // Update plan status
        let mut plans = self.plans.write().await;
        if let Some(plan) = plans.get_mut(&plan.plan_id) {
            plan.status = PlanStatus::Completed;
        }

        Ok(moved_tenants)
    }

    async fn remove_tenant_from_host(
        &self,
        tenant_id: &str,
        host_id: &str,
    ) -> ConsolidationResult<()> {
        let placements = self.placements.read().await;
        let placement = placements.get(tenant_id)
            .ok_or_else(|| ConsolidationError::InvalidPlacement(
                format!("Tenant {} not placed", tenant_id)
            ))?;

        let profile = placement.workload_profile.clone();
        drop(placements);

        let mut hosts = self.hosts.write().await;
        if let Some(host) = hosts.get_mut(host_id) {
            host.allocated_cpu_cores -= profile.peak_cpu_percent / 100.0;
            host.allocated_memory_mb = host.allocated_memory_mb.saturating_sub(profile.peak_memory_mb);
            host.allocated_iops = host.allocated_iops.saturating_sub(profile.peak_iops);
            host.tenant_count = host.tenant_count.saturating_sub(1);
        }

        Ok(())
    }

    /// Get consolidation metrics
    pub async fn get_metrics(&self) -> ConsolidationMetrics {
        let hosts = self.hosts.read().await;
        let placements = self.placements.read().await;

        let total_hosts = hosts.len() as u32;
        let active_hosts = hosts.values().filter(|h| h.is_active).count() as u32;
        let total_tenants = placements.len() as u32;

        let avg_tenants_per_host = if active_hosts > 0 {
            total_tenants as f64 / active_hosts as f64
        } else {
            0.0
        };

        let mut total_utilization = 0.0;
        let mut underutilized = 0;
        let mut overutilized = 0;

        for host in hosts.values() {
            if !host.is_active {
                continue;
            }

            let util = host.utilization_score();
            total_utilization += util;

            if util < 0.3 {
                underutilized += 1;
            } else if util > 0.9 {
                overutilized += 1;
            }
        }

        let avg_host_utilization = if active_hosts > 0 {
            total_utilization / active_hosts as f64
        } else {
            0.0
        };

        let consolidation_ratio = if total_hosts > 0 {
            active_hosts as f64 / total_hosts as f64
        } else {
            0.0
        };

        let potential_savings = (underutilized as f64 / total_hosts.max(1) as f64) * 100.0;

        ConsolidationMetrics {
            total_hosts,
            active_hosts,
            total_tenants,
            avg_tenants_per_host,
            avg_host_utilization,
            underutilized_hosts: underutilized,
            overutilized_hosts: overutilized,
            consolidation_ratio,
            potential_savings_percent: potential_savings,
            measurement_time: SystemTime::now(),
        }
    }

    /// Get placement for a tenant
    pub async fn get_placement(&self, tenant_id: &str) -> Option<TenantPlacement> {
        let placements = self.placements.read().await;
        placements.get(tenant_id).cloned()
    }

    /// List all placements
    pub async fn list_placements(&self) -> Vec<TenantPlacement> {
        let placements = self.placements.read().await;
        placements.values().cloned().collect()
    }
}

impl Default for ConsolidationPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_host_registration() {
        let planner = ConsolidationPlanner::new();
        let host = ConsolidationHost::new(
            "host1".to_string(),
            16,
            65536,
            1000,
            10000,
        );

        planner.register_host(host).await;

        let metrics = planner.get_metrics().await;
        assert_eq!(metrics.total_hosts, 1);
    }

    #[tokio::test]
    async fn test_tenant_placement() {
        let planner = ConsolidationPlanner::new();

        let host = ConsolidationHost::new(
            "host1".to_string(),
            16,
            65536,
            1000,
            10000,
        );
        planner.register_host(host).await;

        let profile = WorkloadProfile {
            tenant_id: "tenant1".to_string(),
            avg_cpu_percent: 20.0,
            peak_cpu_percent: 40.0,
            avg_memory_mb: 2048,
            peak_memory_mb: 4096,
            avg_iops: 500,
            peak_iops: 1000,
            avg_network_mbps: 50,
            read_write_ratio: 0.7,
            transaction_rate: 100.0,
            query_complexity: QueryComplexity::Moderate,
            workload_type: WorkloadType::Oltp,
            time_pattern: TimePattern::BusinessHours,
        };

        planner.update_workload_profile(profile).await;

        let result = planner.place_tenant("tenant1".to_string(), None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "host1");
    }

    #[tokio::test]
    async fn test_affinity_rules() {
        let planner = ConsolidationPlanner::new();

        let rule = AffinityRule {
            rule_id: "rule1".to_string(),
            rule_type: AffinityType::AntiAffinity,
            tenant_ids: vec!["tenant1".to_string(), "tenant2".to_string()],
            description: "Keep these tenants apart".to_string(),
            is_hard: true,
        };

        let result = planner.add_affinity_rule(rule).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_consolidation_plan() {
        let planner = ConsolidationPlanner::new();

        // Add hosts
        for i in 0..3 {
            let host = ConsolidationHost::new(
                format!("host{}", i),
                16,
                65536,
                1000,
                10000,
            );
            planner.register_host(host).await;
        }

        let plan = planner.generate_consolidation_plan().await;
        assert!(plan.is_ok());
    }
}
