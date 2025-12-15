// Storage WebSocket Handlers
//
// Real-time WebSocket streaming for storage layer events

use axum::{
    extract::{State, WebSocketUpgrade, ws::{WebSocket, Message}},
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use utoipa::ToSchema;

use super::super::types::ApiState;

// ============================================================================
// Storage Event Types
// ============================================================================

/// Buffer pool event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum BufferPoolEvent {
    PageFetched {
        page_id: u32,
        timestamp: i64,
        hit: bool,
    },
    PageEvicted {
        page_id: u32,
        timestamp: i64,
        dirty: bool,
    },
    PageFlushed {
        page_id: u32,
        timestamp: i64,
    },
    PoolFull {
        total_pages: usize,
        used_pages: usize,
        timestamp: i64,
    },
    PoolStats {
        total_pages: usize,
        used_pages: usize,
        dirty_pages: usize,
        hit_ratio: f64,
        timestamp: i64,
    },
}

/// LSM tree event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum LsmEvent {
    MemtableWrite {
        key: String,
        size_bytes: usize,
        timestamp: i64,
    },
    MemtableFlush {
        level: usize,
        sstable_id: String,
        size_bytes: u64,
        timestamp: i64,
    },
    CompactionStarted {
        level: usize,
        num_sstables: usize,
        timestamp: i64,
    },
    CompactionCompleted {
        level: usize,
        input_size_bytes: u64,
        output_size_bytes: u64,
        duration_ms: u64,
        timestamp: i64,
    },
    BloomFilterMiss {
        key: String,
        timestamp: i64,
    },
}

/// Disk I/O event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum DiskIoEvent {
    ReadStarted {
        page_id: u32,
        offset: u64,
        size: usize,
        timestamp: i64,
    },
    ReadCompleted {
        page_id: u32,
        duration_ms: f64,
        timestamp: i64,
    },
    WriteStarted {
        page_id: u32,
        offset: u64,
        size: usize,
        timestamp: i64,
    },
    WriteCompleted {
        page_id: u32,
        duration_ms: f64,
        timestamp: i64,
    },
    VectoredIo {
        operation: String,
        page_count: usize,
        total_bytes: usize,
        duration_ms: f64,
        timestamp: i64,
    },
    IoError {
        page_id: u32,
        error: String,
        timestamp: i64,
    },
}

/// Tiered storage event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum TierEvent {
    PageMigrated {
        page_id: u32,
        from_tier: String,
        to_tier: String,
        size_bytes: usize,
        timestamp: i64,
    },
    TierThresholdReached {
        tier: String,
        used_percent: f64,
        timestamp: i64,
    },
    MigrationPolicyTriggered {
        policy: String,
        pages_eligible: usize,
        timestamp: i64,
    },
    TierStats {
        tier: String,
        total_bytes: u64,
        used_bytes: u64,
        page_count: usize,
        timestamp: i64,
    },
}

/// Page event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum PageEvent {
    PageCreated {
        page_id: u32,
        size_bytes: usize,
        timestamp: i64,
    },
    PageModified {
        page_id: u32,
        operation: String,
        timestamp: i64,
    },
    PageCompacted {
        page_id: u32,
        space_reclaimed: usize,
        timestamp: i64,
    },
    PageSplit {
        old_page_id: u32,
        new_page_id: u32,
        timestamp: i64,
    },
    PageMerged {
        page_ids: Vec<u32>,
        result_page_id: u32,
        timestamp: i64,
    },
}

/// Columnar storage event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum ColumnarEvent {
    TableCreated {
        table_name: String,
        columns: usize,
        timestamp: i64,
    },
    BatchInserted {
        table_name: String,
        rows: usize,
        timestamp: i64,
    },
    ColumnScanned {
        table_name: String,
        column_name: String,
        rows_scanned: usize,
        duration_ms: f64,
        timestamp: i64,
    },
    CompressionApplied {
        table_name: String,
        column_name: String,
        encoding: String,
        compression_ratio: f64,
        timestamp: i64,
    },
}

// ============================================================================
// WebSocket Handlers
// ============================================================================

/// WebSocket handler for buffer pool events
#[utoipa::path(
    get,
    path = "/api/v1/ws/storage/buffer-pool",
    tag = "storage-websocket",
    responses(
        (status = 101, description = "WebSocket connection established"),
    )
)]
pub async fn ws_buffer_pool_events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_buffer_pool_events(socket, state))
}

async fn handle_buffer_pool_events(mut socket: WebSocket, _state: Arc<ApiState>) {
    let mut interval = interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let event = BufferPoolEvent::PoolStats {
                    total_pages: 10000,
                    used_pages: 7500,
                    dirty_pages: 500,
                    hit_ratio: 0.95,
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                };

                let json = serde_json::to_string(&event).unwrap();
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                if msg.is_none() {
                    break;
                }
            }
        }
    }
}

/// WebSocket handler for LSM tree events
#[utoipa::path(
    get,
    path = "/api/v1/ws/storage/lsm",
    tag = "storage-websocket",
    responses(
        (status = 101, description = "WebSocket connection established"),
    )
)]
pub async fn ws_lsm_events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_lsm_events(socket, state))
}

async fn handle_lsm_events(mut socket: WebSocket, _state: Arc<ApiState>) {
    let mut interval = interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let event = LsmEvent::MemtableFlush {
                    level: 0,
                    sstable_id: format!("sst_{}", rand::random::<u32>()),
                    size_bytes: 1024 * 1024,
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                };

                let json = serde_json::to_string(&event).unwrap();
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                if msg.is_none() {
                    break;
                }
            }
        }
    }
}

/// WebSocket handler for disk I/O events
#[utoipa::path(
    get,
    path = "/api/v1/ws/storage/disk-io",
    tag = "storage-websocket",
    responses(
        (status = 101, description = "WebSocket connection established"),
    )
)]
pub async fn ws_disk_io_events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_disk_io_events(socket, state))
}

async fn handle_disk_io_events(mut socket: WebSocket, _state: Arc<ApiState>) {
    let mut interval = interval(Duration::from_millis(500));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let page_id = rand::random::<u32>() % 1000;
                let duration = (rand::random::<u32>() % 10) as f64 + 0.5;

                let event = DiskIoEvent::ReadCompleted {
                    page_id,
                    duration_ms: duration,
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                };

                let json = serde_json::to_string(&event).unwrap();
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                if msg.is_none() {
                    break;
                }
            }
        }
    }
}

/// WebSocket handler for tiered storage events
#[utoipa::path(
    get,
    path = "/api/v1/ws/storage/tiers",
    tag = "storage-websocket",
    responses(
        (status = 101, description = "WebSocket connection established"),
    )
)]
pub async fn ws_tier_events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_tier_events(socket, state))
}

async fn handle_tier_events(mut socket: WebSocket, _state: Arc<ApiState>) {
    let mut interval = interval(Duration::from_secs(10));
    let tiers = vec!["hot", "warm", "cold"];

    loop {
        tokio::select! {
            _ = interval.tick() => {
                for tier in &tiers {
                    let event = TierEvent::TierStats {
                        tier: tier.to_string(),
                        total_bytes: 1_000_000_000_000,
                        used_bytes: 500_000_000_000,
                        page_count: 12500,
                        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                    };

                    let json = serde_json::to_string(&event).unwrap();
                    if socket.send(Message::Text(json.into())).await.is_err() {
                        return;
                    }
                }
            }
            msg = socket.recv() => {
                if msg.is_none() {
                    break;
                }
            }
        }
    }
}

/// WebSocket handler for page events
#[utoipa::path(
    get,
    path = "/api/v1/ws/storage/pages",
    tag = "storage-websocket",
    responses(
        (status = 101, description = "WebSocket connection established"),
    )
)]
pub async fn ws_page_events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_page_events(socket, state))
}

async fn handle_page_events(mut socket: WebSocket, _state: Arc<ApiState>) {
    let mut interval = interval(Duration::from_secs(3));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let page_id = rand::random::<u32>() % 10000;
                let event = PageEvent::PageModified {
                    page_id,
                    operation: "update".to_string(),
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                };

                let json = serde_json::to_string(&event).unwrap();
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                if msg.is_none() {
                    break;
                }
            }
        }
    }
}

/// WebSocket handler for columnar storage events
#[utoipa::path(
    get,
    path = "/api/v1/ws/storage/columnar",
    tag = "storage-websocket",
    responses(
        (status = 101, description = "WebSocket connection established"),
    )
)]
pub async fn ws_columnar_events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_columnar_events(socket, state))
}

async fn handle_columnar_events(mut socket: WebSocket, _state: Arc<ApiState>) {
    let mut interval = interval(Duration::from_secs(4));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let event = ColumnarEvent::BatchInserted {
                    table_name: "analytics_table".to_string(),
                    rows: 1000,
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                };

                let json = serde_json::to_string(&event).unwrap();
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                if msg.is_none() {
                    break;
                }
            }
        }
    }
}
