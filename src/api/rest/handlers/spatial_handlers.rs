// Spatial Database API Handlers
//
// REST API endpoints for spatial database operations including:
// - Spatial queries (within, intersects, nearest)
// - Route calculation and network analysis
// - Geometry operations
// - Coordinate transformations

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::api::rest::types::{ApiState, ApiError, ApiResult};
use crate::spatial::{
    SpatialEngine, Geometry, Point, Coordinate, BoundingBox,
    WktParser, TopologicalOps, DistanceOps, BufferOps,
    Network, Node, Edge, DijkstraRouter,
    CoordinateTransformer, well_known_srid,
};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpatialQueryRequest {
    // Geometry in WKT format
    pub geometry: String,
    // Query type: within, intersects, contains, touches
    pub query_type: String,
    // Target layer/table
    pub layer: String,
    // Distance for buffer (optional)
    pub distance: Option<f64>,
    // SRID for coordinate system
    pub srid: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpatialQueryResponse {
    pub results: Vec<SpatialFeature>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpatialFeature {
    pub id: String,
    pub geometry_wkt: String,
    pub properties: serde_json::Value,
    pub distance: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RouteRequest {
    pub start: CoordinateInput,
    pub end: CoordinateInput,
    // Algorithm: dijkstra, astar
    pub algorithm: Option<String>,
    // Optimization: shortest, fastest
    pub optimization: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CoordinateInput {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RouteResponse {
    pub path: Vec<CoordinateInput>,
    pub distance: f64,
    pub duration: Option<f64>,
    pub geometry_wkt: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NearestRequest {
    pub point: CoordinateInput,
    pub layer: String,
    pub count: Option<usize>,
    pub max_distance: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NearestResponse {
    pub features: Vec<SpatialFeature>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BufferRequest {
    pub geometry_wkt: String,
    pub distance: f64,
    pub srid: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BufferResponse {
    pub buffered_geometry_wkt: String,
    pub area: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TransformRequest {
    pub geometry_wkt: String,
    pub from_srid: i32,
    pub to_srid: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TransformResponse {
    pub transformed_geometry_wkt: String,
    pub from_srid: i32,
    pub to_srid: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WithinRequest {
    pub point: CoordinateInput,
    pub polygon_wkt: String,
    pub srid: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WithinResponse {
    pub is_within: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IntersectsRequest {
    pub geometry1_wkt: String,
    pub geometry2_wkt: String,
    pub srid: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IntersectsResponse {
    pub intersects: bool,
}

// ============================================================================
// Handler Functions
// ============================================================================

// Global spatial engine instance
lazy_static::lazy_static! {
    static ref SPATIAL_ENGINE: SpatialEngine = SpatialEngine::new();
    static ref SPATIAL_NETWORK: parking_lot::RwLock<Network> = parking_lot::RwLock::new(Network::new());
}

/// Execute a spatial query
#[utoipa::path(
    post,
    path = "/api/v1/spatial/query",
    request_body = SpatialQueryRequest,
    responses(
        (status = 200, description = "Query executed successfully", body = SpatialQueryResponse),
        (status = 400, description = "Invalid query", body = ApiError),
    ),
    tag = "spatial"
)]
pub async fn spatial_query(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<SpatialQueryRequest>,
) -> ApiResult<Json<SpatialQueryResponse>> {
    // Parse query geometry
    let query_geom = SPATIAL_ENGINE.parse_wkt(&request.geometry)
        .map_err(|e| ApiError::new("INVALID_GEOMETRY", format!("Invalid WKT geometry: {}", e)))?;

    // In a real implementation, this would query a spatial index
    // For now, return mock results
    let results = Vec::new();

    Ok(Json(SpatialQueryResponse {
        count: results.len(),
        results,
    }))
}

/// Calculate a route between two points
#[utoipa::path(
    post,
    path = "/api/v1/spatial/route",
    request_body = RouteRequest,
    responses(
        (status = 200, description = "Route calculated", body = RouteResponse),
        (status = 404, description = "No route found", body = ApiError),
    ),
    tag = "spatial"
)]
pub async fn calculate_route(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<RouteRequest>,
) -> ApiResult<Json<RouteResponse>> {
    let network = SPATIAL_NETWORK.read();

    // Find nearest nodes to start and end coordinates
    // For simplicity, using mock node IDs
    let start_node = 1;
    let end_node = 2;

    // Calculate route using Dijkstra
    let router = DijkstraRouter::new(&*network);
    let path_result = router.shortest_path(start_node, end_node)
        .map_err(|e| ApiError::new("ROUTE_FAILED", format!("Route calculation failed: {}", e)))?;

    let path = match path_result {
        Some(p) => p,
        None => return Err(ApiError::new("NOT_FOUND", "No route found")),
    };

    // Convert path to coordinates
    let coordinates: Vec<CoordinateInput> = path.vertices.iter().map(|node_id| {
        // Get node coordinates (mock data)
        CoordinateInput { x: *node_id as f64, y: *node_id as f64 }
    }).collect();

    // Build WKT LineString
    let wkt_coords: Vec<String> = coordinates.iter()
        .map(|c| format!("{} {}", c.x, c.y))
        .collect();
    let geometry_wkt = format!("LINESTRING({})", wkt_coords.join(", "));

    Ok(Json(RouteResponse {
        path: coordinates,
        distance: path.total_cost,
        duration: None,
        geometry_wkt,
    }))
}

/// Find features within a geometry
#[utoipa::path(
    post,
    path = "/api/v1/spatial/within",
    request_body = WithinRequest,
    responses(
        (status = 200, description = "Within test completed", body = WithinResponse),
        (status = 400, description = "Invalid geometry", body = ApiError),
    ),
    tag = "spatial"
)]
pub async fn find_within(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<WithinRequest>,
) -> ApiResult<Json<WithinResponse>> {
    // Parse polygon
    let polygon_geom = SPATIAL_ENGINE.parse_wkt(&request.polygon_wkt)
        .map_err(|e| ApiError::new("INVALID_GEOMETRY", format!("Invalid polygon WKT: {}", e)))?;

    // Create point geometry
    let point_geom = Geometry::Point(Point::new(Coordinate::new(request.point.x, request.point.y)));

    // Check if point is within polygon
    let is_within = TopologicalOps::within(&point_geom, &polygon_geom)
        .map_err(|e| ApiError::new("OPERATION_FAILED", format!("Within test failed: {}", e)))?;

    Ok(Json(WithinResponse { is_within }))
}

/// Find nearest features to a point
#[utoipa::path(
    post,
    path = "/api/v1/spatial/nearest",
    request_body = NearestRequest,
    responses(
        (status = 200, description = "Nearest features found", body = NearestResponse),
    ),
    tag = "spatial"
)]
pub async fn find_nearest(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<NearestRequest>,
) -> ApiResult<Json<NearestResponse>> {
    // In a real implementation, this would use a spatial index (R-tree)
    // to find nearest features efficiently
    let features = Vec::new();

    Ok(Json(NearestResponse { features }))
}

/// Check if two geometries intersect
#[utoipa::path(
    post,
    path = "/api/v1/spatial/intersects",
    request_body = IntersectsRequest,
    responses(
        (status = 200, description = "Intersection test completed", body = IntersectsResponse),
        (status = 400, description = "Invalid geometry", body = ApiError),
    ),
    tag = "spatial"
)]
pub async fn check_intersects(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<IntersectsRequest>,
) -> ApiResult<Json<IntersectsResponse>> {
    let geom1 = SPATIAL_ENGINE.parse_wkt(&request.geometry1_wkt)
        .map_err(|e| ApiError::new("INVALID_GEOMETRY", format!("Invalid geometry 1: {}", e)))?;

    let geom2 = SPATIAL_ENGINE.parse_wkt(&request.geometry2_wkt)
        .map_err(|e| ApiError::new("INVALID_GEOMETRY", format!("Invalid geometry 2: {}", e)))?;

    let intersects = TopologicalOps::intersects(&geom1, &geom2)
        .map_err(|e| ApiError::new("OPERATION_FAILED", format!("Intersects test failed: {}", e)))?;

    Ok(Json(IntersectsResponse { intersects }))
}

/// Create a buffer around a geometry
#[utoipa::path(
    post,
    path = "/api/v1/spatial/buffer",
    request_body = BufferRequest,
    responses(
        (status = 200, description = "Buffer created", body = BufferResponse),
        (status = 400, description = "Invalid geometry", body = ApiError),
    ),
    tag = "spatial"
)]
pub async fn create_buffer(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<BufferRequest>,
) -> ApiResult<Json<BufferResponse>> {
    let geometry = SPATIAL_ENGINE.parse_wkt(&request.geometry_wkt)
        .map_err(|e| ApiError::new("INVALID_GEOMETRY", format!("Invalid geometry: {}", e)))?;

    let buffered = BufferOps::buffer(&geometry, request.distance)
        .map_err(|e| ApiError::new("BUFFER_FAILED", format!("Buffer operation failed: {}", e)))?;

    // Convert to WKT
    let buffered_wkt = buffered.to_wkt();

    Ok(Json(BufferResponse {
        buffered_geometry_wkt: buffered_wkt,
        area: 0.0, // Would calculate actual area
    }))
}

/// Transform geometry between coordinate systems
#[utoipa::path(
    post,
    path = "/api/v1/spatial/transform",
    request_body = TransformRequest,
    responses(
        (status = 200, description = "Geometry transformed", body = TransformResponse),
        (status = 400, description = "Invalid geometry or SRID", body = ApiError),
    ),
    tag = "spatial"
)]
pub async fn transform_geometry(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<TransformRequest>,
) -> ApiResult<Json<TransformResponse>> {
    let geometry = SPATIAL_ENGINE.parse_wkt(&request.geometry_wkt)
        .map_err(|e| ApiError::new("INVALID_GEOMETRY", format!("Invalid geometry: {}", e)))?;

    let transformer = SPATIAL_ENGINE.transformer();

    // Transform coordinates based on geometry type
    let transformed_geom = match geometry {
        Geometry::Point(point) => {
            let transformed_coord = transformer.transform(
                &point.coord,
                request.from_srid,
                request.to_srid
            ).map_err(|e| ApiError::new("TRANSFORM_FAILED", format!("Transform failed: {}", e)))?;

            Geometry::Point(Point::new(transformed_coord))
        },
        _ => {
            // For other geometry types, would need to transform all coordinates
            return Err(ApiError::new("UNSUPPORTED", "Only Point transformation is currently supported"));
        }
    };

    Ok(Json(TransformResponse {
        transformed_geometry_wkt: transformed_geom.to_wkt(),
        from_srid: request.from_srid,
        to_srid: request.to_srid,
    }))
}

/// Calculate distance between two geometries
#[utoipa::path(
    get,
    path = "/api/v1/spatial/distance",
    params(
        ("geom1" = String, Query(description = "First geometry (WKT)")),
        ("geom2" = String, Query(description = "Second geometry (WKT)"))
    ),
    responses(
        (status = 200, description = "Distance calculated", body = f64),
        (status = 400, description = "Invalid geometry", body = ApiError),
    ),
    tag = "spatial"
)]
pub async fn calculate_distance(
    State(_state): State<Arc<ApiState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<f64>> {
    let geom1_wkt = params.get("geom1")
        .ok_or_else(|| ApiError::new("MISSING_PARAMETER", "Missing geom1 parameter"))?;
    let geom2_wkt = params.get("geom2")
        .ok_or_else(|| ApiError::new("MISSING_PARAMETER", "Missing geom2 parameter"))?;

    let geom1 = SPATIAL_ENGINE.parse_wkt(geom1_wkt)
        .map_err(|e| ApiError::new("INVALID_GEOMETRY", format!("Invalid geometry 1: {}", e)))?;

    let geom2 = SPATIAL_ENGINE.parse_wkt(geom2_wkt)
        .map_err(|e| ApiError::new("INVALID_GEOMETRY", format!("Invalid geometry 2: {}", e)))?;

    let distance = DistanceOps::distance(&geom1, &geom2)
        .map_err(|e| ApiError::new("DISTANCE_FAILED", format!("Distance calculation failed: {}", e)))?;

    Ok(Json(distance))
}

/// Add a node to the routing network
#[utoipa::path(
    post,
    path = "/api/v1/spatial/network/nodes",
    responses(
        (status = 201, description = "Node added"),
    ),
    tag = "spatial"
)]
pub async fn add_network_node(
    State(_state): State<Arc<ApiState>>,
    Json(coord): Json<CoordinateInput>,
) -> ApiResult<StatusCode> {
    let mut network = SPATIAL_NETWORK.write();

    let node_id = network.node_count() as u64 + 1;
    let node = Node::new(node_id, Coordinate::new(coord.x, coord.y));

    network.add_node(node);

    Ok(StatusCode::CREATED)
}

/// Add an edge to the routing network
#[utoipa::path(
    post,
    path = "/api/v1/spatial/network/edges",
    responses(
        (status = 201, description = "Edge added"),
    ),
    tag = "spatial"
)]
pub async fn add_network_edge(
    State(_state): State<Arc<ApiState>>,
    Json(edge_data): Json<serde_json::Value>,
) -> ApiResult<StatusCode> {
    let mut network = SPATIAL_NETWORK.write();

    let source = edge_data["source"].as_u64().unwrap_or(0);
    let target = edge_data["target"].as_u64().unwrap_or(0);
    let cost = edge_data["cost"].as_f64().unwrap_or(1.0);

    let edge_id = network.edge_count() as u64 + 1;
    let edge = Edge::new(edge_id, source, target, cost);

    network.add_edge(edge)
        .map_err(|e| ApiError::new("EDGE_ADD_FAILED", format!("Failed to add edge: {}", e)))?;

    Ok(StatusCode::CREATED)
}
