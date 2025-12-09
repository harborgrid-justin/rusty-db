// CTE dependency graph management

use std::collections::{HashMap, HashSet};
use crate::error::DbError;
use crate::execution::planner::PlanNode;
use super::core::CteDefinition;

// CTE dependency graph for optimization
pub struct CteDependencyGraph {
    dependencies: HashMap<String, Vec<String>>,
}

impl CteDependencyGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }

    pub fn build(&mut self, ctes: &[CteDefinition]) {
        for cte in ctes {
            let deps = self.extract_dependencies(&cte.query);
            self.dependencies.insert(cte.name.clone(), deps);
        }
    }

    fn extract_dependencies(&self, plan: &PlanNode) -> Vec<String> {
        let mut deps = Vec::new();
        self.extract_deps_recursive(plan, &mut deps);
        deps
    }

    fn extract_deps_recursive(&self, plan: &PlanNode, deps: &mut Vec<String>) {
        match plan {
            PlanNode::TableScan { table, .. } => {
                if !deps.contains(table) {
                    deps.push(table.clone());
                }
            }
            PlanNode::Filter { input, .. }
            | PlanNode::Project { input, .. }
            | PlanNode::Sort { input, .. }
            | PlanNode::Limit { input, .. }
            | PlanNode::Aggregate { input, .. } => {
                self.extract_deps_recursive(input, deps);
            }
            PlanNode::Join { left, right, .. } => {
                self.extract_deps_recursive(left, deps);
                self.extract_deps_recursive(right, deps);
            }
            PlanNode::Subquery { plan, .. } => {
                self.extract_deps_recursive(plan, deps);
            }
        }
    }

    pub fn topological_sort(&self, ctes: &[CteDefinition]) -> Result<Vec<String>, DbError> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut in_progress = HashSet::new();

        for cte in ctes {
            if !visited.contains(&cte.name) {
                self.visit(&cte.name, &mut visited, &mut in_progress, &mut sorted)?;
            }
        }

        sorted.reverse();
        Ok(sorted)
    }

    fn visit(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        in_progress: &mut HashSet<String>,
        sorted: &mut Vec<String>,
    ) -> Result<(), DbError> {
        if in_progress.contains(name) {
            return Err(DbError::InvalidOperation(format!(
                "Circular dependency detected in CTE '{}'",
                name
            )));
        }

        if visited.contains(name) {
            return Ok(());
        }

        in_progress.insert(name.to_string());

        if let Some(deps) = self.dependencies.get(name) {
            for dep in deps {
                self.visit(dep, visited, in_progress, sorted)?;
            }
        }

        in_progress.remove(name);
        visited.insert(name.to_string());
        sorted.push(name.to_string());

        Ok(())
    }

    pub fn has_circular_dependency(&self) -> bool {
        let mut visited = HashSet::new();
        let mut in_progress = HashSet::new();

        for name in self.dependencies.keys() {
            if !visited.contains(name) {
                if self.has_cycle_dfs(name, &mut visited, &mut in_progress) {
                    return true;
                }
            }
        }
        false
    }

    fn has_cycle_dfs(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        in_progress: &mut HashSet<String>,
    ) -> bool {
        if in_progress.contains(name) {
            return true;
        }

        if visited.contains(name) {
            return false;
        }

        in_progress.insert(name.to_string());

        if let Some(deps) = self.dependencies.get(name) {
            for dep in deps {
                if self.has_cycle_dfs(dep, visited, in_progress) {
                    return true;
                }
            }
        }

        in_progress.remove(name);
        visited.insert(name.to_string());

        false
    }
}
