use crate::parser::SqlStatement;
use crate::Result;

/// Query plan node
#[derive(Debug, Clone)]
pub enum PlanNode {
    TableScan {
        table: String,
        columns: Vec<String>,
    },
    Filter {
        input: Box<PlanNode>,
        predicate: String,
    },
    Project {
        input: Box<PlanNode>,
        columns: Vec<String>,
    },
    Join {
        left: Box<PlanNode>,
        right: Box<PlanNode>,
        condition: String,
    },
}

/// Query planner
pub struct Planner;

impl Planner {
    pub fn new() -> Self {
        Self
    }
    
    pub fn plan(&self, stmt: &SqlStatement) -> Result<PlanNode> {
        match stmt {
            SqlStatement::Select { table, columns, filter } => {
                let mut plan = PlanNode::TableScan {
                    table: table.clone(),
                    columns: columns.clone(),
                };
                
                if let Some(pred) = filter {
                    plan = PlanNode::Filter {
                        input: Box::new(plan),
                        predicate: pred.clone(),
                    };
                }
                
                Ok(plan)
            }
            _ => Ok(PlanNode::TableScan {
                table: "".to_string(),
                columns: Vec::new(),
            }),
        }
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_planner() {
        let planner = Planner::new();
        let stmt = SqlStatement::Select {
            table: "users".to_string(),
            columns: vec!["id".to_string(), "name".to_string()],
            filter: None,
        };
        
        let plan = planner.plan(&stmt).unwrap();
        matches!(plan, PlanNode::TableScan { .. });
    }
}
