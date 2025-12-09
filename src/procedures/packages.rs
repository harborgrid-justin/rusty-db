/// Package System for RustyDB
///
/// This module implements Oracle-style packages, which provide a way to group
/// related procedures, functions, variables, and types into a single namespace
/// with public and private visibility control.

use crate::{Result, DbError};
use crate::procedures::parser::{PlSqlBlock, PlSqlType};
use crate::procedures::runtime::{RuntimeExecutor, RuntimeValue};
use crate::procedures::functions::{ScalarFunction, TableFunction};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Visibility level for package members
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Visibility {
    /// Public member (visible in specification)
    Public,
    /// Private member (only in body)
    Private,
}

/// Package procedure definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageProcedure {
    pub name: String,
    pub parameters: Vec<PackageParameter>,
    pub body: Option<PlSqlBlock>,
    pub visibility: Visibility,
}

/// Package function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageFunction {
    pub name: String,
    pub parameters: Vec<PackageParameter>,
    pub return_type: PlSqlType,
    pub body: Option<PlSqlBlock>,
    pub visibility: Visibility,
}

/// Package parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageParameter {
    pub name: String,
    pub data_type: PlSqlType,
    pub mode: ParameterMode,
    pub default_value: Option<String>,
}

/// Parameter mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParameterMode {
    In,
    Out,
    InOut,
}

/// Package variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVariable {
    pub name: String,
    pub data_type: PlSqlType,
    pub initial_value: Option<String>,
    pub is_constant: bool,
    pub visibility: Visibility,
}

/// Package cursor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageCursor {
    pub name: String,
    pub query: String,
    pub parameters: Vec<PackageParameter>,
    pub visibility: Visibility,
}

/// Package type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageType {
    pub name: String,
    pub definition: PlSqlType,
    pub visibility: Visibility,
}

/// Package exception
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageException {
    pub name: String,
    pub error_code: Option<i32>,
    pub visibility: Visibility,
}

/// Package specification (public interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSpecification {
    pub name: String,
    pub procedures: Vec<PackageProcedure>,
    pub functions: Vec<PackageFunction>,
    pub variables: Vec<PackageVariable>,
    pub cursors: Vec<PackageCursor>,
    pub types: Vec<PackageType>,
    pub exceptions: Vec<PackageException>,
}

impl PackageSpecification {
    pub fn new(name: String) -> Self {
        Self {
            name,
            procedures: Vec::new(),
            functions: Vec::new(),
            variables: Vec::new(),
            cursors: Vec::new(),
            types: Vec::new(),
            exceptions: Vec::new(),
        }
    }

    /// Get all public members
    pub fn get_public_procedures(&self) -> Vec<&PackageProcedure> {
        self.procedures.iter()
            .filter(|p| p.visibility == Visibility::Public)
            .collect()
    }

    pub fn get_public_functions(&self) -> Vec<&PackageFunction> {
        self.functions.iter()
            .filter(|f| f.visibility == Visibility::Public)
            .collect()
    }

    pub fn get_public_variables(&self) -> Vec<&PackageVariable> {
        self.variables.iter()
            .filter(|v| v.visibility == Visibility::Public)
            .collect()
    }
}

/// Package body (implementation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageBody {
    pub name: String,
    pub procedures: Vec<PackageProcedure>,
    pub functions: Vec<PackageFunction>,
    pub variables: Vec<PackageVariable>,
    pub cursors: Vec<PackageCursor>,
    pub types: Vec<PackageType>,
    pub initialization: Option<PlSqlBlock>,
}

impl PackageBody {
    pub fn new(name: String) -> Self {
        Self {
            name,
            procedures: Vec::new(),
            functions: Vec::new(),
            variables: Vec::new(),
            cursors: Vec::new(),
            types: Vec::new(),
            initialization: None,
        }
    }
}

/// Complete package (specification + body)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub specification: PackageSpecification,
    pub body: Option<PackageBody>,
    pub instantiated: bool,
}

impl Package {
    pub fn new(specification: PackageSpecification) -> Self {
        Self {
            specification,
            body: None,
            instantiated: false,
        }
    }

    pub fn with_body(mut self, body: PackageBody) -> Result<Self> {
        // Validate that body implements all public specification items
        self.validate_body(&body)?;
        self.body = Some(body);
        Ok(self)
    }

    fn validate_body(&self, body: &PackageBody) -> Result<()> {
        // Check that body matches specification name
        if self.specification.name != body.name {
            return Err(DbError::InvalidInput(
                format!("Package body name '{}' does not match specification '{}'",
                    body.name, self.specification.name)
            )));
        }

        // Check that all public procedures are implemented
        for spec_proc in &self.specification.procedures {
            if spec_proc.visibility == Visibility::Public {
                let implemented = body.procedures.iter()
                    .any(|p| p.name == spec_proc.name);

                if !implemented {
                    return Err(DbError::InvalidInput(
                        format!("Procedure '{}' declared in specification but not implemented in body",
                            spec_proc.name)
                    )));
                }
            }
        }

        // Check that all public functions are implemented
        for spec_func in &self.specification.functions {
            if spec_func.visibility == Visibility::Public {
                let implemented = body.functions.iter()
                    .any(|f| f.name == spec_func.name);

                if !implemented {
                    return Err(DbError::InvalidInput(
                        format!("Function '{}' declared in specification but not implemented in body",
                            spec_func.name)
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get procedure by name (checks both public and private)
    pub fn get_procedure(&self, name: &str, allow_private: bool) -> Option<&PackageProcedure> {
        // Check specification
        if let Some(proc) = self.specification.procedures.iter().find(|p| p.name == name) {
            if proc.visibility == Visibility::Public || allow_private {
                return Some(proc);
            }
        }

        // Check body if allowed
        if allow_private {
            if let Some(ref body) = self.body {
                return body.procedures.iter().find(|p| p.name == name);
            }
        }

        None
    }

    /// Get function by name (checks both public and private)
    pub fn get_function(&self, name: &str, allow_private: bool) -> Option<&PackageFunction> {
        // Check specification
        if let Some(func) = self.specification.functions.iter().find(|f| f.name == name) {
            if func.visibility == Visibility::Public || allow_private {
                return Some(func);
            }
        }

        // Check body if allowed
        if allow_private {
            if let Some(ref body) = self.body {
                return body.functions.iter().find(|f| f.name == name);
            }
        }

        None
    }
}

/// Package instance state
#[derive(Clone)]
pub struct PackageInstance {
    pub package_name: String,
    pub variables: HashMap<String, RuntimeValue>,
    pub cursors: HashMap<String, CursorState>,
    pub initialized: bool,
}

impl PackageInstance {
    pub fn new(package_name: String) -> Self {
        Self {
            package_name,
            variables: HashMap::new(),
            cursors: HashMap::new(),
            initialized: false,
        }
    }

    pub fn initialize(&mut self, runtime: &RuntimeExecutor, initblock: Option<&PlSqlBlock>) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        if let Some(block) = init_block {
            runtime.execute(block)?;
        }

        self.initialized = true;
        Ok(())
    }

    pub fn set_variable(&mut self, name: String, value: RuntimeValue) {
        self.variables.insert(name, value);
    }

    pub fn get_variable(&self, name: &str) -> Option<&RuntimeValue> {
        self.variables.get(name)
    }
}

/// Cursor state
#[derive(Debug, Clone)]
pub struct CursorState {
    pub is_open: bool,
    pub current_row: usize,
    pub total_rows: usize,
}

/// Package manager
pub struct PackageManager {
    packages: Arc<RwLock<HashMap<String, Package>>>,
    instances: Arc<RwLock<HashMap<String, PackageInstance>>>,
    runtime: Arc<RuntimeExecutor>,
}

impl PackageManager {
    pub fn new() -> Self {
        Self {
            packages: Arc::new(RwLock::new(HashMap::new())),
            instances: Arc::new(RwLock::new(HashMap::new())),
            runtime: Arc::new(RuntimeExecutor::new()),
        }
    }

    /// Create package specification
    pub fn create_package_spec(&self, spec: PackageSpecification) -> Result<()> {
        let mut packages = self.packages.write();

        if packages.contains_key(&spec.name) {
            return Err(DbError::AlreadyExists(
                format!("Package '{}' already exists", spec.name)
            )));
        }

        packages.insert(spec.name.clone(), Package::new(spec));
        Ok(())
    }

    /// Create package body
    pub fn create_package_body(&self, body: PackageBody) -> Result<()> {
        let mut packages = self.packages.write();

        let package = packages.get_mut(&body.name).ok_or_else(||
            DbError::NotFound(format!("Package specification '{}' not found", body.name))
        )?);

        // Validate and attach body
        let validated_package = package.clone().with_body(body)?;
        *package = validated_package;

        Ok(())
    }

    /// Drop a package
    pub fn drop_package(&self, name: &str) -> Result<()> {
        let mut packages = self.packages.write();
        let mut instances = self.instances.write();

        if packages.remove(name).is_none() {
            return Err(DbError::NotFound(
                format!("Package '{}' not found", name)
            )));
        }

        // Remove instance if exists
        instances.remove(name);

        Ok(())
    }

    /// Get package instance (create if needed)
    pub fn get_instance(&self, package_name: &str) -> Result<PackageInstance> {
        let instances = self.instances.read();

        if let Some(instance) = instances.get(package_name) {
            Ok(instance.clone())
        } else {
            drop(instances);
            self.create_instance(package_name)
        }
    }

    fn create_instance(&self, packagename: &str) -> Result<PackageInstance> {
        let packages = self.packages.read();
        let mut instances = self.instances.write();

        let package = packages.get(package_name).ok_or_else(||
            DbError::NotFound(format!("Package '{}' not found", package_name))
        )?);

        let mut instance = PackageInstance::new(package_name.to_string());

        // Initialize package variables
        for var in &package.specification.variables {
            if let Some(_init_val) = &var.initial_value {
                // TODO: Parse and evaluate initial value
                instance.set_variable(var.name.clone(), RuntimeValue::Null);
            }
        }

        // Run initialization block if present
        if let Some(ref body) = package.body {
            instance.initialize(&self.runtime, body.initialization.as_ref())?;
        }

        instances.insert(package_name.to_string(), instance.clone());
        Ok(instance)
    }

    /// Call a package procedure
    pub fn call_procedure(
        &self,
        package_name: &str,
        procedure_name: &str,
        arguments: Vec<RuntimeValue>,
    ) -> Result<()> {
        let packages = self.packages.read();

        let package = packages.get(package_name).ok_or_else(||
            DbError::NotFound(format!("Package '{}' not found", package_name))
        )?);

        let procedure = package.get_procedure(procedure_name, false).ok_or_else(||
            DbError::NotFound(format!("Procedure '{}' not found in package '{}'",
                procedure_name, package_name))
        )?);

        // Validate argument count
        if arguments.len() != procedure.parameters.len() {
            return Err(DbError::InvalidInput(
                format!("Procedure '{}' expects {} arguments, got {}",
                    procedure_name, procedure.parameters.len(), arguments.len())
            )));
        }

        // Execute procedure body
        if let Some(ref body) = procedure.body {
            self.runtime.execute(body)?;
        }

        Ok(())
    }

    /// Call a package function
    pub fn call_function(
        &self,
        package_name: &str,
        function_name: &str,
        arguments: Vec<RuntimeValue>,
    ) -> Result<RuntimeValue> {
        let packages = self.packages.read();

        let package = packages.get(package_name).ok_or_else(||
            DbError::NotFound(format!("Package '{}' not found", package_name))
        )?);

        let function = package.get_function(function_name, false).ok_or_else(||
            DbError::NotFound(format!("Function '{}' not found in package '{}'",
                function_name, package_name))
        )?);

        // Validate argument count
        if arguments.len() != function.parameters.len() {
            return Err(DbError::InvalidInput(
                format!("Function '{}' expects {} arguments, got {}",
                    function_name, function.parameters.len(), arguments.len())
            )));
        }

        // Execute function body
        if let Some(ref body) = function.body {
            let result = self.runtime.execute(body)?;
            result.return_value.ok_or_else(||
                DbError::Runtime(format!("Function '{}' did not return a value", function_name))
            )
        } else {
            Err(DbError::Runtime(format!("Function '{}' has no body", function_name)))
        }
    }

    /// Get package variable value
    pub fn get_variable(
        &self,
        package_name: &str,
        variable_name: &str,
    ) -> Result<RuntimeValue> {
        let instances = self.instances.read());

        let instance = instances.get(package_name).ok_or_else(||
            DbError::NotFound(format!("Package instance '{}' not found", package_name))
        )?);

        instance.get_variable(variable_name)
            .cloned()
            .ok_or_else(|| DbError::NotFound(
                format!("Variable '{}' not found in package '{}'", variable_name, package_name)
            ))
    }

    /// Set package variable value
    pub ffn set_variable(
        &self,
        packagename: &str,
        variablename: &str,
        value: RuntimeValue,
    ) Result<()> {
        let packages = self.packages.read());
        let mut instances = self.instances.write();

        // Verify package and variable exist
        let package = packages.get(package_name).ok_or_else(||
            DbError::NotFound(format!("Package '{}' not found", package_name))
        )?);

        let var = package.specification.variables.iter()
            .find(|v| v.name == variable_name)
            .ok_or_else(|| DbError::NotFound(
                format!("Variable '{}' not found in package '{}'", variable_name, package_name)
            ))?);

        // Check if variable is constant
        if var.is_constant {
            return Err(DbError::InvalidInput(
                format!("Cannot modify constant variable '{}'", variable_name)
            )));
        }

        // Get or create instance
        let instance = instances.entry(package_name.to_string())
            .or_insert_with(|| PackageInstance::new(package_name.to_string()));

        instance.set_variable(variable_name.to_string(), value);

        Ok(())
    }

    /// List all packages
    pub fn list_packages(&self) -> Vec<String> {
        let packages = self.packages.read();
        packages.keys().cloned().collect()
    }

    /// Get package by name
    pub fn get_package(&self, name: &str) -> Result<Package> {
        let packages = self.packages.read();
        packages.get(name)
            .cloned()
            .ok_or_else(|| DbError::NotFound(
                format!("Package '{}' not found", name)
            ))
    }

    /// Get package documentation
    pub fn get_documentation(&self, packagename: &str) -> Result<PackageDocumentation> {
        let package = self.get_package(package_name)?);

        let mut doc = PackageDocumentation {
            name: package.specification.name.clone(),
            procedures: Vec::new(),
            functions: Vec::new(),
            variables: Vec::new(),
            cursors: Vec::new(),
            types: Vec::new(),
            exceptions: Vec::new(),
        };

        // Document public procedures
        for proc in package.specification.get_public_procedures() {
            let params: Vec<String> = proc.parameters.iter()
                .map(|p| format!("{} {:?} {:?}", p.name, p.mode, p.data_type))
                .collect());
            doc.procedures.push(format!("{}({})", proc.name, params.join(", "))));
        }

        // Document public functions
        for func in package.specification.get_public_functions() {
            let params: Vec<String> = func.parameters.iter()
                .map(|p| format!("{} {:?} {:?}", p.name, p.mode, p.data_type))
                .collect());
            doc.functions.push(format!("{}({}) RETURN {:?}",
                func.name, params.join(", "), func.return_type)));
        }

        // Document public variables
        for var in package.specification.get_public_variables() {
            let const_marker = if var.is_constant { " CONSTANT" } else { "" };
            doc.variables.push(format!("{} {:?}{}", var.name, var.data_type, const_marker)));
        }

        Ok(doc)
    }
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Package documentation structure
#[derive(Debug, Clone)]
pub struct PackageDocumentation {
    pub name: String,
    pub procedures: Vec<String>,
    pub functions: Vec<String>,
    pub variables: Vec<String>,
    pub cursors: Vec<String>,
    pub types: Vec<String>,
    pub exceptions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_package_spec() -> Result<()> {
        let manager = PackageManager::new();

        let mut spec = PackageSpecification::new("test_package".to_string());
        spec.variables.push(PackageVariable {
            name: "counter".to_string(),
            data_type: PlSqlType::Integer,
            initial_value: Some("0".to_string()),
            is_constant: false,
            visibility: Visibility::Public,
        });

        manager.create_package_spec(spec)?;

        assert_eq!(manager.list_packages().len(), 1);

        Ok(())
    }

    #[test]
    fn test_package_with_body() -> Result<()> {
        let manager = PackageManager::new();

        let mut spec = PackageSpecification::new("math_package".to_string());
        spec.functions.push(PackageFunction {
            name: "add".to_string(),
            parameters: vec![],
            return_type: PlSqlType::Integer,
            body: None,
            visibility: Visibility::Public,
        });

        manager.create_package_spec(spec)?;

        let mut body = PackageBody::new("math_package".to_string());
        body.functions.push(PackageFunction {
            name: "add".to_string(),
            parameters: vec![],
            return_type: PlSqlType::Integer,
            body: Some(PlSqlBlock {
                declarations: Vec::new(),
                statements: Vec::new(),
                exception_handlers: Vec::new(),
            }),
            visibility: Visibility::Public,
        });

        manager.create_package_body(body)?;

        let package = manager.get_package("math_package")?;
        assert!(package.body.is_some());

        Ok(())
    }

    #[test]
    fn test_package_variable_access() -> Result<()> {
        let manager = PackageManager::new();

        let mut spec = PackageSpecification::new("vars_package".to_string());
        spec.variables.push(PackageVariable {
            name: "counter".to_string(),
            data_type: PlSqlType::Integer,
            initial_value: Some("42".to_string()),
            is_constant: false,
            visibility: Visibility::Public,
        });

        manager.create_package_spec(spec)?;

        // Initialize instance
        let instance = manager.get_instance("vars_package")?;

        // Set variable
        manager.set_variable("vars_package", "counter", RuntimeValue::Integer(100))?;

        // Get variable
        let value = manager.get_variable("vars_package", "counter")?;
        assert_eq!(value, RuntimeValue::Integer(100));

        Ok(())
    }
}


