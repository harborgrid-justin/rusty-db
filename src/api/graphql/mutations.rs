// GraphQL Mutation Operations
//
// Mutation resolvers for the GraphQL API

use async_graphql::{
    Context, Enum, Error, InputObject, Object, Result as GqlResult, SimpleObject, ID,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::error::DbError;
use super::types::*;
use super::models::*;
use super::{GraphQLEngine, AuthorizationContext};

// ============================================================================
// PART 3: MUTATION OPERATIONS (500+ lines)
// ============================================================================

/// Root mutation type
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Insert a single row
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

    /// Insert multiple rows
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

    /// Update a single row by ID
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

    /// Update multiple rows matching a condition
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

    /// Delete a single row by ID
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

    /// Delete multiple rows matching a condition
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

    /// Upsert (insert or update) a row
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

    /// Begin a new transaction
    async fn begin_transaction(
        &self,
        ctx: &Context<'_>,
        isolation_level: Option<IsolationLevel>,
    ) -> GqlResult<TransactionResult> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.begin_transaction(isolation_level).await
    }

    /// Commit a transaction
    async fn commit_transaction(
        &self,
        ctx: &Context<'_>,
        transaction_id: String,
    ) -> GqlResult<TransactionResult> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.commit_transaction(&transaction_id).await
    }

    /// Rollback a transaction
    async fn rollback_transaction(
        &self,
        ctx: &Context<'_>,
        transaction_id: String,
    ) -> GqlResult<TransactionResult> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.rollback_transaction(&transaction_id).await
    }

    /// Execute multiple mutations in a transaction
    async fn execute_transaction(
        &self,
        ctx: &Context<'_>,
        operations: Vec<TransactionOperation>,
        isolation_level: Option<IsolationLevel>,
    ) -> GqlResult<TransactionExecutionResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        match engine.execute_transaction(operations, isolation_level).await {
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

    /// Bulk insert with optimizations
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

        match engine.bulk_insert(&table, data, batch_size.unwrap_or(1000)).await {
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
}

/// Transaction result
#[derive(SimpleObject, Clone, Debug)]
pub struct TransactionResult {
    pub transaction_id: String,
    pub status: String,
    pub timestamp: DateTime,
}

/// Transaction operation input
#[derive(InputObject, Clone, Debug)]
pub struct TransactionOperation {
    pub operation_type: TransactionOpType,
    pub table: String,
    pub data: Option<HashMap<String, Json>>,
    pub where_clause: Option<WhereClause>,
    pub id: Option<ID>,
}

/// Transaction operation type
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum TransactionOpType {
    Insert,
    Update,
    Delete,
}

/// Transaction execution result
#[derive(SimpleObject, Clone, Debug)]
pub struct TransactionExecutionResult {
    pub success: bool,
    pub results: Vec<String>,
    pub execution_time_ms: f64,
    pub error: Option<String>,
}
