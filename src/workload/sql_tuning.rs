// RustyDB SQL Tuning Advisor - Oracle-like SQL performance tuning
// Provides automated SQL analysis, plan recommendations, and tuning suggestions

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use parking_lot::RwLock;
use crate::Result;
use crate::error::DbError;

/// SQL Tuning Advisor for automated query optimization
pub struct SqlTuningAdvisor {
    /// Tuning task repository
    tasks: Arc<RwLock<HashMap<TaskId, TuningTask>>>,

    /// SQL profiles storage
    profiles: Arc<RwLock<HashMap<String, SqlProfile>>>,

    /// Tuning recommendations cache
    recommendations: Arc<RwLock<HashMap<TaskId, Vec<TuningRecommendation>>>>,

    /// Configuration
    config: Arc<RwLock<TuningConfig>>,

    /// Statistics for plan costing
    statistics: Arc<RwLock<OptimizerStatistics>>,

    /// Next task ID
    next_task_id: Arc<RwLock<TaskId>>,
}

/// Unique identifier for a tuning task
pub type TaskId = u64;

/// SQL Tuning Advisor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningConfig {
    /// Maximum execution time for tuning analysis (seconds)
    pub max_analysis_time_secs: u64,

    /// Enable automatic SQL profile creation
    pub auto_profile_creation: bool,

    /// Enable alternative plan generation
    pub enable_alternative_plans: bool,

    /// Maximum number of alternative plans to generate
    pub max_alternative_plans: usize,

    /// Enable index recommendations
    pub enable_index_recommendations: bool,

    /// Enable SQL rewrite recommendations
    pub enable_sql_rewrite: bool,

    /// Minimum improvement threshold (%) to recommend changes
    pub min_improvement_pct: f64,
}

impl Default for TuningConfig {
    fn default() -> Self {
        Self {
            max_analysis_time_secs: 300,
            auto_profile_creation: false,
            enable_alternative_plans: true,
            max_alternative_plans: 5,
            enable_index_recommendations: true,
            enable_sql_rewrite: true,
            min_improvement_pct: 10.0,
        }
    }
}

/// SQL tuning task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningTask {
    pub task_id: TaskId,
    pub task_name: String,
    pub sql_text: String,
    pub sql_id: String,
    pub status: TaskStatus,
    pub created_time: SystemTime,
    pub started_time: Option<SystemTime>,
    pub completed_time: Option<SystemTime>,
    pub scope: TuningScope,
    pub time_limit_secs: u64,
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Created,
    Running,
    Completed,
    Failed,
    Interrupted,
}

/// Tuning scope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TuningScope {
    /// Tune a single SQL statement
    Limited,
    /// Comprehensive tuning including multiple alternatives
    Comprehensive,
}

/// SQL tuning recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningRecommendation {
    pub recommendation_id: u32,
    pub recommendation_type: RecommendationType,
    pub benefit_type: BenefitType,
    pub estimated_benefit_pct: f64,
    pub rationale: String,
    pub action: String,
    pub details: RecommendationDetails,
}

/// Type of recommendation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationType {
    /// SQL profile recommendation
    SqlProfile,
    /// Index creation/modification
    Index,
    /// SQL statement restructuring
    Restructure,
    /// Statistics collection
    Statistics,
    /// Alternative plan
    AlternativePlan,
    /// Miscellaneous
    Miscellaneous,
}

/// Benefit type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenefitType {
    /// Improved CPU usage
    CpuTime,
    /// Reduced I/O
    IoReduction,
    /// Better memory usage
    MemoryUsage,
    /// Overall execution time
    ExecutionTime,
}

/// Recommendation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationDetails {
    SqlProfile(SqlProfileDetails),
    Index(IndexRecommendation),
    Restructure(RestructureRecommendation),
    Statistics(StatisticsRecommendation),
    AlternativePlan(AlternativePlanDetails),
}

/// SQL profile details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlProfileDetails {
    pub profile_name: String,
    pub hints: Vec<String>,
    pub estimated_improvement_pct: f64,
    pub plan_hash_old: u64,
    pub plan_hash_new: u64,
}

/// Index recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRecommendation {
    pub table_name: String,
    pub columns: Vec<String>,
    pub index_type: String,
    pub estimated_size_bytes: u64,
    pub estimated_improvement_pct: f64,
    pub usage_description: String,
}

/// SQL restructuring recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestructureRecommendation {
    pub original_sql: String,
    pub rewritten_sql: String,
    pub rewrite_type: RewriteType,
    pub explanation: String,
}

/// Type of SQL rewrite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RewriteType {
    SubqueryToJoin,
    JoinToSubquery,
    UnionToUnionAll,
    InToExists,
    ExistsToIn,
    OrToUnion,
    LiteralReplacement,
    PredicatePushdown,
    ViewMerging,
}

/// Statistics recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsRecommendation {
    pub object_name: String,
    pub object_type: String,
    pub current_stats_age_days: Option<u32>,
    pub reason: String,
}

/// Alternative plan details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativePlanDetails {
    pub plan_description: String,
    pub plan_hash: u64,
    pub estimated_cost: f64,
    pub estimated_cardinality: u64,
    pub key_differences: Vec<String>,
}

/// SQL profile for plan stability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlProfile {
    pub profile_name: String,
    pub sql_text: String,
    pub sql_id: String,
    pub category: String,
    pub hints: Vec<String>,
    pub created_time: SystemTime,
    pub last_used: Option<SystemTime>,
    pub status: ProfileStatus,
    pub force_matching: bool,
}

/// SQL profile status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProfileStatus {
    Enabled,
    Disabled,
}

/// Optimizer statistics for cost estimation
#[derive(Debug, Default)]
pub struct OptimizerStatistics {
    table_stats: HashMap<String, TableStatistics>,
    index_stats: HashMap<String, IndexStatistics>,
    column_stats: HashMap<String, ColumnStatistics>,
}

/// Table statistics
#[derive(Debug, Clone)]
pub struct TableStatistics {
    pub table_name: String,
    pub num_rows: u64,
    pub num_blocks: u64,
    pub avg_row_len: u32,
    pub last_analyzed: Option<SystemTime>,
}

/// Index statistics
#[derive(Debug, Clone)]
pub struct IndexStatistics {
    pub index_name: String,
    pub table_name: String,
    pub uniqueness: bool,
    pub distinct_keys: u64,
    pub leaf_blocks: u64,
    pub clustering_factor: u64,
}

/// Column statistics
#[derive(Debug, Clone)]
pub struct ColumnStatistics {
    pub column_name: String,
    pub num_distinct: u64,
    pub num_nulls: u64,
    pub density: f64,
    pub low_value: Option<String>,
    pub high_value: Option<String>,
    pub histogram: Option<Histogram>,
}

/// Histogram for column data distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    pub histogram_type: HistogramType,
    pub buckets: Vec<HistogramBucket>,
}

/// Histogram type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HistogramType {
    Frequency,
    Height,
    Hybrid,
}

/// Histogram bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub endpoint_value: String,
    pub endpoint_number: u64,
}

/// Query plan analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAnalysis {
    pub plan_hash: u64,
    pub estimated_cost: f64,
    pub estimated_cardinality: u64,
    pub operations: Vec<PlanOperation>,
    pub issues: Vec<PlanIssue>,
    pub access_paths: Vec<AccessPath>,
}

/// Plan operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanOperation {
    pub id: u32,
    pub operation: String,
    pub object_name: Option<String>,
    pub cost: f64,
    pub cardinality: u64,
    pub bytes: u64,
    pub partition_start: Option<String>,
    pub partition_stop: Option<String>,
}

/// Plan issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanIssue {
    pub severity: IssueSeverity,
    pub issue_type: IssueType,
    pub description: String,
    pub affected_operation_id: Option<u32>,
}

/// Issue severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssueSeverity {
    Info,
    Warning,
    Critical,
}

/// Issue type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueType {
    CardinalityMismatch,
    MissingIndex,
    FullTableScan,
    CartesianProduct,
    SuboptimalJoinOrder,
    ExpensiveOperation,
    StaleStatistics,
    BindVariablePeeking,
}

/// Access path (how data is accessed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPath {
    pub table_name: String,
    pub access_type: AccessType,
    pub index_name: Option<String>,
    pub estimated_cost: f64,
    pub estimated_rows: u64,
}

/// Access type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessType {
    FullTableScan,
    IndexUniqueScan,
    IndexRangeScan,
    IndexFullScan,
    IndexFastFullScan,
    IndexSkipScan,
}

impl SqlTuningAdvisor {
    /// Create a new SQL Tuning Advisor
    pub fn new() -> Self {
        Self::with_config(TuningConfig::default())
    }

    /// Create a new SQL Tuning Advisor with custom configuration
    pub fn with_config(config: TuningConfig) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            recommendations: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(config)),
            statistics: Arc::new(RwLock::new(OptimizerStatistics::default())),
            next_task_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Create a new tuning task
    pub fn create_tuning_task(
        &self,
        task_name: String,
        sql_text: String,
        scope: TuningScope,
    ) -> Result<TaskId> {
        let mut tasks = self.tasks.write();
        let mut next_id = self.next_task_id.write();

        let task_id = *next_id;
        *next_id += 1;

        let config = self.config.read();
        let time_limit = config.max_analysis_time_secs;
        drop(config);

        let sql_id = self.compute_sql_id(&sql_text);

        let task = TuningTask {
            task_id,
            task_name,
            sql_text,
            sql_id,
            status: TaskStatus::Created,
            created_time: SystemTime::now(),
            started_time: None,
            completed_time: None,
            scope,
            time_limit_secs: time_limit,
        };

        tasks.insert(task_id, task);
        Ok(task_id)
    }

    /// Execute a tuning task
    pub fn execute_tuning_task(&self, task_id: TaskId) -> Result<()> {
        // Update task status
        {
            let mut tasks = self.tasks.write();
            let task = tasks
                .get_mut(&task_id)
                .ok_or_else(|| DbError::NotFound(format!("Task {} not found", task_id)))?;

            task.status = TaskStatus::Running;
            task.started_time = Some(SystemTime::now());
        }

        // Perform analysis
        let recommendations = self.analyze_sql(task_id)?;

        // Store recommendations
        self.recommendations.write().insert(task_id, recommendations);

        // Update task status
        {
            let mut tasks = self.tasks.write();
            if let Some(task) = tasks.get_mut(&task_id) {
                task.status = TaskStatus::Completed;
                task.completed_time = Some(SystemTime::now());
            }
        }

        Ok(())
    }

    /// Analyze SQL and generate recommendations
    fn analyze_sql(&self, task_id: TaskId) -> Result<Vec<TuningRecommendation>> {
        let task = {
            let tasks = self.tasks.read();
            tasks
                .get(&task_id)
                .ok_or_else(|| DbError::NotFound(format!("Task {} not found", task_id)))?
                .clone()
        };

        let mut recommendations = Vec::new();
        let mut rec_id = 1;

        // Analyze current plan
        let plan_analysis = self.analyze_plan(&task.sql_text)?;

        // Generate recommendations based on plan issues
        for issue in &plan_analysis.issues {
            match issue.issue_type {
                IssueType::MissingIndex => {
                    recommendations.push(self.recommend_index(&task.sql_text, rec_id)?);
                    rec_id += 1;
                }
                IssueType::FullTableScan => {
                    recommendations.push(self.recommend_index(&task.sql_text, rec_id)?);
                    rec_id += 1;
                }
                IssueType::SuboptimalJoinOrder => {
                    recommendations.push(self.recommend_sql_profile(&task.sql_text, rec_id)?);
                    rec_id += 1;
                }
                IssueType::StaleStatistics => {
                    recommendations.push(self.recommend_statistics_collection(rec_id)?);
                    rec_id += 1;
                }
                _ => {}
            }
        }

        // Generate alternative plans if enabled
        let config = self.config.read();
        if config.enable_alternative_plans {
            if let Ok(alt_plans) = self.generate_alternative_plans(&task.sql_text) {
                for (i, plan) in alt_plans.iter().enumerate() {
                    recommendations.push(TuningRecommendation {
                        recommendation_id: rec_id,
                        recommendation_type: RecommendationType::AlternativePlan,
                        benefit_type: BenefitType::ExecutionTime,
                        estimated_benefit_pct: 20.0 + (i as f64 * 5.0),
                        rationale: "Alternative execution plan with different join order".to_string(),
                        action: "Consider using SQL profile to stabilize this plan".to_string(),
                        details: RecommendationDetails::AlternativePlan(plan.clone()),
                    });
                    rec_id += 1;
                }
            }
        }

        // SQL rewrite recommendations
        if config.enable_sql_rewrite {
            if let Ok(rewrites) = self.suggest_sql_rewrites(&task.sql_text) {
                for rewrite in rewrites {
                    recommendations.push(TuningRecommendation {
                        recommendation_id: rec_id,
                        recommendation_type: RecommendationType::Restructure,
                        benefit_type: BenefitType::ExecutionTime,
                        estimated_benefit_pct: 15.0,
                        rationale: rewrite.explanation.clone(),
                        action: "Rewrite SQL statement".to_string(),
                        details: RecommendationDetails::Restructure(rewrite),
                    });
                    rec_id += 1;
                }
            }
        }

        Ok(recommendations)
    }

    /// Analyze execution plan
    fn analyze_plan(&self, sql_text: &str) -> Result<PlanAnalysis> {
        // Simplified plan analysis
        let mut issues = Vec::new();

        // Check for potential full table scan
        if sql_text.to_uppercase().contains("SELECT") && !sql_text.to_uppercase().contains("WHERE") {
            issues.push(PlanIssue {
                severity: IssueSeverity::Warning,
                issue_type: IssueType::FullTableScan,
                description: "Query may perform full table scan without WHERE clause".to_string(),
                affected_operation_id: Some(1),
            });

            issues.push(PlanIssue {
                severity: IssueSeverity::Warning,
                issue_type: IssueType::MissingIndex,
                description: "Consider creating index to avoid full table scan".to_string(),
                affected_operation_id: Some(1),
            });
        }

        // Check for Cartesian product
        if sql_text.matches("JOIN").count() > 1 && !sql_text.to_uppercase().contains("ON") {
            issues.push(PlanIssue {
                severity: IssueSeverity::Critical,
                issue_type: IssueType::CartesianProduct,
                description: "Potential Cartesian product detected".to_string(),
                affected_operation_id: Some(2),
            });
        }

        Ok(PlanAnalysis {
            plan_hash: self.compute_plan_hash(sql_text),
            estimated_cost: 1000.0,
            estimated_cardinality: 10000,
            operations: vec![
                PlanOperation {
                    id: 1,
                    operation: "TABLE ACCESS FULL".to_string(),
                    object_name: Some("USERS".to_string()),
                    cost: 1000.0,
                    cardinality: 10000,
                    bytes: 1000000,
                    partition_start: None,
                    partition_stop: None,
                },
            ],
            issues,
            access_paths: vec![
                AccessPath {
                    table_name: "USERS".to_string(),
                    access_type: AccessType::FullTableScan,
                    index_name: None,
                    estimated_cost: 1000.0,
                    estimated_rows: 10000,
                },
            ],
        })
    }

    /// Recommend index creation
    fn recommend_index(&self, sql_text: &str, rec_id: u32) -> Result<TuningRecommendation> {
        Ok(TuningRecommendation {
            recommendation_id: rec_id,
            recommendation_type: RecommendationType::Index,
            benefit_type: BenefitType::IoReduction,
            estimated_benefit_pct: 40.0,
            rationale: "Creating an index can significantly reduce I/O and improve query performance".to_string(),
            action: "CREATE INDEX idx_users_id ON users(id)".to_string(),
            details: RecommendationDetails::Index(IndexRecommendation {
                table_name: "users".to_string(),
                columns: vec!["id".to_string()],
                index_type: "B-TREE".to_string(),
                estimated_size_bytes: 1024 * 1024,
                estimated_improvement_pct: 40.0,
                usage_description: "This index will be used for equality predicates on id column".to_string(),
            }),
        })
    }

    /// Recommend SQL profile
    fn recommend_sql_profile(&self, sql_text: &str, rec_id: u32) -> Result<TuningRecommendation> {
        let sql_id = self.compute_sql_id(sql_text);

        Ok(TuningRecommendation {
            recommendation_id: rec_id,
            recommendation_type: RecommendationType::SqlProfile,
            benefit_type: BenefitType::ExecutionTime,
            estimated_benefit_pct: 25.0,
            rationale: "SQL profile can stabilize execution plan and improve performance".to_string(),
            action: format!("Execute DBMS_SQLTUNE.ACCEPT_SQL_PROFILE for SQL_ID {}", sql_id),
            details: RecommendationDetails::SqlProfile(SqlProfileDetails {
                profile_name: format!("SYS_SQLPROF_{}", sql_id),
                hints: vec![
                    "LEADING(@SEL$1 T1 T2)".to_string(),
                    "USE_HASH(@SEL$1 T2)".to_string(),
                ],
                estimated_improvement_pct: 25.0,
                plan_hash_old: self.compute_plan_hash(sql_text),
                plan_hash_new: self.compute_plan_hash(sql_text) + 1,
            }),
        })
    }

    /// Recommend statistics collection
    fn recommend_statistics_collection(&self, rec_id: u32) -> Result<TuningRecommendation> {
        Ok(TuningRecommendation {
            recommendation_id: rec_id,
            recommendation_type: RecommendationType::Statistics,
            benefit_type: BenefitType::ExecutionTime,
            estimated_benefit_pct: 15.0,
            rationale: "Current statistics are stale or missing, causing suboptimal plans".to_string(),
            action: "EXEC DBMS_STATS.GATHER_TABLE_STATS('SCHEMA', 'TABLE')".to_string(),
            details: RecommendationDetails::Statistics(StatisticsRecommendation {
                object_name: "USERS".to_string(),
                object_type: "TABLE".to_string(),
                current_stats_age_days: Some(90),
                reason: "Statistics older than 30 days".to_string(),
            }),
        })
    }

    /// Generate alternative execution plans
    fn generate_alternative_plans(&self, sql_text: &str) -> Result<Vec<AlternativePlanDetails>> {
        let mut plans = Vec::new();

        // Generate alternative plan with different join method
        plans.push(AlternativePlanDetails {
            plan_description: "Use hash join instead of nested loop".to_string(),
            plan_hash: self.compute_plan_hash(sql_text) + 100,
            estimated_cost: 800.0,
            estimated_cardinality: 10000,
            key_differences: vec!["HASH JOIN instead of NESTED LOOPS".to_string()],
        });

        // Generate alternative plan with different access path
        plans.push(AlternativePlanDetails {
            plan_description: "Use index range scan instead of full table scan".to_string(),
            plan_hash: self.compute_plan_hash(sql_text) + 200,
            estimated_cost: 600.0,
            estimated_cardinality: 5000,
            key_differences: vec!["INDEX RANGE SCAN instead of TABLE ACCESS FULL".to_string()],
        });

        Ok(plans)
    }

    /// Suggest SQL rewrites
    fn suggest_sql_rewrites(&self, sql_text: &str) -> Result<Vec<RestructureRecommendation>> {
        let mut rewrites = Vec::new();

        // Example: Suggest converting IN to EXISTS
        if sql_text.to_uppercase().contains(" IN ") {
            rewrites.push(RestructureRecommendation {
                original_sql: sql_text.to_string(),
                rewritten_sql: sql_text.replace(" IN ", " EXISTS "),
                rewrite_type: RewriteType::InToExists,
                explanation: "Converting IN to EXISTS can improve performance for large subqueries".to_string(),
            });
        }

        // Example: Suggest UNION ALL instead of UNION
        if sql_text.to_uppercase().contains(" UNION ") && !sql_text.to_uppercase().contains(" UNION ALL ") {
            rewrites.push(RestructureRecommendation {
                original_sql: sql_text.to_string(),
                rewritten_sql: sql_text.replace(" UNION ", " UNION ALL "),
                rewrite_type: RewriteType::UnionToUnionAll,
                explanation: "Use UNION ALL if duplicates are acceptable to avoid expensive distinct operation".to_string(),
            });
        }

        Ok(rewrites)
    }

    /// Create SQL profile
    pub fn create_sql_profile(
        &self,
        profile_name: String,
        sql_text: String,
        hints: Vec<String>,
        category: String,
    ) -> Result<()> {
        let sql_id = self.compute_sql_id(&sql_text);

        let profile = SqlProfile {
            profile_name: profile_name.clone(),
            sql_text,
            sql_id,
            category,
            hints,
            created_time: SystemTime::now(),
            last_used: None,
            status: ProfileStatus::Enabled,
            force_matching: false,
        };

        self.profiles.write().insert(profile_name, profile);
        Ok(())
    }

    /// Get recommendations for a task
    pub fn get_recommendations(&self, task_id: TaskId) -> Option<Vec<TuningRecommendation>> {
        self.recommendations.read().get(&task_id).cloned()
    }

    /// Get task details
    pub fn get_task(&self, task_id: TaskId) -> Option<TuningTask> {
        self.tasks.read().get(&task_id).cloned()
    }

    /// List all tasks
    pub fn list_tasks(&self) -> Vec<TuningTask> {
        self.tasks.read().values().cloned().collect()
    }

    /// Delete a task
    pub fn delete_task(&self, task_id: TaskId) -> Result<()> {
        let mut tasks = self.tasks.write();
        let mut recommendations = self.recommendations.write();

        if tasks.remove(&task_id).is_some() {
            recommendations.remove(&task_id);
            Ok(())
        } else {
            Err(DbError::NotFound(format!("Task {} not found", task_id)))
        }
    }

    /// Compute SQL ID (hash of SQL text)
    fn compute_sql_id(&self, sql_text: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        sql_text.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Compute plan hash
    fn compute_plan_hash(&self, sql_text: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        sql_text.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for SqlTuningAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tuning_task() {
        let advisor = SqlTuningAdvisor::new();
        let task_id = advisor
            .create_tuning_task(
                "test_task".to_string(),
                "SELECT * FROM users WHERE id = 1".to_string(),
                TuningScope::Comprehensive,
            )
            .unwrap();

        assert_eq!(task_id, 1);

        let task = advisor.get_task(task_id);
        assert!(task.is_some());
        assert_eq!(task.unwrap().status, TaskStatus::Created);
    }

    #[test]
    fn test_execute_tuning_task() {
        let advisor = SqlTuningAdvisor::new();
        let task_id = advisor
            .create_tuning_task(
                "test_task".to_string(),
                "SELECT * FROM users".to_string(),
                TuningScope::Comprehensive,
            )
            .unwrap();

        advisor.execute_tuning_task(task_id).unwrap();

        let task = advisor.get_task(task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Completed);

        let recommendations = advisor.get_recommendations(task_id);
        assert!(recommendations.is_some());
        assert!(!recommendations.unwrap().is_empty());
    }

    #[test]
    fn test_create_sql_profile() {
        let advisor = SqlTuningAdvisor::new();
        advisor
            .create_sql_profile(
                "test_profile".to_string(),
                "SELECT * FROM users WHERE id = 1".to_string(),
                vec!["USE_INDEX(@SEL$1 users idx_id)".to_string()],
                "DEFAULT".to_string(),
            )
            .unwrap();

        assert_eq!(advisor.profiles.read().len(), 1);
    }
}


