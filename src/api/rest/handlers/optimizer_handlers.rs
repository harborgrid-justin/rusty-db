// Query Optimizer REST API Handlers
//
// Provides REST endpoints for:
// - Optimizer hints management
// - SQL plan baselines
// - Query execution plan analysis (EXPLAIN)
//
// Implements Oracle-compatible query optimization features.

use axum::{
    extract::{Path, State, Query as AxumQuery},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::SystemTime;

use crate::api::rest::types::ApiState;
use crate::error::{DbError, Result};
use crate::optimizer_pro::{
    HintParser, PlanBaselineManager, SqlPlanBaseline,
    QueryFingerprint, PhysicalPlan, QueryOptimizer, OptimizerConfig,
};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListHintsQuery {
    pub category: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HintDefinitionResponse {
    pub name: String,
    pub category: String,
    pub description: String,
    pub parameters: Vec<String>,
    pub example: String,
}

#[derive(Debug, Serialize)]
pub struct ActiveHintsResponse {
    pub session_id: String,
    pub hints: Vec<HintInfo>,
}

#[derive(Debug, Serialize)]
pub struct HintInfo {
    pub hint: String,
    pub applied_at: String,
    pub effective: bool,
}

#[derive(Debug, Deserialize)]
pub struct ApplyHintRequest {
    pub query: String,
    pub hints: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ApplyHintResponse {
    pub hint_id: String,
    pub parsed_hints: Vec<String>,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBaselineRequest {
    pub query_text: String,
    pub param_types: Option<Vec<String>>,
    pub schema_version: Option<u64>,
    pub enabled: Option<bool>,
    pub fixed: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct BaselineResponse {
    pub fingerprint: String,
    pub enabled: bool,
    pub fixed: bool,
    pub origin: String,
    pub created_at: String,
    pub last_modified: String,
    pub last_evolved: Option<String>,
    pub execution_count: u64,
    pub avg_execution_time_ms: f64,
    pub accepted_plans_count: usize,
}

#[derive(Debug, Serialize)]
pub struct BaselineDetailResponse {
    pub fingerprint: String,
    pub enabled: bool,
    pub fixed: bool,
    pub origin: String,
    pub created_at: String,
    pub last_modified: String,
    pub last_evolved: Option<String>,
    pub execution_count: u64,
    pub avg_execution_time_ms: f64,
    pub accepted_plans: Vec<PlanSummary>,
}

#[derive(Debug, Serialize)]
pub struct PlanSummary {
    pub plan_id: u64,
    pub cost: f64,
    pub cardinality: usize,
    pub operator_type: String,
    pub from_baseline: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBaselineRequest {
    pub enabled: Option<bool>,
    pub fixed: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct EvolveBaselineResponse {
    pub evolved_plans: usize,
    pub new_plans_added: Vec<u64>,
    pub evolution_time_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct ExplainRequest {
    pub query: String,
    pub analyze: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ExplainResponse {
    pub query: String,
    pub plan: ExplainPlan,
    pub estimated_cost: f64,
    pub estimated_rows: usize,
    pub planning_time_ms: f64,
    pub execution_time_ms: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ExplainPlan {
    pub operator: String,
    pub cost: f64,
    pub rows: usize,
    pub details: serde_json::Value,
    pub children: Vec<ExplainPlan>,
}

// ============================================================================
// Optimizer Hints Endpoints
// ============================================================================

/// GET /api/v1/optimizer/hints
/// List all available optimizer hints
pub async fn list_hints(
    State(_state): State<Arc<ApiState>>,
    AxumQuery(query): AxumQuery<ListHintsQuery>,
) -> Result<Response> {
    let hint_parser = HintParser::new();
    let all_hints = hint_parser.get_supported_hints();

    let mut hints: Vec<HintDefinitionResponse> = all_hints
        .into_iter()
        .filter(|h| {
            if let Some(ref search) = query.search {
                h.name.to_lowercase().contains(&search.to_lowercase())
                    || h.description.to_lowercase().contains(&search.to_lowercase())
            } else {
                true
            }
        })
        .filter(|h| {
            if let Some(ref category) = query.category {
                format!("{:?}", h.category).eq_ignore_ascii_case(category)
            } else {
                true
            }
        })
        .map(|h| HintDefinitionResponse {
            name: h.name.clone(),
            category: format!("{:?}", h.category),
            description: h.description.clone(),
            parameters: h.parameters.clone(),
            example: format!("/*+ {} */", h.name),
        })
        .collect();

    hints.sort_by(|a, b| a.name.cmp(&b.name));

    Ok((StatusCode::OK, Json(serde_json::json!({
        "hints": hints,
        "total": hints.len(),
    })))
    .into_response())
}

/// GET /api/v1/optimizer/hints/active
/// Get active hints for current session
pub async fn get_active_hints(
    State(_state): State<Arc<ApiState>>,
) -> Result<Response> {
    // In a real implementation, this would retrieve hints from session state
    // For now, return an empty list
    Ok((StatusCode::OK, Json(ActiveHintsResponse {
        session_id: "current".to_string(),
        hints: vec![],
    }))
    .into_response())
}

/// POST /api/v1/optimizer/hints
/// Apply hints to a query
pub async fn apply_hints(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<ApplyHintRequest>,
) -> Result<Response> {
    let hint_parser = HintParser::new();

    // Parse hints from the query
    let parsed_hints = hint_parser.parse_hints(&request.query)
        .map_err(|e| DbError::Internal(format!("Failed to parse hints: {}", e)))?;

    let hint_id = format!("hint_{}", uuid::Uuid::new_v4());

    let response = ApplyHintResponse {
        hint_id,
        parsed_hints: parsed_hints.iter().map(|h| format!("{}", h)).collect(),
        conflicts: vec![],
        warnings: vec![],
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// DELETE /api/v1/optimizer/hints/{id}
/// Remove a specific hint
pub async fn remove_hint(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<Response> {
    // In a real implementation, this would remove the hint from session state
    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": format!("Hint {} removed", id),
        "success": true,
    })))
    .into_response())
}

// ============================================================================
// Plan Baselines Endpoints
// ============================================================================

/// GET /api/v1/optimizer/baselines
/// List all plan baselines
pub async fn list_baselines(
    State(_state): State<Arc<ApiState>>,
) -> Result<Response> {
    let baseline_manager = PlanBaselineManager::new();
    let baselines = baseline_manager.get_all_baselines();

    let baseline_responses: Vec<BaselineResponse> = baselines
        .iter()
        .map(|b| baseline_to_response(b))
        .collect();

    Ok((StatusCode::OK, Json(serde_json::json!({
        "baselines": baseline_responses,
        "total": baseline_responses.len(),
    })))
    .into_response())
}

/// POST /api/v1/optimizer/baselines
/// Create a new plan baseline
pub async fn create_baseline(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<CreateBaselineRequest>,
) -> Result<Response> {
    let baseline_manager = PlanBaselineManager::new();

    // Create query fingerprint
    let fingerprint = QueryFingerprint::new(
        &request.query_text,
        request.param_types.unwrap_or_default(),
        request.schema_version.unwrap_or(1),
    );

    // Create optimizer and generate a plan
    let optimizer = QueryOptimizer::new(OptimizerConfig::default());
    let query = crate::optimizer_pro::Query::parse(&request.query_text)
        .map_err(|e| DbError::Internal(format!("Failed to parse query: {}", e)))?;

    let plan = optimizer.optimize(&query)
        .map_err(|e| DbError::Internal(format!("Failed to optimize query: {}", e)))?;

    // Capture the baseline
    baseline_manager.capture_baseline(fingerprint.clone(), plan)
        .map_err(|e| DbError::Internal(format!("Failed to capture baseline: {}", e)))?;

    // Get the created baseline
    let baseline = baseline_manager.get_baseline(&fingerprint)?
        .ok_or_else(|| DbError::Internal("Failed to retrieve created baseline".to_string()))?;

    let response = baseline_to_response(&baseline);

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// GET /api/v1/optimizer/baselines/{id}
/// Get plan baseline details
pub async fn get_baseline(
    State(_state): State<Arc<ApiState>>,
    Path(fingerprint_str): Path<String>,
) -> Result<Response> {
    let baseline_manager = PlanBaselineManager::new();

    // Parse fingerprint from string (simplified)
    let fingerprint = QueryFingerprint::new(&fingerprint_str, vec![], 1);

    let baseline = baseline_manager.get_baseline(&fingerprint)?
        .ok_or_else(|| DbError::Internal(format!("Baseline not found: {}", fingerprint_str)))?;

    let response = BaselineDetailResponse {
        fingerprint: fingerprint_str,
        enabled: baseline.enabled,
        fixed: baseline.fixed,
        origin: format!("{:?}", baseline.origin),
        created_at: format_system_time(baseline.created_at),
        last_modified: format_system_time(baseline.last_modified),
        last_evolved: baseline.last_evolved.map(format_system_time),
        execution_count: baseline.execution_count,
        avg_execution_time_ms: baseline.avg_execution_time.as_secs_f64() * 1000.0,
        accepted_plans: baseline.accepted_plans.iter().map(plan_to_summary).collect(),
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// PUT /api/v1/optimizer/baselines/{id}
/// Update plan baseline settings
pub async fn update_baseline(
    State(_state): State<Arc<ApiState>>,
    Path(fingerprint_str): Path<String>,
    Json(request): Json<UpdateBaselineRequest>,
) -> Result<Response> {
    let baseline_manager = PlanBaselineManager::new();
    let fingerprint = QueryFingerprint::new(&fingerprint_str, vec![], 1);

    if let Some(enabled) = request.enabled {
        if enabled {
            baseline_manager.enable_baseline(&fingerprint)?;
        } else {
            baseline_manager.disable_baseline(&fingerprint)?;
        }
    }

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Baseline updated successfully",
        "fingerprint": fingerprint_str,
    })))
    .into_response())
}

/// DELETE /api/v1/optimizer/baselines/{id}
/// Delete a plan baseline
pub async fn delete_baseline(
    State(_state): State<Arc<ApiState>>,
    Path(fingerprint_str): Path<String>,
) -> Result<Response> {
    let baseline_manager = PlanBaselineManager::new();
    let fingerprint = QueryFingerprint::new(&fingerprint_str, vec![], 1);

    baseline_manager.delete_baseline(&fingerprint)?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Baseline deleted successfully",
        "fingerprint": fingerprint_str,
    })))
    .into_response())
}

/// POST /api/v1/optimizer/baselines/{id}/evolve
/// Evolve a plan baseline with new candidate plans
pub async fn evolve_baseline(
    State(_state): State<Arc<ApiState>>,
    Path(_fingerprint_str): Path<String>,
) -> Result<Response> {
    let baseline_manager = PlanBaselineManager::new();
    let start = std::time::Instant::now();

    let evolved_count = baseline_manager.evolve_baselines()
        .map_err(|e| DbError::Internal(format!("Failed to evolve baselines: {}", e)))?;

    let evolution_time_ms = start.elapsed().as_millis() as u64;

    let response = EvolveBaselineResponse {
        evolved_plans: evolved_count,
        new_plans_added: vec![],
        evolution_time_ms,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

// ============================================================================
// EXPLAIN Endpoints
// ============================================================================

/// POST /api/v1/query/explain
/// Get query execution plan (EXPLAIN)
pub async fn explain_query(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<ExplainRequest>,
) -> Result<Response> {
    let start = std::time::Instant::now();

    let optimizer = QueryOptimizer::new(OptimizerConfig::default());
    let query = crate::optimizer_pro::Query::parse(&request.query)
        .map_err(|e| DbError::Internal(format!("Failed to parse query: {}", e)))?;

    let plan = optimizer.optimize(&query)
        .map_err(|e| DbError::Internal(format!("Failed to optimize query: {}", e)))?;

    let planning_time = start.elapsed().as_secs_f64() * 1000.0;

    let response = ExplainResponse {
        query: request.query,
        plan: physical_plan_to_explain(&plan),
        estimated_cost: plan.cost,
        estimated_rows: plan.cardinality,
        planning_time_ms: planning_time,
        execution_time_ms: None,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/v1/query/explain/analyze
/// Get query execution plan with actual execution statistics (EXPLAIN ANALYZE)
pub async fn explain_analyze_query(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<ExplainRequest>,
) -> Result<Response> {
    let start = std::time::Instant::now();

    let optimizer = QueryOptimizer::new(OptimizerConfig::default());
    let query = crate::optimizer_pro::Query::parse(&request.query)
        .map_err(|e| DbError::Internal(format!("Failed to parse query: {}", e)))?;

    let plan = optimizer.optimize(&query)
        .map_err(|e| DbError::Internal(format!("Failed to optimize query: {}", e)))?;

    let planning_time = start.elapsed().as_secs_f64() * 1000.0;

    // Execute the plan to get actual statistics
    let exec_start = std::time::Instant::now();
    let _result = optimizer.execute_adaptive(&plan)
        .map_err(|e| DbError::Internal(format!("Failed to execute query: {}", e)))?;
    let execution_time = exec_start.elapsed().as_secs_f64() * 1000.0;

    let response = ExplainResponse {
        query: request.query,
        plan: physical_plan_to_explain(&plan),
        estimated_cost: plan.cost,
        estimated_rows: plan.cardinality,
        planning_time_ms: planning_time,
        execution_time_ms: Some(execution_time),
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

// ============================================================================
// Helper Functions
// ============================================================================

fn baseline_to_response(baseline: &SqlPlanBaseline) -> BaselineResponse {
    BaselineResponse {
        fingerprint: format!("{:?}", baseline.fingerprint),
        enabled: baseline.enabled,
        fixed: baseline.fixed,
        origin: format!("{:?}", baseline.origin),
        created_at: format_system_time(baseline.created_at),
        last_modified: format_system_time(baseline.last_modified),
        last_evolved: baseline.last_evolved.map(format_system_time),
        execution_count: baseline.execution_count,
        avg_execution_time_ms: baseline.avg_execution_time.as_secs_f64() * 1000.0,
        accepted_plans_count: baseline.accepted_plans.len(),
    }
}

fn plan_to_summary(plan: &PhysicalPlan) -> PlanSummary {
    PlanSummary {
        plan_id: plan.plan_id.0,
        cost: plan.cost,
        cardinality: plan.cardinality,
        operator_type: format!("{:?}", plan.operator).split('{').next().unwrap_or("Unknown").to_string(),
        from_baseline: plan.metadata.from_baseline,
    }
}

fn physical_plan_to_explain(plan: &PhysicalPlan) -> ExplainPlan {
    use crate::optimizer_pro::PhysicalOperator;

    let (operator, details, children) = match &plan.operator {
        PhysicalOperator::SeqScan { table_id, filter } => {
            ("SeqScan".to_string(), serde_json::json!({
                "table_id": table_id,
                "filter": filter.as_ref().map(|f| format!("{:?}", f)),
            }), vec![])
        },
        PhysicalOperator::IndexScan { table_id, index_id, key_conditions, filter } => {
            ("IndexScan".to_string(), serde_json::json!({
                "table_id": table_id,
                "index_id": index_id,
                "key_conditions": key_conditions.len(),
                "filter": filter.as_ref().map(|f| format!("{:?}", f)),
            }), vec![])
        },
        PhysicalOperator::HashJoin { left, right, hash_keys, condition, join_type } => {
            ("HashJoin".to_string(), serde_json::json!({
                "join_type": format!("{:?}", join_type),
                "hash_keys": hash_keys.len(),
                "condition": condition.as_ref().map(|c| format!("{:?}", c)),
            }), vec![
                physical_plan_to_explain(left),
                physical_plan_to_explain(right),
            ])
        },
        PhysicalOperator::NestedLoopJoin { left, right, condition, join_type } => {
            ("NestedLoopJoin".to_string(), serde_json::json!({
                "join_type": format!("{:?}", join_type),
                "condition": condition.as_ref().map(|c| format!("{:?}", c)),
            }), vec![
                physical_plan_to_explain(left),
                physical_plan_to_explain(right),
            ])
        },
        PhysicalOperator::Sort { input, sort_keys } => {
            ("Sort".to_string(), serde_json::json!({
                "sort_keys": sort_keys.len(),
            }), vec![physical_plan_to_explain(input)])
        },
        PhysicalOperator::Aggregate { input, group_by, aggregates } => {
            ("Aggregate".to_string(), serde_json::json!({
                "group_by": group_by.len(),
                "aggregates": aggregates.len(),
            }), vec![physical_plan_to_explain(input)])
        },
        _ => {
            (format!("{:?}", plan.operator).split('{').next().unwrap_or("Unknown").to_string(),
             serde_json::json!({}), vec![])
        }
    };

    ExplainPlan {
        operator,
        cost: plan.cost,
        rows: plan.cardinality,
        details,
        children,
    }
}

fn format_system_time(time: SystemTime) -> String {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => {
            let secs = duration.as_secs();
            let datetime = chrono::DateTime::from_timestamp(secs as i64, 0)
                .unwrap_or_default();
            datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
        },
        Err(_) => "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hint_definition_response() {
        let response = HintDefinitionResponse {
            name: "FULL".to_string(),
            category: "AccessPath".to_string(),
            description: "Force full table scan".to_string(),
            parameters: vec!["table".to_string()],
            example: "/*+ FULL */".to_string(),
        };

        assert_eq!(response.name, "FULL");
        assert_eq!(response.category, "AccessPath");
    }

    #[test]
    fn test_explain_plan_structure() {
        let plan = ExplainPlan {
            operator: "SeqScan".to_string(),
            cost: 100.0,
            rows: 1000,
            details: serde_json::json!({"table": "users"}),
            children: vec![],
        };

        assert_eq!(plan.operator, "SeqScan");
        assert_eq!(plan.cost, 100.0);
        assert_eq!(plan.rows, 1000);
    }
}
