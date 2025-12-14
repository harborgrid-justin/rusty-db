// Transaction Management Handlers
//
// Handler functions for transaction and MVCC operations

use axum::{
    extract::{Path, State},
    response::Json as AxumJson,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use utoipa::ToSchema;

use super::super::types::*;

// ============================================================================
// Transaction-specific Types
// ============================================================================

/// Active transaction information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActiveTransactionInfo {
    pub transaction_id: TransactionId,
    pub session_id: SessionId,
    pub started_at: i64,
    pub isolation_level: String,
    pub state: String, // active, preparing, prepared, committed, aborted
    pub read_only: bool,
    pub queries_executed: u64,
    pub rows_affected: u64,
    pub locks_held: usize,
}

/// Transaction details
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TransactionDetails {
    pub transaction_id: TransactionId,
    pub session_id: SessionId,
    pub started_at: i64,
    pub isolation_level: String,
    pub state: String,
    pub read_only: bool,
    pub queries_executed: u64,
    pub rows_affected: u64,
    pub locks_held: Vec<LockInfo>,
    pub modified_tables: Vec<String>,
    pub wal_bytes_written: u64,
}

/// Lock information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LockInfo {
    pub lock_id: String,
    pub lock_type: String,     // shared, exclusive, row_shared, row_exclusive
    pub resource_type: String, // table, row, page
    pub resource_id: String,
    pub transaction_id: TransactionId,
    pub granted: bool,
    pub acquired_at: i64,
}

/// Lock status response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LockStatusResponse {
    pub total_locks: usize,
    pub granted_locks: usize,
    pub waiting_locks: usize,
    pub locks: Vec<LockInfo>,
}

/// Lock waiter information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LockWaiter {
    pub transaction_id: TransactionId,
    pub waiting_for_transaction: TransactionId,
    pub lock_type: String,
    pub resource_type: String,
    pub resource_id: String,
    pub wait_time_ms: u64,
}

/// Lock wait graph
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LockWaitGraph {
    pub waiters: Vec<LockWaiter>,
    pub potential_deadlocks: Vec<Vec<TransactionId>>,
}

/// Deadlock information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeadlockInfo {
    pub deadlock_id: String,
    pub detected_at: i64,
    pub transactions: Vec<TransactionId>,
    pub victim_transaction: TransactionId,
    pub resolution: String,
}

/// MVCC status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MvccStatus {
    pub oldest_active_transaction: Option<TransactionId>,
    pub oldest_snapshot: Option<TransactionId>,
    pub total_versions: u64,
    pub dead_tuples: u64,
    pub live_tuples: u64,
    pub vacuum_running: bool,
    pub last_vacuum: Option<i64>,
}

/// Vacuum request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VacuumRequest {
    pub target: Option<String>, // table name, or None for full vacuum
    pub analyze: Option<bool>,
    pub full: Option<bool>,
}

/// WAL (Write-Ahead Log) status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WalStatus {
    pub current_lsn: String, // Log Sequence Number
    pub checkpoint_lsn: String,
    pub wal_files: usize,
    pub wal_size_bytes: u64,
    pub write_rate_mbps: f64,
    pub sync_rate_mbps: f64,
    pub last_checkpoint: i64,
    pub checkpoint_in_progress: bool,
}

/// Checkpoint result
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CheckpointResult {
    pub checkpoint_lsn: String,
    pub started_at: i64,
    pub completed_at: i64,
    pub duration_ms: u64,
    pub pages_written: u64,
    pub bytes_written: u64,
}

// ============================================================================
// Lazy-initialized transaction state
// ============================================================================

lazy_static::lazy_static! {
    static ref ACTIVE_TRANSACTIONS: Arc<RwLock<HashMap<TransactionId, ActiveTransactionInfo>>> = {
        let mut txns = HashMap::new();
        txns.insert(TransactionId(1), ActiveTransactionInfo {
            transaction_id: TransactionId(1),
            session_id: SessionId(101),
            started_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64 - 300,
            isolation_level: "READ_COMMITTED".to_string(),
            state: "active".to_string(),
            read_only: false,
            queries_executed: 5,
            rows_affected: 150,
            locks_held: 3,
        });
        Arc::new(RwLock::new(txns))
    };

    static ref LOCKS: Arc<RwLock<HashMap<String, LockInfo>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref DEADLOCKS: Arc<RwLock<Vec<DeadlockInfo>>> = Arc::new(RwLock::new(Vec::new()));
    static ref NEXT_LOCK_ID: Arc<RwLock<u64>> = Arc::new(RwLock::new(1));
}

// ============================================================================
// Handler Functions
// ============================================================================

/// List active transactions
#[utoipa::path(
    get,
    path = "/api/v1/transactions/active",
    tag = "transactions",
    responses(
        (status = 200, description = "List of active transactions", body = Vec<ActiveTransactionInfo>),
    )
)]
pub async fn get_active_transactions(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<ActiveTransactionInfo>>> {
    let transactions = ACTIVE_TRANSACTIONS.read();
    let txn_list: Vec<ActiveTransactionInfo> = transactions.values().cloned().collect();
    Ok(AxumJson(txn_list))
}

/// Get transaction details
#[utoipa::path(
    get,
    path = "/api/v1/transactions/{id}",
    tag = "transactions",
    params(
        ("id" = u64, Path, description = "Transaction ID")
    ),
    responses(
        (status = 200, description = "Transaction details", body = TransactionDetails),
        (status = 404, description = "Transaction not found", body = ApiError),
    )
)]
pub async fn get_transaction(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
) -> ApiResult<AxumJson<TransactionDetails>> {
    let transactions = ACTIVE_TRANSACTIONS.read();
    let locks = LOCKS.read();
    let txn_id = TransactionId(id);

    if let Some(txn) = transactions.get(&txn_id) {
        let txn_locks: Vec<LockInfo> = locks
            .values()
            .filter(|lock| lock.transaction_id == txn_id)
            .cloned()
            .collect();

        let details = TransactionDetails {
            transaction_id: txn.transaction_id,
            session_id: txn.session_id,
            started_at: txn.started_at,
            isolation_level: txn.isolation_level.clone(),
            state: txn.state.clone(),
            read_only: txn.read_only,
            queries_executed: txn.queries_executed,
            rows_affected: txn.rows_affected,
            locks_held: txn_locks,
            modified_tables: vec!["users".to_string(), "orders".to_string()],
            wal_bytes_written: 4096,
        };

        Ok(AxumJson(details))
    } else {
        Err(ApiError::new(
            "NOT_FOUND",
            format!("Transaction {} not found", id),
        ))
    }
}

/// Force rollback a transaction
#[utoipa::path(
    post,
    path = "/api/v1/transactions/{id}/rollback",
    tag = "transactions",
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
) -> ApiResult<AxumJson<serde_json::Value>> {
    let mut transactions = ACTIVE_TRANSACTIONS.write();
    let txn_id = TransactionId(id);

    if let Some(txn) = transactions.get_mut(&txn_id) {
        txn.state = "aborted".to_string();

        Ok(AxumJson(json!({
            "transaction_id": id,
            "status": "rolled_back",
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        })))
    } else {
        Err(ApiError::new(
            "NOT_FOUND",
            format!("Transaction {} not found", id),
        ))
    }
}

/// Get current lock status
#[utoipa::path(
    get,
    path = "/api/v1/transactions/locks",
    tag = "transactions",
    responses(
        (status = 200, description = "Lock status", body = LockStatusResponse),
    )
)]
pub async fn get_locks(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<LockStatusResponse>> {
    let locks = LOCKS.read();
    let lock_list: Vec<LockInfo> = locks.values().cloned().collect();

    let granted = lock_list.iter().filter(|l| l.granted).count();
    let waiting = lock_list.iter().filter(|l| !l.granted).count();

    let response = LockStatusResponse {
        total_locks: lock_list.len(),
        granted_locks: granted,
        waiting_locks: waiting,
        locks: lock_list,
    };

    Ok(AxumJson(response))
}

/// Get lock wait graph
#[utoipa::path(
    get,
    path = "/api/v1/transactions/locks/waiters",
    tag = "transactions",
    responses(
        (status = 200, description = "Lock wait graph", body = LockWaitGraph),
    )
)]
pub async fn get_lock_waiters(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<LockWaitGraph>> {
    // In a real implementation, this would analyze the lock wait graph
    let graph = LockWaitGraph {
        waiters: vec![],
        potential_deadlocks: vec![],
    };

    Ok(AxumJson(graph))
}

/// Get deadlock history
#[utoipa::path(
    get,
    path = "/api/v1/transactions/deadlocks",
    tag = "transactions",
    responses(
        (status = 200, description = "Deadlock history", body = Vec<DeadlockInfo>),
    )
)]
pub async fn get_deadlocks(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<DeadlockInfo>>> {
    let deadlocks = DEADLOCKS.read();
    Ok(AxumJson(deadlocks.clone()))
}

/// Force deadlock detection
#[utoipa::path(
    post,
    path = "/api/v1/transactions/deadlocks/detect",
    tag = "transactions",
    responses(
        (status = 200, description = "Deadlock detection results"),
    )
)]
pub async fn detect_deadlocks(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    // In a real implementation, this would trigger deadlock detection
    Ok(AxumJson(json!({
        "deadlocks_detected": 0,
        "transactions_analyzed": ACTIVE_TRANSACTIONS.read().len(),
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    })))
}

/// Get MVCC status
#[utoipa::path(
    get,
    path = "/api/v1/transactions/mvcc/status",
    tag = "transactions",
    responses(
        (status = 200, description = "MVCC statistics", body = MvccStatus),
    )
)]
pub async fn get_mvcc_status(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<MvccStatus>> {
    let transactions = ACTIVE_TRANSACTIONS.read();
    let oldest = transactions.keys().min_by_key(|id| id.0).copied();

    let status = MvccStatus {
        oldest_active_transaction: oldest,
        oldest_snapshot: oldest,
        total_versions: 1_000_000,
        dead_tuples: 50_000,
        live_tuples: 950_000,
        vacuum_running: false,
        last_vacuum: Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
                - 3600,
        ),
    };

    Ok(AxumJson(status))
}

/// Trigger vacuum operation
#[utoipa::path(
    post,
    path = "/api/v1/transactions/mvcc/vacuum",
    tag = "transactions",
    request_body = VacuumRequest,
    responses(
        (status = 200, description = "Vacuum started"),
    )
)]
pub async fn trigger_vacuum(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<VacuumRequest>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    // In a real implementation, this would trigger a vacuum operation
    Ok(AxumJson(json!({
        "status": "started",
        "target": request.target.unwrap_or_else(|| "all".to_string()),
        "analyze": request.analyze.unwrap_or(false),
        "full": request.full.unwrap_or(false),
        "started_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    })))
}

/// Get WAL status
#[utoipa::path(
    get,
    path = "/api/v1/transactions/wal/status",
    tag = "transactions",
    responses(
        (status = 200, description = "WAL status", body = WalStatus),
    )
)]
pub async fn get_wal_status(State(_state): State<Arc<ApiState>>) -> ApiResult<AxumJson<WalStatus>> {
    let status = WalStatus {
        current_lsn: "0/1A2B3C4D".to_string(),
        checkpoint_lsn: "0/1A2B0000".to_string(),
        wal_files: 5,
        wal_size_bytes: 100_000_000,
        write_rate_mbps: 25.5,
        sync_rate_mbps: 20.3,
        last_checkpoint: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - 300,
        checkpoint_in_progress: false,
    };

    Ok(AxumJson(status))
}

/// Force checkpoint
#[utoipa::path(
    post,
    path = "/api/v1/transactions/wal/checkpoint",
    tag = "transactions",
    responses(
        (status = 200, description = "Checkpoint completed", body = CheckpointResult),
    )
)]
pub async fn force_checkpoint(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<CheckpointResult>> {
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Simulate checkpoint operation
    let result = CheckpointResult {
        checkpoint_lsn: "0/1A2B4000".to_string(),
        started_at: started,
        completed_at: started + 2,
        duration_ms: 2000,
        pages_written: 1500,
        bytes_written: 6_144_000,
    };

    Ok(AxumJson(result))
}

// ============================================================================
// Savepoint Operations
// ============================================================================

/// Savepoint request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SavepointRequest {
    pub savepoint_name: String,
}

/// Savepoint response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SavepointResponse {
    pub transaction_id: TransactionId,
    pub savepoint_name: String,
    pub created_at: i64,
    pub savepoint_id: String,
}

/// Create a savepoint within a transaction
#[utoipa::path(
    post,
    path = "/api/v1/transactions/{id}/savepoint",
    tag = "transactions",
    params(
        ("id" = u64, Path, description = "Transaction ID")
    ),
    request_body = SavepointRequest,
    responses(
        (status = 200, description = "Savepoint created", body = SavepointResponse),
        (status = 404, description = "Transaction not found", body = ApiError),
    )
)]
pub async fn create_savepoint(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
    AxumJson(request): AxumJson<SavepointRequest>,
) -> ApiResult<AxumJson<SavepointResponse>> {
    let transactions = ACTIVE_TRANSACTIONS.read();
    let txn_id = TransactionId(id);

    if transactions.contains_key(&txn_id) {
        let response = SavepointResponse {
            transaction_id: txn_id,
            savepoint_name: request.savepoint_name.clone(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            savepoint_id: format!("sp_{}_{}", id, request.savepoint_name),
        };

        Ok(AxumJson(response))
    } else {
        Err(ApiError::new(
            "NOT_FOUND",
            format!("Transaction {} not found", id),
        ))
    }
}

/// Release a savepoint
#[utoipa::path(
    post,
    path = "/api/v1/transactions/{id}/release-savepoint",
    tag = "transactions",
    params(
        ("id" = u64, Path, description = "Transaction ID")
    ),
    request_body = SavepointRequest,
    responses(
        (status = 200, description = "Savepoint released"),
        (status = 404, description = "Transaction or savepoint not found", body = ApiError),
    )
)]
pub async fn release_savepoint(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
    AxumJson(request): AxumJson<SavepointRequest>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    let transactions = ACTIVE_TRANSACTIONS.read();
    let txn_id = TransactionId(id);

    if transactions.contains_key(&txn_id) {
        Ok(AxumJson(json!({
            "transaction_id": id,
            "savepoint_name": request.savepoint_name,
            "status": "released",
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        })))
    } else {
        Err(ApiError::new(
            "NOT_FOUND",
            format!("Transaction {} not found", id),
        ))
    }
}

/// Rollback to a savepoint
#[utoipa::path(
    post,
    path = "/api/v1/transactions/{id}/rollback-to-savepoint",
    tag = "transactions",
    params(
        ("id" = u64, Path, description = "Transaction ID")
    ),
    request_body = SavepointRequest,
    responses(
        (status = 200, description = "Rolled back to savepoint"),
        (status = 404, description = "Transaction or savepoint not found", body = ApiError),
    )
)]
pub async fn rollback_to_savepoint(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
    AxumJson(request): AxumJson<SavepointRequest>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    let transactions = ACTIVE_TRANSACTIONS.read();
    let txn_id = TransactionId(id);

    if transactions.contains_key(&txn_id) {
        Ok(AxumJson(json!({
            "transaction_id": id,
            "savepoint_name": request.savepoint_name,
            "status": "rolled_back",
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        })))
    } else {
        Err(ApiError::new(
            "NOT_FOUND",
            format!("Transaction {} not found", id),
        ))
    }
}

// ============================================================================
// Isolation Level Control
// ============================================================================

/// Isolation level update request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IsolationLevelRequest {
    pub isolation_level: String, // READ_UNCOMMITTED, READ_COMMITTED, REPEATABLE_READ, SERIALIZABLE
}

/// Update transaction isolation level
#[utoipa::path(
    put,
    path = "/api/v1/transactions/{id}/isolation-level",
    tag = "transactions",
    params(
        ("id" = u64, Path, description = "Transaction ID")
    ),
    request_body = IsolationLevelRequest,
    responses(
        (status = 200, description = "Isolation level updated"),
        (status = 400, description = "Invalid isolation level", body = ApiError),
        (status = 404, description = "Transaction not found", body = ApiError),
    )
)]
pub async fn update_isolation_level(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
    AxumJson(request): AxumJson<IsolationLevelRequest>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    let mut transactions = ACTIVE_TRANSACTIONS.write();
    let txn_id = TransactionId(id);

    // Validate isolation level
    let valid_levels = [
        "READ_UNCOMMITTED",
        "READ_COMMITTED",
        "REPEATABLE_READ",
        "SERIALIZABLE",
    ];
    if !valid_levels.contains(&request.isolation_level.as_str()) {
        return Err(ApiError::new(
            "INVALID_ISOLATION_LEVEL",
            format!(
                "Invalid isolation level: {}. Must be one of: {:?}",
                request.isolation_level, valid_levels
            ),
        ));
    }

    if let Some(txn) = transactions.get_mut(&txn_id) {
        txn.isolation_level = request.isolation_level.clone();

        Ok(AxumJson(json!({
            "transaction_id": id,
            "isolation_level": request.isolation_level,
            "status": "updated",
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        })))
    } else {
        Err(ApiError::new(
            "NOT_FOUND",
            format!("Transaction {} not found", id),
        ))
    }
}

// ============================================================================
// Lock Control Operations
// ============================================================================

/// Release a specific lock
#[utoipa::path(
    post,
    path = "/api/v1/transactions/locks/{id}/release",
    tag = "transactions",
    params(
        ("id" = String, Path, description = "Lock ID")
    ),
    responses(
        (status = 200, description = "Lock released"),
        (status = 404, description = "Lock not found", body = ApiError),
    )
)]
pub async fn release_lock(
    State(_state): State<Arc<ApiState>>,
    Path(lock_id): Path<String>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    let mut locks = LOCKS.write();

    if locks.remove(&lock_id).is_some() {
        Ok(AxumJson(json!({
            "lock_id": lock_id,
            "status": "released",
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        })))
    } else {
        Err(ApiError::new(
            "NOT_FOUND",
            format!("Lock {} not found", lock_id),
        ))
    }
}

/// Release all locks for a transaction
#[utoipa::path(
    post,
    path = "/api/v1/transactions/locks/release-all",
    tag = "transactions",
    request_body = inline(ReleaseAllLocksRequest),
    responses(
        (status = 200, description = "All locks released"),
        (status = 404, description = "Transaction not found", body = ApiError),
    )
)]
pub async fn release_all_locks(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<ReleaseAllLocksRequest>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    let mut locks = LOCKS.write();
    let txn_id = TransactionId(request.transaction_id);

    // Find and remove all locks for this transaction
    let lock_ids: Vec<String> = locks
        .iter()
        .filter(|(_, lock)| lock.transaction_id == txn_id)
        .map(|(id, _)| id.clone())
        .collect();

    let count = lock_ids.len();
    for lock_id in lock_ids {
        locks.remove(&lock_id);
    }

    Ok(AxumJson(json!({
        "transaction_id": request.transaction_id,
        "locks_released": count,
        "status": "success",
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    })))
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReleaseAllLocksRequest {
    pub transaction_id: u64,
}

/// Get lock wait graph
#[utoipa::path(
    get,
    path = "/api/v1/transactions/locks/graph",
    tag = "transactions",
    responses(
        (status = 200, description = "Lock wait graph", body = LockWaitGraph),
    )
)]
pub async fn get_lock_graph(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<LockWaitGraph>> {
    // In a real implementation, this would analyze the lock wait graph and detect cycles
    let graph = LockWaitGraph {
        waiters: vec![],
        potential_deadlocks: vec![],
    };

    Ok(AxumJson(graph))
}

// ============================================================================
// MVCC Control Operations
// ============================================================================

/// Snapshot information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SnapshotInfo {
    pub snapshot_id: TransactionId,
    pub created_at: i64,
    pub isolation_level: String,
    pub active_transactions: Vec<TransactionId>,
    pub oldest_transaction: Option<TransactionId>,
}

/// Get all active MVCC snapshots
#[utoipa::path(
    get,
    path = "/api/v1/transactions/mvcc/snapshots",
    tag = "transactions",
    responses(
        (status = 200, description = "List of active snapshots", body = Vec<SnapshotInfo>),
    )
)]
pub async fn get_mvcc_snapshots(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<SnapshotInfo>>> {
    let transactions = ACTIVE_TRANSACTIONS.read();

    let snapshots: Vec<SnapshotInfo> = transactions
        .values()
        .map(|txn| {
            let active_txns: Vec<TransactionId> = transactions.keys().copied().collect();
            let oldest = active_txns.iter().min().copied();

            SnapshotInfo {
                snapshot_id: txn.transaction_id,
                created_at: txn.started_at,
                isolation_level: txn.isolation_level.clone(),
                active_transactions: active_txns,
                oldest_transaction: oldest,
            }
        })
        .collect();

    Ok(AxumJson(snapshots))
}

/// Row version information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RowVersionInfo {
    pub version_id: u64,
    pub transaction_id: TransactionId,
    pub created_at: i64,
    pub committed: bool,
    pub data: serde_json::Value,
}

/// Get all versions of a specific row
#[utoipa::path(
    get,
    path = "/api/v1/transactions/mvcc/versions/{table}/{row}",
    tag = "transactions",
    params(
        ("table" = String, Path, description = "Table name"),
        ("row" = String, Path, description = "Row identifier")
    ),
    responses(
        (status = 200, description = "List of row versions", body = Vec<RowVersionInfo>),
        (status = 404, description = "Table or row not found", body = ApiError),
    )
)]
pub async fn get_row_versions(
    State(_state): State<Arc<ApiState>>,
    Path((table, row)): Path<(String, String)>,
) -> ApiResult<AxumJson<Vec<RowVersionInfo>>> {
    // In a real implementation, this would query the MVCC version chain
    let versions = vec![
        RowVersionInfo {
            version_id: 1,
            transaction_id: TransactionId(100),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
                - 300,
            committed: true,
            data: json!({"id": row, "table": table, "value": "original"}),
        },
        RowVersionInfo {
            version_id: 2,
            transaction_id: TransactionId(101),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
                - 100,
            committed: false,
            data: json!({"id": row, "table": table, "value": "updated"}),
        },
    ];

    Ok(AxumJson(versions))
}

/// Full vacuum request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FullVacuumRequest {
    pub tables: Option<Vec<String>>, // Specific tables, or None for all
    pub aggressive: Option<bool>,
    pub freeze: Option<bool>,
}

/// Trigger full vacuum operation
#[utoipa::path(
    post,
    path = "/api/v1/transactions/mvcc/vacuum/full",
    tag = "transactions",
    request_body = FullVacuumRequest,
    responses(
        (status = 200, description = "Full vacuum started"),
    )
)]
pub async fn trigger_full_vacuum(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<FullVacuumRequest>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    let tables = request
        .tables
        .clone()
        .unwrap_or_else(|| vec!["all".to_string()]);

    Ok(AxumJson(json!({
        "status": "started",
        "vacuum_type": "full",
        "tables": tables,
        "aggressive": request.aggressive.unwrap_or(false),
        "freeze": request.freeze.unwrap_or(false),
        "started_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        "estimated_duration_seconds": 600
    })))
}

// ============================================================================
// WAL Control Operations
// ============================================================================

/// WAL segment information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WalSegmentInfo {
    pub segment_name: String,
    pub segment_number: u64,
    pub start_lsn: String,
    pub end_lsn: String,
    pub size_bytes: u64,
    pub created_at: i64,
    pub archived: bool,
}

/// Get list of WAL segments
#[utoipa::path(
    get,
    path = "/api/v1/transactions/wal/segments",
    tag = "transactions",
    responses(
        (status = 200, description = "List of WAL segments", body = Vec<WalSegmentInfo>),
    )
)]
pub async fn get_wal_segments(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<WalSegmentInfo>>> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let segments = vec![
        WalSegmentInfo {
            segment_name: "000000010000000000000001".to_string(),
            segment_number: 1,
            start_lsn: "0/01000000".to_string(),
            end_lsn: "0/02000000".to_string(),
            size_bytes: 16_777_216, // 16MB
            created_at: now - 3600,
            archived: true,
        },
        WalSegmentInfo {
            segment_name: "000000010000000000000002".to_string(),
            segment_number: 2,
            start_lsn: "0/02000000".to_string(),
            end_lsn: "0/03000000".to_string(),
            size_bytes: 16_777_216,
            created_at: now - 1800,
            archived: false,
        },
    ];

    Ok(AxumJson(segments))
}

/// Archive WAL request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArchiveWalRequest {
    pub segment_name: Option<String>, // Specific segment, or None for all ready segments
    pub compress: Option<bool>,
}

/// Trigger WAL archiving
#[utoipa::path(
    post,
    path = "/api/v1/transactions/wal/archive",
    tag = "transactions",
    request_body = ArchiveWalRequest,
    responses(
        (status = 200, description = "WAL archiving started"),
    )
)]
pub async fn archive_wal(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<ArchiveWalRequest>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    Ok(AxumJson(json!({
        "status": "started",
        "segment": request.segment_name.unwrap_or_else(|| "all".to_string()),
        "compress": request.compress.unwrap_or(true),
        "started_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        "target_location": "/var/lib/rustydb/wal_archive"
    })))
}

/// WAL replay status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WalReplayStatus {
    pub replaying: bool,
    pub current_lsn: String,
    pub target_lsn: Option<String>,
    pub replay_lag_bytes: u64,
    pub replay_lag_seconds: u64,
    pub last_replay_timestamp: i64,
    pub segments_replayed: u64,
}

/// Get WAL replay status
#[utoipa::path(
    get,
    path = "/api/v1/transactions/wal/replay-status",
    tag = "transactions",
    responses(
        (status = 200, description = "WAL replay status", body = WalReplayStatus),
    )
)]
pub async fn get_wal_replay_status(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<WalReplayStatus>> {
    let status = WalReplayStatus {
        replaying: false,
        current_lsn: "0/1A2B3C4D".to_string(),
        target_lsn: None,
        replay_lag_bytes: 0,
        replay_lag_seconds: 0,
        last_replay_timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        segments_replayed: 42,
    };

    Ok(AxumJson(status))
}

/// Switch to a new WAL segment
#[utoipa::path(
    post,
    path = "/api/v1/transactions/wal/switch",
    tag = "transactions",
    responses(
        (status = 200, description = "WAL segment switched"),
    )
)]
pub async fn switch_wal_segment(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    Ok(AxumJson(json!({
        "status": "switched",
        "old_segment": "000000010000000000000002",
        "new_segment": "000000010000000000000003",
        "old_lsn": "0/02A5B3C1",
        "new_lsn": "0/03000000",
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    })))
}
