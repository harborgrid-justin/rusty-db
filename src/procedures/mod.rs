// RustyDB Stored Procedures Module
// Enterprise-grade PL/SQL-compatible stored procedures, functions, triggers, and packages

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use crate::Result;
use crate::error::DbError;

// Sub-modules
pub mod parser;
pub mod runtime;
pub mod functions;
pub mod triggers;
pub mod packages;
pub mod cursors;
pub mod builtins;
pub mod compiler;

// Parameter mode for stored procedures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParameterMode {
    In,
    Out,
    InOut,
}

// Stored procedure parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcedureParameter {
    pub name: String,
    pub data_type: String,
    pub mode: ParameterMode,
}

// Stored procedure definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredProcedure {
    pub name: String,
    pub parameters: Vec<ProcedureParameter>,
    pub body: String,
    pub language: ProcedureLanguage,
}

// Language for stored procedure implementation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcedureLanguage {
    Sql,
    // Note: Native procedures are not yet implemented
    // Leaving this variant commented out until implementation is ready
    // Native,  // For future Rust-based procedures
}

// Procedure execution context
#[derive(Debug, Clone)]
pub struct ProcedureContext {
    pub parameters: HashMap<String, String>,
}

// Stored procedure manager
pub struct ProcedureManager {
    procedures: Arc<RwLock<HashMap<String, StoredProcedure>>>,
}

impl ProcedureManager {
    pub fn new() -> Self {
        Self {
            procedures: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Create a new stored procedure
    pub fn create_procedure(&self, procedure: StoredProcedure) -> Result<()> {
        let mut procedures = self.procedures.write();

        if procedures.contains_key(&procedure.name) {
            return Err(DbError::AlreadyExists(
                format!("Procedure '{}' already exists", procedure.name)
            ));
        }

        procedures.insert(procedure.name.clone(), procedure);
        Ok(())
    }

    // Drop a stored procedure
    pub fn drop_procedure(&self, name: &str) -> Result<()> {
        let mut procedures = self.procedures.write();

        if procedures.remove(name).is_none() {
            return Err(DbError::NotFound(
                format!("Procedure '{}' not found", name)
            ));
        }

        Ok(())
    }

    // Get a stored procedure by name
    pub fn get_procedure(&self, name: &str) -> Result<StoredProcedure> {
        let procedures = self.procedures.read();

        procedures.get(name)
            .cloned()
            .ok_or_else(|| DbError::NotFound(
                format!("Procedure '{}' not found", name)
            ))
    }

    // List all stored procedures
    pub fn list_procedures(&self) -> Vec<String> {
        let procedures = self.procedures.read();
        procedures.keys().cloned().collect()
    }

    // Execute a stored procedure
    pub fn execute_procedure(
        &self,
        name: &str,
        context: &ProcedureContext,
    ) -> Result<ProcedureResult> {
        let procedure = self.get_procedure(name)?;

        // Validate parameters
        self.validate_parameters(&procedure, context)?;

        // Execute based on language (only SQL is currently supported)
        self.execute_sql_procedure(&procedure, context)
    }

    // Validate procedure parameters
    fn validate_parameters(
        &self,
        procedure: &StoredProcedure,
        context: &ProcedureContext,
    ) -> Result<()> {
        for param in &procedure.parameters {
            if param.mode == ParameterMode::In || param.mode == ParameterMode::InOut {
                if !context.parameters.contains_key(&param.name) {
                    return Err(DbError::InvalidInput(
                        format!("Missing parameter '{}'", param.name)
                    ));
                }
            }
        }
        Ok(())
    }

    // Execute SQL-based procedure
    fn execute_sql_procedure(
        &self,
        procedure: &StoredProcedure,
        context: &ProcedureContext,
    ) -> Result<ProcedureResult> {
        // Parse and execute SQL statements in procedure body
        let mut output_parameters = HashMap::new();
        let mut rows_affected = 0;

        // Substitute input parameters into the procedure body
        let mut body = procedure.body.clone();
        for (param_name, param_value) in &context.parameters {
            // Replace parameter references like :param_name or @param_name
            body = body.replace(&format!(":{}", param_name), param_value);
            body = body.replace(&format!("@{}", param_name), param_value);
        }

        // Split body into individual statements (separated by semicolons)
        let statements: Vec<&str> = body
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for stmt in statements {
            let stmt_upper = stmt.to_uppercase();

            // Handle SELECT INTO statements (assign to output parameters)
            if stmt_upper.starts_with("SELECT") && stmt_upper.contains("INTO") {
                // Parse SELECT ... INTO variable pattern
                // Example: SELECT column INTO :output_var FROM table WHERE ...
                if let Some(into_pos) = stmt_upper.find("INTO") {
                    let after_into = &stmt[into_pos + 4..].trim();
                    // Extract variable name (until FROM or next whitespace)
                    let var_end = after_into.find(|c: char| c.is_whitespace() || c == ',')
                        .unwrap_or(after_into.len());
                    let var_name = after_into[..var_end].trim().trim_start_matches(':').trim_start_matches('@');

                    // Check if this is an OUT parameter
                    for param in &procedure.parameters {
                        if param.name.eq_ignore_ascii_case(var_name)
                            && (param.mode == ParameterMode::Out || param.mode == ParameterMode::InOut) {
                            // In a full implementation, execute the SELECT and get the result
                            // For now, set a placeholder value
                            output_parameters.insert(param.name.clone(), "<result>".to_string());
                        }
                    }
                }
            } else if stmt_upper.starts_with("INSERT")
                || stmt_upper.starts_with("UPDATE")
                || stmt_upper.starts_with("DELETE") {
                // DML statements affect rows
                // In a full implementation, execute and get actual row count
                rows_affected += 1;
            }
            // Other statements (SET, DECLARE, etc.) would be handled here
        }

        // Copy INOUT parameters that weren't modified to output
        for param in &procedure.parameters {
            if param.mode == ParameterMode::InOut && !output_parameters.contains_key(&param.name) {
                if let Some(value) = context.parameters.get(&param.name) {
                    output_parameters.insert(param.name.clone(), value.clone());
                }
            }
        }

        Ok(ProcedureResult {
            output_parameters,
            rows_affected,
        })
    }
}

impl Default for ProcedureManager {
    fn default() -> Self {
        Self::new()
    }
}

// Result of stored procedure execution
#[derive(Debug, Clone)]
pub struct ProcedureResult {
    pub output_parameters: HashMap<String, String>,
    pub rows_affected: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_procedure() -> Result<()> {
        let pm = ProcedureManager::new();

        let procedure = StoredProcedure {
            name: "calculate_discount".to_string(),
            parameters: vec![
                ProcedureParameter {
                    name: "price".to_string(),
                    data_type: "FLOAT".to_string(),
                    mode: ParameterMode::In,
                },
                ProcedureParameter {
                    name: "discount".to_string(),
                    data_type: "FLOAT".to_string(),
                    mode: ParameterMode::Out,
                },
            ],
            body: "SELECT price * 0.1 INTO discount;".to_string(),
            language: ProcedureLanguage::Sql,
        };

        pm.create_procedure(procedure)?;

        let procedures = pm.list_procedures();
        assert_eq!(procedures.len(), 1);
        assert!(procedures.contains(&"calculate_discount".to_string()));

        Ok(())
    }

    #[test]
    fn test_drop_procedure() -> Result<()> {
        let pm = ProcedureManager::new();

        let procedure = StoredProcedure {
            name: "test_proc".to_string(),
            parameters: vec![],
            body: "".to_string(),
            language: ProcedureLanguage::Sql,
        };

        pm.create_procedure(procedure)?;
        assert_eq!(pm.list_procedures().len(), 1);

        pm.drop_procedure("test_proc")?;
        assert_eq!(pm.list_procedures().len(), 0);

        Ok(())
    }

    #[test]
    fn test_duplicate_procedure() -> Result<()> {
        let pm = ProcedureManager::new();

        let procedure = StoredProcedure {
            name: "duplicate".to_string(),
            parameters: vec![],
            body: "".to_string(),
            language: ProcedureLanguage::Sql,
        };

        pm.create_procedure(procedure.clone())?;

        let result = pm.create_procedure(procedure);
        assert!(result.is_err());

        Ok(())
    }
}
