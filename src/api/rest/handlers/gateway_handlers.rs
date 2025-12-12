// Gateway Management Handlers
//
// Handler functions for API Gateway management operations including:
// - Route management (CRUD)
// - Rate limiting configuration
// - Service management
// - Gateway metrics
// - Security configuration (IP filters)

use axum::{
    extract::{Path, Query, State},
    response::Json as AxumJson,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use uuid::Uuid;
use parking_lot::RwLock;
use utoipa::ToSchema;

use super::super::types::*;
use crate::api::gateway::{
    ApiGateway, GatewayConfig, Route, BackendService, ServiceEndpoint,
    RateLimitConfig, RateLimitType, HttpMethod, Protocol, LoadBalancingStrategy,
    CircuitBreakerConfig, RetryPolicy,
};

// Lazy-initialized shared API Gateway instance
lazy_static::lazy_static! {
    static ref API_GATEWAY: Arc<RwLock<ApiGateway>> = {
        let config = GatewayConfig::default();
        Arc::new(RwLock::new(ApiGateway::new(config)))
    };

    // In-memory storage for rate limit configurations
    static ref RATE_LIMIT_CONFIGS: Arc<RwLock<HashMap<String, RateLimitConfigData>>> =
        Arc::new(RwLock::new(HashMap::new()));

    // In-memory storage for IP filter rules
    static ref IP_FILTERS: Arc<RwLock<HashMap<String, IpFilterRule>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

// Route creation/update request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRouteRequest {
    pub name: String,
    pub path_pattern: String,
    pub methods: Vec<String>,
    pub backend_service: String,
    pub auth_required: bool,
    pub required_permissions: Vec<String>,
    #[serde(default)]
    pub enable_cache: bool,
    pub cache_ttl: Option<u64>,
}

// Route response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RouteResponse {
    pub id: String,
    pub name: String,
    pub path_pattern: String,
    pub methods: Vec<String>,
    pub backend_service: String,
    pub auth_required: bool,
    pub required_permissions: Vec<String>,
    pub enable_cache: bool,
    pub cache_ttl: Option<u64>,
    pub created_at: u64,
}

// Rate limit configuration request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRateLimitRequest {
    pub name: String,
    pub limit_type: String, // "token_bucket", "sliding_window", "fixed_window"
    pub requests: u64,
    pub window_seconds: u64,
    pub burst: Option<u64>,
    pub description: Option<String>,
}

// Rate limit configuration data (internal)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfigData {
    pub id: String,
    pub name: String,
    pub config: RateLimitConfig,
    pub description: Option<String>,
    pub created_at: u64,
}

// Rate limit response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RateLimitResponse {
    pub id: String,
    pub name: String,
    pub limit_type: String,
    pub requests: u64,
    pub window_seconds: u64,
    pub burst: Option<u64>,
    pub description: Option<String>,
    pub created_at: u64,
}

// Service registration request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterServiceRequest {
    pub name: String,
    pub endpoints: Vec<ServiceEndpointRequest>,
    pub load_balancing: String, // "round_robin", "least_connections", "random", etc.
    pub health_check_enabled: bool,
}

// Service endpoint request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServiceEndpointRequest {
    pub host: String,
    pub port: u16,
    pub protocol: String, // "http", "grpc", "websocket"
    pub weight: u32,
}

// Service response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServiceResponse {
    pub id: String,
    pub name: String,
    pub endpoints: Vec<EndpointResponse>,
    pub load_balancing: String,
    pub health_check_enabled: bool,
    pub created_at: u64,
}

// Endpoint response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EndpointResponse {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub protocol: String,
    pub weight: u32,
    pub healthy: bool,
}

// Service health response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServiceHealthResponse {
    pub service_id: String,
    pub service_name: String,
    pub overall_health: String, // "healthy", "degraded", "unhealthy"
    pub endpoints: Vec<EndpointHealthStatus>,
    pub last_check: u64,
}

// Endpoint health status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EndpointHealthStatus {
    pub endpoint_id: String,
    pub host: String,
    pub port: u16,
    pub healthy: bool,
    pub last_check_timestamp: u64,
}

// Gateway metrics response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GatewayMetricsResponse {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
    pub requests_by_protocol: HashMap<String, u64>,
    pub requests_by_route: HashMap<String, u64>,
    pub auth_failures: u64,
    pub authz_failures: u64,
    pub rate_limit_hits: u64,
    pub security_blocks: u64,
    pub uptime_seconds: u64,
}

// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLogEntry {
    pub id: String,
    pub timestamp: u64,
    pub event_type: String,
    pub user_id: Option<String>,
    pub client_ip: String,
    pub request_id: String,
    pub resource: String,
    pub action: String,
    pub result: String,
    pub details: HashMap<String, String>,
}

// Audit log response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLogResponse {
    pub entries: Vec<AuditLogEntry>,
    pub total_count: usize,
    pub page: usize,
    pub page_size: usize,
}

// IP filter rule request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateIpFilterRequest {
    pub ip_address: String,
    pub filter_type: String, // "whitelist" or "blacklist"
    pub description: Option<String>,
}

// IP filter rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpFilterRule {
    pub id: String,
    pub ip_address: IpAddr,
    pub filter_type: String,
    pub description: Option<String>,
    pub created_at: u64,
}

// IP filter response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IpFilterResponse {
    pub id: String,
    pub ip_address: String,
    pub filter_type: String,
    pub description: Option<String>,
    pub created_at: u64,
}

// List filters query parameters
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

fn default_page() -> usize { 1 }
fn default_page_size() -> usize { 50 }

// ============================================================================
// ROUTE MANAGEMENT ENDPOINTS
// ============================================================================

/// Create a new route
#[utoipa::path(
    post,
    path = "/api/v1/gateway/routes",
    tag = "gateway",
    request_body = CreateRouteRequest,
    responses(
        (status = 201, description = "Route created", body = RouteResponse),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn create_route(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<CreateRouteRequest>,
) -> ApiResult<(StatusCode, AxumJson<RouteResponse>)> {
    let route_id = Uuid::new_v4().to_string();

    // Parse HTTP methods
    let methods: Vec<HttpMethod> = request.methods.iter()
        .filter_map(|m| match m.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::Get),
            "POST" => Some(HttpMethod::Post),
            "PUT" => Some(HttpMethod::Put),
            "DELETE" => Some(HttpMethod::Delete),
            "PATCH" => Some(HttpMethod::Patch),
            "HEAD" => Some(HttpMethod::Head),
            "OPTIONS" => Some(HttpMethod::Options),
            _ => None,
        })
        .collect();

    // Create a basic backend service (would be linked to actual service in production)
    let backend = BackendService {
        name: request.backend_service.clone(),
        endpoints: vec![],
        load_balancing: LoadBalancingStrategy::RoundRobin,
        circuit_breaker: CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: 60,
            half_open_max_requests: 3,
        },
        retry_policy: RetryPolicy {
            max_attempts: 3,
            initial_backoff: 100,
            max_backoff: 5000,
            backoff_multiplier: 2.0,
            retryable_status_codes: vec![502, 503, 504],
        },
    };

    let route = Route {
        id: route_id.clone(),
        name: request.name.clone(),
        path_pattern: request.path_pattern.clone(),
        methods,
        backend,
        middleware: vec![],
        rate_limit: None,
        auth_required: request.auth_required,
        required_permissions: request.required_permissions.clone(),
        request_transform: None,
        response_transform: None,
        enable_cache: request.enable_cache,
        cache_ttl: request.cache_ttl,
    };

    // Register route with gateway
    let gateway = API_GATEWAY.read();
    gateway.register_route(route);

    let response = RouteResponse {
        id: route_id,
        name: request.name,
        path_pattern: request.path_pattern,
        methods: request.methods,
        backend_service: request.backend_service,
        auth_required: request.auth_required,
        required_permissions: request.required_permissions,
        enable_cache: request.enable_cache,
        cache_ttl: request.cache_ttl,
        created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
    };

    Ok((StatusCode::CREATED, AxumJson(response)))
}

/// List all routes
#[utoipa::path(
    get,
    path = "/api/v1/gateway/routes",
    tag = "gateway",
    params(
        ("page" = Option<usize>, Query, description = "Page number"),
        ("page_size" = Option<usize>, Query, description = "Page size"),
    ),
    responses(
        (status = 200, description = "List of routes", body = Vec<RouteResponse>),
    )
)]
pub async fn list_routes(
    State(_state): State<Arc<ApiState>>,
    Query(params): Query<ListQuery>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    let gateway = API_GATEWAY.read();
    let routes = gateway.get_routes();

    let route_responses: Vec<RouteResponse> = routes.iter()
        .map(|r| RouteResponse {
            id: r.id.clone(),
            name: r.name.clone(),
            path_pattern: r.path_pattern.clone(),
            methods: r.methods.iter().map(|m| format!("{:?}", m).to_uppercase()).collect(),
            backend_service: r.backend.name.clone(),
            auth_required: r.auth_required,
            required_permissions: r.required_permissions.clone(),
            enable_cache: r.enable_cache,
            cache_ttl: r.cache_ttl,
            created_at: 0,
        })
        .collect();

    // Apply pagination
    let total = route_responses.len();
    let start = (params.page.saturating_sub(1)) * params.page_size;
    let paginated_routes: Vec<RouteResponse> = route_responses.into_iter()
        .skip(start)
        .take(params.page_size)
        .collect();

    Ok(AxumJson(json!({
        "routes": paginated_routes,
        "total": total,
        "page": params.page,
        "page_size": params.page_size,
    })))
}

/// Get a specific route
#[utoipa::path(
    get,
    path = "/api/v1/gateway/routes/{id}",
    tag = "gateway",
    params(
        ("id" = String, Path, description = "Route ID"),
    ),
    responses(
        (status = 200, description = "Route details", body = RouteResponse),
        (status = 404, description = "Route not found", body = ApiError),
    )
)]
pub async fn get_route(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<AxumJson<RouteResponse>> {
    let gateway = API_GATEWAY.read();
    let routes = gateway.get_routes();

    let route = routes.iter()
        .find(|r| r.id == id)
        .ok_or_else(|| ApiError::new("NOT_FOUND", "Route not found"))?;

    let response = RouteResponse {
        id: route.id.clone(),
        name: route.name.clone(),
        path_pattern: route.path_pattern.clone(),
        methods: route.methods.iter().map(|m| format!("{:?}", m).to_uppercase()).collect(),
        backend_service: route.backend.name.clone(),
        auth_required: route.auth_required,
        required_permissions: route.required_permissions.clone(),
        enable_cache: route.enable_cache,
        cache_ttl: route.cache_ttl,
        created_at: 0,
    };

    Ok(AxumJson(response))
}

/// Update a route
#[utoipa::path(
    put,
    path = "/api/v1/gateway/routes/{id}",
    tag = "gateway",
    params(
        ("id" = String, Path, description = "Route ID"),
    ),
    request_body = CreateRouteRequest,
    responses(
        (status = 200, description = "Route updated", body = RouteResponse),
        (status = 404, description = "Route not found", body = ApiError),
    )
)]
pub async fn update_route(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    AxumJson(request): AxumJson<CreateRouteRequest>,
) -> ApiResult<AxumJson<RouteResponse>> {
    // Remove old route
    let gateway = API_GATEWAY.read();
    let removed = gateway.remove_route(&id);

    if !removed {
        return Err(ApiError::new("NOT_FOUND", "Route not found"));
    }

    // Parse HTTP methods
    let methods: Vec<HttpMethod> = request.methods.iter()
        .filter_map(|m| match m.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::Get),
            "POST" => Some(HttpMethod::Post),
            "PUT" => Some(HttpMethod::Put),
            "DELETE" => Some(HttpMethod::Delete),
            "PATCH" => Some(HttpMethod::Patch),
            "HEAD" => Some(HttpMethod::Head),
            "OPTIONS" => Some(HttpMethod::Options),
            _ => None,
        })
        .collect();

    // Create backend service
    let backend = BackendService {
        name: request.backend_service.clone(),
        endpoints: vec![],
        load_balancing: LoadBalancingStrategy::RoundRobin,
        circuit_breaker: CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: 60,
            half_open_max_requests: 3,
        },
        retry_policy: RetryPolicy {
            max_attempts: 3,
            initial_backoff: 100,
            max_backoff: 5000,
            backoff_multiplier: 2.0,
            retryable_status_codes: vec![502, 503, 504],
        },
    };

    // Create updated route with same ID
    let route = Route {
        id: id.clone(),
        name: request.name.clone(),
        path_pattern: request.path_pattern.clone(),
        methods,
        backend,
        middleware: vec![],
        rate_limit: None,
        auth_required: request.auth_required,
        required_permissions: request.required_permissions.clone(),
        request_transform: None,
        response_transform: None,
        enable_cache: request.enable_cache,
        cache_ttl: request.cache_ttl,
    };

    gateway.register_route(route);

    let response = RouteResponse {
        id,
        name: request.name,
        path_pattern: request.path_pattern,
        methods: request.methods,
        backend_service: request.backend_service,
        auth_required: request.auth_required,
        required_permissions: request.required_permissions,
        enable_cache: request.enable_cache,
        cache_ttl: request.cache_ttl,
        created_at: 0,
    };

    Ok(AxumJson(response))
}

/// Delete a route
#[utoipa::path(
    delete,
    path = "/api/v1/gateway/routes/{id}",
    tag = "gateway",
    params(
        ("id" = String, Path, description = "Route ID"),
    ),
    responses(
        (status = 204, description = "Route deleted"),
        (status = 404, description = "Route not found", body = ApiError),
    )
)]
pub async fn delete_route(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let gateway = API_GATEWAY.read();
    let removed = gateway.remove_route(&id);

    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new("NOT_FOUND", "Route not found"))
    }
}

// ============================================================================
// RATE LIMITING CONFIGURATION ENDPOINTS
// ============================================================================

/// Get rate limit configurations
#[utoipa::path(
    get,
    path = "/api/v1/gateway/rate-limits",
    tag = "gateway",
    responses(
        (status = 200, description = "List of rate limit configurations", body = Vec<RateLimitResponse>),
    )
)]
pub async fn get_rate_limits(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<RateLimitResponse>>> {
    let configs = RATE_LIMIT_CONFIGS.read();

    let responses: Vec<RateLimitResponse> = configs.values()
        .map(|c| {
            let limit_type = match c.config.limit_type {
                RateLimitType::TokenBucket => "token_bucket",
                RateLimitType::SlidingWindow => "sliding_window",
                RateLimitType::FixedWindow => "fixed_window",
            };

            RateLimitResponse {
                id: c.id.clone(),
                name: c.name.clone(),
                limit_type: limit_type.to_string(),
                requests: c.config.requests,
                window_seconds: c.config.window,
                burst: c.config.burst,
                description: c.description.clone(),
                created_at: c.created_at,
            }
        })
        .collect();

    Ok(AxumJson(responses))
}

/// Create a rate limit configuration
#[utoipa::path(
    post,
    path = "/api/v1/gateway/rate-limits",
    tag = "gateway",
    request_body = CreateRateLimitRequest,
    responses(
        (status = 201, description = "Rate limit created", body = RateLimitResponse),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn create_rate_limit(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<CreateRateLimitRequest>,
) -> ApiResult<(StatusCode, AxumJson<RateLimitResponse>)> {
    let id = Uuid::new_v4().to_string();

    let limit_type = match request.limit_type.to_lowercase().as_str() {
        "token_bucket" => RateLimitType::TokenBucket,
        "sliding_window" => RateLimitType::SlidingWindow,
        "fixed_window" => RateLimitType::FixedWindow,
        _ => return Err(ApiError::new("INVALID_LIMIT_TYPE", "Invalid rate limit type")),
    };

    let config = RateLimitConfig {
        limit_type,
        requests: request.requests,
        window: request.window_seconds,
        burst: request.burst,
    };

    let config_data = RateLimitConfigData {
        id: id.clone(),
        name: request.name.clone(),
        config: config.clone(),
        description: request.description.clone(),
        created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
    };

    let mut configs = RATE_LIMIT_CONFIGS.write();
    configs.insert(id.clone(), config_data.clone());

    let response = RateLimitResponse {
        id,
        name: request.name,
        limit_type: request.limit_type,
        requests: request.requests,
        window_seconds: request.window_seconds,
        burst: request.burst,
        description: request.description,
        created_at: config_data.created_at,
    };

    Ok((StatusCode::CREATED, AxumJson(response)))
}

/// Update a rate limit configuration
#[utoipa::path(
    put,
    path = "/api/v1/gateway/rate-limits/{id}",
    tag = "gateway",
    params(
        ("id" = String, Path, description = "Rate limit ID"),
    ),
    request_body = CreateRateLimitRequest,
    responses(
        (status = 200, description = "Rate limit updated", body = RateLimitResponse),
        (status = 404, description = "Rate limit not found", body = ApiError),
    )
)]
pub async fn update_rate_limit(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    AxumJson(request): AxumJson<CreateRateLimitRequest>,
) -> ApiResult<AxumJson<RateLimitResponse>> {
    let mut configs = RATE_LIMIT_CONFIGS.write();

    if !configs.contains_key(&id) {
        return Err(ApiError::new("NOT_FOUND", "Rate limit configuration not found"));
    }

    let limit_type = match request.limit_type.to_lowercase().as_str() {
        "token_bucket" => RateLimitType::TokenBucket,
        "sliding_window" => RateLimitType::SlidingWindow,
        "fixed_window" => RateLimitType::FixedWindow,
        _ => return Err(ApiError::new("INVALID_LIMIT_TYPE", "Invalid rate limit type")),
    };

    let config = RateLimitConfig {
        limit_type,
        requests: request.requests,
        window: request.window_seconds,
        burst: request.burst,
    };

    let config_data = RateLimitConfigData {
        id: id.clone(),
        name: request.name.clone(),
        config,
        description: request.description.clone(),
        created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
    };

    configs.insert(id.clone(), config_data.clone());

    let response = RateLimitResponse {
        id,
        name: request.name,
        limit_type: request.limit_type,
        requests: request.requests,
        window_seconds: request.window_seconds,
        burst: request.burst,
        description: request.description,
        created_at: config_data.created_at,
    };

    Ok(AxumJson(response))
}

/// Delete a rate limit configuration
#[utoipa::path(
    delete,
    path = "/api/v1/gateway/rate-limits/{id}",
    tag = "gateway",
    params(
        ("id" = String, Path, description = "Rate limit ID"),
    ),
    responses(
        (status = 204, description = "Rate limit deleted"),
        (status = 404, description = "Rate limit not found", body = ApiError),
    )
)]
pub async fn delete_rate_limit(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let mut configs = RATE_LIMIT_CONFIGS.write();

    if configs.remove(&id).is_some() {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new("NOT_FOUND", "Rate limit configuration not found"))
    }
}

// ============================================================================
// SERVICE MANAGEMENT ENDPOINTS
// ============================================================================

/// List backend services
#[utoipa::path(
    get,
    path = "/api/v1/gateway/services",
    tag = "gateway",
    responses(
        (status = 200, description = "List of services", body = Vec<ServiceResponse>),
    )
)]
pub async fn list_services(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<ServiceResponse>>> {
    let gateway = API_GATEWAY.read();
    let registry = gateway.service_registry.read();

    let services: Vec<ServiceResponse> = registry.services.values()
        .map(|s| {
            let endpoints: Vec<EndpointResponse> = s.endpoints.iter()
                .map(|e| EndpointResponse {
                    id: e.id.clone(),
                    host: e.host.clone(),
                    port: e.port,
                    protocol: format!("{:?}", e.protocol),
                    weight: e.weight,
                    healthy: *e.healthy.read(),
                })
                .collect();

            ServiceResponse {
                id: s.name.clone(),
                name: s.name.clone(),
                endpoints,
                load_balancing: format!("{:?}", s.load_balancing),
                health_check_enabled: true,
                created_at: 0,
            }
        })
        .collect();

    Ok(AxumJson(services))
}

/// Register a backend service
#[utoipa::path(
    post,
    path = "/api/v1/gateway/services",
    tag = "gateway",
    request_body = RegisterServiceRequest,
    responses(
        (status = 201, description = "Service registered", body = ServiceResponse),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn register_service(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<RegisterServiceRequest>,
) -> ApiResult<(StatusCode, AxumJson<ServiceResponse>)> {
    let load_balancing = match request.load_balancing.to_lowercase().as_str() {
        "round_robin" => LoadBalancingStrategy::RoundRobin,
        "least_connections" => LoadBalancingStrategy::LeastConnections,
        "weighted_round_robin" => LoadBalancingStrategy::WeightedRoundRobin,
        "random" => LoadBalancingStrategy::Random,
        "ip_hash" => LoadBalancingStrategy::IpHash,
        "least_response_time" => LoadBalancingStrategy::LeastResponseTime,
        _ => return Err(ApiError::new("INVALID_STRATEGY", "Invalid load balancing strategy")),
    };

    let endpoints: Vec<ServiceEndpoint> = request.endpoints.iter()
        .map(|e| {
            let protocol = match e.protocol.to_lowercase().as_str() {
                "http" => Protocol::Http,
                "grpc" => Protocol::Grpc,
                "websocket" => Protocol::WebSocket,
                _ => Protocol::Http,
            };

            ServiceEndpoint {
                id: Uuid::new_v4().to_string(),
                host: e.host.clone(),
                port: e.port,
                protocol,
                weight: e.weight,
                healthy: Arc::new(RwLock::new(true)),
                last_health_check: Arc::new(RwLock::new(Instant::now())),
            }
        })
        .collect();

    let service = BackendService {
        name: request.name.clone(),
        endpoints: endpoints.clone(),
        load_balancing,
        circuit_breaker: CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: 60,
            half_open_max_requests: 3,
        },
        retry_policy: RetryPolicy {
            max_attempts: 3,
            initial_backoff: 100,
            max_backoff: 5000,
            backoff_multiplier: 2.0,
            retryable_status_codes: vec![502, 503, 504],
        },
    };

    let gateway = API_GATEWAY.read();
    gateway.register_service(service);

    let endpoint_responses: Vec<EndpointResponse> = endpoints.iter()
        .map(|e| EndpointResponse {
            id: e.id.clone(),
            host: e.host.clone(),
            port: e.port,
            protocol: format!("{:?}", e.protocol),
            weight: e.weight,
            healthy: *e.healthy.read(),
        })
        .collect();

    let response = ServiceResponse {
        id: request.name.clone(),
        name: request.name,
        endpoints: endpoint_responses,
        load_balancing: request.load_balancing,
        health_check_enabled: request.health_check_enabled,
        created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
    };

    Ok((StatusCode::CREATED, AxumJson(response)))
}

/// Update a backend service
#[utoipa::path(
    put,
    path = "/api/v1/gateway/services/{id}",
    tag = "gateway",
    params(
        ("id" = String, Path, description = "Service ID"),
    ),
    request_body = RegisterServiceRequest,
    responses(
        (status = 200, description = "Service updated", body = ServiceResponse),
        (status = 404, description = "Service not found", body = ApiError),
    )
)]
pub async fn update_service(
    State(state): State<Arc<ApiState>>,
    Path(_id): Path<String>,
    AxumJson(request): AxumJson<RegisterServiceRequest>,
) -> ApiResult<AxumJson<ServiceResponse>> {
    // For now, just re-register the service (would need proper update logic in production)
    let (_, response) = register_service(State(state), AxumJson(request)).await?;
    Ok(response)
}

/// Deregister a backend service
#[utoipa::path(
    delete,
    path = "/api/v1/gateway/services/{id}",
    tag = "gateway",
    params(
        ("id" = String, Path, description = "Service ID"),
    ),
    responses(
        (status = 204, description = "Service deregistered"),
        (status = 404, description = "Service not found", body = ApiError),
    )
)]
pub async fn deregister_service(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let gateway = API_GATEWAY.read();
    let mut registry = gateway.service_registry.write();

    if registry.services.remove(&id).is_some() {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new("NOT_FOUND", "Service not found"))
    }
}

/// Get service health status
#[utoipa::path(
    get,
    path = "/api/v1/gateway/services/{id}/health",
    tag = "gateway",
    params(
        ("id" = String, Path, description = "Service ID"),
    ),
    responses(
        (status = 200, description = "Service health status", body = ServiceHealthResponse),
        (status = 404, description = "Service not found", body = ApiError),
    )
)]
pub async fn get_service_health(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<AxumJson<ServiceHealthResponse>> {
    let gateway = API_GATEWAY.read();
    let registry = gateway.service_registry.read();

    let service = registry.services.get(&id)
        .ok_or_else(|| ApiError::new("NOT_FOUND", "Service not found"))?;

    let endpoint_health: Vec<EndpointHealthStatus> = service.endpoints.iter()
        .map(|e| {
            let healthy = *e.healthy.read();
            let last_check = *e.last_health_check.read();

            EndpointHealthStatus {
                endpoint_id: e.id.clone(),
                host: e.host.clone(),
                port: e.port,
                healthy,
                last_check_timestamp: last_check.elapsed().as_secs(),
            }
        })
        .collect();

    let healthy_count = endpoint_health.iter().filter(|e| e.healthy).count();
    let total_count = endpoint_health.len();

    let overall_health = if healthy_count == 0 {
        "unhealthy"
    } else if healthy_count == total_count {
        "healthy"
    } else {
        "degraded"
    };

    let response = ServiceHealthResponse {
        service_id: id.clone(),
        service_name: service.name.clone(),
        overall_health: overall_health.to_string(),
        endpoints: endpoint_health,
        last_check: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
    };

    Ok(AxumJson(response))
}

// ============================================================================
// GATEWAY METRICS ENDPOINTS
// ============================================================================

/// Get gateway metrics
#[utoipa::path(
    get,
    path = "/api/v1/gateway/metrics",
    tag = "gateway",
    responses(
        (status = 200, description = "Gateway metrics", body = GatewayMetricsResponse),
    )
)]
pub async fn get_gateway_metrics(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<GatewayMetricsResponse>> {
    let gateway = API_GATEWAY.read();
    let metrics = gateway.get_metrics();

    let avg_response_time = if metrics.successful_requests > 0 {
        metrics.total_duration_ms as f64 / metrics.successful_requests as f64
    } else {
        0.0
    };

    let requests_by_protocol: HashMap<String, u64> = metrics.requests_by_protocol
        .iter()
        .map(|(k, v)| (format!("{:?}", k), *v))
        .collect();

    let response = GatewayMetricsResponse {
        total_requests: metrics.total_requests,
        successful_requests: metrics.successful_requests,
        failed_requests: metrics.failed_requests,
        average_response_time_ms: avg_response_time,
        requests_by_protocol,
        requests_by_route: metrics.requests_by_route,
        auth_failures: metrics.auth_failures,
        authz_failures: metrics.authz_failures,
        rate_limit_hits: metrics.rate_limit_hits,
        security_blocks: metrics.security_blocks,
        uptime_seconds: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
    };

    Ok(AxumJson(response))
}

/// Get gateway audit log
#[utoipa::path(
    get,
    path = "/api/v1/gateway/audit",
    tag = "gateway",
    params(
        ("page" = Option<usize>, Query, description = "Page number"),
        ("page_size" = Option<usize>, Query, description = "Page size"),
    ),
    responses(
        (status = 200, description = "Audit log entries", body = AuditLogResponse),
    )
)]
pub async fn get_audit_log(
    State(_state): State<Arc<ApiState>>,
    Query(params): Query<ListQuery>,
) -> ApiResult<AxumJson<AuditLogResponse>> {
    // In production, this would query the actual audit log
    // For now, return empty list
    let response = AuditLogResponse {
        entries: vec![],
        total_count: 0,
        page: params.page,
        page_size: params.page_size,
    };

    Ok(AxumJson(response))
}

// ============================================================================
// SECURITY CONFIGURATION ENDPOINTS
// ============================================================================

/// Get IP filter rules
#[utoipa::path(
    get,
    path = "/api/v1/gateway/ip-filters",
    tag = "gateway",
    responses(
        (status = 200, description = "List of IP filter rules", body = Vec<IpFilterResponse>),
    )
)]
pub async fn get_ip_filters(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<IpFilterResponse>>> {
    let filters = IP_FILTERS.read();

    let responses: Vec<IpFilterResponse> = filters.values()
        .map(|f| IpFilterResponse {
            id: f.id.clone(),
            ip_address: f.ip_address.to_string(),
            filter_type: f.filter_type.clone(),
            description: f.description.clone(),
            created_at: f.created_at,
        })
        .collect();

    Ok(AxumJson(responses))
}

/// Add IP filter rule
#[utoipa::path(
    post,
    path = "/api/v1/gateway/ip-filters",
    tag = "gateway",
    request_body = CreateIpFilterRequest,
    responses(
        (status = 201, description = "IP filter created", body = IpFilterResponse),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn add_ip_filter(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<CreateIpFilterRequest>,
) -> ApiResult<(StatusCode, AxumJson<IpFilterResponse>)> {
    let id = Uuid::new_v4().to_string();

    // Parse IP address
    let ip_addr: IpAddr = request.ip_address.parse()
        .map_err(|_| ApiError::new("INVALID_IP", "Invalid IP address"))?;

    // Validate filter type
    if request.filter_type != "whitelist" && request.filter_type != "blacklist" {
        return Err(ApiError::new("INVALID_FILTER_TYPE", "Filter type must be 'whitelist' or 'blacklist'"));
    }

    let filter = IpFilterRule {
        id: id.clone(),
        ip_address: ip_addr,
        filter_type: request.filter_type.clone(),
        description: request.description.clone(),
        created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
    };

    let mut filters = IP_FILTERS.write();
    filters.insert(id.clone(), filter.clone());

    let response = IpFilterResponse {
        id,
        ip_address: request.ip_address,
        filter_type: request.filter_type,
        description: request.description,
        created_at: filter.created_at,
    };

    Ok((StatusCode::CREATED, AxumJson(response)))
}

/// Remove IP filter rule
#[utoipa::path(
    delete,
    path = "/api/v1/gateway/ip-filters/{id}",
    tag = "gateway",
    params(
        ("id" = String, Path, description = "IP filter ID"),
    ),
    responses(
        (status = 204, description = "IP filter deleted"),
        (status = 404, description = "IP filter not found", body = ApiError),
    )
)]
pub async fn remove_ip_filter(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let mut filters = IP_FILTERS.write();

    if filters.remove(&id).is_some() {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new("NOT_FOUND", "IP filter not found"))
    }
}
