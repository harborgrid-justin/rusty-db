use crate::parser::{SqlStatement, JoinType, OrderByClause};
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
        join_type: JoinType,
        left: Box<PlanNode>,
        right: Box<PlanNode>,
        condition: String,
    },
    Aggregate {
        input: Box<PlanNode>,
        group_by: Vec<String>,
        aggregates: Vec<AggregateExpr>,
        having: Option<String>,
    },
    Sort {
        input: Box<PlanNode>,
        order_by: Vec<OrderByClause>,
    },
    Limit {
        input: Box<PlanNode>,
        limit: usize,
        offset: Option<usize>,
    },
    Subquery {
        plan: Box<PlanNode>,
        alias: String,
    },
}

/// Aggregate expression
#[derive(Debug, Clone)]
pub struct AggregateExpr {
    pub function: AggregateFunction,
    pub column: String,
    pub alias: Option<String>,
}

/// Aggregate function types
#[derive(Debug, Clone)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    StdDev,
    Variance,
}

/// Query planner
pub struct Planner;

impl Planner {
    pub fn new() -> Self {
        Self
    }
    
    pub fn plan(&self, stmt: &SqlStatement) -> Result<PlanNode> {
        match stmt {
            SqlStatement::Select { 
                table, 
                columns, 
                filter, 
                join, 
                group_by, 
                having, 
                order_by, 
                limit 
            } => {
                // Start with table scan
                let mut plan = PlanNode::TableScan {
                    table: table.clone(),
                    columns: if columns.contains(&"*".to_string()) {
                        vec!["*".to_string()]
                    } else {
                        columns.clone()
                    },
                };
                
                // Add JOIN if present
                if let Some(join_clause) = join {
                    let right_plan = PlanNode::TableScan {
                        table: join_clause.table.clone(),
                        columns: vec!["*".to_string()],
                    };
                    
                    plan = PlanNode::Join {
                        join_type: join_clause.join_type.clone(),
                        left: Box::new(plan),
                        right: Box::new(right_plan),
                        condition: join_clause.condition.clone(),
                    };
                }
                
                // Add FILTER if present
                if let Some(pred) = filter {
                    plan = PlanNode::Filter {
                        input: Box::new(plan),
                        predicate: pred.clone(),
                    };
                }
                
                // Add GROUP BY/HAVING if present
                if !group_by.is_empty() {
                    // Extract aggregate expressions from columns
                    let aggregates = self.extract_aggregates(columns);
                    
                    plan = PlanNode::Aggregate {
                        input: Box::new(plan),
                        group_by: group_by.clone(),
                        aggregates,
                        having: having.clone(),
                    };
                }
                
                // Add ORDER BY if present
                if !order_by.is_empty() {
                    plan = PlanNode::Sort {
                        input: Box::new(plan),
                        order_by: order_by.clone(),
                    };
                }
                
                // Add LIMIT if present
                if let Some(limit_val) = limit {
                    plan = PlanNode::Limit {
                        input: Box::new(plan),
                        limit: *limit_val,
                        offset: None,
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
    
    fn extract_aggregates(&self, columns: &[String]) -> Vec<AggregateExpr> {
        let mut aggregates = Vec::new();
        
        for col in columns {
            // Simple pattern matching for aggregate functions
            if col.to_uppercase().starts_with("COUNT(") {
                aggregates.push(AggregateExpr {
                    function: AggregateFunction::Count,
                    column: col.clone(),
                    alias: None,
                });
            } else if col.to_uppercase().starts_with("SUM(") {
                aggregates.push(AggregateExpr {
                    function: AggregateFunction::Sum,
                    column: col.clone(),
                    alias: None,
                });
            } else if col.to_uppercase().starts_with("AVG(") {
                aggregates.push(AggregateExpr {
                    function: AggregateFunction::Avg,
                    column: col.clone(),
                    alias: None,
                });
            } else if col.to_uppercase().starts_with("MIN(") {
                aggregates.push(AggregateExpr {
                    function: AggregateFunction::Min,
                    column: col.clone(),
                    alias: None,
                });
            } else if col.to_uppercase().starts_with("MAX(") {
                aggregates.push(AggregateExpr {
                    function: AggregateFunction::Max,
                    column: col.clone(),
                    alias: None,
                });
            }
        }
        
        aggregates
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
            join: None,
            group_by: Vec::new(),
            having: None,
            order_by: Vec::new(),
            limit: None,
        };
        
        let plan = planner.plan(&stmt).unwrap();
        matches!(plan, PlanNode::TableScan { .. });
    }
}
