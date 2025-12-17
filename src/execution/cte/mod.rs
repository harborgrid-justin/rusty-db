// # Common Table Expressions (CTE) Support
//
// This module provides comprehensive support for CTEs including:
// - Non-recursive CTEs (WITH clause)
// - Recursive CTEs (WITH RECURSIVE)
// - Multiple CTEs in a single query
// - CTE materialization and optimization

mod core;
mod dependency;
mod optimizer;
mod statistics;

// Re-export public types
pub use core::{CteContext, CteDefinition, CtePlanNode, CycleDetector, RecursiveCteEvaluator};
pub use dependency::CteDependencyGraph;
pub use optimizer::{
    CteOptimizer, CteReferenceTracker, CteRewriteRules, MaterializationStrategy,
    MaterializationStrategySelector, NestedCteHandler,
};
pub use statistics::{CteDetail, CteStatistics, CteStatisticsReport};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::{planner::PlanNode, QueryResult};
    use crate::parser::JoinType;

    #[test]
    fn test_cte_context() {
        let mut context = CteContext::new();

        let cte = CteDefinition {
            name: "test_cte".to_string(),
            columns: vec!["id".to_string(), "name".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "users".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };

        assert!(context.register_cte(cte.clone()).is_ok());
        assert!(context.is_cte("test_cte"));
        assert!(!context.is_cte("other_cte"));

        // Test duplicate registration
        assert!(context.register_cte(cte).is_err());
    }

    #[test]
    fn test_cte_materialization() {
        let mut context = CteContext::new();

        let result = QueryResult::new(
            vec!["id".to_string(), "name".to_string()],
            vec![
                vec!["1".to_string(), "Alice".to_string()],
                vec!["2".to_string(), "Bob".to_string()],
            ],
        );

        context.materialize("test_cte".to_string(), result)?;

        let materialized = context.get_materialized("test_cte");
        assert!(materialized.is_some());
        assert_eq!(materialized.unwrap().rows.len(), 2);
    }

    #[test]
    fn test_recursive_cte_evaluator() {
        let evaluator = RecursiveCteEvaluator::new();

        let base_result = QueryResult::new(
            vec!["id".to_string(), "value".to_string()],
            vec![vec!["1".to_string(), "10".to_string()]],
        );

        let recursive_plan = PlanNode::TableScan {
            table: "test_cte".to_string(),
            columns: vec!["*".to_string()],
        };

        let result = evaluator.evaluate("test_cte", base_result, &recursive_plan);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cte_optimizer_materialization() {
        let cte_recursive = CteDefinition {
            name: "recursive_cte".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "test".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: true,
        };

        assert!(CteOptimizer::should_materialize(&cte_recursive, 1));

        let cte_multi_ref = CteDefinition {
            name: "multi_ref_cte".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "test".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };

        assert!(CteOptimizer::should_materialize(&cte_multi_ref, 2));
    }

    #[test]
    fn test_cte_reference_tracker() {
        let mut tracker = CteReferenceTracker::new();
        let mut context = CteContext::new();

        let cte = CteDefinition {
            name: "test_cte".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "users".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };

        context.register_cte(cte).unwrap();

        let plan = PlanNode::Join {
            join_type: JoinType::Inner,
            left: Box::new(PlanNode::TableScan {
                table: "test_cte".to_string(),
                columns: vec!["*".to_string()],
            }),
            right: Box::new(PlanNode::TableScan {
                table: "test_cte".to_string(),
                columns: vec!["*".to_string()],
            }),
            condition: "true".to_string(),
        };

        tracker.track_plan(&plan, &context);
        assert_eq!(tracker.get_reference_count("test_cte"), 2);
    }

    #[test]
    fn test_cycle_detector() {
        let mut detector = CycleDetector::new();

        let rows = vec![
            vec!["1".to_string(), "A".to_string()],
            vec!["2".to_string(), "B".to_string()],
        ];

        assert!(!detector.has_cycle(&rows));
        detector.add_rows(&rows);
        assert!(detector.has_cycle(&rows));

        detector.clear();
        assert!(!detector.has_cycle(&rows));
    }

    #[test]
    fn test_cte_dependency_graph() {
        let mut graph = CteDependencyGraph::new();

        let cte1 = CteDefinition {
            name: "cte1".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "base_table".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };

        let cte2 = CteDefinition {
            name: "cte2".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "cte1".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };

        graph.build(&[cte1.clone(), cte2.clone()]);

        let sorted = graph.topological_sort(&[cte1, cte2]);
        assert!(sorted.is_ok());

        let order = sorted.unwrap();
        assert!(order.len() >= 2);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = CteDependencyGraph::new();

        let cte1 = CteDefinition {
            name: "cte1".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "cte2".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };

        let cte2 = CteDefinition {
            name: "cte2".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "cte1".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };

        graph.build(&[cte1.clone(), cte2.clone()]);
        assert!(graph.has_circular_dependency());

        let result = graph.topological_sort(&[cte1, cte2]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cte_statistics() {
        let mut stats = CteStatistics::new();

        stats.record_execution("test_cte", 100, 50, 1024);
        stats.record_execution("test_cte", 150, 50, 1024);

        let avg_time = stats.get_average_execution_time("test_cte");
        assert_eq!(avg_time, Some(125.0));

        let total_memory = stats.get_total_memory_usage();
        assert_eq!(total_memory, 1024);

        let report = stats.generate_report();
        assert_eq!(report.total_ctes, 1);
        assert_eq!(report.total_executions, 2);
    }

    #[test]
    fn test_nested_cte_handler() {
        let mut handler = NestedCteHandler::new();

        assert_eq!(handler.get_nesting_level(), 0);

        assert!(handler.enter_nesting().is_ok());
        assert_eq!(handler.get_nesting_level(), 1);

        handler.exit_nesting();
        assert_eq!(handler.get_nesting_level(), 0);
    }

    #[test]
    fn test_max_nesting_level() {
        let mut handler = NestedCteHandler::with_max_nesting(2);

        assert!(handler.enter_nesting().is_ok());
        assert!(handler.enter_nesting().is_ok());

        let result = handler.enter_nesting();
        assert!(result.is_err());
    }

    #[test]
    fn test_eliminate_unused_ctes() {
        let used_cte = CteDefinition {
            name: "used".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "base".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };

        let unused_cte = CteDefinition {
            name: "unused".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "base".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };

        let main_query = PlanNode::TableScan {
            table: "used".to_string(),
            columns: vec!["*".to_string()],
        };

        let ctes = vec![used_cte, unused_cte];
        let filtered = CteRewriteRules::eliminate_unused(ctes, &main_query);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "used");
    }

    #[test]
    fn test_materialization_strategy_selector() {
        let selector = MaterializationStrategySelector::new();

        let recursive_cte = CteDefinition {
            name: "recursive".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "test".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: true,
        };

        let strategy = selector.select_strategy(&recursive_cte, 1, 100.0);
        assert_eq!(strategy, MaterializationStrategy::AlwaysMaterialize);

        let simple_cte = CteDefinition {
            name: "simple".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "test".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };

        let strategy = selector.select_strategy(&simple_cte, 1, 100.0);
        assert_eq!(strategy, MaterializationStrategy::AlwaysInline);
    }
}
