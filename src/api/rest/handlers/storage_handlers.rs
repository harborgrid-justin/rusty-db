// Storage Management Handlers
//
// Handler functions for storage and disk management operations

use axum::{
    extract::{Path, State},
    response::Json as AxumJson,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use utoipa::ToSchema;
use parking_lot::RwLock;

use super::super::types::*;

// ============================================================================
// Storage-specific Types
// ============================================================================

/// Overall storage status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StorageStatus {
    pub total_space_bytes: u64,
    pub used_space_bytes: u64,
    pub available_space_bytes: u64,
    pub utilization_percent: f64,
    pub disk_count: usize,
    pub partition_count: usize,
    pub tablespace_count: usize,
}

/// Disk device information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DiskInfo {
    pub disk_id: String,
    pub device_path: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub read_iops: u64,
    pub write_iops: u64,
    pub read_throughput_mbps: f64,
    pub write_throughput_mbps: f64,
    pub avg_latency_ms: f64,
}

/// Partition information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PartitionInfo {
    pub partition_id: String,
    pub table_name: String,
    pub partition_name: String,
    pub partition_type: String, // range, list, hash
    pub partition_key: String,
    pub partition_value: String,
    pub row_count: u64,
    pub size_bytes: u64,
    pub created_at: i64,
}

/// Create partition request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePartitionRequest {
    pub table_name: String,
    pub partition_name: String,
    pub partition_type: String,
    pub partition_key: String,
    pub partition_value: String,
}

/// Buffer pool statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BufferPoolStats {
    pub total_pages: usize,
    pub used_pages: usize,
    pub free_pages: usize,
    pub dirty_pages: usize,
    pub hit_ratio: f64,
    pub evictions: u64,
    pub reads: u64,
    pub writes: u64,
    pub flushes: u64,
}

/// Tablespace information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TablespaceInfo {
    pub tablespace_id: String,
    pub name: String,
    pub location: String,
    pub size_bytes: u64,
    pub used_bytes: u64,
    pub auto_extend: bool,
    pub max_size_bytes: Option<u64>,
    pub status: String, // online, offline
}

/// Create tablespace request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateTablespaceRequest {
    pub name: String,
    pub location: String,
    pub initial_size_mb: u64,
    pub auto_extend: Option<bool>,
    pub max_size_mb: Option<u64>,
}

/// Update tablespace request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateTablespaceRequest {
    pub auto_extend: Option<bool>,
    pub max_size_mb: Option<u64>,
    pub status: Option<String>,
}

/// I/O statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IoStats {
    pub total_reads: u64,
    pub total_writes: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub avg_read_latency_ms: f64,
    pub avg_write_latency_ms: f64,
    pub read_iops: f64,
    pub write_iops: f64,
    pub timestamp: i64,
}

// ============================================================================
// Lazy-initialized storage state
// ============================================================================

lazy_static::lazy_static! {
    static ref DISKS: Arc<RwLock<HashMap<String, DiskInfo>>> = {
        let mut disks = HashMap::new();
        disks.insert("disk0".to_string(), DiskInfo {
            disk_id: "disk0".to_string(),
            device_path: "/dev/sda1".to_string(),
            mount_point: "/data".to_string(),
            total_bytes: 1_000_000_000_000,
            used_bytes: 500_000_000_000,
            available_bytes: 500_000_000_000,
            read_iops: 1000,
            write_iops: 800,
            read_throughput_mbps: 150.5,
            write_throughput_mbps: 120.3,
            avg_latency_ms: 2.5,
        });
        Arc::new(RwLock::new(disks))
    };

    static ref PARTITIONS: Arc<RwLock<HashMap<String, PartitionInfo>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref TABLESPACES: Arc<RwLock<HashMap<String, TablespaceInfo>>> = {
        let mut tablespaces = HashMap::new();
        tablespaces.insert("system".to_string(), TablespaceInfo {
            tablespace_id: "ts_system".to_string(),
            name: "system".to_string(),
            location: "/data/tablespaces/system".to_string(),
            size_bytes: 10_000_000_000,
            used_bytes: 5_000_000_000,
            auto_extend: true,
            max_size_bytes: Some(50_000_000_000),
            status: "online".to_string(),
        });
        Arc::new(RwLock::new(tablespaces))
    };
    static ref NEXT_PARTITION_ID: Arc<RwLock<u64>> = Arc::new(RwLock::new(1));
}

// ============================================================================
// Handler Functions
// ============================================================================

/// Get overall storage status
#[utoipa::path(
    get,
    path = "/api/v1/storage/status",
    tag = "storage",
    responses(
        (status = 200, description = "Storage status", body = StorageStatus),
    )
)]
pub async fn get_storage_status(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<StorageStatus>> {
    let disks = DISKS.read();
    let partitions = PARTITIONS.read();
    let tablespaces = TABLESPACES.read();

    let total_space: u64 = disks.values().map(|d| d.total_bytes).sum();
    let used_space: u64 = disks.values().map(|d| d.used_bytes).sum();
    let available_space: u64 = disks.values().map(|d| d.available_bytes).sum();
    let utilization = if total_space > 0 {
        (used_space as f64 / total_space as f64) * 100.0
    } else {
        0.0
    };

    let response = StorageStatus {
        total_space_bytes: total_space,
        used_space_bytes: used_space,
        available_space_bytes: available_space,
        utilization_percent: utilization,
        disk_count: disks.len(),
        partition_count: partitions.len(),
        tablespace_count: tablespaces.len(),
    };

    Ok(AxumJson(response))
}

/// List disk devices and statistics
#[utoipa::path(
    get,
    path = "/api/v1/storage/disks",
    tag = "storage",
    responses(
        (status = 200, description = "List of disks", body = Vec<DiskInfo>),
    )
)]
pub async fn get_disks(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<DiskInfo>>> {
    let disks = DISKS.read();
    let disk_list: Vec<DiskInfo> = disks.values().cloned().collect();
    Ok(AxumJson(disk_list))
}

/// List all partitions
#[utoipa::path(
    get,
    path = "/api/v1/storage/partitions",
    tag = "storage",
    responses(
        (status = 200, description = "List of partitions", body = Vec<PartitionInfo>),
    )
)]
pub async fn get_partitions(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<PartitionInfo>>> {
    let partitions = PARTITIONS.read();
    let partition_list: Vec<PartitionInfo> = partitions.values().cloned().collect();
    Ok(AxumJson(partition_list))
}

/// Create a new partition
#[utoipa::path(
    post,
    path = "/api/v1/storage/partitions",
    tag = "storage",
    request_body = CreatePartitionRequest,
    responses(
        (status = 201, description = "Partition created", body = PartitionInfo),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn create_partition(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<CreatePartitionRequest>,
) -> ApiResult<(StatusCode, AxumJson<PartitionInfo>)> {
    let mut partitions = PARTITIONS.write();
    let mut next_id = NEXT_PARTITION_ID.write();

    let partition_id = format!("part_{}", *next_id);
    *next_id += 1;

    let partition = PartitionInfo {
        partition_id: partition_id.clone(),
        table_name: request.table_name,
        partition_name: request.partition_name,
        partition_type: request.partition_type,
        partition_key: request.partition_key,
        partition_value: request.partition_value,
        row_count: 0,
        size_bytes: 0,
        created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
    };

    partitions.insert(partition_id, partition.clone());

    Ok((StatusCode::CREATED, AxumJson(partition)))
}

/// Delete a partition
#[utoipa::path(
    delete,
    path = "/api/v1/storage/partitions/{id}",
    tag = "storage",
    params(
        ("id" = String, Path, description = "Partition ID")
    ),
    responses(
        (status = 204, description = "Partition deleted"),
        (status = 404, description = "Partition not found", body = ApiError),
    )
)]
pub async fn delete_partition(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let mut partitions = PARTITIONS.write();

    if partitions.remove(&id).is_some() {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new("NOT_FOUND", format!("Partition {} not found", id)))
    }
}

/// Get buffer pool statistics
#[utoipa::path(
    get,
    path = "/api/v1/storage/buffer-pool",
    tag = "storage",
    responses(
        (status = 200, description = "Buffer pool statistics", body = BufferPoolStats),
    )
)]
pub async fn get_buffer_pool_stats(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<BufferPoolStats>> {
    let stats = BufferPoolStats {
        total_pages: 10000,
        used_pages: 7500,
        free_pages: 2500,
        dirty_pages: 500,
        hit_ratio: 0.95,
        evictions: 1000,
        reads: 50000,
        writes: 25000,
        flushes: 500,
    };

    Ok(AxumJson(stats))
}

/// Flush buffer pool
#[utoipa::path(
    post,
    path = "/api/v1/storage/buffer-pool/flush",
    tag = "storage",
    responses(
        (status = 200, description = "Buffer pool flushed"),
    )
)]
pub async fn flush_buffer_pool(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    // In a real implementation, this would flush the buffer pool to disk
    Ok(AxumJson(json!({
        "status": "success",
        "pages_flushed": 500,
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    })))
}

/// List tablespaces
#[utoipa::path(
    get,
    path = "/api/v1/storage/tablespaces",
    tag = "storage",
    responses(
        (status = 200, description = "List of tablespaces", body = Vec<TablespaceInfo>),
    )
)]
pub async fn get_tablespaces(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<TablespaceInfo>>> {
    let tablespaces = TABLESPACES.read();
    let tablespace_list: Vec<TablespaceInfo> = tablespaces.values().cloned().collect();
    Ok(AxumJson(tablespace_list))
}

/// Create a new tablespace
#[utoipa::path(
    post,
    path = "/api/v1/storage/tablespaces",
    tag = "storage",
    request_body = CreateTablespaceRequest,
    responses(
        (status = 201, description = "Tablespace created", body = TablespaceInfo),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn create_tablespace(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<CreateTablespaceRequest>,
) -> ApiResult<(StatusCode, AxumJson<TablespaceInfo>)> {
    let mut tablespaces = TABLESPACES.write();

    let tablespace_id = format!("ts_{}", request.name);
    let initial_bytes = request.initial_size_mb * 1024 * 1024;

    let tablespace = TablespaceInfo {
        tablespace_id: tablespace_id.clone(),
        name: request.name.clone(),
        location: request.location,
        size_bytes: initial_bytes,
        used_bytes: 0,
        auto_extend: request.auto_extend.unwrap_or(true),
        max_size_bytes: request.max_size_mb.map(|mb| mb * 1024 * 1024),
        status: "online".to_string(),
    };

    tablespaces.insert(request.name, tablespace.clone());

    Ok((StatusCode::CREATED, AxumJson(tablespace)))
}

/// Update a tablespace
#[utoipa::path(
    put,
    path = "/api/v1/storage/tablespaces/{id}",
    tag = "storage",
    params(
        ("id" = String, Path, description = "Tablespace name")
    ),
    request_body = UpdateTablespaceRequest,
    responses(
        (status = 200, description = "Tablespace updated", body = TablespaceInfo),
        (status = 404, description = "Tablespace not found", body = ApiError),
    )
)]
pub async fn update_tablespace(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    AxumJson(request): AxumJson<UpdateTablespaceRequest>,
) -> ApiResult<AxumJson<TablespaceInfo>> {
    let mut tablespaces = TABLESPACES.write();

    if let Some(tablespace) = tablespaces.get_mut(&id) {
        if let Some(auto_extend) = request.auto_extend {
            tablespace.auto_extend = auto_extend;
        }
        if let Some(max_size_mb) = request.max_size_mb {
            tablespace.max_size_bytes = Some(max_size_mb * 1024 * 1024);
        }
        if let Some(status) = request.status {
            tablespace.status = status;
        }

        Ok(AxumJson(tablespace.clone()))
    } else {
        Err(ApiError::new("NOT_FOUND", format!("Tablespace {} not found", id)))
    }
}

/// Delete a tablespace
#[utoipa::path(
    delete,
    path = "/api/v1/storage/tablespaces/{id}",
    tag = "storage",
    params(
        ("id" = String, Path, description = "Tablespace name")
    ),
    responses(
        (status = 204, description = "Tablespace deleted"),
        (status = 404, description = "Tablespace not found", body = ApiError),
    )
)]
pub async fn delete_tablespace(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let mut tablespaces = TABLESPACES.write();

    if tablespaces.remove(&id).is_some() {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new("NOT_FOUND", format!("Tablespace {} not found", id)))
    }
}

/// Get I/O statistics
#[utoipa::path(
    get,
    path = "/api/v1/storage/io-stats",
    tag = "storage",
    responses(
        (status = 200, description = "I/O statistics", body = IoStats),
    )
)]
pub async fn get_io_stats(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<IoStats>> {
    let stats = IoStats {
        total_reads: 1_000_000,
        total_writes: 500_000,
        bytes_read: 100_000_000_000,
        bytes_written: 50_000_000_000,
        avg_read_latency_ms: 2.5,
        avg_write_latency_ms: 3.2,
        read_iops: 1500.0,
        write_iops: 800.0,
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
    };

    Ok(AxumJson(stats))
}
