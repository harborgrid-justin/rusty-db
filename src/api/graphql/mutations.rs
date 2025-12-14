// GraphQL Mutation Operations
//
// Mutation resolvers for the GraphQL API

use async_graphql::{
    Context, Enum, Error, InputObject, Object, Result as GqlResult, SimpleObject, ID,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use super::models::*;
use super::types::*;
use super::GraphQLEngine;
use crate::api::AuthorizationContext;

// ============================================================================
// PART 3: MUTATION OPERATIONS (500+ lines)
// ============================================================================

// Root mutation type
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    // Insert a single row
    async fn insert_one(
        &self,
        ctx: &Context<'_>,
        table: String,
        data: HashMap<String, Json>,
    ) -> GqlResult<MutationResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", table)),
            }));
        }

        match engine.insert_one(&table, data).await {
            Ok(row) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(MutationResult::Success(MutationSuccess {
                    affected_rows: 1,
                    returning: Some(vec![row]),
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(MutationResult::Error(MutationError {
                message: e.to_string(),
                code: "INSERT_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Insert multiple rows
    async fn insert_many(
        &self,
        ctx: &Context<'_>,
        table: String,
        data: Vec<HashMap<String, Json>>,
    ) -> GqlResult<MutationResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", table)),
            }));
        }

        match engine.insert_many(&table, data).await {
            Ok(rows) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                let affected = rows.len() as i32;
                Ok(MutationResult::Success(MutationSuccess {
                    affected_rows: affected,
                    returning: Some(rows),
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(MutationResult::Error(MutationError {
                message: e.to_string(),
                code: "INSERT_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Update a single row by ID
    async fn update_one(
        &self,
        ctx: &Context<'_>,
        table: String,
        id: ID,
        data: HashMap<String, Json>,
    ) -> GqlResult<MutationResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", table)),
            }));
        }

        match engine.update_one(&table, &id, data).await {
            Ok(Some(row)) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(MutationResult::Success(MutationSuccess {
                    affected_rows: 1,
                    returning: Some(vec![row]),
                    execution_time_ms: execution_time,
                }))
            }
            Ok(None) => Ok(MutationResult::Error(MutationError {
                message: "Row not found".to_string(),
                code: "NOT_FOUND".to_string(),
                details: Some(format!("No row with id: {}", id.as_str())),
            })),
            Err(e) => Ok(MutationResult::Error(MutationError {
                message: e.to_string(),
                code: "UPDATE_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Update multiple rows matching a condition
    async fn update_many(
        &self,
        ctx: &Context<'_>,
        table: String,
        where_clause: WhereClause,
        data: HashMap<String, Json>,
    ) -> GqlResult<MutationResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", table)),
            }));
        }

        match engine.update_many(&table, where_clause, data).await {
            Ok(rows) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                let affected = rows.len() as i32;
                Ok(MutationResult::Success(MutationSuccess {
                    affected_rows: affected,
                    returning: Some(rows),
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(MutationResult::Error(MutationError {
                message: e.to_string(),
                code: "UPDATE_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Delete a single row by ID
    async fn delete_one(
        &self,
        ctx: &Context<'_>,
        table: String,
        id: ID,
    ) -> GqlResult<MutationResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", table)),
            }));
        }

        match engine.delete_one(&table, &id).await {
            Ok(true) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(MutationResult::Success(MutationSuccess {
                    affected_rows: 1,
                    returning: None,
                    execution_time_ms: execution_time,
                }))
            }
            Ok(false) => Ok(MutationResult::Error(MutationError {
                message: "Row not found".to_string(),
                code: "NOT_FOUND".to_string(),
                details: Some(format!("No row with id: {}", id.as_str())),
            })),
            Err(e) => Ok(MutationResult::Error(MutationError {
                message: e.to_string(),
                code: "DELETE_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Delete multiple rows matching a condition
    async fn delete_many(
        &self,
        ctx: &Context<'_>,
        table: String,
        where_clause: WhereClause,
    ) -> GqlResult<MutationResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", table)),
            }));
        }

        match engine.delete_many(&table, where_clause).await {
            Ok(affected) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(MutationResult::Success(MutationSuccess {
                    affected_rows: affected,
                    returning: None,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(MutationResult::Error(MutationError {
                message: e.to_string(),
                code: "DELETE_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Upsert (insert or update) a row
    async fn upsert(
        &self,
        ctx: &Context<'_>,
        table: String,
        unique_fields: Vec<String>,
        data: HashMap<String, Json>,
    ) -> GqlResult<MutationResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", table)),
            }));
        }

        match engine.upsert(&table, unique_fields, data).await {
            Ok((row, _was_inserted)) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(MutationResult::Success(MutationSuccess {
                    affected_rows: 1,
                    returning: Some(vec![row]),
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(MutationResult::Error(MutationError {
                message: e.to_string(),
                code: "UPSERT_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Begin a new transaction
    async fn begin_transaction(
        &self,
        ctx: &Context<'_>,
        isolation_level: Option<IsolationLevel>,
    ) -> GqlResult<TransactionResult> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.begin_transaction(isolation_level).await
    }

    // Commit a transaction
    async fn commit_transaction(
        &self,
        ctx: &Context<'_>,
        transaction_id: String,
    ) -> GqlResult<TransactionResult> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.commit_transaction(&transaction_id).await
    }

    // Rollback a transaction
    async fn rollback_transaction(
        &self,
        ctx: &Context<'_>,
        transaction_id: String,
    ) -> GqlResult<TransactionResult> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.rollback_transaction(&transaction_id).await
    }

    // Execute multiple mutations in a transaction
    async fn execute_transaction(
        &self,
        ctx: &Context<'_>,
        operations: Vec<TransactionOperation>,
        isolation_level: Option<IsolationLevel>,
    ) -> GqlResult<TransactionExecutionResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        match engine
            .execute_transaction(operations, isolation_level)
            .await
        {
            Ok(results) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(TransactionExecutionResult {
                    success: true,
                    results,
                    execution_time_ms: execution_time,
                    error: None,
                })
            }
            Err(e) => Ok(TransactionExecutionResult {
                success: false,
                results: vec![],
                execution_time_ms: start.elapsed().as_secs_f64() * 1000.0,
                error: Some(e.to_string()),
            }),
        }
    }

    // Bulk insert with optimizations
    async fn bulk_insert(
        &self,
        ctx: &Context<'_>,
        table: String,
        data: Vec<HashMap<String, Json>>,
        batch_size: Option<i32>,
    ) -> GqlResult<MutationResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", table)),
            }));
        }

        match engine
            .bulk_insert(&table, data, batch_size.unwrap_or(1000))
            .await
        {
            Ok(affected) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(MutationResult::Success(MutationSuccess {
                    affected_rows: affected,
                    returning: None,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(MutationResult::Error(MutationError {
                message: e.to_string(),
                code: "BULK_INSERT_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // ========================================================================
    // DDL OPERATIONS - Database Management
    // ========================================================================

    // Create a new database
    async fn create_database(
        &self,
        ctx: &Context<'_>,
        name: String,
        if_not_exists: Option<bool>,
    ) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.create_database")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.create_database permission".to_string()),
            }));
        }

        match engine
            .create_database(&name, if_not_exists.unwrap_or(false))
            .await
        {
            Ok(()) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!("Database '{}' created successfully", name),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "CREATE_DATABASE_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Drop a database
    async fn drop_database(
        &self,
        ctx: &Context<'_>,
        name: String,
        if_exists: Option<bool>,
    ) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.drop_database")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.drop_database permission".to_string()),
            }));
        }

        match engine
            .drop_database(&name, if_exists.unwrap_or(false))
            .await
        {
            Ok(()) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!("Database '{}' dropped successfully", name),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "DROP_DATABASE_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Backup a database
    async fn backup_database(
        &self,
        ctx: &Context<'_>,
        name: String,
        location: String,
        full_backup: Option<bool>,
    ) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.backup_database")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.backup_database permission".to_string()),
            }));
        }

        match engine
            .backup_database(&name, &location, full_backup.unwrap_or(true))
            .await
        {
            Ok(()) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!("Database '{}' backed up to '{}'", name, location),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "BACKUP_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // ========================================================================
    // DDL OPERATIONS - Table Management
    // ========================================================================

    // Alter table - add column
    async fn alter_table_add_column(
        &self,
        ctx: &Context<'_>,
        table: String,
        column: ColumnDefinitionInput,
    ) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.alter_table")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.alter_table permission".to_string()),
            }));
        }

        match engine.alter_table_add_column(&table, column).await {
            Ok(()) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!("Column added to table '{}'", table),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "ALTER_TABLE_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Alter table - drop column
    async fn alter_table_drop_column(
        &self,
        ctx: &Context<'_>,
        table: String,
        column_name: String,
        if_exists: Option<bool>,
    ) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.alter_table")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.alter_table permission".to_string()),
            }));
        }

        match engine
            .alter_table_drop_column(&table, &column_name, if_exists.unwrap_or(false))
            .await
        {
            Ok(()) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!("Column '{}' dropped from table '{}'", column_name, table),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "ALTER_TABLE_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Alter table - modify column
    async fn alter_table_modify_column(
        &self,
        ctx: &Context<'_>,
        table: String,
        column: ColumnDefinitionInput,
    ) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.alter_table")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.alter_table permission".to_string()),
            }));
        }

        match engine.alter_table_modify_column(&table, column).await {
            Ok(()) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!("Column modified in table '{}'", table),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "ALTER_TABLE_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Alter table - add constraint
    async fn alter_table_add_constraint(
        &self,
        ctx: &Context<'_>,
        table: String,
        constraint: ConstraintInput,
    ) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.alter_table")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.alter_table permission".to_string()),
            }));
        }

        match engine.alter_table_add_constraint(&table, constraint).await {
            Ok(()) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!("Constraint added to table '{}'", table),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "ALTER_TABLE_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Alter table - drop constraint
    async fn alter_table_drop_constraint(
        &self,
        ctx: &Context<'_>,
        table: String,
        constraint_name: String,
        if_exists: Option<bool>,
    ) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.alter_table")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.alter_table permission".to_string()),
            }));
        }

        match engine
            .alter_table_drop_constraint(&table, &constraint_name, if_exists.unwrap_or(false))
            .await
        {
            Ok(()) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!(
                        "Constraint '{}' dropped from table '{}'",
                        constraint_name, table
                    ),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "ALTER_TABLE_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Truncate table
    async fn truncate_table(&self, ctx: &Context<'_>, table: String) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.truncate_table")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.truncate_table permission".to_string()),
            }));
        }

        match engine.truncate_table(&table).await {
            Ok(()) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!("Table '{}' truncated successfully", table),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "TRUNCATE_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // ========================================================================
    // DDL OPERATIONS - View Management
    // ========================================================================

    // Create a view
    async fn create_view(
        &self,
        ctx: &Context<'_>,
        name: String,
        query: String,
        or_replace: Option<bool>,
    ) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.create_view")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.create_view permission".to_string()),
            }));
        }

        match engine
            .create_view(&name, &query, or_replace.unwrap_or(false))
            .await
        {
            Ok(()) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!("View '{}' created successfully", name),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "CREATE_VIEW_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Drop a view
    async fn drop_view(
        &self,
        ctx: &Context<'_>,
        name: String,
        if_exists: Option<bool>,
    ) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.drop_view")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.drop_view permission".to_string()),
            }));
        }

        match engine.drop_view(&name, if_exists.unwrap_or(false)).await {
            Ok(()) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!("View '{}' dropped successfully", name),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "DROP_VIEW_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // ========================================================================
    // DDL OPERATIONS - Index Management
    // ========================================================================

    // Create an index
    async fn create_index(
        &self,
        ctx: &Context<'_>,
        table: String,
        index_name: String,
        columns: Vec<String>,
        unique: Option<bool>,
        if_not_exists: Option<bool>,
    ) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.create_index")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.create_index permission".to_string()),
            }));
        }

        match engine
            .create_index(
                &table,
                &index_name,
                columns,
                unique.unwrap_or(false),
                if_not_exists.unwrap_or(false),
            )
            .await
        {
            Ok(()) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!("Index '{}' created on table '{}'", index_name, table),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "CREATE_INDEX_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Drop an index
    async fn drop_index(
        &self,
        ctx: &Context<'_>,
        index_name: String,
        table: Option<String>,
        if_exists: Option<bool>,
    ) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.drop_index")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.drop_index permission".to_string()),
            }));
        }

        match engine
            .drop_index(&index_name, table.as_deref(), if_exists.unwrap_or(false))
            .await
        {
            Ok(()) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!("Index '{}' dropped successfully", index_name),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "DROP_INDEX_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // ========================================================================
    // STORED PROCEDURES
    // ========================================================================

    // Create a stored procedure
    async fn create_procedure(
        &self,
        ctx: &Context<'_>,
        name: String,
        parameters: Vec<ProcedureParameter>,
        body: String,
        or_replace: Option<bool>,
    ) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.create_procedure")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.create_procedure permission".to_string()),
            }));
        }

        match engine
            .create_procedure(&name, parameters, &body, or_replace.unwrap_or(false))
            .await
        {
            Ok(()) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!("Procedure '{}' created successfully", name),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "CREATE_PROCEDURE_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Execute a stored procedure
    async fn execute_procedure(
        &self,
        ctx: &Context<'_>,
        name: String,
        arguments: Option<Vec<Json>>,
    ) -> GqlResult<ProcedureResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check execute permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("execute.procedure")? {
            return Ok(ProcedureResult::Error(ProcedureError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires execute.procedure permission".to_string()),
            }));
        }

        match engine
            .execute_procedure(&name, arguments.unwrap_or_default())
            .await
        {
            Ok(result) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(ProcedureResult::Success(ProcedureSuccess {
                    result,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(ProcedureResult::Error(ProcedureError {
                message: e.to_string(),
                code: "EXECUTE_PROCEDURE_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // ========================================================================
    // ADVANCED QUERY OPERATIONS
    // ========================================================================

    // Insert into ... select
    async fn insert_into_select(
        &self,
        ctx: &Context<'_>,
        target_table: String,
        target_columns: Option<Vec<String>>,
        source_query: String,
    ) -> GqlResult<MutationResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&target_table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", target_table)),
            }));
        }

        match engine
            .insert_into_select(&target_table, target_columns, &source_query)
            .await
        {
            Ok(affected) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(MutationResult::Success(MutationSuccess {
                    affected_rows: affected,
                    returning: None,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(MutationResult::Error(MutationError {
                message: e.to_string(),
                code: "INSERT_SELECT_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Select into (create new table from select)
    async fn select_into(
        &self,
        ctx: &Context<'_>,
        new_table: String,
        source_query: String,
    ) -> GqlResult<DdlResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check admin permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.create_table")? {
            return Ok(DdlResult::Error(DdlError {
                success: false,
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some("Requires admin.create_table permission".to_string()),
            }));
        }

        match engine.select_into(&new_table, &source_query).await {
            Ok(row_count) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(DdlResult::Success(DdlSuccess {
                    success: true,
                    message: format!("Table '{}' created with {} rows", new_table, row_count),
                    affected_rows: 1,
                    execution_time_ms: execution_time,
                }))
            }
            Err(e) => Ok(DdlResult::Error(DdlError {
                success: false,
                message: e.to_string(),
                code: "SELECT_INTO_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // ========================================================================
    // STRING FUNCTIONS
    // ========================================================================

    // Execute a string function
    async fn execute_string_function(
        &self,
        ctx: &Context<'_>,
        function_type: StringFunctionTypeEnum,
        parameters: Vec<String>,
    ) -> GqlResult<StringFunctionResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        match engine
            .execute_string_function(function_type, parameters)
            .await
        {
            Ok(result) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(StringFunctionResult {
                    result,
                    execution_time_ms: execution_time,
                })
            }
            Err(e) => Err(Error::new(e.to_string())),
        }
    }

    // Batch execute string functions
    async fn batch_string_functions(
        &self,
        ctx: &Context<'_>,
        functions: Vec<StringFunctionInput>,
    ) -> GqlResult<BatchStringFunctionResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        let mut results = Vec::new();
        for func in functions {
            match engine
                .execute_string_function(func.function_type, func.parameters)
                .await
            {
                Ok(result) => results.push(result),
                Err(e) => return Err(Error::new(e.to_string())),
            }
        }

        let execution_time = start.elapsed().as_secs_f64() * 1000.0;
        Ok(BatchStringFunctionResult {
            results,
            execution_time_ms: execution_time,
        })
    }
}

// Transaction result
#[derive(SimpleObject, Clone, Debug)]
pub struct TransactionResult {
    pub transaction_id: String,
    pub status: String,
    pub timestamp: DateTime,
}

// Transaction operation input
#[derive(InputObject, Clone, Debug)]
pub struct TransactionOperation {
    pub operation_type: TransactionOpType,
    pub table: String,
    pub data: Option<HashMap<String, Json>>,
    pub where_clause: Option<WhereClause>,
    pub id: Option<ID>,
}

// Transaction operation type
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum TransactionOpType {
    Insert,
    Update,
    Delete,
}

// Transaction execution result
#[derive(SimpleObject, Clone, Debug)]
pub struct TransactionExecutionResult {
    pub success: bool,
    pub results: Vec<String>,
    pub execution_time_ms: f64,
    pub error: Option<String>,
}

// ============================================================================
// DDL RESULT TYPES
// ============================================================================

/// DDL operation result (union type)
///
/// This is a GraphQL Union type, which means queries must use inline fragments
/// to access the specific fields of Success or Error variants.
///
/// # Correct Usage Examples
///
/// ## Query Success Fields
/// ```graphql
/// mutation {
///   createDatabase(name: "mydb") {
///     ... on DdlSuccess {
///       success
///       message
///       affected_rows
///       execution_time_ms
///     }
///     ... on DdlError {
///       success
///       message
///       code
///       details
///     }
///   }
/// }
/// ```
///
/// ## Use __typename to determine result type
/// ```graphql
/// mutation {
///   createDatabase(name: "mydb") {
///     __typename
///     ... on DdlSuccess { message affected_rows }
///     ... on DdlError { message code }
///   }
/// }
/// ```
///
/// # IMPORTANT: Invalid Queries
///
/// ❌ This will NOT work (cannot query union fields directly):
/// ```graphql
/// mutation {
///   createDatabase(name: "mydb") {
///     success message  # ERROR: Union has no direct fields
///   }
/// }
/// ```
///
/// ✅ Must use fragments:
/// ```graphql
/// mutation {
///   createDatabase(name: "mydb") {
///     ... on DdlSuccess { success message }
///     ... on DdlError { success message }
///   }
/// }
/// ```
#[derive(async_graphql::Union)]
pub enum DdlResult {
    Success(DdlSuccess),
    Error(DdlError),
}

/// DDL success result
///
/// Returned when a DDL operation (CREATE, DROP, ALTER, etc.) succeeds.
/// Contains information about what was changed and execution metrics.
#[derive(SimpleObject, Clone, Debug)]
pub struct DdlSuccess {
    /// Success indicator - always true for this variant
    pub success: bool,
    /// Human-readable success message describing what was done
    pub message: String,
    /// Number of database objects affected (tables, databases, indexes, etc.)
    pub affected_rows: i32,
    /// Execution time in milliseconds
    pub execution_time_ms: f64,
}

/// DDL error result
///
/// Returned when a DDL operation fails. Contains error details for debugging.
#[derive(SimpleObject, Clone, Debug)]
pub struct DdlError {
    /// Success indicator - always false for this variant
    pub success: bool,
    /// Human-readable error message
    pub message: String,
    /// Machine-readable error code (e.g., "PERMISSION_DENIED", "TABLE_NOT_FOUND")
    pub code: String,
    /// Optional detailed error information for debugging
    pub details: Option<String>,
}

// ============================================================================
// STORED PROCEDURE TYPES
// ============================================================================

// Procedure parameter input
#[derive(InputObject, Clone, Debug)]
pub struct ProcedureParameter {
    pub name: String,
    pub data_type: String,
    pub mode: Option<ParameterMode>,
}

// Parameter mode
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum ParameterMode {
    In,
    Out,
    InOut,
}

// Procedure result (union type)
#[derive(async_graphql::Union)]
pub enum ProcedureResult {
    Success(ProcedureSuccess),
    Error(ProcedureError),
}

// Procedure success result
#[derive(SimpleObject, Clone, Debug)]
pub struct ProcedureSuccess {
    pub result: Json,
    pub execution_time_ms: f64,
}

// Procedure error result
#[derive(SimpleObject, Clone, Debug)]
pub struct ProcedureError {
    pub message: String,
    pub code: String,
    pub details: Option<String>,
}

// ============================================================================
// ALTER TABLE TYPES
// ============================================================================

// Column definition input
#[derive(InputObject, Clone, Debug)]
pub struct ColumnDefinitionInput {
    pub name: String,
    pub data_type: String,
    pub nullable: Option<bool>,
    pub default_value: Option<Json>,
    pub primary_key: Option<bool>,
    pub unique: Option<bool>,
    pub auto_increment: Option<bool>,
}

// Constraint input
#[derive(InputObject, Clone, Debug)]
pub struct ConstraintInput {
    pub name: String,
    pub constraint_type: ConstraintTypeEnum,
    pub columns: Vec<String>,
    pub reference_table: Option<String>,
    pub reference_columns: Option<Vec<String>>,
    pub check_expression: Option<String>,
}

// Constraint type enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum ConstraintTypeEnum {
    PrimaryKey,
    ForeignKey,
    Unique,
    Check,
    Default,
}

// ============================================================================
// STRING FUNCTION TYPES
// ============================================================================

/// String function input
#[derive(InputObject, Clone, Debug)]
pub struct StringFunctionInput {
    pub function_type: StringFunctionTypeEnum,
    pub parameters: Vec<String>,
}

/// String function types enum - all 32 SQL Server string functions
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum StringFunctionTypeEnum {
    Ascii,
    Char,
    CharIndex,
    Concat,
    ConcatWs,
    DataLength,
    Difference,
    Format,
    Left,
    Len,
    Lower,
    LTrim,
    NChar,
    PatIndex,
    QuoteName,
    Replace,
    Replicate,
    Reverse,
    Right,
    RTrim,
    Soundex,
    Space,
    Str,
    Stuff,
    Substring,
    Translate,
    Trim,
    Unicode,
    Upper,
}

/// String function result
#[derive(SimpleObject, Clone, Debug)]
pub struct StringFunctionResult {
    pub result: String,
    pub execution_time_ms: f64,
}

/// Batch string function result
#[derive(SimpleObject, Clone, Debug)]
pub struct BatchStringFunctionResult {
    pub results: Vec<String>,
    pub execution_time_ms: f64,
}
