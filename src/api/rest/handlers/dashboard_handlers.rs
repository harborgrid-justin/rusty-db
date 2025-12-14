// Dashboard Handlers
//
// REST API handlers for dashboard management

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json as AxumJson,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use utoipa::ToSchema;

use super::super::types::*;

// Dashboard request/response types
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DashboardRequest {
    pub name: String,
    pub description: Option<String>,
    pub widgets: Vec<WidgetRequest>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WidgetRequest {
    pub title: String,
    pub widget_type: String, // line_chart, area_chart, bar_chart, gauge, counter, table, heatmap, alert
    pub queries: Vec<QueryConfig>,
    pub refresh_interval_seconds: Option<u64>,
    pub position: WidgetPositionRequest,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QueryConfig {
    pub metric_name: String,
    pub aggregation: String, // avg, sum, min, max, count, rate, p50, p95, p99
    pub step_seconds: Option<u64>,
    pub labels: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WidgetPositionRequest {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DashboardResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub widgets: Vec<WidgetResponse>,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub created_by: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WidgetResponse {
    pub id: String,
    pub title: String,
    pub widget_type: String,
    pub queries: Vec<QueryConfig>,
    pub refresh_interval_seconds: u64,
    pub position: WidgetPositionRequest,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DashboardListResponse {
    pub dashboards: Vec<DashboardSummary>,
    pub total_count: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DashboardSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub widget_count: usize,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub created_by: String,
}

/// POST /api/v1/dashboards
///
/// Create a new dashboard
#[utoipa::path(
    post,
    path = "/api/v1/dashboards",
    tag = "dashboards",
    request_body = DashboardRequest,
    responses(
        (status = 201, description = "Dashboard created", body = DashboardResponse),
    )
)]
pub async fn create_dashboard(
    State(_state): State<Arc<ApiState>>,
    axum::Json(request): axum::Json<DashboardRequest>,
) -> ApiResult<(StatusCode, AxumJson<DashboardResponse>)> {
    // In a real implementation, this would create the dashboard in DashboardManager
    let dashboard_id = uuid::Uuid::new_v4().to_string();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let widgets: Vec<WidgetResponse> = request
        .widgets
        .into_iter()
        .map(|w| WidgetResponse {
            id: uuid::Uuid::new_v4().to_string(),
            title: w.title,
            widget_type: w.widget_type,
            queries: w.queries,
            refresh_interval_seconds: w.refresh_interval_seconds.unwrap_or(30),
            position: w.position,
        })
        .collect();

    let response = DashboardResponse {
        id: dashboard_id,
        name: request.name,
        description: request.description.unwrap_or_default(),
        widgets,
        tags: request.tags.unwrap_or_default(),
        created_at: now,
        updated_at: now,
        created_by: "admin".to_string(), // Would come from auth context
    };

    Ok((StatusCode::CREATED, AxumJson(response)))
}

/// GET /api/v1/dashboards
///
/// List all dashboards
#[utoipa::path(
    get,
    path = "/api/v1/dashboards",
    tag = "dashboards",
    responses(
        (status = 200, description = "List of dashboards", body = DashboardListResponse),
    )
)]
pub async fn list_dashboards(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<DashboardListResponse>> {
    // In a real implementation, this would fetch dashboards from DashboardManager
    let response = DashboardListResponse {
        dashboards: vec![],
        total_count: 0,
    };

    Ok(AxumJson(response))
}

/// GET /api/v1/dashboards/{id}
///
/// Get a specific dashboard
#[utoipa::path(
    get,
    path = "/api/v1/dashboards/{id}",
    tag = "dashboards",
    responses(
        (status = 200, description = "Dashboard details", body = DashboardResponse),
        (status = 404, description = "Dashboard not found"),
    )
)]
pub async fn get_dashboard(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<AxumJson<DashboardResponse>> {
    // In a real implementation, this would fetch the dashboard from DashboardManager
    // For now, return a mock response
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let response = DashboardResponse {
        id,
        name: "System Overview".to_string(),
        description: "Main system dashboard".to_string(),
        widgets: vec![],
        tags: vec!["system".to_string()],
        created_at: now - 86400, // 1 day ago
        updated_at: now,
        created_by: "admin".to_string(),
    };

    Ok(AxumJson(response))
}

/// PUT /api/v1/dashboards/{id}
///
/// Update an existing dashboard
#[utoipa::path(
    put,
    path = "/api/v1/dashboards/{id}",
    tag = "dashboards",
    request_body = DashboardRequest,
    responses(
        (status = 200, description = "Dashboard updated", body = DashboardResponse),
        (status = 404, description = "Dashboard not found"),
    )
)]
pub async fn update_dashboard(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    axum::Json(request): axum::Json<DashboardRequest>,
) -> ApiResult<AxumJson<DashboardResponse>> {
    // In a real implementation, this would update the dashboard in DashboardManager
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let widgets: Vec<WidgetResponse> = request
        .widgets
        .into_iter()
        .map(|w| WidgetResponse {
            id: uuid::Uuid::new_v4().to_string(),
            title: w.title,
            widget_type: w.widget_type,
            queries: w.queries,
            refresh_interval_seconds: w.refresh_interval_seconds.unwrap_or(30),
            position: w.position,
        })
        .collect();

    let response = DashboardResponse {
        id,
        name: request.name,
        description: request.description.unwrap_or_default(),
        widgets,
        tags: request.tags.unwrap_or_default(),
        created_at: now - 86400, // Mock created time
        updated_at: now,
        created_by: "admin".to_string(),
    };

    Ok(AxumJson(response))
}

/// DELETE /api/v1/dashboards/{id}
///
/// Delete a dashboard
#[utoipa::path(
    delete,
    path = "/api/v1/dashboards/{id}",
    tag = "dashboards",
    responses(
        (status = 204, description = "Dashboard deleted"),
        (status = 404, description = "Dashboard not found"),
    )
)]
pub async fn delete_dashboard(
    State(_state): State<Arc<ApiState>>,
    Path(_id): Path<String>,
) -> ApiResult<StatusCode> {
    // In a real implementation, this would delete the dashboard from DashboardManager
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// WebSocket Streaming for Dashboard Real-time Metrics
// ============================================================================

use axum::extract::WebSocketUpgrade;
use axum::response::Response;

/// GET /api/v1/ws/dashboard
///
/// WebSocket endpoint for real-time dashboard metrics streaming
pub async fn ws_dashboard_stream(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_dashboard_websocket(socket, state))
}

async fn handle_dashboard_websocket(
    mut socket: axum::extract::ws::WebSocket,
    _state: Arc<ApiState>,
) {
    use axum::extract::ws::Message;
    use tokio::time::{interval, Duration};

    let mut interval = interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // Collect real-time metrics
                let cpu_usage = sys_info::loadavg().map(|l| l.one * 10.0).unwrap_or(0.0);
                let mem_info = sys_info::mem_info().unwrap_or(sys_info::MemInfo {
                    total: 0, free: 0, avail: 0, buffers: 0, cached: 0,
                    swap_total: 0, swap_free: 0,
                });
                let mem_usage = if mem_info.total > 0 {
                    (mem_info.total - mem_info.free) as f64 / mem_info.total as f64 * 100.0
                } else {
                    0.0
                };

                // Create real-time dashboard update
                let update = serde_json::json!({
                    "timestamp": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
                    "metrics": {
                        "cpu_usage_percent": cpu_usage,
                        "memory_usage_percent": mem_usage,
                        "active_connections": 0,
                        "queries_per_second": 0.0,
                    },
                    "alerts": []
                });

                let message = Message::Text(update.to_string().into());
                if socket.send(message).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
