// # Governance Policy Engine
//
// This module implements policy definition, storage, evaluation, and inheritance
// for query governance. Policies can be defined at global, role, and user levels
// with proper inheritance and violation handling.

use crate::error::{DbError, Result};
use crate::common::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

// ============================================================================
// Policy Types and Structures
// ============================================================================

/// Policy scope determines where a policy applies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyScope {
    /// Global policy applies to all users
    Global,
    /// Role-based policy applies to specific roles
    Role,
    /// User-specific policy
    User,
}

/// Policy priority for conflict resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PolicyPriority {
    /// Lowest priority (Global)
    Low = 1,
    /// Medium priority (Role)
    Medium = 2,
    /// Highest priority (User)
    High = 3,
}

impl From<PolicyScope> for PolicyPriority {
    fn from(scope: PolicyScope) -> Self {
        match scope {
            PolicyScope::Global => PolicyPriority::Low,
            PolicyScope::Role => PolicyPriority::Medium,
            PolicyScope::User => PolicyPriority::High,
        }
    }
}

/// Resource limits for policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyLimits {
    /// Maximum CPU time in milliseconds
    pub max_cpu_time_ms: Option<u64>,
    /// Maximum memory in bytes
    pub max_memory_bytes: Option<u64>,
    /// Maximum execution time in milliseconds
    pub max_execution_time_ms: Option<u64>,
    /// Maximum result set size (rows)
    pub max_result_rows: Option<u64>,
    /// Maximum concurrent queries
    pub max_concurrent_queries: Option<u32>,
    /// Maximum query complexity score
    pub max_complexity_score: Option<u64>,
}

impl Default for PolicyLimits {
    fn default() -> Self {
        Self {
            max_cpu_time_ms: None,
            max_memory_bytes: None,
            max_execution_time_ms: None,
            max_result_rows: None,
            max_concurrent_queries: None,
            max_complexity_score: None,
        }
    }
}

impl PolicyLimits {
    /// Merge two policy limits, taking the more restrictive value
    pub fn merge(&self, other: &PolicyLimits) -> PolicyLimits {
        PolicyLimits {
            max_cpu_time_ms: Self::min_option(self.max_cpu_time_ms, other.max_cpu_time_ms),
            max_memory_bytes: Self::min_option(self.max_memory_bytes, other.max_memory_bytes),
            max_execution_time_ms: Self::min_option(
                self.max_execution_time_ms,
                other.max_execution_time_ms,
            ),
            max_result_rows: Self::min_option(self.max_result_rows, other.max_result_rows),
            max_concurrent_queries: Self::min_option(
                self.max_concurrent_queries,
                other.max_concurrent_queries,
            ),
            max_complexity_score: Self::min_option(
                self.max_complexity_score,
                other.max_complexity_score,
            ),
        }
    }

    fn min_option<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
        match (a, b) {
            (Some(x), Some(y)) => Some(x.min(y)),
            (Some(x), None) => Some(x),
            (None, Some(y)) => Some(y),
            (None, None) => None,
        }
    }
}

/// Governance policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernancePolicy {
    /// Unique policy identifier
    pub policy_id: String,
    /// Policy name
    pub name: String,
    /// Policy description
    pub description: String,
    /// Policy scope
    pub scope: PolicyScope,
    /// Target identifier (username or role name)
    pub target: String,
    /// Resource limits
    pub limits: PolicyLimits,
    /// Whether policy is enabled
    pub enabled: bool,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last updated timestamp
    pub updated_at: SystemTime,
}

impl GovernancePolicy {
    /// Create a new governance policy
    pub fn new(
        policy_id: String,
        name: String,
        description: String,
        scope: PolicyScope,
        target: String,
        limits: PolicyLimits,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            policy_id,
            name,
            description,
            scope,
            target,
            limits,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get policy priority
    pub fn priority(&self) -> PolicyPriority {
        self.scope.into()
    }
}

/// Policy violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    /// Violation ID
    pub violation_id: String,
    /// Policy that was violated
    pub policy_id: String,
    /// User who violated the policy
    pub user_id: String,
    /// Session ID
    pub session_id: SessionId,
    /// Violation type
    pub violation_type: ViolationType,
    /// Violation details
    pub details: String,
    /// Timestamp of violation
    pub timestamp: SystemTime,
    /// Whether violation was handled
    pub handled: bool,
}

/// Type of policy violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationType {
    /// CPU time limit exceeded
    CpuTimeExceeded,
    /// Memory limit exceeded
    MemoryExceeded,
    /// Execution time limit exceeded
    ExecutionTimeExceeded,
    /// Result set size limit exceeded
    ResultSizeExceeded,
    /// Concurrent query limit exceeded
    ConcurrencyExceeded,
    /// Query complexity limit exceeded
    ComplexityExceeded,
}

impl std::fmt::Display for ViolationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViolationType::CpuTimeExceeded => write!(f, "CPU time limit exceeded"),
            ViolationType::MemoryExceeded => write!(f, "Memory limit exceeded"),
            ViolationType::ExecutionTimeExceeded => write!(f, "Execution time limit exceeded"),
            ViolationType::ResultSizeExceeded => write!(f, "Result size limit exceeded"),
            ViolationType::ConcurrencyExceeded => write!(f, "Concurrent query limit exceeded"),
            ViolationType::ComplexityExceeded => write!(f, "Query complexity limit exceeded"),
        }
    }
}

// ============================================================================
// Policy Engine
// ============================================================================

/// Governance policy engine
#[derive(Clone)]
pub struct PolicyEngine {
    /// Policy storage indexed by policy ID
    policies: Arc<RwLock<HashMap<String, GovernancePolicy>>>,
    /// User to policies mapping
    user_policies: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Role to policies mapping
    role_policies: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Global policies
    global_policies: Arc<RwLock<Vec<String>>>,
    /// Violation records
    violations: Arc<RwLock<Vec<PolicyViolation>>>,
    /// Maximum violations to store
    max_violations: usize,
}

impl PolicyEngine {
    /// Create a new policy engine
    pub fn new() -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            user_policies: Arc::new(RwLock::new(HashMap::new())),
            role_policies: Arc::new(RwLock::new(HashMap::new())),
            global_policies: Arc::new(RwLock::new(Vec::new())),
            violations: Arc::new(RwLock::new(Vec::new())),
            max_violations: 10000,
        }
    }

    /// Add a new policy
    pub fn add_policy(&self, policy: GovernancePolicy) -> Result<()> {
        let policy_id = policy.policy_id.clone();
        let scope = policy.scope;
        let target = policy.target.clone();

        // Store the policy
        {
            let mut policies = self.policies.write()
                .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;

            if policies.contains_key(&policy_id) {
                return Err(DbError::AlreadyExists(format!(
                    "Policy {} already exists",
                    policy_id
                )));
            }

            policies.insert(policy_id.clone(), policy);
        }

        // Index the policy
        match scope {
            PolicyScope::Global => {
                let mut global = self.global_policies.write()
                    .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;
                global.push(policy_id);
            }
            PolicyScope::Role => {
                let mut roles = self.role_policies.write()
                    .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;
                roles.entry(target).or_insert_with(Vec::new).push(policy_id);
            }
            PolicyScope::User => {
                let mut users = self.user_policies.write()
                    .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;
                users.entry(target).or_insert_with(Vec::new).push(policy_id);
            }
        }

        Ok(())
    }

    /// Remove a policy
    pub fn remove_policy(&self, policy_id: &str) -> Result<()> {
        let policy = {
            let mut policies = self.policies.write()
                .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;

            policies.remove(policy_id)
                .ok_or_else(|| DbError::NotFound(format!("Policy {} not found", policy_id)))?
        };

        // Remove from index
        match policy.scope {
            PolicyScope::Global => {
                let mut global = self.global_policies.write()
                    .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;
                global.retain(|id| id != policy_id);
            }
            PolicyScope::Role => {
                let mut roles = self.role_policies.write()
                    .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;
                if let Some(role_policies) = roles.get_mut(&policy.target) {
                    role_policies.retain(|id| id != policy_id);
                }
            }
            PolicyScope::User => {
                let mut users = self.user_policies.write()
                    .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;
                if let Some(user_policies) = users.get_mut(&policy.target) {
                    user_policies.retain(|id| id != policy_id);
                }
            }
        }

        Ok(())
    }

    /// Get effective limits for a user (with inheritance)
    pub fn get_effective_limits(&self, user_id: &str, roles: &[String]) -> Result<PolicyLimits> {
        let policies = self.policies.read()
            .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;

        let mut applicable_policies = Vec::new();

        // Collect global policies
        {
            let global = self.global_policies.read()
                .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;
            for policy_id in global.iter() {
                if let Some(policy) = policies.get(policy_id) {
                    if policy.enabled {
                        applicable_policies.push(policy.clone());
                    }
                }
            }
        }

        // Collect role policies
        {
            let role_map = self.role_policies.read()
                .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;
            for role in roles {
                if let Some(policy_ids) = role_map.get(role) {
                    for policy_id in policy_ids {
                        if let Some(policy) = policies.get(policy_id) {
                            if policy.enabled {
                                applicable_policies.push(policy.clone());
                            }
                        }
                    }
                }
            }
        }

        // Collect user policies
        {
            let user_map = self.user_policies.read()
                .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;
            if let Some(policy_ids) = user_map.get(user_id) {
                for policy_id in policy_ids {
                    if let Some(policy) = policies.get(policy_id) {
                        if policy.enabled {
                            applicable_policies.push(policy.clone());
                        }
                    }
                }
            }
        }

        // Sort by priority (highest first)
        applicable_policies.sort_by(|a, b| b.priority().cmp(&a.priority()));

        // Merge all policies, taking most restrictive limits
        let mut effective_limits = PolicyLimits::default();
        for policy in applicable_policies {
            effective_limits = effective_limits.merge(&policy.limits);
        }

        Ok(effective_limits)
    }

    /// Record a policy violation
    pub fn record_violation(
        &self,
        policy_id: String,
        user_id: String,
        session_id: SessionId,
        violation_type: ViolationType,
        details: String,
    ) -> Result<()> {
        let violation = PolicyViolation {
            violation_id: format!("viol_{}", uuid::Uuid::new_v4()),
            policy_id,
            user_id,
            session_id,
            violation_type,
            details,
            timestamp: SystemTime::now(),
            handled: false,
        };

        let mut violations = self.violations.write()
            .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;

        violations.push(violation);

        // Trim old violations if limit exceeded
        if violations.len() > self.max_violations {
            let excess = violations.len() - self.max_violations;
            violations.drain(0..excess);
        }

        Ok(())
    }

    /// Get violations for a user
    pub fn get_user_violations(&self, user_id: &str) -> Result<Vec<PolicyViolation>> {
        let violations = self.violations.read()
            .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;

        Ok(violations
            .iter()
            .filter(|v| v.user_id == user_id)
            .cloned()
            .collect())
    }

    /// Get all violations
    pub fn get_all_violations(&self) -> Result<Vec<PolicyViolation>> {
        let violations = self.violations.read()
            .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;

        Ok(violations.clone())
    }

    /// Clear violations older than specified duration
    pub fn clear_old_violations(&self, max_age: Duration) -> Result<usize> {
        let mut violations = self.violations.write()
            .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;

        let cutoff = SystemTime::now() - max_age;
        let original_count = violations.len();

        violations.retain(|v| v.timestamp > cutoff);

        Ok(original_count - violations.len())
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_creation() {
        let limits = PolicyLimits {
            max_cpu_time_ms: Some(1000),
            max_memory_bytes: Some(1024 * 1024),
            ..Default::default()
        };

        let policy = GovernancePolicy::new(
            "pol_1".to_string(),
            "Test Policy".to_string(),
            "A test policy".to_string(),
            PolicyScope::User,
            "user1".to_string(),
            limits,
        );

        assert_eq!(policy.policy_id, "pol_1");
        assert_eq!(policy.scope, PolicyScope::User);
        assert!(policy.enabled);
    }

    #[test]
    fn test_policy_engine_add_remove() {
        let engine = PolicyEngine::new();

        let policy = GovernancePolicy::new(
            "pol_1".to_string(),
            "Test Policy".to_string(),
            "Test".to_string(),
            PolicyScope::Global,
            "global".to_string(),
            PolicyLimits::default(),
        );

        engine.add_policy(policy).unwrap();
        engine.remove_policy("pol_1").unwrap();
    }
}
