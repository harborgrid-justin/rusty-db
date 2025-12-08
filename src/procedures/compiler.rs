/// PL/SQL Compiler with Dependency Tracking
///
/// This module provides compilation, validation, semantic analysis, and dependency
/// tracking for stored procedures, functions, packages, and triggers.

use crate::{Result, DbError};
use crate::procedures::parser::{PlSqlParser, PlSqlBlock, Statement, Expression, PlSqlType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::Arc;
use parking_lot::RwLock;

/// Compilation result
#[derive(Debug, Clone)]
pub struct CompilationResult {
    pub success: bool,
    pub errors: Vec<CompilationError>,
    pub warnings: Vec<CompilationWarning>,
    pub dependencies: HashSet<String>,
}

impl CompilationResult {
    pub fn new() -> Self {
        Self {
            success: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            dependencies: HashSet::new(),
        }
    }

    pub fn add_error(&mut self, error: CompilationError) {
        self.success = false;
        self.errors.push(error);
    }

    pub fn add_warning(&mut selfing: CompilationWarning) {
        self.warnings.push(warning);
    }

    pub fn add_dependency(&mut self, dep: String) {
        self.dependencies.insert(dep);
    }
}

impl Default for CompilationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Compilation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationError {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub error_type: ErrorType,
}

/// Error types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorType {
    SyntaxError,
    SemanticError,
    TypeMismatch,
    UndefinedVariable,
    UndefinedFunction,
    InvalidArgument,
    CircularDependency,
}

/// Compilation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationWarning {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub warning_type: WarningType,
}

/// Warning types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WarningType {
    UnusedVariable,
    UnreachableCode,
    ImplicitConversion,
    DeprecatedFeature,
}

/// Object type for dependency tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ObjectType {
    Procedure,
    Function,
    Package,
    Trigger,
    Table,
    View,
}

/// Database object metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseObject {
    pub name: String,
    pub object_type: ObjectType,
    pub status: CompilationStatus,
    pub dependencies: HashSet<String>,
    pub dependents: HashSet<String>,
    pub last_compiled: String,
}

/// Compilation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompilationStatus {
    Valid,
    Invalid,
    NeedsRecompilation,
}

/// PL/SQL Compiler
pub struct PlSqlCompiler {
    parser: PlSqlParser,
    objects: Arc<RwLock<HashMap<String, DatabaseObject>>>,
    symbol_table: Arc<RwLock<SymbolTable>>,
}

impl PlSqlCompiler {
    pub fn new() -> Self {
        Self {
            parser: PlSqlParser::new(),
            objects: Arc::new(RwLock::new(HashMap::new())),
            symbol_table: Arc::new(RwLock::new(SymbolTable::new())),
        }
    }

    /// Compile a PL/SQL block
    pub fn compile(&mut self, source: &str) -> Result<CompilationResult> {
        let mut result = CompilationResult::new();

        // Phase 1: Syntax analysis (parsing)
        let block = match self.parser.parse(source) {
            Ok(b) => b,
            Err(e) => {
                result.add_error(CompilationError {
                    line: 0,
                    column: 0,
                    message: e.to_string(),
                    error_type: ErrorType::SyntaxError,
                });
                return Ok(result);
            }
        };

        // Phase 2: Semantic analysis
        self.analyze_semantics(&block, &mut result)?;

        // Phase 3: Type checking
        self.check_types(&block, &mut result)?;

        // Phase 4: Dependency analysis
        self.analyze_dependencies(&block, &mut result)?;

        Ok(result)
    }

    /// Perform semantic analysis
    fn analyze_semantics(&self, block: &PlSqlBlock, result: &mut CompilationResult) -> Result<()> {
        let mut symbol_table = self.symbol_table.write();

        // Check declarations
        for decl in &block.declarations {
            if symbol_table.is_defined(&decl.name) {
                result.add_error(CompilationError {
                    line: 0,
                    column: 0,
                    message: format!("Variable '{}' is already declared", decl.name),
                    error_type: ErrorType::SemanticError,
                });
            } else {
                symbol_table.add_variable(decl.name.clone(), decl.data_type.clone());
            }
        }

        // Check statements
        for stmt in &block.statements {
            self.analyze_statement(stmt, &mut symbol_table, result)?;
        }

        Ok(())
    }

    /// Analyze a statement
    fn analyze_statement(
        &self,
        stmt: &Statement,
        symbol_table: &mut SymbolTable,
        result: &mut CompilationResult,
    ) -> Result<()> {
        match stmt {
            Statement::Assignment { target, value } => {
                // Check if variable is defined
                if !symbol_table.is_defined(target) {
                    result.add_error(CompilationError {
                        line: 0,
                        column: 0,
                        message: format!("Variable '{}' is not declared", target),
                        error_type: ErrorType::UndefinedVariable,
                    });
                }

                // Analyze value expression
                self.analyze_expression(value, symbol_table, result)?;
            }

            Statement::If { condition, then_block, elsif_blocks, else_block } => {
                self.analyze_expression(condition, symbol_table, result)?;

                for stmt in then_block {
                    self.analyze_statement(stmt, symbol_table, result)?;
                }

                for (elsif_cond, elsif_stmts) in elsif_blocks {
                    self.analyze_expression(elsif_cond, symbol_table, result)?;
                    for stmt in elsif_stmts {
                        self.analyze_statement(stmt, symbol_table, result)?;
                    }
                }

                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.analyze_statement(stmt, symbol_table, result)?;
                    }
                }
            }

            Statement::Loop { statements } => {
                for stmt in statements {
                    self.analyze_statement(stmt, symbol_table, result)?;
                }
            }

            Statement::While { condition, statements } => {
                self.analyze_expression(condition, symbol_table, result)?;
                for stmt in statements {
                    self.analyze_statement(stmt, symbol_table, result)?;
                }
            }

            Statement::ForNumeric { iterator, start, end, statements, .. } => {
                // Add iterator to symbol table temporarily
                symbol_table.add_variable(iterator.clone(), PlSqlType::Integer);

                self.analyze_expression(start, symbol_table, result)?;
                self.analyze_expression(end, symbol_table, result)?;

                for stmt in statements {
                    self.analyze_statement(stmt, symbol_table, result)?;
                }

                // Remove iterator from symbol table
                symbol_table.remove_variable(iterator);
            }

            Statement::Call { name, arguments } => {
                // Check if procedure/function exists
                result.add_dependency(name.clone());

                // Analyze arguments
                for arg in arguments {
                    self.analyze_expression(arg, symbol_table, result)?;
                }
            }

            Statement::SelectInto { columns, into_vars, from, where_clause } => {
                // Add table dependency
                result.add_dependency(from.clone());

                // Check that INTO variables are declared
                for var in into_vars {
                    if !symbol_table.is_defined(var) {
                        result.add_error(CompilationError {
                            line: 0,
                            column: 0,
                            message: format!("Variable '{}' is not declared", var),
                            error_type: ErrorType::UndefinedVariable,
                        });
                    }
                }

                // Analyze WHERE clause if present
                if let Some(where_expr) = where_clause {
                    self.analyze_expression(where_expr, symbol_table, result)?;
                }
            }

            Statement::Case { selector, when_clauses, else_clause } => {
                if let Some(sel) = selector {
                    self.analyze_expression(sel, symbol_table, result)?;
                }

                for (when_expr, when_stmts) in when_clauses {
                    self.analyze_expression(when_expr, symbol_table, result)?;
                    for stmt in when_stmts {
                        self.analyze_statement(stmt, symbol_table, result)?;
                    }
                }

                if let Some(else_stmts) = else_clause {
                    for stmt in else_stmts {
                        self.analyze_statement(stmt, symbol_table, result)?;
                    }
                }
            }

            _ => {
                // Other statements - basic validation
            }
        }

        Ok(())
    }

    /// Analyze an expression
    fn analyze_expression(
        &self,
        expr: &Expression,
        symbol_table: &SymbolTable,
        result: &mut CompilationResult,
    ) -> Result<()> {
        match expr {
            Expression::Variable(name) => {
                if !symbol_table.is_defined(name) {
                    result.add_error(CompilationError {
                        line: 0,
                        column: 0,
                        message: format!("Variable '{}' is not declared", name),
                        error_type: ErrorType::UndefinedVariable,
                    });
                }
            }

            Expression::BinaryOp { left, right, .. } => {
                self.analyze_expression(left, symbol_table, result)?;
                self.analyze_expression(right, symbol_table, result)?;
            }

            Expression::UnaryOp { operand, .. } => {
                self.analyze_expression(operand, symbol_table, result)?;
            }

            Expression::FunctionCall { name, arguments } => {
                // Add function dependency
                result.add_dependency(name.clone());

                // Analyze arguments
                for arg in arguments {
                    self.analyze_expression(arg, symbol_table, result)?;
                }
            }

            Expression::FieldAccess { record, .. } => {
                if !symbol_table.is_defined(record) {
                    result.add_error(CompilationError {
                        line: 0,
                        column: 0,
                        message: format!("Record '{}' is not declared", record),
                        error_type: ErrorType::UndefinedVariable,
                    });
                }
            }

            _ => {
                // Literals and other expressions don't need validation
            }
        }

        Ok(())
    }

    /// Type checking
    fn check_types(&self, block: &PlSqlBlock, result: &mut CompilationResult) -> Result<()> {
        let symbol_table = self.symbol_table.read();

        // TODO: Implement comprehensive type checking
        // For now, perform basic checks

        for decl in &block.declarations {
            if let Some(ref init_val) = decl.initial_value {
                // Check that initial value type matches declaration type
                // This would require type inference for expressions
            }
        }

        Ok(())
    }

    /// Analyze dependencies
    fn analyze_dependencies(&self, block: &PlSqlBlock, result: &mut CompilationResult) -> Result<()> {
        // Dependencies were collected during semantic analysis
        // Here we could perform additional dependency-related checks

        // Check for circular dependencies
        self.check_circular_dependencies(result)?;

        Ok(())
    }

    /// Check for circular dependencies
    fn check_circular_dependencies(&self, result: &CompilationResult) -> Result<()> {
        let objects = self.objects.read();

        for dep in &result.dependencies {
            if let Some(obj) = objects.get(dep) {
                if self.has_circular_dependency(dep, &obj.dependencies, &objects) {
                    let mut err_result = result.clone();
                    err_result.add_error(CompilationError {
                        line: 0,
                        column: 0,
                        message: format!("Circular dependency detected involving '{}'", dep),
                        error_type: ErrorType::CircularDependency,
                    });
                }
            }
        }

        Ok(())
    }

    fn has_circular_dependency(
        &self,
        target: &str,
        deps: &HashSet<String>,
        all_objects: &HashMap<String, DatabaseObject>,
    ) -> bool {
        let mut visited = HashSet::new();
        self.dfs_check_cycle(target, deps, all_objects, &mut visited)
    }

    fn dfs_check_cycle(
        &self,
        target: &str,
        current_deps: &HashSet<String>,
        all_objects: &HashMap<String, DatabaseObject>,
        visited: &mut HashSet<String>,
    ) -> bool {
        for dep in current_deps {
            if dep == target {
                return true;
            }

            if visited.contains(dep) {
                continue;
            }

            visited.insert(dep.clone());

            if let Some(obj) = all_objects.get(dep) {
                if self.dfs_check_cycle(target, &obj.dependencies, all_objects, visited) {
                    return true;
                }
            }

            visited.remove(dep);
        }

        false
    }

    /// Register a database object
    pub fn register_object(&self, object: DatabaseObject) {
        let mut objects = self.objects.write();
        objects.insert(object.name.clone(), object);
    }

    /// Get objects that depend on a given object
    pub fn get_dependents(&self, object_name: &str) -> Vec<String> {
        let objects = self.objects.read();

        if let Some(obj) = objects.get(object_name) {
            obj.dependents.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Mark objects for recompilation
    pub fn mark_for_recompilation(&self, object_name: &str) -> Result<Vec<String>> {
        let mut objects = self.objects.write();
        let mut to_recompile = Vec::new();

        // Mark the object itself
        if let Some(obj) = objects.get_mut(object_name) {
            obj.status = CompilationStatus::NeedsRecompilation;
            to_recompile.push(object_name.to_string());

            // Clone dependents to avoid borrowing conflict
            let dependents = obj.dependents.clone();

            // Mark all dependents
            for dependent in dependents {
                to_recompile.push(dependent.clone());
                if let Some(dep_obj) = objects.get_mut(&dependent) {
                    dep_obj.status = CompilationStatus::NeedsRecompilation;
                }
            }
        }

        Ok(to_recompile)
    }

    /// Recompile all invalid objects
    pub fn recompile_invalid(&mut self) -> Result<HashMap<String, CompilationResult>> {
        let objects = self.objects.read();
        let mut results = HashMap::new();

        for (name, obj) in objects.iter() {
            if obj.status == CompilationStatus::Invalid || obj.status == CompilationStatus::NeedsRecompilation {
                // TODO: Get source code for object and recompile
                // For now, create a placeholder result
                let _result = CompilationResult::new();
                results.insert(name.clone(), result);
            }
        }

        Ok(results)
    }
}

impl Default for PlSqlCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Symbol table for variable and type tracking
pub struct SymbolTable {
    variables: HashMap<String, PlSqlType>,
    types: HashMap<String, PlSqlType>,
    constants: HashSet<String>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            types: HashMap::new(),
            constants: HashSet::new(),
        }
    }

    pub fn add_variable(&mut self, name: String, data_type: PlSqlType) {
        self.variables.insert(name, data_type);
    }

    pub fn add_constant(&mut self, name: String, data_type: PlSqlType) {
        self.variables.insert(name.clone(), data_type);
        self.constants.insert(name);
    }

    pub fn remove_variable(&mut self, name: &str) {
        self.variables.remove(name);
        self.constants.remove(name);
    }

    pub fn is_defined(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    pub fn is_constant(&self, name: &str) -> bool {
        self.constants.contains(name)
    }

    pub fn get_type(&self, name: &str) -> Option<&PlSqlType> {
        self.variables.get(name)
    }

    pub fn add_type(&mut self, name: String, definition: PlSqlType) {
        self.types.insert(name, definition);
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency graph
pub struct DependencyGraph {
    graph: Arc<RwLock<HashMap<String<String>>>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a dependency edge
    pub fn add_dependency(&self, from: String, to: String) {
        let mut graph = self.graph.write();
        graph.entry(from)
            .or_insert_with(HashSet::new)
            .insert(to);
    }

    /// Get all dependencies of an object
    pub fn get_dependencies(&self, object: &str) -> HashSet<String> {
        let graph = self.graph.read();
        graph.get(object).cloned().unwrap_or_default()
    }

    /// Get topological order for compilation
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        let graph = self.graph.read();
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut result = Vec::new();

        // Calculate in-degrees
        for (node, deps) in graph.iter() {
            in_degree.entry(node.clone()).or_insert(0);
            for dep in deps {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }

        // Find nodes with in-degree 0
        let mut queue: Vec<String> = in_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(node, _)| node.clone())
            .collect();

        while let Some(node) = queue.pop() {
            result.push(node.clone());

            if let Some(deps) = graph.get(&node) {
                for dep in deps {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(dep.clone());
                        }
                    }
                }
            }
        }

        // Check for cycles
        if result.len() != in_degree.len() {
            return Err(DbError::InvalidInput("Circular dependency detected".to_string()));
        }

        Ok(result)
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple_block() -> Result<()> {
        let mut compiler = PlSqlCompiler::new();

        let _source = r#"
            DECLARE
                x INTEGER := 10;
            BEGIN
                x := x + 5;
            END;
        "#;

        let _result = compiler.compile(source)?;

        assert!(result.success);
        assert_eq!(result.errors.len(), 0);

        Ok(())
    }

    #[test]
    fn test_undefined_variable_error() -> Result<()> {
        let mut compiler = PlSqlCompiler::new();

        let _source = r#"
            BEGIN
                y := x + 5;
            END;
        "#;

        let _result = compiler.compile(source)?;

        assert!(!result.success);
        assert!(result.errors.iter().any(|e| e.error_type == ErrorType::UndefinedVariable));

        Ok(())
    }

    #[test]
    fn test_dependency_graph() -> Result<()> {
        let graph = DependencyGraph::new();

        graph.add_dependency("proc_a".to_string(), "proc_b".to_string());
        graph.add_dependency("proc_b".to_string(), "proc_c".to_string());

        let deps = graph.get_dependencies("proc_a");
        assert_eq!(deps.len(), 1);
        assert!(deps.contains("proc_b"));

        Ok(())
    }

    #[test]
    fn test_symbol_table() {
        let mut table = SymbolTable::new();

        table.add_variable("x".to_string(), PlSqlType::Integer);
        table.add_constant("PI".to_string(), PlSqlType::Number { precision: None, scale: None });

        assert!(table.is_defined("x"));
        assert!(table.is_defined("PI"));
        assert!(table.is_constant("PI"));
        assert!(!table.is_constant("x"));

        table.remove_variable("x");
        assert!(!table.is_defined("x"));
    }
}


