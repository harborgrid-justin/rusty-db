// # REST API Handlers
//
// Handler functions for all REST API endpoints.
// Each handler implements proper error handling and uses dependency injection.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json as AxumJson,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

use super::{CATALOG, SQL_PARSER, TXN_MANAGER};
use crate::api::rest::types::*;
use crate::catalog::{Column, DataType, Schema};
use crate::error::DbError;
use crate::execution::Executor;

/// Helper function to format DataType enum as a string for display
fn format_data_type(data_type: &DataType) -> String {
    match data_type {
        DataType::Integer => "INTEGER".to_string(),
        DataType::BigInt => "BIGINT".to_string(),
        DataType::Float => "FLOAT".to_string(),
        DataType::Double => "DOUBLE".to_string(),
        DataType::Varchar(size) => format!("VARCHAR({})", size),
        DataType::Text => "TEXT".to_string(),
        DataType::Boolean => "BOOLEAN".to_string(),
        DataType::Date => "DATE".to_string(),
        DataType::Timestamp => "TIMESTAMP".to_string(),
    }
}

// Execute a SQL query
#[axum::debug_handler]
#[utoipa::path(
    post,
    path = "/api/v1/query",
    tag = "database",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Query executed successfully", body = QueryResponse),
        (status = 400, description = "Invalid query", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn execute_query(
    State(state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<QueryRequest>,
) -> ApiResult<AxumJson<QueryResponse>> {
    let query_id = Uuid::new_v4();
    let start = SystemTime::now();

    // Validate SQL
    if request.sql.trim().is_empty() {
        return Err(ApiError::new("INVALID_INPUT", "SQL query cannot be empty"));
    }

    // Record query execution
    {
        let mut queries = state.active_queries.write().await;
        queries.insert(
            query_id,
            QueryExecution {
                query_id,
                sql: request.sql.clone(),
                started_at: start,
                session_id: SessionId(1), // Default session
                status: "running".to_string(),
            },
        );
    }

    // Parse SQL
    let stmts = SQL_PARSER
        .parse(&request.sql)
        .map_err(|e| ApiError::new("SQL_PARSE_ERROR", &e.to_string()))?;

    // Get first statement
    let stmt = stmts
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::new("SQL_PARSE_ERROR", "No valid SQL statement found"))?;

    // Execute query
    let executor = {
        let catalog_guard = CATALOG.read();
        let catalog_snapshot = (*catalog_guard).clone();
        Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone())
    };
    let result = executor
        .execute(stmt)
        .map_err(|e| ApiError::new("EXECUTION_ERROR", &e.to_string()))?;

    let execution_time = start.elapsed().unwrap_or_default().as_millis() as u64;

    // Clean up query tracking
    {
        let mut queries = state.active_queries.write().await;
        queries.remove(&query_id);
    }

    // Convert QueryResult to QueryResponse
    let columns_meta: Vec<ColumnMetadata> = result
        .columns
        .iter()
        .map(|name| ColumnMetadata {
            name: name.clone(),
            data_type: "TEXT".to_string(),
            nullable: true,
            precision: None,
            scale: None,
        })
        .collect();

    let rows: Vec<HashMap<String, serde_json::Value>> = result
        .rows
        .iter()
        .map(|row| {
            let mut map = HashMap::new();
            for (i, val) in row.iter().enumerate() {
                if let Some(col_name) = result.columns.get(i) {
                    map.insert(col_name.clone(), serde_json::Value::String(val.clone()));
                }
            }
            map
        })
        .collect();

    let response = QueryResponse {
        query_id: query_id.to_string(),
        row_count: rows.len(),
        rows,
        columns: columns_meta,
        affected_rows: Some(result.rows_affected),
        execution_time_ms: execution_time,
        plan: None,
        warnings: vec![],
        has_more: false,
    };

    Ok(AxumJson(response))
}

// Execute batch operations
#[utoipa::path(
    post,
    path = "/api/v1/batch",
    tag = "database",
    request_body = BatchRequest,
    responses(
        (status = 200, description = "Batch executed", body = BatchResponse),
        (status = 400, description = "Invalid batch", body = ApiError),
    )
)]
pub async fn execute_batch(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<BatchRequest>,
) -> ApiResult<AxumJson<BatchResponse>> {
    let batch_id = Uuid::new_v4();
    let start = SystemTime::now();

    if request.statements.is_empty() {
        return Err(ApiError::new(
            "INVALID_INPUT",
            "Batch must contain at least one statement",
        ));
    }

    let mut results = Vec::new();
    let mut success_count = 0;
    let mut failure_count = 0;

    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    drop(catalog_guard);
    let executor = Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone());

    for (index, statement) in request.statements.iter().enumerate() {
        let stmt_start = SystemTime::now();

        let result = match SQL_PARSER.parse(statement) {
            Ok(stmts) => {
                if let Some(stmt) = stmts.into_iter().next() {
                    executor.execute(stmt)
                } else {
                    Err(DbError::SqlParse(
                        "No valid SQL statement found".to_string(),
                    ))
                }
            }
            Err(e) => Err(DbError::SqlParse(e.to_string())),
        };

        let success = result.is_ok();
        let (affected_rows, error) = match result {
            Ok(res) => (Some(res.rows_affected), None),
            Err(e) => (None, Some(e.to_string())),
        };

        if success {
            success_count += 1;
        } else {
            failure_count += 1;
        }

        results.push(BatchStatementResult {
            statement_index: index,
            success,
            affected_rows,
            error,
            execution_time_ms: stmt_start.elapsed().unwrap_or_default().as_millis() as u64,
        });

        if !success && request.stop_on_error {
            break;
        }
    }

    let total_time = start.elapsed().unwrap_or_default().as_millis() as u64;

    let response = BatchResponse {
        batch_id: batch_id.to_string(),
        results,
        total_time_ms: total_time,
        success_count,
        failure_count,
    };

    Ok(AxumJson(response))
}

// Get table information
#[utoipa::path(
    get,
    path = "/api/v1/tables/{name}",
    tag = "database",
    params(
        ("name" = String, Path, description = "Table name")
    ),
    responses(
        (status = 200, description = "Table info", body = TableInfo),
        (status = 404, description = "Table not found", body = ApiError),
    )
)]
pub async fn get_table(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<AxumJson<TableInfo>> {
    let catalog = CATALOG.read();
    let schema = catalog
        .get_table(&name)
        .map_err(|_| ApiError::new("NOT_FOUND", &format!("Table {} not found", name)))?;

    let columns = schema
        .columns
        .iter()
        .map(|c| ColumnMetadata {
            name: c.name.clone(),
            data_type: format_data_type(&c.data_type),
            nullable: c.nullable,
            precision: None,
            scale: None,
        })
        .collect();

    let table = TableInfo {
        name: schema.name.clone(),
        schema: "public".to_string(),
        row_count: 0,
        size_bytes: 0,
        columns,
        indexes: vec![],
    };

    Ok(AxumJson(table))
}

/// Create a new table
#[utoipa::path(
    post,
    path = "/api/v1/tables/{name}",
    tag = "database",
    request_body = TableRequest,
    responses(
        (status = 201, description = "Table created"),
        (status = 409, description = "Table already exists", body = ApiError),
    )
)]
pub async fn create_table(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
    AxumJson(request): AxumJson<TableRequest>,
) -> ApiResult<StatusCode> {
    // Validate input
    if request.columns.is_empty() {
        return Err(ApiError::new(
            "INVALID_INPUT",
            "Table must have at least one column",
        ));
    }

    // Convert API column definitions to catalog columns
    let columns: Vec<Column> = request
        .columns
        .iter()
        .map(|col| {
            let data_type = parse_data_type(&col.data_type);
            Column {
                name: col.name.clone(),
                data_type,
                nullable: col.nullable,
                default: col.default_value.as_ref().map(|v| v.to_string()),
            }
        })
        .collect();

    // Create schema
    let schema = if let Some(pk) = &request.primary_key {
        if let Some(pk_col) = pk.first() {
            Schema::new(name.clone(), columns).with_primary_key(pk_col.clone())
        } else {
            Schema::new(name.clone(), columns)
        }
    } else {
        Schema::new(name.clone(), columns)
    };

    // Add to catalog
    let catalog = CATALOG.write();
    match catalog.create_table(schema) {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(e) => {
            if e.to_string().contains("already exists") {
                Err(ApiError::new(
                    "CONFLICT",
                    format!("Table '{}' already exists", name),
                ))
            } else {
                Err(ApiError::new(
                    "DATABASE_ERROR",
                    format!("Failed to create table: {}", e),
                ))
            }
        }
    }
}

/// Helper function to parse data type string into DataType enum
fn parse_data_type(type_str: &str) -> DataType {
    let upper = type_str.to_uppercase();
    if upper.starts_with("VARCHAR") {
        // Extract size from VARCHAR(n)
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

/// Update table schema
#[utoipa::path(
    put,
    path = "/api/v1/tables/{name}",
    tag = "database",
    request_body = TableRequest,
    responses(
        (status = 200, description = "Table updated"),
        (status = 404, description = "Table not found", body = ApiError),
    )
)]
pub async fn update_table(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
    AxumJson(request): AxumJson<TableRequest>,
) -> ApiResult<StatusCode> {
    // First verify the table exists
    {
        let catalog = CATALOG.read();
        if catalog.get_table(&name).is_err() {
            return Err(ApiError::new(
                "NOT_FOUND",
                format!("Table '{}' not found", name),
            ));
        }
    }

    // Drop and recreate with new schema (simple approach)
    // A production system would support ALTER TABLE operations
    let catalog = CATALOG.write();

    // Drop existing table
    if let Err(e) = catalog.drop_table(&name) {
        return Err(ApiError::new(
            "DATABASE_ERROR",
            format!("Failed to update table: {}", e),
        ));
    }

    // Create new schema with updated columns
    let columns: Vec<Column> = request
        .columns
        .iter()
        .map(|col| Column {
            name: col.name.clone(),
            data_type: parse_data_type(&col.data_type),
            nullable: col.nullable,
            default: col.default_value.as_ref().map(|v| v.to_string()),
        })
        .collect();

    let schema = Schema::new(name.clone(), columns);

    match catalog.create_table(schema) {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err(ApiError::new(
            "DATABASE_ERROR",
            format!("Failed to recreate table: {}", e),
        )),
    }
}

/// Delete a table
#[utoipa::path(
    delete,
    path = "/api/v1/tables/{name}",
    tag = "database",
    responses(
        (status = 204, description = "Table deleted"),
        (status = 404, description = "Table not found", body = ApiError),
    )
)]
pub async fn delete_table(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let catalog = CATALOG.write();

    match catalog.drop_table(&name) {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err(ApiError::new(
            "NOT_FOUND",
            format!("Table '{}' not found: {}", name, e),
        )),
    }
}

// Get database schema
#[utoipa::path(
    get,
    path = "/api/v1/schema",
    tag = "database",
    responses(
        (status = 200, description = "Schema information", body = SchemaResponse),
    )
)]
pub async fn get_schema(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<SchemaResponse>> {
    let catalog = CATALOG.read();
    let table_names = catalog.list_tables();

    let mut tables: Vec<TableInfo> = Vec::new();

    for table_name in &table_names {
        if let Ok(schema) = catalog.get_table(table_name) {
            let columns: Vec<ColumnMetadata> = schema
                .columns
                .iter()
                .map(|col| ColumnMetadata {
                    name: col.name.clone(),
                    data_type: format_data_type(&col.data_type),
                    nullable: col.nullable,
                    precision: None,
                    scale: None,
                })
                .collect();

            tables.push(TableInfo {
                name: schema.name.clone(),
                schema: "public".to_string(),
                row_count: 0,
                size_bytes: 0,
                columns,
                indexes: vec![],
            });
        }
    }

    let total_count = tables.len();

    let response = SchemaResponse {
        database_name: "rustydb".to_string(),
        tables,
        views: vec![],      // Would need view catalog integration
        procedures: vec![], // Would need procedure catalog integration
        total_count,
    };

    Ok(AxumJson(response))
}

// Begin a new transaction
#[utoipa::path(
    post,
    path = "/api/v1/transactions",
    tag = "database",
    request_body = TransactionRequest,
    responses(
        (status = 201, description = "Transaction started", body = TransactionResponse),
    )
)]
pub async fn begin_transaction(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<TransactionRequest>,
) -> ApiResult<AxumJson<TransactionResponse>> {
    // Begin a real transaction using the transaction manager
    let txn_id = match TXN_MANAGER.begin() {
        Ok(id) => id,
        Err(e) => {
            return Err(ApiError::new(
                "TRANSACTION_ERROR",
                format!("Failed to begin transaction: {}", e),
            ));
        }
    };

    let isolation_level = request
        .isolation_level
        .unwrap_or_else(|| "READ_COMMITTED".to_string());

    let response = TransactionResponse {
        transaction_id: TransactionId(txn_id),
        isolation_level,
        started_at: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        status: "active".to_string(),
    };

    Ok(AxumJson(response))
}

/// Commit a transaction
#[utoipa::path(
    post,
    path = "/api/v1/transactions/{id}/commit",
    tag = "database",
    params(
        ("id" = u64, Path, description = "Transaction ID")
    ),
    responses(
        (status = 200, description = "Transaction committed"),
        (status = 404, description = "Transaction not found", body = ApiError),
    )
)]
pub async fn commit_transaction(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    // Verify transaction exists
    if !TXN_MANAGER.is_active(id) {
        return Err(ApiError::new(
            "NOT_FOUND",
            format!("Transaction {} not found or already completed", id),
        ));
    }

    // Commit the transaction
    match TXN_MANAGER.commit(id) {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err(ApiError::new(
            "TRANSACTION_ERROR",
            format!("Failed to commit transaction: {}", e),
        )),
    }
}

/// Rollback a transaction
#[utoipa::path(
    post,
    path = "/api/v1/transactions/{id}/rollback",
    tag = "database",
    params(
        ("id" = u64, Path, description = "Transaction ID")
    ),
    responses(
        (status = 200, description = "Transaction rolled back"),
        (status = 404, description = "Transaction not found", body = ApiError),
    )
)]
pub async fn rollback_transaction(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    // Verify transaction exists
    if !TXN_MANAGER.is_active(id) {
        return Err(ApiError::new(
            "NOT_FOUND",
            format!("Transaction {} not found or already completed", id),
        ));
    }

    // Abort/rollback the transaction
    match TXN_MANAGER.abort(id) {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err(ApiError::new(
            "TRANSACTION_ERROR",
            format!("Failed to rollback transaction: {}", e),
        )),
    }
}
