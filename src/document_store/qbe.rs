// # Query By Example (QBE)
//
// MongoDB-like query syntax for document queries with comparison operators,
// logical operators, array operators, and regular expressions.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{HashMap};
use crate::error::{Result, DbError};
use super::document::{Document, DocumentId};
use super::jsonpath::JsonPathEvaluator;

/// Query document for Query By Example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryDocument {
    /// Query conditions
    #[serde(flatten)]
    pub conditions: HashMap<String, Value>,
}

impl QueryDocument {
    /// Create a new empty query
    pub fn new() -> Self {
        Self {
            conditions: HashMap::new(),
        }
    }

    /// Create from JSON value
    pub fn from_json(value: Value) -> Result<Self> {
        if let Value::Object(obj) = value {
            Ok(Self {
                conditions: obj.into_iter().collect(),
            })
        } else {
            Err(crate::error::DbError::InvalidInput(
                "Query must be a JSON object".to_string()
            ))
        }
    }

    /// Add a condition
    pub fn add_condition(&mut self, field: String, value: Value) {
        self.conditions.insert(field, value);
    }

    /// Evaluate query against a document
    pub fn matches(&self, doc: &Document) -> Result<bool> {
        let json = doc.as_json()?;
        self.matches_value(&json)
    }

    /// Evaluate query against a JSON value
    pub fn matches_value(&self, value: &Value) -> Result<bool> {
        for (field, condition) in &self.conditions {
            if !self.evaluate_condition(field, condition, value)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn evaluate_condition(&self, field: &str, condition: &Value, doc: &Value) -> Result<bool> {
        // Handle logical operators
        if field == "$and" {
            return self.evaluate_and(condition, doc);
        }
        if field == "$or" {
            return self.evaluate_or(condition, doc);
        }
        if field == "$nor" {
            return self.evaluate_nor(condition, doc);
        }
        if field == "$not" {
            return self.evaluate_not(condition, doc);
        }

        // Get field value from document
        let field_value = self.get_field_value(field, doc)?;

        // Handle operator conditions
        if let Value::Object(operators) = condition {
            self.evaluate_operators(&field_value, operators)
        } else {
            // Simple equality check
            Ok(field_value == *condition)
        }
    }

    fn evaluate_operators(&self, field_value: &Value, operators: &serde_json::Map<String, Value>) -> Result<bool> {
        for (op, value) in operators {
            let result = match op.as_str() {
                "$eq" => self.op_eq(field_value, value),
                "$ne" => self.op_ne(field_value, value),
                "$gt" => self.op_gt(field_value, value),
                "$gte" => self.op_gte(field_value, value),
                "$lt" => self.op_lt(field_value, value),
                "$lte" => self.op_lte(field_value, value),
                "$in" => self.op_in(field_value, value),
                "$nin" => self.op_nin(field_value, value),
                "$exists" => self.op_exists(field_value, value),
                "$type" => self.op_type(field_value, value),
                "$regex" => self.op_regex(field_value, value),
                "$mod" => self.op_mod(field_value, value),
                "$size" => self.op_size(field_value, value),
                "$all" => self.op_all(field_value, value),
                "$elemMatch" => self.op_elem_match(field_value, value)?,
                _ => {
                    return Err(crate::error::DbError::InvalidInput(
                        format!("Unknown operator: {}", op)
                    ));
                }
            };

            if !result {
                return Ok(false);
            }
        }
        Ok(true)
    }

    // Comparison operators
    fn op_eq(&self, field_value: &Value, query_value: &Value) -> bool {
        field_value == query_value
    }

    fn op_ne(&self, field_value: &Value, query_value: &Value) -> bool {
        field_value != query_value
    }

    fn op_gt(&self, field_value: &Value, query_value: &Value) -> bool {
        compare_values(field_value, query_value) == Some(std::cmp::Ordering::Greater)
    }

    fn op_gte(&self, field_value: &Value, query_value: &Value) -> bool {
        matches!(
            compare_values(field_value, query_value),
            Some(std::cmp::Ordering::Greater) | Some(std::cmp::Ordering::Equal)
        )
    }

    fn op_lt(&self, field_value: &Value, query_value: &Value) -> bool {
        compare_values(field_value, query_value) == Some(std::cmp::Ordering::Less)
    }

    fn op_lte(&self, field_value: &Value, query_value: &Value) -> bool {
        matches!(
            compare_values(field_value, query_value),
            Some(std::cmp::Ordering::Less) | Some(std::cmp::Ordering::Equal)
        )
    }

    // Array operators
    fn op_in(&self, field_value: &Value, query_value: &Value) -> bool {
        if let Value::Array(arr) = query_value {
            arr.contains(field_value)
        } else {
            false
        }
    }

    fn op_nin(&self, field_value: &Value, query_value: &Value) -> bool {
        !self.op_in(field_value, query_value)
    }

    fn op_all(&self, field_value: &Value, query_value: &Value) -> bool {
        if let (Value::Array(field_arr), Value::Array(query_arr)) = (field_value, query_value) {
            query_arr.iter().all(|q| field_arr.contains(q))
        } else {
            false
        }
    }

    fn op_elem_match(&self, field_value: &Value, query_value: &Value) -> Result<bool> {
        if let Value::Array(arr) = field_value {
            if let Value::Object(conditions) = query_value {
                for elem in arr {
                    let mut matches = true;
                    for (field, condition) in conditions {
                        if !self.evaluate_condition(field, condition, elem)? {
                            matches = false;
                            break;
                        }
                    }
                    if matches {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    fn op_size(&self, field_value: &Value, query_value: &Value) -> bool {
        if let (Value::Array(arr), Value::Number(n)) = (field_value, query_value) {
            if let Some(size) = n.as_u64() {
                return arr.len() == size as usize;
            }
        }
        false
    }

    // Type and existence operators
    fn op_exists(&self, field_value: &Value, query_value: &Value) -> bool {
        if let Value::Bool(should_exist) = query_value {
            let exists = !matches!(field_value, Value::Null);
            exists == *should_exist
        } else {
            false
        }
    }

    fn op_type(&self, field_value: &Value, query_value: &Value) -> bool {
        if let Value::String(type_name) = query_value {
            match type_name.as_str() {
                "null" => matches!(field_value, Value::Null),
                "bool" | "boolean" => matches!(field_value, Value::Bool(_)),
                "number" => matches!(field_value, Value::Number(_)),
                "string" => matches!(field_value, Value::String(_)),
                "array" => matches!(field_value, Value::Array(_)),
                "object" => matches!(field_value, Value::Object(_)),
                _ => false,
            }
        } else {
            false
        }
    }

    // String operators
    fn op_regex(&self, field_value: &Value, query_value: &Value) -> bool {
        if let (Value::String(text), Value::String(pattern)) = (field_value, query_value) {
            regex::Regex::new(pattern)
                .map(|re| re.is_match(text))
                .unwrap_or(false)
        } else {
            false
        }
    }

    // Math operators
    fn op_mod(&self, field_value: &Value, query_value: &Value) -> bool {
        if let (Value::Number(n), Value::Array(arr)) = (field_value, query_value) {
            if arr.len() == 2 {
                if let (Some(divisor), Some(remainder)) = (
                    arr[0].as_i64(),
                    arr[1].as_i64(),
                ) {
                    if let Some(num) = n.as_i64() {
                        return num % divisor == remainder;
                    }
                }
            }
        }
        false
    }

    // Logical operators
    fn evaluate_and(&self, condition: &Value, doc: &Value) -> Result<bool> {
        if let Value::Array(conditions) = condition {
            for cond in conditions {
                if let Value::Object(obj) = cond {
                    for (field, value) in obj {
                        if !self.evaluate_condition(field, value, doc)? {
                            return Ok(false);
                        }
                    }
                }
            }
            Ok(true)
        } else {
            Err(crate::error::DbError::InvalidInput(
                "$and requires an array".to_string()
            ))
        }
    }

    fn evaluate_or(&self, condition: &Value, doc: &Value) -> Result<bool> {
        if let Value::Array(conditions) = condition {
            for cond in conditions {
                if let Value::Object(obj) = cond {
                    let mut all_match = true;
                    for (field, value) in obj {
                        if !self.evaluate_condition(field, value, doc)? {
                            all_match = false;
                            break;
                        }
                    }
                    if all_match {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        } else {
            Err(crate::error::DbError::InvalidInput(
                "$or requires an array".to_string()
            ))
        }
    }

    fn evaluate_nor(&self, condition: &Value, doc: &Value) -> Result<bool> {
        Ok(!self.evaluate_or(condition, doc)?)
    }

    fn evaluate_not(&self, condition: &Value, doc: &Value) -> Result<bool> {
        if let Value::Object(obj) = condition {
            for (field, value) in obj {
                if self.evaluate_condition(field, value, doc)? {
                    return Ok(false);
                }
            }
            Ok(true)
        } else {
            Err(crate::error::DbError::InvalidInput(
                "$not requires an object".to_string()
            ))
        }
    }

    fn get_field_value(&self, field: &str, doc: &Value) -> Result<Value> {
        let path = if field.starts_with('$') {
            field.to_string()
        } else {
            format!("$.{}", field)
        };

        let mut parser = super::jsonpath::JsonPathParser::new(path);
        let json_path = parser.parse()?;
        let results = JsonPathEvaluator::evaluate(&json_path, doc)?;

        if results.is_empty() {
            Ok(Value::Null)
        } else {
            Ok(results[0].clone())
        }
    }
}

impl Default for QueryDocument {
    fn default() -> Self {
        Self::new()
    }
}

/// Compare two JSON values
fn compare_values(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Number(n1), Value::Number(n2)) => {
            let f1 = n1.as_f64()?;
            let f2 = n2.as_f64()?;
            f1.partial_cmp(&f2)
        }
        (Value::String(s1), Value::String(s2)) => Some(s1.cmp(s2)),
        (Value::Bool(b1), Value::Bool(b2)) => Some(b1.cmp(b2)),
        _ => None,
    }
}

/// Query executor for executing queries on collections
pub struct QueryExecutor;

impl QueryExecutor {
    /// Execute a query on a collection of documents
    pub fn execute(
        query: &QueryDocument,
        documents: &HashMap<DocumentId, Document>,
    ) -> Result<Vec<DocumentId>> {
        let mut results = Vec::new();

        for (doc_id, doc) in documents {
            if query.matches(doc)? {
                results.push(doc_id.clone());
            }
        }

        Ok(results)
    }

    /// Execute a query with projection
    pub fn execute_with_projection(
        query: &QueryDocument,
        documents: &HashMap<DocumentId, Document>,
        projection: &Projection,
    ) -> Result<Vec<(DocumentId, Value)>> {
        let mut results = Vec::new();

        for (doc_id, doc) in documents {
            if query.matches(doc)? {
                let json = doc.as_json()?;
                let projected = projection.apply(&json)?;
                results.push((doc_id.clone(), projected));
            }
        }

        Ok(results)
    }
}

/// Projection specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projection {
    /// Fields to include (true) or exclude (false)
    pub fields: HashMap<String, bool>,
}

impl Projection {
    /// Create a new projection
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    /// Create from JSON value
    pub fn from_json(value: Value) -> Result<Self> {
        if let Value::Object(obj) = value {
            let mut fields = HashMap::new();
            for (key, val) in obj {
                if let Value::Bool(b) = val {
                    fields.insert(key, b);
                } else if let Value::Number(n) = val {
                    fields.insert(key, n.as_i64().unwrap_or(0) != 0);
                }
            }
            Ok(Self { fields })
        } else {
            Err(crate::error::DbError::InvalidInput(
                "Projection must be a JSON object".to_string()
            ))
        }
    }

    /// Include a field
    pub fn include(&mut self, field: String) {
        self.fields.insert(field, true);
    }

    /// Exclude a field
    pub fn exclude(&mut self, field: String) {
        self.fields.insert(field, false);
    }

    /// Apply projection to a JSON value
    pub fn apply(&self, value: &Value) -> Result<Value> {
        if self.fields.is_empty() {
            return Ok(value.clone());
        }

        // Determine if this is an inclusion or exclusion projection
        let is_inclusion = self.fields.values().any(|&v| v);

        if let Value::Object(obj) = value {
            let mut result = serde_json::Map::new();

            if is_inclusion {
                // Include only specified fields
                for (field, &include) in &self.fields {
                    if include {
                        if let Some(value) = obj.get(field) {
                            result.insert(field.clone(), value.clone());
                        }
                    }
                }
            } else {
                // Exclude specified fields
                for (key, value) in obj {
                    if !self.fields.get(key).copied().unwrap_or(false) {
                        result.insert(key.clone(), value.clone());
                    }
                }
            }

            Ok(Value::Object(result))
        } else {
            Ok(value.clone())
        }
    }
}

impl Default for Projection {
    fn default() -> Self {
        Self::new()
    }
}

/// Geospatial query support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoQuery {
    /// Query type
    pub query_type: GeoQueryType,
    /// Coordinates
    pub coordinates: Vec<f64>,
    /// Maximum distance (for $near)
    pub max_distance: Option<f64>,
    /// Minimum distance (for $near)
    pub min_distance: Option<f64>,
}

/// Geospatial query types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeoQueryType {
    /// Find points near a location
    Near,
    /// Find points within a bounding box
    Within,
    /// Find points that intersect with a geometry
    Intersects,
}

impl GeoQuery {
    /// Create a $near query
    pub fn near(longitude: f64, latitude: f64) -> Self {
        Self {
            query_type: GeoQueryType::Near,
            coordinates: vec![longitude, latitude],
            max_distance: None,
            min_distance: None,
        }
    }

    /// Set maximum distance
    pub fn max_distance(mut self, distance: f64) -> Self {
        self.max_distance = Some(distance);
        self
    }

    /// Set minimum distance
    pub fn min_distance(mut self, distance: f64) -> Self {
        self.min_distance = Some(distance);
        self
    }

    /// Check if a point matches this geospatial query
    pub fn matches(&self, point: &[f64]) -> bool {
        if point.len() < 2 {
            return false;
        }

        match self.query_type {
            GeoQueryType::Near => {
                let distance = self.haversine_distance(point);

                if let Some(max) = self.max_distance {
                    if distance > max {
                        return false;
                    }
                }

                if let Some(min) = self.min_distance {
                    if distance < min {
                        return false;
                    }
                }

                true
            }
            _ => false,
        }
    }

    /// Calculate Haversine distance in meters
    fn haversine_distance(&self, point: &[f64]) -> f64 {
        const EARTH_RADIUS_METERS: f64 = 6371000.0;

        let lon1 = self.coordinates[0].to_radians();
        let lat1 = self.coordinates[1].to_radians();
        let lon2 = point[0].to_radians();
        let lat2 = point[1].to_radians();

        let dlat = lat2 - lat1;
        let dlon = lon2 - lon1;

        let a = (dlat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        EARTH_RADIUS_METERS * c
    }
}

/// Query builder for fluent query construction
pub struct QueryBuilder {
    query: QueryDocument,
}

impl QueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            query: QueryDocument::new(),
        }
    }

    /// Add equality condition
    pub fn eq(mut self, field: impl Into<String>, value: Value) -> Self {
        self.query.add_condition(field.into(), value);
        self
    }

    /// Add not-equal condition
    pub fn ne(mut self, field: impl Into<String>, value: Value) -> Self {
        self.query.add_condition(
            field.into(),
            serde_json::json!({"$ne": value}),
        );
        self
    }

    /// Add greater-than condition
    pub fn gt(mut self, field: impl Into<String>, value: Value) -> Self {
        self.query.add_condition(
            field.into(),
            serde_json::json!({"$gt": value}),
        );
        self
    }

    /// Add greater-than-or-equal condition
    pub fn gte(mut self, field: impl Into<String>, value: Value) -> Self {
        self.query.add_condition(
            field.into(),
            serde_json::json!({"$gte": value}),
        );
        self
    }

    /// Add less-than condition
    pub fn lt(mut self, field: impl Into<String>, value: Value) -> Self {
        self.query.add_condition(
            field.into(),
            serde_json::json!({"$lt": value}),
        );
        self
    }

    /// Add less-than-or-equal condition
    pub fn lte(mut self, field: impl Into<String>, value: Value) -> Self {
        self.query.add_condition(
            field.into(),
            serde_json::json!({"$lte": value}),
        );
        self
    }

    /// Add in-array condition
    pub fn in_array(mut self, field: impl Into<String>, values: Vec<Value>) -> Self {
        self.query.add_condition(
            field.into(),
            serde_json::json!({"$in": values}),
        );
        self
    }

    /// Add regex condition
    pub fn regex(mut self, field: impl Into<String>, pattern: impl Into<String>) -> Self {
        self.query.add_condition(
            field.into(),
            serde_json::json!({"$regex": pattern.into()}),
        );
        self
    }

    /// Add exists condition
    pub fn exists(mut self, field: impl Into<String>, exists: bool) -> Self {
        self.query.add_condition(
            field.into(),
            serde_json::json!({"$exists": exists}),
        );
        self
    }

    /// Build the query
    pub fn build(self) -> QueryDocument {
        self.query
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_equality_query() {
        let query = QueryDocument::from_json(json!({
            "name": "Alice"
        })).unwrap();

        let doc = json!({
            "name": "Alice",
            "age": 30
        });

        assert!(query.matches_value(&doc).unwrap());
    }

    #[test]
    fn test_comparison_operators() {
        let query = QueryDocument::from_json(json!({
            "age": {"$gt": 25, "$lt": 35}
        })).unwrap();

        let doc1 = json!({"age": 30});
        let doc2 = json!({"age": 20});

        assert!(query.matches_value(&doc1).unwrap());
        assert!(!query.matches_value(&doc2).unwrap());
    }

    #[test]
    fn test_array_operators() {
        let query = QueryDocument::from_json(json!({
            "tags": {"$in": ["rust", "database"]}
        })).unwrap();

        let doc = json!({"tags": "rust"});
        assert!(query.matches_value(&doc).unwrap());
    }

    #[test]
    fn test_logical_operators() {
        let query = QueryDocument::from_json(json!({
            "$or": [
                {"age": {"$lt": 25}},
                {"age": {"$gt": 35}}
            ]
        })).unwrap();

        let doc1 = json!({"age": 20});
        let doc2 = json!({"age": 30});

        assert!(query.matches_value(&doc1).unwrap());
        assert!(!query.matches_value(&doc2).unwrap());
    }

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::new()
            .eq("name", json!("Alice"))
            .gte("age", json!(18))
            .lt("age", json!(65))
            .build();

        let doc = json!({
            "name": "Alice",
            "age": 30
        });

        assert!(query.matches_value(&doc).unwrap());
    }

    #[test]
    fn test_projection() {
        let mut projection = Projection::new();
        projection.include("name".to_string());
        projection.include("age".to_string());

        let doc = json!({
            "name": "Alice",
            "age": 30,
            "email": "alice@example.com"
        });

        let result = projection.apply(&doc).unwrap();
        assert!(result.get("name").is_some());
        assert!(result.get("age").is_some());
        assert!(result.get("email").is_none());
    }
}
