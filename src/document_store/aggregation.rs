// # Aggregation Pipeline
//
// MongoDB-style aggregation pipeline with stages like $match, $project, $group,
// $sort, $limit, $skip, $unwind, $lookup, and $facet.

use std::collections::HashSet;
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{HashMap};
use crate::error::Result;
use super::document::{Document, DocumentId};
use super::qbe::QueryDocument;

/// Aggregation pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    /// Pipeline stages
    pub stages: Vec<PipelineStage>,
}

impl Pipeline {
    /// Create a new empty pipeline
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
        }
    }

    /// Add a stage to the pipeline
    pub fn add_stage(mut self, stage: PipelineStage) -> Self {
        self.stages.push(stage);
        self
    }

    /// Execute the pipeline on a collection of documents
    pub fn execute(&self, documents: &HashMap<DocumentId, Document>) -> Result<Vec<Value>> {
        let mut results: Vec<Value> = documents
            .values()
            .map(|doc| doc.as_json())
            .collect::<Result<Vec<_>>>()?;

        for stage in &self.stages {
            results = stage.execute(results)?;
        }

        Ok(results)
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Pipeline stage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PipelineStage {
    /// Match documents by query
    #[serde(rename = "$match")]
    Match { query: Value },

    /// Project (reshape) documents
    #[serde(rename = "$project")]
    Project { projection: Value },

    /// Group documents by key
    #[serde(rename = "$group")]
    Group { group_spec: GroupSpec },

    /// Sort documents
    #[serde(rename = "$sort")]
    Sort { sort_spec: BTreeMap<String, i32> },

    /// Limit number of documents
    #[serde(rename = "$limit")]
    Limit { count: usize },

    /// Skip documents
    #[serde(rename = "$skip")]
    Skip { count: usize },

    /// Unwind array field
    #[serde(rename = "$unwind")]
    Unwind { path: String, preserve_null_and_empty: bool },

    /// Lookup/join from another collection
    #[serde(rename = "$lookup")]
    Lookup { lookup_spec: LookupSpec },

    /// Multi-faceted aggregation
    #[serde(rename = "$facet")]
    Facet { facets: HashMap<String, Vec<PipelineStage>> },

    /// Add computed fields
    #[serde(rename = "$addFields")]
    AddFields { fields: HashMap<String, Expression> },

    /// Count documents
    #[serde(rename = "$count")]
    Count { field: String },

    /// Replace root document
    #[serde(rename = "$replaceRoot")]
    ReplaceRoot { new_root: String },
}

impl PipelineStage {
    /// Execute this stage
    pub fn execute(&self, documents: Vec<Value>) -> Result<Vec<Value>> {
        match self {
            PipelineStage::Match { query } => self.execute_match(documents, query),
            PipelineStage::Project { projection } => self.execute_project(documents, projection),
            PipelineStage::Group { group_spec } => self.execute_group(documents, group_spec),
            PipelineStage::Sort { sort_spec } => self.execute_sort(documents, sort_spec),
            PipelineStage::Limit { count } => Ok(self.execute_limit(documents, *count)),
            PipelineStage::Skip { count } => Ok(self.execute_skip(documents, *count)),
            PipelineStage::Unwind { path, preserve_null_and_empty } => {
                self.execute_unwind(documents, path, *preserve_null_and_empty)
            }
            PipelineStage::Lookup { lookup_spec } => self.execute_lookup(documents, lookup_spec),
            PipelineStage::Facet { facets } => self.execute_facet(documents, facets),
            PipelineStage::AddFields { fields } => self.execute_add_fields(documents, fields),
            PipelineStage::Count { field } => self.execute_count(documents, field),
            PipelineStage::ReplaceRoot { new_root } => self.execute_replace_root(documents, new_root),
        }
    }

    fn execute_match(&self, documents: Vec<Value>, query: &Value) -> Result<Vec<Value>> {
        let query_doc = QueryDocument::from_json(query.clone())?;
        let mut results = Vec::new();

        for doc in documents {
            if query_doc.matches_value(&doc)? {
                results.push(doc);
            }
        }

        Ok(results)
    }

    fn execute_project(&self, documents: Vec<Value>, projection: &Value) -> Result<Vec<Value>> {
        let mut results = Vec::new();

        for doc in documents {
            if let Value::Object(obj) = doc {
                let mut projected = serde_json::Map::new();

                if let Value::Object(proj_spec) = projection {
                    for (key, value) in proj_spec {
                        if let Value::Bool(include) = value {
                            if *include {
                                if let Some(field_value) = obj.get(key) {
                                    projected.insert(key.clone(), field_value.clone());
                                }
                            }
                        } else if let Value::Number(n) = value {
                            if n.as_i64().unwrap_or(0) != 0 {
                                if let Some(field_value) = obj.get(key) {
                                    projected.insert(key.clone(), field_value.clone());
                                }
                            }
                        } else {
                            // Expression-based projection
                            let expr = Expression::from_value(value.clone());
                            let result = expr.evaluate(&Value::Object(obj.clone()))?;
                            projected.insert(key.clone(), result);
                        }
                    }
                }

                results.push(Value::Object(projected));
            }
        }

        Ok(results)
    }

    fn execute_group(&self, documents: Vec<Value>, group_spec: &GroupSpec) -> Result<Vec<Value>> {
        let mut groups: HashMap<Value, Vec<Value>> = HashMap::new();

        // Group documents by _id expression
        for doc in documents {
            let group_key = group_spec.id.evaluate(&doc)?;
            groups.entry(group_key).or_insert_with(Vec::new).push(doc);
        }

        // Compute aggregations for each group
        let mut results = Vec::new();

        for (key, group_docs) in groups {
            let mut result = serde_json::Map::new();
            result.insert("_id".to_string(), key);

            for (field, accumulator) in &group_spec.accumulators {
                let value = accumulator.compute(&group_docs)?;
                result.insert(field.clone(), value);
            }

            results.push(Value::Object(result));
        }

        Ok(results)
    }

    fn execute_sort(&self, mut documents: Vec<Value>, sort_spec: &BTreeMap<String, i32>) -> Result<Vec<Value>> {
        documents.sort_by(|a, b| {
            for (field, order) in sort_spec {
                let a_val = get_field_value(a, field);
                let b_val = get_field_value(b, field);

                let cmp = compare_json_values(&a_val, &b_val);
                let cmp = if *order < 0 { cmp.reverse() } else { cmp };

                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
            }
            std::cmp::Ordering::Equal
        });

        Ok(documents)
    }

    fn execute_limit(&self, documents: Vec<Value>, count: usize) -> Vec<Value> {
        documents.into_iter().take(count).collect()
    }

    fn execute_skip(&self, documents: Vec<Value>, count: usize) -> Vec<Value> {
        documents.into_iter().skip(count).collect()
    }

    fn execute_unwind(&self, documents: Vec<Value>, path: &str, preserve: bool) -> Result<Vec<Value>> {
        let mut results = Vec::new();
        let field = path.trim_start_matches('$');

        for doc in documents {
            if let Value::Object(mut obj) = doc.clone() {
                if let Some(array_value) = obj.get(field) {
                    if let Value::Array(arr) = array_value {
                        if arr.is_empty() && preserve {
                            results.push(doc);
                        } else {
                            for item in arr {
                                let mut unwound = obj.clone();
                                unwound.insert(field.to_string(), item.clone());
                                results.push(Value::Object(unwound));
                            }
                        }
                    } else {
                        results.push(doc);
                    }
                } else if preserve {
                    results.push(doc);
                }
            } else {
                results.push(doc);
            }
        }

        Ok(results)
    }

    fn execute_lookup(&self, documents: Vec<Value>, _lookup_spec: &LookupSpec) -> Result<Vec<Value>> {
        // Simplified lookup implementation
        // In a real implementation, this would join with another collection
        Ok(documents)
    }

    fn execute_facet(&self, documents: Vec<Value>, facets: &HashMap<String, Vec<PipelineStage>>) -> Result<Vec<Value>> {
        let mut result = serde_json::Map::new();

        for (facet_name, stages) in facets {
            let mut facet_results = documents.clone();

            for stage in stages {
                facet_results = stage.execute(facet_results)?;
            }

            result.insert(facet_name.clone(), Value::Array(facet_results));
        }

        Ok(vec![Value::Object(result)])
    }

    fn execute_add_fields(&self, documents: Vec<Value>, fields: &HashMap<String, Expression>) -> Result<Vec<Value>> {
        let mut results = Vec::new();

        for doc in documents {
            if let Value::Object(mut obj) = doc {
                for (field, expr) in fields {
                    let value = expr.evaluate(&Value::Object(obj.clone()))?;
                    obj.insert(field.clone(), value);
                }
                results.push(Value::Object(obj));
            } else {
                results.push(doc);
            }
        }

        Ok(results)
    }

    fn execute_count(&self, documents: Vec<Value>, field: &str) -> Result<Vec<Value>> {
        let mut result = serde_json::Map::new();
        result.insert(field.to_string(), Value::Number(documents.len().into()));
        Ok(vec![Value::Object(result)])
    }

    fn execute_replace_root(&self, documents: Vec<Value>, new_root: &str) -> Result<Vec<Value>> {
        let mut results = Vec::new();
        let field = new_root.trim_start_matches('$');

        for doc in documents {
            if let Some(new_doc) = get_field_value(&doc, field).as_object() {
                results.push(Value::Object(new_doc.clone()));
            }
        }

        Ok(results)
    }
}

/// Group specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupSpec {
    /// Group key expression
    #[serde(rename = "_id")]
    pub id: Expression,
    /// Accumulator expressions
    #[serde(flatten)]
    pub accumulators: HashMap<String, Accumulator>,
}

/// Expression for computed values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Expression {
    /// Field reference (starts with $)
    Field(String),
    /// Literal value
    Literal(Value),
    /// Complex expression
    Complex(Box<ComplexExpression>),
}

impl Expression {
    /// Create from JSON value
    pub fn from_value(value: Value) -> Self {
        if let Value::String(s) = &value {
            if s.starts_with('$') {
                return Expression::Field(s.clone());
            }
        }
        Expression::Literal(value)
    }

    /// Evaluate expression against a document
    pub fn evaluate(&self, doc: &Value) -> Result<Value> {
        match self {
            Expression::Field(field) => {
                let field_name = field.trim_start_matches('$');
                Ok(get_field_value(doc, field_name))
            }
            Expression::Literal(value) => Ok(value.clone()),
            Expression::Complex(expr) => expr.evaluate(doc),
        }
    }
}

/// Complex expression types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum ComplexExpression {
    /// Addition
    #[serde(rename = "$add")]
    Add { values: Vec<Expression> },
    /// Subtraction
    #[serde(rename = "$subtract")]
    Subtract { values: Vec<Expression> },
    /// Multiplication
    #[serde(rename = "$multiply")]
    Multiply { values: Vec<Expression> },
    /// Division
    #[serde(rename = "$divide")]
    Divide { dividend: Expression, divisor: Expression },
    /// Concatenation
    #[serde(rename = "$concat")]
    Concat { values: Vec<Expression> },
    /// Conditional
    #[serde(rename = "$cond")]
    Cond { if_expr: Expression, then_expr: Expression, else_expr: Expression },
}

impl ComplexExpression {
    /// Evaluate complex expression
    pub fn evaluate(&self, doc: &Value) -> Result<Value> {
        match self {
            ComplexExpression::Add { values } => {
                let mut sum = 0.0;
                for expr in values {
                    let val = expr.evaluate(doc)?;
                    if let Some(n) = val.as_f64() {
                        sum += n;
                    }
                }
                Ok(Value::Number(serde_json::Number::from_f64(sum).unwrap()))
            }
            ComplexExpression::Subtract { values } => {
                if values.is_empty() {
                    return Ok(Value::Number(0.into()));
                }
                let first = values[0].evaluate(doc)?;
                let mut result = first.as_f64().unwrap_or(0.0);
                for expr in &values[1..] {
                    let val = expr.evaluate(doc)?;
                    result -= val.as_f64().unwrap_or(0.0);
                }
                Ok(Value::Number(serde_json::Number::from_f64(result).unwrap()))
            }
            ComplexExpression::Multiply { values } => {
                let mut product = 1.0;
                for expr in values {
                    let val = expr.evaluate(doc)?;
                    if let Some(n) = val.as_f64() {
                        product *= n;
                    }
                }
                Ok(Value::Number(serde_json::Number::from_f64(product).unwrap()))
            }
            ComplexExpression::Divide { dividend, divisor } => {
                let num = dividend.evaluate(doc)?.as_f64().unwrap_or(0.0);
                let den = divisor.evaluate(doc)?.as_f64().unwrap_or(1.0);
                let result = if den != 0.0 { num / den } else { 0.0 };
                Ok(Value::Number(serde_json::Number::from_f64(result).unwrap()))
            }
            ComplexExpression::Concat { values } => {
                let mut result = String::new();
                for expr in values {
                    let val = expr.evaluate(doc)?;
                    if let Some(s) = val.as_str() {
                        result.push_str(s);
                    }
                }
                Ok(Value::String(result))
            }
            ComplexExpression::Cond { if_expr, then_expr, else_expr } => {
                let condition = if_expr.evaluate(doc)?;
                if condition.as_bool().unwrap_or(false) {
                    then_expr.evaluate(doc)
                } else {
                    else_expr.evaluate(doc)
                }
            }
        }
    }
}

/// Accumulator for group operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum Accumulator {
    /// Sum of values
    #[serde(rename = "$sum")]
    Sum { expr: Expression },
    /// Average of values
    #[serde(rename = "$avg")]
    Avg { expr: Expression },
    /// Minimum value
    #[serde(rename = "$min")]
    Min { expr: Expression },
    /// Maximum value
    #[serde(rename = "$max")]
    Max { expr: Expression },
    /// Count of documents
    #[serde(rename = "$count")]
    Count,
    /// First value
    #[serde(rename = "$first")]
    First { expr: Expression },
    /// Last value
    #[serde(rename = "$last")]
    Last { expr: Expression },
    /// Push to array
    #[serde(rename = "$push")]
    Push { expr: Expression },
    /// Add to set (unique values)
    #[serde(rename = "$addToSet")]
    AddToSet { expr: Expression },
}

impl Accumulator {
    /// Compute accumulator over a group of documents
    pub fn compute(&self, documents: &[Value]) -> Result<Value> {
        match self {
            Accumulator::Sum { expr } => {
                let mut sum = 0.0;
                for doc in documents {
                    let val = expr.evaluate(doc)?;
                    if let Some(n) = val.as_f64() {
                        sum += n;
                    }
                }
                Ok(Value::Number(serde_json::Number::from_f64(sum).unwrap()))
            }
            Accumulator::Avg { expr } => {
                let mut sum = 0.0;
                let mut count = 0;
                for doc in documents {
                    let val = expr.evaluate(doc)?;
                    if let Some(n) = val.as_f64() {
                        sum += n;
                        count += 1;
                    }
                }
                let avg = if count > 0 { sum / count as f64 } else { 0.0 };
                Ok(Value::Number(serde_json::Number::from_f64(avg).unwrap()))
            }
            Accumulator::Min { expr } => {
                let mut min: Option<f64> = None;
                for doc in documents {
                    let val = expr.evaluate(doc)?;
                    if let Some(n) = val.as_f64() {
                        min = Some(min.map_or(n, |m| m.min(n)));
                    }
                }
                Ok(min.map(|m| Value::Number(serde_json::Number::from_f64(m).unwrap()))
                    .unwrap_or(Value::Null))
            }
            Accumulator::Max { expr } => {
                let mut max: Option<f64> = None;
                for doc in documents {
                    let val = expr.evaluate(doc)?;
                    if let Some(n) = val.as_f64() {
                        max = Some(max.map_or(n, |m| m.max(n)));
                    }
                }
                Ok(max.map(|m| Value::Number(serde_json::Number::from_f64(m).unwrap()))
                    .unwrap_or(Value::Null))
            }
            Accumulator::Count => {
                Ok(Value::Number(documents.len().into()))
            }
            Accumulator::First { expr } => {
                if let Some(first) = documents.first() {
                    expr.evaluate(first)
                } else {
                    Ok(Value::Null)
                }
            }
            Accumulator::Last { expr } => {
                if let Some(last) = documents.last() {
                    expr.evaluate(last)
                } else {
                    Ok(Value::Null)
                }
            }
            Accumulator::Push { expr } => {
                let mut arr = Vec::new();
                for doc in documents {
                    arr.push(expr.evaluate(doc)?);
                }
                Ok(Value::Array(arr))
            }
            Accumulator::AddToSet { expr } => {
                let mut set = std::collections::HashSet::new();
                let mut arr = Vec::new();
                for doc in documents {
                    let val = expr.evaluate(doc)?;
                    let val_str = serde_json::to_string(&val).unwrap();
                    if set.insert(val_str) {
                        arr.push(val);
                    }
                }
                Ok(Value::Array(arr))
            }
        }
    }
}

/// Lookup specification for $lookup stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupSpec {
    /// Foreign collection name
    pub from: String,
    /// Local field
    pub local_field: String,
    /// Foreign field
    pub foreign_field: String,
    /// Output array field name
    pub as_field: String,
}

/// Helper function to get field value from JSON
fn get_field_value(doc: &Value, field: &str) -> Value {
    if let Value::Object(obj) = doc {
        obj.get(field).cloned().unwrap_or(Value::Null)
    } else {
        Value::Null
    }
}

/// Helper function to compare JSON values
fn compare_json_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Number(n1), Value::Number(n2)) => {
            let f1 = n1.as_f64().unwrap_or(0.0);
            let f2 = n2.as_f64().unwrap_or(0.0);
            f1.partial_cmp(&f2).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Value::String(s1), Value::String(s2)) => s1.cmp(s2),
        (Value::Bool(b1), Value::Bool(b2)) => b1.cmp(b2),
        (Value::Null, Value::Null) => std::cmp::Ordering::Equal,
        (Value::Null, _) => std::cmp::Ordering::Less,
        (_, Value::Null) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    }
}

/// Pipeline builder for fluent construction
pub struct PipelineBuilder {
    pipeline: Pipeline,
}

impl PipelineBuilder {
    /// Create a new pipeline builder
    pub fn new() -> Self {
        Self {
            pipeline: Pipeline::new(),
        }
    }

    /// Add a $match stage
    pub fn match_stage(mut self, query: Value) -> Self {
        self.pipeline.stages.push(PipelineStage::Match { query });
        self
    }

    /// Add a $project stage
    pub fn project(mut self, projection: Value) -> Self {
        self.pipeline.stages.push(PipelineStage::Project { projection });
        self
    }

    /// Add a $sort stage
    pub fn sort(mut self, sort_spec: BTreeMap<String, i32>) -> Self {
        self.pipeline.stages.push(PipelineStage::Sort { sort_spec });
        self
    }

    /// Add a $limit stage
    pub fn limit(mut self, count: usize) -> Self {
        self.pipeline.stages.push(PipelineStage::Limit { count });
        self
    }

    /// Add a $skip stage
    pub fn skip(mut self, count: usize) -> Self {
        self.pipeline.stages.push(PipelineStage::Skip { count });
        self
    }

    /// Add an $unwind stage
    pub fn unwind(mut self, path: String) -> Self {
        self.pipeline.stages.push(PipelineStage::Unwind {
            path,
            preserve_null_and_empty: false,
        });
        self
    }

    /// Build the pipeline
    pub fn build(self) -> Pipeline {
        self.pipeline
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_match_stage() {
        let stage = PipelineStage::Match {
            query: json!({"age": {"$gt": 25}}),
        };

        let docs = vec![
            json!({"name": "Alice", "age": 30}),
            json!({"name": "Bob", "age": 20}),
        ];

        let results = stage.execute(docs).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_project_stage() {
        let stage = PipelineStage::Project {
            projection: json!({"name": true, "age": true}),
        };

        let docs = vec![
            json!({"name": "Alice", "age": 30, "email": "alice@example.com"}),
        ];

        let results = stage.execute(docs).unwrap();
        assert!(results[0].get("name").is_some());
        assert!(results[0].get("age").is_some());
        assert!(results[0].get("email").is_none());
    }

    #[test]
    fn test_sort_stage() {
        let mut sort_spec = BTreeMap::new();
        sort_spec.insert("age".to_string(), 1);

        let stage = PipelineStage::Sort { sort_spec };

        let docs = vec![
            json!({"name": "Alice", "age": 30}),
            json!({"name": "Bob", "age": 20}),
            json!({"name": "Charlie", "age": 25}),
        ];

        let results = stage.execute(docs).unwrap();
        assert_eq!(results[0]["age"], 20);
        assert_eq!(results[2]["age"], 30);
    }

    #[test]
    fn test_limit_and_skip() {
        let limit_stage = PipelineStage::Limit { count: 2 };
        let skip_stage = PipelineStage::Skip { count: 1 };

        let docs = vec![
            json!({"id": 1}),
            json!({"id": 2}),
            json!({"id": 3}),
            json!({"id": 4}),
        ];

        let after_skip = skip_stage.execute(docs).unwrap();
        assert_eq!(after_skip.len(), 3);

        let after_limit = limit_stage.execute(after_skip).unwrap();
        assert_eq!(after_limit.len(), 2);
    }

    #[test]
    fn test_unwind_stage() {
        let stage = PipelineStage::Unwind {
            path: "$tags".to_string(),
            preserve_null_and_empty: false,
        };

        let docs = vec![
            json!({"name": "Alice", "tags": ["rust", "database"]}),
        ];

        let results = stage.execute(docs).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["tags"], "rust");
        assert_eq!(results[1]["tags"], "database");
    }
}
