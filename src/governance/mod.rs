// # Query Governance Module
//
// This module provides comprehensive query governance capabilities including:
// - Resource limits (CPU, memory, execution time)
// - Query complexity analysis
// - Query throttling and queuing
// - Query cancellation
// - Per-user, per-session, and per-query quotas
// - Governance policies with inheritance
// - Violation tracking and handling
//
// ## Architecture
//
// The governance system consists of three main components:
//
// 1. **Query Governor** (`query_governor`): Core engine that coordinates all governance
//    activities, analyzes query complexity, manages query queues, and enforces limits.
//
// 2. **Resource Quotas** (`resource_quotas`): Manages resource quotas at different
//    levels (user, session, query) and tracks resource usage.
//
// 3. **Governance Policies** (`governance_policies`): Defines, stores, and evaluates
//    governance policies with support for inheritance (user -> role -> global).
//
// ## Usage Example
//
// ```rust
// use rusty_db::governance::*;
// use rusty_db::Result;
// use std::sync::Arc;
//
// # fn example() -> Result<()> {
// // Create governance components
// let quota_manager = Arc::new(QuotaManager::new());
// let policy_engine = Arc::new(PolicyEngine::new());
// let governor = QueryGovernor::new(quota_manager.clone(), policy_engine.clone());
//
// // Define a policy
// let policy = GovernancePolicy::new(
//     "pol_1".to_string(),
//     "User Policy".to_string(),
//     "Limits for regular users".to_string(),
//     PolicyScope::User,
//     "user1".to_string(),
//     PolicyLimits {
//         max_cpu_time_ms: Some(5000),
//         max_memory_bytes: Some(1024 * 1024 * 100),
//         max_execution_time_ms: Some(30000),
//         max_result_rows: Some(10000),
//         max_concurrent_queries: Some(5),
//         max_complexity_score: Some(1000),
//     },
// );
//
// policy_engine.add_policy(policy)?;
//
// // Submit a query
// let query_id = governor.submit_query(
//     "user1".to_string(),
//     1, // session_id
//     "SELECT * FROM large_table".to_string(),
//     vec!["user_role".to_string()],
// )?;
//
// // Complete the query
// governor.complete_query(&query_id, 1000, 1024 * 1024, 5000)?;
// # Ok(())
// # }
// ```
//
// ## Features
//
// ### Query Complexity Analysis
//
// The governor analyzes queries to estimate their complexity and resource requirements:
// - Counts SQL operations (JOINs, subqueries, aggregations)
// - Detects expensive patterns (Cartesian products, wildcards)
// - Estimates CPU and memory usage
//
// ### Resource Quotas
//
// Multiple levels of quotas:
// - **User Quotas**: Per-user limits with time windows
// - **Session Quotas**: Per-session limits
// - **Query Quotas**: Per-query limits
//
// ### Policy Inheritance
//
// Policies are evaluated with inheritance:
// 1. Global policies (lowest priority)
// 2. Role-based policies (medium priority)
// 3. User-specific policies (highest priority)
//
// More restrictive limits always take precedence.
//
// ### Query Throttling
//
// Queries can be queued when resource limits are reached:
// - Priority-based queue
// - Automatic execution when resources become available
// - Configurable queue size limits
//
// ### Query Cancellation
//
// Queries can be cancelled at any time:
// - Cancel active queries
// - Cancel queued queries
// - Cleanup resources properly

use crate::error::Result;

// Re-export main types and structures
pub mod query_governor;
pub mod resource_quotas;
pub mod governance_policies;

// Re-export commonly used types
pub use query_governor::{
    QueryGovernor,
    QueryId,
    QueryState,
    QueryMetadata,
    ComplexityAnalyzer,
    GovernorStatistics,
};

pub use resource_quotas::{
    QuotaManager,
    ResourceUsage,
    UserQuota,
    SessionQuota,
    QueryQuota,
};

pub use governance_policies::{
    PolicyEngine,
    GovernancePolicy,
    PolicyScope,
    PolicyPriority,
    PolicyLimits,
    PolicyViolation,
    ViolationType,
};

// ============================================================================
// Module-level Functions
// ============================================================================

/// Create a new governance system with default configuration
pub fn create_governance_system() -> (QueryGovernor, QuotaManager, PolicyEngine) {
    use std::sync::Arc;

    let quota_manager = QuotaManager::new();
    let policy_engine = PolicyEngine::new();

    let governor = QueryGovernor::new(
        Arc::new(quota_manager.clone()),
        Arc::new(policy_engine.clone()),
    );

    (governor, quota_manager, policy_engine)
}

/// Create default policy limits for different user tiers
pub fn default_policy_limits(tier: UserTier) -> PolicyLimits {
    match tier {
        UserTier::Free => PolicyLimits {
            max_cpu_time_ms: Some(1000),
            max_memory_bytes: Some(100 * 1024 * 1024), // 100 MB
            max_execution_time_ms: Some(10000), // 10 seconds
            max_result_rows: Some(1000),
            max_concurrent_queries: Some(2),
            max_complexity_score: Some(500),
        },
        UserTier::Standard => PolicyLimits {
            max_cpu_time_ms: Some(10000),
            max_memory_bytes: Some(1024 * 1024 * 1024), // 1 GB
            max_execution_time_ms: Some(60000), // 60 seconds
            max_result_rows: Some(100000),
            max_concurrent_queries: Some(10),
            max_complexity_score: Some(5000),
        },
        UserTier::Premium => PolicyLimits {
            max_cpu_time_ms: Some(60000),
            max_memory_bytes: Some(10 * 1024 * 1024 * 1024), // 10 GB
            max_execution_time_ms: Some(300000), // 5 minutes
            max_result_rows: Some(1000000),
            max_concurrent_queries: Some(50),
            max_complexity_score: Some(50000),
        },
        UserTier::Enterprise => PolicyLimits {
            max_cpu_time_ms: None, // Unlimited
            max_memory_bytes: None, // Unlimited
            max_execution_time_ms: None, // Unlimited
            max_result_rows: None, // Unlimited
            max_concurrent_queries: None, // Unlimited
            max_complexity_score: None, // Unlimited
        },
    }
}

/// User tier for default policy creation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserTier {
    Free,
    Standard,
    Premium,
    Enterprise,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_create_governance_system() {
        let (governor, _quota_manager, _policy_engine) = create_governance_system();

        let stats = governor.get_statistics().unwrap();
        assert_eq!(stats.active_queries, 0);
        assert_eq!(stats.queued_queries, 0);
    }

    #[test]
    fn test_default_policy_limits() {
        let free_limits = default_policy_limits(UserTier::Free);
        let enterprise_limits = default_policy_limits(UserTier::Enterprise);

        assert!(free_limits.max_cpu_time_ms.is_some());
        assert!(enterprise_limits.max_cpu_time_ms.is_none());
    }

    #[test]
    fn test_end_to_end_governance() {
        let quota_manager = Arc::new(QuotaManager::new());
        let policy_engine = Arc::new(PolicyEngine::new());
        let governor = QueryGovernor::new(quota_manager.clone(), policy_engine.clone());

        // Create a session
        quota_manager.create_session_quota(1, "user1".to_string()).unwrap();

        // Submit a query
        let query_id = governor.submit_query(
            "user1".to_string(),
            1,
            "SELECT * FROM test".to_string(),
            vec![],
        ).unwrap();

        // Check status
        let status = governor.get_query_status(&query_id).unwrap();
        assert!(status.state == QueryState::Running || status.state == QueryState::Queued);

        // Complete query
        if status.state == QueryState::Running {
            governor.complete_query(&query_id, 100, 1024, 10).unwrap();
        }
    }
}
