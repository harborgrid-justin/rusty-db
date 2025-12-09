// # JSONPath Engine
//
// Full JSONPath implementation for querying JSON documents with support for
// recursive descent, filter expressions, and array slicing.

use serde_json::Value;
use std::collections::VecDeque;
use crate::error::Result;

/// JSONPath expression
#[derive(Debug, Clone, PartialEq)]
pub enum JsonPath {
    /// Root element ($)
    Root,
    /// Current element (@)
    Current,
    /// Child element by name
    Child(String),
    /// All children (*)
    Wildcard,
    /// Recursive descent (..)
    RecursiveDescent(Box<JsonPath>),
    /// Array index [n]
    Index(i64),
    /// Array slice [start:end:step]
    Slice { start: Option<i64>, end: Option<i64>, step: Option<i64> },
    /// Filter expression [?(...)]
    Filter(Box<FilterExpression>),
    /// Union of multiple paths [path1, path2, ...]
    Union(Vec<JsonPath>),
    /// Sequence of path segments
    Sequence(Vec<JsonPath>),
}

/// Filter expression for conditional selection
#[derive(Debug, Clone, PartialEq)]
pub enum FilterExpression {
    /// Comparison: path op value
    Comparison {
        left: Box<FilterExpression>,
        op: ComparisonOp,
        right: Box<FilterExpression>,
    },
    /// Logical AND
    And(Box<FilterExpression>, Box<FilterExpression>),
    /// Logical OR
    Or(Box<FilterExpression>, Box<FilterExpression>),
    /// Logical NOT
    Not(Box<FilterExpression>),
    /// Path expression
    Path(JsonPath),
    /// Literal value
    Literal(Value),
    /// Exists check
    Exists(JsonPath),
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    RegexMatch,
}

/// JSONPath parser
pub struct JsonPathParser {
    input: String,
    position: usize,
}

impl JsonPathParser {
    /// Create a new parser
    pub fn new(input: String) -> Self {
        Self { input, position: 0 }
    }

    /// Parse a JSONPath expression
    pub fn parse(&mut self) -> Result<JsonPath> {
        self.skip_whitespace();

        if self.current_char() == Some('$') {
            self.position += 1;
            self.parse_path_segments()
        } else {
            Err(crate::error::DbError::InvalidInput(
                "JSONPath must start with $".to_string()
            ))
        }
    }

    fn parse_path_segments(&mut self) -> Result<JsonPath> {
        let mut segments = vec![JsonPath::Root];

        while self.position < self.input.len() {
            self.skip_whitespace();

            match self.current_char() {
                Some('.') => {
                    self.position += 1;
                    if self.current_char() == Some('.') {
                        // Recursive descent
                        self.position += 1;
                        let path = self.parse_single_segment()?;
                        segments.push(JsonPath::RecursiveDescent(Box::new(path)));
                    } else {
                        segments.push(self.parse_single_segment()?;
                    }
                }
                Some('[') => {
                    segments.push(self.parse_bracket_segment()?;
                }
                _ => break,
            }
        }

        if segments.len() == 1 {
            Ok(segments.into_iter().next().unwrap())
        } else {
            Ok(JsonPath::Sequence(segments))
        }
    }

    fn parse_single_segment(&mut self) -> Result<JsonPath> {
        self.skip_whitespace();

        if self.current_char() == Some('*') {
            self.position += 1;
            Ok(JsonPath::Wildcard)
        } else {
            let name = self.parse_identifier()?;
            Ok(JsonPath::Child(name))
        }
    }

    fn parse_bracket_segment(&mut self) -> Result<JsonPath> {
        self.expect_char('[')?;
        self.skip_whitespace();

        let segment = if self.current_char() == Some('?') {
            // Filter expression
            self.position += 1;
            self.expect_char('(')?;
            let filter = self.parse_filter_expression()?;
            self.expect_char(')')?;
            JsonPath::Filter(Box::new(filter))
        } else if self.current_char() == Some('*') {
            // Wildcard
            self.position += 1;
            JsonPath::Wildcard
        } else if self.peek_slice() {
            // Array slice
            self.parse_slice()?
        } else {
            // Index or union
            self.parse_index_or_union()?
        };

        self.skip_whitespace();
        self.expect_char(']')?;

        Ok(segment)
    }

    fn parse_index_or_union(&mut self) -> Result<JsonPath> {
        let mut indices = Vec::new();

        loop {
            self.skip_whitespace();
            let index = self.parse_number()?;
            indices.push(JsonPath::Index(index));

            self.skip_whitespace();
            if self.current_char() == Some(',') {
                self.position += 1;
            } else {
                break;
            }
        }

        if indices.len() == 1 {
            Ok(indices.into_iter().next().unwrap())
        } else {
            Ok(JsonPath::Union(indices))
        }
    }

    fn parse_slice(&mut self) -> Result<JsonPath> {
        let start = if self.current_char() == Some(':') {
            None
        } else {
            Some(self.parse_number()?)
        };

        self.expect_char(':')?;

        let end = if self.current_char() == Some(':') || self.current_char() == Some(']') {
            None
        } else {
            Some(self.parse_number()?)
        };

        let step = if self.current_char() == Some(':') {
            self.position += 1;
            if self.current_char() == Some(']') {
                None
            } else {
                Some(self.parse_number()?)
            }
        } else {
            None
        };

        Ok(JsonPath::Slice { start, end, step })
    }

    fn parse_filter_expression(&mut self) -> Result<FilterExpression> {
        self.parse_or_expression()
    }

    fn parse_or_expression(&mut self) -> Result<FilterExpression> {
        let mut left = self.parse_and_expression()?;

        while self.peek_keyword("||") {
            self.position += 2;
            let right = self.parse_and_expression()?;
            left = FilterExpression::Or(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_and_expression(&mut self) -> Result<FilterExpression> {
        let mut left = self.parse_not_expression()?;

        while self.peek_keyword("&&") {
            self.position += 2;
            let right = self.parse_not_expression()?;
            left = FilterExpression::And(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_not_expression(&mut self) -> Result<FilterExpression> {
        self.skip_whitespace();

        if self.current_char() == Some('!') {
            self.position += 1;
            let expr = self.parse_comparison_expression()?;
            Ok(FilterExpression::Not(Box::new(expr)))
        } else {
            self.parse_comparison_expression()
        }
    }

    fn parse_comparison_expression(&mut self) -> Result<FilterExpression> {
        self.skip_whitespace();

        let left = self.parse_primary_expression()?;
        self.skip_whitespace();

        let op = if self.peek_keyword("==") {
            self.position += 2;
            Some(ComparisonOp::Equal)
        } else if self.peek_keyword("!=") {
            self.position += 2;
            Some(ComparisonOp::NotEqual)
        } else if self.peek_keyword("<=") {
            self.position += 2;
            Some(ComparisonOp::LessThanOrEqual)
        } else if self.peek_keyword(">=") {
            self.position += 2;
            Some(ComparisonOp::GreaterThanOrEqual)
        } else if self.current_char() == Some('<') {
            self.position += 1;
            Some(ComparisonOp::LessThan)
        } else if self.current_char() == Some('>') {
            self.position += 1;
            Some(ComparisonOp::GreaterThan)
        } else if self.peek_keyword("=~") {
            self.position += 2;
            Some(ComparisonOp::RegexMatch)
        } else {
            None
        };

        if let Some(op) = op {
            self.skip_whitespace();
            let right = self.parse_primary_expression()?;
            Ok(FilterExpression::Comparison {
                left: Box::new(left),
                op,
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    fn parse_primary_expression(&mut self) -> Result<FilterExpression> {
        self.skip_whitespace();

        match self.current_char() {
            Some('@') => {
                self.position += 1;
                let path = self.parse_path_segments()?;
                Ok(FilterExpression::Path(path))
            }
            Some('"') | Some('\'') => {
                let s = self.parse_string()?;
                Ok(FilterExpression::Literal(Value::String(s)))
            }
            Some(c) if c.is_ascii_digit() || c == '-' => {
                let n = self.parse_number()?;
                Ok(FilterExpression::Literal(Value::Number(
                    serde_json::Number::from(n)
                )))
            }
            Some('t') if self.peek_keyword("true") => {
                self.position += 4;
                Ok(FilterExpression::Literal(Value::Bool(true)))
            }
            Some('f') if self.peek_keyword("false") => {
                self.position += 5;
                Ok(FilterExpression::Literal(Value::Bool(false)))
            }
            Some('n') if self.peek_keyword("null") => {
                self.position += 4;
                Ok(FilterExpression::Literal(Value::Null))
            }
            Some('(') => {
                self.position += 1;
                let expr = self.parse_filter_expression()?;
                self.skip_whitespace();
                self.expect_char(')')?;
                Ok(expr)
            }
                format!("Unexpected character in filter expression at position {}", self.position)
            ))
        }
    }

    fn parse_identifier(&mut self) -> Result<String> {
        let start = self.position));
        while let Some(c) = self.current_char() {
            if c.is_alphanumeric() || c == '_' {
                self.position += 1;
            } else {
                break;
            }
        }

        if start == self.position {
                "Expected identifier".to_string()
            ))
        } else {
            Ok(self.input[start..self.position].to_string())
        }
    }

    fn parse_number(&mut self) -> Result<i64> {
        let start = self.position;
        if self.current_char() == Some('-') {
            self.position += 1;
        }

        while let Some(c) = self.current_char() {
            if c.is_ascii_digit() {
                self.position += 1;
            } else {
                break;
            }
        }

        self.input[start..self.position]
            .parse()
                "Invalid number".to_string()
            ))
    }

    fn parse_string(&mut self) -> Result<String> {
        let quote = self.current_char().unwrap();
        self.position += 1;

        let start = self.position;
        while let Some(c) = self.current_char() {
            if c == quote {
                let s = self.input[start..self.position].to_string();
                self.position += 1;
                return Ok(s);
            }
            self.position += 1;
        }

            "Unterminated string".to_string()
        ))
    }

    fn current_char(&self) -> Option<char> {
        self.input.chars().nth(self.position)
    }

    fn expect_char(&mut self, expected: char) -> Result<()> {
        if self.current_char() == Some(expected) {
            self.position += 1;
            Ok(())
        } else {
                format!("Expected '{}', got {:?}", expected, self.current_char())
            ))
        }
    }

    fn peek_keyword(&self, keyword: &str) -> bool {
        self.input[self.position..].starts_with(keyword)
    }

    fn peek_slice(&self) -> bool {
        let rest = &self.input[self.position..]));
        rest.contains(':') && !rest.starts_with(':')
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current_char() {
            if c.is_whitespace() {
                self.position += 1;
            } else {
                break;
            }
        }
    }
}

/// JSONPath evaluator
pub struct JsonPathEvaluator;

impl JsonPathEvaluator {
    /// Evaluate a JSONPath expression against a JSON value
    pub fn evaluate(path: &JsonPath, value: &Value) -> Result<Vec<Value>> {
        let mut results = Vec::new();
        Self::evaluate_path(path, value, &mut results)?;
        Ok(results)
    }

    fn evaluate_path(path: &JsonPath, value: &Value, results: &mut Vec<Value>) -> Result<()> {
        match path {
            JsonPath::Root => {
                results.push(value.clone());
            }
            JsonPath::Current => {
                results.push(value.clone());
            }
            JsonPath::Child(name) => {
                if let Some(obj) = value.as_object() {
                    if let Some(child) = obj.get(name) {
                        results.push(child.clone());
                    }
                }
            }
            JsonPath::Wildcard => {
                match value {
                    Value::Object(obj) => {
                        for v in obj.values() {
                            results.push(v.clone());
                        }
                    }
                    Value::Array(arr) => {
                        for v in arr {
                            results.push(v.clone());
                        }
                    }
                    _ => {}
                }
            }
            JsonPath::RecursiveDescent(inner_path) => {
                Self::recursive_descent(inner_path, value, results)?;
            }
            JsonPath::Index(index) => {
                if let Some(arr) = value.as_array() {
                    let idx = Self::normalize_index(*index, arr.len());
                    if let Some(elem) = arr.get(idx) {
                        results.push(elem.clone());
                    }
                }
            }
            JsonPath::Slice { start, end, step } => {
                if let Some(arr) = value.as_array() {
                    let slice_results = Self::slice_array(arr, *start, *end, *step);
                    results.extend(slice_results);
                }
            }
            JsonPath::Filter(filter) => {
                Self::evaluate_filter(filter, value, results)?;
            }
            JsonPath::Union(paths) => {
                for p in paths {
                    Self::evaluate_path(p, value, results)?;
                }
            }
            JsonPath::Sequence(segments) => {
                let mut current_values = vec![value.clone()];

                for segment in segments {
                    let mut next_values = Vec::new();
                    for current in &current_values {
                        Self::evaluate_path(segment, current, &mut next_values)?;
                    }
                    current_values = next_values;
                }

                results.extend(current_values);
            }
        }

        Ok(())
    }

    fn recursive_descent(path: &JsonPath, value: &Value, results: &mut Vec<Value>) -> Result<()> {
        let mut queue = VecDeque::new();
        queue.push_back(value.clone());

        while let Some(current) = queue.pop_front() {
            // Try to match the path at this level
            let mut matches = Vec::new();
            Self::evaluate_path(path, &current, &mut matches)?;
            results.extend(matches);

            // Add children to queue
            match &current {
                Value::Object(obj) => {
                    for v in obj.values() {
                        queue.push_back(v.clone());
                    }
                }
                Value::Array(arr) => {
                    for v in arr {
                        queue.push_back(v.clone());
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn evaluate_filter(
        filter: &FilterExpression,
        value: &Value,
        results: &mut Vec<Value>,
    ) -> Result<()> {
        match value {
            Value::Array(arr) => {
                for item in arr {
                    if Self::test_filter(filter, item)? {
                        results.push(item.clone());
                    }
                }
            }
            _ => {
                if Self::test_filter(filter, value)? {
                    results.push(value.clone());
                }
            }
        }

        Ok(())
    }

    fn test_filter(filter: &FilterExpression, value: &Value) -> Result<bool> {
        match filter {
            FilterExpression::Comparison { left, op, right } => {
                let left_val = Self::evaluate_filter_value(left, value)?;
                let right_val = Self::evaluate_filter_value(right, value)?;
                Ok(Self::compare_values(&left_val, *op, &right_val))
            }
            FilterExpression::And(left, right) => {
                Ok(Self::test_filter(left, value)? && Self::test_filter(right, value)?)
            }
            FilterExpression::Or(left, right) => {
                Ok(Self::test_filter(left, value)? || Self::test_filter(right, value)?)
            }
            FilterExpression::Not(expr) => {
                Ok(!Self::test_filter(expr, value)?)
            }
            FilterExpression::Path(path) => {
                let results = Self::evaluate(path, value)?;
                Ok(!results.is_empty())
            }
            FilterExpression::Literal(v) => {
                Ok(v.as_bool().unwrap_or(true))
            }
            FilterExpression::Exists(path) => {
                let results = Self::evaluate(path, value)?;
                Ok(!results.is_empty())
            }
        }
    }

    fn evaluate_filter_value(expr: &FilterExpression, context: &Value) -> Result<Value> {
        match expr {
            FilterExpression::Path(path) => {
                let results = Self::evaluate(path, context)?;
                Ok(results.first().cloned().unwrap_or(Value::Null))
            }
            FilterExpression::Literal(v) => Ok(v.clone()),
            _ => Ok(Value::Null),
        }
    }

    fn compare_values(left: &Value, op: ComparisonOp, right: &Value) -> bool {
        match op {
            ComparisonOp::Equal => left == right,
            ComparisonOp::NotEqual => left != right,
            ComparisonOp::LessThan => {
                if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                    l < r
                } else if let (Some(l), Some(r)) = (left.as_str(), right.as_str()) {
                    l < r
                } else {
                    false
                }
            }
            ComparisonOp::LessThanOrEqual => {
                if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                    l <= r
                } else if let (Some(l), Some(r)) = (left.as_str(), right.as_str()) {
                    l <= r
                } else {
                    false
                }
            }
            ComparisonOp::GreaterThan => {
                if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                    l > r
                } else if let (Some(l), Some(r)) = (left.as_str(), right.as_str()) {
                    l > r
                } else {
                    false
                }
            }
            ComparisonOp::GreaterThanOrEqual => {
                if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                    l >= r
                } else if let (Some(l), Some(r)) = (left.as_str(), right.as_str()) {
                    l >= r
                } else {
                    false
                }
            }
            ComparisonOp::RegexMatch => {
                if let (Some(text), Some(pattern)) = (left.as_str(), right.as_str()) {
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(text))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
        }
    }

    fn normalize_index(index: i64, len: usize) -> usize {
        if index < 0 {
            (len as i64 + index).max(0) as usize
        } else {
            index as usize
        }
    }

    fn slice_array(
        arr: &[Value],
        start: Option<i64>,
        end: Option<i64>,
        step: Option<i64>,
    ) -> Vec<Value> {
        let len = arr.len() as i64;
        let step = step.unwrap_or(1);

        if step == 0 {
            return Vec::new();
        }

        let (start, end) = if step > 0 {
            let start = start.unwrap_or(0);
            let end = end.unwrap_or(len);
            (Self::normalize_index(start, arr.len()) as i64, end.min(len))
        } else {
            let start = start.unwrap_or(len - 1);
            let end = end.unwrap_or(-len - 1);
            (start.min(len - 1), end.max(-1))
        };

        let mut results = Vec::new();
        let mut i = start;

        if step > 0 {
            while i < end {
                if i >= 0 && (i as usize) < arr.len() {
                    results.push(arr[i as usize].clone());
                }
                i += step;
            }
        } else {
            while i > end {
                if i >= 0 && (i as usize) < arr.len() {
                    results.push(arr[i as usize].clone());
                }
                i += step;
            }
        }

        results
    }
}

/// High-level JSONPath query API
pub fn query(jsonpath: &str, value: &Value) -> Result<Vec<Value>> {
    let mut parser = JsonPathParser::new(json_path.to_string());
    let path = parser.parse()?;
    JsonPathEvaluator::evaluate(&path, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_basic_child_access() {
        let data = json!({
            "store": {
                "book": [
                    {"title": "Book 1", "price": 10},
                    {"title": "Book 2", "price": 20}
                ]
            }
        });

        let results = query("$.store.book", &data).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_array_index() {
        let data = json!({
            "items": [1, 2, 3, 4, 5]
        });

        let results = query("$.items[0]", &data).unwrap();
        assert_eq!(results, vec![json!(1)]);

        let results = query("$.items[-1]", &data).unwrap();
        assert_eq!(results, vec![json!(5)]);
    }

    #[test]
    fn test_array_slice() {
        let data = json!({
            "items": [1, 2, 3, 4, 5]
        });

        let results = query("$.items[1:3]", &data).unwrap();
        assert_eq!(results, vec![json!(2), json!(3)]);
    }

    #[test]
    fn test_wildcard() {
        let data = json!({
            "store": {
                "book": {"title": "Book"},
                "music": {"title": "Album"}
            }
        });

        let results = query("$.store.*", &data).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_recursive_descent() {
        let data = json!({
            "store": {
                "book": [
                    {"title": "Book 1"},
                    {"title": "Book 2"}
                ]
            }
        });

        let results = query("$..title", &data).unwrap();
        assert_eq!(results.len(), 2);
    }
}
