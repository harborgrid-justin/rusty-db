//! SQL Plan Management - Plan baselines and stability
//!
//! Implements Oracle-like SQL Plan Management with:
//! - SQL Plan Baselines
//! - Plan capture and evolution
//! - Plan stability guarantees
//! - Plan history and comparison
//! - Automatic plan regression detection

use crate::error::{DbError, Result};
use crate::optimizer_pro::{PhysicalPlan, PlanId, QueryFingerprint};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

// ============================================================================
// Plan Baseline Manager
// ============================================================================

/// SQL Plan Baseline Manager
pub struct PlanBaselineManager {
    /// Active baselines
    baselines: Arc<RwLock<HashMap<QueryFingerprint, SqlPlanBaseline>>>,
    /// Plan history
    plan_history: Arc<RwLock<HashMap<QueryFingerprint, PlanHistory>>>,
    /// Evolution settings
    evolution_config: Arc<RwLock<EvolutionConfig>>,
    /// Baseline capture settings
    capture_config: Arc<RwLock<CaptureConfig>>,
    /// Regression detector
    regression_detector: Arc<RegressionDetector>,
}

impl PlanBaselineManager {
    /// Create a new plan baseline manager
    pub fn new() -> Self {
        Self {
            baselines: Arc::new(RwLock::new(HashMap::new())),
            plan_history: Arc::new(RwLock::new(HashMap::new())),
            evolution_config: Arc::new(RwLock::new(EvolutionConfig::default())),
            capture_config: Arc::new(RwLock::new(CaptureConfig::default())),
            regression_detector: Arc::new(RegressionDetector::new()),
        }
    }

    /// Get baseline for a query
    pub fn get_baseline(&self, fingerprint: &QueryFingerprint) -> Result<Option<SqlPlanBaseline>> {
        let baselines = self.baselines.read().unwrap();

        if let Some(baseline) = baselines.get(fingerprint) {
            // Check if baseline is enabled
            if baseline.enabled {
                return Ok(Some(baseline.clone()));
            }
        }

        Ok(None)
    }

    /// Capture a plan baseline
    pub fn capture_baseline(
        &self,
        fingerprint: QueryFingerprint,
        plan: PhysicalPlan,
    ) -> Result<()> {
        let capture_config = self.capture_config.read().unwrap();

        // Check if automatic capture is enabled
        if !capture_config.auto_capture {
            return Ok(());
        }

        let mut baselines = self.baselines.write().unwrap();
        let mut plan_history = self.plan_history.write().unwrap();

        // Add to history
        let history = plan_history
            .entry(fingerprint.clone())
            .or_insert_with(|| PlanHistory::new(fingerprint.clone()));

        history.add_plan(plan.clone());

        // Create or update baseline
        if let Some(baseline) = baselines.get_mut(&fingerprint) {
            // Add plan to baseline if it's better
            if self.should_add_to_baseline(&plan, baseline)? {
                baseline.add_accepted_plan(plan);
                baseline.last_modified = SystemTime::now();
            }
        } else {
            // Create new baseline
            let baseline = SqlPlanBaseline::new(fingerprint.clone(), plan);
            baselines.insert(fingerprint, baseline);
        }

        Ok(())
    }

    /// Evolve plan baselines
    pub fn evolve_baselines(&self) -> Result<usize> {
        let evolution_config = self.evolution_config.read().unwrap();

        if !evolution_config.auto_evolution {
            return Ok(0);
        }

        let mut evolved_count = 0;
        let mut baselines = self.baselines.write().unwrap();
        let plan_history = self.plan_history.read().unwrap();

        for (fingerprint, baseline) in baselines.iter_mut() {
            if let Some(history) = plan_history.get(fingerprint) {
                // Find candidate plans for evolution
                let candidates = history.get_evolution_candidates(
                    &baseline.accepted_plans,
                    evolution_config.min_executions,
                    evolution_config.performance_threshold,
                );

                for candidate in candidates {
                    // Check for regression
                    if !self.regression_detector.is_regression(&candidate, baseline)? {
                        baseline.add_accepted_plan(candidate);
                        evolved_count += 1;
                    }
                }

                baseline.last_evolved = Some(SystemTime::now());
            }
        }

        Ok(evolved_count)
    }

    /// Load baseline from disk (simplified)
    pub fn load_baseline(&self, _fingerprint: &QueryFingerprint) -> Result<Option<SqlPlanBaseline>> {
        // In production, this would load from persistent storage
        Ok(None)
    }

    /// Save baseline to disk (simplified)
    pub fn save_baseline(&self, _baseline: &SqlPlanBaseline) -> Result<()> {
        // In production, this would persist to disk
        Ok(())
    }

    /// Delete baseline
    pub fn delete_baseline(&self, fingerprint: &QueryFingerprint) -> Result<()> {
        self.baselines.write().unwrap().remove(fingerprint);
        Ok(())
    }

    /// Enable baseline
    pub fn enable_baseline(&self, fingerprint: &QueryFingerprint) -> Result<()> {
        if let Some(baseline) = self.baselines.write().unwrap().get_mut(fingerprint) {
            baseline.enabled = true;
        }
        Ok(())
    }

    /// Disable baseline
    pub fn disable_baseline(&self, fingerprint: &QueryFingerprint) -> Result<()> {
        if let Some(baseline) = self.baselines.write().unwrap().get_mut(fingerprint) {
            baseline.enabled = false;
        }
        Ok(())
    }

    /// Get all baselines
    pub fn get_all_baselines(&self) -> Vec<SqlPlanBaseline> {
        self.baselines.read().unwrap().values().cloned().collect()
    }

    /// Get plan history for a query
    pub fn get_plan_history(&self, fingerprint: &QueryFingerprint) -> Option<PlanHistory> {
        self.plan_history.read().unwrap().get(fingerprint).cloned()
    }

    /// Compare two plans
    pub fn compare_plans(&self, plan1: &PhysicalPlan, plan2: &PhysicalPlan) -> PlanComparison {
        PlanComparison {
            cost_diff: plan1.cost - plan2.cost,
            cardinality_diff: plan1.cardinality as i64 - plan2.cardinality as i64,
            operator_diff: self.compare_operators(&plan1.operator, &plan2.operator),
            better_plan: if plan1.cost < plan2.cost { 1 } else { 2 },
        }
    }

    /// Check if plan should be added to baseline
    fn should_add_to_baseline(&self, plan: &PhysicalPlan, baseline: &SqlPlanBaseline) -> Result<bool> {
        // Don't add if plan already exists
        if baseline.accepted_plans.iter().any(|p| p.plan_id == plan.plan_id) {
            return Ok(false);
        }

        // Add if plan is significantly better (cost-wise)
        if let Some(best_plan) = baseline.accepted_plans.first() {
            if plan.cost < best_plan.cost * 0.9 {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Compare two operators
    fn compare_operators(
        &self,
        op1: &crate::optimizer_pro::PhysicalOperator,
        op2: &crate::optimizer_pro::PhysicalOperator,
    ) -> String {
        format!("{:?} vs {:?}", op1, op2)
    }

    /// Get baseline statistics
    pub fn get_statistics(&self) -> BaselineStatistics {
        let baselines = self.baselines.read().unwrap();
        let plan_history = self.plan_history.read().unwrap();

        BaselineStatistics {
            total_baselines: baselines.len(),
            enabled_baselines: baselines.values().filter(|b| b.enabled).count(),
            total_accepted_plans: baselines.values().map(|b| b.accepted_plans.len()).sum(),
            total_history_entries: plan_history.values().map(|h| h.plans.len()).sum(),
        }
    }
}

// ============================================================================
// SQL Plan Baseline
// ============================================================================

/// SQL Plan Baseline
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SqlPlanBaseline {
    /// Query fingerprint
    pub fingerprint: QueryFingerprint,
    /// Accepted plans (ordered by cost)
    pub accepted_plans: Vec<PhysicalPlan>,
    /// Baseline enabled
    pub enabled: bool,
    /// Fixed baseline (no evolution)
    pub fixed: bool,
    /// Origin (manual/auto)
    pub origin: BaselineOrigin,
    /// Creation time
    pub created_at: SystemTime,
    /// Last modified time
    pub last_modified: SystemTime,
    /// Last evolved time
    pub last_evolved: Option<SystemTime>,
    /// Execution count
    pub execution_count: u64,
    /// Average execution time
    pub avg_execution_time: Duration,
}

impl SqlPlanBaseline {
    /// Create a new baseline
    pub fn new(fingerprint: QueryFingerprint, plan: PhysicalPlan) -> Self {
        Self {
            fingerprint,
            accepted_plans: vec![plan],
            enabled: true,
            fixed: false,
            origin: BaselineOrigin::AutoCapture,
            created_at: SystemTime::now(),
            last_modified: SystemTime::now(),
            last_evolved: None,
            execution_count: 0,
            avg_execution_time: Duration::from_secs(0),
        }
    }

    /// Get best plan from baseline
    pub fn get_best_plan(&self) -> Option<&PhysicalPlan> {
        self.accepted_plans.first()
    }

    /// Add accepted plan
    /// Optimized to eliminate heap allocations in comparison
    #[inline]
    pub fn add_accepted_plan(&mut self, plan: PhysicalPlan) {
        self.accepted_plans.push(plan);
        // Sort by cost in-place, unstable for better performance
        self.accepted_plans.sort_unstable_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap());
    }

    /// Remove plan
    pub fn remove_plan(&mut self, plan_id: PlanId) {
        self.accepted_plans.retain(|p| p.plan_id != plan_id);
    }

    /// Record execution
    pub fn record_execution(&mut self, execution_time: Duration) {
        self.execution_count += 1;

        // Update average execution time
        let total_time = self.avg_execution_time.as_secs_f64() * (self.execution_count - 1) as f64
            + execution_time.as_secs_f64();
        self.avg_execution_time = Duration::from_secs_f64(total_time / self.execution_count as f64);
    }
}

/// Baseline origin
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaselineOrigin {
    Manual,
    AutoCapture,
    Evolution,
}

// ============================================================================
// Plan History
// ============================================================================

/// Plan history for a query
#[derive(Debug, Clone)]
pub struct PlanHistory {
    /// Query fingerprint
    pub fingerprint: QueryFingerprint,
    /// Historical plans
    pub plans: VecDeque<HistoricalPlan>,
    /// Maximum history size
    max_size: usize,
}

impl PlanHistory {
    /// Create new plan history
    pub fn new(fingerprint: QueryFingerprint) -> Self {
        Self {
            fingerprint,
            plans: VecDeque::new(),
            max_size: 100,
        }
    }

    /// Add plan to history
    pub fn add_plan(&mut self, plan: PhysicalPlan) {
        let historical_plan = HistoricalPlan {
            plan,
            first_seen: SystemTime::now(),
            last_seen: SystemTime::now(),
            execution_count: 1,
            total_execution_time: Duration::from_secs(0),
            min_execution_time: Duration::from_secs(0),
            max_execution_time: Duration::from_secs(0),
            avg_execution_time: Duration::from_secs(0),
        };

        self.plans.push_back(historical_plan);

        // Maintain max size
        if self.plans.len() > self.max_size {
            self.plans.pop_front();
        }
    }

    /// Get evolution candidates
    pub fn get_evolution_candidates(
        &self,
        current_plans: &[PhysicalPlan],
        min_executions: u64,
        performance_threshold: f64,
    ) -> Vec<PhysicalPlan> {
        let mut candidates = Vec::new();

        for historical in &self.plans {
            // Skip if already in baseline
            if current_plans.iter().any(|p| p.plan_id == historical.plan.plan_id) {
                continue;
            }

            // Check execution count
            if historical.execution_count < min_executions {
                continue;
            }

            // Check performance
            if let Some(best_plan) = current_plans.first() {
                let performance_ratio = historical.plan.cost / best_plan.cost;
                if performance_ratio < performance_threshold {
                    candidates.push(historical.plan.clone());
                }
            }
        }

        candidates
    }

    /// Get plan statistics
    pub fn get_statistics(&self, plan_id: PlanId) -> Option<PlanStatistics> {
        self.plans.iter()
            .find(|p| p.plan.plan_id == plan_id)
            .map(|p| PlanStatistics {
                plan_id: p.plan.plan_id,
                execution_count: p.execution_count,
                avg_execution_time: p.avg_execution_time,
                min_execution_time: p.min_execution_time,
                max_execution_time: p.max_execution_time,
            })
    }
}

/// Historical plan
#[derive(Debug, Clone)]
pub struct HistoricalPlan {
    pub plan: PhysicalPlan,
    pub first_seen: SystemTime,
    pub last_seen: SystemTime,
    pub execution_count: u64,
    pub total_execution_time: Duration,
    pub min_execution_time: Duration,
    pub max_execution_time: Duration,
    pub avg_execution_time: Duration,
}

/// Plan statistics
#[derive(Debug, Clone)]
pub struct PlanStatistics {
    pub plan_id: PlanId,
    pub execution_count: u64,
    pub avg_execution_time: Duration,
    pub min_execution_time: Duration,
    pub max_execution_time: Duration,
}

// ============================================================================
// Evolution Configuration
// ============================================================================

/// Evolution configuration
#[derive(Debug, Clone)]
pub struct EvolutionConfig {
    /// Enable automatic evolution
    pub auto_evolution: bool,
    /// Minimum executions before evolution
    pub min_executions: u64,
    /// Performance threshold for evolution (ratio)
    pub performance_threshold: f64,
    /// Evolution interval
    pub evolution_interval: Duration,
    /// Verify evolved plans
    pub verify_evolved_plans: bool,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            auto_evolution: true,
            min_executions: 10,
            performance_threshold: 1.2,
            evolution_interval: Duration::from_secs(3600),
            verify_evolved_plans: true,
        }
    }
}

// ============================================================================
// Capture Configuration
// ============================================================================

/// Baseline capture configuration
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Enable automatic capture
    pub auto_capture: bool,
    /// Capture mode
    pub capture_mode: CaptureMode,
    /// Minimum execution time for capture
    pub min_execution_time: Duration,
    /// Maximum baselines per query
    pub max_baselines_per_query: usize,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            auto_capture: true,
            capture_mode: CaptureMode::Auto,
            min_execution_time: Duration::from_millis(100),
            max_baselines_per_query: 10,
        }
    }
}

/// Capture mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    /// Capture all plans
    Auto,
    /// Only capture manually specified plans
    Manual,
    /// Capture only repeatable plans
    Repeatable,
}

// ============================================================================
// Regression Detector
// ============================================================================

/// Plan regression detector
pub struct RegressionDetector {
    /// Regression threshold (cost ratio)
    regression_threshold: f64,
    /// Regression history
    regression_history: Arc<RwLock<Vec<RegressionEvent>>>,
}

impl RegressionDetector {
    pub fn new() -> Self {
        Self {
            regression_threshold: 1.5, // 50% worse
            regression_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Check if plan is a regression
    pub fn is_regression(
        &self,
        candidate: &PhysicalPlan,
        baseline: &SqlPlanBaseline,
    ) -> Result<bool> {
        if let Some(best_plan) = baseline.get_best_plan() {
            let cost_ratio = candidate.cost / best_plan.cost;

            if cost_ratio > self.regression_threshold {
                // Record regression event
                let event = RegressionEvent {
                    baseline_fingerprint: baseline.fingerprint.clone(),
                    baseline_plan_id: best_plan.plan_id,
                    candidate_plan_id: candidate.plan_id,
                    cost_ratio,
                    detected_at: SystemTime::now(),
                };

                self.regression_history.write().unwrap().push(event);

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get regression events
    pub fn get_regression_events(&self) -> Vec<RegressionEvent> {
        self.regression_history.read().unwrap().clone()
    }

    /// Clear regression history
    pub fn clear_history(&self) {
        self.regression_history.write().unwrap().clear();
    }
}

/// Regression event
#[derive(Debug, Clone)]
pub struct RegressionEvent {
    pub baseline_fingerprint: QueryFingerprint,
    pub baseline_plan_id: PlanId,
    pub candidate_plan_id: PlanId,
    pub cost_ratio: f64,
    pub detected_at: SystemTime,
}

// ============================================================================
// Plan Comparison
// ============================================================================

/// Plan comparison result
#[derive(Debug, Clone)]
pub struct PlanComparison {
    pub cost_diff: f64,
    pub cardinality_diff: i64,
    pub operator_diff: String,
    pub better_plan: u8,
}

impl PlanComparison {
    /// Check if plans are equivalent
    pub fn are_equivalent(&self) -> bool {
        self.cost_diff.abs() < 0.01 && self.cardinality_diff.abs() < 10
    }

    /// Get improvement percentage
    pub fn get_improvement_percentage(&self) -> f64 {
        if self.cost_diff == 0.0 {
            return 0.0;
        }

        (self.cost_diff.abs() / self.cost_diff.abs()) * 100.0
    }
}

// ============================================================================
// Baseline Statistics
// ============================================================================

/// Baseline statistics
#[derive(Debug, Clone)]
pub struct BaselineStatistics {
    pub total_baselines: usize,
    pub enabled_baselines: usize,
    pub total_accepted_plans: usize,
    pub total_history_entries: usize,
}

// ============================================================================
// Baseline Export/Import
// ============================================================================

/// Baseline export format
#[derive(Debug, Clone)]
pub struct BaselineExport {
    pub baseline: SqlPlanBaseline,
    pub export_version: String,
    pub exported_at: SystemTime,
}

impl BaselineExport {
    /// Export baseline to JSON (simplified)
    pub fn to_json(&self) -> String {
        format!("{{\"baseline\": \"...\"}}")
    }

    /// Import baseline from JSON (simplified)
    pub fn from_json(_json: &str) -> Result<Self> {
        Err(DbError::Internal("Not implemented".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimizer_pro::{PhysicalOperator, Schema, PlanMetadata};
    use crate::common::TableId;

    #[test]
    fn test_baseline_creation() {
        let fingerprint = QueryFingerprint::new("SELECT * FROM users", vec![], 1);
        let plan = create_test_plan();

        let baseline = SqlPlanBaseline::new(fingerprint, plan);
        assert_eq!(baseline.accepted_plans.len(), 1);
        assert!(baseline.enabled);
    }

    #[test]
    fn test_plan_history() {
        let fingerprint = QueryFingerprint::new("SELECT * FROM users", vec![], 1);
        let mut history = PlanHistory::new(fingerprint);

        let plan = create_test_plan();
        history.add_plan(plan);

        assert_eq!(history.plans.len(), 1);
    }

    #[test]
    fn test_regression_detector() {
        let detector = RegressionDetector::new();
        let fingerprint = QueryFingerprint::new("SELECT * FROM users", vec![], 1);

        let good_plan = create_test_plan();
        let mut bad_plan = create_test_plan();
        bad_plan.cost = 1000.0; // Much worse cost

        let baseline = SqlPlanBaseline::new(fingerprint, good_plan);

        let is_regression = detector.is_regression(&bad_plan, &baseline).unwrap();
        assert!(is_regression);
    }

    #[test]
    fn test_baseline_manager() {
        let manager = PlanBaselineManager::new();
        let fingerprint = QueryFingerprint::new("SELECT * FROM users", vec![], 1);
        let plan = create_test_plan();

        manager.capture_baseline(fingerprint.clone(), plan).unwrap();

        let baseline = manager.get_baseline(&fingerprint).unwrap();
        assert!(baseline.is_some());
    }

    fn create_test_plan() -> PhysicalPlan {
        PhysicalPlan {
            plan_id: PlanId(1),
            operator: PhysicalOperator::SeqScan {
                table_id: TableId(1),
                filter: None,
            },
            cost: 100.0,
            cardinality: 1000,
            schema: Schema { columns: vec![] },
            metadata: PlanMetadata {
                created_at: SystemTime::now(),
                optimizer_version: "1.0".to_string(),
                hints: vec![],
                transformations: vec![],
                from_baseline: false,
            },
        }
    }
}


