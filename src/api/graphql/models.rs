// GraphQL Data Models and Types
//
// Data structure definitions for the GraphQL API

use async_graphql::{
    Context, Enum, Error, ErrorExtensions, InputObject, Object,
    Result as GqlResult, Schema, SimpleObject, Subscription, Union, ID,
};
use chrono::{DateTime as ChronoDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::DbError;
use super::types::*;

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

