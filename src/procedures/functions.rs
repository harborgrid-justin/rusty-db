// User-Defined Functions Module
//
// This module provides support for creating and managing user-defined functions,
// including scalar functions, table-valued functions, and custom aggregate functions.

use crate::procedures::parser::{Expression, PlSqlBlock, PlSqlType};
use crate::procedures::runtime::{RuntimeExecutor, RuntimeValue};
use crate::{DbError, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// Function parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParameter {
    pub name: String,
    pub data_type: PlSqlType,
    pub default_value: Option<Expression>,
}

// Function return type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FunctionReturnType {
    // Scalar return type (single value)
    Scalar(PlSqlType),
    // Table return type (set of rows)
    Table { columns: Vec<(String, PlSqlType)> },
}

// Determinism level of a function
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Determinism {
    // Function always returns the same result for the same inputs
    Deterministic,
    // Function may return different results for the same inputs
    NonDeterministic,
}

// User-defined scalar function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarFunction {
    pub name: String,
    pub parameters: Vec<FunctionParameter>,
    pub return_type: PlSqlType,
    pub body: PlSqlBlock,
    pub determinism: Determinism,
    pub parallel_enabled: bool,
}

// User-defined table function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableFunction {
    pub name: String,
    pub parameters: Vec<FunctionParameter>,
    pub return_columns: Vec<(String, PlSqlType)>,
    pub body: PlSqlBlock,
    pub determinism: Determinism,
    pub parallel_enabled: bool,
}

// Aggregate function accumulator
#[derive(Debug, Clone)]
pub struct AggregateState {
    pub data: HashMap<String, RuntimeValue>,
}

impl AggregateState {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: String, value: RuntimeValue) {
        self.data.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&RuntimeValue> {
        self.data.get(key)
    }
}

impl Default for AggregateState {
    fn default() -> Self {
        Self::new()
    }
}

// User-defined aggregate function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateFunction {
    pub name: String,
    pub input_type: PlSqlType,
    pub return_type: PlSqlType,
    pub initialize_body: PlSqlBlock,
    pub accumulate_body: PlSqlBlock,
    pub merge_body: Option<PlSqlBlock>,
    pub finalize_body: PlSqlBlock,
    pub parallel_enabled: bool,
}

// Function type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserFunction {
    Scalar(ScalarFunction),
    Table(TableFunction),
    Aggregate(AggregateFunction),
}

impl UserFunction {
    pub fn name(&self) -> &str {
        match self {
            UserFunction::Scalar(f) => &f.name,
            UserFunction::Table(f) => &f.name,
            UserFunction::Aggregate(f) => &f.name,
        }
    }

    pub fn is_deterministic(&self) -> bool {
        match self {
            UserFunction::Scalar(f) => f.determinism == Determinism::Deterministic,
            UserFunction::Table(f) => f.determinism == Determinism::Deterministic,
            UserFunction::Aggregate(_) => false, // Aggregates are typically non-deterministic
        }
    }

    pub fn is_parallel_enabled(&self) -> bool {
        match self {
            UserFunction::Scalar(f) => f.parallel_enabled,
            UserFunction::Table(f) => f.parallel_enabled,
            UserFunction::Aggregate(f) => f.parallel_enabled,
        }
    }
}

// Function manager
pub struct FunctionManager {
    functions: Arc<RwLock<HashMap<String, UserFunction>>>,
    runtime: Arc<RuntimeExecutor>,
}

impl FunctionManager {
    pub fn new() -> Self {
        Self {
            functions: Arc::new(RwLock::new(HashMap::new())),
            runtime: Arc::new(RuntimeExecutor::new()),
        }
    }

    // Create a scalar function
    pub fn create_scalar_function(&self, function: ScalarFunction) -> Result<()> {
        let mut functions = self.functions.write();

        if functions.contains_key(&function.name) {
            return Err(DbError::AlreadyExists(format!(
                "Function '{}' already exists",
                function.name
            )));
        }

        functions.insert(function.name.clone(), UserFunction::Scalar(function));
        Ok(())
    }

    // Create a table function
    pub fn create_table_function(&self, function: TableFunction) -> Result<()> {
        let mut functions = self.functions.write();

        if functions.contains_key(&function.name) {
            return Err(DbError::AlreadyExists(format!(
                "Function '{}' already exists",
                function.name
            )));
        }

        functions.insert(function.name.clone(), UserFunction::Table(function));
        Ok(())
    }

    // Create an aggregate function
    pub fn create_aggregate_function(&self, function: AggregateFunction) -> Result<()> {
        let mut functions = self.functions.write();

        if functions.contains_key(&function.name) {
            return Err(DbError::AlreadyExists(format!(
                "Function '{}' already exists",
                function.name
            )));
        }

        functions.insert(function.name.clone(), UserFunction::Aggregate(function));
        Ok(())
    }

    // Drop a function
    pub fn drop_function(&self, name: &str) -> Result<()> {
        let mut functions = self.functions.write();

        if functions.remove(name).is_none() {
            return Err(DbError::NotFound(format!("Function '{}' not found", name)));
        }

        Ok(())
    }

    // Get a function by name
    pub fn get_function(&self, name: &str) -> Result<UserFunction> {
        let functions = self.functions.read();

        functions
            .get(name)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("Function '{}' not found", name)))
    }

    // List all functions
    pub fn list_functions(&self) -> Vec<String> {
        let functions = self.functions.read();
        functions.keys().cloned().collect()
    }

    // Execute a scalar function
    pub fn execute_scalar(&self, name: &str, arguments: Vec<RuntimeValue>) -> Result<RuntimeValue> {
        let function = self.get_function(name)?;

        match function {
            UserFunction::Scalar(func) => {
                // Validate parameter count
                if arguments.len() != func.parameters.len() {
                    return Err(DbError::InvalidInput(format!(
                        "Function '{}' expects {} arguments, got {}",
                        name,
                        func.parameters.len(),
                        arguments.len()
                    )));
                }

                // Execute the function body
                let result = self.runtime.execute(&func.body)?;

                // Return the result
                result.return_value.ok_or_else(|| {
                    DbError::Runtime(format!("Function '{}' did not return a value", name))
                })
            }
            _ => Err(DbError::InvalidInput(format!(
                "'{}' is not a scalar function",
                name
            ))),
        }
    }

    // Execute a table function
    pub fn execute_table(
        &self,
        name: &str,
        arguments: Vec<RuntimeValue>,
    ) -> Result<Vec<HashMap<String, RuntimeValue>>> {
        let function = self.get_function(name)?;

        match function {
            UserFunction::Table(func) => {
                // Validate parameter count
                if arguments.len() != func.parameters.len() {
                    return Err(DbError::InvalidInput(format!(
                        "Function '{}' expects {} arguments, got {}",
                        name,
                        func.parameters.len(),
                        arguments.len()
                    )));
                }

                // Execute the function body and collect rows
                // Create a new context with parameters bound
                let result = self.runtime.execute(&func.body)?;

                // Extract rows from the result
                // The function body should populate a collection that we return
                // Check the return value for an array of records
                let rows = match result.return_value {
                    Some(RuntimeValue::Array(arr)) => arr
                        .into_iter()
                        .filter_map(|item| match item {
                            RuntimeValue::Record(record) => Some(record),
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                };

                Ok(rows)
            }
            _ => Err(DbError::InvalidInput(format!(
                "'{}' is not a table function",
                name
            ))),
        }
    }

    // Initialize aggregate state
    pub fn initialize_aggregate(&self, name: &str) -> Result<AggregateState> {
        let function = self.get_function(name)?;

        match function {
            UserFunction::Aggregate(func) => {
                let _result = self.runtime.execute(&func.initialize_body)?;
                Ok(AggregateState::new())
            }
            _ => Err(DbError::InvalidInput(format!(
                "'{}' is not an aggregate function",
                name
            ))),
        }
    }

    // Accumulate a value in the aggregate
    pub fn accumulate_aggregate(
        &self,
        name: &str,
        state: &mut AggregateState,
        value: RuntimeValue,
    ) -> Result<()> {
        let function = self.get_function(name)?;

        match function {
            UserFunction::Aggregate(func) => {
                // Pass state and value to accumulate_body
                // The accumulate body should update the state based on the input value

                // Execute the accumulate body
                // In a full implementation, we would:
                // 1. Bind the current state to a context variable
                // 2. Bind the input value to a context variable
                // 3. Execute the body
                // 4. Extract the updated state
                let result = self.runtime.execute(&func.accumulate_body)?;

                // Update state from the result's output parameters or return value
                if let Some(return_val) = result.return_value {
                    if let RuntimeValue::Record(record) = return_val {
                        for (key, val) in record {
                            state.set(key, val);
                        }
                    }
                }

                // Handle standard aggregation patterns
                // Update running totals, counts, etc. in state
                match value {
                    RuntimeValue::Integer(i) => {
                        // Update sum
                        let current_sum = state
                            .get("_sum")
                            .and_then(|v| v.as_integer().ok())
                            .unwrap_or(0);
                        state.set("_sum".to_string(), RuntimeValue::Integer(current_sum + i));

                        // Update count
                        let current_count = state
                            .get("_count")
                            .and_then(|v| v.as_integer().ok())
                            .unwrap_or(0);
                        state.set(
                            "_count".to_string(),
                            RuntimeValue::Integer(current_count + 1),
                        );

                        // Update min
                        let current_min = state.get("_min").and_then(|v| v.as_integer().ok());
                        if current_min.is_none() || i < current_min.unwrap() {
                            state.set("_min".to_string(), RuntimeValue::Integer(i));
                        }

                        // Update max
                        let current_max = state.get("_max").and_then(|v| v.as_integer().ok());
                        if current_max.is_none() || i > current_max.unwrap() {
                            state.set("_max".to_string(), RuntimeValue::Integer(i));
                        }
                    }
                    RuntimeValue::Float(f) => {
                        let current_sum = state
                            .get("_sum")
                            .and_then(|v| v.as_float().ok())
                            .unwrap_or(0.0);
                        state.set("_sum".to_string(), RuntimeValue::Float(current_sum + f));

                        let current_count = state
                            .get("_count")
                            .and_then(|v| v.as_integer().ok())
                            .unwrap_or(0);
                        state.set(
                            "_count".to_string(),
                            RuntimeValue::Integer(current_count + 1),
                        );
                    }
                    RuntimeValue::Null => {
                        // NULL values are typically ignored in aggregates
                    }
                    _ => {
                        // For other types, just increment count
                        let current_count = state
                            .get("_count")
                            .and_then(|v| v.as_integer().ok())
                            .unwrap_or(0);
                        state.set(
                            "_count".to_string(),
                            RuntimeValue::Integer(current_count + 1),
                        );
                    }
                }

                Ok(())
            }
            _ => Err(DbError::InvalidInput(format!(
                "'{}' is not an aggregate function",
                name
            ))),
        }
    }

    // Finalize aggregate and return result
    pub fn finalize_aggregate(&self, name: &str, state: &AggregateState) -> Result<RuntimeValue> {
        let function = self.get_function(name)?;

        match function {
            UserFunction::Aggregate(func) => {
                // Pass state to finalize_body and compute final result
                // Execute the finalize body which should compute the final aggregate value
                let result = self.runtime.execute(&func.finalize_body)?;

                // If the body returned a value, use it
                if let Some(return_val) = result.return_value {
                    return Ok(return_val);
                }

                // Otherwise, compute a default result based on common aggregate patterns
                // Check what type of aggregate this might be based on state
                if let Some(sum) = state.get("_sum") {
                    if let Some(count) = state.get("_count") {
                        // If we have both sum and count, might be AVG
                        if func.name.to_uppercase().contains("AVG") {
                            let sum_val = sum.as_float().unwrap_or(0.0);
                            let count_val = count.as_integer().unwrap_or(1) as f64;
                            if count_val > 0.0 {
                                return Ok(RuntimeValue::Float(sum_val / count_val));
                            }
                        }
                        // Otherwise return sum (for SUM-like aggregates)
                        return Ok(sum.clone());
                    }
                    return Ok(sum.clone());
                }

                if let Some(count) = state.get("_count") {
                    return Ok(count.clone());
                }

                if let Some(min) = state.get("_min") {
                    if func.name.to_uppercase().contains("MIN") {
                        return Ok(min.clone());
                    }
                }

                if let Some(max) = state.get("_max") {
                    if func.name.to_uppercase().contains("MAX") {
                        return Ok(max.clone());
                    }
                }

                Err(DbError::Runtime(format!(
                    "Aggregate function '{}' did not return a value",
                    name
                )))
            }
            _ => Err(DbError::InvalidInput(format!(
                "'{}' is not an aggregate function",
                name
            ))),
        }
    }

    // Get function signature for documentation
    pub fn get_signature(&self, name: &str) -> Result<String> {
        let function = self.get_function(name)?;

        let signature = match &function {
            UserFunction::Scalar(func) => {
                let params: Vec<String> = func
                    .parameters
                    .iter()
                    .map(|p| format!("{} {:?}", p.name, p.data_type))
                    .collect();
                format!(
                    "{}({}) RETURN {:?}",
                    func.name,
                    params.join(", "),
                    func.return_type
                )
            }
            UserFunction::Table(func) => {
                let params: Vec<String> = func
                    .parameters
                    .iter()
                    .map(|p| format!("{} {:?}", p.name, p.data_type))
                    .collect();
                let returns: Vec<String> = func
                    .return_columns
                    .iter()
                    .map(|(n, t)| format!("{} {:?}", n, t))
                    .collect();
                format!(
                    "{}({}) RETURN TABLE({})",
                    func.name,
                    params.join(", "),
                    returns.join(", ")
                )
            }
            UserFunction::Aggregate(func) => {
                format!(
                    "{}({:?}) RETURN {:?}",
                    func.name, func.input_type, func.return_type
                )
            }
        };

        Ok(signature)
    }
}

impl Default for FunctionManager {
    fn default() -> Self {
        Self::new()
    }
}

// Built-in scalar functions
pub struct BuiltInFunctions;

impl BuiltInFunctions {
    // String functions
    pub fn upper(s: &str) -> String {
        s.to_uppercase()
    }

    pub fn lower(s: &str) -> String {
        s.to_lowercase()
    }

    pub fn trim(s: &str) -> String {
        s.trim().to_string()
    }

    pub fn ltrim(s: &str) -> String {
        s.trim_start().to_string()
    }

    pub fn rtrim(s: &str) -> String {
        s.trim_end().to_string()
    }

    pub fn length(s: &str) -> i64 {
        s.len() as i64
    }

    pub fn substr(s: &str, start: i64, length: Option<i64>) -> String {
        let start = (start - 1).max(0) as usize;
        if let Some(len) = length {
            s.chars().skip(start).take(len.max(0) as usize).collect()
        } else {
            s.chars().skip(start).collect()
        }
    }

    pub fn replace(s: &str, from: &str, to: &str) -> String {
        s.replace(from, to)
    }

    pub fn concat(strings: Vec<&str>) -> String {
        strings.join("")
    }

    // Numeric functions
    pub fn abs_int(n: i64) -> i64 {
        n.abs()
    }

    pub fn abs_float(n: f64) -> f64 {
        n.abs()
    }

    pub fn ceil(n: f64) -> i64 {
        n.ceil() as i64
    }

    pub fn floor(n: f64) -> i64 {
        n.floor() as i64
    }

    pub fn round(n: f64, decimals: i32) -> f64 {
        let multiplier = 10_f64.powi(decimals);
        (n * multiplier).round() / multiplier
    }

    pub fn trunc(n: f64, decimals: i32) -> f64 {
        let multiplier = 10_f64.powi(decimals);
        (n * multiplier).trunc() / multiplier
    }

    pub fn power(base: f64, exp: f64) -> f64 {
        base.powf(exp)
    }

    pub fn sqrt(n: f64) -> Result<f64> {
        if n < 0.0 {
            Err(DbError::Runtime(
                "Cannot take square root of negative number".to_string(),
            ))
        } else {
            Ok(n.sqrt())
        }
    }

    pub fn mod_op(a: i64, b: i64) -> Result<i64> {
        if b == 0 {
            Err(DbError::Runtime("Division by zero".to_string()))
        } else {
            Ok(a % b)
        }
    }

    pub fn sign(n: f64) -> i64 {
        if n > 0.0 {
            1
        } else if n < 0.0 {
            -1
        } else {
            0
        }
    }

    // Date/Time functions (simplified)
    pub fn current_date() -> String {
        // In production, would use chrono or similar
        "2024-01-01".to_string()
    }

    pub fn current_timestamp() -> String {
        // In production, would use chrono or similar
        "2024-01-01 00:00:00".to_string()
    }

    // Conversion functions
    pub fn to_char_int(n: i64) -> String {
        n.to_string()
    }

    pub fn to_char_float(n: f64) -> String {
        n.to_string()
    }

    pub fn to_number(s: &str) -> Result<f64> {
        s.parse::<f64>()
            .map_err(|_| DbError::Runtime(format!("Cannot convert '{}' to number", s)))
    }

    pub fn to_date(s: &str) -> Result<String> {
        // Simplified - in production would validate and parse date
        Ok(s.to_string())
    }

    // NULL-related functions
    pub fn nvl(value: &RuntimeValue, default: &RuntimeValue) -> RuntimeValue {
        if value.is_null() {
            default.clone()
        } else {
            value.clone()
        }
    }

    pub fn nvl2(
        value: &RuntimeValue,
        if_not_null: &RuntimeValue,
        if_null: &RuntimeValue,
    ) -> RuntimeValue {
        if value.is_null() {
            if_null.clone()
        } else {
            if_not_null.clone()
        }
    }

    pub fn coalesce(values: Vec<&RuntimeValue>) -> RuntimeValue {
        for val in values {
            if !val.is_null() {
                return val.clone();
            }
        }
        RuntimeValue::Null
    }

    // Conditional functions
    pub fn decode(
        expr: &RuntimeValue,
        search_result_pairs: Vec<(&RuntimeValue, &RuntimeValue)>,
        default: &RuntimeValue,
    ) -> RuntimeValue {
        for (search, result) in search_result_pairs {
            // Simple equality check
            if format!("{:?}", expr) == format!("{:?}", search) {
                return result.clone();
            }
        }
        default.clone()
    }

    // Aggregate helper functions
    pub fn greatest(values: Vec<RuntimeValue>) -> Result<RuntimeValue> {
        if values.is_empty() {
            return Ok(RuntimeValue::Null);
        }

        let mut max = values[0].clone();
        for val in values.iter().skip(1) {
            // Simplified comparison
            if let (RuntimeValue::Integer(a), RuntimeValue::Integer(b)) = (&max, val) {
                if b > a {
                    max = val.clone();
                }
            } else if let (RuntimeValue::Float(a), RuntimeValue::Float(b)) = (&max, val) {
                if b > a {
                    max = val.clone();
                }
            }
        }

        Ok(max)
    }

    pub fn least(values: Vec<RuntimeValue>) -> Result<RuntimeValue> {
        if values.is_empty() {
            return Ok(RuntimeValue::Null);
        }

        let mut min = values[0].clone();
        for val in values.iter().skip(1) {
            // Simplified comparison
            if let (RuntimeValue::Integer(a), RuntimeValue::Integer(b)) = (&min, val) {
                if b < a {
                    min = val.clone();
                }
            } else if let (RuntimeValue::Float(a), RuntimeValue::Float(b)) = (&min, val) {
                if b < a {
                    min = val.clone();
                }
            }
        }

        Ok(min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_functions() {
        assert_eq!(BuiltInFunctions::upper("hello"), "HELLO");
        assert_eq!(BuiltInFunctions::lower("WORLD"), "world");
        assert_eq!(BuiltInFunctions::length("test"), 4);
        assert_eq!(BuiltInFunctions::substr("hello", 2, Some(3)), "ell");
    }

    #[test]
    fn test_numeric_functions() {
        assert_eq!(BuiltInFunctions::abs_int(-5), 5);
        assert_eq!(BuiltInFunctions::ceil(3.2), 4);
        assert_eq!(BuiltInFunctions::floor(3.8), 3);
        assert_eq!(BuiltInFunctions::round(std::f64::consts::PI, 2), 3.14);
    }

    #[test]
    fn test_nvl() {
        let null_val = RuntimeValue::Null;
        let int_val = RuntimeValue::Integer(42);
        let default_val = RuntimeValue::Integer(0);

        assert_eq!(BuiltInFunctions::nvl(&null_val, &default_val), default_val);
        assert_eq!(BuiltInFunctions::nvl(&int_val, &default_val), int_val);
    }

    #[test]
    fn test_coalesce() {
        let null1 = RuntimeValue::Null;
        let null2 = RuntimeValue::Null;
        let val = RuntimeValue::Integer(42);

        assert_eq!(BuiltInFunctions::coalesce(vec![&null1, &null2, &val]), val);
    }

    #[test]
    fn test_create_scalar_function() -> Result<()> {
        let manager = FunctionManager::new();

        // Create a simple scalar function (body would be parsed in real usage)
        let func = ScalarFunction {
            name: "add_ten".to_string(),
            parameters: vec![FunctionParameter {
                name: "x".to_string(),
                data_type: PlSqlType::Integer,
                default_value: None,
            }],
            return_type: PlSqlType::Integer,
            body: PlSqlBlock {
                declarations: Vec::new(),
                statements: Vec::new(),
                exception_handlers: Vec::new(),
            },
            determinism: Determinism::Deterministic,
            parallel_enabled: true,
        };

        manager.create_scalar_function(func)?;

        assert_eq!(manager.list_functions().len(), 1);

        Ok(())
    }
}
