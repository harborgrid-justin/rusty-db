// # Expression Parser and Evaluator
//
// Handles parsing and evaluation of SQL expressions including:
// - CASE expressions
// - BETWEEN predicates
// - IN lists
// - NULL checks
// - Pattern matching (LIKE)
// - Mathematical and logical operations

use crate::error::DbError;
use crate::Result;
use std::collections::HashMap;

/// Represents a SQL expression that can be evaluated
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Column reference
    Column(String),

    /// Literal value
    Literal(LiteralValue),

    /// Binary operation (e.g., a + b, a > b)
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },

    /// Unary operation (e.g., NOT a, -a)
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expression>,
    },

    /// CASE expression
    Case {
        operand: Option<Box<Expression>>,
        conditions: Vec<(Expression, Expression)>,
        else_result: Option<Box<Expression>>,
    },

    /// BETWEEN predicate
    Between {
        expr: Box<Expression>,
        low: Box<Expression>,
        high: Box<Expression>,
        negated: bool,
    },

    /// IN predicate
    In {
        expr: Box<Expression>,
        list: Vec<Expression>,
        negated: bool,
    },

    /// IS NULL / IS NOT NULL
    IsNull {
        expr: Box<Expression>,
        negated: bool,
    },

    /// LIKE pattern matching
    Like {
        expr: Box<Expression>,
        pattern: Box<Expression>,
        escape: Option<Box<Expression>>,
        negated: bool,
    },

    /// Function call
    Function {
        name: String,
        args: Vec<Expression>,
    },

    /// Subquery (for EXISTS, IN, etc.)
    Subquery(String),
}

/// Literal value types
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Date(String),
    Timestamp(String),
}

impl LiteralValue {
    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self {
            LiteralValue::Null => "NULL".to_string(),
            LiteralValue::Boolean(b) => b.to_string(),
            LiteralValue::Integer(i) => i.to_string(),
            LiteralValue::Float(f) => f.to_string(),
            LiteralValue::String(s) => s.clone(),
            LiteralValue::Date(d) => d.clone(),
            LiteralValue::Timestamp(t) => t.clone(),
        }
    }

    /// Try to convert to a specific type
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            LiteralValue::Boolean(b) => Some(*b),
            LiteralValue::Integer(i) => Some(*i != 0),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            LiteralValue::Integer(i) => Some(*i),
            LiteralValue::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            LiteralValue::Float(f) => Some(*f),
            LiteralValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,

    // Logical
    And,
    Or,

    // String
    Concat,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Not,
    Negate,
    Plus,
}

/// Expression evaluator
pub struct ExpressionEvaluator {
    /// Row data for column lookups
    row_data: HashMap<String, LiteralValue>,
}

impl ExpressionEvaluator {
    /// Create a new evaluator with the given row data
    pub fn new(row_data: HashMap<String, LiteralValue>) -> Self {
        Self { row_data }
    }

    /// Evaluate an expression to a literal value
    pub fn evaluate(&self, expr: &Expression) -> Result<LiteralValue> {
        match expr {
            Expression::Column(name) => {
                self.row_data
                    .get(name)
                    .cloned()
                    .ok_or_else(|| DbError::Execution(format!("Column {} not found", name)))
            }

            Expression::Literal(value) => Ok(value.clone()),

            Expression::BinaryOp { left, op, right } => {
                let left_val = self.evaluate(left)?;
                let right_val = self.evaluate(right)?;
                self.evaluate_binary_op(&left_val, *op, &right_val)
            }

            Expression::UnaryOp { op, expr } => {
                let val = self.evaluate(expr)?;
                self.evaluate_unary_op(*op, &val)
            }

            Expression::Case { operand, conditions, else_result } => {
                self.evaluate_case(operand.as_deref(), conditions, else_result.as_deref())
            }

            Expression::Between { expr, low, high, negated } => {
                let val = self.evaluate(expr)?;
                let low_val = self.evaluate(low)?;
                let high_val = self.evaluate(high)?;

                let result = self.compare_values(&val, &low_val)? >= 0
                    && self.compare_values(&val, &high_val)? <= 0;

                Ok(LiteralValue::Boolean(if *negated { !result } else { result }))
            }

            Expression::In { expr, list, negated } => {
                let val = self.evaluate(expr)?;
                let mut found = false;

                for item in list {
                    let item_val = self.evaluate(item)?;
                    if self.values_equal(&val, &item_val) {
                        found = true;
                        break;
                    }
                }

                Ok(LiteralValue::Boolean(if *negated { !found } else { found }))
            }

            Expression::IsNull { expr, negated } => {
                let val = self.evaluate(expr)?;
                let is_null = matches!(val, LiteralValue::Null);
                Ok(LiteralValue::Boolean(if *negated { !is_null } else { is_null }))
            }

            Expression::Like { expr, pattern, escape, negated } => {
                let val = self.evaluate(expr)?;
                let pattern_val = self.evaluate(pattern)?;

                let text = val.to_string();
                let pattern_str = pattern_val.to_string();

                let matches = self.match_like_pattern(&text, &pattern_str);
                Ok(LiteralValue::Boolean(if *negated { !matches } else { matches }))
            }

            Expression::Function { name, args } => {
                self.evaluate_function(name, args)
            }

            Expression::Subquery(_) => {
                // Subqueries would need the full executor context
                Err(DbError::Execution("Subquery evaluation not implemented in expression context".to_string()))
            }
        }
    }

    /// Evaluate a CASE expression
    fn evaluate_case(
        &self,
        operand: Option<&Expression>,
        conditions: &[(Expression, Expression)],
        else_result: Option<&Expression>,
    ) -> Result<LiteralValue> {
        match operand {
            Some(op_expr) => {
                // Simple CASE: CASE operand WHEN value THEN result ... END
                let op_val = self.evaluate(op_expr)?;

                for (when_expr, then_expr) in conditions {
                    let when_val = self.evaluate(when_expr)?;
                    if self.values_equal(&op_val, &when_val) {
                        return self.evaluate(then_expr);
                    }
                }
            }
            None => {
                // Searched CASE: CASE WHEN condition THEN result ... END
                for (when_expr, then_expr) in conditions {
                    let when_val = self.evaluate(when_expr)?;
                    if let Some(true) = when_val.as_bool() {
                        return self.evaluate(then_expr);
                    }
                }
            }
        }

        // No condition matched, use ELSE clause or NULL
        if let Some(else_expr) = else_result {
            self.evaluate(else_expr)
        } else {
            Ok(LiteralValue::Null)
        }
    }

    /// Evaluate a binary operation
    fn evaluate_binary_op(
        &self,
        left: &LiteralValue,
        op: BinaryOperator,
        right: &LiteralValue,
    ) -> Result<LiteralValue> {
        // Handle NULL values
        if matches!(left, LiteralValue::Null) || matches!(right, LiteralValue::Null) {
            return Ok(LiteralValue::Null);
        }

        match op {
            // Arithmetic operations
            BinaryOperator::Add => {
                if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                    Ok(LiteralValue::Float(l + r))
                } else {
                    Err(DbError::Execution("Type mismatch in addition".to_string()))
                }
            }
            BinaryOperator::Subtract => {
                if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                    Ok(LiteralValue::Float(l - r))
                } else {
                    Err(DbError::Execution("Type mismatch in subtraction".to_string()))
                }
            }
            BinaryOperator::Multiply => {
                if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                    Ok(LiteralValue::Float(l * r))
                } else {
                    Err(DbError::Execution("Type mismatch in multiplication".to_string()))
                }
            }
            BinaryOperator::Divide => {
                if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                    if r == 0.0 {
                        Err(DbError::Execution("Division by zero".to_string()))
                    } else {
                        Ok(LiteralValue::Float(l / r))
                    }
                } else {
                    Err(DbError::Execution("Type mismatch in division".to_string()))
                }
            }
            BinaryOperator::Modulo => {
                if let (Some(l), Some(r)) = (left.as_i64(), right.as_i64()) {
                    if r == 0 {
                        Err(DbError::Execution("Modulo by zero".to_string()))
                    } else {
                        Ok(LiteralValue::Integer(l % r))
                    }
                } else {
                    Err(DbError::Execution("Type mismatch in modulo".to_string()))
                }
            }

            // Comparison operations
            BinaryOperator::Equal => {
                Ok(LiteralValue::Boolean(self.values_equal(left, right)))
            }
            BinaryOperator::NotEqual => {
                Ok(LiteralValue::Boolean(!self.values_equal(left, right)))
            }
            BinaryOperator::LessThan => {
                Ok(LiteralValue::Boolean(self.compare_values(left, right)? < 0))
            }
            BinaryOperator::LessThanOrEqual => {
                Ok(LiteralValue::Boolean(self.compare_values(left, right)? <= 0))
            }
            BinaryOperator::GreaterThan => {
                Ok(LiteralValue::Boolean(self.compare_values(left, right)? > 0))
            }
            BinaryOperator::GreaterThanOrEqual => {
                Ok(LiteralValue::Boolean(self.compare_values(left, right)? >= 0))
            }

            // Logical operations
            BinaryOperator::And => {
                if let (Some(l), Some(r)) = (left.as_bool(), right.as_bool()) {
                    Ok(LiteralValue::Boolean(l && r))
                } else {
                    Err(DbError::Execution("Type mismatch in AND operation".to_string()))
                }
            }
            BinaryOperator::Or => {
                if let (Some(l), Some(r)) = (left.as_bool(), right.as_bool()) {
                    Ok(LiteralValue::Boolean(l || r))
                } else {
                    Err(DbError::Execution("Type mismatch in OR operation".to_string()))
                }
            }

            // String concatenation
            BinaryOperator::Concat => {
                Ok(LiteralValue::String(format!("{}{}", left.to_string(), right.to_string())))
            }
        }
    }

    /// Evaluate a unary operation
    fn evaluate_unary_op(&self, op: UnaryOperator, val: &LiteralValue) -> Result<LiteralValue> {
        match op {
            UnaryOperator::Not => {
                if let Some(b) = val.as_bool() {
                    Ok(LiteralValue::Boolean(!b))
                } else {
                    Err(DbError::Execution("Type mismatch in NOT operation".to_string()))
                }
            }
            UnaryOperator::Negate => {
                if let Some(i) = val.as_i64() {
                    Ok(LiteralValue::Integer(-i))
                } else if let Some(f) = val.as_f64() {
                    Ok(LiteralValue::Float(-f))
                } else {
                    Err(DbError::Execution("Type mismatch in negation".to_string()))
                }
            }
            UnaryOperator::Plus => Ok(val.clone()),
        }
    }

    /// Evaluate a function call
    fn evaluate_function(&self, name: &str, args: &[Expression]) -> Result<LiteralValue> {
        match name.to_uppercase().as_str() {
            "UPPER" => {
                if args.len() != 1 {
                    return Err(DbError::Execution("UPPER expects 1 argument".to_string()));
                }
                let val = self.evaluate(&args[0])?;
                Ok(LiteralValue::String(val.to_string().to_uppercase()))
            }
            "LOWER" => {
                if args.len() != 1 {
                    return Err(DbError::Execution("LOWER expects 1 argument".to_string()));
                }
                let val = self.evaluate(&args[0])?;
                Ok(LiteralValue::String(val.to_string().to_lowercase()))
            }
            "LENGTH" | "LEN" => {
                if args.len() != 1 {
                    return Err(DbError::Execution("LENGTH expects 1 argument".to_string()));
                }
                let val = self.evaluate(&args[0])?;
                Ok(LiteralValue::Integer(val.to_string().len() as i64))
            }
            "ABS" => {
                if args.len() != 1 {
                    return Err(DbError::Execution("ABS expects 1 argument".to_string()));
                }
                let val = self.evaluate(&args[0])?;
                if let Some(i) = val.as_i64() {
                    Ok(LiteralValue::Integer(i.abs()))
                } else if let Some(f) = val.as_f64() {
                    Ok(LiteralValue::Float(f.abs()))
                } else {
                    Err(DbError::Execution("ABS expects numeric argument".to_string()))
                }
            }
            "COALESCE" => {
                for arg in args {
                    let val = self.evaluate(arg)?;
                    if !matches!(val, LiteralValue::Null) {
                        return Ok(val);
                    }
                }
                Ok(LiteralValue::Null)
            }
            _ => Err(DbError::Execution(format!("Unknown function: {}", name))),
        }
    }

    /// Check if two values are equal
    fn values_equal(&self, left: &LiteralValue, right: &LiteralValue) -> bool {
        match (left, right) {
            (LiteralValue::Null, LiteralValue::Null) => true,
            (LiteralValue::Boolean(l), LiteralValue::Boolean(r)) => l == r,
            (LiteralValue::Integer(l), LiteralValue::Integer(r)) => l == r,
            (LiteralValue::Float(l), LiteralValue::Float(r)) => (l - r).abs() < f64::EPSILON,
            (LiteralValue::String(l), LiteralValue::String(r)) => l == r,
            (LiteralValue::Date(l), LiteralValue::Date(r)) => l == r,
            (LiteralValue::Timestamp(l), LiteralValue::Timestamp(r)) => l == r,
            _ => false,
        }
    }

    /// Compare two values, returning -1, 0, or 1
    fn compare_values(&self, left: &LiteralValue, right: &LiteralValue) -> Result<i32> {
        match (left, right) {
            (LiteralValue::Integer(l), LiteralValue::Integer(r)) => {
                Ok(if l < r { -1 } else if l > r { 1 } else { 0 })
            }
            (LiteralValue::Float(l), LiteralValue::Float(r)) => {
                Ok(if l < r { -1 } else if l > r { 1 } else { 0 })
            }
            (LiteralValue::String(l), LiteralValue::String(r)) => {
                Ok(if l < r { -1 } else if l > r { 1 } else { 0 })
            }
            (LiteralValue::Date(l), LiteralValue::Date(r)) => {
                Ok(if l < r { -1 } else if l > r { 1 } else { 0 })
            }
            (LiteralValue::Timestamp(l), LiteralValue::Timestamp(r)) => {
                Ok(if l < r { -1 } else if l > r { 1 } else { 0 })
            }
            _ => Err(DbError::Execution("Type mismatch in comparison".to_string())),
        }
    }

    /// Match a string against a LIKE pattern
    fn match_like_pattern(&self, text: &str, pattern: &str) -> bool {
        let text_chars: Vec<char> = text.chars().collect();
        let pattern_chars: Vec<char> = pattern.chars().collect();

        self.like_match_recursive(&text_chars, &pattern_chars, 0, 0)
    }

    /// Recursive LIKE pattern matching
    fn like_match_recursive(&self, text: &[char], pattern: &[char], t_idx: usize, p_idx: usize) -> bool {
        if p_idx >= pattern.len() {
            return t_idx >= text.len();
        }

        match pattern[p_idx] {
            '%' => {
                // Match zero or more characters
                if self.like_match_recursive(text, pattern, t_idx, p_idx + 1) {
                    return true;
                }
                if t_idx < text.len() && self.like_match_recursive(text, pattern, t_idx + 1, p_idx) {
                    return true;
                }
                false
            }
            '_' => {
                // Match exactly one character
                if t_idx < text.len() {
                    self.like_match_recursive(text, pattern, t_idx + 1, p_idx + 1)
                } else {
                    false
                }
            }
            c => {
                // Match exact character
                if t_idx < text.len() && text[t_idx].eq_ignore_ascii_case(&c) {
                    self.like_match_recursive(text, pattern, t_idx + 1, p_idx + 1)
                } else {
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_expression() {
        let mut row_data = HashMap::new();
        row_data.insert("status".to_string(), LiteralValue::Integer(1));

        let evaluator = ExpressionEvaluator::new(row_data);

        let case_expr = Expression::Case {
            operand: Some(Box::new(Expression::Column("status".to_string()))),
            conditions: vec![
                (
                    Expression::Literal(LiteralValue::Integer(1)),
                    Expression::Literal(LiteralValue::String("Active".to_string())),
                ),
                (
                    Expression::Literal(LiteralValue::Integer(2)),
                    Expression::Literal(LiteralValue::String("Inactive".to_string())),
                ),
            ],
            else_result: Some(Box::new(Expression::Literal(LiteralValue::String("Unknown".to_string())))),
        };

        let result = evaluator.evaluate(&case_expr).unwrap();
        assert_eq!(result, LiteralValue::String("Active".to_string()));
    }

    #[test]
    fn test_between_expression() {
        let mut row_data = HashMap::new();
        row_data.insert("age".to_string(), LiteralValue::Integer(25));

        let evaluator = ExpressionEvaluator::new(row_data);

        let between_expr = Expression::Between {
            expr: Box::new(Expression::Column("age".to_string())),
            low: Box::new(Expression::Literal(LiteralValue::Integer(18))),
            high: Box::new(Expression::Literal(LiteralValue::Integer(65))),
            negated: false,
        };

        let result = evaluator.evaluate(&between_expr).unwrap();
        assert_eq!(result, LiteralValue::Boolean(true));
    }

    #[test]
    fn test_in_expression() {
        let mut row_data = HashMap::new();
        row_data.insert("category".to_string(), LiteralValue::String("A".to_string()));

        let evaluator = ExpressionEvaluator::new(row_data);

        let in_expr = Expression::In {
            expr: Box::new(Expression::Column("category".to_string())),
            list: vec![
                Expression::Literal(LiteralValue::String("A".to_string())),
                Expression::Literal(LiteralValue::String("B".to_string())),
                Expression::Literal(LiteralValue::String("C".to_string())),
            ],
            negated: false,
        };

        let result = evaluator.evaluate(&in_expr).unwrap();
        assert_eq!(result, LiteralValue::Boolean(true));
    }

    #[test]
    fn test_like_expression() {
        let mut row_data = HashMap::new();
        row_data.insert("name".to_string(), LiteralValue::String("John Doe".to_string()));

        let evaluator = ExpressionEvaluator::new(row_data);

        let like_expr = Expression::Like {
            expr: Box::new(Expression::Column("name".to_string())),
            pattern: Box::new(Expression::Literal(LiteralValue::String("John%".to_string()))),
            escape: None,
            negated: false,
        };

        let result = evaluator.evaluate(&like_expr).unwrap();
        assert_eq!(result, LiteralValue::Boolean(true));
    }
}
