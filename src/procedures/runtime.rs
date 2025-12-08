/// PL/SQL Runtime Execution Engine
///
/// This module provides the runtime environment for executing PL/SQL blocks,
/// managing execution context, variable bindings, and control flow.

use crate::{Result, DbError};
use crate::procedures::parser::{
    PlSqlBlock, Statement, Expression, LiteralValue, BinaryOperator, UnaryOperator,
    Declaration, ExceptionHandler, ExceptionType,
};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Runtime value types
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
    /// Convert to integer if possible
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

    /// Convert to float if possible
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

    /// Convert to string
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

    /// Convert to boolean
    #[inline]
    pub fn as_boolean(&self) -> Result<bool> {
        match self {
            RuntimeValue::Boolean(b) => Ok(*b),
            RuntimeValue::Integer(v) => Ok(*v != 0),
            RuntimeValue::Null => Ok(false),
            _ => Err(DbError::Runtime(format!("Cannot convert {:?} to boolean", self))),
        }
    }

    /// Check if value is null
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, RuntimeValue::Null)
    }
}

/// Cursor state for runtime
#[derive(Debug, Clone, PartialEq)]
pub struct CursorState {
    pub name: String,
    pub query: String,
    pub is_open: bool,
    pub current_row: usize,
    pub rows: Vec<HashMap<String, RuntimeValue>>,
}

/// Execution context for a PL/SQL block
pub struct ExecutionContext {
    /// Variable bindings (name -> value)
    variables: HashMap<String, RuntimeValue>,
    /// Output parameters
    output_params: HashMap<String, RuntimeValue>,
    /// Parent context (for nested blocks)
    parent: Option<Arc<RwLock<ExecutionContext>>>,
    /// Return value
    return_value: Option<RuntimeValue>,
    /// Exception state
    exception_raised: Option<String>,
    /// Loop control flags
    exit_loop: bool,
    continue_loop: bool,
    /// Transaction savepoints
    savepoints: Vec<String>,
    /// Cursors
    cursors: HashMap<String, CursorState>,
    /// Debug mode
    debug: bool,
    /// Output buffer (for DBMS_OUTPUT)
    output_buffer: Vec<String>,
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
        }
    }

    /// Create a child context
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
        }
    }

    /// Set a variable value
    #[inline]
    pub fn set_variable(&mut self, name: String, value: RuntimeValue) {
        self.variables.insert(name, value);
    }

    /// Get a variable value
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

    /// Set an output parameter
    pub fn set_output(&mut self, name: String, value: RuntimeValue) {
        self.output_params.insert(name, value);
    }

    /// Get all output parameters
    pub fn get_outputs(&self) -> &HashMap<String, RuntimeValue> {
        &self.output_params
    }

    /// Set return value
    pub fn set_return(&mut self, value: RuntimeValue) {
        self.return_value = Some(value);
    }

    /// Get return value
    pub fn get_return(&self) -> Option<&RuntimeValue> {
        self.return_value.as_ref()
    }

    /// Raise an exception
    pub fn raise_exception(&mut self, exception: String) {
        self.exception_raised = Some(exception);
    }

    /// Check if exception is raised
    pub fn has_exception(&self) -> bool {
        self.exception_raised.is_some()
    }

    /// Get exception name
    pub fn get_exception(&self) -> Option<&String> {
        self.exception_raised.as_ref()
    }

    /// Clear exception
    pub fn clear_exception(&mut self) {
        self.exception_raised = None;
    }

    /// Set exit loop flag
    pub fn set_exit_loop(&mut self) {
        self.exit_loop = true;
    }

    /// Check exit loop flag
    pub fn should_exit_loop(&self) -> bool {
        self.exit_loop
    }

    /// Clear exit loop flag
    pub fn clear_exit_loop(&mut self) {
        self.exit_loop = false;
    }

    /// Set continue loop flag
    pub fn set_continue_loop(&mut self) {
        self.continue_loop = true;
    }

    /// Check continue loop flag
    pub fn should_continue_loop(&self) -> bool {
        self.continue_loop
    }

    /// Clear continue loop flag
    pub fn clear_continue_loop(&mut self) {
        self.continue_loop = false;
    }

    /// Add output line (for DBMS_OUTPUT)
    pub fn add_output(&mut self, line: String) {
        self.output_buffer.push(line);
    }

    /// Get output buffer
    pub fn get_output_buffer(&self) -> &[String] {
        &self.output_buffer
    }

    /// Enable debug mode
    pub fn enable_debug(&mut self) {
        self.debug = true;
    }

    /// Check if debug mode is enabled
    pub fn is_debug(&self) -> bool {
        self.debug
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// PL/SQL Runtime Executor
pub struct RuntimeExecutor {
    /// Global execution context
    context: Arc<RwLock<ExecutionContext>>,
}

impl RuntimeExecutor {
    pub fn new() -> Self {
        Self {
            context: Arc::new(RwLock::new(ExecutionContext::new())),
        }
    }

    /// Execute a PL/SQL block
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
            rows_affected: 0, // TODO: Track actual rows affected
            output_lines: ctx.get_output_buffer().to_vec(),
        };

        Ok(result)
    }

    /// Execute a declaration
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

    /// Execute a statement
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
                // TODO: Integrate with transaction manager
                if ctx.is_debug() {
                    ctx.add_output("COMMIT executed".to_string());
                }
            }

            Statement::Rollback { to_savepoint } => {
                // TODO: Integrate with transaction manager
                if ctx.is_debug() {
                    if let Some(sp) = to_savepoint {
                        ctx.add_output(format!("ROLLBACK TO SAVEPOINT {}", sp));
                    } else {
                        ctx.add_output("ROLLBACK executed".to_string());
                    }
                }
            }

            Statement::Savepoint { name } => {
                // TODO: Integrate with transaction manager
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

                // TODO: Integrate with procedure manager to call other procedures
                if ctx.is_debug() {
                    ctx.add_output(format!("CALL {}({:?})", name, arg_values));
                }
            }

            Statement::Null => {
                // No operation
            }

            Statement::SelectInto { columns, into_vars, from, where_clause } => {
                // TODO: Integrate with SQL executor
                if ctx.is_debug() {
                    ctx.add_output(format!(
                        "SELECT {} INTO {:?} FROM {} WHERE {:?}",
                        columns.join(", "),
                        into_vars,
                        from,
                        where_clause
                    ));
                }

                // For now, set variables to NULL
                for var in into_vars {
                    ctx.set_variable(var.clone(), RuntimeValue::Null);
                }
            }

            Statement::Insert { table, columns, values } => {
                // TODO: Integrate with SQL executor
                if ctx.is_debug() {
                    ctx.add_output(format!(
                        "INSERT INTO {} ({}) VALUES",
                        table,
                        columns.join(", ")
                    ));
                }
            }

            Statement::Update { table, assignments, where_clause } => {
                // TODO: Integrate with SQL executor
                if ctx.is_debug() {
                    ctx.add_output(format!(
                        "UPDATE {} SET ... WHERE {:?}",
                        table,
                        where_clause
                    ));
                }
            }

            Statement::Delete { table, where_clause } => {
                // TODO: Integrate with SQL executor
                if ctx.is_debug() {
                    ctx.add_output(format!(
                        "DELETE FROM {} WHERE {:?}",
                        table,
                        where_clause
                    ));
                }
            }

            Statement::OpenCursor { cursor, arguments } => {
                // TODO: Implement cursor opening
                if ctx.is_debug() {
                    ctx.add_output(format!("OPEN cursor {}", cursor));
                }
            }

            Statement::FetchCursor { cursor, into_vars } => {
                // TODO: Implement cursor fetching
                if ctx.is_debug() {
                    ctx.add_output(format!("FETCH {} INTO {:?}", cursor, into_vars));
                }
            }

            Statement::CloseCursor { cursor } => {
                // TODO: Implement cursor closing
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

            Statement::ForCursor { .. } => {
                // TODO: Implement cursor FOR loops
                return Err(DbError::Runtime("Cursor FOR loops not yet implemented".to_string()));
            }
        }

        Ok(())
    }

    /// Evaluate an expression
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

    /// Apply binary operator
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

    /// Apply unary operator
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

    /// Check if two values are equal
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

    /// Call a built-in function
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

    /// Handle exception
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

/// Result of PL/SQL execution
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
