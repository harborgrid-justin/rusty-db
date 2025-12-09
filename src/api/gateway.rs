// # API Gateway & Security Layer
//
// Enterprise-grade API gateway providing comprehensive security, authentication,
// authorization, rate limiting, and request routing for RustyDB.
// //
// ## Features
//
// - **Multi-Protocol Support**: HTTP/REST, gRPC, WebSocket
// - **Authentication**: JWT, OAuth 2.0, OIDC, mTLS, API Keys
// - **Authorization**: RBAC, ABAC, Policy-based (OPA-compatible)
// - **Rate Limiting**: Token bucket, sliding window, adaptive throttling
// - **Security**: Request validation, SQL injection prevention, XSS protection
// - **Audit Logging**: Comprehensive security event tracking
// - **Service Discovery**: Dynamic backend routing
//
// ## Architecture
//
// ```text
// ┌─────────────────────────────────────────────────────────────┐
// │                     API Gateway Layer                        │
// ├─────────────────────────────────────────────────────────────┤
// │  Request Router  │  Auth  │  Authz  │  Rate Limit │  WAF    │
// ├─────────────────────────────────────────────────────────────┤
// │            Protocol Translation & Transformation             │
// ├─────────────────────────────────────────────────────────────┤
// │                    Backend Services                          │
// └─────────────────────────────────────────────────────────────┘
// ```

use std::collections::VecDeque;
use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Instant;
use std::time::SystemTime;
use std::collections::{HashMap};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration};

use parking_lot::{RwLock};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Sha512, Digest};
use hmac::{Hmac, Mac};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use uuid::Uuid;

use crate::error::DbError;

type Result<T> = std::result::Result<T, DbError>;

// ============================================================================
// API Gateway Core - Request Routing and Protocol Translation
// ============================================================================

/// API Gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Gateway listen address
    pub listen_address: String,
    /// Gateway listen port
    pub listen_port: u16,
    /// Enable HTTPS/TLS
    pub enable_tls: bool,
    /// TLS certificate path
    pub tls_cert_path: Option<String>,
    /// TLS key path
    pub tls_key_path: Option<String>,
    /// Enable mTLS (mutual TLS)
    pub enable_mtls: bool,
    /// Trusted CA certificates for mTLS
    pub mtls_ca_certs: Vec<String>,
    /// Enable HTTP/2
    pub enable_http2: bool,
    /// Enable gRPC
    pub enable_grpc: bool,
    /// Enable WebSocket
    pub enable_websocket: bool,
    /// Request timeout (seconds)
    pub request_timeout: u64,
    /// Maximum request size (bytes)
    pub max_request_size: usize,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Enable request compression
    pub enable_compression: bool,
    /// Enable CORS
    pub enable_cors: bool,
    /// Allowed CORS origins
    pub cors_origins: Vec<String>,
    /// Service discovery backend
    pub service_discovery: ServiceDiscoveryConfig,
}

/// Service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoveryConfig {
    /// Discovery mechanism
    pub mechanism: DiscoveryMechanism,
    /// Service registry address
    pub registry_address: Option<String>,
    /// Health check interval (seconds)
    pub health_check_interval: u64,
    /// Health check timeout (seconds)
    pub health_check_timeout: u64,
}

/// Service discovery mechanism
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiscoveryMechanism {
    /// Static configuration
    Static,
    /// Consul integration
    Consul,
    /// Kubernetes service discovery
    Kubernetes,
    /// etcd-based discovery
    Etcd,
    /// DNS-based discovery
    Dns,
}

/// Protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    /// HTTP/1.1 or HTTP/2
    Http,
    /// gRPC over HTTP/2
    Grpc,
    /// WebSocket
    WebSocket,
}

/// HTTP method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

/// API request representation
#[derive(Debug, Clone)]
pub struct ApiRequest {
    /// Request ID for tracing
    pub request_id: String,
    /// Protocol
    pub protocol: Protocol,
    /// HTTP method
    pub method: HttpMethod,
    /// Request path
    pub path: String,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Headers
    pub headers: HashMap<String, String>,
    /// Request body
    pub body: Vec<u8>,
    /// Client IP address
    pub client_ip: IpAddr,
    /// Client port
    pub client_port: u16,
    /// Request timestamp
    pub timestamp: SystemTime,
}

/// API response representation
#[derive(Debug, Clone)]
pub struct ApiResponse {
    /// Status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Vec<u8>,
    /// Processing duration
    pub duration: Duration,
}

/// Route definition
#[derive(Debug, Clone)]
pub struct Route {
    /// Route ID
    pub id: String,
    /// Route name
    pub name: String,
    /// Path pattern (supports wildcards)
    pub path_pattern: String,
    /// Allowed methods
    pub methods: Vec<HttpMethod>,
    /// Backend service
    pub backend: BackendService,
    /// Route-specific middleware
    pub middleware: Vec<String>,
    /// Rate limit override
    pub rate_limit: Option<RateLimitConfig>,
    /// Authentication required
    pub auth_required: bool,
    /// Required permissions
    pub required_permissions: Vec<String>,
    /// Request transformation
    pub request_transform: Option<RequestTransform>,
    /// Response transformation
    pub response_transform: Option<ResponseTransform>,
    /// Enable caching
    pub enable_cache: bool,
    /// Cache TTL (seconds)
    pub cache_ttl: Option<u64>,
}

/// Backend service configuration
#[derive(Debug, Clone)]
pub struct BackendService {
    /// Service name
    pub name: String,
    /// Service endpoints
    pub endpoints: Vec<ServiceEndpoint>,
    /// Load balancing strategy
    pub load_balancing: LoadBalancingStrategy,
    /// Circuit breaker config
    pub circuit_breaker: CircuitBreakerConfig,
    /// Retry policy
    pub retry_policy: RetryPolicy,
}

/// Service endpoint
#[derive(Debug, Clone)]
pub struct ServiceEndpoint {
    /// Endpoint ID
    pub id: String,
    /// Host address
    pub host: String,
    /// Port
    pub port: u16,
    /// Protocol
    pub protocol: Protocol,
    /// Weight for load balancing
    pub weight: u32,
    /// Health status
    pub healthy: Arc<RwLock<bool>>,
    /// Last health check
    pub last_health_check: Arc<RwLock<Instant>>,
}

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    /// Round-robin
    RoundRobin,
    /// Least connections
    LeastConnections,
    /// Weighted round-robin
    WeightedRoundRobin,
    /// Random
    Random,
    /// IP hash (sticky sessions)
    IpHash,
    /// Least response time
    LeastResponseTime,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Failure threshold
    pub failure_threshold: u32,
    /// Success threshold to close circuit
    pub success_threshold: u32,
    /// Timeout duration (seconds)
    pub timeout: u64,
    /// Half-open max requests
    pub half_open_max_requests: u32,
}

/// Retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Initial backoff (milliseconds)
    pub initial_backoff: u64,
    /// Max backoff (milliseconds)
    pub max_backoff: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Retryable status codes
    pub retryable_status_codes: Vec<u16>,
}

/// Request transformation
#[derive(Debug, Clone)]
pub struct RequestTransform {
    /// Add headers
    pub add_headers: HashMap<String, String>,
    /// Remove headers
    pub remove_headers: Vec<String>,
    /// Path rewrite rules
    pub path_rewrites: Vec<PathRewrite>,
    /// Query parameter transformations
    pub query_transforms: Vec<QueryTransform>,
}

/// Response transformation
#[derive(Debug, Clone)]
pub struct ResponseTransform {
    /// Add headers
    pub add_headers: HashMap<String, String>,
    /// Remove headers
    pub remove_headers: Vec<String>,
    /// Body transformation
    pub body_transform: Option<BodyTransform>,
}

/// Path rewrite rule
#[derive(Debug, Clone)]
pub struct PathRewrite {
    /// Pattern to match
    pub pattern: String,
    /// Replacement
    pub replacement: String,
}

/// Query parameter transformation
#[derive(Debug, Clone)]
pub struct QueryTransform {
    /// Parameter name
    pub name: String,
    /// Transformation type
    pub transform_type: QueryTransformType,
}

/// Query transformation type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryTransformType {
    /// Add parameter
    Add(String),
    /// Remove parameter
    Remove,
    /// Rename parameter
    Rename(String),
}

/// Body transformation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyTransform {
    /// No transformation
    None,
    /// JSON to XML
    JsonToXml,
    /// XML to JSON
    XmlToJson,
    /// Custom transformation
    Custom(String),
}

/// API Gateway core engine
pub struct ApiGateway {
    /// Configuration
    config: Arc<RwLock<GatewayConfig>>,
    /// Route registry
    routes: Arc<RwLock<HashMap<String, Route>>>,
    /// Authentication manager
    auth_manager: Arc<AuthenticationManager>,
    /// Authorization engine
    authz_engine: Arc<AuthorizationEngine>,
    /// Rate limiter
    rate_limiter: Arc<RateLimiter>,
    /// Security filter
    security_filter: Arc<SecurityFilter>,
    /// Service registry
    service_registry: Arc<RwLock<ServiceRegistry>>,
    /// Request metrics
    metrics: Arc<RwLock<GatewayMetrics>>,
    /// Audit logger
    audit_logger: Arc<Mutex<AuditLogger>>,
}

/// Service registry
pub struct ServiceRegistry {
    /// Registered services
    services: HashMap<String, BackendService>,
    /// Service health status
    health_status: HashMap<String, bool>,
}

/// Gateway metrics
#[derive(Debug, Default)]
pub struct GatewayMetrics {
    /// Total requests
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Total request duration
    pub total_duration_ms: u64,
    /// Requests per protocol
    pub requests_by_protocol: HashMap<Protocol, u64>,
    /// Requests per route
    pub requests_by_route: HashMap<String, u64>,
    /// Authentication failures
    pub auth_failures: u64,
    /// Authorization failures
    pub authz_failures: u64,
    /// Rate limit hits
    pub rate_limit_hits: u64,
    /// Security blocks
    pub security_blocks: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            listen_address: "0.0.0.0".to_string(),
            listen_port: 8080,
            enable_tls: true,
            tls_cert_path: None,
            tls_key_path: None,
            enable_mtls: false,
            mtls_ca_certs: Vec::new(),
            enable_http2: true,
            enable_grpc: true,
            enable_websocket: true,
            request_timeout: 30,
            max_request_size: 10 * 1024 * 1024, // 10MB
            max_connections: 10000,
            enable_compression: true,
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
            service_discovery: ServiceDiscoveryConfig::default(),
        }
    }
}

impl Default for ServiceDiscoveryConfig {
    fn default() -> Self {
        Self {
            mechanism: DiscoveryMechanism::Static,
            registry_address: None,
            health_check_interval: 10,
            health_check_timeout: 5,
        }
    }
}

impl ApiGateway {
    /// Create a new API gateway
    pub fn new(config: GatewayConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            routes: Arc::new(RwLock::new(HashMap::new())),
            auth_manager: Arc::new(AuthenticationManager::new()),
            authz_engine: Arc::new(AuthorizationEngine::new()),
            rate_limiter: Arc::new(RateLimiter::new()),
            security_filter: Arc::new(SecurityFilter::new()),
            service_registry: Arc::new(RwLock::new(ServiceRegistry::new())),
            metrics: Arc::new(RwLock::new(GatewayMetrics::default())),
            audit_logger: Arc::new(Mutex::new(AuditLogger::new())),
        }
    }

    /// Register a new route
    pub fn register_route(&self, route: Route) {
        let mut routes = self.routes.write();
        routes.insert(route.id.clone(), route);
    }

    /// Remove a route
    pub fn remove_route(&self, route_id: &str) -> bool {
        let mut routes = self.routes.write();
        routes.remove(route_id).is_some()
    }

    /// Get all routes
    pub fn get_routes(&self) -> Vec<Route> {
        let routes = self.routes.read();
        routes.values().cloned().collect()
    }

    /// Find matching route for request
    pub fn find_route(&self, request: &ApiRequest) -> Option<Route> {
        let routes = self.routes.read();

        for route in routes.values() {
            if self.matches_route(route, request) {
                return Some(route.clone());
            }
        }

        None
    }

    /// Check if request matches route
    fn matches_route(&self, route: &Route, request: &ApiRequest) -> bool {
        // Check method
        if !route.methods.contains(&request.method) {
            return false;
        }

        // Check path pattern (simplified wildcard matching)
        self.matches_path_pattern(&route.path_pattern, &request.path)
    }

    /// Match path against pattern
    fn matches_path_pattern(&self, pattern: &str, path: &str) -> bool {
        if pattern == path {
            return true;
        }

        // Simple wildcard matching
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                return path.starts_with(parts[0]) && path.ends_with(parts[1]);
            }
        }

        false
    }

    /// Process incoming request
    pub async fn process_request(&self, request: ApiRequest) -> std::result::Result<ApiResponse, DbError> {
        let start = Instant::now();

        // Update metrics
        {
            let mut metrics = self.metrics.write();
            metrics.total_requests += 1;
            *metrics.requests_by_protocol.entry(request.protocol).or_insert(0) += 1;
        }

        // Log request
        self.audit_logger.lock().unwrap().log_request(&request);

        // Security filtering
        if let Err(e) = self.security_filter.validate_request(&request) {
            let mut metrics = self.metrics.write();
            metrics.security_blocks += 1;
            metrics.failed_requests += 1;

            self.audit_logger.lock().unwrap().log_security_event(&SecurityEvent {
                event_type: SecurityEventType::RequestBlocked,
                request_id: request.request_id.clone(),
                client_ip: request.client_ip,
                reason: format!("Security filter: {}", e),
                timestamp: SystemTime::now(),
            });

            return Ok(ApiResponse {
                status_code: 403,
                headers: HashMap::new(),
                body: b"Forbidden".to_vec(),
                duration: start.elapsed(),
            });
        }

        // Find matching route
        let route = match self.find_route(&request) {
            Some(r) => r,
            None => {
                let mut metrics = self.metrics.write();
                metrics.failed_requests += 1;

                return Ok(ApiResponse {
                    status_code: 404,
                    headers: HashMap::new(),
                    body: b"Not Found".to_vec(),
                    duration: start.elapsed(),
                });
            }
        };

        // Update route metrics
        {
            let mut metrics = self.metrics.write();
            *metrics.requests_by_route.entry(route.id.clone()).or_insert(0) += 1;
        }

        // Authentication
        if route.auth_required {
            match self.auth_manager.authenticate(&request).await {
                Ok(session) => {
                    // Authorization
                    if !route.required_permissions.is_empty() {
                        match self.authz_engine.authorize(&session, &route.required_permissions) {
                            Ok(true) => {},
                            Ok(false) | Err(_) => {
                                let mut metrics = self.metrics.write();
                                metrics.authz_failures += 1;
                                metrics.failed_requests += 1;

                                self.audit_logger.lock().unwrap().log_security_event(&SecurityEvent {
                                    event_type: SecurityEventType::AuthorizationFailed,
                                    request_id: request.request_id.clone(),
                                    client_ip: request.client_ip,
                                    reason: format!("Missing permissions: {:?}", route.required_permissions),
                                    timestamp: SystemTime::now(),
                                });

                                return Ok(ApiResponse {
                                    status_code: 403,
                                    headers: HashMap::new(),
                                    body: b"Forbidden".to_vec(),
                                    duration: start.elapsed(),
                                });
                            }
                        }
                    }
                },
                Err(_) => {
                    let mut metrics = self.metrics.write();
                    metrics.auth_failures += 1;
                    metrics.failed_requests += 1;

                    self.audit_logger.lock().unwrap().log_security_event(&SecurityEvent {
                        event_type: SecurityEventType::AuthenticationFailed,
                        request_id: request.request_id.clone(),
                        client_ip: request.client_ip,
                        reason: "Authentication failed".to_string(),
                        timestamp: SystemTime::now(),
                    });

                    return Ok(ApiResponse {
                        status_code: 401,
                        headers: HashMap::new(),
                        body: b"Unauthorized".to_vec(),
                        duration: start.elapsed(),
                    });
                }
            }
        }

        // Rate limiting
        let rate_limit_key = format!("{}:{}", request.client_ip, route.id);
        if let Err(_) = self.rate_limiter.check_rate_limit(
            &rate_limit_key,
            route.rate_limit.as_ref()
        ) {
            let mut metrics = self.metrics.write();
            metrics.rate_limit_hits += 1;
            metrics.failed_requests += 1;

            return Ok(ApiResponse {
                status_code: 429,
                headers: HashMap::new(),
                body: b"Too Many Requests".to_vec(),
                duration: start.elapsed(),
            });
        }

        // Transform request
        let transformed_request = self.transform_request(&request, &route);

        // Forward to backend
        let response = self.forward_to_backend(&transformed_request, &route).await?;

        // Transform response
        let transformed_response = self.transform_response(response, &route);

        // Update success metrics
        {
            let mut metrics = self.metrics.write();
            metrics.successful_requests += 1;
            metrics.total_duration_ms += start.elapsed().as_millis() as u64;
        }

        Ok(transformed_response)
    }

    /// Transform request before forwarding
    fn transform_request(&self, request: &ApiRequest, route: &Route) -> ApiRequest {
        let mut transformed = request.clone();

        if let Some(transform) = &route.request_transform {
            // Add headers
            for (key, value) in &transform.add_headers {
                transformed.headers.insert(key.clone(), value.clone());
            }

            // Remove headers
            for key in &transform.remove_headers {
                transformed.headers.remove(key);
            }

            // Apply path rewrites
            for rewrite in &transform.path_rewrites {
                transformed.path = transformed.path.replace(&rewrite.pattern, &rewrite.replacement);
            }

            // Apply query transformations
            for query_transform in &transform.query_transforms {
                match &query_transform.transform_type {
                    QueryTransformType::Add(value) => {
                        transformed.query_params.insert(query_transform.name.clone(), value.clone());
                    },
                    QueryTransformType::Remove => {
                        transformed.query_params.remove(&query_transform.name);
                    },
                    QueryTransformType::Rename(new_name) => {
                        if let Some(value) = transformed.query_params.remove(&query_transform.name) {
                            transformed.query_params.insert(new_name.clone(), value);
                        }
                    }
                }
            }
        }

        transformed
    }

    /// Transform response before returning
    fn transform_response(&self, mut response: ApiResponse, route: &Route) -> ApiResponse {
        if let Some(transform) = &route.response_transform {
            // Add headers
            for (key, value) in &transform.add_headers {
                response.headers.insert(key.clone(), value.clone());
            }

            // Remove headers
            for key in &transform.remove_headers {
                response.headers.remove(key);
            }
        }

        response
    }

    /// Forward request to backend service
    async fn forward_to_backend(&self, request: &ApiRequest, route: &Route) -> std::result::Result<ApiResponse, DbError> {
        // Select endpoint using load balancing
        let endpoint = self.select_endpoint(&route.backend)?;

        // TODO: Implement actual HTTP/gRPC/WebSocket client
        // For now, return a mock response
        Ok(ApiResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: b"OK".to_vec(),
            duration: Duration::from_millis(10),
        })
    }

    /// Select backend endpoint using load balancing strategy
    fn select_endpoint(&self, backend: &BackendService) -> std::result::Result<ServiceEndpoint, DbError> {
        let healthy_endpoints: Vec<_> = backend.endpoints.iter()
            .filter(|e| *e.healthy.read())
            .collect();

        if healthy_endpoints.is_empty() {
            return Err(DbError::InvalidOperation(
                "No healthy endpoints available".to_string()
            ));
        }

        match backend.load_balancing {
            LoadBalancingStrategy::Random => {
                let idx = rand::random::<usize>() % healthy_endpoints.len();
                Ok(healthy_endpoints[idx].clone())
            },
            LoadBalancingStrategy::RoundRobin => {
                // Simplified round-robin
                Ok(healthy_endpoints[0].clone())
            },
            _ => Ok(healthy_endpoints[0].clone()),
        }
    }

    /// Get gateway metrics
    pub fn get_metrics(&self) -> GatewayMetrics {
        self.metrics.read().clone()
    }

    /// Register backend service
    pub fn register_service(&self, service: BackendService) {
        let mut registry = self.service_registry.write();
        registry.services.insert(service.name.clone(), service);
    }

    /// Start health checking for all services
    pub async fn start_health_checks(&self) {
        // TODO: Implement periodic health checks
    }
}

impl ServiceRegistry {
    fn new() -> Self {
        Self {
            services: HashMap::new(),
            health_status: HashMap::new(),
        }
    }
}

impl Clone for GatewayMetrics {
    fn clone(&self) -> Self {
        Self {
            total_requests: self.total_requests,
            successful_requests: self.successful_requests,
            failed_requests: self.failed_requests,
            total_duration_ms: self.total_duration_ms,
            requests_by_protocol: self.requests_by_protocol.clone(),
            requests_by_route: self.requests_by_route.clone(),
            auth_failures: self.auth_failures,
            authz_failures: self.authz_failures,
            rate_limit_hits: self.rate_limit_hits,
            security_blocks: self.security_blocks,
        }
    }
}

// ============================================================================
// Authentication System - JWT, OAuth, API Keys, mTLS
// ============================================================================

/// Authentication manager
pub struct AuthenticationManager {
    /// JWT validator
    jwt_validator: Arc<JwtValidator>,
    /// OAuth provider
    oauth_provider: Arc<OAuthProvider>,
    /// API key store
    api_key_store: Arc<RwLock<ApiKeyStore>>,
    /// Session manager
    session_manager: Arc<SessionManager>,
    /// mTLS validator
    mtls_validator: Arc<MtlsValidator>,
    /// MFA manager
    mfa_manager: Arc<MfaManager>,
}

/// JWT validator
pub struct JwtValidator {
    /// Signing keys (kid -> key)
    signing_keys: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    /// Issuer
    issuer: String,
    /// Audience
    audience: Vec<String>,
    /// Algorithm
    algorithm: JwtAlgorithm,
    /// Token expiration tolerance (seconds)
    expiration_tolerance: u64,
}

/// JWT algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JwtAlgorithm {
    HS256,
    HS384,
    HS512,
    RS256,
    RS384,
    RS512,
    ES256,
    ES384,
    ES512,
}

/// JWT claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Issuer
    pub iss: String,
    /// Subject
    pub sub: String,
    /// Audience
    pub aud: Vec<String>,
    /// Expiration time
    pub exp: u64,
    /// Not before
    pub nbf: Option<u64>,
    /// Issued at
    pub iat: u64,
    /// JWT ID
    pub jti: String,
    /// Custom claims
    pub custom: HashMap<String, serde_json::Value>,
}

/// OAuth 2.0 provider
pub struct OAuthProvider {
    /// Provider configuration
    config: Arc<RwLock<OAuthConfig>>,
    /// Token cache
    token_cache: Arc<RwLock<HashMap<String, OAuthToken>>>,
}

/// OAuth configuration
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// Authorization endpoint
    pub auth_endpoint: String,
    /// Token endpoint
    pub token_endpoint: String,
    /// Userinfo endpoint
    pub userinfo_endpoint: Option<String>,
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Redirect URI
    pub redirect_uri: String,
    /// Scopes
    pub scopes: Vec<String>,
    /// OIDC discovery URL
    pub discovery_url: Option<String>,
}

/// OAuth token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    /// Access token
    pub access_token: String,
    /// Token type
    pub token_type: String,
    /// Expires in (seconds)
    pub expires_in: u64,
    /// Refresh token
    pub refresh_token: Option<String>,
    /// Scope
    pub scope: Option<String>,
    /// ID token (OIDC)
    pub id_token: Option<String>,
}

/// API key store
pub struct ApiKeyStore {
    /// API keys (key -> metadata)
    keys: HashMap<String, ApiKeyMetadata>,
}

/// API key metadata
#[derive(Debug, Clone)]
pub struct ApiKeyMetadata {
    /// Key ID
    pub key_id: String,
    /// User ID
    pub user_id: String,
    /// Key hash (never store plain keys)
    pub key_hash: Vec<u8>,
    /// Created at
    pub created_at: SystemTime,
    /// Expires at
    pub expires_at: Option<SystemTime>,
    /// Enabled
    pub enabled: bool,
    /// Scopes/permissions
    pub scopes: Vec<String>,
    /// Last used
    pub last_used: Option<SystemTime>,
    /// Usage count
    pub usage_count: u64,
}

/// Session manager
pub struct SessionManager {
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    /// Session timeout (seconds)
    session_timeout: u64,
}

/// User session
#[derive(Debug, Clone)]
pub struct Session {
    /// Session ID
    pub session_id: String,
    /// User ID
    pub user_id: String,
    /// Username
    pub username: String,
    /// Roles
    pub roles: Vec<String>,
    /// Permissions
    pub permissions: Vec<String>,
    /// Created at
    pub created_at: SystemTime,
    /// Last accessed
    pub last_accessed: SystemTime,
    /// Expires at
    pub expires_at: SystemTime,
    /// IP address
    pub ip_address: IpAddr,
    /// User agent
    pub user_agent: Option<String>,
    /// Additional attributes
    pub attributes: HashMap<String, String>,
}

/// mTLS validator
pub struct MtlsValidator {
    /// Trusted CA certificates
    trusted_cas: Arc<RwLock<Vec<Vec<u8>>>>,
    /// Certificate revocation list
    crl: Arc<RwLock<HashSet<String>>>,
}

/// MFA manager
pub struct MfaManager {
    /// TOTP secrets
    totp_secrets: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    /// Backup codes
    backup_codes: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl AuthenticationManager {
    /// Create new authentication manager
    pub fn new() -> Self {
        Self {
            jwt_validator: Arc::new(JwtValidator::new("rustydb".to_string())),
            oauth_provider: Arc::new(OAuthProvider::new()),
            api_key_store: Arc::new(RwLock::new(ApiKeyStore::new())),
            session_manager: Arc::new(SessionManager::new()),
            mtls_validator: Arc::new(MtlsValidator::new()),
            mfa_manager: Arc::new(MfaManager::new()),
        }
    }

    /// Authenticate request
    pub async fn authenticate(&self, request: &ApiRequest) -> std::result::Result<Session, DbError> {
        // Try different authentication methods in order

        // 1. Try JWT bearer token
        if let Some(auth_header) = request.headers.get("Authorization") {
            if auth_header.starts_with("Bearer ") {
                let token = &auth_header[7..];
                if let Ok(claims) = self.jwt_validator.validate(token) {
                    return self.create_session_from_jwt(claims, request);
                }
            }
        }

        // 2. Try API key
        if let Some(api_key) = request.headers.get("X-API-Key") {
            if let Ok(session) = self.authenticate_api_key(api_key, request).await {
                return Ok(session);
            }
        }

        // 3. Try session cookie
        if let Some(cookie) = request.headers.get("Cookie") {
            if let Some(session_id) = self.extract_session_id(cookie) {
                if let Some(session) = self.session_manager.get_session(&session_id) {
                    return Ok(session);
                }
            }
        }

        Err(DbError::InvalidOperation("Authentication failed".to_string()))
    }

    /// Create session from JWT claims
    fn create_session_from_jwt(&self, claims: JwtClaims, request: &ApiRequest) -> std::result::Result<Session, DbError> {
        let session = Session {
            session_id: Uuid::new_v4().to_string(),
            user_id: claims.sub.clone(),
            username: claims.custom.get("username")
                .and_then(|v| v.as_str())
                .unwrap_or(&claims.sub)
                .to_string(),
            roles: claims.custom.get("roles")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            permissions: Vec::new(),
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            expires_at: UNIX_EPOCH + Duration::from_secs(claims.exp),
            ip_address: request.client_ip,
            user_agent: request.headers.get("User-Agent").cloned(),
            attributes: HashMap::new(),
        };

        Ok(session)
    }

    /// Authenticate using API key
    async fn authenticate_api_key(&self, apikey: &str, request: &ApiRequest) -> std::result::Result<Session, DbError> {
        let key_store = self.api_key_store.read();

        // Hash the provided key
        let key_hash = Self::hash_api_key(api_key);

        // Find matching key
        for metadata in key_store.keys.values() {
            if metadata.key_hash == key_hash && metadata.enabled {
                // Check expiration
                if let Some(expires_at) = metadata.expires_at {
                    if SystemTime::now() > expires_at {
                        continue;
                    }
                }

                // Create session
                let session = Session {
                    session_id: Uuid::new_v4().to_string(),
                    user_id: metadata.user_id.clone(),
                    username: metadata.user_id.clone(),
                    roles: vec!["api_user".to_string()],
                    permissions: metadata.scopes.clone(),
                    created_at: SystemTime::now(),
                    last_accessed: SystemTime::now(),
                    expires_at: SystemTime::now() + Duration::from_secs(3600),
                    ip_address: request.client_ip,
                    user_agent: request.headers.get("User-Agent").cloned(),
                    attributes: HashMap::new(),
                };

                return Ok(session);
            }
        }

        Err(DbError::InvalidOperation("Invalid API key".to_string()))
    }

    /// Extract session ID from cookie
    fn extract_session_id(&self, cookie: &str) -> Option<String> {
        for part in cookie.split(';') {
            let part = part.trim();
            if part.starts_with("session_id=") {
                return Some(part[11..].to_string());
            }
        }
        None
    }

    /// Hash API key
    fn hash_api_key(key: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        hasher.finalize().to_vec()
    }

    /// Generate new API key
    pub fn generate_api_key(&self, user_id: String, scopes: Vec<String>) -> std::result::Result<String, DbError> {
        let key = Uuid::new_v4().to_string();
        let key_hash = Self::hash_api_key(&key);

        let metadata = ApiKeyMetadata {
            key_id: Uuid::new_v4().to_string(),
            user_id,
            key_hash,
            created_at: SystemTime::now(),
            expires_at: None,
            enabled: true,
            scopes,
            last_used: None,
            usage_count: 0,
        };

        let mut key_store = self.api_key_store.write();
        key_store.keys.insert(metadata.key_id.clone(), metadata);

        Ok(key)
    }

    /// Revoke API key
    pub fn revoke_api_key(&self, key_id: &str) -> bool {
        let mut key_store = self.api_key_store.write();
        if let Some(metadata) = key_store.keys.get_mut(key_id) {
            metadata.enabled = false;
            true
        } else {
            false
        }
    }

    /// List API keys for user
    pub fn list_api_keys(&self, user_id: &str) -> Vec<ApiKeyMetadata> {
        let key_store = self.api_key_store.read();
        key_store.keys.values()
            .filter(|m| m.user_id == user_id)
            .cloned()
            .collect()
    }
}

impl JwtValidator {
    /// Create new JWT validator
    pub fn new(issuer: String) -> Self {
        Self {
            signing_keys: Arc::new(RwLock::new(HashMap::new())),
            issuer,
            audience: vec!["rustydb".to_string()],
            algorithm: JwtAlgorithm::HS256,
            expiration_tolerance: 60,
        }
    }

    /// Validate JWT token
    pub fn validate(&self, token: &str) -> std::result::Result<JwtClaims, DbError> {
        // Parse token
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(DbError::InvalidOperation("Invalid JWT format".to_string()));
        }

        // Decode header
        let header_data = BASE64.decode(parts[0])
            .map_err(|_| DbError::InvalidOperation("Invalid JWT header".to_string()))?;
        let header: serde_json::Value = serde_json::from_slice(&header_data)
            .map_err(|_| DbError::InvalidOperation("Invalid JWT header JSON".to_string()))?;

        // Decode payload
        let payload_data = BASE64.decode(parts[1])
            .map_err(|_| DbError::InvalidOperation("Invalid JWT payload".to_string()))?;
        let claims: JwtClaims = serde_json::from_slice(&payload_data)
            .map_err(|_| DbError::InvalidOperation("Invalid JWT claims".to_string()))?;

        // Verify signature
        let signature = BASE64.decode(parts[2])
            .map_err(|_| DbError::InvalidOperation("Invalid JWT signature".to_string()))?;

        let message = format!("{}.{}", parts[0], parts[1]);
        self.verify_signature(&message, &signature)?;

        // Validate claims
        self.validate_claims(&claims)?;

        Ok(claims)
    }

    /// Verify signature
    fn verify_signature(&self, message: &str, signature: &[u8]) -> std::result::Result<(), DbError> {
        // TODO: Implement proper signature verification based on algorithm
        // For now, just check signature is not empty
        if signature.is_empty() {
            return Err(DbError::InvalidOperation("Empty signature".to_string()));
        }
        Ok(())
    }

    /// Validate claims
    fn validate_claims(&self, claims: &JwtClaims) -> std::result::Result<(), DbError> {
        // Check issuer
        if claims.iss != self.issuer {
            return Err(DbError::InvalidOperation("Invalid issuer".to_string()));
        }

        // Check audience
        if !claims.aud.iter().any(|aud| self.audience.contains(aud)) {
            return Err(DbError::InvalidOperation("Invalid audience".to_string()));
        }

        // Check expiration
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if claims.exp + self.expiration_tolerance < now {
            return Err(DbError::InvalidOperation("Token expired".to_string()));
        }

        // Check not before
        if let Some(nbf) = claims.nbf {
            if nbf > now {
                return Err(DbError::InvalidOperation("Token not yet valid".to_string()));
            }
        }

        Ok(())
    }

    /// Add signing key
    pub fn add_signing_key(&self, kid: String, key: Vec<u8>) {
        let mut keys = self.signing_keys.write();
        keys.insert(kid, key);
    }
}

impl OAuthProvider {
    /// Create new OAuth provider
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(OAuthConfig::default())),
            token_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Configure OAuth provider
    pub fn configure(&self, config: OAuthConfig) {
        let mut cfg = self.config.write();
        *cfg = config;
    }

    /// Exchange authorization code for token
    pub async fn exchange_code(&self, code: &str) -> Result<OAuthToken> {
        // TODO: Implement actual OAuth token exchange
        Err(DbError::InvalidOperation("Not implemented".to_string()))
    }

    /// Refresh access token
    pub async fn refresh_token(&self, refreshtoken: &str) -> Result<OAuthToken> {
        // TODO: Implement token refresh
        Err(DbError::InvalidOperation("Not implemented".to_string()))
    }
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            auth_endpoint: String::new(),
            token_endpoint: String::new(),
            userinfo_endpoint: None,
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: String::new(),
            scopes: Vec::new(),
            discovery_url: None,
        }
    }
}

impl ApiKeyStore {
    fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }
}

impl SessionManager {
    fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_timeout: 3600,
        }
    }

    /// Get session by ID
    fn get_session(&self, session_id: &str) -> Option<Session> {
        let sessions = self.sessions.read();
        sessions.get(session_id).cloned()
    }

    /// Create new session
    pub fn create_session(&self, session: Session) {
        let mut sessions = self.sessions.write();
        sessions.insert(session.session_id.clone(), session);
    }

    /// Invalidate session
    pub fn invalidate_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write();
        sessions.remove(session_id).is_some()
    }

    /// Cleanup expired sessions
    pub fn cleanup_expired_sessions(&self) {
        let mut sessions = self.sessions.write();
        let now = SystemTime::now();
        sessions.retain(|_, session| session.expires_at > now);
    }
}

impl MtlsValidator {
    fn new() -> Self {
        Self {
            trusted_cas: Arc::new(RwLock::new(Vec::new())),
            crl: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Add trusted CA certificate
    pub fn add_trusted_ca(&self, cert: Vec<u8>) {
        let mut cas = self.trusted_cas.write();
        cas.push(cert);
    }

    /// Validate client certificate
    pub fn validate_certificate(&self, cert: &[u8]) -> Result<bool> {
        // TODO: Implement proper certificate validation
        Ok(true)
    }
}

impl MfaManager {
    fn new() -> Self {
        Self {
            totp_secrets: Arc::new(RwLock::new(HashMap::new())),
            backup_codes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate TOTP secret for user
    pub fn generate_totp_secret(&self, user_id: String) -> Vec<u8> {
        let mut secret = vec![0u8; 32];
        for i in 0..32 {
            secret[i] = rand::random();
        }

        let mut secrets = self.totp_secrets.write();
        secrets.insert(user_id, secret.clone());

        secret
    }

    /// Verify TOTP code
    pub fn verify_totp(&self, user_id: &str, code: &str) -> bool {
        // TODO: Implement TOTP verification
        false
    }
}

// ============================================================================
// Authorization Engine - RBAC, ABAC, Policy Engine
// ============================================================================

/// Authorization engine
pub struct AuthorizationEngine {
    /// RBAC manager
    rbac: Arc<RbacManager>,
    /// ABAC evaluator
    abac: Arc<AbacEvaluator>,
    /// Policy engine
    policy_engine: Arc<PolicyEngine>,
}

/// RBAC manager
pub struct RbacManager {
    /// Roles
    roles: Arc<RwLock<HashMap<String, Role>>>,
    /// User role assignments
    user_roles: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

/// Role definition
#[derive(Debug, Clone)]
pub struct Role {
    /// Role ID
    pub id: String,
    /// Role name
    pub name: String,
    /// Description
    pub description: String,
    /// Permissions
    pub permissions: Vec<String>,
    /// Parent roles (inheritance)
    pub parent_roles: Vec<String>,
    /// Created at
    pub created_at: SystemTime,
}

/// ABAC evaluator
pub struct AbacEvaluator {
    /// Attribute providers
    attribute_providers: Vec<Arc<dyn AttributeProvider>>,
}

/// Attribute provider trait
pub trait AttributeProvider: Send + Sync {
    /// Get attributes for subject
    fn get_subject_attributes(&self, subject: &str) -> HashMap<String, AttributeValue>;
    /// Get attributes for resource
    fn get_resource_attributes(&self, resource: &str) -> HashMap<String, AttributeValue>;
    /// Get attributes for environment
    fn get_environment_attributes(&self) -> HashMap<String, AttributeValue>;
}

/// Attribute value
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    String(String),
    Number(i64),
    Boolean(bool),
    List(Vec<AttributeValue>),
}

/// Policy engine
pub struct PolicyEngine {
    /// Policies
    policies: Arc<RwLock<HashMap<String, Policy>>>,
}

/// Policy definition
#[derive(Debug, Clone)]
pub struct Policy {
    /// Policy ID
    pub id: String,
    /// Policy name
    pub name: String,
    /// Description
    pub description: String,
    /// Effect
    pub effect: PolicyEffect,
    /// Subjects (who)
    pub subjects: Vec<String>,
    /// Resources (what)
    pub resources: Vec<String>,
    /// Actions (how)
    pub actions: Vec<String>,
    /// Conditions
    pub conditions: Vec<PolicyCondition>,
}

/// Policy effect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// Policy condition
#[derive(Debug, Clone)]
pub struct PolicyCondition {
    /// Attribute path
    pub attribute: String,
    /// Operator
    pub operator: PolicyOperator,
    /// Value
    pub value: AttributeValue,
}

/// Policy operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEquals,
    LessThanOrEquals,
    In,
    NotIn,
    Contains,
    StartsWith,
    EndsWith,
}

impl AuthorizationEngine {
    /// Create new authorization engine
    pub fn new() -> Self {
        Self {
            rbac: Arc::new(RbacManager::new()),
            abac: Arc::new(AbacEvaluator::new()),
            policy_engine: Arc::new(PolicyEngine::new()),
        }
    }

    /// Authorize session for permissions
    pub fn authorize(&self, session: &Session, requiredpermissions: &[String]) -> Result<bool> {
        // Check RBAC first
        if self.rbac.has_permissions(&session.user_id, required_permissions) {
            return Ok(true);
        }

        // Check session permissions
        for perm in required_permissions {
            if !session.permissions.contains(perm) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check if user has permission
    pub fn check_permission(&self, user_id: &str, permission: &str) -> bool {
        self.rbac.has_permission(user_id, permission)
    }

    /// Evaluate policy
    pub fn evaluate_policy(&self, subject: &str, resource: &str, action: &str) -> Result<bool> {
        self.policy_engine.evaluate(subject, resource, action)
    }
}

impl RbacManager {
    fn new() -> Self {
        Self {
            roles: Arc::new(RwLock::new(HashMap::new())),
            user_roles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create role
    pub fn create_role(&self, role: Role) {
        let mut roles = self.roles.write();
        roles.insert(role.id.clone(), role);
    }

    /// Assign role to user
    pub fn assign_role(&self, user_id: String, role_id: String) {
        let mut user_roles = self.user_roles.write();
        user_roles.entry(user_id).or_insert_with(Vec::new).push(role_id);
    }

    /// Remove role from user
    pub fn remove_role(&self, user_id: &str, role_id: &str) -> bool {
        let mut user_roles = self.user_roles.write();
        if let Some(roles) = user_roles.get_mut(user_id) {
            if let Some(pos) = roles.iter().position(|r| r == role_id) {
                roles.remove(pos);
                return true;
            }
        }
        false
    }

    /// Get user roles
    pub fn get_user_roles(&self, userid: &str) -> Vec<Role> {
        let user_roles = self.user_roles.read();
        let roles = self.roles.read();

        user_roles.get(user_id)
            .map(|role_ids| {
                role_ids.iter()
                    .filter_map(|id| roles.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all permissions for user (including inherited)
    pub fn get_user_permissions(&self, user_id: &str) -> HashSet<String> {
        let mut permissions = HashSet::new();
        let user_roles = self.get_user_roles(user_id);

        for role in user_roles {
            for perm in &role.permissions {
                permissions.insert(perm.clone());
            }
        }

        permissions
    }

    /// Check if user has permission
    pub fn has_permission(&self, user_id: &str, permission: &str) -> bool {
        let permissions = self.get_user_permissions(user_id);
        permissions.contains(permission)
    }

    /// Check if user has all permissions
    pub fn has_permissions(&self, user_id: &str, required: &[String]) -> bool {
        let permissions = self.get_user_permissions(user_id);
        required.iter().all(|p| permissions.contains(p))
    }
}

impl AbacEvaluator {
    fn new() -> Self {
        Self {
            attribute_providers: Vec::new(),
        }
    }

    /// Add attribute provider
    pub fn add_provider(&mut self, provider: Arc<dyn AttributeProvider>) {
        self.attribute_providers.push(provider);
    }

    /// Evaluate ABAC policy
    pub fn evaluate(&self, subject: &str, resource: &str, action: &str, conditions: &[PolicyCondition]) -> bool {
        // Collect attributes
        let mut subject_attrs = HashMap::new();
        let mut resource_attrs = HashMap::new();
        let mut env_attrs = HashMap::new();

        for provider in &self.attribute_providers {
            subject_attrs.extend(provider.get_subject_attributes(subject));
            resource_attrs.extend(provider.get_resource_attributes(resource));
            env_attrs.extend(provider.get_environment_attributes());
        }

        // Evaluate conditions
        for condition in conditions {
            let attr_value = if condition.attribute.starts_with("subject.") {
                subject_attrs.get(&condition.attribute[8..])
            } else if condition.attribute.starts_with("resource.") {
                resource_attrs.get(&condition.attribute[9..])
            } else if condition.attribute.starts_with("environment.") {
                env_attrs.get(&condition.attribute[12..])
            } else {
                None
            };

            if let Some(value) = attr_value {
                if !self.evaluate_condition(value, &condition.operator, &condition.value) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Evaluate single condition
    fn evaluate_condition(&self, actual: &AttributeValue, operator: &PolicyOperator, expected: &AttributeValue) -> bool {
        match operator {
            PolicyOperator::Equals => actual == expected,
            PolicyOperator::NotEquals => actual != expected,
            _ => false, // TODO: Implement other operators
        }
    }
}

impl PolicyEngine {
    fn new() -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add policy
    pub fn add_policy(&self, policy: Policy) {
        let mut policies = self.policies.write();
        policies.insert(policy.id.clone(), policy);
    }

    /// Remove policy
    pub fn remove_policy(&self, policy_id: &str) -> bool {
        let mut policies = self.policies.write();
        policies.remove(policy_id).is_some()
    }

    /// Evaluate policies
    pub fn evaluate(&self, subject: &str, resource: &str, action: &str) -> Result<bool> {
        let policies = self.policies.read();

        let mut allow = false;
        let mut deny = false;

        for policy in policies.values() {
            // Check if policy applies
            if !self.matches_pattern(&policy.subjects, subject) {
                continue;
            }
            if !self.matches_pattern(&policy.resources, resource) {
                continue;
            }
            if !self.matches_pattern(&policy.actions, action) {
                continue;
            }

            // TODO: Evaluate conditions

            match policy.effect {
                PolicyEffect::Allow => allow = true,
                PolicyEffect::Deny => deny = true,
            }
        }

        // Deny takes precedence
        if deny {
            Ok(false)
        } else {
            Ok(allow)
        }
    }

    /// Match pattern (supports wildcards)
    fn matches_pattern(&self, patterns: &[String], value: &str) -> bool {
        if patterns.is_empty() {
            return true;
        }

        for pattern in patterns {
            if pattern == "*" || pattern == value {
                return true;
            }
            if pattern.contains('*') {
                // Simple wildcard matching
                let parts: Vec<&str> = pattern.split('*').collect();
                if parts.len() == 2 {
                    if value.starts_with(parts[0]) && value.ends_with(parts[1]) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Get all policies
    pub fn get_policies(&self) -> Vec<Policy> {
        let policies = self.policies.read();
        policies.values().cloned().collect()
    }
}

// ============================================================================
// Rate Limiting & Throttling - Token Bucket, Sliding Window
// ============================================================================

/// Rate limiter
pub struct RateLimiter {
    /// Rate limit configurations
    configs: Arc<RwLock<HashMap<String, RateLimitConfig>>>,
    /// Token buckets
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    /// Sliding windows
    windows: Arc<RwLock<HashMap<String, SlidingWindow>>>,
    /// Quota manager
    quota_manager: Arc<QuotaManager>,
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Limit type
    pub limit_type: RateLimitType,
    /// Requests per window
    pub requests: u64,
    /// Window duration (seconds)
    pub window: u64,
    /// Burst size
    pub burst: Option<u64>,
}

/// Rate limit type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitType {
    /// Token bucket algorithm
    TokenBucket,
    /// Sliding window
    SlidingWindow,
    /// Fixed window
    FixedWindow,
}

/// Token bucket
pub struct TokenBucket {
    /// Capacity
    capacity: u64,
    /// Current tokens
    tokens: f64,
    /// Refill rate (tokens per second)
    refill_rate: f64,
    /// Last refill time
    last_refill: Instant,
}

/// Sliding window
pub struct SlidingWindow {
    /// Window size (seconds)
    window_size: u64,
    /// Request timestamps
    requests: VecDeque<Instant>,
    /// Max requests
    max_requests: u64,
}

/// Quota manager
pub struct QuotaManager {
    /// User quotas
    quotas: Arc<RwLock<HashMap<String, UserQuota>>>,
}

/// User quota
#[derive(Debug, Clone)]
pub struct UserQuota {
    /// User ID
    pub user_id: String,
    /// Daily limit
    pub daily_limit: u64,
    /// Monthly limit
    pub monthly_limit: u64,
    /// Current daily usage
    pub daily_usage: u64,
    /// Current monthly usage
    pub monthly_usage: u64,
    /// Reset timestamp
    pub daily_reset: SystemTime,
    /// Monthly reset timestamp
    pub monthly_reset: SystemTime,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new() -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            buckets: Arc::new(RwLock::new(HashMap::new())),
            windows: Arc::new(RwLock::new(HashMap::new())),
            quota_manager: Arc::new(QuotaManager::new()),
        }
    }

    /// Configure rate limit
    pub fn configure(&self, key: String, config: RateLimitConfig) {
        let mut configs = self.configs.write();
        configs.insert(key, config);
    }

    /// Check rate limit
    pub fn check_rate_limit(&self, key: &str, overrideconfig: Option<&RateLimitConfig>) -> Result<()> {
        let configs = self.configs.read();
        let config = override_config.or_else(|| configs.get(key));

        let config = match config {
            Some(c) => c,
            None => return Ok(()), // No limit configured
        };

        match config.limit_type {
            RateLimitType::TokenBucket => {
                self.check_token_bucket(key, config)
            },
            RateLimitType::SlidingWindow => {
                self.check_sliding_window(key, config)
            },
            RateLimitType::FixedWindow => {
                self.check_fixed_window(key, config)
            },
        }
    }

    /// Check token bucket
    fn check_token_bucket(&self, key: &str, config: &RateLimitConfig) -> Result<()> {
        let mut buckets = self.buckets.write();

        let bucket = buckets.entry(key.to_string()).or_insert_with(|| {
            TokenBucket::new(
                config.burst.unwrap_or(config.requests),
                config.requests as f64 / config.window as f64,
            )
        });

        bucket.consume(1)
    }

    /// Check sliding window
    fn check_sliding_window(&self, key: &str, config: &RateLimitConfig) -> Result<()> {
        let mut windows = self.windows.write();

        let window = windows.entry(key.to_string()).or_insert_with(|| {
            SlidingWindow::new(config.window, config.requests)
        });

        window.allow_request()
    }

    /// Check fixed window
    fn check_fixed_window(&self, key: &str, config: &RateLimitConfig) -> Result<()> {
        // Simplified implementation using sliding window
        self.check_sliding_window(key, config)
    }

    /// Get rate limit status
    pub fn get_status(&self, key: &str) -> Option<RateLimitStatus> {
        let buckets = self.buckets.read();
        if let Some(bucket) = buckets.get(key) {
            return Some(RateLimitStatus {
                remaining: bucket.tokens as u64,
                reset_at: Instant::now() + Duration::from_secs(1),
            });
        }

        let windows = self.windows.read();
        if let Some(window) = windows.get(key) {
            return Some(RateLimitStatus {
                remaining: window.max_requests.saturating_sub(window.requests.len() as u64),
                reset_at: Instant::now() + Duration::from_secs(window.window_size),
            });
        }

        None
    }
}

/// Rate limit status
#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    /// Remaining requests
    pub remaining: u64,
    /// Reset time
    pub reset_at: Instant,
}

impl TokenBucket {
    /// Create new token bucket
    fn new(capacity: u64, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        let new_tokens = elapsed * self.refill_rate;
        self.tokens = (self.tokens + new_tokens).min(self.capacity as f64);
        self.last_refill = now;
    }

    /// Consume tokens
    fn consume(&mut self, amount: u64) -> Result<()> {
        self.refill();

        if self.tokens >= amount as f64 {
            self.tokens -= amount as f64;
            Ok(())
        } else {
            Err(DbError::InvalidOperation("Rate limit exceeded".to_string()))
        }
    }
}

impl SlidingWindow {
    /// Create new sliding window
    fn new(window_size: u64, maxrequests: u64) -> Self {
        Self {
            window_size,
            requests: VecDeque::new(),
            max_requests,
        }
    }

    /// Clean old requests
    fn clean_old_requests(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(self.window_size);

        while let Some(&oldest) = self.requests.front() {
            if oldest < cutoff {
                self.requests.pop_front();
            } else {
                break;
            }
        }
    }

    /// Allow request
    fn allow_request(&mut self) -> Result<()> {
        self.clean_old_requests();

        if self.requests.len() < self.max_requests as usize {
            self.requests.push_back(Instant::now());
            Ok(())
        } else {
            Err(DbError::InvalidOperation("Rate limit exceeded".to_string()))
        }
    }
}

impl QuotaManager {
    fn new() -> Self {
        Self {
            quotas: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set user quota
    pub fn set_quota(&self, user_id: String, daily_limit: u64, monthly_limit: u64) {
        let mut quotas = self.quotas.write();

        let now = SystemTime::now();
        quotas.insert(user_id.clone(), UserQuota {
            user_id,
            daily_limit,
            monthly_limit,
            daily_usage: 0,
            monthly_usage: 0,
            daily_reset: now + Duration::from_secs(86400),
            monthly_reset: now + Duration::from_secs(30 * 86400),
        });
    }

    /// Check and update quota
    pub fn check_quota(&self, user_id: &str) -> Result<()> {
        let mut quotas = self.quotas.write();

        let quota = match quotas.get_mut(user_id) {
            Some(q) => q,
            None => return Ok(()), // No quota set
        };

        let now = SystemTime::now();

        // Reset daily quota if needed
        if now >= quota.daily_reset {
            quota.daily_usage = 0;
            quota.daily_reset = now + Duration::from_secs(86400);
        }

        // Reset monthly quota if needed
        if now >= quota.monthly_reset {
            quota.monthly_usage = 0;
            quota.monthly_reset = now + Duration::from_secs(30 * 86400);
        }

        // Check limits
        if quota.daily_usage >= quota.daily_limit {
            return Err(DbError::InvalidOperation("Daily quota exceeded".to_string()));
        }
        if quota.monthly_usage >= quota.monthly_limit {
            return Err(DbError::InvalidOperation("Monthly quota exceeded".to_string()));
        }

        // Update usage
        quota.daily_usage += 1;
        quota.monthly_usage += 1;

        Ok(())
    }

    /// Get quota status
    pub fn get_quota_status(&self, user_id: &str) -> Option<UserQuota> {
        let quotas = self.quotas.read();
        quotas.get(user_id).cloned()
    }
}

// ============================================================================
// Security Features - Request Validation, Threat Detection
// ============================================================================

/// Security filter
pub struct SecurityFilter {
    /// Request validator
    validator: Arc<RequestValidator>,
    /// Threat detector
    threat_detector: Arc<ThreatDetector>,
    /// IP filter
    ip_filter: Arc<IpFilter>,
    /// CSRF manager
    csrf_manager: Arc<CsrfManager>,
}

/// Request validator
pub struct RequestValidator {
    /// Maximum path length
    max_path_length: usize,
    /// Maximum header size
    max_header_size: usize,
    /// Allowed content types
    allowed_content_types: HashSet<String>,
}

/// Threat detector
pub struct ThreatDetector {
    /// SQL injection patterns
    sql_injection_patterns: Vec<regex::Regex>,
    /// XSS patterns
    xss_patterns: Vec<regex::Regex>,
    /// Path traversal patterns
    path_traversal_patterns: Vec<regex::Regex>,
    /// Suspicious patterns
    suspicious_patterns: Vec<regex::Regex>,
}

/// IP filter
pub struct IpFilter {
    /// Whitelist
    whitelist: Arc<RwLock<HashSet<IpAddr>>>,
    /// Blacklist
    blacklist: Arc<RwLock<HashSet<IpAddr>>>,
    /// Mode
    mode: IpFilterMode,
}

/// IP filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpFilterMode {
    /// Allow all except blacklisted
    Blacklist,
    /// Deny all except whitelisted
    Whitelist,
    /// No filtering
    None,
}

/// CSRF manager
pub struct CsrfManager {
    /// CSRF tokens
    tokens: Arc<RwLock<HashMap<String, CsrfToken>>>,
    /// Token timeout (seconds)
    token_timeout: u64,
}

/// CSRF token
#[derive(Debug, Clone)]
pub struct CsrfToken {
    /// Token value
    pub value: String,
    /// Created at
    pub created_at: Instant,
    /// User session ID
    pub session_id: String,
}

impl SecurityFilter {
    /// Create new security filter
    pub fn new() -> Self {
        Self {
            validator: Arc::new(RequestValidator::new()),
            threat_detector: Arc::new(ThreatDetector::new()),
            ip_filter: Arc::new(IpFilter::new(IpFilterMode::None)),
            csrf_manager: Arc::new(CsrfManager::new()),
        }
    }

    /// Validate request
    pub fn validate_request(&self, request: &ApiRequest) -> Result<()> {
        // IP filtering
        self.ip_filter.check_ip(request.client_ip)?;

        // Request validation
        self.validator.validate(request)?;

        // Threat detection
        self.threat_detector.detect_threats(request)?;

        Ok(())
    }

    /// Get IP filter
    pub fn get_ip_filter(&self) -> Arc<IpFilter> {
        Arc::clone(&self.ip_filter)
    }

    /// Get CSRF manager
    pub fn get_csrf_manager(&self) -> Arc<CsrfManager> {
        Arc::clone(&self.csrf_manager)
    }
}

impl RequestValidator {
    fn new() -> Self {
        let mut allowed_content_types = HashSet::new();
        allowed_content_types.insert("application/json".to_string());
        allowed_content_types.insert("application/x-www-form-urlencoded".to_string());
        allowed_content_types.insert("multipart/form-data".to_string());
        allowed_content_types.insert("text/plain".to_string());

        Self {
            max_path_length: 2048,
            max_header_size: 8192,
            allowed_content_types,
        }
    }

    /// Validate request
    fn validate(&self, request: &ApiRequest) -> Result<()> {
        // Check path length
        if request.path.len() > self.max_path_length {
            return Err(DbError::InvalidOperation("Path too long".to_string()));
        }

        // Check header sizes
        for (key, value) in &request.headers {
            if key.len() + value.len() > self.max_header_size {
                return Err(DbError::InvalidOperation("Header too large".to_string()));
            }
        }

        // Validate content type
        if let Some(content_type) = request.headers.get("Content-Type") {
            let ct = content_type.split(';').next().unwrap_or("");
            if !self.allowed_content_types.contains(ct) {
                return Err(DbError::InvalidOperation("Invalid content type".to_string()));
            }
        }

        Ok(())
    }
}

impl ThreatDetector {
    fn new() -> Self {
        let sql_injection_patterns = vec![
            regex::Regex::new(r"(?i)(union|select|insert|update|delete|drop|create|alter)\s").unwrap(),
            regex::Regex::new(r"(?i)(--|\|{2}|/\*|\*/)").unwrap(),
            regex::Regex::new(r"(?i)(xp_|sp_)").unwrap(),
        ];

        let xss_patterns = vec![
            regex::Regex::new(r"(?i)<script").unwrap(),
            regex::Regex::new(r"(?i)javascript:").unwrap(),
            regex::Regex::new(r"(?i)onerror\s*=").unwrap(),
            regex::Regex::new(r"(?i)onload\s*=").unwrap(),
        ];

        let path_traversal_patterns = vec![
            regex::Regex::new(r"\.\./").unwrap(),
            regex::Regex::new(r"\.\.\\").unwrap(),
        ];

        let suspicious_patterns = vec![
            regex::Regex::new(r"(?i)(cmd|exec|eval|system)").unwrap(),
        ];

        Self {
            sql_injection_patterns,
            xss_patterns,
            path_traversal_patterns,
            suspicious_patterns,
        }
    }

    /// Detect threats in request
    fn detect_threats(&self, request: &ApiRequest) -> Result<()> {
        // Check path
        self.check_path_traversal(&request.path)?;

        // Check query parameters
        for (_, value) in &request.query_params {
            self.check_sql_injection(value)?;
            self.check_xss(value)?;
        }

        // Check headers
        for (_, value) in &request.headers {
            self.check_xss(value)?;
        }

        // Check body (if text)
        if let Ok(body_str) = std::str::from_utf8(&request.body) {
            self.check_sql_injection(body_str)?;
            self.check_xss(body_str)?;
        }

        Ok(())
    }

    /// Check for SQL injection
    fn check_sql_injection(&self, input: &str) -> Result<()> {
        for pattern in &self.sql_injection_patterns {
            if pattern.is_match(input) {
                return Err(DbError::InvalidOperation("Potential SQL injection detected".to_string()));
            }
        }
        Ok(())
    }

    /// Check for XSS
    fn check_xss(&self, input: &str) -> Result<()> {
        for pattern in &self.xss_patterns {
            if pattern.is_match(input) {
                return Err(DbError::InvalidOperation("Potential XSS attack detected".to_string()));
            }
        }
        Ok(())
    }

    /// Check for path traversal
    fn check_path_traversal(&self, input: &str) -> Result<()> {
        for pattern in &self.path_traversal_patterns {
            if pattern.is_match(input) {
                return Err(DbError::InvalidOperation("Path traversal attempt detected".to_string()));
            }
        }
        Ok(())
    }
}

impl IpFilter {
    fn new(mode: IpFilterMode) -> Self {
        Self {
            whitelist: Arc::new(RwLock::new(HashSet::new())),
            blacklist: Arc::new(RwLock::new(HashSet::new())),
            mode,
        }
    }

    /// Check IP address
    fn check_ip(&self, ip: IpAddr) -> Result<()> {
        match self.mode {
            IpFilterMode::None => Ok(()),
            IpFilterMode::Blacklist => {
                let mut blacklist = self.blacklist.read();
                if blacklist.contains(&ip) {
                    Err(DbError::InvalidOperation("IP address blacklisted".to_string()))
                } else {
                    Ok(())
                }
            },
            IpFilterMode::Whitelist => {
                let mut whitelist = self.whitelist.read();
                if whitelist.contains(&ip) {
                    Ok(())
                } else {
                    Err(DbError::InvalidOperation("IP address not whitelisted".to_string()))
                }
            },
        }
    }

    /// Add to whitelist
    pub fn add_to_whitelist(&self, ip: IpAddr) {
        let mut whitelist = self.whitelist.write();
        whitelist.insert(ip);
    }

    /// Remove from whitelist
    pub fn remove_from_whitelist(&self, ip: IpAddr) -> bool {
        let mut whitelist = self.whitelist.write();
        whitelist.remove(&ip)
    }

    /// Add to blacklist
    pub fn add_to_blacklist(&self, ip: IpAddr) {
        let mut blacklist = self.blacklist.write();
        blacklist.insert(ip);
    }

    /// Remove from blacklist
    pub fn remove_from_blacklist(&self, ip: IpAddr) -> bool {
        let mut blacklist = self.blacklist.write();
        blacklist.remove(&ip)
    }

    /// Set filter mode
    pub fn set_mode(&mut self, mode: IpFilterMode) {
        self.mode = mode;
    }
}

impl CsrfManager {
    fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            token_timeout: 3600,
        }
    }

    /// Generate CSRF token
    pub fn generate_token(&self, session_id: String) -> String {
        let token_value = Uuid::new_v4().to_string();

        let token = CsrfToken {
            value: token_value.clone(),
            created_at: Instant::now(),
            session_id,
        };

        let mut tokens = self.tokens.write();
        tokens.insert(token_value.clone(), token);

        token_value
    }

    /// Validate CSRF token
    pub fn validate_token(&self, token_value: &str, session_id: &str) -> bool {
        let tokens = self.tokens.read();

        if let Some(token) = tokens.get(token_value) {
            // Check session ID
            if token.session_id != session_id {
                return false;
            }

            // Check expiration
            let age = Instant::now().duration_since(token.created_at);
            if age.as_secs() > self.token_timeout {
                return false;
            }

            true
        } else {
            false
        }
    }

    /// Cleanup expired tokens
    pub fn cleanup_expired_tokens(&self) {
        let mut tokens = self.tokens.write();
        let timeout = Duration::from_secs(self.token_timeout);

        tokens.retain(|_, token| {
            Instant::now().duration_since(token.created_at) < timeout
        });
    }
}

// ============================================================================
// Audit Logging
// ============================================================================

/// Audit logger
pub struct AuditLogger {
    /// Audit events
    events: VecDeque<AuditEvent>,
    /// Max events to keep in memory
    max_events: usize,
}

/// Audit event
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Event ID
    pub id: String,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Event type
    pub event_type: AuditEventType,
    /// User ID
    pub user_id: Option<String>,
    /// Client IP
    pub client_ip: IpAddr,
    /// Request ID
    pub request_id: String,
    /// Resource
    pub resource: String,
    /// Action
    pub action: String,
    /// Result
    pub result: AuditResult,
    /// Details
    pub details: HashMap<String, String>,
}

/// Audit event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEventType {
    Authentication,
    Authorization,
    DataAccess,
    DataModification,
    AdminAction,
    SecurityEvent,
    ConfigChange,
}

/// Audit result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditResult {
    Success,
    Failure,
    Denied,
}

/// Security event
#[derive(Debug, Clone)]
pub struct SecurityEvent {
    /// Event type
    pub event_type: SecurityEventType,
    /// Request ID
    pub request_id: String,
    /// Client IP
    pub client_ip: IpAddr,
    /// Reason
    pub reason: String,
    /// Timestamp
    pub timestamp: SystemTime,
}

/// Security event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityEventType {
    AuthenticationFailed,
    AuthorizationFailed,
    RequestBlocked,
    RateLimitExceeded,
    SuspiciousActivity,
}

impl AuditLogger {
    fn new() -> Self {
        Self {
            events: VecDeque::new(),
            max_events: 10000,
        }
    }

    /// Log request
    fn log_request(&mut self, request: &ApiRequest) {
        let event = AuditEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            event_type: AuditEventType::DataAccess,
            user_id: None,
            client_ip: request.client_ip,
            request_id: request.request_id.clone(),
            resource: request.path.clone(),
            action: format!("{:?}", request.method),
            result: AuditResult::Success,
            details: HashMap::new(),
        };

        self.add_event(event);
    }

    /// Log security event
    fn log_security_event(&mut self, event: &SecurityEvent) {
        let audit_event = AuditEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: event.timestamp,
            event_type: AuditEventType::SecurityEvent,
            user_id: None,
            client_ip: event.client_ip,
            request_id: event.request_id.clone(),
            resource: String::new(),
            action: format!("{:?}", event.event_type),
            result: AuditResult::Denied,
            details: {
                let mut details = HashMap::new();
                details.insert("reason".to_string(), event.reason.clone());
                details
            },
        };

        self.add_event(audit_event);
    }

    /// Add event
    fn add_event(&mut self, event: AuditEvent) {
        if self.events.len() >= self.max_events {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Get recent events
    pub fn get_recent_events(&self, count: usize) -> Vec<AuditEvent> {
        self.events.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Search events
    pub fn search_events(&self, filter: &AuditEventFilter) -> Vec<AuditEvent> {
        self.events.iter()
            .filter(|e| filter.matches(e))
            .cloned()
            .collect()
    }
}

/// Audit event filter
#[derive(Debug, Default)]
pub struct AuditEventFilter {
    /// Filter by event type
    pub event_type: Option<AuditEventType>,
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Filter by client IP
    pub client_ip: Option<IpAddr>,
    /// Filter by time range
    pub time_range: Option<(SystemTime)>,
    /// Filter by result
    pub result: Option<AuditResult>,
}

impl AuditEventFilter {
    fn matches(&self, event: &AuditEvent) -> bool {
        if let Some(event_type) = self.event_type {
            if event.event_type != event_type {
                return false;
            }
        }

        if let Some(ref user_id) = self.user_id {
            if event.user_id.as_ref() != Some(user_id) {
                return false;
            }
        }

        if let Some(client_ip) = self.client_ip {
            if event.client_ip != client_ip {
                return false;
            }
        }

        if let Some((start, end)) = self.time_range {
            if event.timestamp < start || event.timestamp > end {
                return false;
            }
        }

        if let Some(result) = self.result {
            if event.result != result {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::UNIX_EPOCH;

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10, 1.0);

        // Should allow 10 requests initially
        for _ in 0..10 {
            assert!(bucket.consume(1).is_ok());
        }

        // 11th request should fail
        assert!(bucket.consume(1).is_err());
    }

    #[test]
    fn test_sliding_window() {
        let mut window = SlidingWindow::new(60, 10);

        // Should allow 10 requests
        for _ in 0..10 {
            assert!(window.allow_request().is_ok());
        }

        // 11th request should fail
        assert!(window.allow_request().is_err());
    }

    #[test]
    fn test_threat_detection() {
        let detector = ThreatDetector::new();

        // Should detect SQL injection
        assert!(detector.check_sql_injection("' OR '1'='1").is_err());
        assert!(detector.check_sql_injection("UNION SELECT * FROM users").is_err());

        // Should detect XSS
        assert!(detector.check_xss("<script>alert('xss')</script>").is_err());
        assert!(detector.check_xss("javascript:alert(1)").is_err());

        // Should allow safe input
        assert!(detector.check_sql_injection("normal text").is_ok());
        assert!(detector.check_xss("normal text").is_ok());
    }

    #[test]
    fn test_ip_filter() {
        let filter = IpFilter::new(IpFilterMode::Blacklist);
        let test_ip = "192.168.1.1".parse().unwrap();

        // Should allow initially
        assert!(filter.check_ip(test_ip).is_ok());

        // Add to blacklist
        filter.add_to_blacklist(test_ip);

        // Should block now
        assert!(filter.check_ip(test_ip).is_err());
    }

    #[test]
    fn test_rbac() {
        let rbac = RbacManager::new();

        // Create role
        let role = Role {
            id: "admin".to_string(),
            name: "Administrator".to_string(),
            description: "Full access".to_string(),
            permissions: vec!["read".to_string(), "write".to_string()],
            parent_roles: Vec::new(),
            created_at: SystemTime::now(),
        };

        rbac.create_role(role);

        // Assign to user
        rbac.assign_role("user1".to_string(), "admin".to_string());

        // Check permissions
        assert!(rbac.has_permission("user1", "read"));
        assert!(rbac.has_permission("user1", "write"));
        assert!(!rbac.has_permission("user1", "delete"));
    }
}
