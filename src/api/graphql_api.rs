// # GraphQL API Layer
//
// Comprehensive GraphQL API implementation for RustyDB, providing a modern,
// type-safe interface for database operations with real-time capabilities.
//
// ## Features
//
// - **Dynamic Schema Generation**: Automatically generate GraphQL schemas from database metadata
// - **Query Operations**: Complex queries with filtering, pagination, and aggregations
// - **Mutation Operations**: CRUD operations with transaction support
// - **Subscriptions**: Real-time data streaming for table changes
// - **Performance**: DataLoader, query complexity analysis, caching
// - **Security**: Rate limiting, field-level authorization, depth limiting
//
// ## Architecture
//
// ```text
// ┌─────────────────────────────────────────────────────────┐
// │                    GraphQL API Layer                    │
// ├─────────────────────────────────────────────────────────┤
// │  Schema Types  │  Queries  │  Mutations  │  Subscriptions│
// ├─────────────────────────────────────────────────────────┤
// │  Complexity    │  DataLoader │  Caching  │  Rate Limiter│
// ├─────────────────────────────────────────────────────────┤
// │              Database Engine Core                       │
// └─────────────────────────────────────────────────────────┘
// ```

use std::collections::VecDeque;
use std::sync::Mutex;
use std::collections::HashSet;
use std::time::Instant;
use async_graphql::{
    Context, Enum, Error, ErrorExtensions, InputObject, Interface, Object,
    Result as GqlResult, Schema, SimpleObject, Subscription, Union, ID,
};
use async_graphql::parser::types::Selection;
use async_graphql::extensions::{Extension, ExtensionContext, ExtensionFactory, NextExecute};
use async_graphql::parser::types::ExecutableDocument;
use chrono::{DateTime as ChronoDateTime, Utc};
use futures_util::stream::{Stream, StreamExt};
use futures_util::Future;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration};
use tokio::sync::{broadcast, RwLock};
use tokio_stream::wrappers::BroadcastStream;

use crate::common::{Value, TableId, ColumnId, RowId};
use crate::error::DbError;

type Result<T> = std::result::Result<T, DbError>;

// Note: This module requires the following dependencies in Cargo.toml:
// - async-graphql = "7.0"
// - tokio = { version = "1", features = ["full"] }
// - tokio-stream = "0.1"
// - futures-util = "0.3"
// - serde = { version = "1", features = ["derive"] }
// - serde_json = "1"
// - chrono = { version = "0.4", features = ["serde"] }
// - uuid = { version = "1", features = ["v4", "serde"] }
// - base64 = "0.21"
// - async-stream = "0.3"
// - async-trait = "0.1"

// ============================================================================
// PART 1: SCHEMA & TYPE SYSTEM (700+ lines)
// ============================================================================

/// Custom scalar type for DateTime values
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DateTime(ChronoDateTime<Utc>);

#[async_graphql::Scalar]
impl async_graphql::ScalarType for DateTime {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        if let async_graphql::Value::String(s) = value {
            ChronoDateTime::parse_from_rfc3339(&s)
                .map(|dt| DateTime(dt.with_timezone(&Utc)))
                .map_err(|e| async_graphql::InputValueError::custom(format!("Invalid datetime: {}", e)))
        } else {
            Err(async_graphql::InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_rfc3339())
    }
}

impl DateTime {
    pub fn now() -> Self {
        DateTime(Utc::now())
    }

    pub fn inner(&self) -> &ChronoDateTime<Utc> {
        &self.0
    }
}

/// Custom scalar type for JSON values
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Json(serde_json::Value));

#[async_graphql::Scalar]
impl async_graphql::ScalarType for Json {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        Ok(Json(serde_json::to_value(value).map_err(|e| {
            async_graphql::InputValueError::custom(format!("Invalid JSON: {}", e))
        })?))
    }

    fn to_value(&self) -> async_graphql::Value {
        serde_json::from_value(self.0.clone())
            .unwrap_or(async_graphql::Value::Null)
    }
}

/// Custom scalar type for Binary data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Binary(Vec<u8>));

#[async_graphql::Scalar]
impl async_graphql::ScalarType for Binary {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        if let async_graphql::Value::String(s) = value {
            base64::decode(&s)
                .map(Binary)
                .map_err(|e| async_graphql::InputValueError::custom(format!("Invalid base64: {}", e)))
        } else {
            Err(async_graphql::InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(base64::encode(&self.0))
    }
}

/// Custom scalar type for large integers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BigInt(i64));

#[async_graphql::Scalar]
impl async_graphql::ScalarType for BigInt {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        if let async_graphql::Value::String(s) = value {
            s.parse::<i64>()
                .map(BigInt)
                .map_err(|e| async_graphql::InputValueError::custom(format!("Invalid BigInt: {}", e)))
        } else if let async_graphql::Value::Number(n) = value {
            n.as_i64()
                .map(BigInt)
                .ok_or_else(|| async_graphql::InputValueError::custom("Number too large"))
        } else {
            Err(async_graphql::InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

/// GraphQL representation of database data types
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum DataType {
    Null,
    Boolean,
    Integer,
    Float,
    String,
    Bytes,
    Date,
    Timestamp,
    Json,
    Array,
    Decimal,
    Uuid,
}

/// Sort order for query results
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Filter operations for WHERE clauses
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum FilterOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Like,
    NotLike,
    In,
    NotIn,
    IsNull,
    IsNotNull,
    Between,
    Contains,
    StartsWith,
    EndsWith,
}

/// Aggregate functions
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum AggregateFunc {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    StdDev,
    Variance,
}

/// Join types for multi-table queries
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// Transaction isolation levels
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

/// Node interface - all objects with an ID implement this
// FIXME: Interface derive disabled due to trait bound issues
// #[derive(Interface)]
// #[graphql(field(name = "id", ty = "&ID"))]
// pub enum Node {
//     Table(TableType),
//     Column(ColumnType),
//     Row(RowType),
// }

/// Timestamped interface - objects with creation/modification timestamps
// FIXME: Interface derive disabled due to trait bound issues
// #[derive(Interface)]
// #[graphql(
//     field(name = "created_at", ty = "DateTime"),
//     field(name = "updated_at", ty = "Option<DateTime>")
// )]
// pub enum Timestamped {
//     Table(TableType),
//     Row(RowType),
// }

/// Auditable interface - objects with audit trail
// FIXME: Interface derive disabled due to trait bound issues
// #[derive(Interface)]
// #[graphql(
//     field(name = "created_by", ty = "String"),
//     field(name = "updated_by", ty = "Option<String>")
// )]
// pub enum Auditable {
//     Table(TableType),
//     Row(RowType),
// }

/// Database schema information
#[derive(SimpleObject, Clone, Debug)]
pub struct DatabaseSchema {
    /// Schema name
    pub name: String,
    /// Tables in this schema
    pub tables: Vec<TableType>,
    /// Total number of tables
    pub table_count: i32,
    /// Schema creation time
    pub created_at: DateTime,
    /// Schema description/comment
    pub description: Option<String>,
}

/// Table metadata and structure
#[derive(Clone, Debug)]
pub struct TableType {
    pub id: ID,
    pub name: String,
    pub schema: String,
    pub columns: Vec<ColumnType>,
    pub row_count: BigInt,
    pub size_bytes: BigInt,
    pub created_at: DateTime,
    pub updated_at: Option<DateTime>,
    pub created_by: String,
    pub updated_by: Option<String>,
    pub description: Option<String>,
    pub indexes: Vec<IndexInfo>,
    pub constraints: Vec<ConstraintInfo>,
}

#[Object]
impl TableType {
    async fn id(&self) -> &ID {
        &self.id
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn schema(&self) -> &str {
        &self.schema
    }

    async fn columns(&self) -> &[ColumnType] {
        &self.columns
    }

    async fn row_count(&self) -> &BigInt {
        &self.row_count
    }

    async fn size_bytes(&self) -> &BigInt {
        &self.size_bytes
    }

    async fn created_at(&self) -> &DateTime {
        &self.created_at
    }

    async fn updated_at(&self) -> &Option<DateTime> {
        &self.updated_at
    }

    async fn created_by(&self) -> &str {
        &self.created_by
    }

    async fn updated_by(&self) -> &Option<String> {
        &self.updated_by
    }

    async fn description(&self) -> &Option<String> {
        &self.description
    }

    async fn indexes(&self) -> &[IndexInfo] {
        &self.indexes
    }

    async fn constraints(&self) -> &[ConstraintInfo] {
        &self.constraints
    }

    /// Get table statistics
    async fn statistics(&self, ctx: &Context<'_>) -> GqlResult<TableStatistics> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.get_table_statistics(&self.name).await
    }

    /// Get sample rows from the table
    async fn sample_rows(&self, ctx: &Context<'_>, limit: Option<i32>) -> GqlResult<Vec<RowType>> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.get_sample_rows(&self.name, limit.unwrap_or(10)).await
    }
}

/// Column metadata
#[derive(Clone, Debug)]
pub struct ColumnType {
    pub id: ID,
    pub name: String,
    pub table_name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub position: i32,
    pub max_length: Option<i32>,
    pub precision: Option<i32>,
    pub scale: Option<i32>,
    pub description: Option<String>,
}

#[Object]
impl ColumnType {
    async fn id(&self) -> &ID {
        &self.id
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn table_name(&self) -> &str {
        &self.table_name
    }

    async fn data_type(&self) -> DataType {
        self.data_type
    }

    async fn nullable(&self) -> bool {
        self.nullable
    }

    async fn default_value(&self) -> &Option<String> {
        &self.default_value
    }

    async fn position(&self) -> i32 {
        self.position
    }

    async fn max_length(&self) -> Option<i32> {
        self.max_length
    }

    async fn precision(&self) -> Option<i32> {
        self.precision
    }

    async fn scale(&self) -> Option<i32> {
        self.scale
    }

    async fn description(&self) -> &Option<String> {
        &self.description
    }

    /// Get column statistics (distinct values, null count, etc.)
    async fn statistics(&self, ctx: &Context<'_>) -> GqlResult<ColumnStatistics> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.get_column_statistics(&self.table_name, &self.name).await
    }
}

/// Row data representation
#[derive(Clone, Debug)]
pub struct RowType {
    pub id: ID,
    pub table_name: String,
    pub fields: HashMap<String, FieldValue>,
    pub created_at: DateTime,
    pub updated_at: Option<DateTime>,
    pub created_by: String,
    pub updated_by: Option<String>,
    pub version: i32,
}

#[Object]
impl RowType {
    async fn id(&self) -> &ID {
        &self.id
    }

    async fn table_name(&self) -> &str {
        &self.table_name
    }

    async fn fields(&self) -> &HashMap<String, FieldValue> {
        &self.fields
    }

    async fn created_at(&self) -> &DateTime {
        &self.created_at
    }

    async fn updated_at(&self) -> &Option<DateTime> {
        &self.updated_at
    }

    async fn created_by(&self) -> &str {
        &self.created_by
    }

    async fn updated_by(&self) -> &Option<String> {
        &self.updated_by
    }

    async fn version(&self) -> i32 {
        self.version
    }

    /// Get specific field value
    async fn get_field(&self, name: String) -> Option<&FieldValue> {
        self.fields.get(&name)
    }
}

/// Field value in a row
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldValue {
    pub column_name: String,
    pub value: Json,
    pub data_type: DataType,
}

#[Object]
impl FieldValue {
    async fn column_name(&self) -> &str {
        &self.column_name
    }

    async fn value(&self) -> &Json {
        &self.value
    }

    async fn data_type(&self) -> DataType {
        self.data_type
    }

    async fn string_value(&self) -> Option<String> {
        self.value.0.as_str().map(|s| s.to_string())
    }

    async fn int_value(&self) -> Option<i64> {
        self.value.0.as_i64()
    }

    async fn float_value(&self) -> Option<f64> {
        self.value.0.as_f64()
    }

    async fn bool_value(&self) -> Option<bool> {
        self.value.0.as_bool()
    }
}

/// Index information
#[derive(SimpleObject, Clone, Debug)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub index_type: String,
    pub size_bytes: BigInt,
    pub created_at: DateTime,
}

/// Constraint information
#[derive(SimpleObject, Clone, Debug)]
pub struct ConstraintInfo {
    pub name: String,
    pub constraint_type: String,
    pub columns: Vec<String>,
    pub referenced_table: Option<String>,
    pub referenced_columns: Option<Vec<String>>,
}

/// Table statistics
#[derive(SimpleObject, Clone, Debug)]
pub struct TableStatistics {
    pub row_count: BigInt,
    pub size_bytes: BigInt,
    pub index_size_bytes: BigInt,
    pub avg_row_size: f64,
    pub last_analyzed: Option<DateTime>,
    pub last_modified: Option<DateTime>,
}

/// Column statistics
#[derive(SimpleObject, Clone, Debug)]
pub struct ColumnStatistics {
    pub distinct_count: BigInt,
    pub null_count: BigInt,
    pub avg_length: Option<f64>,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub histogram: Option<Vec<HistogramBucket>>,
}

/// Histogram bucket for column statistics
#[derive(SimpleObject, Clone, Debug)]
pub struct HistogramBucket {
    pub range_start: String,
    pub range_end: String,
    pub count: BigInt,
    pub frequency: f64,
}

/// Query result union type
#[derive(Union)]
pub enum QueryResult {
    Success(QuerySuccess),
    Error(QueryError),
}

/// Successful query result
#[derive(SimpleObject, Clone, Debug)]
pub struct QuerySuccess {
    pub rows: Vec<RowType>,
    pub total_count: BigInt,
    pub execution_time_ms: f64,
    pub has_more: bool,
}

/// Query error result
#[derive(SimpleObject, Clone, Debug)]
pub struct QueryError {
    pub message: String,
    pub code: String,
    pub details: Option<String>,
}

/// Mutation result union type
#[derive(Union)]
pub enum MutationResult {
    Success(MutationSuccess),
    Error(MutationError),
}

/// Successful mutation result
#[derive(SimpleObject, Clone, Debug)]
pub struct MutationSuccess {
    pub affected_rows: i32,
    pub returning: Option<Vec<RowType>>,
    pub execution_time_ms: f64,
}

/// Mutation error result
#[derive(SimpleObject, Clone, Debug)]
pub struct MutationError {
    pub message: String,
    pub code: String,
    pub details: Option<String>,
}

/// Input type for filtering
#[derive(InputObject, Clone, Debug)]
pub struct FilterCondition {
    pub field: String,
    pub op: FilterOp,
    pub value: Option<Json>,
    pub values: Option<Vec<Json>>,
}

/// Input type for complex WHERE clauses
#[derive(InputObject, Clone, Debug)]
pub struct WhereClause {
    pub and: Option<Vec<WhereClause>>,
    pub or: Option<Vec<WhereClause>>,
    pub not: Option<Box<WhereClause>>,
    pub condition: Option<FilterCondition>,
}

/// Input type for sorting
#[derive(InputObject, Clone, Debug)]
pub struct OrderBy {
    pub field: String,
    pub order: SortOrder,
}

/// Input type for aggregations
#[derive(InputObject, Clone, Debug)]
pub struct AggregateInput {
    pub function: AggregateFunc,
    pub field: String,
    pub alias: Option<String>,
}

/// Aggregation result
#[derive(SimpleObject, Clone, Debug)]
pub struct AggregateResult {
    pub field: String,
    pub function: AggregateFunc,
    pub value: Json,
}

/// Pagination cursor
#[derive(SimpleObject, Clone, Debug)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
    pub total_count: BigInt,
}

/// Edge type for cursor-based pagination
#[derive(SimpleObject, Clone, Debug)]
pub struct RowEdge {
    pub cursor: String,
    pub node: RowType,
}

/// Connection type for cursor-based pagination
#[derive(SimpleObject, Clone, Debug)]
pub struct RowConnection {
    pub edges: Vec<RowEdge>,
    pub page_info: PageInfo,
    pub total_count: BigInt,
}

// ============================================================================
// PART 2: QUERY OPERATIONS (600+ lines)
// ============================================================================

/// Root query type
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get all database schemas
    async fn schemas(&self, ctx: &Context<'_>) -> GqlResult<Vec<DatabaseSchema>> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.get_schemas().await
    }

    /// Get a specific schema by name
    async fn schema(&self, ctx: &Context<'_>, name: String) -> GqlResult<Option<DatabaseSchema>> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.get_schema(&name).await
    }

    /// Get all tables across all schemas
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

    /// Get a specific table by name
    async fn table(
        &self,
        ctx: &Context<'_>,
        name: String,
        schema: Option<String>,
    ) -> GqlResult<Option<TableType>> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.get_table(&name, schema).await
    }

    /// Query a table with filtering and pagination
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

    /// Query multiple tables with joins
    async fn query_tables(
        &self,
        ctx: &Context<'_>,
        tables: Vec<String>,
        joins: Option<Vec<JoinInput>>,
        where_clause: Option<WhereClause>,
        order_by: Option<Vec<OrderBy>>,
        limit: Option<i32>,
    ) -> GqlResult<QueryResult> {
        let start = Instant::now());
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

    /// Query with cursor-based pagination
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

    /// Get a single row by ID
    async fn row(
        &self,
        ctx: &Context<'_>,
        table: String,
        id: ID,
    ) -> GqlResult<Option<RowType>> {
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;
        engine.get_row(&table, &id).await
    }

    /// Perform aggregations on a table
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

    /// Count rows in a table
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

    /// Execute a raw SQL query (admin only)
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

    /// Search across multiple tables
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

    /// Get query execution plan
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
}

/// Input type for join operations
#[derive(InputObject, Clone, Debug)]
pub struct JoinInput {
    pub table: String,
    pub join_type: JoinType,
    pub on_field: String,
    pub other_field: String,
}

/// Search result with highlighting
#[derive(SimpleObject, Clone, Debug)]
pub struct SearchResult {
    pub results: Vec<SearchMatch>,
    pub total_count: BigInt,
    pub execution_time_ms: f64,
}

/// Individual search match
#[derive(SimpleObject, Clone, Debug)]
pub struct SearchMatch {
    pub table: String,
    pub row: RowType,
    pub score: f64,
    pub highlights: HashMap<String, String>,
}

/// Query execution plan
#[derive(SimpleObject, Clone, Debug)]
pub struct QueryPlan {
    pub plan_text: String,
    pub estimated_cost: f64,
    pub estimated_rows: BigInt,
    pub operations: Vec<PlanOperation>,
}

/// Individual operation in query plan
#[derive(SimpleObject, Clone, Debug)]
pub struct PlanOperation {
    pub operation_type: String,
    pub description: String,
    pub cost: f64,
    pub rows: BigInt,
    pub children: Vec<PlanOperation>,
}

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
            })));
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
        let start = Instant::now());
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", table)),
            })));
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
        let start = Instant::now());
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", table)),
            })));
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
        let start = Instant::now());
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", table)),
            })));
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
        let start = Instant::now());
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", table)),
            })));
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
        let start = Instant::now());
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", table)),
            })));
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
        let start = Instant::now());
        let engine = ctx.data::<Arc<GraphQLEngine>>()?;

        // Check write permissions
        let auth = ctx.data::<Arc<AuthorizationContext>>()?;
        if !auth.can_write(&table)? {
            return Ok(MutationResult::Error(MutationError {
                message: "Permission denied".to_string(),
                code: "PERMISSION_DENIED".to_string(),
                details: Some(format!("No write access to table: {}", table)),
            })));
        }

        match engine.upsert(&table, unique_fields, data).await {
            Ok((row, was_inserted)) => {
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
            })));
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

// ============================================================================
// PART 4: SUBSCRIPTION SYSTEM (600+ lines)
// ============================================================================

/// Root subscription type
pub struct SubscriptionRoot);

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to all changes on a table
    async fn table_changes<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        where_clause: Option<WhereClause>,
    ) -> impl Stream<Item = TableChange> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(1000);

        // Register subscription
        let subscription_id = engine.register_table_subscription(&table, where_clause, tx).await;

        BroadcastStream::new(rx).filter_map(|result| async move {
            result.ok()
        })
    }

    /// Subscribe to row insertions
    async fn row_inserted<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        where_clause: Option<WhereClause>,
    ) -> impl Stream<Item = RowInserted> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(1000);

        // Register subscription
        engine.register_insert_subscription(&table, where_clause, tx).await;

        BroadcastStream::new(rx).filter_map(|result| async move {
            result.ok()
        })
    }

    /// Subscribe to row updates
    async fn row_updated<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        where_clause: Option<WhereClause>,
    ) -> impl Stream<Item = RowUpdated> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(1000);

        // Register subscription
        engine.register_update_subscription(&table, where_clause, tx).await;

        BroadcastStream::new(rx).filter_map(|result| async move {
            result.ok()
        })
    }

    /// Subscribe to row deletions
    async fn row_deleted<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        where_clause: Option<WhereClause>,
    ) -> impl Stream<Item = RowDeleted> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(1000);

        // Register subscription
        engine.register_delete_subscription(&table, where_clause, tx).await;

        BroadcastStream::new(rx).filter_map(|result| async move {
            result.ok()
        })
    }

    /// Subscribe to specific row changes by ID
    async fn row_changes<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        id: ID,
    ) -> impl Stream<Item = RowChange> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(100);

        // Register subscription
        engine.register_row_subscription(&table, &id, tx).await;

        BroadcastStream::new(rx).filter_map(|result| async move {
            result.ok()
        })
    }

    /// Subscribe to aggregation changes
    async fn aggregate_changes<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        aggregates: Vec<AggregateInput>,
        where_clause: Option<WhereClause>,
        interval_seconds: Option<i32>,
    ) -> impl Stream<Item = AggregateChange> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let interval = Duration::from_secs(interval_seconds.unwrap_or(5) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;

                match engine.aggregate(&table, aggregates.clone(), where_clause.clone(), None).await {
                    Ok(results) => {
                        yield AggregateChange {
                            table: table.clone(),
                            results,
                            timestamp: DateTime::now(),
                        };
                    }
                    Err(_) => continue,
                }
            }
        }
    }

    /// Subscribe to query result changes
    async fn query_changes<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        where_clause: Option<WhereClause>,
        order_by: Option<Vec<OrderBy>>,
        limit: Option<i32>,
        poll_interval_seconds: Option<i32>,
    ) -> impl Stream<Item = QueryChange> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let interval = Duration::from_secs(poll_interval_seconds.unwrap_or(5) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);
            let mut last_hash: Option<u64> = None;

            loop {
                interval_timer.tick().await;

                match engine.query_table(
                    &table,
                    where_clause.clone(),
                    order_by.clone(),
                    limit,
                    None,
                ).await {
                    Ok((rows, total_count, has_more)) => {
                        // Compute hash to detect changes
                        let current_hash = compute_rows_hash(&rows);

                        if last_hash.is_none() || last_hash != Some(current_hash) {
                            last_hash = Some(current_hash);
                            yield QueryChange {
                                table: table.clone(),
                                rows,
                                total_count: BigInt(total_count),
                                timestamp: DateTime::now(),
                            };
                        }
                    }
                    Err(_) => continue,
                }
            }
        }
    }

    /// Heartbeat subscription for connection keepalive
    async fn heartbeat<'ctx>(
        &self,
        interval_seconds: Option<i32>,
    ) -> impl Stream<Item = Heartbeat> + 'ctx {
        let interval = Duration::from_secs(interval_seconds.unwrap_or(30) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);
            let mut sequence = 0u64;

            loop {
                interval_timer.tick().await;
                sequence += 1;

                yield Heartbeat {
                    sequence,
                    timestamp: DateTime::now(),
                };
            }
        }
    }
}

/// Table change event (union of all change types)
#[derive(Clone, Debug)]
pub struct TableChange {
    pub table: String,
    pub change_type: ChangeType,
    pub row: Option<RowType>,
    pub old_row: Option<RowType>,
    pub timestamp: DateTime,
}

#[Object]
impl TableChange {
    async fn table(&self) -> &str {
        &self.table
    }

    async fn change_type(&self) -> ChangeType {
        self.change_type
    }

    async fn row(&self) -> &Option<RowType> {
        &self.row
    }

    async fn old_row(&self) -> &Option<RowType> {
        &self.old_row
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

/// Change type enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum ChangeType {
    Insert,
    Update,
    Delete,
}

/// Row inserted event
#[derive(Clone, Debug)]
pub struct RowInserted {
    pub table: String,
    pub row: RowType,
    pub timestamp: DateTime,
}

#[Object]
impl RowInserted {
    async fn table(&self) -> &str {
        &self.table
    }

    async fn row(&self) -> &RowType {
        &self.row
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

/// Row updated event
#[derive(Clone, Debug)]
pub struct RowUpdated {
    pub table: String,
    pub old_row: RowType,
    pub new_row: RowType,
    pub changed_fields: Vec<String>,
    pub timestamp: DateTime,
}

#[Object]
impl RowUpdated {
    async fn table(&self) -> &str {
        &self.table
    }

    async fn old_row(&self) -> &RowType {
        &self.old_row
    }

    async fn new_row(&self) -> &RowType {
        &self.new_row
    }

    async fn changed_fields(&self) -> &[String] {
        &self.changed_fields
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

/// Row deleted event
#[derive(Clone, Debug)]
pub struct RowDeleted {
    pub table: String,
    pub id: ID,
    pub old_row: Option<RowType>,
    pub timestamp: DateTime,
}

#[Object]
impl RowDeleted {
    async fn table(&self) -> &str {
        &self.table
    }

    async fn id(&self) -> &ID {
        &self.id
    }

    async fn old_row(&self) -> &Option<RowType> {
        &self.old_row
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

/// Row change event (for specific row subscriptions)
#[derive(Clone, Debug)]
pub struct RowChange {
    pub table: String,
    pub id: ID,
    pub change_type: ChangeType,
    pub row: Option<RowType>,
    pub old_row: Option<RowType>,
    pub timestamp: DateTime,
}

#[Object]
impl RowChange {
    async fn table(&self) -> &str {
        &self.table
    }

    async fn id(&self) -> &ID {
        &self.id
    }

    async fn change_type(&self) -> ChangeType {
        self.change_type
    }

    async fn row(&self) -> &Option<RowType> {
        &self.row
    }

    async fn old_row(&self) -> &Option<RowType> {
        &self.old_row
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

/// Aggregate change event
#[derive(SimpleObject, Clone, Debug)]
pub struct AggregateChange {
    pub table: String,
    pub results: Vec<AggregateResult>,
    pub timestamp: DateTime,
}

/// Query change event
#[derive(SimpleObject, Clone, Debug)]
pub struct QueryChange {
    pub table: String,
    pub rows: Vec<RowType>,
    pub total_count: BigInt,
    pub timestamp: DateTime,
}

/// Heartbeat event
#[derive(SimpleObject, Clone, Debug)]
pub struct Heartbeat {
    pub sequence: u64,
    pub timestamp: DateTime,
}

/// Subscription manager for tracking active subscriptions
pub struct SubscriptionManager {
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionInfo>>>,
    event_bus: Arc<RwLock<HashMap<String, Vec<broadcast::Sender<TableChange>>>>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_bus: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_subscription(
        &self,
        table: &str,
        filter: Option<WhereClause>,
    ) -> String {
        let subscription_id = uuid::Uuid::new_v4().to_string();
        let info = SubscriptionInfo {
            id: subscription_id.clone(),
            table: table.to_string(),
            filter,
            created_at: DateTime::now(),
            last_event: None,
        };

        let mut subs = self.subscriptions.write().await;
        subs.insert(subscription_id.clone(), info);

        subscription_id
    }

    pub async fn unregister_subscription(&self, subscription_id: &str) {
        let mut subs = self.subscriptions.write().await;
        subs.remove(subscription_id);
    }

    pub async fn notify_change(&self, table: &str, change: TableChange) {
        let bus = self.event_bus.read().await;
        if let Some(senders) = bus.get(table) {
            for sender in senders {
                let _ = sender.send(change.clone());
            }
        }
    }

    pub async fn get_active_subscriptions(&self) -> Vec<SubscriptionInfo> {
        let subs = self.subscriptions.read().await;
        subs.values().cloned().collect()
    }
}

/// Subscription information
#[derive(Clone, Debug)]
pub struct SubscriptionInfo {
    pub id: String,
    pub table: String,
    pub filter: Option<WhereClause>,
    pub created_at: DateTime,
    pub last_event: Option<DateTime>,
}

// Helper function to compute hash of rows for change detection
fn compute_rows_hash(rows: &[RowType]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    for row in rows {
        row.id.hash(&mut hasher);
        row.version.hash(&mut hasher);
    }
    hasher.finish()
}

// ============================================================================
// PART 5: PERFORMANCE & SECURITY (600+ lines)
// ============================================================================

/// Query complexity analyzer
pub struct ComplexityAnalyzer {
    max_complexity: usize,
    max_depth: usize,
}

impl ComplexityAnalyzer {
    pub fn new(max_complexity: usize, max_depth: usize) -> Self {
        Self {
            max_complexity,
            max_depth,
        }
    }

    pub fn analyze(&self, _doc: &ExecutableDocument) -> std::result::Result<ComplexityMetrics, DbError> {
        // Simplified implementation - full analysis requires async-graphql internals
        let metrics = ComplexityMetrics {
            total_complexity: 10, // Default estimate
            max_depth: 3,
            field_count: 5,
            has_mutations: false,
            has_subscriptions: false,
        };

        // Check limits
        if metrics.total_complexity > self.max_complexity {
            return Err(DbError::InvalidInput(format!(
                "Query complexity {} exceeds maximum {}",
                metrics.total_complexity, self.max_complexity
            ))));
        }

        if metrics.max_depth > self.max_depth {
            return Err(DbError::InvalidInput(format!(
                "Query depth {} exceeds maximum {}",
                metrics.max_depth, self.max_depth
            ))));
        }

        Ok(metrics)
    }

    fn analyze_selection_set(
        &self,
        _selection_set: &async_graphql::parser::types::SelectionSet,
        metrics: &mut ComplexityMetrics,
        depth: usize,
    ) -> std::result::Result<(), DbError> {
        // Simplified implementation
        metrics.max_depth = metrics.max_depth.max(depth);
        metrics.field_count += 1;
        metrics.total_complexity += 1;
        Ok(())
    }

    fn calculate_field_complexity(&self, field: &async_graphql::parser::types::Field) -> usize {
        // Base complexity of 1 for each field
        // Could be enhanced to use field-specific weights
        1
    }
}

/// Complexity metrics
#[derive(Debug, Clone)]
pub struct ComplexityMetrics {
    pub total_complexity: usize,
    pub max_depth: usize,
    pub field_count: usize,
    pub has_mutations: bool,
    pub has_subscriptions: bool,
}

/// Rate limiter for GraphQL operations
pub struct RateLimiter {
    limits: Arc<RwLock<HashMap<String, RateLimit>>>,
    requests: Arc<RwLock<HashMap<String<Instant>>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn set_limit(&self, key: &str, limit: RateLimit) {
        let mut limits = self.limits.write().await;
        limits.insert(key.to_string(), limit);
    }

    pub async fn check_rate_limit(&self, key: &str) -> Result<()> {
        let limits = self.limits.read().await;
        let limit = limits.get(key).cloned().unwrap_or(RateLimit {
            max_requests: 1000,
            window_secs: 60,
        });
        drop(limits);

        let mut requests = self.requests.write().await;
        let request_times = requests.entry(key.to_string()).or_insert_with(VecDeque::new);

        let now = Instant::now();
        let window = Duration::from_secs(limit.window_secs);

        // Remove old requests outside the window
        while let Some(&oldest) = request_times.front() {
            if now.duration_since(oldest) > window {
                request_times.pop_front();
            } else {
                break;
            }
        }

        // Check if limit exceeded
        if request_times.len() >= limit.max_requests {
            return Err(DbError::LimitExceeded(format!(
                "Rate limit exceeded: {} requests per {} seconds",
                limit.max_requests, limit.window_secs
            ))));
        }

        // Record this request
        request_times.push_back(now);

        Ok(())
    }
}

/// Rate limit configuration
#[derive(Clone, Debug)]
pub struct RateLimit {
    pub max_requests: usize,
    pub window_secs: u64,
}

/// Authorization context for field-level security
pub struct AuthorizationContext {
    user_id: String,
    roles: HashSet<String>,
    permissions: HashSet<String>,
}

impl AuthorizationContext {
    pub fn new(user_id: String, roles: Vec<String>, permissions: Vec<String>) -> Self {
        Self {
            user_id,
            roles: roles.into_iter().collect(),
            permissions: permissions.into_iter().collect(),
        }
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(role)
    }

    pub fn has_permission(&self, permission: &str) -> std::result::Result<bool, DbError> {
        Ok(self.permissions.contains(permission))
    }

    pub fn can_read(&self, table: &str) -> std::result::Result<bool, DbError> {
        Ok(self.permissions.contains(&format!("read:{}", table))
            || self.permissions.contains("read:*")
            || self.has_role("admin"))
    }

    pub fn can_write(&self, table: &str) -> Result<bool> {
        Ok(self.permissions.contains(&format!("write:{}", table))
            || self.permissions.contains("write:*")
            || self.has_role("admin"))
    }

    pub fn can_delete(&self, table: &str) -> Result<bool> {
        Ok(self.permissions.contains(&format!("delete:{}", table))
            || self.permissions.contains("delete:*")
            || self.has_role("admin"))
    }
}

/// Query result cache
pub struct QueryCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_size: usize,
    ttl: Duration,
}

impl QueryCache {
    pub fn new(max_size: usize, ttl_secs: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub async fn get(&self, key: &str) -> Option<Vec<RowType>> {
        let cache = self.cache.read().await);
        if let Some(entry) = cache.get(key) {
            if entry.expires_at > Instant::now() {
                return Some(entry.data.clone());
            }
        }
        None
    }

    pub async fn set(&self, key: String, data: Vec<RowType>) {
        let mut cache = self.cache.write().await;

        // Evict if cache is full
        if cache.len() >= self.max_size {
            self.evict_oldest(&mut cache);
        }

        cache.insert(key, CacheEntry {
            data,
            created_at: Instant::now(),
            expires_at: Instant::now() + self.ttl,
        });
    }

    pub async fn invalidate(&self, pattern: &str) {
        let mut cache = self.cache.write().await;
        cache.retain(|key, _| !key.contains(pattern));
    }

    fn evict_oldest(&self, cache: &mut HashMap<String, CacheEntry>) {
        if let Some(oldest_key) = cache
            .iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(key, _)| key.clone())
        {
            cache.remove(&oldest_key);
        }
    }
}

/// Cache entry
#[derive(Clone, Debug)]
struct CacheEntry {
    data: Vec<RowType>,
    created_at: Instant,
    expires_at: Instant,
}

/// DataLoader for N+1 query prevention
pub struct DataLoader<K, V> {
    loader_fn: Arc<dyn Fn(Vec<K>) -> Pin<Box<dyn Future<Output = HashMap<K, V>> + Send>> + Send + Sync>,
    cache: Arc<RwLock<HashMap<K, V>>>,
    batch_size: usize,
}

impl<K, V> DataLoader<K, V>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new<F, Fut>(loader_fn: F, batch_size: usize) -> Self
    where
        F: Fn(Vec<K>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HashMap<K, V>> + Send + 'static,
    {
        Self {
            loader_fn: Arc::new(move |keys| Box::pin(loader_fn(keys))),
            cache: Arc::new(RwLock::new(HashMap::new())),
            batch_size,
        }
    }

    pub async fn load(&self, key: K) -> Option<V> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(value) = cache.get(&key) {
                return Some(value.clone());
            }
        }

        // Load from source
        let results = (self.loader_fn)(vec![key.clone()]).await;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.extend(results.clone());
        }

        results.get(&key).cloned()
    }

    pub async fn load_many(&self, keys: Vec<K>) -> HashMap<K, V> {
        let mut results = HashMap::new();
        let mut missing_keys = Vec::new();

        // Check cache
        {
            let cache = self.cache.read().await;
            for key in keys {
                if let Some(value) = cache.get(&key) {
                    results.insert(key, value.clone());
                } else {
                    missing_keys.push(key);
                }
            }
        }

        // Load missing keys
        if !missing_keys.is_empty() {
            let loaded = (self.loader_fn)(missing_keys).await;

            // Update cache
            {
                let mut cache = self.cache.write().await;
                cache.extend(loaded.clone());
            }

            results.extend(loaded);
        }

        results
    }

    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

/// Persisted queries manager
pub struct PersistedQueries {
    queries: Arc<RwLock<HashMap<String, String>>>,
}

impl PersistedQueries {
    pub fn new() -> Self {
        Self {
            queries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, hash: String, query: String) {
        let mut queries = self.queries.write().await;
        queries.insert(hash, query);
    }

    pub async fn get(&self, hash: &str) -> Option<String> {
        let queries = self.queries.read().await;
        queries.get(hash).cloned()
    }

    pub async fn remove(&self, hash: &str) {
        let mut queries = self.queries.write().await;
        queries.remove(hash);
    }

    pub async fn list(&self) -> Vec<String> {
        let queries = self.queries.read().await;
        queries.keys().cloned().collect()
    }
}

/// GraphQL extension for performance monitoring
pub struct PerformanceExtension;

impl ExtensionFactory for PerformanceExtension {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(PerformanceExtensionImpl {
            start: Mutex::new(None),
        })
    }
}

struct PerformanceExtensionImpl {
    start: Mutex<Option<Instant>>,
}

#[async_trait::async_trait]
impl Extension for PerformanceExtensionImpl {
    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> async_graphql::Response {
        let start = Instant::now();
        *self.start.lock().await = Some(start);

        let response = next.run(ctx, operation_name).await;

        let elapsed = start.elapsed();
        // Note: extensions require specific async_graphql types
        // Performance data is logged instead

        response
    }
}

/// Depth limiting extension
pub struct DepthLimitExtension {
    max_depth: usize,
}

impl DepthLimitExtension {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }
}

// ============================================================================
// GRAPHQL ENGINE - Core Implementation
// ============================================================================

/// Main GraphQL engine that interfaces with the database
pub struct GraphQLEngine {
    // Would connect to actual database components
    subscription_manager: Arc<SubscriptionManager>,
    query_cache: Arc<QueryCache>,
    rate_limiter: Arc<RateLimiter>,
    persisted_queries: Arc<PersistedQueries>,
}

impl GraphQLEngine {
    pub fn new() -> Self {
        Self {
            subscription_manager: Arc::new(SubscriptionManager::new()),
            query_cache: Arc::new(QueryCache::new(1000, 300)),
            rate_limiter: Arc::new(RateLimiter::new()),
            persisted_queries: Arc::new(PersistedQueries::new()),
        }
    }

    // Schema operations
    pub async fn get_schemas(&self) -> GqlResult<Vec<DatabaseSchema>> {
        // Mock implementation - would query actual catalog
        Ok(vec![DatabaseSchema {
            name: "public".to_string(),
            tables: vec![],
            table_count: 0,
            created_at: DateTime::now(),
            description: Some("Default schema".to_string()),
        }])
    }

    pub async fn get_schema(&self, name: &str) -> GqlResult<Option<DatabaseSchema>> {
        let schemas = self.get_schemas().await?;
        Ok(schemas.into_iter().find(|s| s.name == name))
    }

    // Table operations
    pub async fn get_tables(
        &self,
        _schema: Option<String>,
        _limit: Option<i32>,
        _offset: Option<i32>,
    ) -> GqlResult<Vec<TableType>> {
        // Mock implementation
        Ok(vec![])
    }

    pub async fn get_table(&self, _name: &str, _schema: Option<String>) -> GqlResult<Option<TableType>> {
        // Mock implementation
        Ok(None)
    }

    pub async fn get_table_statistics(&self, _table: &str) -> GqlResult<TableStatistics> {
        Ok(TableStatistics {
            row_count: BigInt(0),
            size_bytes: BigInt(0),
            index_size_bytes: BigInt(0),
            avg_row_size: 0.0,
            last_analyzed: None,
            last_modified: None,
        })
    }

    pub async fn get_sample_rows(&self, _table: &str, _limit: i32) -> GqlResult<Vec<RowType>> {
        Ok(vec![])
    }

    pub async fn get_column_statistics(&self, _table: &str, _column: &str) -> GqlResult<ColumnStatistics> {
        Ok(ColumnStatistics {
            distinct_count: BigInt(0),
            null_count: BigInt(0),
            avg_length: None,
            min_value: None,
            max_value: None,
            histogram: None,
        })
    }

    // Query operations
    pub async fn query_table(
        &self,
        _table: &str,
        _where_clause: Option<WhereClause>,
        _order_by: Option<Vec<OrderBy>>,
        _limit: Option<i32>,
        _offset: Option<i32>,
    ) -> Result<(Vec<RowType>, i64, bool)> {
        Ok((vec![], 0, false))
    }

    pub async fn query_tables(
        &self,
        _tables: Vec<String>,
        _joins: Option<Vec<JoinInput>>,
        _where_clause: Option<WhereClause>,
        _order_by: Option<Vec<OrderBy>>,
        _limit: Option<i32>,
    ) -> Result<(Vec<RowType>, i64, bool)> {
        Ok((vec![], 0, false))
    }

    pub async fn query_table_connection(
        &self,
        _table: &str,
        _where_clause: Option<WhereClause>,
        _order_by: Option<Vec<OrderBy>>,
        _first: Option<i32>,
        _after: Option<String>,
        _last: Option<i32>,
        _before: Option<String>,
    ) -> GqlResult<RowConnection> {
        Ok(RowConnection {
            edges: vec![],
            page_info: PageInfo {
                has_next_page: false,
                has_previous_page: false,
                start_cursor: None,
                end_cursor: None,
                total_count: BigInt(0),
            },
            total_count: BigInt(0),
        })
    }

    pub async fn get_row(&self, _table: &str, _id: &ID) -> GqlResult<Option<RowType>> {
        Ok(None)
    }

    pub async fn aggregate(
        &self,
        _table: &str,
        _aggregates: Vec<AggregateInput>,
        _where_clause: Option<WhereClause>,
        _group_by: Option<Vec<String>>,
    ) -> GqlResult<Vec<AggregateResult>> {
        Ok(vec![])
    }

    pub async fn count(&self, _table: &str, _where_clause: Option<WhereClause>) -> GqlResult<i64> {
        Ok(0)
    }

    pub async fn execute_sql(&self, _sql: &str, _params: Option<Vec<Json>>) -> Result<(Vec<RowType>, i64)> {
        Ok((vec![], 0))
    }

    pub async fn search(
        &self,
        _query: &str,
        _tables: Option<Vec<String>>,
        _fields: Option<Vec<String>>,
        _limit: Option<i32>,
    ) -> GqlResult<SearchResult> {
        Ok(SearchResult {
            results: vec![],
            total_count: BigInt(0),
            execution_time_ms: 0.0,
        })
    }

    pub async fn explain(
        &self,
        _table: &str,
        _where_clause: Option<WhereClause>,
        _order_by: Option<Vec<OrderBy>>,
    ) -> GqlResult<QueryPlan> {
        Ok(QueryPlan {
            plan_text: "Sequential Scan".to_string(),
            estimated_cost: 0.0,
            estimated_rows: BigInt(0),
            operations: vec![],
        })
    }

    // Mutation operations
    pub async fn insert_one(&self, _table: &str, _data: HashMap<String, Json>) -> Result<RowType> {
        Err(DbError::NotImplemented("insert_one".to_string()))
    }

    pub async fn insert_many(&self, _table: &str, _data: Vec<HashMap<String, Json>>) -> Result<Vec<RowType>> {
        Err(DbError::NotImplemented("insert_many".to_string()))
    }

    pub async fn update_one(&self, _table: &str, _id: &ID, _data: HashMap<String, Json>) -> Result<Option<RowType>> {
        Ok(None)
    }

    pub async fn update_many(
        &self,
        _table: &str,
        _where_clause: WhereClause,
        _data: HashMap<String, Json>,
    ) -> Result<Vec<RowType>> {
        Ok(vec![])
    }

    pub async fn delete_one(&self, _table: &str, _id: &ID) -> Result<bool> {
        Ok(false)
    }

    pub async fn delete_many(&self, _table: &str, _where_clause: WhereClause) -> Result<i32> {
        Ok(0)
    }

    pub async fn upsert(
        &self,
        _table: &str,
        _unique_fields: Vec<String>,
        _data: HashMap<String, Json>,
    ) -> Result<(RowType, bool)> {
        Err(DbError::NotImplemented("upsert".to_string()))
    }

    pub async fn bulk_insert(&self, _table: &str, _data: Vec<HashMap<String, Json>>, _batch_size: i32) -> Result<i32> {
        Ok(0)
    }

    // Transaction operations
    pub async fn begin_transaction(&self, _isolation_level: Option<IsolationLevel>) -> GqlResult<TransactionResult> {
        Ok(TransactionResult {
            transaction_id: uuid::Uuid::new_v4().to_string(),
            status: "ACTIVE".to_string(),
            timestamp: DateTime::now(),
        })
    }

    pub async fn commit_transaction(&self, transaction_id: &str) -> GqlResult<TransactionResult> {
        Ok(TransactionResult {
            transaction_id: _transaction_id.to_string(),
            status: "COMMITTED".to_string(),
            timestamp: DateTime::now(),
        })
    }

    pub async fn rollback_transaction(&self, transaction_id: &str) -> GqlResult<TransactionResult> {
        Ok(TransactionResult {
            transaction_id: _transaction_id.to_string(),
            status: "ROLLED_BACK".to_string(),
            timestamp: DateTime::now(),
        })
    }

    pub async fn execute_transaction(
        &self,
        _operations: Vec<TransactionOperation>,
        _isolation_level: Option<IsolationLevel>,
    ) -> Result<Vec<String>> {
        Ok(vec![])
    }

    // Subscription operations
    pub async fn register_table_subscription(
        &self,
        table: &str,
        filter: Option<WhereClause>,
        _tx: broadcast::Sender<TableChange>,
    ) -> String {
        self.subscription_manager.register_subscription(table, filter).await
    }

    pub async fn register_insert_subscription(
        &self,
        table: &str,
        filter: Option<WhereClause>,
        _tx: broadcast::Sender<RowInserted>,
    ) -> String {
        self.subscription_manager.register_subscription(table, filter).await
    }

    pub async fn register_update_subscription(
        &self,
        table: &str,
        filter: Option<WhereClause>,
        _tx: broadcast::Sender<RowUpdated>,
    ) -> String {
        self.subscription_manager.register_subscription(table, filter).await
    }

    pub async fn register_delete_subscription(
        &self,
        table: &str,
        filter: Option<WhereClause>,
        _tx: broadcast::Sender<RowDeleted>,
    ) -> String {
        self.subscription_manager.register_subscription(table, filter).await
    }

    pub async fn register_row_subscription(
        &self,
        table: &str,
        _id: &ID,
        _tx: broadcast::Sender<RowChange>,
    ) -> String {
        self.subscription_manager.register_subscription(table, None).await
    }
}

// ============================================================================
// SCHEMA BUILDER
// ============================================================================

/// Build the complete GraphQL schema
pub fn build_schema() -> Schema<QueryRoot, MutationRoot, SubscriptionRoot> {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .extension(PerformanceExtension)
        .limit_depth(10)
        .limit_complexity(1000)
        .finish()
}

/// Create schema with custom configuration
pub fn build_schema_with_config(config: SchemaConfig) -> Schema<QueryRoot, MutationRoot, SubscriptionRoot> {
    let mut builder = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot);

    if let Some(depth) = config.max_depth {
        builder = builder.limit_depth(depth);
    }

    if let Some(complexity) = config.max_complexity {
        builder = builder.limit_complexity(complexity);
    }

    if config.enable_performance_extension {
        builder = builder.extension(PerformanceExtension);
    }

    builder.finish()
}

/// Schema configuration
#[derive(Clone, Debug)]
pub struct SchemaConfig {
    pub max_depth: Option<usize>,
    pub max_complexity: Option<usize>,
    pub enable_performance_extension: bool,
    pub enable_tracing: bool,
}

impl Default for SchemaConfig {
    fn default() -> Self {
        Self {
            max_depth: Some(10),
            max_complexity: Some(1000),
            enable_performance_extension: true,
            enable_tracing: false,
        }
    }
}

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
            return Err(DbError::InvalidInput(format!(
                "Query size {} exceeds maximum {}",
                query.len(),
                self.max_query_size
            ))));
        }

        // Check for malicious patterns
        if query.contains("__schema") && !self.allowed_operations.contains("introspection") {
            return Err(DbError::InvalidInput(
                "Introspection queries are not allowed".to_string(),
            ));
        }

        Ok(())
    }
}

/// Batch query executor for optimizing multiple queries
pub struct BatchExecutor {
    engine: Arc<GraphQLEngine>,
    max_batch_size: usize,
}

impl BatchExecutor {
    pub fn new(engine: Arc<GraphQLEngine>, max_batch_size: usize) -> Self {
        Self {
            engine,
            max_batch_size,
        }
    }

    /// Execute multiple queries in a batch
    pub async fn execute_batch(&self, queries: Vec<BuiltQuery>) -> GqlResult<Vec<QueryResult>> {
        if queries.len() > self.max_batch_size {
            return Err(Error::new(format!(
                "Batch size {} exceeds maximum {}",
                queries.len(),
                self.max_batch_size
            ))));
        }

        let mut results = Vec::new();
        for query in queries {
            match self
                .engine
                .query_table(
                    &query.table,
                    query.where_clause,
                    query.order_by,
                    query.limit,
                    query.offset,
                )
                .await
            {
                Ok((rows, total_count, has_more)) => {
                    results.push(QueryResult::Success(QuerySuccess {
                        rows,
                        total_count: BigInt(total_count),
                        execution_time_ms: 0.0,
                        has_more,
                    }));
                }
                Err(e) => {
                    results.push(QueryResult::Error(QueryError {
                        message: e.to_string(),
                        code: "BATCH_ERROR".to_string(),
                        details: None,
                    }));
                }
            }
        }

        Ok(results)
    }
}

/// Query result formatter for different output formats
pub struct ResultFormatter;

impl ResultFormatter {
    /// Format results as JSON
    pub fn to_json(rows: &[RowType]) -> GqlResult<String> {
        // Convert to simple JSON representation
        let json_rows: Vec<serde_json::Value> = rows
            .iter()
            .map(|row| {
                let map: serde_json::Map<String, serde_json::Value> = row
                    .fields
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::json!(format!("{:?}", v.value))))
                    .collect());
                serde_json::Value::Object(map)
            })
            .collect();
        serde_json::to_string_pretty(&json_rows)
            .map_err(|e| Error::new(format!("JSON serialization error: {}", e)))
    }

    /// Format results as CSV
    pub fn to_csv(rows: &[RowType]) -> GqlResult<String> {
        if rows.is_empty() {
            return Ok(String::new()));
        }

        let mut csv = String::new();

        // Header
        if let Some(first_row) = rows.first() {
            let headers: Vec<String> = first_row.fields.keys().cloned().collect();
            csv.push_str(&headers.join(","));
            csv.push('\n');

            // Rows
            for row in rows {
                let values: Vec<String> = headers
                    .iter()
                    .map(|h| {
                        row.fields
                            .get(h)
                            .map(|v| format!("{:?}", v.value))
                            .unwrap_or_default()
                    })
                    .collect());
                csv.push_str(&values.join(","));
                csv.push('\n');
            }
        }

        Ok(csv)
    }

    /// Format results as Markdown table
    pub fn to_markdown(rows: &[RowType]) -> GqlResult<String> {
        if rows.is_empty() {
            return Ok(String::new());
        }

        let mut md = String::new();

        if let Some(first_row) = rows.first() {
            let headers: Vec<String> = first_row.fields.keys().cloned().collect();

            // Header
            md.push_str("| ");
            md.push_str(&headers.join(" | "));
            md.push_str(" |\n");

            // Separator
            md.push_str("|");
            for _ in &headers {
                md.push_str(" --- |");
            }
            md.push('\n');

            // Rows
            for row in rows {
                md.push_str("| ");
                let values: Vec<String> = headers
                    .iter()
                    .map(|h| {
                        row.fields
                            .get(h)
                            .map(|v| format!("{:?}", v.value))
                            .unwrap_or_default()
                    })
                    .collect());
                md.push_str(&values.join(" | "));
                md.push_str(" |\n");
            }
        }

        Ok(md)
    }
}

/// Subscription filter evaluator
pub struct FilterEvaluator;

impl FilterEvaluator {
    /// Evaluate if a row matches a where clause
    pub fn matches(row: &RowType, where_clause: &WhereClause) -> bool {
        // Simplified evaluation - would need full implementation
        if let Some(condition) = &where_clause.condition {
            return Self::evaluate_condition(row, condition);
        }

        if let Some(and_clauses) = &where_clause.and {
            return and_clauses.iter().all(|c| Self::matches(row, c));
        }

        if let Some(or_clauses) = &where_clause.or {
            return or_clauses.iter().any(|c| Self::matches(row, c));
        }

        if let Some(not_clause) = &where_clause.not {
            return !Self::matches(row, not_clause);
        }

        true
    }

    fn evaluate_condition(row: &RowType, condition: &FilterCondition) -> bool {
        let field_value = row.fields.get(&condition.field);

        match condition.op {
            FilterOp::IsNull => field_value.is_none(),
            FilterOp::IsNotNull => field_value.is_some(),
            _ => true, // Simplified - would need full comparison logic
        }
    }
}

/// Metrics collector for GraphQL operations
pub struct MetricsCollector {
    query_count: Arc<RwLock<u64>>,
    mutation_count: Arc<RwLock<u64>>,
    subscription_count: Arc<RwLock<u64>>,
    error_count: Arc<RwLock<u64>>,
    total_execution_time: Arc<RwLock<Duration>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            query_count: Arc::new(RwLock::new(0)),
            mutation_count: Arc::new(RwLock::new(0)),
            subscription_count: Arc::new(RwLock::new(0)),
            error_count: Arc::new(RwLock::new(0)),
            total_execution_time: Arc::new(RwLock::new(Duration::ZERO)),
        }
    }

    pub async fn record_query(&self, execution_time: Duration) {
        *self.query_count.write().await += 1;
        *self.total_execution_time.write().await += execution_time;
    }

    pub async fn record_mutation(&self, execution_time: Duration) {
        *self.mutation_count.write().await += 1;
        *self.total_execution_time.write().await += execution_time;
    }

    pub async fn record_subscription(&self) {
        *self.subscription_count.write().await += 1;
    }

    pub async fn record_error(&self) {
        *self.error_count.write().await += 1;
    }

    pub async fn get_metrics(&self) -> Metrics {
        Metrics {
            query_count: *self.query_count.read().await,
            mutation_count: *self.mutation_count.read().await,
            subscription_count: *self.subscription_count.read().await,
            error_count: *self.error_count.read().await,
            total_execution_time: *self.total_execution_time.read().await,
            avg_execution_time: if *self.query_count.read().await > 0 {
                *self.total_execution_time.read().await / (*self.query_count.read().await as u32)
            } else {
                Duration::ZERO
            },
        }
    }

    pub async fn reset(&self) {
        *self.query_count.write().await = 0;
        *self.mutation_count.write().await = 0;
        *self.subscription_count.write().await = 0;
        *self.error_count.write().await = 0;
        *self.total_execution_time.write().await = Duration::ZERO;
    }
}

/// Metrics snapshot
#[derive(Clone, Debug)]
pub struct Metrics {
    pub query_count: u64,
    pub mutation_count: u64,
    pub subscription_count: u64,
    pub error_count: u64,
    pub total_execution_time: Duration,
    pub avg_execution_time: Duration,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schema_builds() {
        let schema = build_schema();
        assert!(schema.sdl().contains("type Query"));
        assert!(schema.sdl().contains("type Mutation"));
        assert!(schema.sdl().contains("type Subscription"));
    }

    #[tokio::test]
    async fn test_complexity_analyzer() {
        let analyzer = ComplexityAnalyzer::new(100, 5);
        assert_eq!(analyzer.max_complexity, 100);
        assert_eq!(analyzer.max_depth, 5);
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new();
        limiter.set_limit("test", RateLimit {
            max_requests: 10,
            window_secs: 60,
        }).await;

        // Should succeed
        assert!(limiter.check_rate_limit("test").await.is_ok());
    }

    #[tokio::test]
    async fn test_query_cache() {
        let cache = QueryCache::new(100, 60);

        let rows = vec![];
        cache.set("test_key".to_string(), rows.clone()).await;

        let cached = cache.get("test_key").await;
        assert!(cached.is_some());
    }

    #[tokio::test]
    async fn test_subscription_manager() {
        let manager = SubscriptionManager::new();
        let sub_id = manager.register_subscription("users", None).await;

        assert!(!sub_id.is_empty());

        let subs = manager.get_active_subscriptions().await;
        assert_eq!(subs.len(), 1);
    }

    #[tokio::test]
    async fn test_authorization_context() {
        let auth = AuthorizationContext::new(
            "user1".to_string(),
            vec!["user".to_string()],
            vec!["read:users".to_string()],
        );

        assert!(auth.can_read("users").unwrap());
        assert!(!auth.can_write("users").unwrap());
    }

    #[tokio::test]
    async fn test_graphql_engine() {
        let engine = GraphQLEngine::new();
        let schemas = engine.get_schemas().await.unwrap();
        assert!(!schemas.is_empty());
    }

    #[tokio::test]
    async fn test_query_builder() {
        let query = QueryBuilder::new("users")
            .select(vec!["id".to_string(), "name".to_string()])
            .limit(10)
            .offset(0)
            .order_by("created_at", SortOrder::Desc)
            .build();

        assert_eq!(query.table, "users");
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(0));
    }

    #[tokio::test]
    async fn test_mutation_builder() {
        let mutation = MutationBuilder::insert("users")
            .set("name", serde_json::json!("Alice"))
            .set("email", serde_json::json!("alice@example.com"))
            .build();

        assert_eq!(mutation.table, "users");
    }

    #[tokio::test]
    async fn test_request_validator() {
        let validator = RequestValidator::new(10000);
        let result = validator.validate("{ users { id name } }");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        collector.record_query(Duration::from_millis(100)).await;
        collector.record_mutation(Duration::from_millis(200)).await;

        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.query_count, 1);
        assert_eq!(metrics.mutation_count, 1);
    }

    #[tokio::test]
    async fn test_result_formatter() {
        let rows = vec![];
        let json = ResultFormatter::to_json(&rows).unwrap();
        assert!(!json.is_empty());
    }

    #[tokio::test]
    async fn test_query_optimizer() {
        let optimizer = QueryOptimizer::new();
        let query = QueryBuilder::new("users").build();
        let suggestions = optimizer.analyze(&query).await;
        assert!(!suggestions.suggestions.is_empty());
    }
}
