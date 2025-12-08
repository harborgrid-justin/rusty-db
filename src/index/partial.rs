/// Partial and Expression Indexes
///
/// This module provides advanced indexing capabilities:
/// - Partial indexes with filter predicates
/// - Function-based/expression indexes
/// - Computed column indexes
/// - Index-only scans support

use crate::Result;
use crate::error::DbError;
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// Partial Index
///
/// Indexes only rows that satisfy a predicate condition
pub struct PartialIndex<K: Ord + Clone, V: Clone> {
    name: String,
    /// The underlying index structure
    index: Arc<RwLock<BTreeMap<K, Vec<V>>>>,
    /// Predicate that determines which rows to index
    predicate: Arc<Predicate>,
    /// Statistics
    stats: Arc<RwLock<PartialIndexStats>>,
}

impl<K: Ord + Clone, V: Clone> PartialIndex<K, V> {
    /// Create a new partial index
    pub fn new(name: String, predicate: Predicate) -> Self {
        Self {
            name,
            index: Arc::new(RwLock::new(BTreeMap::new())),
            predicate: Arc::new(predicate),
            stats: Arc::new(RwLock::new(PartialIndexStats::default())),
        }
    }

    /// Insert a key-value pair if it satisfies the predicate
    pub fn insert(&self, key: K, value: V, row_data: &RowData) -> Result<bool> {
        if self.predicate.evaluate(row_data)? {
            let mut index = self.index.write();
            index.entry(key).or_insert_with(Vec::new).push(value);

            let mut stats = self.stats.write();
            stats.total_entries += 1;

            Ok(true)
        } else {
            let mut stats = self.stats.write();
            stats.filtered_entries += 1;
            Ok(false)
        }
    }

    /// Search for a key
    pub fn search(&self, key: &K) -> Result<Vec<V>> {
        let index = self.index.read();
        Ok(index.get(key).cloned().unwrap_or_default())
    }

    /// Range search
    pub fn range_search(&self, start: &K, end: &K) -> Result<Vec<V>> {
        let index = self.index.read();
        let mut results = Vec::new();

        for (_, values) in index.range(start.clone()..=end.clone()) {
            results.extend(values.clone());
        }

        Ok(results)
    }

    /// Delete a key-value pair
    pub fn delete(&self, key: &K, value: &V) -> Result<bool>
    where
        V: PartialEq,
    {
        let mut index = self.index.write();

        if let Some(values) = index.get_mut(key) {
            let initial_len = values.len();
            values.retain(|v| v != value);

            if values.is_empty() {
                index.remove(key);
            }

            let deleted = values.len() < initial_len;
            if deleted {
                let mut stats = self.stats.write();
                stats.total_entries -= 1;
            }

            Ok(deleted)
        } else {
            Ok(false)
        }
    }

    /// Get statistics
    pub fn stats(&self) -> PartialIndexStats {
        self.stats.read().clone()
    }

    /// Get the predicate
    pub fn predicate(&self) -> &Predicate {
        &self.predicate
    }
}

/// Expression Index
///
/// Indexes computed values based on an expression
pub struct ExpressionIndex<V: Clone> {
    name: String,
    /// The underlying index structure (maps computed values to row IDs)
    index: Arc<RwLock<BTreeMap<ComputedValue, Vec<V>>>>,
    /// Expression to compute indexed values
    expression: Arc<Expression>,
    /// Statistics
    stats: Arc<RwLock<ExpressionIndexStats>>,
}

impl<V: Clone> ExpressionIndex<V> {
    /// Create a new expression index
    pub fn new(name: String, expression: Expression) -> Self {
        Self {
            name,
            index: Arc::new(RwLock::new(BTreeMap::new())),
            expression: Arc::new(expression),
            stats: Arc::new(RwLock::new(ExpressionIndexStats::default())),
        }
    }

    /// Insert a row by computing the expression
    pub fn insert(&self, row_data: &RowData, row_id: V) -> Result<()> {
        let computed_value = self.expression.evaluate(row_data)?;

        let mut index = self.index.write();
        index
            .entry(computed_value)
            .or_insert_with(Vec::new)
            .push(row_id);

        let mut stats = self.stats.write();
        stats.total_entries += 1;
        stats.total_computations += 1;

        Ok(())
    }

    /// Search for a computed value
    pub fn search(&self, value: &ComputedValue) -> Result<Vec<V>> {
        let index = self.index.read();
        Ok(index.get(value).cloned().unwrap_or_default())
    }

    /// Search using a row's data (computes the expression)
    pub fn search_by_row(&self, row_data: &RowData) -> Result<Vec<V>> {
        let computed_value = self.expression.evaluate(row_data)?;
        self.search(&computed_value)
    }

    /// Range search on computed values
    pub fn range_search(&self, start: &ComputedValue, end: &ComputedValue) -> Result<Vec<V>> {
        let index = self.index.read();
        let mut results = Vec::new();

        for (_, values) in index.range(start.clone()..=end.clone()) {
            results.extend(values.clone());
        }

        Ok(results)
    }

    /// Get statistics
    pub fn stats(&self) -> ExpressionIndexStats {
        self.stats.read().clone()
    }

    /// Get the expression
    pub fn expression(&self) -> &Expression {
        &self.expression
    }
}

/// Covering Index
///
/// An index that includes all columns needed for a query (enables index-only scans)
pub struct CoveringIndex<K: Ord + Clone> {
    name: String,
    /// Indexed columns (key)
    indexed_columns: Vec<String>,
    /// Included columns (stored with index for index-only scans)
    included_columns: Vec<String>,
    /// The index structure
    index: Arc<RwLock<BTreeMap<K, Vec<CoveringEntry>>>>,
}

impl<K: Ord + Clone> CoveringIndex<K> {
    /// Create a new covering index
    pub fn new(
        name: String,
        indexed_columns: Vec<String>,
        included_columns: Vec<String>,
    ) -> Self {
        Self {
            name,
            indexed_columns,
            included_columns,
            index: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// Insert a row
    pub fn insert(&self, key: K, row_id: u64, included_values: Vec<ColumnValue>) -> Result<()> {
        let mut index = self.index.write();

        index.entry(key).or_insert_with(Vec::new).push(CoveringEntry {
            row_id,
            included_values,
        });

        Ok(())
    }

    /// Search and return both row IDs and included column values
    pub fn search_covering(&self, key: &K) -> Result<Vec<CoveringEntry>> {
        let index = self.index.read();
        Ok(index.get(key).cloned().unwrap_or_default())
    }

    /// Check if this index can cover a query
    pub fn can_cover(&self, required_columns: &[String]) -> bool {
        let all_columns: Vec<_> = self
            .indexed_columns
            .iter()
            .chain(self.included_columns.iter())
            .collect();

        required_columns
            .iter()
            .all(|col| all_columns.contains(&col))
    }
}

/// Predicate for partial indexes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Predicate {
    /// Simple comparison: column op value
    Comparison {
        column: String,
        operator: ComparisonOp,
        value: ColumnValue,
    },
    /// Logical AND
    And(Box<Predicate>, Box<Predicate>),
    /// Logical OR
    Or(Box<Predicate>, Box<Predicate>),
    /// Logical NOT
    Not(Box<Predicate>),
    /// Check if column is NULL
    IsNull(String),
    /// Check if column is NOT NULL
    IsNotNull(String),
}

impl Predicate {
    /// Evaluate predicate against row data
    pub fn evaluate(&self, row_data: &RowData) -> Result<bool> {
        match self {
            Predicate::Comparison {
                column,
                operator,
                value,
            } => {
                let row_value = row_data.get_column(column)?;
                operator.compare(&row_value, value)
            }
            Predicate::And(left, right) => {
                Ok(left.evaluate(row_data)? && right.evaluate(row_data)?)
            }
            Predicate::Or(left, right) => {
                Ok(left.evaluate(row_data)? || right.evaluate(row_data)?)
            }
            Predicate::Not(pred) => Ok(!pred.evaluate(row_data)?),
            Predicate::IsNull(column) => {
                let value = row_data.get_column(column)?;
                Ok(matches!(value, ColumnValue::Null))
            }
            Predicate::IsNotNull(column) => {
                let value = row_data.get_column(column)?;
                Ok(!matches!(value, ColumnValue::Null))
            }
        }
    }
}

/// Comparison operators
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

impl ComparisonOp {
    fn compare(&self, left: &ColumnValue, right: &ColumnValue) -> Result<bool> {
        match (left, right) {
            (ColumnValue::Integer(l), ColumnValue::Integer(r)) => Ok(match self {
                ComparisonOp::Equal => l == r,
                ComparisonOp::NotEqual => l != r,
                ComparisonOp::LessThan => l < r,
                ComparisonOp::LessThanOrEqual => l <= r,
                ComparisonOp::GreaterThan => l > r,
                ComparisonOp::GreaterThanOrEqual => l >= r,
            }),
            (ColumnValue::String(l), ColumnValue::String(r)) => Ok(match self {
                ComparisonOp::Equal => l == r,
                ComparisonOp::NotEqual => l != r,
                ComparisonOp::LessThan => l < r,
                ComparisonOp::LessThanOrEqual => l <= r,
                ComparisonOp::GreaterThan => l > r,
                ComparisonOp::GreaterThanOrEqual => l >= r,
            }),
            (ColumnValue::Null, ColumnValue::Null) => Ok(match self {
                ComparisonOp::Equal => true,
                ComparisonOp::NotEqual => false,
                _ => false,
            }),
            _ => Ok(false),
        }
    }
}

/// Expression for computed indexes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expression {
    /// Reference to a column
    Column(String),
    /// Constant value
    Constant(ColumnValue),
    /// Function call
    Function {
        name: String,
        args: Vec<Expression>,
    },
    /// Binary operation
    BinaryOp {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
}

impl Expression {
    /// Evaluate expression against row data
    pub fn evaluate(&self, row_data: &RowData) -> Result<ComputedValue> {
        match self {
            Expression::Column(name) => {
                let value = row_data.get_column(name)?;
                Ok(ComputedValue::from_column_value(value))
            }
            Expression::Constant(value) => Ok(ComputedValue::from_column_value(value.clone())),
            Expression::Function { name, args } => {
                self.evaluate_function(name, args, row_data)
            }
            Expression::BinaryOp {
                left,
                operator,
                right,
            } => {
                let left_val = left.evaluate(row_data)?;
                let right_val = right.evaluate(row_data)?;
                operator.apply(&left_val, &right_val)
            }
        }
    }

    fn evaluate_function(
        &self,
        name: &str,
        args: &[Expression],
        row_data: &RowData,
    ) -> Result<ComputedValue> {
        match name.to_uppercase().as_str() {
            "UPPER" => {
                if args.len() != 1 {
                    return Err(DbError::Internal("UPPER requires 1 argument".into()));
                }
                let arg = args[0].evaluate(row_data)?;
                match arg {
                    ComputedValue::String(s) => Ok(ComputedValue::String(s.to_uppercase())),
                    _ => Err(DbError::Internal("UPPER requires string argument".into())),
                }
            }
            "LOWER" => {
                if args.len() != 1 {
                    return Err(DbError::Internal("LOWER requires 1 argument".into()));
                }
                let arg = args[0].evaluate(row_data)?;
                match arg {
                    ComputedValue::String(s) => Ok(ComputedValue::String(s.to_lowercase())),
                    _ => Err(DbError::Internal("LOWER requires string argument".into())),
                }
            }
            "ABS" => {
                if args.len() != 1 {
                    return Err(DbError::Internal("ABS requires 1 argument".into()));
                }
                let arg = args[0].evaluate(row_data)?;
                match arg {
                    ComputedValue::Integer(i) => Ok(ComputedValue::Integer(i.abs())),
                    _ => Err(DbError::Internal("ABS requires numeric argument".into())),
                }
            }
            _ => Err(DbError::Internal(format!("Unknown function: {}", name))),
        }
    }
}

/// Binary operators for expressions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Concat,
}

impl BinaryOperator {
    fn apply(&self, left: &ComputedValue, right: &ComputedValue) -> Result<ComputedValue> {
        match (left, right) {
            (ComputedValue::Integer(l), ComputedValue::Integer(r)) => {
                Ok(ComputedValue::Integer(match self {
                    BinaryOperator::Add => l + r,
                    BinaryOperator::Subtract => l - r,
                    BinaryOperator::Multiply => l * r,
                    BinaryOperator::Divide => {
                        if *r == 0 {
                            return Err(DbError::Internal("Division by zero".into()));
                        }
                        l / r
                    }
                    BinaryOperator::Concat => {
                        return Err(DbError::Internal("Cannot concat integers".into()))
                    }
                }))
            }
            (ComputedValue::String(l), ComputedValue::String(r)) => match self {
                BinaryOperator::Concat => Ok(ComputedValue::String(format!("{}{}", l, r))),
                _ => Err(DbError::Internal("Invalid operation on strings".into())),
            },
            _ => Err(DbError::Internal("Type mismatch in binary operation".into())),
        }
    }
}

/// Row data for predicate/expression evaluation
#[derive(Debug, Clone)]
pub struct RowData {
    columns: std::collections::HashMap<String, ColumnValue>,
}

impl RowData {
    pub fn new() -> Self {
        Self {
            columns: std::collections::HashMap::new(),
        }
    }

    pub fn set_column(&mut self, name: String, value: ColumnValue) {
        self.columns.insert(name, value);
    }

    pub fn get_column(&self, name: &str) -> Result<ColumnValue> {
        self.columns
            .get(name)
            .cloned()
            .ok_or_else(|| DbError::Internal(format!("Column not found: {}", name)))
    }
}

/// Column value
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ColumnValue {
    Null,
    Integer(i64),
    String(String),
    Boolean(bool),
}

/// Computed value (result of expression evaluation)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComputedValue {
    Null,
    Integer(i64),
    String(String),
    Boolean(bool),
}

impl ComputedValue {
    fn from_column_value(value: ColumnValue) -> Self {
        match value {
            ColumnValue::Null => ComputedValue::Null,
            ColumnValue::Integer(i) => ComputedValue::Integer(i),
            ColumnValue::String(s) => ComputedValue::String(s),
            ColumnValue::Boolean(b) => ComputedValue::Boolean(b),
        }
    }
}

/// Entry in covering index
#[derive(Debug, Clone)]
pub struct CoveringEntry {
    pub row_id: u64,
    pub included_values: Vec<ColumnValue>,
}

/// Partial index statistics
#[derive(Debug, Clone, Default)]
pub struct PartialIndexStats {
    pub total_entries: usize,
    pub filtered_entries: usize,
}

impl PartialIndexStats {
    pub fn selectivity(&self) -> f64 {
        let total = self.total_entries + self.filtered_entries;
        if total == 0 {
            0.0
        } else {
            self.total_entries as f64 / total as f64
        }
    }
}

/// Expression index statistics
#[derive(Debug, Clone, Default)]
pub struct ExpressionIndexStats {
    pub total_entries: usize,
    pub total_computations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partial_index() {
        let predicate = Predicate::Comparison {
            column: "status".to_string(),
            operator: ComparisonOp::Equal,
            value: ColumnValue::String("active".to_string()),
        };

        let index: PartialIndex<i64, u64> = PartialIndex::new("idx_active".to_string(), predicate);

        let mut row1 = RowData::new();
        row1.set_column("status".to_string(), ColumnValue::String("active".to_string()));

        let mut row2 = RowData::new();
        row2.set_column("status".to_string(), ColumnValue::String("inactive".to_string()));

        // Insert active row - should be indexed
        assert!(index.insert(1, 100, &row1).unwrap());

        // Insert inactive row - should not be indexed
        assert!(!index.insert(2, 200, &row2).unwrap());

        // Search should find the active row
        let results = index.search(&1).unwrap();
        assert_eq!(results, vec![100]);

        let stats = index.stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.filtered_entries, 1);
    }

    #[test]
    fn test_expression_index() {
        let expression = Expression::Function {
            name: "UPPER".to_string(),
            args: vec![Expression::Column("email".to_string())],
        };

        let index: ExpressionIndex<u64> =
            ExpressionIndex::new("idx_upper_email".to_string(), expression);

        let mut row = RowData::new();
        row.set_column("email".to_string(), ColumnValue::String("user@example.com".to_string()));

        index.insert(&row, 100).unwrap();

        // Search using computed value
        let search_value = ComputedValue::String("USER@EXAMPLE.COM".to_string());
        let results = index.search(&search_value).unwrap();
        assert_eq!(results, vec![100]);
    }

    #[test]
    fn test_covering_index() {
        let index: CoveringIndex<i64> = CoveringIndex::new(
            "idx_covering".to_string(),
            vec!["id".to_string()],
            vec!["name".to_string(), "email".to_string()],
        );

        index
            .insert(
                1,
                100,
                vec![
                    ColumnValue::String("John".to_string()),
                    ColumnValue::String("john@example.com".to_string()),
                ],
            )
            .unwrap();

        let results = index.search_covering(&1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].row_id, 100);

        // Check if index can cover a query
        assert!(index.can_cover(&["id".to_string(), "name".to_string()]));
        assert!(!index.can_cover(&["id".to_string(), "address".to_string()]));
    }

    #[test]
    fn test_predicate_evaluation() {
        let predicate = Predicate::And(
            Box::new(Predicate::Comparison {
                column: "age".to_string(),
                operator: ComparisonOp::GreaterThan,
                value: ColumnValue::Integer(18),
            }),
            Box::new(Predicate::Comparison {
                column: "status".to_string(),
                operator: ComparisonOp::Equal,
                value: ColumnValue::String("active".to_string()),
            }),
        );

        let mut row = RowData::new();
        row.set_column("age".to_string(), ColumnValue::Integer(25));
        row.set_column("status".to_string(), ColumnValue::String("active".to_string()));

        assert!(predicate.evaluate(&row).unwrap());
    }

    #[test]
    fn test_expression_binary_op() {
        let expression = Expression::BinaryOp {
            left: Box::new(Expression::Column("quantity".to_string())),
            operator: BinaryOperator::Multiply,
            right: Box::new(Expression::Column("price".to_string())),
        };

        let mut row = RowData::new();
        row.set_column("quantity".to_string(), ColumnValue::Integer(5));
        row.set_column("price".to_string(), ColumnValue::Integer(10));

        let result = expression.evaluate(&row).unwrap();
        assert_eq!(result, ComputedValue::Integer(50));
    }
}


