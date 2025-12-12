// GraphQL Query Operations
//
// Query resolvers for the GraphQL API

use async_graphql::{Context, Error, ErrorExtensions, InputObject, Object, Result as GqlResult, SimpleObject, ID};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use super::types::{BigInt, Json, JoinType};
use super::models::{DatabaseSchema, TableType, QueryResult, QuerySuccess, QueryError, RowType, AggregateInput, AggregateResult, OrderBy, RowConnection, WhereClause};
use super::monitoring_types::{
    MetricsResponse, SessionStats, QueryStats, PerformanceData, Alert, AlertSeverity,
    ClusterNode, ClusterTopology, ReplicationStatus, StorageStatus, BufferPoolStats,
    Tablespace, Partition, ActiveTransaction, Lock, Deadlock, MvccStatus,
    User, Role, ServerConfig, ServerInfo, ConnectionPool, Connection, Session,
};
use super::GraphQLEngine;
use crate::api::AuthorizationContext;

// ============================================================================
// PART 2: QUERY OPERATIONS (600+ lines)
// ============================================================================

// Root query type
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    // Get all database schemas
    async fn schemas(&self, ctx: &Context<'_>) -> GqlResult<Vec<DatabaseSchema>> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.get_schemas().await
    }

    // Get a specific schema by name
    async fn schema(&self, ctx: &Context<'_>, name: String) -> GqlResult<Option<DatabaseSchema>> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.get_schema(&name).await
    }

    // Get all tables across all schemas
    async fn tables(
        &self,
        ctx: &Context<'_>,
        schema: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> GqlResult<Vec<TableType>> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.get_tables(schema, limit, offset).await
    }

    // Get a specific table by name
    async fn table(
        &self,
        ctx: &Context<'_>,
        name: String,
        schema: Option<String>,
    ) -> GqlResult<Option<TableType>> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.get_table(&name, schema).await
    }

    // Query a table with filtering and pagination
    async fn query_table(
        &self,
        ctx: &Context<'_>,
        table: String,
        where_clause: Option<WhereClause>,
        order_by: Option<Vec<OrderBy>>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> GqlResult<QueryResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        match engine.query_table(&table, where_clause, order_by, limit, offset).await {
            Ok((rows, total_count, has_more)) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(QueryResult::Success(QuerySuccess {
                    rows,
                    total_count: BigInt(total_count),
                    execution_time_ms: execution_time,
                    has_more,
                }))
            }
            Err(e) => Ok(QueryResult::Error(QueryError {
                message: e.to_string(),
                code: "QUERY_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Query multiple tables with joins
    async fn query_tables(
        &self,
        ctx: &Context<'_>,
        tables: Vec<String>,
        joins: Option<Vec<JoinInput>>,
        where_clause: Option<WhereClause>,
        order_by: Option<Vec<OrderBy>>,
        limit: Option<i32>,
    ) -> GqlResult<QueryResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        match engine.query_tables(tables, joins, where_clause, order_by, limit).await {
            Ok((rows, total_count, has_more)) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(QueryResult::Success(QuerySuccess {
                    rows,
                    total_count: BigInt(total_count),
                    execution_time_ms: execution_time,
                    has_more,
                }))
            }
            Err(e) => Ok(QueryResult::Error(QueryError {
                message: e.to_string(),
                code: "QUERY_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Query with cursor-based pagination
    async fn query_table_connection(
        &self,
        ctx: &Context<'_>,
        table: String,
        where_clause: Option<WhereClause>,
        order_by: Option<Vec<OrderBy>>,
        first: Option<i32>,
        after: Option<String>,
        last: Option<i32>,
        before: Option<String>,
    ) -> GqlResult<RowConnection> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.query_table_connection(
            &table,
            where_clause,
            order_by,
            first,
            after,
            last,
            before,
        ).await
    }

    // Get a single row by ID
    async fn row(
        &self,
        ctx: &Context<'_>,
        table: String,
        id: ID,
    ) -> GqlResult<Option<RowType>> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.get_row(&table, &id).await
    }

    // Perform aggregations on a table
    async fn aggregate(
        &self,
        ctx: &Context<'_>,
        table: String,
        aggregates: Vec<AggregateInput>,
        where_clause: Option<WhereClause>,
        group_by: Option<Vec<String>>,
    ) -> GqlResult<Vec<AggregateResult>> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.aggregate(&table, aggregates, where_clause, group_by).await
    }

    // Count rows in a table
    async fn count(
        &self,
        ctx: &Context<'_>,
        table: String,
        where_clause: Option<WhereClause>,
    ) -> GqlResult<BigInt> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        let count = engine.count(&table, where_clause).await?;
        Ok(BigInt(count))
    }

    // Execute a raw SQL query (admin only)
    async fn execute_sql(
        &self,
        ctx: &Context<'_>,
        sql: String,
        params: Option<Vec<Json>>,
    ) -> GqlResult<QueryResult> {
        // Check permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.has_permission("admin.execute_sql")? {
            return Err(Error::new("Permission denied").extend_with(|_, e| {
                e.set("code", "PERMISSION_DENIED");
            }));
        }

        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        match engine.execute_sql(&sql, params).await {
            Ok((rows, total_count)) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(QueryResult::Success(QuerySuccess {
                    rows,
                    total_count: BigInt(total_count),
                    execution_time_ms: execution_time,
                    has_more: false,
                }))
            }
            Err(e) => Ok(QueryResult::Error(QueryError {
                message: e.to_string(),
                code: "SQL_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }

    // Search across multiple tables
    async fn search(
        &self,
        ctx: &Context<'_>,
        query: String,
        tables: Option<Vec<String>>,
        fields: Option<Vec<String>>,
        limit: Option<i32>,
    ) -> GqlResult<SearchResult> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.search(&query, tables, fields, limit).await
    }

    // Get query execution plan
    async fn explain(
        &self,
        ctx: &Context<'_>,
        table: String,
        where_clause: Option<WhereClause>,
        order_by: Option<Vec<OrderBy>>,
    ) -> GqlResult<QueryPlan> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.explain(&table, where_clause, order_by).await
    }

    // ========================================================================
    // ADVANCED QUERY OPERATIONS
    // ========================================================================

    // Execute UNION query
    async fn execute_union(
        &self,
        ctx: &Context<'_>,
        queries: Vec<String>,
        union_all: Option<bool>,
    ) -> GqlResult<QueryResult> {
        let start = Instant::now();
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        match engine.execute_union(queries, union_all.unwrap_or(false)).await {
            Ok((rows, total_count)) => {
                let execution_time = start.elapsed().as_secs_f64() * 1000.0;
                Ok(QueryResult::Success(QuerySuccess {
                    rows,
                    total_count: BigInt(total_count),
                    execution_time_ms: execution_time,
                    has_more: false,
                }))
            }
            Err(e) => Ok(QueryResult::Error(QueryError {
                message: e.to_string(),
                code: "UNION_ERROR".to_string(),
                details: Some(format!("{:?}", e)),
            })),
        }
    }
}

// Input type for join operations
#[derive(InputObject, Clone, Debug)]
pub struct JoinInput {
    pub table: String,
    pub join_type: JoinType,
    pub on_field: String,
    pub other_field: String,
}

// Search result with highlighting
#[derive(SimpleObject, Clone, Debug)]
pub struct SearchResult {
    pub results: Vec<SearchMatch>,
    pub total_count: BigInt,
    pub execution_time_ms: f64,
}

// Individual search match
#[derive(SimpleObject, Clone, Debug)]
pub struct SearchMatch {
    pub table: String,
    pub row: RowType,
    pub score: f64,
    pub highlights: HashMap<String, String>,
}

// Query execution plan
#[derive(SimpleObject, Clone, Debug)]
pub struct QueryPlan {
    pub plan_text: String,
    pub estimated_cost: f64,
    pub estimated_rows: BigInt,
    pub operations: Vec<PlanOperation>,
}

// Individual operation in query plan
#[derive(SimpleObject, Clone, Debug)]
pub struct PlanOperation {
    pub operation_type: String,
    pub description: String,
    pub cost: f64,
    pub rows: BigInt,
    pub children: Vec<PlanOperation>,
}
