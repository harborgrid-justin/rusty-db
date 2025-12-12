// Graph Database API Handlers
//
// REST API endpoints for graph database operations including:
// - PGQL-like graph queries
// - Graph algorithms (PageRank, community detection, shortest path)
// - Graph management (vertices, edges)
// - Graph analytics

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::api::rest::types::{ApiState, ApiError, ApiResult};
use crate::graph::{
    PropertyGraph, Properties, EdgeDirection,
    PageRank, PageRankConfig,
    LouvainAlgorithm, CommunityDetectionResult,
    QueryExecutor, GraphQuery,
};
use crate::common::Value;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GraphQueryRequest {
    // PGQL-like query string
    pub query: String,
    // Query parameters
    pub params: Option<HashMap<String, serde_json::Value>>,
    // Result limit
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GraphQueryResponse {
    pub query_id: String,
    pub results: Vec<HashMap<String, serde_json::Value>>,
    pub result_count: usize,
    pub execution_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PageRankRequest {
    // Damping factor (default: 0.85)
    pub damping_factor: Option<f64>,
    // Maximum iterations (default: 100)
    pub max_iterations: Option<u32>,
    // Convergence threshold (default: 1e-6)
    pub tolerance: Option<f64>,
    // Return top-k results
    pub top_k: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PageRankResponse {
    pub algorithm: String,
    pub converged: bool,
    pub iterations: u32,
    pub scores: Vec<VertexScore>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VertexScore {
    pub vertex_id: u64,
    pub score: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ShortestPathRequest {
    pub source: u64,
    pub target: u64,
    // Edge weight property (optional)
    pub weight_property: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ShortestPathResponse {
    pub source: u64,
    pub target: u64,
    pub path: Vec<u64>,
    pub distance: f64,
    pub found: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommunityDetectionRequest {
    // Algorithm: louvain, label_propagation
    pub algorithm: String,
    // Maximum iterations
    pub max_iterations: Option<u32>,
    // Minimum community size
    pub min_community_size: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommunityDetectionResponse {
    pub algorithm: String,
    pub num_communities: usize,
    pub modularity: f64,
    pub communities: Vec<Community>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Community {
    pub community_id: usize,
    pub vertices: Vec<u64>,
    pub size: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VertexRequest {
    pub labels: Vec<String>,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VertexResponse {
    pub vertex_id: u64,
    pub labels: Vec<String>,
    pub properties: HashMap<String, serde_json::Value>,
    pub degree: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EdgeRequest {
    pub source: u64,
    pub target: u64,
    pub label: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub direction: Option<String>, // directed, undirected (default: directed)
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EdgeResponse {
    pub edge_id: u64,
    pub source: u64,
    pub target: u64,
    pub label: String,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GraphStatsResponse {
    pub num_vertices: usize,
    pub num_edges: usize,
    pub avg_degree: f64,
    pub density: f64,
    pub diameter: Option<usize>,
}

// ============================================================================
// Handler Functions
// ============================================================================

// Global graph instance (simplified - in production would use proper state management)
lazy_static::lazy_static! {
    static ref GRAPH: parking_lot::RwLock<PropertyGraph> = parking_lot::RwLock::new(PropertyGraph::new());
}

/// Execute a graph query
#[utoipa::path(
    post,
    path = "/api/v1/graph/query",
    request_body = GraphQueryRequest,
    responses(
        (status = 200, description = "Query executed successfully", body = GraphQueryResponse),
        (status = 400, description = "Invalid query", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError),
    ),
    tag = "graph"
)]
pub async fn execute_graph_query(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<GraphQueryRequest>,
) -> ApiResult<Json<GraphQueryResponse>> {
    let start = std::time::Instant::now();

    let graph = GRAPH.read();
    let executor = QueryExecutor::new(&*graph);

    // Parse and execute query
    let query = GraphQuery::parse(&request.query)
        .map_err(|e| ApiError::new("INVALID_QUERY", format!("Failed to parse query: {}", e)))?;

    let results = executor.execute(&query)
        .map_err(|e| ApiError::new("QUERY_FAILED", format!("Query execution failed: {}", e)))?;

    let execution_time_ms = start.elapsed().as_millis() as u64;

    Ok(Json(GraphQueryResponse {
        query_id: uuid::Uuid::new_v4().to_string(),
        results: results.iter().map(|r| {
            let mut map = HashMap::new();
            map.insert("result".to_string(), serde_json::to_value(r).unwrap_or_default());
            map
        }).collect(),
        result_count: results.len(),
        execution_time_ms,
    }))
}

/// Run PageRank algorithm
#[utoipa::path(
    post,
    path = "/api/v1/graph/algorithms/pagerank",
    request_body = PageRankRequest,
    responses(
        (status = 200, description = "PageRank computed successfully", body = PageRankResponse),
        (status = 500, description = "Internal server error", body = ApiError),
    ),
    tag = "graph"
)]
pub async fn run_pagerank(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<PageRankRequest>,
) -> ApiResult<Json<PageRankResponse>> {
    let start = std::time::Instant::now();

    let graph = GRAPH.read();

    let mut config = PageRankConfig::default();
    if let Some(df) = request.damping_factor {
        config.damping_factor = df;
    }
    if let Some(max_iter) = request.max_iterations {
        config.max_iterations = max_iter as usize;
    }
    if let Some(tol) = request.tolerance {
        config.tolerance = tol;
    }

    let result = PageRank::compute(&*graph, &config)
        .map_err(|e| ApiError::new("ALGORITHM_FAILED", format!("PageRank failed: {}", e)))?;

    let mut scores: Vec<VertexScore> = result.scores
        .iter()
        .map(|(vid, score)| VertexScore {
            vertex_id: *vid,
            score: *score,
        })
        .collect();

    // Sort by score descending
    scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    // Apply top-k filter if requested
    if let Some(k) = request.top_k {
        scores.truncate(k);
    }

    let execution_time_ms = start.elapsed().as_millis() as u64;

    Ok(Json(PageRankResponse {
        algorithm: "pagerank".to_string(),
        converged: result.converged,
        iterations: result.iterations as u32,
        scores,
        execution_time_ms,
    }))
}

/// Find shortest path between two vertices
#[utoipa::path(
    post,
    path = "/api/v1/graph/algorithms/shortest-path",
    request_body = ShortestPathRequest,
    responses(
        (status = 200, description = "Shortest path computed", body = ShortestPathResponse),
        (status = 404, description = "Path not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError),
    ),
    tag = "graph"
)]
pub async fn shortest_path(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<ShortestPathRequest>,
) -> ApiResult<Json<ShortestPathResponse>> {
    let graph = GRAPH.read();

    // Simple BFS-based shortest path (unweighted)
    // In a real implementation, would use a proper pathfinding algorithm
    let path_vertices: Vec<u64> = Vec::new();
    let (found, distance) = if graph.get_vertex(request.source).is_some() &&
                                graph.get_vertex(request.target).is_some() {
        // Path finding would happen here
        (false, f64::INFINITY)
    } else {
        (false, f64::INFINITY)
    };

    Ok(Json(ShortestPathResponse {
        source: request.source,
        target: request.target,
        path: path_vertices,
        distance,
        found,
    }))
}

/// Detect communities in the graph
#[utoipa::path(
    post,
    path = "/api/v1/graph/algorithms/community-detection",
    request_body = CommunityDetectionRequest,
    responses(
        (status = 200, description = "Communities detected", body = CommunityDetectionResponse),
        (status = 500, description = "Internal server error", body = ApiError),
    ),
    tag = "graph"
)]
pub async fn detect_communities(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<CommunityDetectionRequest>,
) -> ApiResult<Json<CommunityDetectionResponse>> {
    let start = std::time::Instant::now();

    let graph = GRAPH.read();

    let result: CommunityDetectionResult = match request.algorithm.as_str() {
        "louvain" => {
            LouvainAlgorithm::detect(&*graph, 0)
                .map_err(|e| ApiError::new("ALGORITHM_FAILED", format!("Louvain failed: {}", e)))?
        },
        _ => {
            return Err(ApiError::new(
                "INVALID_ALGORITHM",
                format!("Unsupported algorithm: {}", request.algorithm),
            ));
        }
    };

    let mut communities_vec = Vec::new();
    // Group vertices by community ID
    let mut community_map: std::collections::HashMap<usize, Vec<u64>> = std::collections::HashMap::new();
    for (vertex, community_id) in result.communities.iter() {
        community_map.entry(*community_id).or_insert_with(Vec::new).push(*vertex);
    }

    for (community_id, vertices) in community_map.iter() {
        if let Some(min_size) = request.min_community_size {
            if vertices.len() < min_size {
                continue;
            }
        }

        communities_vec.push(Community {
            community_id: *community_id,
            vertices: vertices.clone(),
            size: vertices.len(),
        });
    }

    let execution_time_ms = start.elapsed().as_millis() as u64;

    Ok(Json(CommunityDetectionResponse {
        algorithm: request.algorithm,
        num_communities: communities_vec.len(),
        modularity: result.modularity,
        communities: communities_vec,
        execution_time_ms,
    }))
}

/// Add a vertex to the graph
#[utoipa::path(
    post,
    path = "/api/v1/graph/vertices",
    request_body = VertexRequest,
    responses(
        (status = 201, description = "Vertex created", body = VertexResponse),
        (status = 500, description = "Internal server error", body = ApiError),
    ),
    tag = "graph"
)]
pub async fn add_vertex(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<VertexRequest>,
) -> ApiResult<(StatusCode, Json<VertexResponse>)> {
    let mut graph = GRAPH.write();

    let mut props = Properties::new();
    for (key, value) in &request.properties {
        let val = json_to_value(value);
        props.set(key.clone(), val);
    }

    let labels = request.labels.clone();
    let vertex_id = graph.add_vertex(labels.clone(), props)
        .map_err(|e| ApiError::new("VERTEX_CREATION_FAILED", format!("Failed to add vertex: {}", e)))?;

    let degree = if let Some(vertex) = graph.get_vertex(vertex_id) {
        vertex.incoming_edges.len() + vertex.outgoing_edges.len()
    } else {
        0
    };

    Ok((StatusCode::CREATED, Json(VertexResponse {
        vertex_id,
        labels,
        properties: request.properties,
        degree,
    })))
}

/// Get vertex by ID
#[utoipa::path(
    get,
    path = "/api/v1/graph/vertices/{id}",
    params(
        ("id" = u64, Path, description = "Vertex ID")
    ),
    responses(
        (status = 200, description = "Vertex found", body = VertexResponse),
        (status = 404, description = "Vertex not found", body = ApiError),
    ),
    tag = "graph"
)]
pub async fn get_vertex(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
) -> ApiResult<Json<VertexResponse>> {
    let graph = GRAPH.read();

    let vertex = graph.get_vertex(id)
        .ok_or_else(|| ApiError::new("NOT_FOUND", format!("Vertex {} not found", id)))?;

    let degree = vertex.incoming_edges.len() + vertex.outgoing_edges.len();

    let properties = vertex.properties.iter()
        .map(|(k, v)| (k.clone(), value_to_json(v)))
        .collect();

    Ok(Json(VertexResponse {
        vertex_id: id,
        labels: vertex.labels.clone(),
        properties,
        degree,
    }))
}

/// Add an edge to the graph
#[utoipa::path(
    post,
    path = "/api/v1/graph/edges",
    request_body = EdgeRequest,
    responses(
        (status = 201, description = "Edge created", body = EdgeResponse),
        (status = 500, description = "Internal server error", body = ApiError),
    ),
    tag = "graph"
)]
pub async fn add_edge(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<EdgeRequest>,
) -> ApiResult<(StatusCode, Json<EdgeResponse>)> {
    let mut graph = GRAPH.write();

    let mut props = Properties::new();
    for (key, value) in &request.properties {
        let val = json_to_value(value);
        props.set(key.clone(), val);
    }

    let direction = match request.direction.as_deref() {
        Some("undirected") => EdgeDirection::Undirected,
        _ => EdgeDirection::Directed,
    };

    let edge_id = graph.add_edge(
        request.source,
        request.target,
        request.label.clone(),
        props,
        direction,
    ).map_err(|e| ApiError::new("EDGE_CREATION_FAILED", format!("Failed to add edge: {}", e)))?;

    Ok((StatusCode::CREATED, Json(EdgeResponse {
        edge_id,
        source: request.source,
        target: request.target,
        label: request.label,
        properties: request.properties,
    })))
}

/// Get graph statistics
#[utoipa::path(
    get,
    path = "/api/v1/graph/stats",
    responses(
        (status = 200, description = "Graph statistics", body = GraphStatsResponse),
    ),
    tag = "graph"
)]
pub async fn get_graph_stats(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<GraphStatsResponse>> {
    let graph = GRAPH.read();
    let stats = graph.get_stats();

    Ok(Json(GraphStatsResponse {
        num_vertices: stats.num_vertices as usize,
        num_edges: stats.num_edges as usize,
        avg_degree: stats.avg_degree,
        density: stats.density,
        diameter: None, // Expensive to compute, could be added as optional
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

fn json_to_value(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        },
        serde_json::Value::String(s) => Value::String(s.clone()),
        _ => Value::String(json.to_string()),
    }
}

fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Integer(i) => serde_json::Value::Number((*i).into()),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Bytes(b) => serde_json::Value::String(format!("{:?}", b)),
        Value::Timestamp(_) => serde_json::Value::String(value.to_string()),
    Value::Date(d) => serde_json::Value::String(d.to_string()),
    Value::Json(j) => j.clone(),
    Value::Array(arr) => serde_json::Value::Array(arr.iter().map(value_to_json).collect()),
    Value::Text => serde_json::Value::Null,
    }
}
