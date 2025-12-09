// GraphQL Query Builders
//
// Programmatic query construction utilities

use async_graphql::{Enum, Result as GqlResult, SimpleObject, ID};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::DbError;
use super::types::*;
use super::models::*;

// ============================================================================
// ADDITIONAL UTILITIES & HELPERS
// ============================================================================

/// Query builder for constructing complex queries programmatically
pub struct QueryBuilder {
    table: String,
    where_clauses: Vec<WhereClause>,
    order_by: Vec<OrderBy>,
    limit: Option<i32>,
    offset: Option<i32>,
    select_fields: Vec<String>,
}

impl QueryBuilder {
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            where_clauses: vec![],
            order_by: vec![],
            limit: None,
            offset: None,
            select_fields: vec![],
        }
    }

    pub fn select(mut self, fields: Vec<String>) -> Self {
        self.select_fields = fields;
        self
    }

    pub fn where_clause(mut self, clause: WhereClause) -> Self {
        self.where_clauses.push(clause);
        self
    }

    pub fn order_by(mut self, field: impl Into<String>, order: SortOrder) -> Self {
        self.order_by.push(OrderBy {
            field: field.into(),
            order,
        });
        self
    }

    pub fn limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: i32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn build(self) -> BuiltQuery {
        let where_clause = Self::combine_where_clauses_static(&self.where_clauses);
        BuiltQuery {
            table: self.table,
            where_clause,
            order_by: if self.order_by.is_empty() {
                None
            } else {
                Some(self.order_by)
            },
            limit: self.limit,
            offset: self.offset,
            select_fields: self.select_fields,
        }
    }

    fn combine_where_clauses_static(where_clauses: &[WhereClause]) -> Option<WhereClause> {
        if where_clauses.is_empty() {
            None
        } else if where_clauses.len() == 1 {
            Some(where_clauses[0].clone())
        } else {
            Some(WhereClause {
                and: Some(where_clauses.to_vec()),
                or: None,
                not: None,
                condition: None,
            })
        }
    }
}

/// Built query ready for execution
#[derive(Clone, Debug)]
pub struct BuiltQuery {
    pub table: String,
    pub where_clause: Option<WhereClause>,
    pub order_by: Option<Vec<OrderBy>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub select_fields: Vec<String>,
}

/// Mutation builder for constructing complex mutations
pub struct MutationBuilder {
    table: String,
    operation: MutationOperation,
}

#[derive(Clone, Debug)]
pub enum MutationOperation {
    Insert { data: HashMap<String, Json> },
    Update { id: ID, data: HashMap<String, Json> },
    Delete { id: ID },
    BulkInsert { data: Vec<HashMap<String, Json>> },
    BulkUpdate { where_clause: WhereClause, data: HashMap<String, Json> },
    BulkDelete { where_clause: WhereClause },
}

impl MutationBuilder {
    pub fn insert(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            operation: MutationOperation::Insert {
                data: HashMap::new(),
            },
        }
    }

    pub fn update(table: impl Into<String>, id: ID) -> Self {
        Self {
            table: table.into(),
            operation: MutationOperation::Update {
                id,
                data: HashMap::new(),
            },
        }
    }

    pub fn delete(table: impl Into<String>, id: ID) -> Self {
        Self {
            table: table.into(),
            operation: MutationOperation::Delete { id },
        }
    }

    pub fn set(mut self, field: impl Into<String>, value: serde_json::Value) -> Self {
        match &mut self.operation {
            MutationOperation::Insert { data } | MutationOperation::Update { data, .. } => {
                data.insert(field.into(), Json(value));
            }
            _ => {}
        }
        self
    }

    pub fn build(self) -> BuiltMutation {
        BuiltMutation {
            table: self.table,
            operation: self.operation,
        }
    }
}

/// Built mutation ready for execution
#[derive(Clone, Debug)]
pub struct BuiltMutation {
    pub table: String,
    pub operation: MutationOperation,
}

/// Schema introspection utilities
pub struct SchemaIntrospector {
    engine: Arc<GraphQLEngine>,
}

impl SchemaIntrospector {
    pub fn new(engine: Arc<GraphQLEngine>) -> Self {
        Self { engine }
    }

    /// Get all available types in the schema
    pub async fn get_types(&self) -> GqlResult<Vec<TypeInfo>> {
        Ok(vec![
            TypeInfo {
                name: "Query".to_string(),
                kind: TypeKind::Object,
                description: Some("Root query type".to_string()),
            },
            TypeInfo {
                name: "Mutation".to_string(),
                kind: TypeKind::Object,
                description: Some("Root mutation type".to_string()),
            },
            TypeInfo {
                name: "Subscription".to_string(),
                kind: TypeKind::Object,
                description: Some("Root subscription type".to_string()),
            },
        ])
    }

    /// Get all available queries
    pub async fn get_queries(&self) -> GqlResult<Vec<FieldInfo>> {
        Ok(vec![
            FieldInfo {
                name: "schemas".to_string(),
                description: Some("Get all database schemas".to_string()),
                return_type: "DatabaseSchema".to_string(),
                arguments: vec![],
            },
            FieldInfo {
                name: "tables".to_string(),
                description: Some("Get all tables".to_string()),
                return_type: "TableType".to_string(),
                arguments: vec![],
            },
        ])
    }

    /// Get all available mutations
    pub async fn get_mutations(&self) -> GqlResult<Vec<FieldInfo>> {
        Ok(vec![
            FieldInfo {
                name: "insert_one".to_string(),
                description: Some("Insert a single row".to_string()),
                return_type: "MutationResult".to_string(),
                arguments: vec![],
            },
            FieldInfo {
                name: "update_one".to_string(),
                description: Some("Update a single row".to_string()),
                return_type: "MutationResult".to_string(),
                arguments: vec![],
            },
        ])
    }

    /// Get all available subscriptions
    pub async fn get_subscriptions(&self) -> GqlResult<Vec<FieldInfo>> {
        Ok(vec![
            FieldInfo {
                name: "table_changes".to_string(),
                description: Some("Subscribe to table changes".to_string()),
                return_type: "TableChange".to_string(),
                arguments: vec![],
            },
        ])
    }
}

/// Type information
#[derive(SimpleObject, Clone, Debug)]
pub struct TypeInfo {
    pub name: String,
    pub kind: TypeKind,
    pub description: Option<String>,
}

/// Type kind enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum TypeKind {
    Scalar,
    Object,
    Interface,
    Union,
    Enum,
    InputObject,
    List,
    NonNull,
}

/// Field information
#[derive(SimpleObject, Clone, Debug)]
pub struct FieldInfo {
    pub name: String,
    pub description: Option<String>,
    pub return_type: String,
    pub arguments: Vec<ArgumentInfo>,
}

/// Argument information
#[derive(SimpleObject, Clone, Debug)]
pub struct ArgumentInfo {
    pub name: String,
    pub type_name: String,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

/// Query optimizer for analyzing and improving query performance
pub struct QueryOptimizer {
    stats_cache: Arc<RwLock<HashMap<String, TableStats>>>,
}

impl QueryOptimizer {
    pub fn new() -> Self {
        Self {
            stats_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Analyze a query and suggest optimizations
    pub async fn analyze(&self, query: &BuiltQuery) -> OptimizationSuggestions {
        let mut suggestions = OptimizationSuggestions {
            suggestions: vec![],
            estimated_cost: 0.0,
            estimated_rows: 0,
        };

        // Check for missing indexes
        if query.where_clause.is_some() {
            suggestions.suggestions.push(
                "Consider adding an index on filtered columns for better performance".to_string(),
            );
        }

        // Check for large result sets
        if query.limit.is_none() {
            suggestions.suggestions.push(
                "Add a LIMIT clause to prevent retrieving too many rows".to_string(),
            );
        }

        // Check for unnecessary columns
        if query.select_fields.is_empty() {
            suggestions.suggestions.push(
                "Specify only the columns you need instead of selecting all".to_string(),
            );
        }

        suggestions
    }

    /// Update statistics for a table
    pub async fn update_stats(&self, table: String, stats: TableStats) {
        let mut cache = self.stats_cache.write().await;
        cache.insert(table, stats);
    }

    /// Get statistics for a table
    pub async fn get_stats(&self, table: &str) -> Option<TableStats> {
        let cache = self.stats_cache.read().await;
        cache.get(table).cloned()
    }
}

/// Optimization suggestions
#[derive(SimpleObject, Clone, Debug)]
pub struct OptimizationSuggestions {
    pub suggestions: Vec<String>,
    pub estimated_cost: f64,
    pub estimated_rows: i64,
}

/// Table statistics for optimization
#[derive(Clone, Debug)]
pub struct TableStats {
    pub row_count: i64,
    pub avg_row_size: f64,
    pub indexes: Vec<String>,
    pub last_vacuum: Option<DateTime>,
    pub last_analyze: Option<DateTime>,
}

/// GraphQL request validator
pub struct RequestValidator {
    max_query_size: usize,
    allowed_operations: HashSet<String>,
}

impl RequestValidator {
    pub fn new(max_query_size: usize) -> Self {
        Self {
            max_query_size,
            allowed_operations: HashSet::new(),
        }
    }

    pub fn allow_operation(&mut self, operation: impl Into<String>) {
        self.allowed_operations.insert(operation.into());
    }

    /// Validate a GraphQL request
    pub fn validate(&self, query: &str) -> Result<()> {
        // Check query size
        if query.len() > self.max_query_size {
            return Err(DbError::InvalidInput {
                message: format!("Query exceeds maximum size of {} bytes", self.max_query_size),
            });
        }
        Ok(())
    }
}