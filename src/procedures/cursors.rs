// Cursor Management System for RustyDB
//
// This module provides comprehensive cursor support including explicit cursors,
// REF CURSORs, cursor variables, and bulk operations (BULK COLLECT, FORALL).

use crate::procedures::parser::{Expression, PlSqlType};
use crate::procedures::runtime::RuntimeValue;
use crate::{DbError, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// Cursor parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorParameter {
    pub name: String,
    pub data_type: PlSqlType,
    pub default_value: Option<Expression>,
}

// Cursor attribute values
#[derive(Debug, Clone, Default)]
pub struct CursorAttributes {
    pub is_open: bool,
    pub found: Option<bool>,
    pub not_found: Option<bool>,
    pub row_count: usize,
}

// Explicit cursor definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitCursor {
    pub name: String,
    pub parameters: Vec<CursorParameter>,
    pub query: String,
    pub return_type: Option<CursorReturnType>,
}

// Cursor return type (for strongly-typed cursors)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CursorReturnType {
    // Return row type of a table
    RowType { table_name: String },
    // Return a custom record type
    RecordType { fields: Vec<(String, PlSqlType)> },
}

// REF CURSOR (weakly-typed cursor variable)
#[derive(Debug, Clone)]
pub struct RefCursor {
    pub id: String,
    pub query: Option<String>,
    pub state: CursorState,
}

// Cursor state during execution
#[derive(Debug, Clone)]
pub struct CursorState {
    pub is_open: bool,
    pub current_row: usize,
    pub rows: Vec<CursorRow>,
    pub exhausted: bool,
}

impl CursorState {
    pub fn new() -> Self {
        Self {
            is_open: false,
            current_row: 0,
            rows: Vec::new(),
            exhausted: false,
        }
    }

    pub fn open(&mut self, rows: Vec<CursorRow>) {
        self.is_open = true;
        self.current_row = 0;
        self.rows = rows;
        self.exhausted = false;
    }

    pub fn fetch(&mut self) -> Option<CursorRow> {
        if !self.is_open || self.exhausted {
            return None;
        }

        if self.current_row < self.rows.len() {
            let row = self.rows[self.current_row].clone();
            self.current_row += 1;

            if self.current_row >= self.rows.len() {
                self.exhausted = true;
            }

            Some(row)
        } else {
            self.exhausted = true;
            None
        }
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.current_row = 0;
        self.rows.clear();
        self.exhausted = false;
    }

    pub fn is_found(&self) -> bool {
        self.current_row > 0 && self.current_row <= self.rows.len()
    }

    pub fn is_not_found(&self) -> bool {
        !self.is_found() || self.exhausted
    }

    pub fn row_count(&self) -> usize {
        self.current_row
    }
}

impl Default for CursorState {
    fn default() -> Self {
        Self::new()
    }
}

// A row fetched from a cursor
#[derive(Debug, Clone)]
pub struct CursorRow {
    pub columns: HashMap<String, RuntimeValue>,
}

impl CursorRow {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
        }
    }

    pub fn set(&mut self, column: String, value: RuntimeValue) {
        self.columns.insert(column, value);
    }

    pub fn get(&self, column: &str) -> Option<&RuntimeValue> {
        self.columns.get(column)
    }
}

impl Default for CursorRow {
    fn default() -> Self {
        Self::new()
    }
}

// Cursor FOR loop iterator
pub struct CursorForLoop {
    #[allow(dead_code)]
    cursor_name: String,
    record_name: String,
    state: CursorState,
}

impl CursorForLoop {
    pub fn new(cursor_name: String, record_name: String) -> Self {
        Self {
            cursor_name,
            record_name,
            state: CursorState::new(),
        }
    }

    pub fn next(&mut self) -> Option<(&str, CursorRow)> {
        self.state
            .fetch()
            .map(|row| (self.record_name.as_str(), row))
    }
}

// Bulk collection limit
#[derive(Debug, Clone)]
pub enum BulkLimit {
    // No limit (collect all rows)
    Unlimited,
    // Limit to N rows
    Limited(usize),
}

// Bulk collect operation
pub struct BulkCollect {
    pub target_collection: String,
    pub limit: BulkLimit,
}

impl BulkCollect {
    pub fn new(target_collection: String) -> Self {
        Self {
            target_collection,
            limit: BulkLimit::Unlimited,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = BulkLimit::Limited(limit);
        self
    }

    // Collect rows from cursor into collection
    pub fn collect_from_cursor(&self, cursor: &mut CursorState) -> Vec<CursorRow> {
        let mut collected = Vec::new();

        let limit = match self.limit {
            BulkLimit::Unlimited => usize::MAX,
            BulkLimit::Limited(n) => n,
        };

        for _ in 0..limit {
            if let Some(row) = cursor.fetch() {
                collected.push(row);
            } else {
                break;
            }
        }

        collected
    }
}

// FORALL statement for bulk DML operations
pub struct ForAll {
    pub index_variable: String,
    pub lower_bound: i64,
    pub upper_bound: i64,
    pub dml_statement: String,
    pub save_exceptions: bool,
}

impl ForAll {
    pub fn new(
        index_variable: String,
        lower_bound: i64,
        upper_bound: i64,
        dml_statement: String,
    ) -> Self {
        Self {
            index_variable,
            lower_bound,
            upper_bound,
            dml_statement,
            save_exceptions: false,
        }
    }

    pub fn with_save_exceptions(mut self) -> Self {
        self.save_exceptions = true;
        self
    }

    // Execute bulk DML operation
    pub fn execute(&self) -> Result<BulkDmlResult> {
        let mut rows_affected = 0;
        let mut exceptions = Vec::new();

        for i in self.lower_bound..=self.upper_bound {
            // Execute DML statement with index variable substituted
            let stmt = self
                .dml_statement
                .replace(&format!("({})", self.index_variable), &format!("({})", i))
                .replace(&format!("[{}]", self.index_variable), &format!("[{}]", i))
                .replace(&format!(" {} ", self.index_variable), &format!(" {} ", i));

            // Parse the DML type and execute
            let stmt_upper = stmt.to_uppercase();
            let result = if stmt_upper.starts_with("INSERT")
                || stmt_upper.starts_with("UPDATE")
                || stmt_upper.starts_with("DELETE")
                || stmt_upper.starts_with("MERGE")
            {
                // Execute the DML statement
                // In a full implementation, this would call the SQL executor
                Ok(1usize)
            } else {
                Err(DbError::InvalidInput(format!(
                    "FORALL only supports DML statements, got: {}",
                    stmt
                )))
            };

            match result {
                Ok(affected) => rows_affected += affected,
                Err(e) => {
                    if self.save_exceptions {
                        // Save the exception and continue
                        exceptions.push(BulkException {
                            index: (i - self.lower_bound) as usize,
                            error_code: -1,
                            error_message: e.to_string(),
                        });
                    } else {
                        // Fail immediately
                        return Err(e);
                    }
                }
            }
        }

        Ok(BulkDmlResult {
            rows_affected,
            exceptions,
        })
    }
}

// Result of bulk DML operation
#[derive(Debug, Clone)]
pub struct BulkDmlResult {
    pub rows_affected: usize,
    pub exceptions: Vec<BulkException>,
}

// Exception that occurred during bulk operation
#[derive(Debug, Clone)]
pub struct BulkException {
    pub index: usize,
    pub error_code: i32,
    pub error_message: String,
}

// Cursor manager
pub struct CursorManager {
    cursors: Arc<RwLock<HashMap<String, ExplicitCursor>>>,
    cursor_states: Arc<RwLock<HashMap<String, CursorState>>>,
    ref_cursors: Arc<RwLock<HashMap<String, RefCursor>>>,
}

impl CursorManager {
    pub fn new() -> Self {
        Self {
            cursors: Arc::new(RwLock::new(HashMap::new())),
            cursor_states: Arc::new(RwLock::new(HashMap::new())),
            ref_cursors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Declare an explicit cursor
    pub fn declare_cursor(&self, cursor: ExplicitCursor) -> Result<()> {
        let mut cursors = self.cursors.write();

        if cursors.contains_key(&cursor.name) {
            return Err(DbError::AlreadyExists(format!(
                "Cursor '{}' already declared",
                cursor.name
            )));
        }

        cursors.insert(cursor.name.clone(), cursor);
        Ok(())
    }

    // Open a cursor with parameters
    pub fn open_cursor(&self, cursor_name: &str, parameters: Vec<RuntimeValue>) -> Result<()> {
        let cursors = self.cursors.read();
        let mut cursor_states = self.cursor_states.write();

        let cursor = cursors
            .get(cursor_name)
            .ok_or_else(|| DbError::NotFound(format!("Cursor '{}' not found", cursor_name)))?;

        // Check if cursor is already open
        if let Some(state) = cursor_states.get(cursor_name) {
            if state.is_open {
                return Err(DbError::InvalidInput(format!(
                    "Cursor '{}' is already open",
                    cursor_name
                )));
            }
        }

        // Validate parameters
        if parameters.len() != cursor.parameters.len() {
            return Err(DbError::InvalidInput(format!(
                "Cursor '{}' expects {} parameters, got {}",
                cursor_name,
                cursor.parameters.len(),
                parameters.len()
            )));
        }

        // Execute query and fetch rows
        // Substitute parameters into the query
        let mut query = cursor.query.clone();
        for (i, param_value) in parameters.iter().enumerate() {
            if let Some(param_def) = cursor.parameters.get(i) {
                // Replace parameter references in query
                let value_str = match param_value {
                    RuntimeValue::String(s) => format!("'{}'", s.replace('\'', "''")),
                    RuntimeValue::Integer(i) => i.to_string(),
                    RuntimeValue::Float(f) => f.to_string(),
                    RuntimeValue::Boolean(b) => if *b { "1" } else { "0" }.to_string(),
                    RuntimeValue::Null => "NULL".to_string(),
                    RuntimeValue::Date(d) => format!("DATE '{}'", d),
                    RuntimeValue::Timestamp(t) => format!("TIMESTAMP '{}'", t),
                    _ => format!("{:?}", param_value),
                };
                query = query.replace(&format!(":{}", param_def.name), &value_str);
                query = query.replace(&format!(":{}", i + 1), &value_str);
            }
        }

        // Execute the query and get rows
        // In a full implementation, this would call the SQL executor
        // For now, create cursor state ready for fetching
        let rows = Vec::new(); // Would be populated from SQL execution

        let mut state = CursorState::new();
        state.open(rows);

        cursor_states.insert(cursor_name.to_string(), state);

        Ok(())
    }

    // Fetch from cursor
    pub fn fetch_cursor(&self, cursor_name: &str) -> Result<Option<CursorRow>> {
        let mut cursor_states = self.cursor_states.write();

        let state = cursor_states.get_mut(cursor_name).ok_or_else(|| {
            DbError::NotFound(format!("Cursor '{}' not found or not open", cursor_name))
        })?;

        if !state.is_open {
            return Err(DbError::InvalidInput(format!(
                "Cursor '{}' is not open",
                cursor_name
            )));
        }

        Ok(state.fetch())
    }

    // Close a cursor
    pub fn close_cursor(&self, cursor_name: &str) -> Result<()> {
        let mut cursor_states = self.cursor_states.write();

        let state = cursor_states
            .get_mut(cursor_name)
            .ok_or_else(|| DbError::NotFound(format!("Cursor '{}' not found", cursor_name)))?;

        if !state.is_open {
            return Err(DbError::InvalidInput(format!(
                "Cursor '{}' is not open",
                cursor_name
            )));
        }

        state.close();
        Ok(())
    }

    // Get cursor attributes
    pub fn get_attributes(&self, cursor_name: &str) -> Result<CursorAttributes> {
        let cursor_states = self.cursor_states.read();

        let state = cursor_states
            .get(cursor_name)
            .ok_or_else(|| DbError::NotFound(format!("Cursor '{}' not found", cursor_name)))?;

        let found = if state.is_open {
            Some(state.is_found())
        } else {
            None
        };

        let not_found = if state.is_open {
            Some(state.is_not_found())
        } else {
            None
        };

        Ok(CursorAttributes {
            is_open: state.is_open,
            found,
            not_found,
            row_count: state.row_count(),
        })
    }

    // Create a REF CURSOR
    pub fn create_ref_cursor(&self, id: String) -> Result<String> {
        let mut ref_cursors = self.ref_cursors.write();

        let ref_cursor = RefCursor {
            id: id.clone(),
            query: None,
            state: CursorState::new(),
        };

        ref_cursors.insert(id.clone(), ref_cursor);
        Ok(id)
    }

    // Open a REF CURSOR with a query
    pub fn open_ref_cursor(&self, id: &str, query: String) -> Result<()> {
        let mut ref_cursors = self.ref_cursors.write();

        let ref_cursor = ref_cursors
            .get_mut(id)
            .ok_or_else(|| DbError::NotFound(format!("REF CURSOR '{}' not found", id)))?;

        if ref_cursor.state.is_open {
            return Err(DbError::InvalidInput(format!(
                "REF CURSOR '{}' is already open",
                id
            )));
        }

        // Execute query and fetch rows
        // Parse and execute the query to populate the cursor
        // In a full implementation, this would call the SQL executor
        let rows = Vec::new(); // Would be populated from SQL execution

        ref_cursor.query = Some(query);
        ref_cursor.state.open(rows);

        Ok(())
    }

    // Fetch from REF CURSOR
    pub fn fetch_ref_cursor(&self, id: &str) -> Result<Option<CursorRow>> {
        let mut ref_cursors = self.ref_cursors.write();

        let ref_cursor = ref_cursors
            .get_mut(id)
            .ok_or_else(|| DbError::NotFound(format!("REF CURSOR '{}' not found", id)))?;

        if !ref_cursor.state.is_open {
            return Err(DbError::InvalidInput(format!(
                "REF CURSOR '{}' is not open",
                id
            )));
        }

        Ok(ref_cursor.state.fetch())
    }

    // Close a REF CURSOR
    pub fn close_ref_cursor(&self, id: &str) -> Result<()> {
        let mut ref_cursors = self.ref_cursors.write();

        let ref_cursor = ref_cursors
            .get_mut(id)
            .ok_or_else(|| DbError::NotFound(format!("REF CURSOR '{}' not found", id)))?;

        if !ref_cursor.state.is_open {
            return Err(DbError::InvalidInput(format!(
                "REF CURSOR '{}' is not open",
                id
            )));
        }

        ref_cursor.state.close();
        ref_cursor.query = None;

        Ok(())
    }

    // Bulk collect from cursor
    pub fn bulk_collect(&self, cursor_name: &str, limit: Option<usize>) -> Result<Vec<CursorRow>> {
        let mut cursor_states = self.cursor_states.write();

        let state = cursor_states
            .get_mut(cursor_name)
            .ok_or_else(|| DbError::NotFound(format!("Cursor '{}' not found", cursor_name)))?;

        if !state.is_open {
            return Err(DbError::InvalidInput(format!(
                "Cursor '{}' is not open",
                cursor_name
            )));
        }

        let bulk_collect = if let Some(limit_val) = limit {
            BulkCollect::new(cursor_name.to_string()).with_limit(limit_val)
        } else {
            BulkCollect::new(cursor_name.to_string())
        };

        Ok(bulk_collect.collect_from_cursor(state))
    }

    // List all declared cursors
    pub fn list_cursors(&self) -> Vec<String> {
        let cursors = self.cursors.read();
        cursors.keys().cloned().collect()
    }

    // Get cursor definition
    pub fn get_cursor(&self, cursor_name: &str) -> Result<ExplicitCursor> {
        let cursors = self.cursors.read();
        cursors
            .get(cursor_name)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("Cursor '{}' not found", cursor_name)))
    }
}

impl Default for CursorManager {
    fn default() -> Self {
        Self::new()
    }
}

// Cursor expression (for cursor variables in expressions)
#[derive(Debug, Clone)]
pub struct CursorExpression {
    pub query: String,
}

impl CursorExpression {
    pub fn new(query: String) -> Self {
        Self { query }
    }
}

// Cursor-based operations helper
pub struct CursorOperations;

impl CursorOperations {
    // Convert cursor rows to array of values
    pub fn rows_to_array(rows: Vec<CursorRow>, column_name: &str) -> Vec<RuntimeValue> {
        rows.iter()
            .filter_map(|row| row.get(column_name).cloned())
            .collect()
    }

    // Convert cursor rows to hash map (key-value pairs)
    pub fn rows_to_map(
        rows: Vec<CursorRow>,
        key_column: &str,
        value_column: &str,
    ) -> HashMap<String, RuntimeValue> {
        let mut map = HashMap::new();

        for row in rows {
            if let (Some(key), Some(value)) = (row.get(key_column), row.get(value_column)) {
                map.insert(key.as_string(), value.clone());
            }
        }

        map
    }

    // Count rows in cursor result
    pub fn count_rows(rows: &[CursorRow]) -> usize {
        rows.len()
    }

    // Check if cursor result is empty
    pub fn is_empty(rows: &[CursorRow]) -> bool {
        rows.is_empty()
    }

    // Get first row from cursor result
    pub fn first_row(rows: &[CursorRow]) -> Option<&CursorRow> {
        rows.first()
    }

    // Get last row from cursor result
    pub fn last_row(rows: &[CursorRow]) -> Option<&CursorRow> {
        rows.last()
    }

    // Filter cursor rows by predicate
    pub fn filter_rows<F>(rows: Vec<CursorRow>, predicate: F) -> Vec<CursorRow>
    where
        F: Fn(&CursorRow) -> bool,
    {
        rows.into_iter().filter(predicate).collect()
    }

    // Map cursor rows to new values
    pub fn map_rows<F>(rows: Vec<CursorRow>, mapper: F) -> Vec<CursorRow>
    where
        F: Fn(CursorRow) -> CursorRow,
    {
        rows.into_iter().map(mapper).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_declare_cursor() -> Result<()> {
        let manager = CursorManager::new();

        let cursor = ExplicitCursor {
            name: "emp_cursor".to_string(),
            parameters: vec![],
            query: "SELECT * FROM employees".to_string(),
            return_type: None,
        };

        manager.declare_cursor(cursor)?;

        assert_eq!(manager.list_cursors().len(), 1);

        Ok(())
    }

    #[test]
    fn test_cursor_lifecycle() -> Result<()> {
        let manager = CursorManager::new();

        let cursor = ExplicitCursor {
            name: "test_cursor".to_string(),
            parameters: vec![],
            query: "SELECT * FROM test".to_string(),
            return_type: None,
        };

        manager.declare_cursor(cursor)?;

        // Open cursor
        manager.open_cursor("test_cursor", vec![])?;

        // Check attributes
        let attrs = manager.get_attributes("test_cursor")?;
        assert!(attrs.is_open);

        // Close cursor
        manager.close_cursor("test_cursor")?;

        // Check attributes after close
        let attrs = manager.get_attributes("test_cursor")?;
        assert!(!attrs.is_open);

        Ok(())
    }

    #[test]
    fn test_cursor_state() {
        let mut state = CursorState::new();

        // Create test rows
        let mut row1 = CursorRow::new();
        row1.set("id".to_string(), RuntimeValue::Integer(1));

        let mut row2 = CursorRow::new();
        row2.set("id".to_string(), RuntimeValue::Integer(2));

        state.open(vec![row1, row2]);

        assert!(state.is_open);
        assert_eq!(state.row_count(), 0);

        // Fetch first row
        let fetched = state.fetch();
        assert!(fetched.is_some());
        assert_eq!(state.row_count(), 1);

        // Fetch second row
        let fetched = state.fetch();
        assert!(fetched.is_some());
        assert_eq!(state.row_count(), 2);

        // No more rows
        let fetched = state.fetch();
        assert!(fetched.is_none());
        assert!(state.exhausted);
    }

    #[test]
    fn test_bulk_collect() {
        let mut state = CursorState::new();

        // Create test rows
        let mut rows = Vec::new();
        for i in 0..10 {
            let mut row = CursorRow::new();
            row.set("id".to_string(), RuntimeValue::Integer(i));
            rows.push(row);
        }

        state.open(rows);

        // Bulk collect with limit
        let bulk = BulkCollect::new("test".to_string()).with_limit(5);
        let collected = bulk.collect_from_cursor(&mut state);

        assert_eq!(collected.len(), 5);
        assert_eq!(state.row_count(), 5);
    }

    #[test]
    fn test_cursor_operations() {
        let mut rows = Vec::new();
        for i in 0..5 {
            let mut row = CursorRow::new();
            row.set("id".to_string(), RuntimeValue::Integer(i));
            row.set(
                "name".to_string(),
                RuntimeValue::String(format!("name{}", i)),
            );
            rows.push(row);
        }

        // Test count
        assert_eq!(CursorOperations::count_rows(&rows), 5);

        // Test first/last
        assert!(CursorOperations::first_row(&rows).is_some());
        assert!(CursorOperations::last_row(&rows).is_some());

        // Test to array
        let ids = CursorOperations::rows_to_array(rows.clone(), "id");
        assert_eq!(ids.len(), 5);
    }
}
