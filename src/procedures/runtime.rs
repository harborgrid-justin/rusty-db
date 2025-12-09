// PL/SQL Runtime Execution Engine
//
// This module provides the runtime environment for executing PL/SQL blocks,
// managing execution context, variable bindings, and control flow.

use crate::{Result, DbError};
use crate::procedures::parser::{
    PlSqlBlock, Statement, Expression, LiteralValue, BinaryOperator, UnaryOperator,
    Declaration, ExceptionHandler, ExceptionType,
};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

// Runtime value types
#[repr(C)]
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Date(String),
    Timestamp(String),
    Null,
    Cursor(CursorState),
    Record(HashMap<String, RuntimeValue>),
    Array(Vec<RuntimeValue>),
}

impl RuntimeValue {
    // Convert to integer if possible
    #[inline]
    pub fn as_integer(&self) -> Result<i64> {
        match self {
            RuntimeValue::Integer(v) => Ok(*v),
            RuntimeValue::Float(v) => Ok(*v as i64),
            RuntimeValue::String(s) => s.parse::<i64>()
                .map_err(|_| DbError::Runtime(format!("Cannot convert '{}' to integer", s))),
            _ => Err(DbError::Runtime(format!("Cannot convert {:?} to integer", self))),
        }
    }

    // Convert to float if possible
    #[inline]
    pub fn as_float(&self) -> Result<f64> {
        match self {
            RuntimeValue::Float(v) => Ok(*v),
            RuntimeValue::Integer(v) => Ok(*v as f64),
            RuntimeValue::String(s) => s.parse::<f64>()
                .map_err(|_| DbError::Runtime(format!("Cannot convert '{}' to float", s))),
            _ => Err(DbError::Runtime(format!("Cannot convert {:?} to float", self))),
        }
    }

    // Convert to string
    pub fn as_string(&self) -> String {
        match self {
            RuntimeValue::String(s) => s.clone(),
            RuntimeValue::Integer(v) => v.to_string(),
            RuntimeValue::Float(v) => v.to_string(),
            RuntimeValue::Boolean(b) => b.to_string(),
            RuntimeValue::Date(d) => d.clone(),
            RuntimeValue::Timestamp(t) => t.clone(),
            RuntimeValue::Null => "NULL".to_string(),
            _ => format!("{:?}", self),
        }
    }

    // Convert to boolean
    #[inline]
    pub fn as_boolean(&self) -> Result<bool> {
        match self {
            RuntimeValue::Boolean(b) => Ok(*b),
            RuntimeValue::Integer(v) => Ok(*v != 0),
            RuntimeValue::Null => Ok(false),
            _ => Err(DbError::Runtime(format!("Cannot convert {:?} to boolean", self))),
        }
    }

    // Check if value is null
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, RuntimeValue::Null)
    }
}

// Cursor state for runtime
#[derive(Debug, Clone, PartialEq)]
pub struct CursorState {
    pub name: String,
    pub query: String,
    pub is_open: bool,
    pub current_row: usize,
    pub rows: Vec<HashMap<String, RuntimeValue>>,
    bulk_exceptions: Vec<BulkException>,
    position: i32,
}

// Bulk exception for FORALL operations
#[derive(Debug, Clone, PartialEq)]
pub struct BulkException {
    pub index: usize,
    pub error_code: i32,
    pub error_message: String,
}

// Execution context for a PL/SQL block
pub struct ExecutionContext {
    // Variable bindings (name -> value)
    variables: HashMap<String, RuntimeValue>,
    // Output parameters
    output_params: HashMap<String, RuntimeValue>,
    // Parent context (for nested blocks)
    parent: Option<Arc<RwLock<ExecutionContext>>>,
    // Return value
    return_value: Option<RuntimeValue>,
    // Exception state
    exception_raised: Option<String>,
    // Loop control flags
    exit_loop: bool,
    continue_loop: bool,
    // Transaction savepoints
    savepoints: Vec<String>,
    // Cursors
    cursors: HashMap<String, CursorState>,
    // Debug mode
    debug: bool,
    // Output buffer (for DBMS_OUTPUT)
    output_buffer: Vec<String>,
    // Rows affected counter
    rows_affected: usize,
    // Procedure calls recorded for later execution
    procedure_calls: Vec<(String, Vec<RuntimeValue>)>,
    // DML operations log
    dml_operations: Vec<(String, String, usize)>, // (operation, table, rows)
    // Transaction committed flag
    transaction_committed: bool,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            output_params: HashMap::new(),
            parent: None,
            return_value: None,
            exception_raised: None,
            exit_loop: false,
            continue_loop: false,
            savepoints: Vec::new(),
            cursors: HashMap::new(),
            debug: false,
            output_buffer: Vec::new(),
            rows_affected: 0,
            procedure_calls: Vec::new(),
            dml_operations: Vec::new(),
            transaction_committed: false,
        }
    }

    // Create a child context
    pub fn create_child(&self) -> Self {
        Self {
            variables: HashMap::new(),
            output_params: HashMap::new(),
            parent: None, // Simplified - in production would link to parent
            return_value: None,
            exception_raised: None,
            exit_loop: false,
            continue_loop: false,
            savepoints: Vec::new(),
            cursors: HashMap::new(),
            debug: self.debug,
            output_buffer: Vec::new(),
            rows_affected: 0,
            procedure_calls: Vec::new(),
            dml_operations: Vec::new(),
            transaction_committed: false,
        }
    }

    // Set a variable value
    #[inline]
    pub fn set_variable(&mut self, name: String, value: RuntimeValue) {
        self.variables.insert(name, value);
    }

    // Get a variable value
    #[inline]
    pub fn get_variable(&self, name: &str) -> Result<RuntimeValue> {
        self.variables.get(name)
            .cloned()
            .ok_or_else(|| Self::variable_not_found(name))
    }

    #[cold]
    #[inline(never)]
    fn variable_not_found(name: &str) -> DbError {
        DbError::Runtime(format!("Variable '{}' not found", name))
    }

    // Set an output parameter
    pub fn set_output(&mut self, name: String, value: RuntimeValue) {
        self.output_params.insert(name, value);
    }

    // Get all output parameters
    pub fn get_outputs(&self) -> &HashMap<String, RuntimeValue> {
        &self.output_params
    }

    // Set return value
    pub fn set_return(&mut self, value: RuntimeValue) {
        self.return_value = Some(value);
    }

    // Get return value
    pub fn get_return(&self) -> Option<&RuntimeValue> {
        self.return_value.as_ref()
    }

    // Raise an exception
    pub fn raise_exception(&mut self, exception: String) {
        self.exception_raised = Some(exception);
    }

    // Check if exception is raised
    pub fn has_exception(&self) -> bool {
        self.exception_raised.is_some()
    }

    // Get exception name
    pub fn get_exception(&self) -> Option<&String> {
        self.exception_raised.as_ref()
    }

    // Clear exception
    pub fn clear_exception(&mut self) {
        self.exception_raised = None;
    }

    // Set exit loop flag
    pub fn set_exit_loop(&mut self) {
        self.exit_loop = true;
    }

    // Check exit loop flag
    pub fn should_exit_loop(&self) -> bool {
        self.exit_loop
    }

    // Clear exit loop flag
    pub fn clear_exit_loop(&mut self) {
        self.exit_loop = false;
    }

    // Set continue loop flag
    pub fn set_continue_loop(&mut self) {
        self.continue_loop = true;
    }

    // Check continue loop flag
    pub fn should_continue_loop(&self) -> bool {
        self.continue_loop
    }

    // Clear continue loop flag
    pub fn clear_continue_loop(&mut self) {
        self.continue_loop = false;
    }

    // Add output line (for DBMS_OUTPUT)
    pub fn add_output(&mut self, line: String) {
        self.output_buffer.push(line);
    }

    // Get output buffer
    pub fn get_output_buffer(&self) -> &[String] {
        &self.output_buffer
    }

    // Enable debug mode
    pub fn enable_debug(&mut self) {
        self.debug = true;
    }

    // Check if debug mode is enabled
    pub fn is_debug(&self) -> bool {
        self.debug
    }

    // Get rows affected
    pub fn get_rows_affected(&self) -> usize {
        self.rows_affected
    }

    // Increment rows affected
    pub fn increment_rows_affected(&mut self, count: usize) {
        self.rows_affected += count;
    }

    // Record DML operation
    pub fn record_dml_operation(&mut self, operation: &str, table: &str, rows: usize) {
        self.dml_operations.push((operation.to_string(), table.to_string(), rows));
        self.rows_affected += rows;
    }

    // Record procedure call
    pub fn record_procedure_call(&mut self, name: &str, args: Vec<RuntimeValue>) {
        self.procedure_calls.push((name.to_string(), args));
    }

    // Append output (for DBMS_OUTPUT.PUT_LINE)
    pub fn append_output(&mut self, text: &str) {
        self.output_buffer.push(text.to_string());
    }

    // Transaction management
    pub fn commit_transaction(&mut self) -> Result<()> {
        self.transaction_committed = true;
        self.savepoints.clear();
        Ok(())
    }

    pub fn rollback_transaction(&mut self) -> Result<()> {
        self.transaction_committed = false;
        self.savepoints.clear();
        Ok(())
    }

    pub fn rollback_to_savepoint(&mut self, name: &str) -> Result<()> {
        // Find savepoint and remove all savepoints after it
        if let Some(pos) = self.savepoints.iter().position(|s| s == name) {
            self.savepoints.truncate(pos + 1);
            Ok(())
        } else {
            Err(DbError::Runtime(format!("Savepoint '{}' not found", name)))
        }
    }

    pub fn create_savepoint(&mut self, name: &str) -> Result<()> {
        self.savepoints.push(name.to_string());
        Ok(())
    }

    // Cursor management
    pub fn open_cursor(&mut self, name: &str, rows: Vec<HashMap<String, RuntimeValue>>) -> Result<()> {
        let cursor = CursorState {
            rows,
            position: 0,
            is_open: true,
            bulk_exceptions: Vec::new(),
            name: todo!(),
            query: todo!(),
            current_row: todo!(),
        };
        self.cursors.insert(name.to_string(), cursor);
        Ok(())
    }

    pub fn fetch_cursor(&mut self, name: &str) -> Result<Option<HashMap<String, RuntimeValue>>> {
        if let Some(cursor) = self.cursors.get_mut(name) {
            if !cursor.is_open {
                return Err(DbError::Runtime(format!("Cursor '{}' is not open", name)));
            }
            if (cursor.position as usize) < cursor.rows.len() {
                let row = cursor.rows[cursor.position as usize].clone();
                cursor.position += 1;
                Ok(Some(row))
            } else {
                Ok(None)
            }
        } else {
            Err(DbError::Runtime(format!("Cursor '{}' not found", name)))
        }
    }

    pub fn close_cursor(&mut self, name: &str) -> Result<()> {
        if let Some(cursor) = self.cursors.get_mut(name) {
            cursor.is_open = false;
            Ok(())
        } else {
            Err(DbError::Runtime(format!("Cursor '{}' not found", name)))
        }
    }

    pub fn is_cursor_open(&self, name: &str) -> bool {
        self.cursors.get(name).map_or(false, |c| c.is_open)
    }

    pub fn set_cursor_not_found(&mut self, _found: bool) {
        // This sets the SQL%NOTFOUND implicit cursor attribute
        // In a full implementation, this would update an implicit cursor state
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

// PL/SQL Runtime Executor
pub struct RuntimeExecutor {
    // Global execution context
    context: Arc<RwLock<ExecutionContext>>,
}

impl RuntimeExecutor {
    pub fn new() -> Self {
        Self {
            context: Arc::new(RwLock::new(ExecutionContext::new())),
        }
    }

    // Execute a PL/SQL block
    pub fn execute(&self, block: &PlSqlBlock) -> Result<ExecutionResult> {
        let mut ctx = self.context.write();

        // Initialize declarations
        for decl in &block.declarations {
            self.execute_declaration(&mut ctx, decl)?;
        }

        // Execute statements
        for stmt in &block.statements {
            self.execute_statement(&mut ctx, stmt)?;

            // Check for exceptions
            if ctx.has_exception() {
                break;
            }

            // Check for return
            if ctx.get_return().is_some() {
                break;
            }
        }

        // Handle exceptions
        if let Some(exception_name) = ctx.get_exception().cloned() {
            self.handle_exception(&mut ctx, &exception_name, &block.exception_handlers)?;
        }

        // Prepare result
        let result = ExecutionResult {
            return_value: ctx.get_return().cloned(),
            output_params: ctx.get_outputs().clone(),
            rows_affected: ctx.get_rows_affected(),
            output_lines: ctx.get_output_buffer().to_vec(),
        };

        Ok(result)
    }

    // Execute a declaration
    fn execute_declaration(&self, ctx: &mut ExecutionContext, decl: &Declaration) -> Result<()> {
        let value = if let Some(init_expr) = &decl.initial_value {
            self.evaluate_expression(ctx, init_expr)?
        } else {
            RuntimeValue::Null
        };

        // Check NOT NULL constraint
        if decl.not_null && value.is_null() {
            return Err(DbError::Runtime(
                format!("Variable '{}' cannot be null", decl.name)
            ));
        }

        ctx.set_variable(decl.name.clone(), value);
        Ok(())
    }

    // Execute a statement
    fn execute_statement(&self, ctx: &mut ExecutionContext, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::Assignment { target, value } => {
                let val = self.evaluate_expression(ctx, value)?;
                ctx.set_variable(target.clone(), val);
            }

            Statement::If { condition, then_block, elsif_blocks, else_block } => {
                let cond_val = self.evaluate_expression(ctx, condition)?;
                if cond_val.as_boolean()? {
                    for s in then_block {
                        self.execute_statement(ctx, s)?;
                        if ctx.has_exception() || ctx.get_return().is_some() {
                            return Ok(());
                        }
                    }
                } else {
                    let mut executed = false;
                    for (elsif_cond, elsif_stmts) in elsif_blocks {
                        let elsif_val = self.evaluate_expression(ctx, elsif_cond)?;
                        if elsif_val.as_boolean()? {
                            for s in elsif_stmts {
                                self.execute_statement(ctx, s)?;
                                if ctx.has_exception() || ctx.get_return().is_some() {
                                    return Ok(());
                                }
                            }
                            executed = true;
                            break;
                        }
                    }

                    if !executed {
                        if let Some(else_stmts) = else_block {
                            for s in else_stmts {
                                self.execute_statement(ctx, s)?;
                                if ctx.has_exception() || ctx.get_return().is_some() {
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }

            Statement::Loop { statements } => {
                loop {
                    for s in statements {
                        self.execute_statement(ctx, s)?;

                        if ctx.should_exit_loop() {
                            ctx.clear_exit_loop();
                            return Ok(());
                        }

                        if ctx.should_continue_loop() {
                            ctx.clear_continue_loop();
                            break;
                        }

                        if ctx.has_exception() || ctx.get_return().is_some() {
                            return Ok(());
                        }
                    }

                    if ctx.should_exit_loop() {
                        ctx.clear_exit_loop();
                        break;
                    }
                }
            }

            Statement::While { condition, statements } => {
                loop {
                    let cond_val = self.evaluate_expression(ctx, condition)?;
                    if !cond_val.as_boolean()? {
                        break;
                    }

                    for s in statements {
                        self.execute_statement(ctx, s)?;

                        if ctx.should_exit_loop() {
                            ctx.clear_exit_loop();
                            return Ok(());
                        }

                        if ctx.should_continue_loop() {
                            ctx.clear_continue_loop();
                            break;
                        }

                        if ctx.has_exception() || ctx.get_return().is_some() {
                            return Ok(());
                        }
                    }
                }
            }

            Statement::ForNumeric { iterator, reverse, start, end, statements } => {
                let start_val = self.evaluate_expression(ctx, start)?.as_integer()?;
                let end_val = self.evaluate_expression(ctx, end)?.as_integer()?;

                let range: Vec<i64> = if *reverse {
                    (end_val..=start_val).rev().collect()
                } else {
                    (start_val..=end_val).collect()
                };

                for i in range {
                    ctx.set_variable(iterator.clone(), RuntimeValue::Integer(i));

                    for s in statements {
                        self.execute_statement(ctx, s)?;

                        if ctx.should_exit_loop() {
                            ctx.clear_exit_loop();
                            return Ok(());
                        }

                        if ctx.should_continue_loop() {
                            ctx.clear_continue_loop();
                            break;
                        }

                        if ctx.has_exception() || ctx.get_return().is_some() {
                            return Ok(());
                        }
                    }
                }
            }

            Statement::Exit { when } => {
                if let Some(condition) = when {
                    let cond_val = self.evaluate_expression(ctx, condition)?;
                    if cond_val.as_boolean()? {
                        ctx.set_exit_loop();
                    }
                } else {
                    ctx.set_exit_loop();
                }
            }

            Statement::Continue { when } => {
                if let Some(condition) = when {
                    let cond_val = self.evaluate_expression(ctx, condition)?;
                    if cond_val.as_boolean()? {
                        ctx.set_continue_loop();
                    }
                } else {
                    ctx.set_continue_loop();
                }
            }

            Statement::Return { value } => {
                if let Some(expr) = value {
                    let val = self.evaluate_expression(ctx, expr)?;
                    ctx.set_return(val);
                } else {
                    ctx.set_return(RuntimeValue::Null);
                }
            }

            Statement::Raise { exception } => {
                ctx.raise_exception(exception.clone());
            }

            Statement::Commit => {
                // Integrate with transaction manager
                // Mark the current transaction as committed
                ctx.commit_transaction();

                if ctx.is_debug() {
                    ctx.add_output("COMMIT executed".to_string());
                }
            }

            Statement::Rollback { to_savepoint } => {
                // Integrate with transaction manager
                if let Some(ref sp) = to_savepoint {
                    // Rollback to specific savepoint
                    ctx.rollback_to_savepoint(sp)?;
                    if ctx.is_debug() {
                        ctx.add_output(format!("ROLLBACK TO SAVEPOINT {}", sp));
                    }
                } else {
                    // Full rollback
                    ctx.rollback_transaction();
                    if ctx.is_debug() {
                        ctx.add_output("ROLLBACK executed".to_string());
                    }
                }
            }

            Statement::Savepoint { name } => {
                // Integrate with transaction manager
                ctx.create_savepoint(&name);

                if ctx.is_debug() {
                    ctx.add_output(format!("SAVEPOINT {} created", name));
                }
            }

            Statement::Call { name, arguments } => {
                // Evaluate arguments
                let mut arg_values = Vec::new();
                for arg in arguments {
                    arg_values.push(self.evaluate_expression(ctx, arg)?);
                }

                // Integrate with procedure manager to call other procedures
                // Check if this is a built-in procedure
                match name.to_uppercase().as_str() {
                    "DBMS_OUTPUT.PUT_LINE" => {
                        if let Some(arg) = arg_values.first() {
                            ctx.add_output(arg.as_string());
                        }
                    }
                    "DBMS_OUTPUT.PUT" => {
                        if let Some(arg) = arg_values.first() {
                            ctx.append_output(&arg.as_string());
                        }
                    }
                    "RAISE_APPLICATION_ERROR" => {
                        let error_code = arg_values.get(0)
                            .and_then(|v| v.as_integer().ok())
                            .unwrap_or(-20000);
                        let error_msg = arg_values.get(1)
                            .map(|v| v.as_string())
                            .unwrap_or_else(|| "Application error".to_string());
                        return Err(DbError::Runtime(format!("ORA{}: {}", error_code, error_msg)));
                    }
                    _ => {
                        // Store the call for later execution by the procedure manager
                        ctx.record_procedure_call(&name, arg_values);

                        if ctx.is_debug() {
                            ctx.add_output(format!("CALL {}(...)", name));
                        }
                    }
                }
            }

            Statement::Null => {
                // No operation
            }

            Statement::SelectInto { columns, into_vars, from, where_clause } => {
                // Integrate with SQL executor
                // Build and execute the SELECT statement
                let where_str = if let Some(ref wc) = where_clause {
                    format!(" WHERE {:?}", wc)
                } else {
                    String::new()
                };

                if ctx.is_debug() {
                    ctx.add_output(format!(
                        "SELECT {} INTO {:?} FROM {}{}",
                        columns.join(", "),
                        into_vars,
                        from,
                        where_str
                    ));
                }

                // Execute the query and fetch one row
                // In a full implementation, this would call the SQL executor
                // and fetch the actual values

                // For demonstration, handle some special cases
                if columns.len() == into_vars.len() {
                    // Check if any column is a simple expression we can evaluate
                    for (i, col) in columns.iter().enumerate() {
                        if let Some(var_name) = into_vars.get(i) {
                            // Try to evaluate as expression (e.g., "COUNT(*)" or literal)
                            let value = if col.to_uppercase().starts_with("COUNT") {
                                RuntimeValue::Integer(0) // Placeholder
                            } else if col.to_uppercase().starts_with("SUM") {
                                RuntimeValue::Float(0.0) // Placeholder
                            } else if col.to_uppercase() == "SYSDATE" || col.to_uppercase() == "CURRENT_DATE" {
                                RuntimeValue::Date(chrono::Utc::now().format("%Y-%m-%d").to_string())
                            } else if col.to_uppercase() == "SYSTIMESTAMP" || col.to_uppercase() == "CURRENT_TIMESTAMP" {
                                RuntimeValue::Timestamp(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())
                            } else {
                                // Default to NULL for unknown columns
                                RuntimeValue::Null
                            };
                            ctx.set_variable(var_name.clone(), value);
                        }
                    }
                } else {
                    // Mismatched column count - set all to NULL
                    for var in into_vars {
                        ctx.set_variable(var.clone(), RuntimeValue::Null);
                    }
                }
            }

            Statement::Insert { table, columns, values } => {
                // Integrate with SQL executor
                // Build and execute the INSERT statement
                let mut evaluated_values = Vec::new();
                for val_expr in values {
                    let val = self.evaluate_expression(ctx, val_expr)?;
                    evaluated_values.push(val);
                }

                if ctx.is_debug() {
                    ctx.add_output(format!(
                        "INSERT INTO {} ({}) VALUES ({:?})",
                        table,
                        columns.join(", "),
                        evaluated_values
                    ));
                }

                // Record the DML operation and increment rows affected
                ctx.record_dml_operation("INSERT", table, 1);
                ctx.increment_rows_affected(1);
            }

            Statement::Update { table, assignments, where_clause } => {
                // Integrate with SQL executor
                // Build and execute the UPDATE statement
                let mut set_clauses = Vec::new();
                for (col, val_expr) in assignments {
                    let val = self.evaluate_expression(ctx, val_expr)?;
                    set_clauses.push(format!("{} = {:?}", col, val));
                }

                let where_str = if let Some(ref wc) = where_clause {
                    format!(" WHERE {:?}", wc)
                } else {
                    String::new()
                };

                if ctx.is_debug() {
                    ctx.add_output(format!(
                        "UPDATE {} SET {}{}",
                        table,
                        set_clauses.join(", "),
                        where_str
                    ));
                }

                // Record the DML operation - in production would return actual rows affected
                let rows = 1; // Placeholder - would be actual count from executor
                ctx.record_dml_operation("UPDATE", table, rows);
                ctx.increment_rows_affected(rows);
            }

            Statement::Delete { table, where_clause } => {
                // Integrate with SQL executor
                let where_str = if let Some(ref wc) = where_clause {
                    format!(" WHERE {:?}", wc)
                } else {
                    String::new()
                };

                if ctx.is_debug() {
                    ctx.add_output(format!(
                        "DELETE FROM {}{}",
                        table,
                        where_str
                    ));
                }

                // Record the DML operation - in production would return actual rows affected
                let rows = 1; // Placeholder - would be actual count from executor
                ctx.record_dml_operation("DELETE", table, rows);
                ctx.increment_rows_affected(rows);
            }

            Statement::OpenCursor { cursor, arguments } => {
                // Implement cursor opening
                // Evaluate cursor arguments
                let mut arg_values = Vec::new();
                for arg in arguments {
                    arg_values.push(self.evaluate_expression(ctx, arg)?);
                }

                // Open the cursor in the context with empty result set
                // In a full implementation, this would execute the cursor query with the arguments
                ctx.open_cursor(cursor, Vec::new())?;

                if ctx.is_debug() {
                    ctx.add_output(format!("OPEN cursor {}", cursor));
                }
            }

            Statement::FetchCursor { cursor, into_vars } => {
                // Implement cursor fetching
                // Fetch the next row from the cursor
                let row = ctx.fetch_cursor(cursor)?;

                if let Some(row_data) = row {
                    // Bind fetched values to variables
                    // Since row_data is a HashMap, we need to iterate over its values
                    let values: Vec<RuntimeValue> = row_data.values().cloned().collect();
                    for (i, var_name) in into_vars.iter().enumerate() {
                        let value = values.get(i).cloned().unwrap_or(RuntimeValue::Null);
                        ctx.set_variable(var_name.clone(), value);
                    }
                } else {
                    // No more rows - cursor is exhausted
                    // Set cursor %NOTFOUND to true
                    ctx.set_cursor_not_found(true);

                    // Set variables to NULL
                    for var_name in into_vars {
                        ctx.set_variable(var_name.clone(), RuntimeValue::Null);
                    }
                }

                if ctx.is_debug() {
                    ctx.add_output(format!("FETCH {} INTO {:?}", cursor, into_vars));
                }
            }

            Statement::CloseCursor { cursor } => {
                // Implement cursor closing
                ctx.close_cursor(cursor)?;

                if ctx.is_debug() {
                    ctx.add_output(format!("CLOSE cursor {}", cursor));
                }
            }

            Statement::Case { selector, when_clauses, else_clause } => {
                let selector_val = if let Some(sel) = selector {
                    Some(self.evaluate_expression(ctx, sel)?)
                } else {
                    None
                };

                let mut executed = false;
                for (when_expr, when_stmts) in when_clauses {
                    let should_execute = if let Some(ref sel_val) = selector_val {
                        let when_val = self.evaluate_expression(ctx, when_expr)?;
                        self.values_equal(sel_val, &when_val)?
                    } else {
                        self.evaluate_expression(ctx, when_expr)?.as_boolean()?
                    };

                    if should_execute {
                        for s in when_stmts {
                            self.execute_statement(ctx, s)?;
                            if ctx.has_exception() || ctx.get_return().is_some() {
                                return Ok(());
                            }
                        }
                        executed = true;
                        break;
                    }
                }

                if !executed {
                    if let Some(else_stmts) = else_clause {
                        for s in else_stmts {
                            self.execute_statement(ctx, s)?;
                            if ctx.has_exception() || ctx.get_return().is_some() {
                                return Ok(());
                            }
                        }
                    }
                }
            }

            Statement::ForCursor { record, cursor, statements } => {
                // Implement cursor FOR loops
                // Implicitly open the cursor if not already open
                if !ctx.is_cursor_open(cursor) {
                    ctx.open_cursor(cursor, Vec::new())?;
                }

                // Iterate through all rows
                loop {
                    // Fetch next row
                    let row = ctx.fetch_cursor(cursor)?;

                    if let Some(row_data) = row {
                        // Create a record variable with the row data
                        // The row_data is already a HashMap<String, RuntimeValue>, so use it directly
                        ctx.set_variable(record.clone(), RuntimeValue::Record(row_data));

                        // Execute loop body
                        for stmt in statements {
                            self.execute_statement(ctx, stmt)?;

                            if ctx.should_exit_loop() {
                                ctx.clear_exit_loop();
                                // Close cursor and exit
                                ctx.close_cursor(cursor)?;
                                return Ok(());
                            }

                            if ctx.should_continue_loop() {
                                ctx.clear_continue_loop();
                                break; // Continue to next row
                            }

                            if ctx.has_exception() || ctx.get_return().is_some() {
                                ctx.close_cursor(cursor)?;
                                return Ok(());
                            }
                        }
                    } else {
                        // No more rows - exit loop
                        break;
                    }
                }

                // Implicitly close the cursor
                ctx.close_cursor(cursor)?;
            }
        }

        Ok(())
    }

    // Evaluate an expression
    fn evaluate_expression(&self, ctx: &ExecutionContext, expr: &Expression) -> Result<RuntimeValue> {
        match expr {
            Expression::Literal(lit) => Ok(match lit {
                LiteralValue::Integer(v) => RuntimeValue::Integer(*v),
                LiteralValue::Float(v) => RuntimeValue::Float(*v),
                LiteralValue::String(s) => RuntimeValue::String(s.clone()),
                LiteralValue::Boolean(b) => RuntimeValue::Boolean(*b),
                LiteralValue::Null => RuntimeValue::Null,
                LiteralValue::Date(d) => RuntimeValue::Date(d.clone()),
                LiteralValue::Timestamp(t) => RuntimeValue::Timestamp(t.clone()),
            }),

            Expression::Variable(name) => ctx.get_variable(name),

            Expression::BinaryOp { left, op, right } => {
                let left_val = self.evaluate_expression(ctx, left)?;
                let right_val = self.evaluate_expression(ctx, right)?;
                self.apply_binary_op(&left_val, op, &right_val)
            }

            Expression::UnaryOp { op, operand } => {
                let val = self.evaluate_expression(ctx, operand)?;
                self.apply_unary_op(op, &val)
            }

            Expression::FunctionCall { name, arguments } => {
                let mut arg_values = Vec::new();
                for arg in arguments {
                    arg_values.push(self.evaluate_expression(ctx, arg)?);
                }
                self.call_function(name, arg_values)
            }

            Expression::FieldAccess { record, field } => {
                let record_val = ctx.get_variable(record)?;
                if let RuntimeValue::Record(fields) = record_val {
                    fields.get(field)
                        .cloned()
                        .ok_or_else(|| DbError::Runtime(format!("Field '{}' not found", field)))
                } else {
                    Err(DbError::Runtime(format!("'{}' is not a record", record)))
                }
            }

            _ => Err(DbError::Runtime("Expression type not yet implemented".to_string())),
        }
    }

    // Apply binary operator
    fn apply_binary_op(&self, left: &RuntimeValue, op: &BinaryOperator, right: &RuntimeValue) -> Result<RuntimeValue> {
        match op {
            BinaryOperator::Add => {
                if let (RuntimeValue::Integer(l), RuntimeValue::Integer(r)) = (left, right) {
                    Ok(RuntimeValue::Integer(l + r))
                } else {
                    let l = left.as_float()?;
                    let r = right.as_float()?;
                    Ok(RuntimeValue::Float(l + r))
                }
            }

            BinaryOperator::Subtract => {
                if let (RuntimeValue::Integer(l), RuntimeValue::Integer(r)) = (left, right) {
                    Ok(RuntimeValue::Integer(l - r))
                } else {
                    let l = left.as_float()?;
                    let r = right.as_float()?;
                    Ok(RuntimeValue::Float(l - r))
                }
            }

            BinaryOperator::Multiply => {
                if let (RuntimeValue::Integer(l), RuntimeValue::Integer(r)) = (left, right) {
                    Ok(RuntimeValue::Integer(l * r))
                } else {
                    let l = left.as_float()?;
                    let r = right.as_float()?;
                    Ok(RuntimeValue::Float(l * r))
                }
            }

            BinaryOperator::Divide => {
                let l = left.as_float()?;
                let r = right.as_float()?;
                if r == 0.0 {
                    Err(DbError::Runtime("Division by zero".to_string()))
                } else {
                    Ok(RuntimeValue::Float(l / r))
                }
            }

            BinaryOperator::Modulo => {
                let l = left.as_integer()?;
                let r = right.as_integer()?;
                if r == 0 {
                    Err(DbError::Runtime("Modulo by zero".to_string()))
                } else {
                    Ok(RuntimeValue::Integer(l % r))
                }
            }

            BinaryOperator::Equal => Ok(RuntimeValue::Boolean(self.values_equal(left, right)?)),

            BinaryOperator::NotEqual => Ok(RuntimeValue::Boolean(!self.values_equal(left, right)?)),

            BinaryOperator::LessThan => {
                if let (RuntimeValue::Integer(l), RuntimeValue::Integer(r)) = (left, right) {
                    Ok(RuntimeValue::Boolean(l < r))
                } else {
                    let l = left.as_float()?;
                    let r = right.as_float()?;
                    Ok(RuntimeValue::Boolean(l < r))
                }
            }

            BinaryOperator::LessThanOrEqual => {
                if let (RuntimeValue::Integer(l), RuntimeValue::Integer(r)) = (left, right) {
                    Ok(RuntimeValue::Boolean(l <= r))
                } else {
                    let l = left.as_float()?;
                    let r = right.as_float()?;
                    Ok(RuntimeValue::Boolean(l <= r))
                }
            }

            BinaryOperator::GreaterThan => {
                if let (RuntimeValue::Integer(l), RuntimeValue::Integer(r)) = (left, right) {
                    Ok(RuntimeValue::Boolean(l > r))
                } else {
                    let l = left.as_float()?;
                    let r = right.as_float()?;
                    Ok(RuntimeValue::Boolean(l > r))
                }
            }

            BinaryOperator::GreaterThanOrEqual => {
                if let (RuntimeValue::Integer(l), RuntimeValue::Integer(r)) = (left, right) {
                    Ok(RuntimeValue::Boolean(l >= r))
                } else {
                    let l = left.as_float()?;
                    let r = right.as_float()?;
                    Ok(RuntimeValue::Boolean(l >= r))
                }
            }

            BinaryOperator::And => {
                let l = left.as_boolean()?;
                let r = right.as_boolean()?;
                Ok(RuntimeValue::Boolean(l && r))
            }

            BinaryOperator::Or => {
                let l = left.as_boolean()?;
                let r = right.as_boolean()?;
                Ok(RuntimeValue::Boolean(l || r))
            }

            BinaryOperator::Concat => {
                Ok(RuntimeValue::String(format!("{}{}", left.as_string(), right.as_string())))
            }

            _ => Err(DbError::Runtime(format!("Binary operator {:?} not implemented", op))),
        }
    }

    // Apply unary operator
    fn apply_unary_op(&self, op: &UnaryOperator, val: &RuntimeValue) -> Result<RuntimeValue> {
        match op {
            UnaryOperator::Not => {
                let b = val.as_boolean()?;
                Ok(RuntimeValue::Boolean(!b))
            }

            UnaryOperator::Minus => {
                if let RuntimeValue::Integer(v) = val {
                    Ok(RuntimeValue::Integer(-v))
                } else {
                    let f = val.as_float()?;
                    Ok(RuntimeValue::Float(-f))
                }
            }

            UnaryOperator::Plus => Ok(val.clone()),
        }
    }

    // Check if two values are equal
    fn values_equal(&self, left: &RuntimeValue, right: &RuntimeValue) -> Result<bool> {
        Ok(match (left, right) {
            (RuntimeValue::Null, RuntimeValue::Null) => true,
            (RuntimeValue::Null, _) | (_, RuntimeValue::Null) => false,
            (RuntimeValue::Integer(l), RuntimeValue::Integer(r)) => l == r,
            (RuntimeValue::Float(l), RuntimeValue::Float(r)) => (l - r).abs() < f64::EPSILON,
            (RuntimeValue::String(l), RuntimeValue::String(r)) => l == r,
            (RuntimeValue::Boolean(l), RuntimeValue::Boolean(r)) => l == r,
            _ => false,
        })
    }

    // Call a built-in function
    fn call_function(&self, name: &str, args: Vec<RuntimeValue>) -> Result<RuntimeValue> {
        match name.to_uppercase().as_str() {
            "UPPER" => {
                if args.len() != 1 {
                    return Err(DbError::Runtime("UPPER expects 1 argument".to_string()));
                }
                Ok(RuntimeValue::String(args[0].as_string().to_uppercase()))
            }

            "LOWER" => {
                if args.len() != 1 {
                    return Err(DbError::Runtime("LOWER expects 1 argument".to_string()));
                }
                Ok(RuntimeValue::String(args[0].as_string().to_lowercase()))
            }

            "LENGTH" => {
                if args.len() != 1 {
                    return Err(DbError::Runtime("LENGTH expects 1 argument".to_string()));
                }
                Ok(RuntimeValue::Integer(args[0].as_string().len() as i64))
            }

            "SUBSTR" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(DbError::Runtime("SUBSTR expects 2 or 3 arguments".to_string()));
                }
                let s = args[0].as_string();
                let start = (args[1].as_integer()? - 1).max(0) as usize;
                let result = if args.len() == 3 {
                    let len = args[2].as_integer()?.max(0) as usize;
                    s.chars().skip(start).take(len).collect()
                } else {
                    s.chars().skip(start).collect()
                };
                Ok(RuntimeValue::String(result))
            }

            "ABS" => {
                if args.len() != 1 {
                    return Err(DbError::Runtime("ABS expects 1 argument".to_string()));
                }
                if let RuntimeValue::Integer(v) = &args[0] {
                    Ok(RuntimeValue::Integer(v.abs()))
                } else {
                    let v = args[0].as_float()?;
                    Ok(RuntimeValue::Float(v.abs()))
                }
            }

            "ROUND" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(DbError::Runtime("ROUND expects 1 or 2 arguments".to_string()));
                }
                let v = args[0].as_float()?;
                let decimals = if args.len() == 2 {
                    args[1].as_integer()? as i32
                } else {
                    0
                };
                let multiplier = 10_f64.powi(decimals);
                Ok(RuntimeValue::Float((v * multiplier).round() / multiplier))
            }

            _ => Err(DbError::Runtime(format!("Unknown function: {}", name))),
        }
    }

    // Handle exception
    fn handle_exception(
        &self,
        ctx: &mut ExecutionContext,
        exception_name: &str,
        handlers: &[ExceptionHandler],
    ) -> Result<()> {
        for handler in handlers {
            let matches = match &handler.exception_type {
                ExceptionType::Others => true,
                ExceptionType::UserDefined(name) => name == exception_name,
                ExceptionType::NoDataFound => exception_name == "NO_DATA_FOUND",
                ExceptionType::TooManyRows => exception_name == "TOO_MANY_ROWS",
                ExceptionType::ZeroDivide => exception_name == "ZERO_DIVIDE",
                ExceptionType::ValueError => exception_name == "VALUE_ERROR",
                ExceptionType::InvalidCursor => exception_name == "INVALID_CURSOR",
                ExceptionType::DupValOnIndex => exception_name == "DUP_VAL_ON_INDEX",
            };

            if matches {
                ctx.clear_exception();
                for stmt in &handler.statements {
                    self.execute_statement(ctx, stmt)?;
                }
                return Ok(());
            }
        }

        // Exception not handled - propagate it
        Err(DbError::Runtime(format!("Unhandled exception: {}", exception_name)))
    }
}

impl Default for RuntimeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// Result of PL/SQL execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub return_value: Option<RuntimeValue>,
    pub output_params: HashMap<String, RuntimeValue>,
    pub rows_affected: usize,
    pub output_lines: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::procedures::parser::PlSqlParser;

    #[test]
    fn test_execute_simple_assignment() -> Result<()> {
        let mut parser = PlSqlParser::new();
        let source = r#"
            DECLARE
                x INTEGER := 10;
                y INTEGER;
            BEGIN
                y := x + 5;
            END;
        "#;

        let block = parser.parse(source)?;
        let executor = RuntimeExecutor::new();
        let result = executor.execute(&block)?;

        assert!(result.return_value.is_none());

        Ok(())
    }

    #[test]
    fn test_execute_if_statement() -> Result<()> {
        let mut parser = PlSqlParser::new();
        let source = r#"
            DECLARE
                x INTEGER := 15;
                result VARCHAR2(10);
            BEGIN
                IF x > 20 THEN
                    result := 'high';
                ELSIF x > 10 THEN
                    result := 'medium';
                ELSE
                    result := 'low';
                END IF;
            END;
        "#;

        let block = parser.parse(source)?;
        let executor = RuntimeExecutor::new();
        let result = executor.execute(&block)?;

        assert!(result.return_value.is_none());

        Ok(())
    }

    #[test]
    fn test_execute_loop() -> Result<()> {
        let mut parser = PlSqlParser::new();
        let source = r#"
            DECLARE
                counter INTEGER := 0;
            BEGIN
                LOOP
                    counter := counter + 1;
                    EXIT WHEN counter >= 5;
                END LOOP;
            END;
        "#;

        let block = parser.parse(source)?;
        let executor = RuntimeExecutor::new();
        let result = executor.execute(&block)?;

        assert!(result.return_value.is_none());

        Ok(())
    }
}
