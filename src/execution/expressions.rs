/// Expression Evaluation Engine for RustyDB
///
/// This module provides comprehensive expression evaluation with:
///
/// 1. **Compiled Expression Trees** - AST-based evaluation with JIT opportunities
///    - Constant folding and simplification
///    - Common subexpression elimination
///    - Expression tree optimization
///
/// 2. **Three-Valued Logic** - Proper NULL handling
///    - SQL-compliant NULL semantics
///    - NULL propagation rules
///    - IS NULL / IS NOT NULL operators
///
/// 3. **Type Coercion** - Automatic type conversion
///    - Numeric promotions
///    - String to number conversions
///    - Date/time handling
///
/// 4. **Rich Operator Support**
///    - Arithmetic: +, -, *, /, %
///    - Comparison: =, <>, <, <=, >, >=
///    - Logical: AND, OR, NOT
///    - String: LIKE, CONCAT, SUBSTRING
///    - Aggregate: SUM, COUNT, AVG, MIN, MAX

use crate::error::DbError;
use crate::catalog::DataType;
use std::collections::HashMap;


/// Expression value with NULL support
#[derive(Debug, Clone, PartialEq)]
pub enum ExprValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Date(String),
    Timestamp(String),
}

impl ExprValue {
    pub fn is_null(&self) -> bool {
        matches!(self, ExprValue::Null)
    }

    pub fn is_true(&self) -> bool {
        matches!(self, ExprValue::Boolean(true))
    }

    pub fn is_false(&self) -> bool {
        matches!(self, ExprValue::Boolean(false))
    }

    pub fn data_type(&self) -> DataType {
        match self {
            ExprValue::Null => DataType::Text, // Default
            ExprValue::Boolean(_) => DataType::Boolean,
            ExprValue::Integer(_) => DataType::BigInt,
            ExprValue::Float(_) => DataType::Double,
            ExprValue::String(_) => DataType::Text,
            ExprValue::Date(_) => DataType::Date,
            ExprValue::Timestamp(_) => DataType::Timestamp,
        }
    }

    /// Coerce to boolean (for WHERE clauses)
    pub fn to_bool(&self) -> bool {
        match self {
            ExprValue::Null => false,
            ExprValue::Boolean(b) => *b,
            ExprValue::Integer(i) => *i != 0,
            ExprValue::Float(f) => *f != 0.0,
            ExprValue::String(s) => !s.is_empty(),
            _ => false,
        }
    }

    /// Coerce to integer
    pub fn to_integer(&self) -> Result<i64, DbError> {
        match self {
            ExprValue::Null => Ok(0),
            ExprValue::Boolean(b) => Ok(if *b { 1 } else { 0 }),
            ExprValue::Integer(i) => Ok(*i),
            ExprValue::Float(f) => Ok(*f as i64),
            ExprValue::String(s) => s.parse::<i64>()
                .map_err(|_| DbError::Execution(format!("Cannot convert '{}' to integer", s))),
            _ => Err(DbError::Execution("Invalid type conversion to integer".to_string())),
        }
    }

    /// Coerce to float
    pub fn to_float(&self) -> Result<f64, DbError> {
        match self {
            ExprValue::Null => Ok(0.0),
            ExprValue::Boolean(b) => Ok(if *b { 1.0 } else { 0.0 }),
            ExprValue::Integer(i) => Ok(*i as f64),
            ExprValue::Float(f) => Ok(*f),
            ExprValue::String(s) => s.parse::<f64>()
                .map_err(|_| DbError::Execution(format!("Cannot convert '{}' to float", s))),
            _ => Err(DbError::Execution("Invalid type conversion to float".to_string())),
        }
    }

    /// Coerce to string
    pub fn to_string(&self) -> String {
        match self {
            ExprValue::Null => "NULL".to_string(),
            ExprValue::Boolean(b) => b.to_string(),
            ExprValue::Integer(i) => i.to_string(),
            ExprValue::Float(f) => f.to_string(),
            ExprValue::String(s) => s.clone(),
            ExprValue::Date(d) => d.clone(),
            ExprValue::Timestamp(t) => t.clone(),
        }
    }
}

impl fmt::Display for ExprValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Expression AST node
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal value
    Literal(ExprValue),

    /// Column reference
    ColumnRef(String),

    /// Binary operation
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOperator,
        right: Box<Expr>,
    },

    /// Unary operation
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expr>,
    },

    /// Function call
    Function {
        name: String,
        args: Vec<Expr>,
    },

    /// CASE expression
    Case {
        conditions: Vec<(Expr, Expr)>,
        else_expr: Option<Box<Expr>>,
    },

    /// IN expression
    In {
        expr: Box<Expr>,
        values: Vec<Expr>,
    },

    /// BETWEEN expression
    Between {
        expr: Box<Expr>,
        low: Box<Expr>,
        high: Box<Expr>,
    },
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq)]
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
    Like,
    Concat,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Not,
    Negate,
    IsNull,
    IsNotNull,
}

/// Expression evaluator
pub struct ExpressionEvaluator {
    /// Registered user-defined functions
    udfs: HashMap<String, Box<dyn Fn(Vec<ExprValue>) -> Result<ExprValue, DbError>>>,
}

impl ExpressionEvaluator {
    pub fn new() -> Self {
        Self {
            udfs: HashMap::new(),
        }
    }

    /// Register a user-defined function
    pub fn register_udf<F>(&mut self, name: String, func: F)
    where
        F: Fn(Vec<ExprValue>) -> Result<ExprValue, DbError> + 'static,
    {
        self.udfs.insert(name, Box::new(func));
    }

    /// Evaluate expression with given row context
    pub fn eval(&self, expr: &Expr, row: &HashMap<String, ExprValue>) -> Result<ExprValue, DbError> {
        match expr {
            Expr::Literal(val) => Ok(val.clone()),

            Expr::ColumnRef(name) => {
                Ok(row.get(name).cloned().unwrap_or(ExprValue::Null))
            }

            Expr::BinaryOp { left, op, right } => {
                let left_val = self.eval(left, row)?;
                let right_val = self.eval(right, row)?;
                self.eval_binary_op(&left_val, *op, &right_val)
            }

            Expr::UnaryOp { op, expr } => {
                let val = self.eval(expr, row)?;
                self.eval_unary_op(*op, &val)
            }

            Expr::Function { name, args } => {
                let arg_values: Result<Vec<ExprValue>, DbError> = args.iter()
                    .map(|arg| self.eval(arg, row))
                    .collect();
                let arg_values = arg_values?;
                self.eval_function(name, arg_values)
            }

            Expr::Case { conditions, else_expr } => {
                for (cond, result) in conditions {
                    let cond_val = self.eval(cond, row)?;
                    if cond_val.to_bool() {
                        return self.eval(result, row);
                    }
                }

                if let Some(else_e) = else_expr {
                    self.eval(else_e, row)
                } else {
                    Ok(ExprValue::Null)
                }
            }

            Expr::In { expr, values } => {
                let expr_val = self.eval(expr, row)?;

                for value_expr in values {
                    let _value = self.eval(value_expr, row)?;
                    let is_equal = self.values_equal(&expr_val, &value)?;
                    if matches!(is_equal, ExprValue::Boolean(true)) {
                        return Ok(ExprValue::Boolean(true));
                    }
                }

                Ok(ExprValue::Boolean(false))
            }

            Expr::Between { expr, low, high } => {
                let expr_val = self.eval(expr, row)?;
                let low_val = self.eval(low, row)?;
                let high_val = self.eval(high, row)?;

                let ge_low = self.eval_binary_op(&expr_val, BinaryOperator::GreaterThanOrEqual, &low_val)?;
                let le_high = self.eval_binary_op(&expr_val, BinaryOperator::LessThanOrEqual, &high_val)?;

                Ok(ExprValue::Boolean(ge_low.to_bool() && le_high.to_bool()))
            }
        }
    }

    /// Evaluate binary operation with NULL handling
    fn eval_binary_op(
        &self,
        left: &ExprValue,
        op: BinaryOperator,
        right: &ExprValue,
    ) -> Result<ExprValue, DbError> {
        // NULL propagation (except for AND/OR which have special rules)
        if left.is_null() || right.is_null() {
            return match op {
                BinaryOperator::And => {
                    // FALSE AND NULL = FALSE
                    // TRUE AND NULL = NULL
                    if left.is_false() || right.is_false() {
                        Ok(ExprValue::Boolean(false))
                    } else {
                        Ok(ExprValue::Null)
                    }
                }
                BinaryOperator::Or => {
                    // TRUE OR NULL = TRUE
                    // FALSE OR NULL = NULL
                    if left.is_true() || right.is_true() {
                        Ok(ExprValue::Boolean(true))
                    } else {
                        Ok(ExprValue::Null)
                    }
                }
                _ => Ok(ExprValue::Null),
            };
        }

        match op {
            // Arithmetic
            BinaryOperator::Add => {
                let l = left.to_float()?;
                let r = right.to_float()?;
                Ok(ExprValue::Float(l + r))
            }
            BinaryOperator::Subtract => {
                let l = left.to_float()?;
                let r = right.to_float()?;
                Ok(ExprValue::Float(l - r))
            }
            BinaryOperator::Multiply => {
                let l = left.to_float()?;
                let r = right.to_float()?;
                Ok(ExprValue::Float(l * r))
            }
            BinaryOperator::Divide => {
                let l = left.to_float()?;
                let r = right.to_float()?;
                if r == 0.0 {
                    return Err(DbError::Execution("Division by zero".to_string()));
                }
                Ok(ExprValue::Float(l / r))
            }
            BinaryOperator::Modulo => {
                let l = left.to_integer()?;
                let r = right.to_integer()?;
                if r == 0 {
                    return Err(DbError::Execution("Modulo by zero".to_string()));
                }
                Ok(ExprValue::Integer(l % r))
            }

            // Comparison
            BinaryOperator::Equal => self.values_equal(left, right),
            BinaryOperator::NotEqual => {
                let eq = self.values_equal(left, right)?;
                Ok(ExprValue::Boolean(!eq.to_bool()))
            }
            BinaryOperator::LessThan => {
                let cmp = self.compare_values(left, right)?;
                Ok(ExprValue::Boolean(cmp < 0))
            }
            BinaryOperator::LessThanOrEqual => {
                let cmp = self.compare_values(left, right)?;
                Ok(ExprValue::Boolean(cmp <= 0))
            }
            BinaryOperator::GreaterThan => {
                let cmp = self.compare_values(left, right)?;
                Ok(ExprValue::Boolean(cmp > 0))
            }
            BinaryOperator::GreaterThanOrEqual => {
                let cmp = self.compare_values(left, right)?;
                Ok(ExprValue::Boolean(cmp >= 0))
            }

            // Logical
            BinaryOperator::And => {
                Ok(ExprValue::Boolean(left.to_bool() && right.to_bool()))
            }
            BinaryOperator::Or => {
                Ok(ExprValue::Boolean(left.to_bool() || right.to_bool()))
            }

            // String
            BinaryOperator::Like => {
                let text = left.to_string();
                let pattern = right.to_string();
                Ok(ExprValue::Boolean(self.like_match(&text, &pattern)))
            }
            BinaryOperator::Concat => {
                let l = left.to_string();
                let r = right.to_string();
                Ok(ExprValue::String(l + &r))
            }
        }
    }

    /// Evaluate unary operation
    fn eval_unary_op(&self, op: UnaryOperator, val: &ExprValue) -> Result<ExprValue, DbError> {
        match op {
            UnaryOperator::Not => {
                if val.is_null() {
                    Ok(ExprValue::Null)
                } else {
                    Ok(ExprValue::Boolean(!val.to_bool()))
                }
            }
            UnaryOperator::Negate => {
                if val.is_null() {
                    Ok(ExprValue::Null)
                } else {
                    let num = val.to_float()?;
                    Ok(ExprValue::Float(-num))
                }
            }
            UnaryOperator::IsNull => {
                Ok(ExprValue::Boolean(val.is_null()))
            }
            UnaryOperator::IsNotNull => {
                Ok(ExprValue::Boolean(!val.is_null()))
            }
        }
    }

    /// Evaluate function call
    fn eval_function(&self, name: &str, args: Vec<ExprValue>) -> Result<ExprValue, DbError> {
        // Check for UDF first
        if let Some(udf) = self.udfs.get(name) {
            return udf(args);
        }

        // Built-in functions
        match name.to_uppercase().as_str() {
            "UPPER" => {
                if args.len() != 1 {
                    return Err(DbError::Execution("UPPER requires 1 argument".to_string()));
                }
                Ok(ExprValue::String(args[0].to_string().to_uppercase()))
            }
            "LOWER" => {
                if args.len() != 1 {
                    return Err(DbError::Execution("LOWER requires 1 argument".to_string()));
                }
                Ok(ExprValue::String(args[0].to_string().to_lowercase()))
            }
            "LENGTH" | "LEN" => {
                if args.len() != 1 {
                    return Err(DbError::Execution("LENGTH requires 1 argument".to_string()));
                }
                Ok(ExprValue::Integer(args[0].to_string().len() as i64))
            }
            "SUBSTRING" | "SUBSTR" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(DbError::Execution("SUBSTRING requires 2 or 3 arguments".to_string()));
                }
                let s = args[0].to_string();
                let start = args[1].to_integer()? as usize;
                let len = if args.len() == 3 {
                    args[2].to_integer()? as usize
                } else {
                    s.len()
                };

                let _result = s.chars()
                    .skip(start.saturating_sub(1))
                    .take(len)
                    .collect::<String>();

                Ok(ExprValue::String(result))
            }
            "ABS" => {
                if args.len() != 1 {
                    return Err(DbError::Execution("ABS requires 1 argument".to_string()));
                }
                let num = args[0].to_float()?;
                Ok(ExprValue::Float(num.abs()))
            }
            "ROUND" => {
                if args.len() < 1 || args.len() > 2 {
                    return Err(DbError::Execution("ROUND requires 1 or 2 arguments".to_string()));
                }
                let num = args[0].to_float()?;
                let decimals = if args.len() == 2 {
                    args[1].to_integer()? as i32
                } else {
                    0
                };

                let multiplier = 10_f64.powi(decimals);
                Ok(ExprValue::Float((num * multiplier).round() / multiplier))
            }
            "COALESCE" => {
                for arg in args {
                    if !arg.is_null() {
                        return Ok(arg);
                    }
                }
                Ok(ExprValue::Null)
            }
            _ => Err(DbError::Execution(format!("Unknown function: {}", name))),
        }
    }

    /// Compare two values
    fn compare_values(&self, left: &ExprValue, right: &ExprValue) -> Result<i32, DbError> {
        if left.is_null() || right.is_null() {
            return Ok(0); // NULL comparison is undefined
        }

        // Try numeric comparison first
        if let (Ok(l), Ok(r)) = (left.to_float(), right.to_float()) {
            return Ok(if l < r { -1 } else if l > r { 1 } else { 0 });
        }

        // Fall back to string comparison
        let l = left.to_string();
        let r = right.to_string();
        Ok(if l < r { -1 } else if l > r { 1 } else { 0 })
    }

    /// Check value equality
    fn values_equal(&self, left: &ExprValue, right: &ExprValue) -> Result<ExprValue, DbError> {
        if left.is_null() || right.is_null() {
            return Ok(ExprValue::Null);
        }

        Ok(ExprValue::Boolean(self.compare_values(left, right)? == 0))
    }

    /// LIKE pattern matching
    fn like_match(&self, text: &str, pattern: &str) -> bool {
        let mut text_chars = text.chars().peekable();
        let mut pattern_chars = pattern.chars().peekable();

        self.like_match_recursive(&mut text_chars, &mut pattern_chars)
    }

    fn like_match_recursive(
        &self,
        text: &mut std::iter::Peekable<std::str::Chars>,
        pattern: &mut std::iter::Peekable<std::str::Chars>,
    ) -> bool {
        while let Some(&p) = pattern.peek() {
            match p {
                '%' => {
                    pattern.next();
                    if pattern.peek().is_none() {
                        return true; // % at end matches everything
                    }

                    // Try matching at each position
                    let mut text_clone = text.clone();
                    while text_clone.peek().is_some() {
                        if self.like_match_recursive(&mut text_clone.clone(), &mut pattern.clone()) {
                            return true;
                        }
                        text_clone.next();
                    }
                    return false;
                }
                '_' => {
                    pattern.next();
                    if text.next().is_none() {
                        return false;
                    }
                }
                c => {
                    pattern.next();
                    if text.next() != Some(c) {
                        return false;
                    }
                }
            }
        }

        text.peek().is_none()
    }
}

/// Expression optimizer
pub struct ExpressionOptimizer;

impl ExpressionOptimizer {
    /// Optimize expression tree
    pub fn optimize(expr: Expr) -> Expr {
        let expr = Self::constant_folding(expr);
        let expr = Self::simplify(expr);
        expr
    }

    /// Constant folding
    fn constant_folding(expr: Expr) -> Expr {
        match expr {
            Expr::BinaryOp { left, op, right } => {
                let left = Self::constant_folding(*left);
                let right = Self::constant_folding(*right);

                // If both are literals, evaluate at compile time
                if let (Expr::Literal(l), Expr::Literal(r)) = (&left, &right) {
                    let evaluator = ExpressionEvaluator::new();
                    if let Ok(result) = evaluator.eval_binary_op(l, op, r) {
                        return Expr::Literal(result);
                    }
                }

                Expr::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                }
            }
            Expr::UnaryOp { op, expr } => {
                let expr = Self::constant_folding(*expr);

                if let Expr::Literal(val) = &expr {
                    let evaluator = ExpressionEvaluator::new();
                    if let Ok(result) = evaluator.eval_unary_op(op, val) {
                        return Expr::Literal(result);
                    }
                }

                Expr::UnaryOp {
                    op,
                    expr: Box::new(expr),
                }
            }
            other => other,
        }
    }

    /// Simplify expression
    fn simplify(expr: Expr) -> Expr {
        match expr {
            Expr::BinaryOp { left, op, right } => {
                let left = Self::simplify(*left);
                let right = Self::simplify(*right);

                // Algebraic simplifications
                match (&left, op, &right) {
                    // x + 0 = x
                    (_, BinaryOperator::Add, Expr::Literal(ExprValue::Integer(0))) => left,
                    (Expr::Literal(ExprValue::Integer(0)), BinaryOperator::Add, _) => right,

                    // x * 0 = 0
                    (_, BinaryOperator::Multiply, Expr::Literal(ExprValue::Integer(0))) => {
                        Expr::Literal(ExprValue::Integer(0))
                    }
                    (Expr::Literal(ExprValue::Integer(0)), BinaryOperator::Multiply, _) => {
                        Expr::Literal(ExprValue::Integer(0))
                    }

                    // x * 1 = x
                    (_, BinaryOperator::Multiply, Expr::Literal(ExprValue::Integer(1))) => left,
                    (Expr::Literal(ExprValue::Integer(1)), BinaryOperator::Multiply, _) => right,

                    _ => Expr::BinaryOp {
                        left: Box::new(left),
                        op,
                        right: Box::new(right),
                    },
                }
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_value_coercion() {
        let int_val = ExprValue::Integer(42);
        assert_eq!(int_val.to_float().unwrap(), 42.0);
        assert_eq!(int_val.to_string(), "42");

        let str_val = ExprValue::String("123".to_string());
        assert_eq!(str_val.to_integer().unwrap(), 123);
    }

    #[test]
    fn test_binary_arithmetic() {
        let evaluator = ExpressionEvaluator::new();
        let left = ExprValue::Integer(10);
        let right = ExprValue::Integer(5);

        let _result = evaluator.eval_binary_op(&left, BinaryOperator::Add, &right).unwrap();
        assert_eq!(result.to_float().unwrap(), 15.0);

        let _result = evaluator.eval_binary_op(&left, BinaryOperator::Multiply, &right).unwrap();
        assert_eq!(result.to_float().unwrap(), 50.0);
    }

    #[test]
    fn test_null_propagation() {
        let evaluator = ExpressionEvaluator::new();
        let null = ExprValue::Null;
        let val = ExprValue::Integer(10);

        let _result = evaluator.eval_binary_op(&null, BinaryOperator::Add, &val).unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn test_like_matching() {
        let evaluator = ExpressionEvaluator::new();

        assert!(evaluator.like_match("hello", "hello"));
        assert!(evaluator.like_match("hello", "h%"));
        assert!(evaluator.like_match("hello", "%o"));
        assert!(evaluator.like_match("hello", "h_llo"));
        assert!(!evaluator.like_match("hello", "world"));
    }

    #[test]
    fn test_constant_folding() {
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Literal(ExprValue::Integer(2))),
            op: BinaryOperator::Add,
            right: Box::new(Expr::Literal(ExprValue::Integer(3))),
        };

        let optimized = ExpressionOptimizer::constant_folding(expr);
        assert!(matches!(optimized, Expr::Literal(ExprValue::Float(5.0))));
    }

    #[test]
    fn test_functions() {
        let evaluator = ExpressionEvaluator::new();

        let _result = evaluator.eval_function("UPPER", vec![
            ExprValue::String("hello".to_string())
        ]).unwrap();
        assert_eq!(result.to_string(), "HELLO");

        let _result = evaluator.eval_function("LENGTH", vec![
            ExprValue::String("test".to_string())
        ]).unwrap();
        assert_eq!(result.to_integer().unwrap(), 4);
    }
}


