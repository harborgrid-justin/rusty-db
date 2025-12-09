use crate::error::DbError;
use crate::execution::{QueryResult, planner::PlanNode};
use crate::parser::{SqlStatement, JoinType};
use crate::catalog::{Catalog, Schema};
use crate::transaction::TransactionManager;
use std::sync::Arc;
use std::collections::HashMap;
use std::cmp::Ordering;

// Query executor
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

    // Execute SQL statement (inline for performance)
    #[inline]
    pub fn execute(&self, stmt: SqlStatement) -> Result<QueryResult, DbError> {
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
                let result_columns = if columns.contains(&"*".to_string()) {
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
            SqlStatement::CreateIndex { name: _, table, columns: _, unique: _ } => {
                // Validate table exists
                let _schema = self.catalog.get_table(&table)?;
                // Index creation would go here
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::CreateView { name: _, query: _ } => {
                // View creation would go here
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::AlterTable { name: _, action: _ } => {
                // Table alteration would go here
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::GrantPermission { permission: _, table: _, user: _ } => {
                // Permission grant would go here
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::RevokePermission { permission: _, table: _, user: _ } => {
                // Permission revoke would go here
                Ok(QueryResult::with_affected(0))
            }
        }
    }

    // Execute a query plan node (inline for performance)
    #[inline]
    pub fn execute_plan(&self, plan: PlanNode) -> Result<QueryResult, DbError> {
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

    fn execute_table_scan(&self, table: &str, columns: &[String]) -> Result<QueryResult, DbError> {
        let schema = self.catalog.get_table(table)?;

        let result_columns = if columns.contains(&"*".to_string()) {
            schema.columns.iter().map(|c| c.name.clone()).collect()
        } else {
            columns.to_vec()
        };

        // Return empty result for now - actual data scanning would go here
        Ok(QueryResult::new(result_columns, Vec::new()))
    }

    fn execute_filter(&self, input: QueryResult, predicate: &str) -> Result<QueryResult, DbError> {
        // Parse and evaluate the predicate for each row
        let filtered_rows: Vec<Vec<String>> = input.rows
            .into_iter()
            .filter(|row| self.evaluate_predicate(predicate, &input.columns, row))
            .collect();

        Ok(QueryResult::new(input.columns, filtered_rows))
    }

    /// Evaluate a predicate expression against a row
    fn evaluate_predicate(&self, predicate: &str, columns: &[String], row: &[String]) -> bool {
        let predicate = predicate.trim();

        // Handle AND conditions
        if let Some(and_pos) = self.find_logical_operator(predicate, " AND ") {
            let left = &predicate[..and_pos];
            let right = &predicate[and_pos + 5..];
            return self.evaluate_predicate(left, columns, row)
                && self.evaluate_predicate(right, columns, row);
        }

        // Handle OR conditions
        if let Some(or_pos) = self.find_logical_operator(predicate, " OR ") {
            let left = &predicate[..or_pos];
            let right = &predicate[or_pos + 4..];
            return self.evaluate_predicate(left, columns, row)
                || self.evaluate_predicate(right, columns, row);
        }

        // Handle NOT conditions
        if predicate.to_uppercase().starts_with("NOT ") {
            return !self.evaluate_predicate(&predicate[4..], columns, row);
        }

        // Handle parentheses
        if predicate.starts_with('(') && predicate.ends_with(')') {
            return self.evaluate_predicate(&predicate[1..predicate.len()-1], columns, row);
        }

        // Parse comparison operators
        self.evaluate_comparison(predicate, columns, row)
    }

    /// Find logical operator position, respecting parentheses
    fn find_logical_operator(&self, expr: &str, op: &str) -> Option<usize> {
        let mut paren_depth = 0;
        let upper = expr.to_uppercase();
        let op_upper = op.to_uppercase();

        for (i, c) in expr.chars().enumerate() {
            match c {
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                _ => {}
            }
            if paren_depth == 0 && upper[i..].starts_with(&op_upper) {
                return Some(i);
            }
        }
        None
    }

    /// Evaluate a single comparison expression
    fn evaluate_comparison(&self, expr: &str, columns: &[String], row: &[String]) -> bool {
        // Handle IS NULL / IS NOT NULL
        let upper = expr.to_uppercase();
        if upper.contains(" IS NOT NULL") {
            let col_name = expr.split_whitespace().next().unwrap_or("");
            if let Some(idx) = columns.iter().position(|c| c.eq_ignore_ascii_case(col_name)) {
                return row.get(idx).map(|v| v != "NULL" && !v.is_empty()).unwrap_or(false);
            }
            return false;
        }
        if upper.contains(" IS NULL") {
            let col_name = expr.split_whitespace().next().unwrap_or("");
            if let Some(idx) = columns.iter().position(|c| c.eq_ignore_ascii_case(col_name)) {
                return row.get(idx).map(|v| v == "NULL" || v.is_empty()).unwrap_or(true);
            }
            return true;
        }

        // Handle LIKE operator
        if upper.contains(" LIKE ") {
            let parts: Vec<&str> = expr.splitn(2, |c: char| c.to_ascii_uppercase() == 'L')
                .collect();
            if parts.len() == 2 && parts[1].to_uppercase().starts_with("IKE ") {
                let col_name = parts[0].trim();
                let pattern = parts[1][4..].trim().trim_matches('\'');
                if let Some(idx) = columns.iter().position(|c| c.eq_ignore_ascii_case(col_name)) {
                    if let Some(value) = row.get(idx) {
                        return self.match_like_pattern(value, pattern);
                    }
                }
            }
            return false;
        }

        // Handle IN operator
        if upper.contains(" IN (") {
            if let Some(in_pos) = upper.find(" IN (") {
                let col_name = expr[..in_pos].trim();
                let values_str = &expr[in_pos + 5..];
                if let Some(end_paren) = values_str.find(')') {
                    let values: Vec<&str> = values_str[..end_paren]
                        .split(',')
                        .map(|v| v.trim().trim_matches('\''))
                        .collect();
                    if let Some(idx) = columns.iter().position(|c| c.eq_ignore_ascii_case(col_name)) {
                        if let Some(row_val) = row.get(idx) {
                            return values.iter().any(|v| v.eq_ignore_ascii_case(row_val));
                        }
                    }
                }
            }
            return false;
        }

        // Handle BETWEEN operator
        if upper.contains(" BETWEEN ") && upper.contains(" AND ") {
            if let Some(between_pos) = upper.find(" BETWEEN ") {
                let col_name = expr[..between_pos].trim();
                let rest = &expr[between_pos + 9..];
                if let Some(and_pos) = rest.to_uppercase().find(" AND ") {
                    let low = rest[..and_pos].trim().trim_matches('\'');
                    let high = rest[and_pos + 5..].trim().trim_matches('\'');
                    if let Some(idx) = columns.iter().position(|c| c.eq_ignore_ascii_case(col_name)) {
                        if let Some(value) = row.get(idx) {
                            return value >= low && value <= high;
                        }
                    }
                }
            }
            return false;
        }

        // Handle standard comparison operators: >=, <=, <>, !=, =, >, <
        let operators = [(">=", "ge"), ("<=", "le"), ("<>", "ne"), ("!=", "ne"),
                         ("=", "eq"), (">", "gt"), ("<", "lt")];

        for (op, op_type) in operators {
            if let Some(pos) = expr.find(op) {
                let left = expr[..pos].trim();
                let right = expr[pos + op.len()..].trim().trim_matches('\'');

                let left_value = self.resolve_value(left, columns, row);
                let right_value = self.resolve_value(right, columns, row);

                // Try numeric comparison first
                if let (Ok(l), Ok(r)) = (left_value.parse::<f64>(), right_value.parse::<f64>()) {
                    return match op_type {
                        "eq" => (l - r).abs() < f64::EPSILON,
                        "ne" => (l - r).abs() >= f64::EPSILON,
                        "gt" => l > r,
                        "ge" => l >= r,
                        "lt" => l < r,
                        "le" => l <= r,
                        _ => false,
                    };
                }

                // Fall back to string comparison
                return match op_type {
                    "eq" => left_value.eq_ignore_ascii_case(&right_value),
                    "ne" => !left_value.eq_ignore_ascii_case(&right_value),
                    "gt" => left_value > right_value,
                    "ge" => left_value >= right_value,
                    "lt" => left_value < right_value,
                    "le" => left_value <= right_value,
                    _ => false,
                };
            }
        }

        // If no operator found, treat as boolean column
        if let Some(idx) = columns.iter().position(|c| c.eq_ignore_ascii_case(expr)) {
            if let Some(value) = row.get(idx) {
                return value == "1" || value.eq_ignore_ascii_case("true");
            }
        }

        false
    }

    /// Resolve a value - either a column reference or a literal
    fn resolve_value(&self, expr: &str, columns: &[String], row: &[String]) -> String {
        let expr = expr.trim().trim_matches('\'');

        // Check if it's a column reference
        if let Some(idx) = columns.iter().position(|c| c.eq_ignore_ascii_case(expr)) {
            return row.get(idx).cloned().unwrap_or_default();
        }

        // Handle table.column format
        if let Some(dot_pos) = expr.find('.') {
            let col_name = &expr[dot_pos + 1..];
            if let Some(idx) = columns.iter().position(|c| {
                c.eq_ignore_ascii_case(col_name) || c.eq_ignore_ascii_case(expr)
            }) {
                return row.get(idx).cloned().unwrap_or_default();
            }
        }

        // It's a literal value
        expr.to_string()
    }

    /// Match a SQL LIKE pattern (supports % and _ wildcards)
    fn match_like_pattern(&self, value: &str, pattern: &str) -> bool {
        let regex_pattern = pattern
            .replace('%', ".*")
            .replace('_', ".");

        if let Ok(re) = regex::Regex::new(&format!("^(?i){}$", regex_pattern)) {
            re.is_match(value)
        } else {
            // Fallback to simple contains check
            let simple_pattern = pattern.replace('%', "").replace('_', "");
            value.to_lowercase().contains(&simple_pattern.to_lowercase())
        }
    }

    fn execute_project(&self, input: QueryResult, columns: &[String]) -> Result<QueryResult, DbError> {
        // Project only the specified columns
        if columns.contains(&"*".to_string()) {
            return Ok(input);
        }

        // Find column indices for projection
        let column_indices: Vec<Option<usize>> = columns
            .iter()
            .map(|col| {
                // Handle table.column format
                let col_name = if let Some(dot_pos) = col.find('.') {
                    &col[dot_pos + 1..]
                } else {
                    col.as_str()
                };

                input.columns.iter().position(|c| {
                    c.eq_ignore_ascii_case(col_name) || c.eq_ignore_ascii_case(col)
                })
            })
            .collect();

        // Project each row
        let projected_rows: Vec<Vec<String>> = input.rows
            .into_iter()
            .map(|row| {
                column_indices
                    .iter()
                    .map(|idx| {
                        idx.and_then(|i| row.get(i).cloned())
                            .unwrap_or_else(|| "NULL".to_string())
                    })
                    .collect()
            })
            .collect();

        Ok(QueryResult::new(columns.to_vec(), projected_rows))
    }

    fn execute_join(
        &self,
        left: QueryResult,
        right: QueryResult,
        join_type: JoinType,
        condition: &str,
    ) -> Result<QueryResult, DbError> {
        // Combine column names from both sides
        let mut result_columns = left.columns.clone();
        result_columns.extend(right.columns.clone());

        let mut result_rows = Vec::new();

        // Helper function to check join condition
        let matches_condition = |left_row: &[String], right_row: &[String]| -> bool {
            if condition.is_empty() {
                return true; // No condition = cross join behavior
            }

            // Combine rows for evaluation
            let mut combined_row = left_row.to_vec();
            combined_row.extend(right_row.to_vec());

            self.evaluate_predicate(condition, &result_columns, &combined_row)
        };

        match join_type {
            JoinType::Inner => {
                // INNER JOIN: Only matching rows
                for left_row in &left.rows {
                    for right_row in &right.rows {
                        if matches_condition(left_row, right_row) {
                            let mut combined_row = left_row.clone();
                            combined_row.extend(right_row.clone());
                            result_rows.push(combined_row);
                        }
                    }
                }
            }
            JoinType::Left => {
                // LEFT JOIN: All left rows, matching right rows or NULLs
                for left_row in &left.rows {
                    let mut found_match = false;
                    for right_row in &right.rows {
                        if matches_condition(left_row, right_row) {
                            let mut combined_row = left_row.clone();
                            combined_row.extend(right_row.clone());
                            result_rows.push(combined_row);
                            found_match = true;
                        }
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
                        if matches_condition(left_row, right_row) {
                            let mut combined_row = left_row.clone();
                            combined_row.extend(right_row.clone());
                            result_rows.push(combined_row);
                            found_match = true;
                        }
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
                        if matches_condition(left_row, right_row) {
                            let mut combined_row = left_row.clone();
                            combined_row.extend(right_row.clone());
                            result_rows.push(combined_row);
                            found_match = true;
                            matched_right[i] = true;
                        }
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
                // CROSS JOIN: Cartesian product (condition is ignored)
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
        having: Option<&str>,
    ) -> Result<QueryResult, DbError> {
        use crate::execution::planner::AggregateFunction;

        // Helper to get column values as floats
        let get_column_values = |col: &str, rows: &[Vec<String>]| -> Vec<f64> {
            let col_idx = input.columns.iter().position(|c| {
                c.eq_ignore_ascii_case(col) || {
                    // Handle aggregate expressions like COUNT(*), SUM(column)
                    if let Some(paren_start) = col.find('(') {
                        if let Some(paren_end) = col.find(')') {
                            let inner = &col[paren_start + 1..paren_end];
                            c.eq_ignore_ascii_case(inner)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
            });

            rows.iter()
                .filter_map(|row| {
                    col_idx
                        .and_then(|idx| row.get(idx))
                        .and_then(|v| v.parse::<f64>().ok())
                })
                .collect()
        };

        // Helper to calculate aggregate value
        let calculate_aggregate = |func: &AggregateFunction, col: &str, rows: &[Vec<String>]| -> String {
            match func {
                AggregateFunction::Count => {
                    if col == "*" {
                        rows.len().to_string()
                    } else {
                        // Count non-null values
                        let col_idx = input.columns.iter().position(|c| c.eq_ignore_ascii_case(col));
                        let count = rows.iter()
                            .filter(|row| {
                                col_idx
                                    .and_then(|idx| row.get(idx))
                                    .map(|v| v != "NULL" && !v.is_empty())
                                    .unwrap_or(false)
                            })
                            .count();
                        count.to_string()
                    }
                }
                AggregateFunction::Sum => {
                    let values = get_column_values(col, rows);
                    if values.is_empty() {
                        "NULL".to_string()
                    } else {
                        let sum: f64 = values.iter().sum();
                        if sum.fract() == 0.0 {
                            (sum as i64).to_string()
                        } else {
                            format!("{:.6}", sum).trim_end_matches('0').trim_end_matches('.').to_string()
                        }
                    }
                }
                AggregateFunction::Avg => {
                    let values = get_column_values(col, rows);
                    if values.is_empty() {
                        "NULL".to_string()
                    } else {
                        let avg = values.iter().sum::<f64>() / values.len() as f64;
                        format!("{:.6}", avg).trim_end_matches('0').trim_end_matches('.').to_string()
                    }
                }
                AggregateFunction::Min => {
                    let values = get_column_values(col, rows);
                    if values.is_empty() {
                        // Try string comparison
                        let col_idx = input.columns.iter().position(|c| c.eq_ignore_ascii_case(col));
                        rows.iter()
                            .filter_map(|row| col_idx.and_then(|idx| row.get(idx)))
                            .filter(|v| *v != "NULL")
                            .min()
                            .cloned()
                            .unwrap_or_else(|| "NULL".to_string())
                    } else {
                        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
                        if min.fract() == 0.0 {
                            (min as i64).to_string()
                        } else {
                            format!("{:.6}", min).trim_end_matches('0').trim_end_matches('.').to_string()
                        }
                    }
                }
                AggregateFunction::Max => {
                    let values = get_column_values(col, rows);
                    if values.is_empty() {
                        // Try string comparison
                        let col_idx = input.columns.iter().position(|c| c.eq_ignore_ascii_case(col));
                        rows.iter()
                            .filter_map(|row| col_idx.and_then(|idx| row.get(idx)))
                            .filter(|v| *v != "NULL")
                            .max()
                            .cloned()
                            .unwrap_or_else(|| "NULL".to_string())
                    } else {
                        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                        if max.fract() == 0.0 {
                            (max as i64).to_string()
                        } else {
                            format!("{:.6}", max).trim_end_matches('0').trim_end_matches('.').to_string()
                        }
                    }
                }
                AggregateFunction::StdDev => {
                    let values = get_column_values(col, rows);
                    if values.len() < 2 {
                        "NULL".to_string()
                    } else {
                        let mean = values.iter().sum::<f64>() / values.len() as f64;
                        let variance = values.iter()
                            .map(|v| (v - mean).powi(2))
                            .sum::<f64>() / (values.len() - 1) as f64; // Sample std dev
                        let std_dev = variance.sqrt();
                        format!("{:.6}", std_dev).trim_end_matches('0').trim_end_matches('.').to_string()
                    }
                }
                AggregateFunction::Variance => {
                    let values = get_column_values(col, rows);
                    if values.len() < 2 {
                        "NULL".to_string()
                    } else {
                        let mean = values.iter().sum::<f64>() / values.len() as f64;
                        let variance = values.iter()
                            .map(|v| (v - mean).powi(2))
                            .sum::<f64>() / (values.len() - 1) as f64; // Sample variance
                        format!("{:.6}", variance).trim_end_matches('0').trim_end_matches('.').to_string()
                    }
                }
            }
        };

        // Extract column name from aggregate expression
        let extract_agg_column = |col: &str| -> &str {
            if let Some(paren_start) = col.find('(') {
                if let Some(paren_end) = col.find(')') {
                    return &col[paren_start + 1..paren_end];
                }
            }
            col
        };

        if group_by.is_empty() {
            // No grouping - single aggregate result
            let mut result_columns = Vec::new();
            let mut result_row = Vec::new();

            for agg in aggregates {
                result_columns.push(agg.column.clone());
                let col = extract_agg_column(&agg.column);
                let value = calculate_aggregate(&agg.function, col, &input.rows);
                result_row.push(value);
            }

            Ok(QueryResult::new(result_columns, vec![result_row]))
        } else {
            // Group by columns - build groups
            let mut groups: HashMap<Vec<String>, Vec<Vec<String>>> = HashMap::new();

            // Find group by column indices
            let group_indices: Vec<usize> = group_by
                .iter()
                .filter_map(|col| {
                    input.columns.iter().position(|c| c.eq_ignore_ascii_case(col))
                })
                .collect();

            // Group rows
            for row in &input.rows {
                let key: Vec<String> = group_indices
                    .iter()
                    .map(|&idx| row.get(idx).cloned().unwrap_or_else(|| "NULL".to_string()))
                    .collect();
                groups.entry(key).or_insert_with(Vec::new).push(row.clone());
            }

            // Build result columns: group by columns + aggregate columns
            let mut result_columns = group_by.to_vec();
            for agg in aggregates {
                result_columns.push(agg.column.clone());
            }

            // Build result rows
            let mut result_rows: Vec<Vec<String>> = Vec::new();

            for (key, group_rows) in groups {
                let mut result_row = key.clone();

                for agg in aggregates {
                    let col = extract_agg_column(&agg.column);
                    let value = calculate_aggregate(&agg.function, col, &group_rows);
                    result_row.push(value);
                }

                result_rows.push(result_row);
            }

            // Apply HAVING clause if present
            if let Some(having_clause) = having {
                result_rows = result_rows
                    .into_iter()
                    .filter(|row| self.evaluate_predicate(having_clause, &result_columns, row))
                    .collect();
            }

            Ok(QueryResult::new(result_columns, result_rows))
        }
    }

    fn execute_sort(
        &self,
        mut input: QueryResult,
        order_by: &[crate::parser::OrderByClause],
    ) -> Result<QueryResult, DbError> {
        if order_by.is_empty() {
            return Ok(input);
        }

        // Find column indices for sorting
        let sort_specs: Vec<(Option<usize>, bool)> = order_by
            .iter()
            .map(|clause| {
                let col_idx = input.columns.iter().position(|c| {
                    c.eq_ignore_ascii_case(&clause.column) || {
                        // Handle table.column format
                        if let Some(dot_pos) = clause.column.find('.') {
                            c.eq_ignore_ascii_case(&clause.column[dot_pos + 1..])
                        } else {
                            false
                        }
                    }
                });
                (col_idx, clause.ascending)
            })
            .collect();

        // Sort the rows
        input.rows.sort_by(|a, b| {
            for (col_idx, ascending) in &sort_specs {
                if let Some(idx) = col_idx {
                    let left = a.get(*idx).map(|s| s.as_str()).unwrap_or("");
                    let right = b.get(*idx).map(|s| s.as_str()).unwrap_or("");

                    // Handle NULL values (NULLs sort last by default)
                    let left_is_null = left == "NULL" || left.is_empty();
                    let right_is_null = right == "NULL" || right.is_empty();

                    if left_is_null && right_is_null {
                        continue;
                    }
                    if left_is_null {
                        return if *ascending { Ordering::Greater } else { Ordering::Less };
                    }
                    if right_is_null {
                        return if *ascending { Ordering::Less } else { Ordering::Greater };
                    }

                    // Try numeric comparison first
                    let cmp = if let (Ok(l), Ok(r)) = (left.parse::<f64>(), right.parse::<f64>()) {
                        l.partial_cmp(&r).unwrap_or(Ordering::Equal)
                    } else {
                        // Fall back to case-insensitive string comparison
                        left.to_lowercase().cmp(&right.to_lowercase())
                    };

                    if cmp != Ordering::Equal {
                        return if *ascending { cmp } else { cmp.reverse() };
                    }
                }
            }
            Ordering::Equal
        });

        Ok(input)
    }

    fn execute_limit(
        &self,
        mut input: QueryResult,
        limit: usize,
        offset: Option<usize>,
    ) -> Result<QueryResult, DbError> {
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
    fn test_executor() -> Result<(), DbError> {
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
