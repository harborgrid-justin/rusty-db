// # SQL/JSON Integration
//
// Oracle-like SQL/JSON functions including JSON_TABLE, JSON_QUERY, JSON_VALUE,
// JSON_EXISTS, IS JSON predicate, and JSON generation functions.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::error::Result;
use super::jsonpath::{JsonPathParser, JsonPathEvaluator};

/// JSON_TABLE function result
#[derive(Debug, Clone)]
pub struct JsonTableResult {
    /// Column names
    pub columns: Vec<String>,
    /// Rows of data
    pub rows: Vec<Vec<Value>>,
}

impl JsonTableResult {
    /// Create a new result
    pub fn new(columns: Vec<String>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
        }
    }

    /// Add a row
    pub fn add_row(&mut self, row: Vec<Value>) {
        self.rows.push(row);
    }

    /// Get row count
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Get column count
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }
}

/// JSON_TABLE column definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonTableColumn {
    /// Column name
    pub name: String,
    /// JSONPath expression
    pub path: String,
    /// Data type
    pub data_type: JsonDataType,
    /// Error handling
    pub on_error: ErrorHandling,
    /// Empty handling
    pub on_empty: ErrorHandling,
}

impl JsonTableColumn {
    /// Create a new column definition
    pub fn new(name: impl Into<String>, path: impl Into<String>, data_type: JsonDataType) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            data_type,
            on_error: ErrorHandling::Null,
            on_empty: ErrorHandling::Null,
        }
    }

    /// Set error handling
    pub fn on_error(mut self, handling: ErrorHandling) -> Self {
        self.on_error = handling;
        self
    }

    /// Set empty handling
    pub fn on_empty(mut self, handling: ErrorHandling) -> Self {
        self.on_empty = handling;
        self
    }
}

/// JSON data types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JsonDataType {
    /// String/VARCHAR
    String,
    /// Number/INTEGER
    Integer,
    /// Number/FLOAT
    Float,
    /// Boolean
    Boolean,
    /// JSON (returns nested JSON)
    Json,
    /// Date
    Date,
    /// Timestamp
    Timestamp,
}

/// Error handling strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorHandling {
    /// Return NULL
    Null,
    /// Return default value
    Default(Value),
    /// Raise error
    Error,
}

/// JSON_TABLE function implementation
pub struct JsonTableFunction;

impl JsonTableFunction {
    /// Execute JSON_TABLE function
    ///
    /// # Example
    /// ```ignore
    /// JSON_TABLE(
    ///     json_doc,
    ///     '$.store.books[*]',
    ///     COLUMNS(
    ///         title VARCHAR PATH '$.title',
    ///         price FLOAT PATH '$.price'
    ///     )
    /// )
    /// ```
    pub fn execute(
        json: &Value,
        row_path: &str,
        columns: Vec<JsonTableColumn>,
    ) -> Result<JsonTableResult> {
        let mut result = JsonTableResult::new(
            columns.iter().map(|c| c.name.clone()).collect()
        );

        // Parse row path
        let mut parser = JsonPathParser::new(row_path.to_string());
        let path = parser.parse()?;

        // Extract rows using row path
        let rows = JsonPathEvaluator::evaluate(&path, json)?;

        // Process each row
        for row_value in rows {
            let mut row_data = Vec::new();

            for column in &columns {
                let cell_value = Self::extract_column_value(&row_value, column)?;
                row_data.push(cell_value);
            }

            result.add_row(row_data);
        }

        Ok(result)
    }

    fn extract_column_value(row: &Value, column: &JsonTableColumn) -> Result<Value> {
        // Parse column path
        let mut parser = JsonPathParser::new(column.path.clone());
        let path = parser.parse()?;

        // Extract value
        let values = JsonPathEvaluator::evaluate(&path, row)?;

        if values.is_empty() {
            return Self::handle_empty(column);
        }

        let _value = &values[0];

        // Convert to target type
        Self::convert_type(value, column.data_type, column)
    }

    fn convert_type(value: &Value, data_type: JsonDataType, column: &JsonTableColumn) -> Result<Value> {
        match data_type {
            JsonDataType::String => {
                if let Some(s) = value.as_str() {
                    Ok(Value::String(s.to_string()))
                } else {
                    Ok(Value::String(value.to_string()))
                }
            }
            JsonDataType::Integer => {
                if let Some(i) = value.as_i64() {
                    Ok(Value::Number(i.into()))
                } else if let Some(f) = value.as_f64() {
                    Ok(Value::Number((f as i64).into()))
                } else {
                    Self::handle_error(column)
                }
            }
            JsonDataType::Float => {
                if let Some(f) = value.as_f64() {
                    Ok(Value::Number(serde_json::Number::from_f64(f).unwrap()))
                } else {
                    Self::handle_error(column)
                }
            }
            JsonDataType::Boolean => {
                if let Some(b) = value.as_bool() {
                    Ok(Value::Bool(b))
                } else {
                    Self::handle_error(column)
                }
            }
            JsonDataType::Json => Ok(value.clone()),
            JsonDataType::Date | JsonDataType::Timestamp => {
                if let Some(s) = value.as_str() {
                    Ok(Value::String(s.to_string()))
                } else {
                    Self::handle_error(column)
                }
            }
        }
    }

    fn handle_error(column: &JsonTableColumn) -> Result<Value> {
        match &column.on_error {
            ErrorHandling::Null => Ok(Value::Null),
            ErrorHandling::Default(v) => Ok(v.clone()),
            ErrorHandling::Error => Err(crate::error::DbError::InvalidInput(
                format!("Type conversion error for column '{}'", column.name)
            )),
        }
    }

    fn handle_empty(column: &JsonTableColumn) -> Result<Value> {
        match &column.on_empty {
            ErrorHandling::Null => Ok(Value::Null),
            ErrorHandling::Default(v) => Ok(v.clone()),
            ErrorHandling::Error => Err(crate::error::DbError::InvalidInput(
                format!("Empty value for column '{}'", column.name)
            )),
        }
    }
}

/// JSON_QUERY function - extract JSON fragments
pub struct JsonQueryFunction;

impl JsonQueryFunction {
    /// Execute JSON_QUERY
    ///
    /// Extracts a JSON fragment from a JSON document
    pub fn execute(
        json: &Value,
        path: &str,
        wrapper: JsonWrapper,
    ) -> Result<Option<Value>> {
        let mut parser = JsonPathParser::new(path.to_string());
        let json_path = parser.parse()?;

        let results = JsonPathEvaluator::evaluate(&json_path, json)?;

        if results.is_empty() {
            return Ok(None);
        }

        match wrapper {
            JsonWrapper::None => {
                if results.len() == 1 {
                    Ok(Some(results[0].clone()))
                } else {
                    Ok(Some(Value::Array(results)))
                }
            }
            JsonWrapper::WithWrapper => {
                Ok(Some(Value::Array(results)))
            }
            JsonWrapper::WithoutWrapper => {
                if results.len() == 1 {
                    Ok(Some(results[0].clone()))
                } else {
                    Err(crate::error::DbError::InvalidInput(
                        "Multiple values found, use WITH WRAPPER".to_string()
                    ))
                }
            }
            JsonWrapper::WithConditionalWrapper => {
                if results.len() == 1 {
                    Ok(Some(results[0].clone()))
                } else {
                    Ok(Some(Value::Array(results)))
                }
            }
        }
    }
}

/// JSON wrapper options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonWrapper {
    /// No wrapper specification
    None,
    /// Always wrap in array
    WithWrapper,
    /// Never wrap (error if multiple values)
    WithoutWrapper,
    /// Wrap only if multiple values
    WithConditionalWrapper,
}

/// JSON_VALUE function - extract scalar value
pub struct JsonValueFunction;

impl JsonValueFunction {
    /// Execute JSON_VALUE
    ///
    /// Extracts a scalar value from a JSON document
    pub fn execute(
        json: &Value,
        path: &str,
        returning_type: JsonDataType,
    ) -> Result<Option<Value>> {
        let mut parser = JsonPathParser::new(path.to_string());
        let json_path = parser.parse()?;

        let results = JsonPathEvaluator::evaluate(&json_path, json)?;

        if results.is_empty() {
            return Ok(None);
        }

        if results.len() > 1 {
            return Err(crate::error::DbError::InvalidInput(
                "JSON_VALUE requires a single scalar value".to_string()
            ));
        }

        let _value = &results[0];

        // Ensure it's a scalar value
        if value.is_object() || value.is_array() {
            return Err(crate::error::DbError::InvalidInput(
                "JSON_VALUE requires a scalar value, not object or array".to_string()
            ));
        }

        // Convert type
        let column = JsonTableColumn::new("", "", returning_type);
        let converted = JsonTableFunction::convert_type(value, returning_type, &column)?;

        Ok(Some(converted))
    }
}

/// JSON_EXISTS function - check if path exists
pub struct JsonExistsFunction;

impl JsonExistsFunction {
    /// Execute JSON_EXISTS
    ///
    /// Returns true if the path exists in the JSON document
    pub fn execute(json: &Value, path: &str) -> Result<bool> {
        let mut parser = JsonPathParser::new(path.to_string());
        let json_path = parser.parse()?;

        let results = JsonPathEvaluator::evaluate(&json_path, json)?;

        Ok(!results.is_empty())
    }

    /// Execute JSON_EXISTS with filter
    pub fn execute_with_filter(
        json: &Value,
        path: &str,
        filter: &str,
    ) -> Result<bool> {
        let full_path = format!("{}[?({})]", path, filter);
        Self::execute(json, &full_path)
    }
}

/// IS JSON predicate
pub struct IsJsonPredicate;

impl IsJsonPredicate {
    /// Check if string is valid JSON
    pub fn is_json(text: &str) -> bool {
        serde_json::from_str::<Value>(text).is_ok()
    }

    /// Check if string is valid JSON object
    pub fn is_json_object(text: &str) -> bool {
        if let Ok(Value::Object(_)) = serde_json::from_str::<Value>(text) {
            true
        } else {
            false
        }
    }

    /// Check if string is valid JSON array
    pub fn is_json_array(text: &str) -> bool {
        if let Ok(Value::Array(_)) = serde_json::from_str::<Value>(text) {
            true
        } else {
            false
        }
    }

    /// Check if string is valid JSON scalar
    pub fn is_json_scalar(text: &str) -> bool {
        if let Ok(value) = serde_json::from_str::<Value>(text) {
            !value.is_object() && !value.is_array()
        } else {
            false
        }
    }
}

/// JSON generation functions
pub struct JsonGenerationFunctions;

impl JsonGenerationFunctions {
    /// JSON_OBJECT - create JSON object from key-value pairs
    ///
    /// # Example
    /// ```ignore
    /// JSON_OBJECT('name' : 'Alice', 'age' : 30)
    /// ```
    pub fn json_object(pairs: Vec<(String, Value)>) -> Value {
        let mut obj = serde_json::Map::new();
        for (key, value) in pairs {
            obj.insert(key, value);
        }
        Value::Object(obj)
    }

    /// JSON_ARRAY - create JSON array from values
    ///
    /// # Example
    /// ```ignore
    /// JSON_ARRAY(1, 2, 3, 'four')
    /// ```
    pub fn json_array(values: Vec<Value>) -> Value {
        Value::Array(values)
    }

    /// JSON_OBJECTAGG - aggregate into JSON object
    pub fn json_objectagg(key_value_pairs: Vec<(Value, Value)>) -> Result<Value> {
        let mut obj = serde_json::Map::new();

        for (key, value) in key_value_pairs {
            let key_str = if let Value::String(s) = key {
                s
            } else {
                key.to_string()
            };
            obj.insert(key_str, value);
        }

        Ok(Value::Object(obj))
    }

    /// JSON_ARRAYAGG - aggregate into JSON array
    pub fn json_arrayagg(values: Vec<Value>) -> Value {
        Value::Array(values)
    }

    /// JSON_MERGEPATCH - merge JSON documents (RFC 7396)
    pub fn json_mergepatch(target: &Value, patch: &Value) -> Value {
        match (target, patch) {
            (Value::Object(target_obj), Value::Object(patch_obj)) => {
                let mut result = target_obj.clone();

                for (key, patch_value) in patch_obj {
                    if patch_value.is_null() {
                        result.remove(key);
                    } else if let Some(target_value) = result.get(key) {
                        result.insert(
                            key.clone(),
                            Self::json_mergepatch(target_value, patch_value),
                        );
                    } else {
                        result.insert(key.clone(), patch_value.clone());
                    }
                }

                Value::Object(result)
            }
            (_, patch) if !patch.is_null() => patch.clone(),
            _ => target.clone(),
        }
    }

    /// JSON_TRANSFORM - transform JSON using operations
    pub fn json_transform(json: &Value, operations: Vec<TransformOperation>) -> Result<Value> {
        let mut result = json.clone();

        for operation in operations {
            result = operation.apply(&result)?;
        }

        Ok(result)
    }
}

/// Transform operation for JSON_TRANSFORM
#[derive(Debug, Clone)]
pub enum TransformOperation {
    /// Set a value at path
    Set { path: String, value: Value },
    /// Remove a path
    Remove { path: String },
    /// Rename a field
    Rename { from: String, to: String },
    /// Keep only specified paths
    Keep { paths: Vec<String> },
    /// Remove specified paths
    RemovePaths { paths: Vec<String> },
}

impl TransformOperation {
    /// Apply transformation
    pub fn apply(&self, json: &Value) -> Result<Value> {
        match self {
            TransformOperation::Set { path, value } => {
                let mut result = json.clone();
                Self::set_at_path(&mut result, path, value.clone())?;
                Ok(result)
            }
            TransformOperation::Remove { path } => {
                let mut result = json.clone();
                Self::remove_at_path(&mut result, path)?;
                Ok(result)
            }
            TransformOperation::Rename { from, to } => {
                if let Value::Object(mut obj) = json.clone() {
                    if let Some(value) = obj.remove(from) {
                        obj.insert(to.clone(), value);
                    }
                    Ok(Value::Object(obj))
                } else {
                    Ok(json.clone())
                }
            }
            TransformOperation::Keep { paths } => {
                if let Value::Object(obj) = json {
                    let mut result = serde_json::Map::new();
                    for path in paths {
                        if let Some(value) = obj.get(path) {
                            result.insert(path.clone(), value.clone());
                        }
                    }
                    Ok(Value::Object(result))
                } else {
                    Ok(json.clone())
                }
            }
            TransformOperation::RemovePaths { paths } => {
                if let Value::Object(mut obj) = json.clone() {
                    for path in paths {
                        obj.remove(path);
                    }
                    Ok(Value::Object(obj))
                } else {
                    Ok(json.clone())
                }
            }
        }
    }

    fn set_at_path(root: &mut Value, path: &str, new_value: Value) -> Result<()> {
        if path.is_empty() {
            *root = new_value;
            return Ok(());
        }

        let parts: Vec<&str> = path.split('.').collect();
        let mut current = root;

        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                if let Value::Object(obj) = current {
                    obj.insert(part.to_string(), new_value);
                    return Ok(());
                }
            } else {
                if let Value::Object(obj) = current {
                    current = obj.entry(part.to_string())
                        .or_insert_with(|| Value::Object(serde_json::Map::new()));
                } else {
                    return Err(crate::error::DbError::InvalidInput(
                        format!("Cannot navigate path: {}", path)
                    ));
                }
            }
        }

        Ok(())
    }

    fn remove_at_path(root: &mut Value, path: &str) -> Result<()> {
        if path.is_empty() {
            return Ok(());
        }

        let parts: Vec<&str> = path.split('.').collect();
        let mut current = root;

        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                if let Value::Object(obj) = current {
                    obj.remove(*part);
                    return Ok(());
                }
            } else {
                if let Value::Object(obj) = current {
                    current = obj.get_mut(*part)
                        .ok_or_else(|| crate::error::DbError::InvalidInput(
                            format!("Path not found: {}", path)
                        ))?;
                } else {
                    return Err(crate::error::DbError::InvalidInput(
                        format!("Cannot navigate path: {}", path)
                    ));
                }
            }
        }

        Ok(())
    }
}

/// SQL/JSON function registry
pub struct SqlJsonFunctions;

impl SqlJsonFunctions {
    /// Execute JSON_TABLE
    pub fn json_table(
        json: &Value,
        row_path: &str,
        columns: Vec<JsonTableColumn>,
    ) -> Result<JsonTableResult> {
        JsonTableFunction::execute(json, row_path, columns)
    }

    /// Execute JSON_QUERY
    pub fn json_query(
        json: &Value,
        path: &str,
        wrapper: JsonWrapper,
    ) -> Result<Option<Value>> {
        JsonQueryFunction::execute(json, path, wrapper)
    }

    /// Execute JSON_VALUE
    pub fn json_value(
        json: &Value,
        path: &str,
        returning_type: JsonDataType,
    ) -> Result<Option<Value>> {
        JsonValueFunction::execute(json, path, returning_type)
    }

    /// Execute JSON_EXISTS
    pub fn json_exists(json: &Value, path: &str) -> Result<bool> {
        JsonExistsFunction::execute(json, path)
    }

    /// Check IS JSON
    pub fn is_json(text: &str) -> bool {
        IsJsonPredicate::is_json(text)
    }

    /// Create JSON object
    pub fn json_object(pairs: Vec<(String, Value)>) -> Value {
        JsonGenerationFunctions::json_object(pairs)
    }

    /// Create JSON array
    pub fn json_array(values: Vec<Value>) -> Value {
        JsonGenerationFunctions::json_array(values)
    }

    /// Merge JSON patches
    pub fn json_mergepatch(target: &Value, patch: &Value) -> Value {
        JsonGenerationFunctions::json_mergepatch(target, patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_table() {
        let data = json!({
            "store": {
                "books": [
                    {"title": "Book 1", "price": 10.99},
                    {"title": "Book 2", "price": 15.99}
                ]
            }
        });

        let columns = vec![
            JsonTableColumn::new("title", "$.title", JsonDataType::String),
            JsonTableColumn::new("price", "$.price", JsonDataType::Float),
        ];

        let _result = JsonTableFunction::execute(
            &data,
            "$.store.books[*]",
            columns,
        ).unwrap();

        assert_eq!(result.row_count(), 2);
        assert_eq!(result.column_count(), 2);
    }

    #[test]
    fn test_json_query() {
        let data = json!({
            "name": "Alice",
            "contacts": [
                {"type": "email", "value": "alice@example.com"},
                {"type": "phone", "value": "555-1234"}
            ]
        });

        let _result = JsonQueryFunction::execute(
            &data,
            "$.contacts",
            JsonWrapper::None,
        ).unwrap();

        assert!(result.is_some());
        assert!(result.unwrap().is_array());
    }

    #[test]
    fn test_json_value() {
        let data = json!({"name": "Alice", "age": 30});

        let _result = JsonValueFunction::execute(
            &data,
            "$.age",
            JsonDataType::Integer,
        ).unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap(), json!(30));
    }

    #[test]
    fn test_json_exists() {
        let data = json!({"name": "Alice", "age": 30});

        assert!(JsonExistsFunction::execute(&data, "$.name").unwrap());
        assert!(JsonExistsFunction::execute(&data, "$.age").unwrap());
        assert!(!JsonExistsFunction::execute(&data, "$.email").unwrap());
    }

    #[test]
    fn test_is_json() {
        assert!(IsJsonPredicate::is_json(r#"{"name":"Alice"}"#));
        assert!(IsJsonPredicate::is_json(r#"[1,2,3]"#));
        assert!(!IsJsonPredicate::is_json("not json"));
    }

    #[test]
    fn test_json_object() {
        let obj = JsonGenerationFunctions::json_object(vec![
            ("name".to_string(), json!("Alice")),
            ("age".to_string(), json!(30)),
        ]);

        assert_eq!(obj["name"], "Alice");
        assert_eq!(obj["age"], 30);
    }

    #[test]
    fn test_json_array() {
        let arr = JsonGenerationFunctions::json_array(vec![
            json!(1),
            json!(2),
            json!(3),
        ]);

        assert!(arr.is_array());
        assert_eq!(arr.as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_json_mergepatch() {
        let target = json!({"name": "Alice", "age": 30});
        let patch = json!({"age": 31, "city": "NYC"});

        let _result = JsonGenerationFunctions::json_mergepatch(&target, &patch);

        assert_eq!(result["name"], "Alice");
        assert_eq!(result["age"], 31);
        assert_eq!(result["city"], "NYC");
    }
}
