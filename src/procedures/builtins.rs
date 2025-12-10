// Built-in Packages for RustyDB
//
// This module provides Oracle-compatible built-in packages like DBMS_OUTPUT,
// DBMS_SQL, UTL_FILE, and DBMS_SCHEDULER for enterprise database operations.

use crate::procedures::runtime::RuntimeValue;
use crate::{DbError, Result};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// DBMS_OUTPUT - Text output buffering
// ============================================================================

// DBMS_OUTPUT package for managing text output from procedures
pub struct DbmsOutput {
    buffer: Arc<RwLock<VecDeque<String>>>,
    enabled: Arc<RwLock<bool>>,
    buffer_size: Arc<RwLock<usize>>,
}

impl DbmsOutput {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(RwLock::new(VecDeque::new())),
            enabled: Arc::new(RwLock::new(false)),
            buffer_size: Arc::new(RwLock::new(20000)),
        }
    }

    // Enable output buffering
    pub fn enable(&self, buffersize: Option<usize>) {
        let mut enabled = self.enabled.write();
        let mut size = self.buffer_size.write();

        *enabled = true;
        if let Some(new_size) = buffersize {
            *size = new_size.min(1_000_000); // Max 1MB
        }
    }

    // Disable output buffering
    pub fn disable(&self) {
        let mut enabled = self.enabled.write();
        let mut buffer = self.buffer.write();

        *enabled = false;
        buffer.clear();
    }

    // Put a line into the buffer
    pub fn put_line(&self, line: String) -> Result<()> {
        let enabled = self.enabled.read();
        if !*enabled {
            return Ok(());
        }
        drop(enabled);

        let mut buffer = self.buffer.write();
        let buffer_size = self.buffer_size.read();

        // Check buffer size
        let current_size: usize = buffer.iter().map(|s| s.len()).sum();
        if current_size + line.len() > *buffer_size {
            return Err(DbError::Runtime("Output buffer overflow".to_string()));
        }

        buffer.push_back(line);
        Ok(())
    }

    // Put text without newline
    pub fn put(&self, text: String) -> Result<()> {
        let enabled = self.enabled.read();
        if !*enabled {
            return Ok(());
        }
        drop(enabled);

        let mut buffer = self.buffer.write();

        if let Some(last) = buffer.back_mut() {
            last.push_str(&text);
        } else {
            buffer.push_back(text);
        }

        Ok(())
    }

    // Add a newline to the current line
    pub fn new_line(&self) -> Result<()> {
        let enabled = self.enabled.read();
        if !*enabled {
            return Ok(());
        }
        drop(enabled);

        let mut buffer = self.buffer.write();
        buffer.push_back(String::new());

        Ok(())
    }

    // Get a line from the buffer
    pub fn get_line(&self) -> Result<Option<String>> {
        let enabled = self.enabled.read();
        if !*enabled {
            return Err(DbError::Runtime("DBMS_OUTPUT is not enabled".to_string()));
        }
        drop(enabled);

        let mut buffer = self.buffer.write();
        Ok(buffer.pop_front())
    }

    // Get multiple lines from the buffer
    pub fn get_lines(&self, numlines: usize) -> Result<Vec<String>> {
        let enabled = self.enabled.read();
        if !*enabled {
            return Err(DbError::Runtime("DBMS_OUTPUT is not enabled".to_string()));
        }
        drop(enabled);

        let mut buffer = self.buffer.write();
        let mut lines = Vec::new();

        for _ in 0..numlines {
            if let Some(line) = buffer.pop_front() {
                lines.push(line);
            } else {
                break;
            }
        }

        Ok(lines)
    }
}

impl Default for DbmsOutput {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// DBMS_SQL - Dynamic SQL execution
// ============================================================================

// DBMS_SQL package for dynamic SQL operations
pub struct DbmsSql {
    cursors: Arc<RwLock<HashMap<i32, DynamicCursor>>>,
    next_cursor_id: Arc<RwLock<i32>>,
}

// Dynamic cursor for DBMS_SQL
#[derive(Debug, Clone)]
struct DynamicCursor {
    #[allow(dead_code)]
    id: i32,
    sql: Option<String>,
    parsed: bool,
    executed: bool,
    bind_variables: HashMap<String, RuntimeValue>,
    define_columns: HashMap<usize, ColumnDefinition>,
    /// Result set from query execution
    result_set: Option<ResultSet>,
    /// Current row position for fetching
    current_row: usize,
    /// Statement type (SELECT, INSERT, UPDATE, DELETE, etc.)
    statement_type: StatementType,
    /// Rows affected by DML operations
    rows_affected: usize,
}

/// Column definition for DEFINE_COLUMN
#[derive(Debug, Clone)]
struct ColumnDefinition {
    name: String,
    data_type: ColumnType,
    #[allow(dead_code)]
    max_length: Option<usize>,
}

/// Column type for defined columns
#[derive(Debug, Clone, PartialEq)]
enum ColumnType {
    Integer,
    Float,
    String,
    Date,
    Timestamp,
    Boolean,
    Blob,
    Clob,
}

/// Result set from query execution
#[derive(Debug, Clone)]
struct ResultSet {
    #[allow(dead_code)]
    columns: Vec<String>,
    rows: Vec<Vec<RuntimeValue>>,
}

impl ResultSet {
    fn new(columns: Vec<String>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
        }
    }

    /// Reserved for table functions


    #[allow(dead_code)]


    fn add_row(&mut self, row: Vec<RuntimeValue>) {
        self.rows.push(row);
    }

    fn row_count(&self) -> usize {
        self.rows.len()
    }
}

/// Statement type classification
#[derive(Debug, Clone, PartialEq, Default)]
enum StatementType {
    #[default]
    Unknown,
    Select,
    Insert,
    Update,
    Delete,
    CreateTable,
    DropTable,
    AlterTable,
    CreateIndex,
    PlSqlBlock,
}

impl DbmsSql {
    pub fn new() -> Self {
        Self {
            cursors: Arc::new(RwLock::new(HashMap::new())),
            next_cursor_id: Arc::new(RwLock::new(1)),
        }
    }

    // Open a new cursor
    pub fn open_cursor(&self) -> i32 {
        let mut next_id = self.next_cursor_id.write();
        let mut cursors = self.cursors.write();

        let cursor_id = *next_id;
        *next_id += 1;

        let cursor = DynamicCursor {
            id: cursor_id,
            sql: None,
            parsed: false,
            executed: false,
            bind_variables: HashMap::new(),
            define_columns: HashMap::new(),
            result_set: None,
            current_row: 0,
            statement_type: StatementType::Unknown,
            rows_affected: 0,
        };

        cursors.insert(cursor_id, cursor);
        cursor_id
    }

    // Parse SQL statement
    pub fn parse(&self, cursor_id: i32, sql: String) -> Result<()> {
        let mut cursors = self.cursors.write();

        let cursor = cursors
            .get_mut(&cursor_id)
            .ok_or_else(|| DbError::NotFound(format!("Cursor {} not found", cursor_id)))?;

        // Determine statement type from SQL
        let sql_upper = sql.trim().to_uppercase();
        cursor.statement_type = if sql_upper.starts_with("SELECT") {
            StatementType::Select
        } else if sql_upper.starts_with("INSERT") {
            StatementType::Insert
        } else if sql_upper.starts_with("UPDATE") {
            StatementType::Update
        } else if sql_upper.starts_with("DELETE") {
            StatementType::Delete
        } else if sql_upper.starts_with("CREATE TABLE") {
            StatementType::CreateTable
        } else if sql_upper.starts_with("DROP TABLE") {
            StatementType::DropTable
        } else if sql_upper.starts_with("ALTER TABLE") {
            StatementType::AlterTable
        } else if sql_upper.starts_with("CREATE INDEX") {
            StatementType::CreateIndex
        } else if sql_upper.starts_with("BEGIN") || sql_upper.starts_with("DECLARE") {
            StatementType::PlSqlBlock
        } else {
            StatementType::Unknown
        };

        // Validate SQL syntax by checking for common issues
        Self::validate_sql_syntax(&sql)?;

        cursor.sql = Some(sql);
        cursor.parsed = true;
        cursor.executed = false;
        cursor.result_set = None;
        cursor.current_row = 0;
        cursor.rows_affected = 0;

        Ok(())
    }

    // Validate SQL syntax
    fn validate_sql_syntax(sql: &str) -> Result<()> {
        // Check for balanced parentheses
        let mut paren_count = 0i32;
        for ch in sql.chars() {
            match ch {
                '(' => paren_count += 1,
                ')' => {
                    paren_count -= 1;
                    if paren_count < 0 {
                        return Err(DbError::InvalidInput(
                            "Unbalanced parentheses in SQL statement".to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }
        if paren_count != 0 {
            return Err(DbError::InvalidInput(
                "Unbalanced parentheses in SQL statement".to_string(),
            ));
        }

        // Check for unclosed quotes
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut prev_char = ' ';
        for ch in sql.chars() {
            if ch == '\'' && prev_char != '\\' && !in_double_quote {
                in_single_quote = !in_single_quote;
            } else if ch == '"' && prev_char != '\\' && !in_single_quote {
                in_double_quote = !in_double_quote;
            }
            prev_char = ch;
        }
        if in_single_quote || in_double_quote {
            return Err(DbError::InvalidInput(
                "Unclosed quote in SQL statement".to_string(),
            ));
        }

        Ok(())
    }

    // Bind a variable by name
    pub fn bind_variable(&self, cursor_id: i32, name: String, value: RuntimeValue) -> Result<()> {
        let mut cursors = self.cursors.write();

        let cursor = cursors
            .get_mut(&cursor_id)
            .ok_or_else(|| DbError::NotFound(format!("Cursor {} not found", cursor_id)))?;

        cursor.bind_variables.insert(name, value);

        Ok(())
    }

    // Execute the SQL statement
    pub fn execute(&self, cursor_id: i32) -> Result<usize> {
        let mut cursors = self.cursors.write();

        let cursor = cursors
            .get_mut(&cursor_id)
            .ok_or_else(|| DbError::NotFound(format!("Cursor {} not found", cursor_id)))?;

        if !cursor.parsed {
            return Err(DbError::InvalidInput(
                "Cursor has not been parsed".to_string(),
            ));
        }

        let sql = cursor
            .sql
            .as_ref()
            .ok_or_else(|| DbError::InvalidInput("No SQL statement to execute".to_string()))?
            .clone();

        // Substitute bind variables into the SQL statement
        let resolved_sql = Self::substitute_bind_variables(&sql, &cursor.bind_variables)?;

        // Execute based on statement type
        let rows_affected = match cursor.statement_type {
            StatementType::Select => {
                // For SELECT statements, we prepare a result set
                let result_set =
                    Self::execute_select_statement(&resolved_sql, &cursor.define_columns)?;
                let row_count = result_set.row_count();
                cursor.result_set = Some(result_set);
                cursor.current_row = 0;
                row_count
            }
            StatementType::Insert => {
                Self::execute_dml_statement(&resolved_sql, DmlOperation::Insert)?
            }
            StatementType::Update => {
                Self::execute_dml_statement(&resolved_sql, DmlOperation::Update)?
            }
            StatementType::Delete => {
                Self::execute_dml_statement(&resolved_sql, DmlOperation::Delete)?
            }
            StatementType::CreateTable
            | StatementType::DropTable
            | StatementType::AlterTable
            | StatementType::CreateIndex => {
                Self::execute_ddl_statement(&resolved_sql)?;
                0
            }
            StatementType::PlSqlBlock => {
                Self::execute_plsql_block(&resolved_sql)?;
                0
            }
            StatementType::Unknown => {
                // Try to execute as a generic statement
                Self::execute_generic_statement(&resolved_sql)?
            }
        };

        cursor.executed = true;
        cursor.rows_affected = rows_affected;

        Ok(rows_affected)
    }

    // Substitute bind variables in SQL
    fn substitute_bind_variables(
        sql: &str,
        bind_vars: &HashMap<String, RuntimeValue>,
    ) -> Result<String> {
        let mut result = sql.to_string();

        for (name, value) in bind_vars {
            // Support both :name and :name (colon-prefixed) bind variable syntax
            let placeholder = format!(":{}", name);
            let replacement = Self::runtime_value_to_sql(value)?;
            result = result.replace(&placeholder, &replacement);
        }

        // Check for any remaining unbound variables
        if result.contains(':') {
            // Find unbound variable names
            let unbound: Vec<String> = result
                .split_whitespace()
                .filter(|s| s.starts_with(':') && s.len() > 1)
                .map(|s| {
                    s.trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
                        .to_string()
                })
                .collect();

            if !unbound.is_empty() {
                // Filter out false positives (like timestamps with colons)
                let real_unbound: Vec<_> = unbound
                    .iter()
                    .filter(|s| !s.chars().all(|c| c.is_numeric()))
                    .filter(|s| !s.is_empty())
                    .collect();

                if !real_unbound.is_empty() {
                    return Err(DbError::InvalidInput(format!(
                        "Unbound variables in SQL: {:?}",
                        real_unbound
                    )));
                }
            }
        }

        Ok(result)
    }

    // Convert RuntimeValue to SQL literal
    fn runtime_value_to_sql(value: &RuntimeValue) -> Result<String> {
        Ok(match value {
            RuntimeValue::Null => "NULL".to_string(),
            RuntimeValue::Integer(i) => i.to_string(),
            RuntimeValue::Float(f) => {
                // Ensure proper decimal formatting
                if f.fract() == 0.0 {
                    format!("{:.1}", f)
                } else {
                    f.to_string()
                }
            }
            RuntimeValue::String(s) => {
                // Escape single quotes for SQL
                format!("'{}'", s.replace('\'', "''"))
            }
            RuntimeValue::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            RuntimeValue::Date(d) => format!("DATE '{}'", d),
            RuntimeValue::Timestamp(t) => format!("TIMESTAMP '{}'", t),
            RuntimeValue::Cursor(_) => {
                return Err(DbError::InvalidInput(
                    "Cannot use cursor as bind variable value".to_string(),
                ));
            }
            RuntimeValue::Record(r) => {
                return Err(DbError::InvalidInput(format!(
                    "Cannot use record as bind variable value: {:?}",
                    r
                )));
            }
            RuntimeValue::Array(a) => {
                // Convert array to SQL list
                let values: Result<Vec<String>> =
                    a.iter().map(|v| Self::runtime_value_to_sql(v)).collect();
                format!("({})", values?.join(", "))
            }
        })
    }

    // Execute SELECT statement and return result set
    fn execute_select_statement(
        _sql: &str,
        _columns: &HashMap<usize, ColumnDefinition>,
    ) -> Result<ResultSet> {
        // This would integrate with the main query executor
        // For now, return an empty result set that can be populated
        // by the actual execution engine integration

        // In production, this would:
        // 1. Parse the SQL using SqlParser
        // 2. Create an execution plan
        // 3. Execute the plan and collect results
        // 4. Map results to the defined columns

        Ok(ResultSet::new(Vec::new()))
    }

    // Execute DML statement (INSERT, UPDATE, DELETE)
    fn execute_dml_statement(_sql: &str, _operation: DmlOperation) -> Result<usize> {
        // This would integrate with the main executor
        // For now, return 0 rows affected

        // In production, this would:
        // 1. Parse the SQL using SqlParser
        // 2. Acquire appropriate locks via TransactionManager
        // 3. Execute the DML operation
        // 4. Return the number of rows affected

        Ok(0)
    }

    // Execute DDL statement (CREATE, DROP, ALTER)
    fn execute_ddl_statement(_sql: &str) -> Result<()> {
        // This would integrate with the catalog
        // For now, return success

        // In production, this would:
        // 1. Parse the SQL using SqlParser
        // 2. Acquire schema locks
        // 3. Execute the DDL operation
        // 4. Update the catalog

        Ok(())
    }

    // Execute PL/SQL block
    fn execute_plsql_block(_sql: &str) -> Result<()> {
        // This would integrate with the PL/SQL runtime executor
        // For now, return success

        // In production, this would:
        // 1. Parse the PL/SQL block using PlSqlParser
        // 2. Execute using RuntimeExecutor

        Ok(())
    }

    // Execute generic statement
    fn execute_generic_statement(_sql: &str) -> Result<usize> {
        // Fallback for unknown statement types
        Ok(0)
    }

    // Define a column for fetching
    pub fn define_column(
        &self,
        cursor_id: i32,
        position: usize,
        column_type: &str,
        max_length: Option<usize>,
    ) -> Result<()> {
        let mut cursors = self.cursors.write();

        let cursor = cursors
            .get_mut(&cursor_id)
            .ok_or_else(|| DbError::NotFound(format!("Cursor {} not found", cursor_id)))?;

        let col_type = match column_type.to_uppercase().as_str() {
            "INTEGER" | "INT" | "NUMBER" => ColumnType::Integer,
            "FLOAT" | "DOUBLE" | "REAL" => ColumnType::Float,
            "VARCHAR" | "VARCHAR2" | "CHAR" | "STRING" => ColumnType::String,
            "DATE" => ColumnType::Date,
            "TIMESTAMP" => ColumnType::Timestamp,
            "BOOLEAN" | "BOOL" => ColumnType::Boolean,
            "BLOB" => ColumnType::Blob,
            "CLOB" => ColumnType::Clob,
            _ => ColumnType::String,
        };

        let definition = ColumnDefinition {
            name: format!("COL{}", position),
            data_type: col_type,
            max_length,
        };

        cursor.define_columns.insert(position, definition);
        Ok(())
    }

    // Fetch a row from the cursor
    pub fn fetch_rows(&self, cursor_id: i32) -> Result<bool> {
        let mut cursors = self.cursors.write();

        let cursor = cursors
            .get_mut(&cursor_id)
            .ok_or_else(|| DbError::NotFound(format!("Cursor {} not found", cursor_id)))?;

        if !cursor.executed {
            return Err(DbError::InvalidInput(
                "Cursor has not been executed".to_string(),
            ));
        }

        if cursor.statement_type != StatementType::Select {
            return Err(DbError::InvalidInput(
                "FETCH_ROWS is only valid for SELECT statements".to_string(),
            ));
        }

        let result_set = cursor
            .result_set
            .as_ref()
            .ok_or_else(|| DbError::InvalidInput("No result set available".to_string()))?;

        if cursor.current_row < result_set.row_count() {
            cursor.current_row += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // Get column value from current row
    pub fn column_value(&self, cursor_id: i32, position: usize) -> Result<RuntimeValue> {
        let cursors = self.cursors.read();

        let cursor = cursors
            .get(&cursor_id)
            .ok_or_else(|| DbError::NotFound(format!("Cursor {} not found", cursor_id)))?;

        if !cursor.executed {
            return Err(DbError::InvalidInput(
                "Cursor has not been executed".to_string(),
            ));
        }

        let result_set = cursor
            .result_set
            .as_ref()
            .ok_or_else(|| DbError::InvalidInput("No result set available".to_string()))?;

        if cursor.current_row == 0 {
            return Err(DbError::InvalidInput(
                "Must call FETCH_ROWS before COLUMN_VALUE".to_string(),
            ));
        }

        let row_idx = cursor.current_row - 1;
        if row_idx >= result_set.row_count() {
            return Err(DbError::InvalidInput("No more rows to fetch".to_string()));
        }

        let row = &result_set.rows[row_idx];
        if position == 0 || position > row.len() {
            return Err(DbError::InvalidInput(format!(
                "Column position {} out of range (1..{})",
                position,
                row.len()
            )));
        }

        Ok(row[position - 1].clone())
    }

    // Get the number of rows affected by last execute
    pub fn rows_affected(&self, cursor_id: i32) -> Result<usize> {
        let cursors = self.cursors.read();

        let cursor = cursors
            .get(&cursor_id)
            .ok_or_else(|| DbError::NotFound(format!("Cursor {} not found", cursor_id)))?;

        Ok(cursor.rows_affected)
    }

    // Get last row count fetched
    pub fn last_row_count(&self, cursor_id: i32) -> Result<usize> {
        let cursors = self.cursors.read();

        let cursor = cursors
            .get(&cursor_id)
            .ok_or_else(|| DbError::NotFound(format!("Cursor {} not found", cursor_id)))?;

        Ok(cursor.current_row)
    }

    // Describe columns in the result set
    pub fn describe_columns(&self, cursor_id: i32) -> Result<Vec<(usize, String, String)>> {
        let cursors = self.cursors.read();

        let cursor = cursors
            .get(&cursor_id)
            .ok_or_else(|| DbError::NotFound(format!("Cursor {} not found", cursor_id)))?;

        if !cursor.parsed {
            return Err(DbError::InvalidInput(
                "Cursor has not been parsed".to_string(),
            ));
        }

        // Return defined columns with their positions and types
        let mut columns: Vec<_> = cursor
            .define_columns
            .iter()
            .map(|(pos, def)| {
                let type_name = match def.data_type {
                    ColumnType::Integer => "INTEGER",
                    ColumnType::Float => "FLOAT",
                    ColumnType::String => "VARCHAR",
                    ColumnType::Date => "DATE",
                    ColumnType::Timestamp => "TIMESTAMP",
                    ColumnType::Boolean => "BOOLEAN",
                    ColumnType::Blob => "BLOB",
                    ColumnType::Clob => "CLOB",
                };
                (*pos, def.name.clone(), type_name.to_string())
            })
            .collect();

        columns.sort_by_key(|(pos, _, _)| *pos);
        Ok(columns)
    }

    // Close a cursor
    pub fn close_cursor(&self, cursor_id: i32) -> Result<()> {
        let mut cursors = self.cursors.write();

        if cursors.remove(&cursor_id).is_none() {
            return Err(DbError::NotFound(format!("Cursor {} not found", cursor_id)));
        }

        Ok(())
    }

    // Check if cursor is open
    pub fn is_open(&self, cursor_id: i32) -> bool {
        let cursors = self.cursors.read();
        cursors.contains_key(&cursor_id)
    }
}

/// DML operation type for execute_dml_statement
#[derive(Debug, Clone, Copy)]
enum DmlOperation {
    Insert,
    Update,
    Delete,
}

impl Default for DbmsSql {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// UTL_FILE - File I/O operations
// ============================================================================

// UTL_FILE package for file operations
pub struct UtlFile {
    file_handles: Arc<RwLock<HashMap<i32, FileHandle>>>,
    next_handle_id: Arc<RwLock<i32>>,
    directories: Arc<RwLock<HashMap<String, PathBuf>>>,
}

// File handle
struct FileHandle {
    #[allow(dead_code)]
    id: i32,
    #[allow(dead_code)]
    directory: String,
    #[allow(dead_code)]
    filename: String,
    mode: FileMode,
    file: Option<File>,
}

// File mode
#[derive(Debug, Clone, PartialEq)]
enum FileMode {
    Read,
    Write,
    Append,
}

impl UtlFile {
    pub fn new() -> Self {
        Self {
            file_handles: Arc::new(RwLock::new(HashMap::new())),
            next_handle_id: Arc::new(RwLock::new(1)),
            directories: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Register a directory for file operations
    pub fn add_directory(&self, alias: String, path: PathBuf) {
        let mut directories = self.directories.write();
        directories.insert(alias, path);
    }

    // Open a file
    pub fn fopen(&self, directory: String, filename: String, mode: String) -> Result<i32> {
        let directories = self.directories.read();
        let dir_path = directories
            .get(&directory)
            .ok_or_else(|| DbError::NotFound(format!("Directory '{}' not found", directory)))?;

        let file_path = dir_path.join(&filename);

        let file_mode = match mode.to_uppercase().as_str() {
            "R" => FileMode::Read,
            "W" => FileMode::Write,
            "A" => FileMode::Append,
            _ => {
                return Err(DbError::InvalidInput(format!(
                    "Invalid file mode: {}",
                    mode
                )))
            }
        };

        let file = match file_mode {
            FileMode::Read => {
                File::open(&file_path).map_err(|e| DbError::IoError(e.to_string()))?
            }
            FileMode::Write => OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&file_path)
                .map_err(|e| DbError::IoError(e.to_string()))?,
            FileMode::Append => OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(&file_path)
                .map_err(|e| DbError::IoError(e.to_string()))?,
        };

        let mut next_id = self.next_handle_id.write();
        let mut handles = self.file_handles.write();

        let handle_id = *next_id;
        *next_id += 1;

        let handle = FileHandle {
            id: handle_id,
            directory: directory.clone(),
            filename: filename.clone(),
            mode: file_mode,
            file: Some(file),
        };

        handles.insert(handle_id, handle);
        Ok(handle_id)
    }

    // Write a line to file
    pub fn put_line(&self, handle_id: i32, text: String) -> Result<()> {
        let mut handles = self.file_handles.write();

        let handle = handles
            .get_mut(&handle_id)
            .ok_or_else(|| DbError::NotFound(format!("File handle {} not found", handle_id)))?;

        if handle.mode == FileMode::Read {
            return Err(DbError::InvalidInput(
                "Cannot write to file opened for reading".to_string(),
            ));
        }

        if let Some(ref mut file) = handle.file {
            writeln!(file, "{}", text).map_err(|e| DbError::IoError(e.to_string()))?;
        }

        Ok(())
    }

    // Read a line from file
    pub fn get_line(&self, handle_id: i32) -> Result<String> {
        let mut handles = self.file_handles.write();

        let handle = handles
            .get_mut(&handle_id)
            .ok_or_else(|| DbError::NotFound(format!("File handle {} not found", handle_id)))?;

        if handle.mode != FileMode::Read {
            return Err(DbError::InvalidInput(
                "Cannot read from file opened for writing".to_string(),
            ));
        }

        if let Some(ref mut file) = handle.file {
            let mut reader = BufReader::new(file);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| DbError::IoError(e.to_string()))?;

            // Remove trailing newline
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }

            Ok(line)
        } else {
            Err(DbError::Runtime("File not open".to_string()))
        }
    }

    // Close a file
    pub fn fclose(&self, handle_id: i32) -> Result<()> {
        let mut handles = self.file_handles.write();

        if handles.remove(&handle_id).is_none() {
            return Err(DbError::NotFound(format!(
                "File handle {} not found",
                handle_id
            )));
        }

        Ok(())
    }

    // Check if file is open
    pub fn is_open(&self, handle_id: i32) -> bool {
        let handles = self.file_handles.read();
        handles.contains_key(&handle_id)
    }
}

impl Default for UtlFile {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// DBMS_SCHEDULER - Job scheduling
// ============================================================================

// DBMS_SCHEDULER package for job scheduling
pub struct DbmsScheduler {
    jobs: Arc<RwLock<HashMap<String, ScheduledJob>>>,
}

// Scheduled job
#[derive(Debug, Clone)]
struct ScheduledJob {
    name: String,
    job_type: JobType,
    job_action: String,
    schedule: Schedule,
    enabled: bool,
    auto_drop: bool,
    comments: Option<String>,
    /// Last run time
    last_run_time: Option<DateTime<Utc>>,
    /// Next scheduled run time
    next_run_time: Option<DateTime<Utc>>,
    /// Total run count
    run_count: u64,
    /// Failure count
    failure_count: u64,
    /// Last run duration in milliseconds
    last_run_duration_ms: Option<u64>,
    /// Last error message if failed
    last_error: Option<String>,
    /// Job state
    state: JobState,
    /// Maximum runtime in seconds (0 = unlimited)
    max_runtime_seconds: u64,
    /// Maximum failures before auto-disable
    max_failures: u32,
    /// Retry count on failure
    retry_count: u32,
    /// Retry delay in seconds
    retry_delay_seconds: u64,
}

/// Job execution state
#[derive(Debug, Clone, PartialEq, Default)]
enum JobState {
    #[default]
    Scheduled,
    Running,
    Completed,
    Failed,
    Disabled,
}

// Job type
#[derive(Debug, Clone)]
enum JobType {
    PlSqlBlock,
    StoredProcedure,
    Executable,
}

// Schedule definition
#[derive(Debug, Clone)]
enum Schedule {
    // Run once at a specific time
    Once { run_date: String },
    // Recurring with interval
    Recurring { interval: String },
    // Calendar expression
    #[allow(dead_code)]
    Calendar { expression: String },
}

impl DbmsScheduler {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Create a job
    pub fn create_job(
        &self,
        name: String,
        job_type: String,
        job_action: String,
        startdate: Option<String>,
        repeatinterval: Option<String>,
        enabled: bool,
    ) -> Result<()> {
        let mut jobs = self.jobs.write();

        if jobs.contains_key(&name) {
            return Err(DbError::AlreadyExists(format!(
                "Job '{}' already exists",
                name
            )));
        }

        let job_type_enum = match job_type.to_uppercase().as_str() {
            "PLSQL_BLOCK" => JobType::PlSqlBlock,
            "STORED_PROCEDURE" => JobType::StoredProcedure,
            "EXECUTABLE" => JobType::Executable,
            _ => {
                return Err(DbError::InvalidInput(format!(
                    "Invalid job type: {}",
                    job_type
                )))
            }
        };

        let schedule = if let Some(interval) = repeatinterval {
            Schedule::Recurring { interval }
        } else if let Some(date) = startdate {
            Schedule::Once { run_date: date }
        } else {
            return Err(DbError::InvalidInput(
                "Must specify either start_date or repeat_interval".to_string(),
            ));
        };

        // Calculate next run time based on schedule
        let next_run_time = Self::calculate_next_run_time(&schedule)?;

        let job = ScheduledJob {
            name: name.clone(),
            job_type: job_type_enum,
            job_action,
            schedule,
            enabled,
            auto_drop: false,
            comments: None,
            last_run_time: None,
            next_run_time: Some(next_run_time),
            run_count: 0,
            failure_count: 0,
            last_run_duration_ms: None,
            last_error: None,
            state: if enabled {
                JobState::Scheduled
            } else {
                JobState::Disabled
            },
            max_runtime_seconds: 3600, // 1 hour default
            max_failures: 5,
            retry_count: 3,
            retry_delay_seconds: 60,
        };

        jobs.insert(name, job);
        Ok(())
    }

    // Calculate next run time based on schedule
    fn calculate_next_run_time(schedule: &Schedule) -> Result<DateTime<Utc>> {
        match schedule {
            Schedule::Once { run_date } => {
                // Parse the run date string
                Self::parse_datetime(run_date)
            }
            Schedule::Recurring { interval } => {
                // Parse interval expression (e.g., "FREQ=DAILY", "FREQ=HOURLY;INTERVAL=2")
                Self::parse_interval_to_next_run(interval)
            }
            Schedule::Calendar { expression } => {
                // Parse calendar expression
                Self::parse_calendar_expression(expression)
            }
        }
    }

    // Parse datetime string
    fn parse_datetime(date_str: &str) -> Result<DateTime<Utc>> {
        // Try multiple datetime formats
        let formats = [
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%dT%H:%M:%S",
            "%Y-%m-%d",
            "%d-%b-%Y %H:%M:%S",
        ];

        for format in &formats {
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, format) {
                return Ok(DateTime::from_naive_utc_and_offset(dt, Utc));
            }
            if let Ok(d) = chrono::NaiveDate::parse_from_str(date_str, format) {
                let dt = d.and_hms_opt(0, 0, 0).unwrap();
                return Ok(DateTime::from_naive_utc_and_offset(dt, Utc));
            }
        }

        Err(DbError::InvalidInput(format!(
            "Cannot parse datetime: {}",
            date_str
        )))
    }

    // Parse interval expression to calculate next run
    fn parse_interval_to_next_run(interval: &str) -> Result<DateTime<Utc>> {
        let now = Utc::now();
        let interval_upper = interval.to_uppercase();

        // Parse FREQ=XXX;INTERVAL=N format
        let mut freq = "DAILY";
        let mut interval_value = 1i64;

        for part in interval_upper.split(';') {
            if let Some(f) = part.strip_prefix("FREQ=") {
                freq = match f {
                    "YEARLY" => "YEARLY",
                    "MONTHLY" => "MONTHLY",
                    "WEEKLY" => "WEEKLY",
                    "DAILY" => "DAILY",
                    "HOURLY" => "HOURLY",
                    "MINUTELY" => "MINUTELY",
                    "SECONDLY" => "SECONDLY",
                    _ => "DAILY",
                };
            } else if let Some(i) = part.strip_prefix("INTERVAL=") {
                interval_value = i.parse().unwrap_or(1);
            }
        }

        // Calculate next run based on frequency
        let next = match freq {
            "YEARLY" => now + chrono::Duration::days(365 * interval_value),
            "MONTHLY" => now + chrono::Duration::days(30 * interval_value),
            "WEEKLY" => now + chrono::Duration::weeks(interval_value),
            "DAILY" => now + chrono::Duration::days(interval_value),
            "HOURLY" => now + chrono::Duration::hours(interval_value),
            "MINUTELY" => now + chrono::Duration::minutes(interval_value),
            "SECONDLY" => now + chrono::Duration::seconds(interval_value),
            _ => now + chrono::Duration::days(1),
        };

        Ok(next)
    }

    // Parse calendar expression
    fn parse_calendar_expression(_expression: &str) -> Result<DateTime<Utc>> {
        // Calendar expressions are complex (BYDAY, BYHOUR, etc.)
        // For now, default to next hour
        Ok(Utc::now() + chrono::Duration::hours(1))
    }

    // Enable a job
    pub fn enable_job(&self, job_name: &str) -> Result<()> {
        let mut jobs = self.jobs.write();

        let job = jobs
            .get_mut(job_name)
            .ok_or_else(|| DbError::NotFound(format!("Job '{}' not found", job_name)))?;

        job.enabled = true;
        Ok(())
    }

    // Disable a job
    pub fn disable_job(&self, job_name: &str) -> Result<()> {
        let mut jobs = self.jobs.write();

        let job = jobs
            .get_mut(job_name)
            .ok_or_else(|| DbError::NotFound(format!("Job '{}' not found", job_name)))?;

        job.enabled = false;
        Ok(())
    }

    // Drop a job
    pub fn drop_job(&self, job_name: &str) -> Result<()> {
        let mut jobs = self.jobs.write();

        if jobs.remove(job_name).is_none() {
            return Err(DbError::NotFound(format!("Job '{}' not found", job_name)));
        }

        Ok(())
    }

    // Run a job immediately
    pub fn run_job(&self, job_name: &str) -> Result<()> {
        // Get a clone of the job to execute (to release the lock)
        let job_clone = {
            let jobs = self.jobs.read();
            let job = jobs
                .get(job_name)
                .ok_or_else(|| DbError::NotFound(format!("Job '{}' not found", job_name)))?;

            if !job.enabled {
                return Err(DbError::InvalidInput(format!(
                    "Job '{}' is disabled",
                    job_name
                )));
            }

            if job.state == JobState::Running {
                return Err(DbError::InvalidInput(format!(
                    "Job '{}' is already running",
                    job_name
                )));
            }

            job.clone()
        };

        // Mark job as running
        {
            let mut jobs = self.jobs.write();
            if let Some(job) = jobs.get_mut(job_name) {
                job.state = JobState::Running;
            }
        }

        // Execute the job
        let start_time = Instant::now();
        let result = self.execute_job_action(&job_clone);
        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Update job state based on result
        {
            let mut jobs = self.jobs.write();
            if let Some(job) = jobs.get_mut(job_name) {
                job.last_run_time = Some(Utc::now());
                job.last_run_duration_ms = Some(duration_ms);
                job.run_count += 1;

                match &result {
                    Ok(_) => {
                        job.state = JobState::Completed;
                        job.last_error = None;
                        job.failure_count = 0; // Reset on success

                        // Update next run time for recurring jobs
                        if let Schedule::Recurring { .. } = &job.schedule {
                            if let Ok(next) = Self::calculate_next_run_time(&job.schedule) {
                                job.next_run_time = Some(next);
                                job.state = JobState::Scheduled;
                            }
                        } else if job.auto_drop {
                            // Auto-drop one-time jobs after successful completion
                            // (handled separately)
                        }
                    }
                    Err(e) => {
                        job.state = JobState::Failed;
                        job.last_error = Some(e.to_string());
                        job.failure_count += 1;

                        // Auto-disable if max failures exceeded
                        if job.failure_count >= job.max_failures as u64 {
                            job.enabled = false;
                            job.state = JobState::Disabled;
                        } else {
                            // Schedule retry
                            job.state = JobState::Scheduled;
                            job.next_run_time = Some(
                                Utc::now()
                                    + chrono::Duration::seconds(job.retry_delay_seconds as i64),
                            );
                        }
                    }
                }
            }
        }

        result
    }

    // Execute the job action based on job type
    fn execute_job_action(&self, job: &ScheduledJob) -> Result<()> {
        match job.job_type {
            JobType::PlSqlBlock => self.execute_plsql_job(&job.job_action),
            JobType::StoredProcedure => self.execute_stored_procedure_job(&job.job_action),
            JobType::Executable => self.execute_external_job(&job.job_action),
        }
    }

    // Execute a PL/SQL block job
    fn execute_plsql_job(&self, action: &str) -> Result<()> {
        // Validate the PL/SQL block
        let action_trimmed = action.trim();

        // Check for valid PL/SQL block structure
        let action_upper = action_trimmed.to_uppercase();
        if !action_upper.starts_with("BEGIN") && !action_upper.starts_with("DECLARE") {
            return Err(DbError::InvalidInput(
                "PL/SQL block must start with BEGIN or DECLARE".to_string(),
            ));
        }

        if !action_upper.contains("END") {
            return Err(DbError::InvalidInput(
                "PL/SQL block must contain END statement".to_string(),
            ));
        }

        // In production, this would:
        // 1. Parse the PL/SQL block using PlSqlParser
        // 2. Execute using RuntimeExecutor
        // 3. Handle any raised exceptions

        // Example integration (commented out as it requires the full runtime):
        // let parser = PlSqlParser::new();
        // let block = parser.parse(action)?;
        // let executor = RuntimeExecutor::new();
        // executor.execute(&block)?;

        Ok(())
    }

    // Execute a stored procedure job
    fn execute_stored_procedure_job(&self, action: &str) -> Result<()> {
        // Parse the procedure call: SCHEMA.PROCEDURE_NAME or just PROCEDURE_NAME
        let action_trimmed = action.trim();

        // Extract procedure name and optional parameters
        let (proc_name, _params) = if let Some(paren_pos) = action_trimmed.find('(') {
            let name = &action_trimmed[..paren_pos];
            let params_str = &action_trimmed[paren_pos..];
            (name.trim(), Some(params_str))
        } else {
            (action_trimmed, None)
        };

        // Validate procedure name format
        if proc_name.is_empty() {
            return Err(DbError::InvalidInput(
                "Procedure name cannot be empty".to_string(),
            ));
        }

        // Check for valid identifier characters
        for ch in proc_name.chars() {
            if !ch.is_alphanumeric() && ch != '_' && ch != '.' {
                return Err(DbError::InvalidInput(format!(
                    "Invalid character '{}' in procedure name",
                    ch
                )));
            }
        }

        // In production, this would:
        // 1. Look up the procedure in the ProcedureManager
        // 2. Parse the parameters
        // 3. Execute the procedure with the provided parameters

        Ok(())
    }

    // Execute an external executable job
    fn execute_external_job(&self, action: &str) -> Result<()> {
        use std::process::Command;

        let action_trimmed = action.trim();

        // Security: Validate the executable path
        // Only allow executables in designated directories
        let allowed_prefixes = ["/usr/bin/", "/usr/local/bin/", "/opt/rustydb/jobs/"];

        let is_allowed = allowed_prefixes
            .iter()
            .any(|prefix| action_trimmed.starts_with(prefix));

        if !is_allowed && !cfg!(test) {
            return Err(DbError::InvalidInput(format!(
                "Executable path must start with one of: {:?}",
                allowed_prefixes
            )));
        }

        // Parse command and arguments
        let parts: Vec<&str> = action_trimmed.split_whitespace().collect();
        if parts.is_empty() {
            return Err(DbError::InvalidInput(
                "Executable command cannot be empty".to_string(),
            ));
        }

        let executable = parts[0];
        let args = &parts[1..];

        // Execute the command
        let output = Command::new(executable)
            .args(args)
            .output()
            .map_err(|e| DbError::Runtime(format!("Failed to execute command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DbError::Runtime(format!(
                "Command failed with exit code {:?}: {}",
                output.status.code(),
                stderr
            )));
        }

        Ok(())
    }

    // List all jobs
    pub fn list_jobs(&self) -> Vec<String> {
        let jobs = self.jobs.read();
        jobs.keys().cloned().collect()
    }

    // Get detailed job information
    pub fn get_job_details(&self, job_name: &str) -> Result<JobDetails> {
        let jobs = self.jobs.read();

        let job = jobs
            .get(job_name)
            .ok_or_else(|| DbError::NotFound(format!("Job '{}' not found", job_name)))?;

        Ok(JobDetails {
            name: job.name.clone(),
            job_type: format!("{:?}", job.job_type),
            job_action: job.job_action.clone(),
            schedule: format!("{:?}", job.schedule),
            enabled: job.enabled,
            state: format!("{:?}", job.state),
            last_run_time: job.last_run_time.map(|dt| dt.to_rfc3339()),
            next_run_time: job.next_run_time.map(|dt| dt.to_rfc3339()),
            run_count: job.run_count,
            failure_count: job.failure_count,
            last_run_duration_ms: job.last_run_duration_ms,
            last_error: job.last_error.clone(),
        })
    }

    // Set job attribute
    pub fn set_attribute(&self, job_name: &str, attribute: &str, value: &str) -> Result<()> {
        let mut jobs = self.jobs.write();

        let job = jobs
            .get_mut(job_name)
            .ok_or_else(|| DbError::NotFound(format!("Job '{}' not found", job_name)))?;

        match attribute.to_uppercase().as_str() {
            "JOB_ACTION" => {
                job.job_action = value.to_string();
            }
            "COMMENTS" => {
                job.comments = Some(value.to_string());
            }
            "AUTO_DROP" => {
                job.auto_drop = value.to_uppercase() == "TRUE";
            }
            "MAX_RUNTIME" => {
                job.max_runtime_seconds = value
                    .parse()
                    .map_err(|_| DbError::InvalidInput("Invalid max_runtime value".to_string()))?;
            }
            "MAX_FAILURES" => {
                job.max_failures = value
                    .parse()
                    .map_err(|_| DbError::InvalidInput("Invalid max_failures value".to_string()))?;
            }
            "RETRY_COUNT" => {
                job.retry_count = value
                    .parse()
                    .map_err(|_| DbError::InvalidInput("Invalid retry_count value".to_string()))?;
            }
            "RETRY_DELAY" => {
                job.retry_delay_seconds = value
                    .parse()
                    .map_err(|_| DbError::InvalidInput("Invalid retry_delay value".to_string()))?;
            }
            _ => {
                return Err(DbError::InvalidInput(format!(
                    "Unknown job attribute: {}",
                    attribute
                )));
            }
        }

        Ok(())
    }

    // Copy a job
    pub fn copy_job(&self, source_name: &str, destination_name: &str) -> Result<()> {
        let jobs = self.jobs.read();

        let source = jobs
            .get(source_name)
            .ok_or_else(|| DbError::NotFound(format!("Job '{}' not found", source_name)))?;

        if jobs.contains_key(destination_name) {
            return Err(DbError::AlreadyExists(format!(
                "Job '{}' already exists",
                destination_name
            )));
        }

        let mut new_job = source.clone();
        new_job.name = destination_name.to_string();
        new_job.run_count = 0;
        new_job.failure_count = 0;
        new_job.last_run_time = None;
        new_job.last_error = None;
        new_job.state = if new_job.enabled {
            JobState::Scheduled
        } else {
            JobState::Disabled
        };

        drop(jobs);

        let mut jobs = self.jobs.write();
        jobs.insert(destination_name.to_string(), new_job);

        Ok(())
    }

    // Stop a running job
    pub fn stop_job(&self, job_name: &str, force: bool) -> Result<()> {
        let mut jobs = self.jobs.write();

        let job = jobs
            .get_mut(job_name)
            .ok_or_else(|| DbError::NotFound(format!("Job '{}' not found", job_name)))?;

        if job.state != JobState::Running {
            return Err(DbError::InvalidInput(format!(
                "Job '{}' is not running",
                job_name
            )));
        }

        // In production, this would signal the running job to stop
        // For now, we just update the state
        if force {
            job.state = JobState::Failed;
            job.last_error = Some("Job forcefully stopped".to_string());
        } else {
            // Graceful stop - job should check for stop signal
            job.state = JobState::Scheduled;
        }

        Ok(())
    }

    // Get running jobs
    pub fn get_running_jobs(&self) -> Vec<String> {
        let jobs = self.jobs.read();
        jobs.iter()
            .filter(|(_, job)| job.state == JobState::Running)
            .map(|(name, _)| name.clone())
            .collect()
    }

    // Purge job run history (resets counters)
    pub fn purge_log(&self, job_name: &str) -> Result<()> {
        let mut jobs = self.jobs.write();

        let job = jobs
            .get_mut(job_name)
            .ok_or_else(|| DbError::NotFound(format!("Job '{}' not found", job_name)))?;

        job.run_count = 0;
        job.failure_count = 0;
        job.last_run_time = None;
        job.last_run_duration_ms = None;
        job.last_error = None;

        Ok(())
    }
}

/// Detailed job information returned by get_job_details
#[derive(Debug, Clone)]
pub struct JobDetails {
    pub name: String,
    pub job_type: String,
    pub job_action: String,
    pub schedule: String,
    pub enabled: bool,
    pub state: String,
    pub last_run_time: Option<String>,
    pub next_run_time: Option<String>,
    pub run_count: u64,
    pub failure_count: u64,
    pub last_run_duration_ms: Option<u64>,
    pub last_error: Option<String>,
}

impl Default for DbmsScheduler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// DBMS_LOCK - Lock management
// ============================================================================

// DBMS_LOCK package for user-defined locks
pub struct DbmsLock {
    locks: Arc<RwLock<HashMap<String, LockHandle>>>,
}

// Lock handle
#[derive(Debug, Clone)]
struct LockHandle {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    mode: LockMode,
    #[allow(dead_code)]
    timeout: Option<i32>,
}

// Lock mode
#[derive(Debug, Clone, PartialEq)]
enum LockMode {
    Exclusive,
    Shared,
    Update,
}

impl DbmsLock {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Request a lock
    pub fn request(&self, lockid: String, lockmode: String, timeout: Option<i32>) -> Result<i32> {
        let mode = match lockmode.to_uppercase().as_str() {
            "EXCLUSIVE" | "X" => LockMode::Exclusive,
            "SHARED" | "S" => LockMode::Shared,
            "UPDATE" | "U" => LockMode::Update,
            _ => {
                return Err(DbError::InvalidInput(format!(
                    "Invalid lock mode: {}",
                    lockmode
                )))
            }
        };

        let mut locks = self.locks.write();

        // Check if lock already exists
        if locks.contains_key(&lockid) {
            return Ok(1); // Lock already held
        }

        let handle = LockHandle {
            id: lockid.clone(),
            mode,
            timeout,
        };

        locks.insert(lockid, handle);
        Ok(0) // Success
    }

    // Release a lock
    pub fn release(&self, lock_id: &str) -> Result<i32> {
        let mut locks = self.locks.write();

        if locks.remove(lock_id).is_some() {
            Ok(0) // Success
        } else {
            Ok(4) // Lock not held
        }
    }

    // Sleep for specified seconds
    pub fn sleep(&self, seconds: f64) -> Result<()> {
        let duration = std::time::Duration::from_secs_f64(seconds);
        std::thread::sleep(duration);
        Ok(())
    }
}

impl Default for DbmsLock {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Built-in Packages Manager
// ============================================================================

// Manager for all built-in packages
pub struct BuiltInPackages {
    pub dbms_output: DbmsOutput,
    pub dbms_sql: DbmsSql,
    pub utl_file: UtlFile,
    pub dbms_scheduler: DbmsScheduler,
    pub dbms_lock: DbmsLock,
}

impl BuiltInPackages {
    pub fn new() -> Self {
        Self {
            dbms_output: DbmsOutput::new(),
            dbms_sql: DbmsSql::new(),
            utl_file: UtlFile::new(),
            dbms_scheduler: DbmsScheduler::new(),
            dbms_lock: DbmsLock::new(),
        }
    }

    // Initialize built-in packages with configuration
    pub fn initialize(&self) {
        // Set up default directories for UTL_FILE
        self.utl_file
            .add_directory("TEMP".to_string(), PathBuf::from("/tmp"));
    }
}

impl Default for BuiltInPackages {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbms_output() -> Result<()> {
        let output = DbmsOutput::new();

        output.enable(Some(1000));
        output.put_line("Hello, World!".to_string())?;
        output.put_line("Line 2".to_string())?;

        let line1 = output.get_line()?;
        assert_eq!(line1, Some("Hello, World!".to_string()));

        let line2 = output.get_line()?;
        assert_eq!(line2, Some("Line 2".to_string()));

        Ok(())
    }

    #[test]
    fn test_dbms_sql() -> Result<()> {
        let dbms_sql = DbmsSql::new();

        let cursor_id = dbms_sql.open_cursor();
        assert!(cursor_id > 0);

        dbms_sql.parse(cursor_id, "SELECT * FROM test".to_string())?;
        dbms_sql.bind_variable(cursor_id, "var1".to_string(), RuntimeValue::Integer(42))?;

        assert!(dbms_sql.is_open(cursor_id));

        dbms_sql.close_cursor(cursor_id)?;
        assert!(!dbms_sql.is_open(cursor_id));

        Ok(())
    }

    #[test]
    fn test_dbms_scheduler() -> Result<()> {
        let scheduler = DbmsScheduler::new();

        scheduler.create_job(
            "test_job".to_string(),
            "PLSQL_BLOCK".to_string(),
            "BEGIN NULL; END;".to_string(),
            None,
            Some("FREQ=DAILY".to_string()),
            true,
        )?;

        let jobs = scheduler.list_jobs();
        assert_eq!(jobs.len(), 1);

        scheduler.disable_job("test_job")?;
        scheduler.drop_job("test_job")?;

        let jobs = scheduler.list_jobs();
        assert_eq!(jobs.len(), 0);

        Ok(())
    }

    #[test]
    fn test_dbms_lock() -> Result<()> {
        let lock_mgr = DbmsLock::new();

        let result = lock_mgr.request("my_lock".to_string(), "EXCLUSIVE".to_string(), Some(10))?;

        assert_eq!(result, 0); // Success

        let release_result = lock_mgr.release("my_lock")?;
        assert_eq!(release_result, 0); // Success

        Ok(())
    }
}
