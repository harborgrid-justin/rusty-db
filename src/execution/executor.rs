use crate::Result;
use crate::execution::{QueryResult, planner::PlanNode};
use crate::parser::{SqlStatement, JoinType};
use crate::catalog::{Catalog, Schema};
use crate::transaction::TransactionManager;
use std::sync::Arc;
use std::collections::HashMap;

/// Query executor
pub struct Executor {
    catalog: Arc<Catalog>,
    txn_manager: Arc<TransactionManager>,
}

impl Executor {
    pub fn new(catalog: Arc<Catalog>, txn_manager: Arc<TransactionManager>) -> Self {
        Self {
            catalog,
            txn_manager,
        }
    }
    
    pub fn execute(&self, stmt: SqlStatement) -> Result<QueryResult> {
        match stmt {
            SqlStatement::CreateTable { name, columns } => {
                let schema = Schema::new(name.clone(), columns);
                self.catalog.create_table(schema)?;
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::DropTable { name } => {
                self.catalog.drop_table(&name)?;
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::Select { table, columns, join, group_by, order_by, limit, .. } => {
                let schema = self.catalog.get_table(&table)?;
                
                // Simple implementation - return empty result with schema
                let mut result_columns = if columns.contains(&"*".to_string()) {
                    schema.columns.iter().map(|c| c.name.clone()).collect()
                } else {
                    columns
                };
                
                // Handle JOIN (placeholder)
                if join.is_some() {
                    // JOIN implementation would go here
                }
                
                // Handle GROUP BY (placeholder)
                if !group_by.is_empty() {
                    // Aggregation would be applied here
                }
                
                // Handle ORDER BY (placeholder)
                if !order_by.is_empty() {
                    // Sorting would be applied here
                }
                
                Ok(QueryResult::new(result_columns, Vec::new()))
            }
            SqlStatement::Insert { table, .. } => {
                // Validate table exists
                let _schema = self.catalog.get_table(&table)?;
                Ok(QueryResult::with_affected(1))
            }
            SqlStatement::Update { table, .. } => {
                // Validate table exists
                let _schema = self.catalog.get_table(&table)?;
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::Delete { table, .. } => {
                // Validate table exists
                let _schema = self.catalog.get_table(&table)?;
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::CreateIndex { name, table, columns, unique } => {
                // Validate table exists
                let _schema = self.catalog.get_table(&table)?;
                // Index creation would go here
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::CreateView { name, query } => {
                // View creation would go here
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::AlterTable { name, action } => {
                // Table alteration would go here
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::GrantPermission { permission, table, user } => {
                // Permission grant would go here
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::RevokePermission { permission, table, user } => {
                // Permission revoke would go here
                Ok(QueryResult::with_affected(0))
            }
        }
    }
    
    /// Execute a query plan node
    pub fn execute_plan(&self, plan: PlanNode) -> Result<QueryResult> {
        match plan {
            PlanNode::TableScan { table, columns } => {
                self.execute_table_scan(&table, &columns)
            }
            PlanNode::Filter { input, predicate } => {
                let input_result = self.execute_plan(*input)?;
                self.execute_filter(input_result, &predicate)
            }
            PlanNode::Project { input, columns } => {
                let input_result = self.execute_plan(*input)?;
                self.execute_project(input_result, &columns)
            }
            PlanNode::Join { join_type, left, right, condition } => {
                let left_result = self.execute_plan(*left)?;
                let right_result = self.execute_plan(*right)?;
                self.execute_join(left_result, right_result, join_type, &condition)
            }
            PlanNode::Aggregate { input, group_by, aggregates, having } => {
                let input_result = self.execute_plan(*input)?;
                self.execute_aggregate(input_result, &group_by, &aggregates, having.as_deref())
            }
            PlanNode::Sort { input, order_by } => {
                let input_result = self.execute_plan(*input)?;
                self.execute_sort(input_result, &order_by)
            }
            PlanNode::Limit { input, limit, offset } => {
                let input_result = self.execute_plan(*input)?;
                self.execute_limit(input_result, limit, offset)
            }
            PlanNode::Subquery { plan, .. } => {
                self.execute_plan(*plan)
            }
        }
    }
    
    fn execute_table_scan(&self, table: &str, columns: &[String]) -> Result<QueryResult> {
        let schema = self.catalog.get_table(table)?;
        
        let result_columns = if columns.contains(&"*".to_string()) {
            schema.columns.iter().map(|c| c.name.clone()).collect()
        } else {
            columns.to_vec()
        };
        
        // Return empty result for now - actual data scanning would go here
        Ok(QueryResult::new(result_columns, Vec::new()))
    }
    
    fn execute_filter(&self, input: QueryResult, _predicate: &str) -> Result<QueryResult> {
        // TODO: Implement actual filtering based on predicate
        // For now, just pass through the input
        Ok(input)
    }
    
    fn execute_project(&self, input: QueryResult, columns: &[String]) -> Result<QueryResult> {
        // Project only the specified columns
        if columns.contains(&"*".to_string()) {
            return Ok(input);
        }
        
        // TODO: Implement column projection
        Ok(QueryResult::new(columns.to_vec(), Vec::new()))
    }
    
    fn execute_join(
        &self,
        left: QueryResult,
        right: QueryResult,
        join_type: JoinType,
        _condition: &str,
    ) -> Result<QueryResult> {
        // Combine column names from both sides
        let mut result_columns = left.columns.clone();
        result_columns.extend(right.columns.clone());
        
        let mut result_rows = Vec::new();
        
        match join_type {
            JoinType::Inner => {
                // INNER JOIN: Only matching rows
                for left_row in &left.rows {
                    for right_row in &right.rows {
                        // TODO: Check join condition
                        let mut combined_row = left_row.clone();
                        combined_row.extend(right_row.clone());
                        result_rows.push(combined_row);
                    }
                }
            }
            JoinType::Left => {
                // LEFT JOIN: All left rows, matching right rows or NULLs
                for left_row in &left.rows {
                    let mut found_match = false;
                    for right_row in &right.rows {
                        // TODO: Check join condition
                        let mut combined_row = left_row.clone();
                        combined_row.extend(right_row.clone());
                        result_rows.push(combined_row);
                        found_match = true;
                    }
                    
                    if !found_match {
                        let mut combined_row = left_row.clone();
                        combined_row.extend(vec!["NULL".to_string(); right.columns.len()]);
                        result_rows.push(combined_row);
                    }
                }
            }
            JoinType::Right => {
                // RIGHT JOIN: All right rows, matching left rows or NULLs
                for right_row in &right.rows {
                    let mut found_match = false;
                    for left_row in &left.rows {
                        // TODO: Check join condition
                        let mut combined_row = left_row.clone();
                        combined_row.extend(right_row.clone());
                        result_rows.push(combined_row);
                        found_match = true;
                    }
                    
                    if !found_match {
                        let mut combined_row = vec!["NULL".to_string(); left.columns.len()];
                        combined_row.extend(right_row.clone());
                        result_rows.push(combined_row);
                    }
                }
            }
            JoinType::Full => {
                // FULL OUTER JOIN: All rows from both tables
                let mut matched_right = vec![false; right.rows.len()];
                
                for left_row in &left.rows {
                    let mut found_match = false;
                    for (i, right_row) in right.rows.iter().enumerate() {
                        // TODO: Check join condition
                        let mut combined_row = left_row.clone();
                        combined_row.extend(right_row.clone());
                        result_rows.push(combined_row);
                        found_match = true;
                        matched_right[i] = true;
                    }
                    
                    if !found_match {
                        let mut combined_row = left_row.clone();
                        combined_row.extend(vec!["NULL".to_string(); right.columns.len()]);
                        result_rows.push(combined_row);
                    }
                }
                
                // Add unmatched right rows
                for (i, right_row) in right.rows.iter().enumerate() {
                    if !matched_right[i] {
                        let mut combined_row = vec!["NULL".to_string(); left.columns.len()];
                        combined_row.extend(right_row.clone());
                        result_rows.push(combined_row);
                    }
                }
            }
            JoinType::Cross => {
                // CROSS JOIN: Cartesian product
                for left_row in &left.rows {
                    for right_row in &right.rows {
                        let mut combined_row = left_row.clone();
                        combined_row.extend(right_row.clone());
                        result_rows.push(combined_row);
                    }
                }
            }
        }
        
        Ok(QueryResult::new(result_columns, result_rows))
    }
    
    fn execute_aggregate(
        &self,
        input: QueryResult,
        group_by: &[String],
        aggregates: &[crate::execution::planner::AggregateExpr],
        _having: Option<&str>,
    ) -> Result<QueryResult> {
        use crate::execution::planner::AggregateFunction;
        
        if group_by.is_empty() {
            // No grouping - single aggregate result
            let mut result_columns = Vec::new();
            let mut result_row = Vec::new();
            
            for agg in aggregates {
                result_columns.push(agg.column.clone());
                
                // Calculate aggregate value
                let value = match agg.function {
                    AggregateFunction::Count => input.rows.len().to_string(),
                    AggregateFunction::Sum => "0".to_string(),  // TODO: Implement
                    AggregateFunction::Avg => "0".to_string(),  // TODO: Implement
                    AggregateFunction::Min => "0".to_string(),  // TODO: Implement
                    AggregateFunction::Max => "0".to_string(),  // TODO: Implement
                    AggregateFunction::StdDev => "0".to_string(), // TODO: Implement
                    AggregateFunction::Variance => "0".to_string(), // TODO: Implement
                };
                
                result_row.push(value);
            }
            
            Ok(QueryResult::new(result_columns, vec![result_row]))
        } else {
            // Group by columns
            let mut _groups: HashMap<Vec<String>, Vec<Vec<String>>> = HashMap::new();
            
            // TODO: Implement grouping logic
            // For now, return empty result
            Ok(QueryResult::new(group_by.to_vec(), Vec::new()))
        }
    }
    
    fn execute_sort(
        &self,
        input: QueryResult,
        _order_by: &[crate::parser::OrderByClause],
    ) -> Result<QueryResult> {
        // TODO: Implement actual sorting based on order_by clauses
        // For now, just return the input as-is
        Ok(input)
    }
    
    fn execute_limit(
        &self,
        mut input: QueryResult,
        limit: usize,
        offset: Option<usize>,
    ) -> Result<QueryResult> {
        let start = offset.unwrap_or(0);
        
        input.rows = input.rows.into_iter()
            .skip(start)
            .take(limit)
            .collect();
        
        Ok(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SqlParser;
    
    #[test]
    fn test_executor() -> Result<()> {
        let catalog = Arc::new(Catalog::new());
        let txn_manager = Arc::new(TransactionManager::new());
        let executor = Executor::new(catalog, txn_manager);
        
        let parser = SqlParser::new();
        let stmts = parser.parse("CREATE TABLE users (id INT, name VARCHAR(255))")?;
        
        let result = executor.execute(stmts[0].clone())?;
        assert_eq!(result.rows_affected, 0);
        
        Ok(())
    }
}
