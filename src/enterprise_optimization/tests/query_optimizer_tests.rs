// Comprehensive tests for Query Optimizer Improvements
//
// Tests for:
// - Q001: Hardware-aware cost model calibration
// - Q002: Adaptive query execution
// - Q003: Plan baseline stability
// - Integration module

use crate::enterprise_optimization::{
    hardware_cost_calibration::{CalibratedCostModel, HardwareProfile, HistogramManager},
    adaptive_execution::{AdaptiveExecutionEngine, ParallelDegreeController, MemoryGrantManager},
    plan_stability::{EnhancedBaselineManager, PlanValidator, PlanQualityScorer},
    query_optimizer_integration::{EnterpriseQueryOptimizer, EnterpriseOptimizerBuilder},
};
use crate::optimizer_pro::{
    CostParameters, Query, QueryFingerprint, PhysicalPlan, PhysicalOperator,
    PlanId, PlanMetadata, Schema,
};
use crate::common::Value;
use std::time::{Duration, SystemTime};

// ============================================================================
// Q001: Hardware Cost Calibration Tests
// ============================================================================

#[test]
fn test_hardware_profile_detection() {
    let profile = HardwareProfile::auto_detect().unwrap();

    assert!(profile.cpu_cores > 0);
    assert!(profile.cpu_speed_ghz > 0.0);
    assert!(profile.memory_bandwidth_gbps > 0.0);
}

#[test]
fn test_calibrated_cost_model_creation() {
    let base_params = CostParameters::default();
    let model = CalibratedCostModel::new(base_params);

    let params = model.get_parameters();
    assert!(params.cpu_tuple_cost > 0.0);
    assert!(params.seq_page_cost > 0.0);
}

#[test]
fn test_cost_model_calibration() {
    let base_params = CostParameters::default();
    let model = CalibratedCostModel::new(base_params);

    // Create test plan
    let plan = create_test_plan();

    // Record multiple executions
    for i in 1..=20 {
        model.record_execution(
            &plan,
            Duration::from_millis(100 + i * 5),
            1000 + i as usize * 10,
        );
    }

    let metrics = model.get_calibration_metrics();
    assert!(metrics.total_executions >= 20);
}

#[test]
fn test_histogram_manager() {
    let manager = HistogramManager::new();

    let values = vec![
        Value::Integer(1), Value::Integer(2), Value::Integer(3),
        Value::Integer(4), Value::Integer(5), Value::Integer(6),
        Value::Integer(7), Value::Integer(8), Value::Integer(9),
        Value::Integer(10),
    ];

    let histogram = manager.build_histogram(
        1,
        "test_column",
        &values,
        4,
        crate::optimizer_pro::cost_model::HistogramType::EqualWidth,
    ).unwrap();

    assert!(!histogram.buckets.is_empty());
    assert_eq!(histogram.histogram_type, crate::optimizer_pro::cost_model::HistogramType::EqualWidth);
}

#[test]
fn test_histogram_caching() {
    let manager = HistogramManager::new();

    let values = vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)];

    manager.build_histogram(
        1,
        "cached_column",
        &values,
        2,
        crate::optimizer_pro::cost_model::HistogramType::Hybrid,
    ).unwrap();

    let cached = manager.get_histogram(1, "cached_column");
    assert!(cached.is_some());
}

// ============================================================================
// Q002: Adaptive Execution Tests
// ============================================================================

#[test]
fn test_parallel_degree_controller() {
    let controller = ParallelDegreeController::new();

    // Small query: should use 1 thread
    let degree1 = controller.compute_initial_degree(1000, 5.0);
    assert_eq!(degree1, 1);

    // Large query: should use multiple threads
    let degree2 = controller.compute_initial_degree(1_000_000, 500.0);
    assert!(degree2 > 1);
    assert!(degree2 <= controller.max_degree);
}

#[test]
fn test_parallel_degree_runtime_adjustment() {
    let controller = ParallelDegreeController::new();

    let current_degree = 2;
    let actual_cardinality = 500_000;

    let new_degree = controller.adjust_degree_runtime(current_degree, actual_cardinality);

    // Should scale up for large cardinality
    assert!(new_degree >= current_degree);
}

#[test]
fn test_parallel_performance_tracking() {
    let controller = ParallelDegreeController::new();

    controller.record_performance(4, 100_000, Duration::from_millis(200));
    controller.record_performance(8, 100_000, Duration::from_millis(150));

    let optimal = controller.get_optimal_degree(100_000);
    assert!(optimal > 0);
}

#[test]
fn test_memory_grant_manager() {
    let manager = MemoryGrantManager::new();

    let grant = manager.request_grant(PlanId(1), 10_000, 100).unwrap();

    assert!(grant > 0);
    assert!(grant <= manager.total_memory / 4); // Max 25% of total

    // Record actual usage
    manager.record_actual_usage(PlanId(1), grant, grant / 2);
}

#[test]
fn test_memory_grant_feedback() {
    let manager = MemoryGrantManager::new();

    // First execution
    let grant1 = manager.request_grant(PlanId(1), 10_000, 100).unwrap();
    manager.record_actual_usage(PlanId(1), grant1, grant1 / 2);

    // Second execution - should use feedback
    let grant2 = manager.request_grant(PlanId(1), 10_000, 100).unwrap();

    // Grant should be adjusted based on actual usage
    assert!(grant2 > 0);
}

#[test]
fn test_adaptive_execution_engine() {
    let engine = AdaptiveExecutionEngine::new();

    let stats = engine.get_statistics();
    assert_eq!(stats.total_executions, 0);
    assert_eq!(stats.plan_switches, 0);
}

// ============================================================================
// Q003: Plan Baseline Stability Tests
// ============================================================================

#[test]
fn test_plan_validator() {
    let validator = PlanValidator::new();

    let plan = create_test_plan();
    let validation = validator.validate(&plan).unwrap();

    assert!(validation.is_valid);
    assert!(validation.failures.is_empty());
}

#[test]
fn test_plan_validator_invalid_cost() {
    let validator = PlanValidator::new();

    let mut plan = create_test_plan();
    plan.cost = -100.0; // Invalid cost

    let validation = validator.validate(&plan).unwrap();

    assert!(!validation.is_valid);
    assert!(!validation.failures.is_empty());
}

#[test]
fn test_plan_quality_scorer() {
    let scorer = PlanQualityScorer::new();

    let plan = create_test_plan();
    let score = scorer.score_plan(&plan, Duration::from_millis(50), 1000);

    assert!(score >= 0.0 && score <= 1.0);
}

#[test]
fn test_quality_scorer_different_scenarios() {
    let scorer = PlanQualityScorer::new();
    let plan = create_test_plan();

    // Good execution
    let good_score = scorer.score_plan(&plan, Duration::from_millis(10), plan.cardinality);

    // Bad execution
    let bad_score = scorer.score_plan(&plan, Duration::from_secs(5), plan.cardinality * 10);

    assert!(good_score > bad_score);
}

#[test]
fn test_enhanced_baseline_manager() {
    let manager = EnhancedBaselineManager::new();

    let fingerprint = QueryFingerprint::new("SELECT * FROM test", vec![], 1);
    let plan = create_test_plan();

    let captured = manager.capture_plan_with_validation(
        fingerprint.clone(),
        plan.clone(),
        Duration::from_millis(100),
        1000,
    ).unwrap();

    assert!(captured);

    let stats = manager.get_statistics();
    assert!(stats.plans_captured > 0);
}

#[test]
fn test_baseline_retrieval() {
    let manager = EnhancedBaselineManager::new();

    let fingerprint = QueryFingerprint::new("SELECT * FROM test", vec![], 1);
    let plan = create_test_plan();

    manager.capture_plan_with_validation(
        fingerprint.clone(),
        plan.clone(),
        Duration::from_millis(100),
        1000,
    ).unwrap();

    let retrieved = manager.get_best_plan(&fingerprint).unwrap();
    assert!(retrieved.is_some());
}

#[test]
fn test_baseline_validation() {
    let manager = EnhancedBaselineManager::new();

    let fingerprint = QueryFingerprint::new("SELECT * FROM test", vec![], 1);
    let plan = create_test_plan();

    manager.capture_plan_with_validation(
        fingerprint,
        plan,
        Duration::from_millis(100),
        1000,
    ).unwrap();

    let report = manager.validate_all_baselines().unwrap();
    assert!(report.total_baselines > 0);
}

#[test]
fn test_baseline_evolution() {
    let manager = EnhancedBaselineManager::new();

    let report = manager.evolve_with_validation().unwrap();
    assert_eq!(report.baselines_evolved, 0); // No baselines yet
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_enterprise_optimizer_creation() {
    let optimizer = EnterpriseOptimizerBuilder::new()
        .with_max_join_combinations(5000)
        .auto_capture_baselines(true)
        .build();

    let stats = optimizer.get_comprehensive_stats();
    assert_eq!(stats.base_stats.queries_optimized, 0);
}

#[test]
fn test_enterprise_optimizer_builder() {
    let config = EnterpriseOptimizerBuilder::new()
        .with_optimization_timeout(Duration::from_secs(10))
        .enable_hardware_calibration(false)
        .enable_adaptive_execution(true)
        .build();

    assert!(!config.config.enable_hardware_calibration);
    assert!(config.config.enable_adaptive_execution);
}

#[test]
fn test_comprehensive_stats() {
    let optimizer = EnterpriseOptimizerBuilder::new().build();

    let stats = optimizer.get_comprehensive_stats();

    // Check all stat components are present
    assert_eq!(stats.base_stats.queries_optimized, 0);
    assert_eq!(stats.calibration_metrics.total_executions, 0);
    assert_eq!(stats.adaptive_stats.total_executions, 0);
    assert_eq!(stats.baseline_stats.total_baselines, 0);
}

#[test]
fn test_calibrated_parameters() {
    let optimizer = EnterpriseOptimizerBuilder::new()
        .enable_hardware_calibration(true)
        .build();

    let params = optimizer.get_calibrated_parameters();

    assert!(params.cpu_tuple_cost > 0.0);
    assert!(params.seq_page_cost > 0.0);
    assert!(params.random_page_cost > 0.0);
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_plan() -> PhysicalPlan {
    PhysicalPlan {
        plan_id: PlanId(1),
        operator: PhysicalOperator::SeqScan {
            table_id: 1,
            filter: None,
        },
        cost: 100.0,
        cardinality: 1000,
        schema: Schema::empty(),
        metadata: PlanMetadata {
            created_at: SystemTime::now(),
            optimizer_version: "1.0".to_string(),
            hints: vec![],
            transformations: vec![],
            from_baseline: false,
        },
    }
}
