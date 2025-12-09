// CTE optimization logic

use std::collections::HashMap;
use crate::error::DbError;
use crate::execution::planner::PlanNode;
use super::core::{CteDefinition, CteContext};

// CTE reference tracker - tracks how many times each CTE is referenced
pub struct CteReferenceTracker {
    references: HashMap<String, usize>,
}

impl CteReferenceTracker {
    pub fn new() -> Self {
        Self {
            references: HashMap::new(),
        }
    }

    pub fn track_plan(&mut self, plan: &PlanNode, cte_context: &CteContext) {
        match plan {
            PlanNode::TableScan { table, .. } => {
                if cte_context.is_cte(table) {
                    *self.references.entry(table.clone()).or_insert(0) += 1;
                }
            }
            PlanNode::Filter { input, .. }
            | PlanNode::Project { input, .. }
            | PlanNode::Limit { input, .. }
            | PlanNode::Sort { input, .. } => {
                self.track_plan(input, cte_context);
            }
            PlanNode::Join { left, right, .. } => {
                self.track_plan(left, cte_context);
                self.track_plan(right, cte_context);
            }
            PlanNode::Aggregate { input, .. } => {
                self.track_plan(input, cte_context);
            }
            PlanNode::Subquery { plan, .. } => {
                self.track_plan(plan, cte_context);
            }
        }
    }

    pub fn get_reference_count(&self, cte_name: &str) -> usize {
        self.references.get(cte_name).copied().unwrap_or(0)
    }
}

// CTE optimizer - optimizes CTE execution strategy
pub struct CteOptimizer;

impl CteOptimizer {
    // Decide whether to materialize or inline a CTE
    pub fn should_materialize(cte: &CteDefinition, reference_count: usize) -> bool {
        if cte.recursive {
            return true;
        }

        if reference_count > 1 {
            return true;
        }

        Self::is_expensive_query(&cte.query)
    }

    fn is_expensive_query(plan: &PlanNode) -> bool {
        match plan {
            PlanNode::Join { .. } => true,
            PlanNode::Aggregate { .. } => true,
            PlanNode::Sort { .. } => true,
            PlanNode::Filter { input, .. }
            | PlanNode::Project { input, .. }
            | PlanNode::Limit { input, .. } => Self::is_expensive_query(input),
            _ => false,
        }
    }

    pub fn optimize_execution_order(ctes: &[CteDefinition]) -> Vec<String> {
        ctes.iter().map(|cte| cte.name.clone()).collect()
    }
}

// CTE rewrite rules for optimization
pub struct CteRewriteRules;

impl CteRewriteRules {
    pub fn merge_ctes(ctes: Vec<CteDefinition>) -> Vec<CteDefinition> {
        ctes
    }

    pub fn push_predicates(_cte: &mut CteDefinition, _predicate: &str) {
        // Implementation would push filter predicates into CTE definitions
    }

    pub fn eliminate_unused(
        ctes: Vec<CteDefinition>,
        main_query: &PlanNode,
    ) -> Vec<CteDefinition> {
        let mut tracker = CteReferenceTracker::new();
        let mut temp_context = CteContext::new();

        for cte in &ctes {
            let _ = temp_context.register_cte(cte.clone());
        }

        tracker.track_plan(main_query, &temp_context);

        ctes.into_iter()
            .filter(|cte| tracker.get_reference_count(&cte.name) > 0)
            .collect()
    }
}

// Advanced CTE materialization strategies
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaterializationStrategy {
    AlwaysInline,
    AlwaysMaterialize,
    CostBased,
    MultiReference,
}

// Materialization strategy selector
pub struct MaterializationStrategySelector {
    threshold_multiple_refs: usize,
    threshold_cost: f64,
}

impl MaterializationStrategySelector {
    pub fn new() -> Self {
        Self {
            threshold_multiple_refs: 2,
            threshold_cost: 1000.0,
        }
    }

    pub fn select_strategy(
        &self,
        cte: &CteDefinition,
        reference_count: usize,
        estimated_cost: f64,
    ) -> MaterializationStrategy {
        if cte.recursive {
            return MaterializationStrategy::AlwaysMaterialize;
        }

        if reference_count >= self.threshold_multiple_refs {
            return MaterializationStrategy::AlwaysMaterialize;
        }

        if estimated_cost >= self.threshold_cost {
            return MaterializationStrategy::AlwaysMaterialize;
        }

        if reference_count == 1 {
            return MaterializationStrategy::AlwaysInline;
        }

        MaterializationStrategy::CostBased
    }
}

// Nested CTE support - handles CTEs within CTEs
pub struct NestedCteHandler {
    nesting_level: usize,
    max_nesting_level: usize,
}

impl NestedCteHandler {
    pub fn new() -> Self {
        Self {
            nesting_level: 0,
            max_nesting_level: 10,
        }
    }

    pub fn with_max_nesting(max_level: usize) -> Self {
        Self {
            nesting_level: 0,
            max_nesting_level: max_level,
        }
    }

    pub fn enter_nesting(&mut self) -> Result<(), DbError> {
        if self.nesting_level >= self.max_nesting_level {
            return Err(DbError::InvalidOperation(format!(
                "Maximum CTE nesting level ({}) exceeded",
                self.max_nesting_level
            )));
        }
        self.nesting_level += 1;
        Ok(())
    }

    pub fn exit_nesting(&mut self) {
        if self.nesting_level > 0 {
            self.nesting_level -= 1;
        }
    }

    pub fn get_nesting_level(&self) -> usize {
        self.nesting_level
    }
}
