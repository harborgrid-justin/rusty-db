//! # Fine-Grained Access Control (FGAC) Module
//!
//! Provides row-level security policies, column-level masking, virtual private
//! database patterns, and predicate injection for queries.
//!
//! ## Features
//!
//! - Row-level security (RLS) policies
//! - Column-level masking and redaction
//! - Virtual Private Database (VPD) implementation
//! - Dynamic predicate injection
//! - Context-sensitive access control
//! - Data classification and sensitivity levels

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use std::sync::Arc;
use crate::Result;
use crate::error::DbError;

/// Table identifier
pub type TableId = String;

/// Column identifier
pub type ColumnId = String;

/// User/Role identifier
pub type PrincipalId = String;

/// Security context for policy evaluation
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Current user
    pub user_id: String,
    /// Active roles
    pub roles: HashSet<String>,
    /// Session attributes
    pub session_attributes: HashMap<String, String>,
    /// IP address
    pub ip_address: Option<String>,
    /// Timestamp
    pub timestamp: i64,
    /// Application context
    pub app_context: HashMap<String, String>,
}

/// Row-level security policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowLevelPolicy {
    /// Policy ID
    pub id: String,
    /// Policy name
    pub name: String,
    /// Table this policy applies to
    pub table_id: TableId,
    /// Policy type
    pub policy_type: PolicyType,
    /// SQL predicate expression
    pub predicate: String,
    /// Principals this policy applies to
    pub principals: Vec<PrincipalId>,
    /// Whether policy is enabled
    pub enabled: bool,
    /// Policy priority (higher = evaluated first)
    pub priority: i32,
    /// Policy creation timestamp
    pub created_at: i64,
    /// Policy update timestamp
    pub updated_at: i64,
    /// Description
    pub description: Option<String>,
}

/// Type of row-level security policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyType {
    /// Permissive policy - ORed with other permissive policies
    Permissive,
    /// Restrictive policy - ANDed with all policies
    Restrictive,
}

/// Column-level access policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnPolicy {
    /// Policy ID
    pub id: String,
    /// Policy name
    pub name: String,
    /// Table ID
    pub table_id: TableId,
    /// Column ID
    pub column_id: ColumnId,
    /// Access level
    pub access_level: ColumnAccessLevel,
    /// Principals this policy applies to
    pub principals: Vec<PrincipalId>,
    /// Masking function (if applicable)
    pub masking_function: Option<MaskingFunction>,
    /// Whether policy is enabled
    pub enabled: bool,
    /// Created timestamp
    pub created_at: i64,
}

/// Column access level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ColumnAccessLevel {
    /// Full access to column
    Full,
    /// Masked/redacted access
    Masked,
    /// No access (column not visible)
    None,
    /// Conditional access based on predicate
    Conditional { predicate: String },
}

/// Masking function for column data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MaskingFunction {
    /// Show only first N characters
    Partial { visible_chars: usize },
    /// Hash the value
    Hash,
    /// Replace with fixed value
    Fixed { value: String },
    /// Email masking (show domain, hide user)
    Email,
    /// Credit card masking (show last 4 digits)
    CreditCard,
    /// Random value from same distribution
    Random,
    /// NULL out the value
    Null,
    /// Custom SQL expression
    Custom { expression: String },
}

/// Virtual Private Database (VPD) context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpdContext {
    /// Context ID
    pub id: String,
    /// Context name
    pub name: String,
    /// Table this context applies to
    pub table_id: TableId,
    /// Context attributes
    pub attributes: HashMap<String, String>,
    /// Security predicate
    pub predicate: String,
    /// Whether context is active
    pub active: bool,
}

/// Data classification level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DataClassification {
    /// Public data
    Public,
    /// Internal use only
    Internal,
    /// Confidential data
    Confidential,
    /// Restricted/highly sensitive
    Restricted,
    /// Top secret
    TopSecret,
}

/// Column classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnClassification {
    /// Table ID
    pub table_id: TableId,
    /// Column ID
    pub column_id: ColumnId,
    /// Classification level
    pub classification: DataClassification,
    /// Tags for additional metadata
    pub tags: HashSet<String>,
}

/// Security predicate that can be injected into queries
#[derive(Debug, Clone)]
pub struct SecurityPredicate {
    /// Predicate expression
    pub expression: String,
    /// Bind parameters
    pub parameters: HashMap<String, String>,
}

/// Fine-grained access control manager
pub struct FgacManager {
    /// Row-level policies by table
    row_policies: Arc<RwLock<HashMap<TableId, Vec<RowLevelPolicy>>>>,
    /// Column-level policies
    column_policies: Arc<RwLock<HashMap<TableId, HashMap<ColumnId, Vec<ColumnPolicy>>>>>,
    /// VPD contexts
    vpd_contexts: Arc<RwLock<HashMap<TableId, Vec<VpdContext>>>>,
    /// Column classifications
    column_classifications: Arc<RwLock<HashMap<TableId, HashMap<ColumnId, ColumnClassification>>>>,
    /// Policy cache for performance
    policy_cache: Arc<RwLock<HashMap<String, Vec<SecurityPredicate>>>>,
}

impl FgacManager {
    /// Create a new FGAC manager
    pub fn new() -> Self {
        Self {
            row_policies: Arc::new(RwLock::new(HashMap::new())),
            column_policies: Arc::new(RwLock::new(HashMap::new())),
            vpd_contexts: Arc::new(RwLock::new(HashMap::new())),
            column_classifications: Arc::new(RwLock::new(HashMap::new())),
            policy_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a row-level security policy
    pub fn add_row_policy(&self, policy: RowLevelPolicy) -> Result<()> {
        let mut policies = self.row_policies.write();
        let table_policies = policies.entry(policy.table_id.clone()).or_insert_with(Vec::new);

        // Check for duplicate policy names on same table
        if table_policies.iter().any(|p| p.name == policy.name) {
            return Err(DbError::AlreadyExists(
                format!("Policy {} already exists on table {}", policy.name, policy.table_id)
            ));
        }

        table_policies.push(policy);

        // Sort by priority (descending)
        table_policies.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Invalidate cache
        self.policy_cache.write().clear();

        Ok(())
    }

    /// Remove a row-level security policy
    pub fn remove_row_policy(&self, table_id: &TableId, policy_id: &str) -> Result<()> {
        let mut policies = self.row_policies.write();

        if let Some(table_policies) = policies.get_mut(table_id) {
            let original_len = table_policies.len();
            table_policies.retain(|p| p.id != policy_id);

            if table_policies.len() == original_len {
                return Err(DbError::NotFound(format!("Policy {} not found", policy_id)));
            }

            // Invalidate cache
            self.policy_cache.write().clear();

            Ok(())
        } else {
            Err(DbError::NotFound(format!("No policies for table {}", table_id)))
        }
    }

    /// Get row-level policies for a table
    pub fn get_row_policies(&self, table_id: &TableId) -> Vec<RowLevelPolicy> {
        self.row_policies.read()
            .get(table_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Add a column-level policy
    pub fn add_column_policy(&self, policy: ColumnPolicy) -> Result<()> {
        let mut policies = self.column_policies.write();
        let table_policies = policies.entry(policy.table_id.clone()).or_insert_with(HashMap::new);
        let column_policies = table_policies.entry(policy.column_id.clone()).or_insert_with(Vec::new);

        // Check for duplicate
        if column_policies.iter().any(|p| p.id == policy.id) {
            return Err(DbError::AlreadyExists(
                format!("Column policy {} already exists", policy.id)
            ));
        }

        column_policies.push(policy);

        // Invalidate cache
        self.policy_cache.write().clear();

        Ok(())
    }

    /// Remove a column-level policy
    pub fn remove_column_policy(
        &self,
        table_id: &TableId,
        column_id: &ColumnId,
        policy_id: &str,
    ) -> Result<()> {
        let mut policies = self.column_policies.write();

        if let Some(table_policies) = policies.get_mut(table_id) {
            if let Some(column_policies) = table_policies.get_mut(column_id) {
                let original_len = column_policies.len();
                column_policies.retain(|p| p.id != policy_id);

                if column_policies.len() == original_len {
                    return Err(DbError::NotFound(format!("Policy {} not found", policy_id)));
                }

                // Invalidate cache
                self.policy_cache.write().clear();

                return Ok(());
            }
        }

        Err(DbError::NotFound("Policy not found".to_string()))
    }

    /// Get column policies for a specific column
    pub fn get_column_policies(
        &self,
        table_id: &TableId,
        column_id: &ColumnId,
    ) -> Vec<ColumnPolicy> {
        self.column_policies.read()
            .get(table_id)
            .and_then(|t| t.get(column_id))
            .cloned()
            .unwrap_or_default()
    }

    /// Get effective column access for a principal
    pub fn get_column_access(
        &self,
        table_id: &TableId,
        column_id: &ColumnId,
        principal_id: &PrincipalId,
        context: &SecurityContext,
    ) -> ColumnAccessLevel {
        let policies = self.get_column_policies(table_id, column_id);

        // If no policies, default to full access
        if policies.is_empty() {
            return ColumnAccessLevel::Full;
        }

        let mut most_restrictive = ColumnAccessLevel::Full;

        for policy in policies {
            if !policy.enabled {
                continue;
            }

            // Check if policy applies to this principal
            let applies = policy.principals.is_empty() ||
                         policy.principals.contains(principal_id) ||
                         policy.principals.iter().any(|p| context.roles.contains(p));

            if applies {
                // Take most restrictive access level
                most_restrictive = match (&most_restrictive, &policy.access_level) {
                    (_, ColumnAccessLevel::None) => ColumnAccessLevel::None,
                    (ColumnAccessLevel::None, _) => ColumnAccessLevel::None,
                    (_, ColumnAccessLevel::Masked) => ColumnAccessLevel::Masked,
                    (ColumnAccessLevel::Masked, _) => ColumnAccessLevel::Masked,
                    (a, ColumnAccessLevel::Full) => a.clone(),
                    (ColumnAccessLevel::Full, b) => b.clone(),
                    (a, _) => a.clone(),
                };
            }
        }

        most_restrictive
    }

    /// Apply masking to a column value
    pub fn apply_masking(&self, value: &str, function: &MaskingFunction) -> String {
        match function {
            MaskingFunction::Partial { visible_chars } => {
                if value.len() <= *visible_chars {
                    value.to_string()
                } else {
                    let visible = &value[..*visible_chars];
                    let masked = "*".repeat(value.len() - visible_chars);
                    format!("{}{}", visible, masked)
                }
            }
            MaskingFunction::Hash => {
                format!("HASH_{:x}", value.len()) // Simplified hash
            }
            MaskingFunction::Fixed { value: fixed } => {
                fixed.clone()
            }
            MaskingFunction::Email => {
                if let Some(at_pos) = value.find('@') {
                    let (user, domain) = value.split_at(at_pos);
                    if user.len() > 2 {
                        format!("{}***{}", &user[..1], domain)
                    } else {
                        format!("***{}", domain)
                    }
                } else {
                    "***@***".to_string()
                }
            }
            MaskingFunction::CreditCard => {
                if value.len() > 4 {
                    let last4 = &value[value.len()-4..];
                    format!("****-****-****-{}", last4)
                } else {
                    "****".to_string()
                }
            }
            MaskingFunction::Random => {
                // Would generate random value with same characteristics
                "RANDOM_VALUE".to_string()
            }
            MaskingFunction::Null => {
                "NULL".to_string()
            }
            MaskingFunction::Custom { expression: _ } => {
                // Would evaluate custom expression
                "CUSTOM_MASKED".to_string()
            }
        }
    }

    /// Add a VPD context
    pub fn add_vpd_context(&self, context: VpdContext) -> Result<()> {
        let mut contexts = self.vpd_contexts.write();
        let table_contexts = contexts.entry(context.table_id.clone()).or_insert_with(Vec::new);

        // Check for duplicate
        if table_contexts.iter().any(|c| c.id == context.id) {
            return Err(DbError::AlreadyExists(
                format!("VPD context {} already exists", context.id)
            ));
        }

        table_contexts.push(context);

        // Invalidate cache
        self.policy_cache.write().clear();

        Ok(())
    }

    /// Remove a VPD context
    pub fn remove_vpd_context(&self, table_id: &TableId, context_id: &str) -> Result<()> {
        let mut contexts = self.vpd_contexts.write();

        if let Some(table_contexts) = contexts.get_mut(table_id) {
            let original_len = table_contexts.len();
            table_contexts.retain(|c| c.id != context_id);

            if table_contexts.len() == original_len {
                return Err(DbError::NotFound(format!("VPD context {} not found", context_id)));
            }

            // Invalidate cache
            self.policy_cache.write().clear();

            Ok(())
        } else {
            Err(DbError::NotFound(format!("No VPD contexts for table {}", table_id)))
        }
    }

    /// Get VPD contexts for a table
    pub fn get_vpd_contexts(&self, table_id: &TableId) -> Vec<VpdContext> {
        self.vpd_contexts.read()
            .get(table_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Generate security predicates for a table based on context
    pub fn generate_security_predicates(
        &self,
        table_id: &TableId,
        context: &SecurityContext,
    ) -> Vec<SecurityPredicate> {
        let mut predicates = Vec::new();

        // Get row-level policies
        let row_policies = self.get_row_policies(table_id);
        let mut permissive_predicates = Vec::new();
        let mut restrictive_predicates = Vec::new();

        for policy in row_policies {
            if !policy.enabled {
                continue;
            }

            // Check if policy applies to this user/roles
            let applies = policy.principals.is_empty() ||
                         policy.principals.contains(&context.user_id) ||
                         policy.principals.iter().any(|p| context.roles.contains(p));

            if applies {
                let predicate = SecurityPredicate {
                    expression: policy.predicate.clone(),
                    parameters: HashMap::new(),
                };

                match policy.policy_type {
                    PolicyType::Permissive => permissive_predicates.push(predicate),
                    PolicyType::Restrictive => restrictive_predicates.push(predicate),
                }
            }
        }

        // Combine permissive policies with OR
        if !permissive_predicates.is_empty() {
            let combined_expr = permissive_predicates.iter()
                .map(|p| format!("({})", p.expression))
                .collect::<Vec<_>>()
                .join(" OR ");

            predicates.push(SecurityPredicate {
                expression: combined_expr,
                parameters: HashMap::new(),
            });
        }

        // Add all restrictive policies with AND
        predicates.extend(restrictive_predicates);

        // Add VPD context predicates
        let vpd_contexts = self.get_vpd_contexts(table_id);
        for vpd_ctx in vpd_contexts {
            if vpd_ctx.active {
                predicates.push(SecurityPredicate {
                    expression: vpd_ctx.predicate.clone(),
                    parameters: HashMap::new(),
                });
            }
        }

        predicates
    }

    /// Set column classification
    pub fn set_column_classification(
        &self,
        table_id: TableId,
        column_id: ColumnId,
        classification: DataClassification,
        tags: HashSet<String>,
    ) -> Result<()> {
        let mut classifications = self.column_classifications.write();
        let table_classifications = classifications.entry(table_id.clone()).or_insert_with(HashMap::new);

        table_classifications.insert(
            column_id.clone(),
            ColumnClassification {
                table_id,
                column_id,
                classification,
                tags,
            },
        );

        Ok(())
    }

    /// Get column classification
    pub fn get_column_classification(
        &self,
        table_id: &TableId,
        column_id: &ColumnId,
    ) -> Option<ColumnClassification> {
        self.column_classifications.read()
            .get(table_id)
            .and_then(|t| t.get(column_id))
            .cloned()
    }

    /// Get all columns with a specific classification level
    pub fn get_columns_by_classification(
        &self,
        classification: &DataClassification,
    ) -> Vec<ColumnClassification> {
        let classifications = self.column_classifications.read();
        let mut result = Vec::new();

        for table_classifications in classifications.values() {
            for col_class in table_classifications.values() {
                if &col_class.classification == classification {
                    result.push(col_class.clone());
                }
            }
        }

        result
    }

    /// Check if user can access data at a specific classification level
    pub fn can_access_classification(
        &self,
        context: &SecurityContext,
        required_level: &DataClassification,
    ) -> bool {
        // Get user's clearance level from session attributes
        let user_clearance = context.session_attributes
            .get("clearance_level")
            .and_then(|s| match s.as_str() {
                "PUBLIC" => Some(DataClassification::Public),
                "INTERNAL" => Some(DataClassification::Internal),
                "CONFIDENTIAL" => Some(DataClassification::Confidential),
                "RESTRICTED" => Some(DataClassification::Restricted),
                "TOP_SECRET" => Some(DataClassification::TopSecret),
                _ => None,
            })
            .unwrap_or(DataClassification::Public);

        // User can access data at or below their clearance level
        user_clearance >= *required_level
    }

    /// Inject security predicates into a query
    pub fn inject_predicates(
        &self,
        original_query: &str,
        table_id: &TableId,
        context: &SecurityContext,
    ) -> String {
        let predicates = self.generate_security_predicates(table_id, context);

        if predicates.is_empty() {
            return original_query.to_string();
        }

        // Build combined WHERE clause
        let security_where = predicates.iter()
            .map(|p| format!("({})", p.expression))
            .collect::<Vec<_>>()
            .join(" AND ");

        // Inject into query (simplified - would need proper SQL parsing)
        if original_query.to_uppercase().contains("WHERE") {
            format!("{} AND ({})", original_query, security_where)
        } else {
            format!("{} WHERE {}", original_query, security_where)
        }
    }

    /// Clear all caches
    pub fn clear_cache(&self) {
        self.policy_cache.write().clear();
    }

    /// Get statistics about policies
    pub fn get_statistics(&self) -> FgacStatistics {
        let row_policies = self.row_policies.read();
        let column_policies = self.column_policies.read();
        let vpd_contexts = self.vpd_contexts.read();
        let classifications = self.column_classifications.read();

        let total_row_policies: usize = row_policies.values().map(|v| v.len()).sum();
        let total_column_policies: usize = column_policies.values()
            .flat_map(|t| t.values())
            .map(|v| v.len())
            .sum();
        let total_vpd_contexts: usize = vpd_contexts.values().map(|v| v.len()).sum();
        let total_classified_columns: usize = classifications.values()
            .map(|t| t.len())
            .sum();

        FgacStatistics {
            total_row_policies,
            total_column_policies,
            total_vpd_contexts,
            total_classified_columns,
            tables_with_policies: row_policies.len(),
        }
    }
}

impl Default for FgacManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about FGAC usage
#[derive(Debug, Clone)]
pub struct FgacStatistics {
    pub total_row_policies: usize,
    pub total_column_policies: usize,
    pub total_vpd_contexts: usize,
    pub total_classified_columns: usize,
    pub tables_with_policies: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_policy() {
        let manager = FgacManager::new();

        let policy = RowLevelPolicy {
            id: "pol1".to_string(),
            name: "User Data Policy".to_string(),
            table_id: "users".to_string(),
            policy_type: PolicyType::Permissive,
            predicate: "user_id = current_user()".to_string(),
            principals: vec!["user".to_string()],
            enabled: true,
            priority: 100,
            created_at: 0,
            updated_at: 0,
            description: None,
        };

        assert!(manager.add_row_policy(policy).is_ok());
    }

    #[test]
    fn test_column_masking() {
        let manager = FgacManager::new();

        // Test email masking
        let masked = manager.apply_masking(
            "user@example.com",
            &MaskingFunction::Email,
        );
        assert!(masked.contains("@example.com"));
        assert!(masked.contains("***"));

        // Test credit card masking
        let masked = manager.apply_masking(
            "1234567890123456",
            &MaskingFunction::CreditCard,
        );
        assert!(masked.ends_with("3456"));
        assert!(masked.contains("****"));
    }

    #[test]
    fn test_data_classification() {
        let manager = FgacManager::new();

        manager.set_column_classification(
            "employees".to_string(),
            "salary".to_string(),
            DataClassification::Confidential,
            HashSet::from(["pii".to_string(), "financial".to_string()]),
        ).unwrap();

        let classification = manager.get_column_classification(
            &"employees".to_string(),
            &"salary".to_string(),
        );

        assert!(classification.is_some());
        assert_eq!(classification.unwrap().classification, DataClassification::Confidential);
    }

    #[test]
    fn test_predicate_generation() {
        let manager = FgacManager::new();

        let policy = RowLevelPolicy {
            id: "pol1".to_string(),
            name: "Department Policy".to_string(),
            table_id: "employees".to_string(),
            policy_type: PolicyType::Restrictive,
            predicate: "department = 'Engineering'".to_string(),
            principals: vec![],
            enabled: true,
            priority: 100,
            created_at: 0,
            updated_at: 0,
            description: None,
        };

        manager.add_row_policy(policy).unwrap();

        let context = SecurityContext {
            user_id: "user1".to_string(),
            roles: HashSet::new(),
            session_attributes: HashMap::new(),
            ip_address: None,
            timestamp: 0,
            app_context: HashMap::new(),
        };

        let predicates = manager.generate_security_predicates(&"employees".to_string(), &context);
        assert_eq!(predicates.len(), 1);
    }
}


