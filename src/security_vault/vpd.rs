// # Virtual Private Database (VPD)
//
// Oracle-like row-level security and column-level security with policy-based
// access control and dynamic predicate injection.
//
// ## Features
//
// - **Row-Level Security (RLS)**: Filter rows based on security policies
// - **Column-Level Security**: Hide or redact columns based on privileges
// - **Dynamic Predicates**: Runtime query rewriting with security predicates
// - **Context-Aware**: Security based on session context attributes
// - **Policy Functions**: Custom security predicates via functions
//
// ## How It Works
//
// ```text
// Original Query:
//   SELECT * FROM employees WHERE department = 'IT'
//
// VPD Predicate Injection:
//   SELECT * FROM employees
//   WHERE department = 'IT'
//   AND (manager_id = SYS_CONTEXT('USER_ID')
//        OR SYS_CONTEXT('ROLE') = 'ADMIN')
//
// Result: User only sees their managed employees unless they're admin
// ```

use crate::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use regex::Regex;

/// Security predicate type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityPredicate {
    /// Static SQL predicate
    Static(String),
    /// Dynamic predicate based on context
    Dynamic {
        /// Template with placeholders
        template: String,
        /// Context variables to substitute
        variables: Vec<String>,
    },
    /// Function-based predicate
    Function {
        /// Function name
        name: String,
        /// Function arguments
        args: Vec<String>,
    },
    /// Composite predicate (AND/OR)
    Composite {
        /// Operator (AND/OR)
        operator: String,
        /// Sub-predicates
        predicates: Vec<SecurityPredicate>,
    },
}

impl SecurityPredicate {
    /// Evaluate predicate with context
    pub fn evaluate(&self, context: &HashMap<String, String>) -> Result<String> {
        match self {
            Self::Static(sql) => Ok(sql.clone()),

            Self::Dynamic { template, variables } => {
                let mut result = template.clone();
                for var in variables {
                    let placeholder = format!("${{{}}}", var));
                    if let Some(value) = context.get(var) {
                        result = result.replace(&placeholder, value);
                    } else {
                        return Err(DbError::InvalidInput(format!(
                            "Context variable not found: {}", var
                        ))));
                    }
                }
                Ok(result)
            }

            Self::Function { name, args } => {
                // In a real implementation, this would call a registered function
                let args_str = args.join(", ");
                Ok(format!("{}({})", name, args_str))
            }

            Self::Composite { operator, predicates } => {
                let evaluated: Result<Vec<String>> = predicates.iter()
                    .map(|p| p.evaluate(context))
                    .collect());
                let evaluated = evaluated?;

                if evaluated.is_empty() {
                    Ok("1=1".to_string())
                } else {
                    Ok(format!("({})", evaluated.join(&format!(" {} ", operator))))
                }
            }
        }
    }

    /// Parse predicate from string
    pub fn parse(s: &str) -> Result<Self> {
        // Simple parser - in production would use proper SQL parser
        if s.contains("${") {
            // Dynamic predicate
            let var_regex = Regex::new(r"\$\{([^}]+)\}")
                .map_err(|e| DbError::InvalidInput(format!("Invalid regex: {}", e)))?);

            let variables: Vec<String> = var_regex.captures_iter(s)
                .map(|cap| cap[1].to_string())
                .collect();

            Ok(Self::Dynamic {
                template: s.to_string(),
                variables,
            })
        } else {
            // Static predicate
            Ok(Self::Static(s.to_string()))
        }
    }
}

/// VPD policy scope
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyScope {
    /// Apply to SELECT queries
    Select,
    /// Apply to INSERT operations
    Insert,
    /// Apply to UPDATE operations
    Update,
    /// Apply to DELETE operations
    Delete,
    /// Apply to all DML operations
    All,
}

/// VPD policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpdPolicy {
    /// Policy name
    pub name: String,
    /// Table name
    pub table_name: String,
    /// Schema name (optional)
    pub schema_name: Option<String>,
    /// Security predicate
    pub predicate: SecurityPredicate,
    /// Policy scope
    pub scope: Vec<PolicyScope>,
    /// Enabled flag
    pub enabled: bool,
    /// Priority (higher applies first)
    pub priority: i32,
    /// Apply to specific users (None = all users)
    pub apply_to_users: Option<Vec<String>>,
    /// Apply to specific roles (None = all roles)
    pub apply_to_roles: Option<Vec<String>>,
    /// Exempt users (bypass policy)
    pub exempt_users: Option<Vec<String>>,
    /// Created timestamp
    pub created_at: i64,
}

impl VpdPolicy {
    /// Create a new VPD policy
    pub fn new(name: String, tablename: String, predicate: SecurityPredicate) -> Self {
        Self {
            name,
            table_name,
            schema_name: None,
            predicate,
            scope: vec![PolicyScope::All],
            enabled: true,
            priority: 0,
            apply_to_users: None,
            apply_to_roles: None,
            exempt_users: None,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Check if policy applies to user and operation
    pub fn applies_to(
        &self,
        user_id: &str,
        userroles: &[String],
        operation: &PolicyScope,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        // Check exemptions
        if let Some(ref exempt) = self.exempt_users {
            if exempt.contains(&user_id.to_string()) {
                return false;
            }
        }

        // Check scope
        if !self.scope.contains(&PolicyScope::All) && !self.scope.contains(operation) {
            return false;
        }

        // Check user restrictions
        if let Some(ref users) = self.apply_to_users {
            if !users.contains(&user_id.to_string()) {
                return false;
            }
        }

        // Check role restrictions
        if let Some(ref roles) = self.apply_to_roles {
            if !user_roles.iter().any(|r| roles.contains(r)) {
                return false;
            }
        }

        true
    }
}

/// Column security policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnPolicy {
    /// Policy name
    pub name: String,
    /// Table name
    pub table_name: String,
    /// Column name
    pub column_name: String,
    /// Action when policy applies
    pub action: ColumnAction,
    /// Apply to specific users
    pub apply_to_users: Option<Vec<String>>,
    /// Apply to specific roles
    pub apply_to_roles: Option<Vec<String>>,
    /// Enabled flag
    pub enabled: bool,
}

/// Column security action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ColumnAction {
    /// Hide column from result
    Hide,
    /// Return NULL
    Nullify,
    /// Redact with mask
    Redact(String),
}

/// Query rewrite result
#[derive(Debug, Clone)]
pub struct RewrittenQuery {
    /// Original query
    pub original: String,
    /// Rewritten query with security predicates
    pub rewritten: String,
    /// Applied policies
    pub applied_policies: Vec<String>,
    /// Hidden columns
    pub hidden_columns: Vec<String>,
}

/// Main VPD Engine
pub struct VpdEngine {
    /// Row-level security policies
    row_policies: RwLock<HashMap<String, VpdPolicy>>,
    /// Column-level security policies
    column_policies: RwLock<HashMap<String, ColumnPolicy>>,
    /// Policy statistics
    stats: RwLock<VpdStats>,
}

/// VPD statistics
#[derive(Debug, Default)]
struct VpdStats {
    total_rewrites: u64,
    policies_applied: u64,
    columns_hidden: u64,
    policy_violations: u64,
}

impl VpdEngine {
    /// Create a new VPD engine
    pub fn new() -> Result<Self> {
        Ok(Self {
            row_policies: RwLock::new(HashMap::new()),
            column_policies: RwLock::new(HashMap::new()),
            stats: RwLock::new(VpdStats::default()),
        })
    }

    /// Create a row-level security policy
    pub fn create_policy(&mut self, table_name: &str, predicate: &str) -> Result<()> {
        let policy_name = format!("vpd_{}", table_name));
        let pred = SecurityPredicate::parse(predicate)?;

        let policy = VpdPolicy::new(
            policy_name.clone(),
            table_name.to_string(),
            pred,
        );

        self.row_policies.write().insert(policy_name, policy);
        Ok(())
    }

    /// Create a policy with custom configuration
    pub fn create_policy_custom(&mut self, policy: VpdPolicy) -> Result<()> {
        self.row_policies.write().insert(policy.name.clone(), policy);
        Ok(())
    }

    /// Drop a policy
    pub fn drop_policy(&mut self, name: &str) -> Result<()> {
        self.row_policies.write().remove(name)
            .ok_or_else(|| DbError::NotFound(format!("Policy not found: {}", name)))?);
        Ok(())
    }

    /// Enable a policy
    pub fn enable_policy(&mut self, name: &str) -> Result<()> {
        let mut policies = self.row_policies.write();
        let policy = policies.get_mut(name)
            .ok_or_else(|| DbError::NotFound(format!("Policy not found: {}", name)))?);
        policy.enabled = true;
        Ok(())
    }

    /// Disable a policy
    pub fn disable_policy(&mut self, name: &str) -> Result<()> {
        let mut policies = self.row_policies.write();
        let policy = policies.get_mut(name)
            .ok_or_else(|| DbError::NotFound(format!("Policy not found: {}", name)))?);
        policy.enabled = false;
        Ok(())
    }

    /// Create a column security policy
    pub ffn create_column_policy(
        &mut self,
        name: String,
        tablename: String,
        columnname: String,
        action: ColumnAction,
    ) Result<()> {
        let policy = ColumnPolicy {
            name: name.clone(),
            table_name,
            column_name,
            action,
            apply_to_users: None,
            apply_to_roles: None,
            enabled: true,
        };

        self.column_policies.write().insert(name, policy);
        Ok(())
    }

    /// Rewrite query with security predicates
    pub fn rewrite_query(
        &self,
        query: &str,
        user_id: &str,
        user_roles: &[String],
        context: &HashMap<String, String>,
    ) -> Result<RewrittenQuery> {
        // Extract table name from query (simplified - real implementation would parse SQL)
        let table_name = self.extract_table_name(query)?;

        // Find applicable policies
        let policies = self.row_policies.read();
        let mut applicable: Vec<&VpdPolicy> = policies.values()
            .filter(|p| {
                p.table_name == table_name &&
                p.applies_to(user_id, user_roles, &PolicyScope::Select)
            })
            .collect();

        // Sort by priority
        applicable.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut rewritten = query.to_string();
        let mut applied_policies = Vec::new();

        // Apply row-level policies
        for policy in applicable {
            let predicate_sql = policy.predicate.evaluate(context)?;
            rewritten = self.inject_predicate(&rewritten, &predicate_sql)?;
            applied_policies.push(policy.name.clone());
        }

        // Apply column-level policies
        let hidden_columns = self.apply_column_policies(
            &mut rewritten,
            &table_name,
            user_id,
            user_roles,
        )?;

        // Update statistics
        let mut stats = self.stats.write();
        stats.total_rewrites += 1;
        stats.policies_applied += applied_policies.len() as u64;
        stats.columns_hidden += hidden_columns.len() as u64;

        Ok(RewrittenQuery {
            original: query.to_string(),
            rewritten,
            applied_policies,
            hidden_columns,
        })
    }

    /// Extract table name from query (simplified)
    fn extract_table_name(&self, query: &str) -> Result<String> {
        // Simplified extraction - real implementation would use SQL parser
        let from_regex = Regex::new(r"(?i)FROM\s+([a-zA-Z0-9_]+)")
            .map_err(|e| DbError::InvalidInput(format!("Invalid regex: {}", e)))?);

        if let Some(cap) = from_regex.captures(query) {
            Ok(cap[1].to_string())
        } else {
            Err(DbError::InvalidInput("Could not extract table name".to_string()))
        }
    }

    /// Inject security predicate into query
    fn inject_predicate(&self, query: &str, predicate: &str) -> Result<String> {
        // Simplified injection - real implementation would use SQL AST manipulation
        let where_regex = Regex::new(r"(?i)(WHERE\s+)")
            .map_err(|e| DbError::InvalidInput(format!("Invalid regex: {}", e)))?);

        if where_regex.is_match(query) {
            // Query already has WHERE clause - AND the predicate
            let rewritten = where_regex.replace(query, |caps: &regex::Captures| {
                format!("{}{} AND ", &caps[1], predicate)
            }));
            Ok(rewritten.to_string())
        } else {
            // No WHERE clause - add one
            let order_regex = Regex::new(r"(?i)(ORDER\s+BY|GROUP\s+BY|LIMIT|$)")
                .map_err(|e| DbError::InvalidInput(format!("Invalid regex: {}", e)))?);

            let rewritten = order_regex.replace(query, |caps: &regex::Captures| {
                if caps[0].is_empty() {
                    format!(" WHERE {} ", predicate)
                } else {
                    format!(" WHERE {} {}", predicate, &caps[0])
                }
            }));
            Ok(rewritten.to_string())
        }
    }

    /// Apply column-level security policies
    fn apply_column_policies(
        &self,
        query: &mut String,
        table_name: &str,
        user_id: &str,
        userroles: &[String],
    ) -> Result<Vec<String>> {
        let policies = self.column_policies.read();
        let mut hidden_columns = Vec::new();

        for policy in policies.values() {
            if policy.table_name != table_name || !policy.enabled {
                continue;
            }

            // Check if policy applies
            if let Some(ref users) = policy.apply_to_users {
                if !users.contains(&user_id.to_string()) {
                    continue;
                }
            }

            if let Some(ref roles) = policy.apply_to_roles {
                if !user_roles.iter().any(|r| roles.contains(r)) {
                    continue;
                }
            }

            // Apply action
            match &policy.action {
                ColumnAction::Hide => {
                    // Remove column from SELECT list
                    *query = self.remove_column(query, &policy.column_name)?;
                    hidden_columns.push(policy.column_name.clone());
                }
                ColumnAction::Nullify => {
                    // Replace column with NULL
                    *query = self.nullify_column(query, &policy.column_name)?;
                }
                ColumnAction::Redact(mask) => {
                    // Replace column with redacted value
                    *query = self.redact_column(query, &policy.column_name, mask)?;
                }
            }
        }

        Ok(hidden_columns)
    }

    /// Remove column from SELECT list
    fn remove_column(&self, query: &str, column_name: &str) -> Result<String> {
        // Simplified - real implementation would parse SELECT list properly
        let pattern = format!(r"(?i),?\s*{}\s*,?", regex::escape(column_name)));
        let regex = Regex::new(&pattern)
            .map_err(|e| DbError::InvalidInput(format!("Invalid regex: {}", e)))?);
        Ok(regex.replace_all(query, "").to_string())
    }

    /// Replace column with NULL
    fn nullify_column(&self, query: &str, column_name: &str) -> Result<String> {
        let pattern = format!(r"(?i)\b{}\b", regex::escape(column_name)));
        let regex = Regex::new(&pattern)
            .map_err(|e| DbError::InvalidInput(format!("Invalid regex: {}", e)))?);
        Ok(regex.replace_all(query, "NULL").to_string())
    }

    /// Replace column with redacted value
    fn redact_column(&self, query: &str, column_name: &str, mask: &str) -> Result<String> {
        let pattern = format!(r"(?i)\b{}\b", regex::escape(column_name)));
        let regex = Regex::new(&pattern)
            .map_err(|e| DbError::InvalidInput(format!("Invalid regex: {}", e)))?);
        Ok(regex.replace_all(query, mask).to_string())
    }

    /// List all policies
    pub fn list_policies(&self) -> Vec<String> {
        self.row_policies.read().keys().cloned().collect()
    }

    /// Get policy details
    pub fn get_policy(&self, name: &str) -> Option<VpdPolicy> {
        self.row_policies.read().get(name).cloned()
    }

    /// List all column policies
    pub fn list_column_policies(&self) -> Vec<String> {
        self.column_policies.read().keys().cloned().collect()
    }

    /// Get VPD statistics
    pub fn get_stats(&self) -> (u64, u64, u64, u64) {
        let stats = self.stats.read();
        (stats.total_rewrites, stats.policies_applied, stats.columns_hidden, stats.policy_violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_predicate() {
        let pred = SecurityPredicate::Static("user_id = 123".to_string());
        let context = HashMap::new();
        let result = pred.evaluate(&context).unwrap();
        assert_eq!(result, "user_id = 123");
    }

    #[test]
    fn test_dynamic_predicate() {
        let pred = SecurityPredicate::Dynamic {
            template: "user_id = ${USER_ID}".to_string(),
            variables: vec!["USER_ID".to_string()],
        };

        let mut context = HashMap::new();
        context.insert("USER_ID".to_string(), "123".to_string());

        let result = pred.evaluate(&context).unwrap();
        assert_eq!(result, "user_id = 123");
    }

    #[test]
    fn test_composite_predicate() {
        let pred = SecurityPredicate::Composite {
            operator: "OR".to_string(),
            predicates: vec![
                SecurityPredicate::Static("role = 'ADMIN'".to_string()),
                SecurityPredicate::Static("user_id = 123".to_string()),
            ],
        };

        let context = HashMap::new();
        let result = pred.evaluate(&context).unwrap();
        assert_eq!(result, "(role = 'ADMIN' OR user_id = 123)");
    }

    #[test]
    fn test_policy_applies_to() {
        let policy = VpdPolicy::new(
            "test_policy".to_string(),
            "employees".to_string(),
            SecurityPredicate::Static("1=1".to_string()),
        );

        assert!(policy.applies_to("user1", &[], &PolicyScope::Select));
    }

    #[test]
    fn test_create_policy() {
        let mut engine = VpdEngine::new().unwrap();
        engine.create_policy("employees", "department_id = ${DEPT_ID}").unwrap();

        let policies = engine.list_policies();
        assert_eq!(policies.len(), 1);
    }

    #[test]
    fn test_query_rewrite_simple() {
        let mut engine = VpdEngine::new().unwrap();

        let policy = VpdPolicy::new(
            "emp_policy".to_string(),
            "employees".to_string(),
            SecurityPredicate::Static("department_id = 10".to_string()),
        );
        engine.create_policy_custom(policy).unwrap();

        let query = "SELECT * FROM employees WHERE active = 1";
        let mut context = HashMap::new();
        context.insert("USER_ID".to_string(), "123".to_string());

        let result = engine.rewrite_query(query, "user1", &[], &context).unwrap();

        assert!(result.rewritten.contains("department_id = 10"));
        assert_eq!(result.applied_policies.len(), 1);
    }

    #[test]
    fn test_column_policy() {
        let mut engine = VpdEngine::new().unwrap();

        engine.create_column_policy(
            "hide_salary".to_string(),
            "employees".to_string(),
            "salary".to_string(),
            ColumnAction::Nullify,
        ).unwrap();

        let column_policies = engine.list_column_policies();
        assert_eq!(column_policies.len(), 1);
    }

    #[test]
    fn test_extract_table_name() {
        let engine = VpdEngine::new().unwrap();

        let query1 = "SELECT * FROM employees WHERE id = 1";
        assert_eq!(engine.extract_table_name(query1).unwrap(), "employees");

        let query2 = "SELECT id, name from users";
        assert_eq!(engine.extract_table_name(query2).unwrap(), "users");
    }

    #[test]
    fn test_predicate_injection() {
        let engine = VpdEngine::new().unwrap();

        let query = "SELECT * FROM employees WHERE active = 1";
        let result = engine.inject_predicate(query, "dept_id = 10").unwrap();

        assert!(result.contains("dept_id = 10 AND"));
        assert!(result.contains("active = 1"));
    }
}
