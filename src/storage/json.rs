/// JSON Data Type and Operations Support
/// 
/// This module provides comprehensive JSON support for RustyDB:
/// - JSON data type with validation
/// - JSON path expressions (JSONPath)
/// - JSON operators (extract, update, delete, contains)
/// - JSON indexing for fast queries
/// - JSON aggregation functions

use crate::error::{Result, DbError};
use serde_json::{Value as JsonValue, json};
use std::collections::HashMap;

/// JSON data wrapper
#[derive(Debug, Clone, PartialEq)]
pub struct JsonData {
    value: JsonValue,
}

impl JsonData {
    /// Create new JSON data from string
    pub fn from_str(s: &str) -> Result<Self> {
        let value = serde_json::from_str(s)
            .map_err(|e| DbError::InvalidInput(format!("Invalid JSON: {}", e)))?);
        Ok(Self { value })
    }
    
    /// Create from serde_json Value
    pub fn from_value(value: JsonValue) -> Self {
        Self { value }
    }
    
    /// Get the underlying JSON value
    pub fn value(&self) -> &JsonValue {
        &self.value
    }
    
    /// Convert to string
    pub fn to_string(&self) -> String {
        self.value.to_string()
    }
    
    /// Convert to pretty-printed string
    pub fn to_string_pretty(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.value)
            .map_err(|e| DbError::Internal(format!("JSON serialization error: {}", e)))
    }
    
    /// Get JSON type
    pub fn json_type(&self) -> JsonType {
        match &self.value {
            JsonValue::Null => JsonType::Null,
            JsonValue::Bool(_) => JsonType::Boolean,
            JsonValue::Number(_) => JsonType::Number,
            JsonValue::String(_) => JsonType::String,
            JsonValue::Array(_) => JsonType::Array,
            JsonValue::Object(_) => JsonType::Object,
        }
    }
}

/// JSON type enum
#[derive(Debug, Clone, PartialEq)]
pub enum JsonType {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object,
}

/// JSON path expression parser and evaluator
pub struct JsonPath));

impl JsonPath {
    /// Extract value at JSON path
    /// 
    /// Examples:
    /// - "$.name" -> root.name
    /// - "$.address.city" -> root.address.city
    /// - "$.items[0]" -> root.items[0]
    /// - "$.items[*].price" -> all prices in items array
    pub fn extract(json: &JsonData, path: &str) -> Result<JsonData> {
        let tokens = Self::parse_path(path)?;
        let mut current = &json.value;
        
        for token in tokens {
            current = Self::navigate(current, &token)?;
        }
        
        Ok(JsonData::from_value(current.clone()))
    }
    
    /// Extract all values matching a path (for wildcards)
    pub fn extract_all(json: &JsonData, path: &str) -> Result<Vec<JsonData>> {
        let tokens = Self::parse_path(path)?;
        let mut results = vec![json.value.clone()];
        
        for token in tokens {
            let mut new_results = Vec::new();
            for value in results {
                match token {
                    PathToken::Field(ref field) if field == "*" => {
                        // Wildcard - get all object values
                        if let JsonValue::Object(obj) = value {
                            new_results.extend(obj.values().cloned());
                        }
                    }
                    PathToken::Index(idx) if idx == -1 => {
                        // Array wildcard [*]
                        if let JsonValue::Array(arr) = value {
                            new_results.extend(arr.clone());
                        }
                    }
                    _ => {
                        if let Ok(navigated) = Self::navigate(&value, &token) {
                            new_results.push(navigated.clone());
                        }
                    }
                }
            }
            results = new_results;
        }
        
        Ok(results.into_iter().map(JsonData::from_value).collect())
    }
    
    fn parse_path(path: &str) -> Result<Vec<PathToken>> {
        let mut tokens = Vec::new();
        let path = path.trim();
        
        if !path.starts_with('$') {
            return Err(DbError::InvalidInput(
                "JSON path must start with '$'".to_string()
            ));
        }
        
        let mut chars = path[1..].chars().peekable();
        
        while chars.peek().is_some() {
            match chars.next() {
                Some('.') => {
                    // Field access
                    let mut field = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch == '.' || ch == '[' {
                            break;
                        }
                        field.push(chars.next().unwrap());
                    }
                    if !field.is_empty() {
                        tokens.push(PathToken::Field(field));
                    }
                }
                Some('[') => {
                    // Array index or wildcard
                    let mut index_str = String::new();
                    while let Some(ch) = chars.next() {
                        if ch == ']' {
                            break;
                        }
                        index_str.push(ch);
                    }
                    
                    if index_str == "*" {
                        tokens.push(PathToken::Index(-1)); // Use -1 for wildcard
                    } else {
                        let index = index_str.parse::<i32>()
                            .map_err(|_| DbError::InvalidInput(
                                format!("Invalid array index: {}", index_str)
                            ))?);
                        tokens.push(PathToken::Index(index));
                    }
                }
                _ => {}
            }
        }
        
        Ok(tokens)
    }
    
    fn navigate<'a>(value: &'a JsonValue, token: &PathToken) -> Result<&'a JsonValue> {
        match token {
            PathToken::Field(field) => {
                value.get(field).ok_or_else(|| {
                    DbError::NotFound(format!("Field '{}' not found", field))
                })
            }
            PathToken::Index(idx) => {
                if let JsonValue::Array(arr) = value {
                    let index = if *idx < 0 {
                        (arr.len() as i32 + idx) as usize
                    } else {
                        *idx as usize
                    }));
                    
                    arr.get(index).ok_or_else(|| {
                        DbError::NotFound(format!("Array index {} out of bounds", idx))
                    })
                } else {
                    Err(DbError::InvalidOperation(
                        "Cannot index non-array value".to_string()
                    ))
                }
            }
        }
    }
}

/// Path token for JSON navigation
#[derive(Debug, Clone)]
enum PathToken {
    Field(String),
    Index(i32), // -1 for wildcard
}

/// JSON operators
pub struct JsonOperators));

impl JsonOperators {
    /// Extract JSON value at path
    pub fn json_extract(json: &JsonData, path: &str) -> Result<JsonData> {
        JsonPath::extract(json, path)
    }
    
    /// Set JSON value at path
    pub fn json_set(json: &JsonData, path: &str, newvalue: JsonData) -> Result<JsonData> {
        let mut result = json.value.clone();
        Self::set_at_path(&mut result, path, new_value.value)?;
        Ok(JsonData::from_value(result))
    }
    
    /// Delete JSON value at path
    pub fn json_delete(json: &JsonData, path: &str) -> Result<JsonData> {
        let mut result = json.value.clone();
        Self::delete_at_path(&mut result, path)?;
        Ok(JsonData::from_value(result))
    }
    
    /// Check if JSON contains a value
    pub fn json_contains(json: &JsonData, search: &JsonData) -> bool {
        Self::contains_value(&json.value, &search.value)
    }
    
    /// Get array length
    pub fn json_array_length(json: &JsonData) -> Result<usize> {
        match &json.value {
            JsonValue::Array(arr) => Ok(arr.len()),
            _ => Err(DbError::InvalidOperation(
                "JSON value is not an array".to_string()
            )),
        }
    }
    
    /// Get object keys
    pub fn json_keys(json: &JsonData) -> Result<Vec<String>> {
        match &json.value {
            JsonValue::Object(obj) => Ok(obj.keys().cloned().collect()),
            _ => Err(DbError::InvalidOperation(
                "JSON value is not an object".to_string()
            )),
        }
    }
    
    /// Merge two JSON objects
    pub fn json_merge(json1: &JsonData, json2: &JsonData) -> Result<JsonData> {
        let mut result = json1.value.clone();
        Self::merge_values(&mut result, &json2.value);
        Ok(JsonData::from_value(result))
    }
    
    fn set_at_path(value: &mut JsonValue, path: &str, newvalue: JsonValue) -> Result<()> {
        let tokens = JsonPath::parse_path(path)?;
        
        if tokens.is_empty() {
            *value = new_value;
            return Ok(());
        }
        
        let mut current = value;
        for (i, token) in tokens.iter().enumerate() {
            let is_last = i == tokens.len() - 1;
            
            match token {
                PathToken::Field(field) => {
                    if !current.is_object() {
                        *current = json!({});
                    }
                    
                    if is_last {
                        current.as_object_mut().unwrap().insert(field.clone(), new_value);
                        return Ok(());
                    } else {
                        current = current.get_mut(field)
                            .ok_or_else(|| DbError::NotFound(format!("Field '{}' not found", field)))?);
                    }
                }
                PathToken::Index(idx) => {
                    if !current.is_array() {
                        return Err(DbError::InvalidOperation(
                            "Cannot index non-array value".to_string()
                        ));
                    }
                    
                    let arr = current.as_array_mut().unwrap();
                    let index = if *idx < 0 {
                        (arr.len() as i32 + idx) as usize
                    } else {
                        *idx as usize
                    };
                    
                    if is_last {
                        if index < arr.len() {
                            arr[index] = new_value;
                        }
                        return Ok(());
                    } else {
                        current = arr.get_mut(index)
                            .ok_or_else(|| DbError::NotFound(format!("Index {} out of bounds", idx)))?);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn delete_at_path(value: &mut JsonValue, path: &str) -> Result<()> {
        let tokens = JsonPath::parse_path(path)?;
        
        if tokens.is_empty() {
            return Ok(());
        }
        
        if tokens.len() == 1 {
            match &tokens[0] {
                PathToken::Field(field) => {
                    if let JsonValue::Object(obj) = value {
                        obj.remove(field);
                    }
                }
                PathToken::Index(idx) => {
                    if let JsonValue::Array(arr) = value {
                        let index = if *idx < 0 {
                            (arr.len() as i32 + idx) as usize
                        } else {
                            *idx as usize
                        };
                        if index < arr.len() {
                            arr.remove(index);
                        }
                    }
                }
            }
            return Ok(());
        }
        
        // Navigate to parent and delete from there
        let parent_path = tokens[..tokens.len() - 1].to_vec();
        let mut current = value;
        
        for token in parent_path {
            match token {
                PathToken::Field(field) => {
                    current = current.get_mut(&field)
                        .ok_or_else(|| DbError::NotFound(format!("Field '{}' not found", field)))?);
                }
                PathToken::Index(idx) => {
                    let arr = current.as_array_mut()
                        .ok_or_else(|| DbError::InvalidOperation("Not an array".to_string()))?;
                    let index = if idx < 0 {
                        (arr.len() as i32 + idx) as usize
                    } else {
                        idx as usize
                    };
                    current = arr.get_mut(index)
                        .ok_or_else(|| DbError::NotFound(format!("Index {} out of bounds", idx)))?);
                }
            }
        }
        
        // Delete from parent
        match &tokens[tokens.len() - 1] {
            PathToken::Field(field) => {
                if let JsonValue::Object(obj) = current {
                    obj.remove(field);
                }
            }
            PathToken::Index(idx) => {
                if let JsonValue::Array(arr) = current {
                    let index = if *idx < 0 {
                        (arr.len() as i32 + idx) as usize
                    } else {
                        *idx as usize
                    };
                    if index < arr.len() {
                        arr.remove(index);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn contains_value(haystack: &JsonValue, needle: &JsonValue) -> bool {
        if haystack == needle {
            return true;
        }
        
        match haystack {
            JsonValue::Array(arr) => {
                arr.iter().any(|v| Self::contains_value(v, needle))
            }
            JsonValue::Object(obj) => {
                obj.values().any(|v| Self::contains_value(v, needle))
            }
            _ => false,
        }
    }
    
    fn merge_values(target: &mut JsonValue, source: &JsonValue) {
        match (target, source) {
            (JsonValue::Object(target_obj), JsonValue::Object(source_obj)) => {
                for (key, value) in source_obj {
                    if let Some(existing) = target_obj.get_mut(key) {
                        Self::merge_values(existing, value);
                    } else {
                        target_obj.insert(key.clone(), value.clone());
                    }
                }
            }
            (target, source) => {
                *target = source.clone();
            }
        }
    }
}

/// JSON index for fast queries
pub struct JsonIndex {
    table_name: String,
    column_name: String,
    /// Path -> (value -> document IDs)
    path_indexes: HashMap<String, HashMap<String, Vec<u64>>>,
}

impl JsonIndex {
    pub fn new(table_name: String, column_name: String) -> Self {
        Self {
            table_name,
            column_name,
            path_indexes: HashMap::new(),
        }
    }
    
    /// Create index on a specific JSON path
    pub fn index_path(&mut self, path: String) {
        self.path_indexes.insert(path, HashMap::new());
    }
    
    /// Add document to index
    pub fn add_document(&mut self, doc_id: u64, json: &JsonData) -> Result<()> {
        for (path, index) in &mut self.path_indexes {
            if let Ok(extracted) = JsonPath::extract(json, path) {
                let value_str = extracted.to_string();
                index.entry(value_str).or_insert_with(Vec::new).push(doc_id);
            }
        }
        Ok(())
    }
    
    /// Search for documents with specific value at path
    pub fn search(&self, path: &str, value: &str) -> Option<&Vec<u64>> {
        self.path_indexes
            .get(path)
            .and_then(|index| index.get(value))
    }
}

/// JSON aggregation functions
pub struct JsonAggregation;

impl JsonAggregation {
    /// Aggregate JSON values into array
    pub fn json_agg(values: Vec<JsonData>) -> JsonData {
        let array: Vec<JsonValue> = values.into_iter().map(|v| v.value).collect();
        JsonData::from_value(JsonValue::Array(array))
    }
    
    /// Aggregate JSON objects by merging
    pub fn json_object_agg(pairs: Vec<(String, JsonData)>) -> JsonData {
        let mut obj = serde_json::Map::new();
        for (key, value) in pairs {
            obj.insert(key, value.value);
        }
        JsonData::from_value(JsonValue::Object(obj))
    }
}

/// JSON builder for constructing JSON values
pub struct JsonBuilder;

impl JsonBuilder {
    /// Build JSON object from key-value pairs
    pub fn build_object(pairs: Vec<(String, JsonData)>) -> JsonData {
        let mut obj = serde_json::Map::new();
        for (key, value) in pairs {
            obj.insert(key, value.value);
        }
        JsonData::from_value(JsonValue::Object(obj))
    }
    
    /// Build JSON array from values
    pub fn build_array(values: Vec<JsonData>) -> JsonData {
        let array: Vec<JsonValue> = values.into_iter().map(|v| v.value).collect();
        JsonData::from_value(JsonValue::Array(array))
    }
}

/// JSON validation
pub struct JsonValidator;

impl JsonValidator {
    /// Validate JSON against a schema (simplified)
    pub fn validate(json: &JsonData, schema: &JsonSchema) -> Result<()> {
        Self::validate_value(&json.value, &schema.schema)
    }
    
    fn validate_value(value: &JsonValue, schema: &JsonValue) -> Result<()> {
        // Simplified validation - check type
        if let Some(expected_type) = schema.get("type").and_then(|v| v.as_str()) {
            let actual_type = match value {
                JsonValue::Null => "null",
                JsonValue::Bool(_) => "boolean",
                JsonValue::Number(_) => "number",
                JsonValue::String(_) => "string",
                JsonValue::Array(_) => "array",
                JsonValue::Object(_) => "object",
            };
            
            if expected_type != actual_type {
                return Err(DbError::InvalidInput(format!(
                    "Type mismatch: expected {}, got {}",
                    expected_type, actual_type
                )))));
            }
        }
        
        Ok(())
    }
}

/// JSON schema definition
pub struct JsonSchema {
    schema: JsonValue,
}

impl JsonSchema {
    pub fn from_str(s: &str) -> Result<Self> {
        let schema = serde_json::from_str(s)
            .map_err(|e| DbError::InvalidInput(format!("Invalid JSON schema: {}", e)))?);
        Ok(Self { schema })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_json_data_creation() {
        let json = JsonData::from_str(r#"{"name": "Alice", "age": 30}"#).unwrap();
        assert_eq!(json.json_type(), JsonType::Object);
    }
    
    #[test]
    fn test_json_path_extract() {
        let json = JsonData::from_str(r#"{"user": {"name": "Alice", "age": 30}}"#).unwrap();
        
        let name = JsonPath::extract(&json, "$.user.name").unwrap();
        assert_eq!(name.to_string(), r#""Alice""#);
    }
    
    #[test]
    fn test_json_path_array() {
        let json = JsonData::from_str(r#"{"items": [{"id": 1}, {"id": 2}]}"#).unwrap();
        
        let first_item = JsonPath::extract(&json, "$.items[0]").unwrap();
        assert!(first_item.value().is_object());
        
        let first_id = JsonPath::extract(&json, "$.items[0].id").unwrap();
        assert_eq!(first_id.to_string(), "1");
    }
    
    #[test]
    fn test_json_path_wildcard() {
        let json = JsonData::from_str(r#"{"items": [{"price": 10}, {"price": 20}]}"#).unwrap();
        
        let prices = JsonPath::extract_all(&json, "$.items[*].price").unwrap();
        assert_eq!(prices.len(), 2);
    }
    
    #[test]
    fn test_json_set() {
        let json = JsonData::from_str(r#"{"name": "Alice"}"#).unwrap();
        let new_value = JsonData::from_str(r#""Bob""#).unwrap();
        
        let updated = JsonOperators::json_set(&json, "$.name", new_value).unwrap();
        let extracted = JsonPath::extract(&updated, "$.name").unwrap();
        assert_eq!(extracted.to_string(), r#""Bob""#);
    }
    
    #[test]
    fn test_json_delete() {
        let json = JsonData::from_str(r#"{"name": "Alice", "age": 30}"#).unwrap();
        
        let updated = JsonOperators::json_delete(&json, "$.age").unwrap();
        assert!(JsonPath::extract(&updated, "$.age").is_err());
        assert!(JsonPath::extract(&updated, "$.name").is_ok());
    }
    
    #[test]
    fn test_json_contains() {
        let json = JsonData::from_str(r#"{"items": [1, 2, 3]}"#).unwrap();
        let search = JsonData::from_str("2").unwrap();
        
        assert!(JsonOperators::json_contains(&json, &search));
        
        let not_found = JsonData::from_str("5").unwrap();
        assert!(!JsonOperators::json_contains(&json, &not_found));
    }
    
    #[test]
    fn test_json_array_length() {
        let json = JsonData::from_str(r#"[1, 2, 3, 4, 5]"#).unwrap();
        let length = JsonOperators::json_array_length(&json).unwrap();
        assert_eq!(length, 5);
    }
    
    #[test]
    fn test_json_keys() {
        let json = JsonData::from_str(r#"{"name": "Alice", "age": 30}"#).unwrap();
        let keys = JsonOperators::json_keys(&json).unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"name".to_string()));
        assert!(keys.contains(&"age".to_string()));
    }
    
    #[test]
    fn test_json_merge() {
        let json1 = JsonData::from_str(r#"{"a": 1, "b": 2}"#).unwrap();
        let json2 = JsonData::from_str(r#"{"b": 3, "c": 4}"#).unwrap();
        
        let merged = JsonOperators::json_merge(&json1, &json2).unwrap();
        
        let b_value = JsonPath::extract(&merged, "$.b").unwrap();
        assert_eq!(b_value.to_string(), "3"); // json2 value wins
        
        let c_value = JsonPath::extract(&merged, "$.c").unwrap();
        assert_eq!(c_value.to_string(), "4");
    }
    
    #[test]
    fn test_json_aggregation() {
        let values = vec![
            JsonData::from_str("1").unwrap(),
            JsonData::from_str("2").unwrap(),
            JsonData::from_str("3").unwrap(),
        ];
        
        let array = JsonAggregation::json_agg(values);
        assert_eq!(array.json_type(), JsonType::Array);
        
        let length = JsonOperators::json_array_length(&array).unwrap();
        assert_eq!(length, 3);
    }
    
    #[test]
    fn test_json_builder() {
        let obj = JsonBuilder::build_object(vec![
            ("name".to_string(), JsonData::from_str(r#""Alice""#).unwrap()),
            ("age".to_string(), JsonData::from_str("30").unwrap()),
        ]);
        
        assert_eq!(obj.json_type(), JsonType::Object);
        
        let name = JsonPath::extract(&obj, "$.name").unwrap();
        assert_eq!(name.to_string(), r#""Alice""#);
    }
}


