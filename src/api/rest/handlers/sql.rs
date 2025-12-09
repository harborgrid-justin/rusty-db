// # SQL Operations Handlers
//
// Comprehensive REST API endpoints for all SQL operations
// Provides 100% coverage of SQL features through HTTP API

use axum::{
    extract::{Path, Query, State},
    response::Json as AxumJson,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

use crate::error::DbError;
use crate::api::rest::types::*;
use crate::parser::{SqlParser, SqlStatement, AlterAction, ConstraintType};
use crate::catalog::{Catalog, Schema, Column, DataType};
use crate::transaction::TransactionManager;
use crate::execution::{Executor, QueryResult};
use super::{CATALOG, TXN_MANAGER, SQL_PARSER};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct AlterTableRequest {
    pub operation: String,  // "add_column", "drop_column", "alter_column", etc.
    pub column_name: Option<String>,
    pub column_definition: Option<ColumnDefinition>,
    pub constraint: Option<ConstraintDefinition>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ConstraintDefinition {
    pub constraint_type: String,  // "primary_key", "foreign_key", "unique", "check"
    pub columns: Vec<String>,
    pub ref_table: Option<String>,
    pub ref_columns: Option<Vec<String>>,
    pub check_expression: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct BackupRequest {
    pub database: String,
    pub path: String,
    pub compression: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ProcedureRequest {
    pub name: String,
    pub parameters: Vec<ParameterDef>,
    pub body: String,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ParameterDef {
    pub name: String,
    pub data_type: String,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ExecProcedureRequest {
    pub name: String,
    pub arguments: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UnionRequest {
    pub left_query: String,
    pub right_query: String,
    pub union_all: bool,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ViewRequest {
    pub name: String,
    pub query: String,
    pub or_replace: bool,
}

// ============================================================================
// DDL Operations
// ============================================================================

/// Create a new database
#[utoipa::path(
    post,
    path = "/api/v1/sql/databases",
    tag = "sql",
    request_body = inline(Object),
    responses(
        (status = 201, description = "Database created"),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn create_database(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<serde_json::Value>,
) -> ApiResult<StatusCode> {
    let name = request.get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::new("INVALID_INPUT", "Database name is required"))?;

    let stmt = SqlStatement::CreateDatabase {
        name: name.to_string(),
    };

    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    drop(catalog_guard);
    let executor = Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone());

    executor.execute(stmt)
        .map_err(|e| ApiError::new("EXECUTION_ERROR", &e.to_string()))?;

    Ok(StatusCode::CREATED)
}

/// Drop a database
#[utoipa::path(
    delete,
    path = "/api/v1/sql/databases/{name}",
    tag = "sql",
    responses(
        (status = 204, description = "Database dropped"),
        (status = 404, description = "Database not found", body = ApiError),
    )
)]
pub async fn drop_database(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let stmt = SqlStatement::DropDatabase { name };

    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    drop(catalog_guard);
    let executor = Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone());

    executor.execute(stmt)
        .map_err(|e| ApiError::new("EXECUTION_ERROR", &e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Backup a database
#[utoipa::path(
    post,
    path = "/api/v1/sql/backup",
    tag = "sql",
    request_body = BackupRequest,
    responses(
        (status = 200, description = "Backup completed"),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn backup_database(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<BackupRequest>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    let stmt = SqlStatement::BackupDatabase {
        database: request.database,
        path: request.path.clone(),
    };

    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    drop(catalog_guard);
    let executor = Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone());

    executor.execute(stmt)
        .map_err(|e| ApiError::new("EXECUTION_ERROR", &e.to_string()))?;

    Ok(AxumJson(json!({
        "status": "success",
        "path": request.path
    })))
}

/// ALTER TABLE operations
#[utoipa::path(
    patch,
    path = "/api/v1/sql/tables/{name}/alter",
    tag = "sql",
    request_body = AlterTableRequest,
    responses(
        (status = 200, description = "Table altered"),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn alter_table(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
    AxumJson(request): AxumJson<AlterTableRequest>,
) -> ApiResult<StatusCode> {
    let action = match request.operation.as_str() {
        "add_column" => {
            let col_def = request.column_definition
                .ok_or_else(|| ApiError::new("INVALID_INPUT", "Column definition is required"))?;

            let column = Column {
                name: col_def.name,
                data_type: parse_data_type(&col_def.data_type),
                nullable: col_def.nullable,
                default: col_def.default_value,
            };

            AlterAction::AddColumn(column)
        }
        "drop_column" => {
            let col_name = request.column_name
                .ok_or_else(|| ApiError::new("INVALID_INPUT", "Column name is required"))?;

            AlterAction::DropColumn(col_name)
        }
        "alter_column" => {
            let col_name = request.column_name
                .ok_or_else(|| ApiError::new("INVALID_INPUT", "Column name is required"))?;
            let col_def = request.column_definition
                .ok_or_else(|| ApiError::new("INVALID_INPUT", "Column definition is required"))?;

            AlterAction::AlterColumn {
                column_name: col_name,
                new_type: parse_data_type(&col_def.data_type),
            }
        }
        "modify_column" => {
            let col_name = request.column_name
                .ok_or_else(|| ApiError::new("INVALID_INPUT", "Column name is required"))?;
            let col_def = request.column_definition
                .ok_or_else(|| ApiError::new("INVALID_INPUT", "Column definition is required"))?;

            AlterAction::ModifyColumn {
                column_name: col_name,
                new_type: parse_data_type(&col_def.data_type),
                nullable: Some(col_def.nullable),
            }
        }
        "add_constraint" => {
            let constraint = request.constraint
                .ok_or_else(|| ApiError::new("INVALID_INPUT", "Constraint definition is required"))?;

            let constraint_type = match constraint.constraint_type.as_str() {
                "primary_key" => ConstraintType::PrimaryKey(constraint.columns),
                "foreign_key" => ConstraintType::ForeignKey {
                    columns: constraint.columns,
                    ref_table: constraint.ref_table.unwrap_or_default(),
                    ref_columns: constraint.ref_columns.unwrap_or_default(),
                },
                "unique" => ConstraintType::Unique(constraint.columns),
                "check" => ConstraintType::Check(constraint.check_expression.unwrap_or_default()),
                _ => return Err(ApiError::new("INVALID_INPUT", "Unknown constraint type")),
            };

            AlterAction::AddConstraint(constraint_type)
        }
        "drop_constraint" => {
            let constraint_name = request.column_name
                .ok_or_else(|| ApiError::new("INVALID_INPUT", "Constraint name is required"))?;

            AlterAction::DropConstraint(constraint_name)
        }
        "drop_default" => {
            let col_name = request.column_name
                .ok_or_else(|| ApiError::new("INVALID_INPUT", "Column name is required"))?;

            AlterAction::DropDefault(col_name)
        }
        _ => return Err(ApiError::new("INVALID_INPUT", "Unknown alter operation")),
    };

    let stmt = SqlStatement::AlterTable {
        name,
        action,
    };

    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    drop(catalog_guard);
    let executor = Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone());

    executor.execute(stmt)
        .map_err(|e| ApiError::new("EXECUTION_ERROR", &e.to_string()))?;

    Ok(StatusCode::OK)
}

// ============================================================================
// View Operations
// ============================================================================

/// Create or replace a view
#[utoipa::path(
    post,
    path = "/api/v1/sql/views",
    tag = "sql",
    request_body = ViewRequest,
    responses(
        (status = 201, description = "View created"),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn create_view(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<ViewRequest>,
) -> ApiResult<StatusCode> {
    let stmt = SqlStatement::CreateView {
        name: request.name,
        query: request.query,
        or_replace: request.or_replace,
    };

    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    drop(catalog_guard);
    let executor = Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone());

    executor.execute(stmt)
        .map_err(|e| ApiError::new("EXECUTION_ERROR", &e.to_string()))?;

    Ok(StatusCode::CREATED)
}

/// Drop a view
#[utoipa::path(
    delete,
    path = "/api/v1/sql/views/{name}",
    tag = "sql",
    responses(
        (status = 204, description = "View dropped"),
        (status = 404, description = "View not found", body = ApiError),
    )
)]
pub async fn drop_view(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let stmt = SqlStatement::DropView { name };

    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    drop(catalog_guard);
    let executor = Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone());

    executor.execute(stmt)
        .map_err(|e| ApiError::new("EXECUTION_ERROR", &e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Index Operations
// ============================================================================

/// Create an index
#[utoipa::path(
    post,
    path = "/api/v1/sql/indexes",
    tag = "sql",
    request_body = inline(Object),
    responses(
        (status = 201, description = "Index created"),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn create_index(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<serde_json::Value>,
) -> ApiResult<StatusCode> {
    let name = request.get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::new("INVALID_INPUT", "Index name is required"))?;

    let table = request.get("table")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::new("INVALID_INPUT", "Table name is required"))?;

    let columns: Vec<String> = request.get("columns")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .ok_or_else(|| ApiError::new("INVALID_INPUT", "Columns are required"))?;

    let unique = request.get("unique")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let stmt = SqlStatement::CreateIndex {
        name: name.to_string(),
        table: table.to_string(),
        columns,
        unique,
    };

    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    drop(catalog_guard);
    let executor = Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone());

    executor.execute(stmt)
        .map_err(|e| ApiError::new("EXECUTION_ERROR", &e.to_string()))?;

    Ok(StatusCode::CREATED)
}

/// Drop an index
#[utoipa::path(
    delete,
    path = "/api/v1/sql/indexes/{name}",
    tag = "sql",
    responses(
        (status = 204, description = "Index dropped"),
        (status = 404, description = "Index not found", body = ApiError),
    )
)]
pub async fn drop_index(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let stmt = SqlStatement::DropIndex { name };

    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    drop(catalog_guard);
    let executor = Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone());

    executor.execute(stmt)
        .map_err(|e| ApiError::new("EXECUTION_ERROR", &e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Stored Procedures
// ============================================================================

/// Create a stored procedure
#[utoipa::path(
    post,
    path = "/api/v1/sql/procedures",
    tag = "sql",
    request_body = ProcedureRequest,
    responses(
        (status = 201, description = "Procedure created"),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn create_procedure(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<ProcedureRequest>,
) -> ApiResult<StatusCode> {
    let parameters: Vec<(String, DataType)> = request.parameters
        .iter()
        .map(|p| (p.name.clone(), parse_data_type(&p.data_type)))
        .collect();

    let stmt = SqlStatement::CreateProcedure {
        name: request.name,
        parameters,
        body: request.body,
    };

    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    drop(catalog_guard);
    let executor = Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone());

    executor.execute(stmt)
        .map_err(|e| ApiError::new("EXECUTION_ERROR", &e.to_string()))?;

    Ok(StatusCode::CREATED)
}

/// Execute a stored procedure
#[utoipa::path(
    post,
    path = "/api/v1/sql/procedures/{name}/execute",
    tag = "sql",
    request_body = ExecProcedureRequest,
    responses(
        (status = 200, description = "Procedure executed"),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn execute_procedure(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
    AxumJson(request): AxumJson<ExecProcedureRequest>,
) -> ApiResult<AxumJson<QueryResponse>> {
    let arguments: Vec<String> = request.arguments
        .iter()
        .map(|v| v.to_string())
        .collect();

    let stmt = SqlStatement::ExecProcedure {
        name,
        arguments,
    };

    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    drop(catalog_guard);
    let executor = Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone());

    let result = executor.execute(stmt)
        .map_err(|e| ApiError::new("EXECUTION_ERROR", &e.to_string()))?;

    let response = QueryResponse {
        query_id: Uuid::new_v4().to_string(),
        row_count: result.rows.len(),
        rows: vec![],
        columns: vec![],
        affected_rows: Some(result.rows_affected),
        execution_time_ms: 0,
        plan: None,
        warnings: vec![],
        has_more: false,
    };

    Ok(AxumJson(response))
}

// ============================================================================
// Advanced Query Operations
// ============================================================================

/// Execute UNION query
#[utoipa::path(
    post,
    path = "/api/v1/sql/union",
    tag = "sql",
    request_body = UnionRequest,
    responses(
        (status = 200, description = "Union query executed"),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn execute_union(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<UnionRequest>,
) -> ApiResult<AxumJson<QueryResponse>> {
    // Parse both queries
    let left_stmts = SQL_PARSER.parse(&request.left_query)
        .map_err(|e| ApiError::new("SQL_PARSE_ERROR", &e.to_string()))?;
    let right_stmts = SQL_PARSER.parse(&request.right_query)
        .map_err(|e| ApiError::new("SQL_PARSE_ERROR", &e.to_string()))?;

    let left_stmt = left_stmts.into_iter().next()
        .ok_or_else(|| ApiError::new("SQL_PARSE_ERROR", "No valid left SQL statement"))?;
    let right_stmt = right_stmts.into_iter().next()
        .ok_or_else(|| ApiError::new("SQL_PARSE_ERROR", "No valid right SQL statement"))?;

    let stmt = SqlStatement::Union {
        left: Box::new(left_stmt),
        right: Box::new(right_stmt),
        all: request.union_all,
    };

    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    drop(catalog_guard);
    let executor = Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone());

    let result = executor.execute(stmt)
        .map_err(|e| ApiError::new("EXECUTION_ERROR", &e.to_string()))?;

    let rows: Vec<HashMap<String, serde_json::Value>> = result.rows.iter().map(|row| {
        let mut map = HashMap::new();
        for (i, val) in row.iter().enumerate() {
            if let Some(col_name) = result.columns.get(i) {
                map.insert(col_name.clone(), serde_json::Value::String(val.clone()));
            }
        }
        map
    }).collect();

    let response = QueryResponse {
        query_id: Uuid::new_v4().to_string(),
        row_count: rows.len(),
        rows,
        columns: result.columns.iter().map(|name| ColumnMetadata {
            name: name.clone(),
            data_type: "TEXT".to_string(),
            nullable: true,
            precision: None,
            scale: None,
        }).collect(),
        affected_rows: None,
        execution_time_ms: 0,
        plan: None,
        warnings: vec![],
        has_more: false,
    };

    Ok(AxumJson(response))
}

/// Truncate table
#[utoipa::path(
    post,
    path = "/api/v1/sql/tables/{name}/truncate",
    tag = "sql",
    responses(
        (status = 200, description = "Table truncated"),
        (status = 404, description = "Table not found", body = ApiError),
    )
)]
pub async fn truncate_table(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let stmt = SqlStatement::TruncateTable { name };

    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    drop(catalog_guard);
    let executor = Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone());

    executor.execute(stmt)
        .map_err(|e| ApiError::new("EXECUTION_ERROR", &e.to_string()))?;

    Ok(StatusCode::OK)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse data type string into DataType enum
fn parse_data_type(type_str: &str) -> DataType {
    let upper = type_str.to_uppercase();
    if upper.starts_with("VARCHAR") {
        let size = upper
            .trim_start_matches("VARCHAR")
            .trim_matches(|c| c == '(' || c == ')' || c == ' ')
            .parse::<usize>()
            .unwrap_or(255);
        DataType::Varchar(size)
    } else {
        match upper.as_str() {
            "INT" | "INTEGER" => DataType::Integer,
            "BIGINT" => DataType::BigInt,
            "FLOAT" | "REAL" => DataType::Float,
            "DOUBLE" | "DOUBLE PRECISION" => DataType::Double,
            "TEXT" => DataType::Text,
            "BOOL" | "BOOLEAN" => DataType::Boolean,
            "DATE" => DataType::Date,
            "TIMESTAMP" | "DATETIME" => DataType::Timestamp,
            _ => DataType::Text,
        }
    }
}
