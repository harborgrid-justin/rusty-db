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
use std::time::{Duration, UNIX_EPOCH};

use parking_lot::{RwLock};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use hmac::Hmac;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use uuid::Uuid;
use crate::api::gateway::{AuthorizationEngine, SecurityFilter, AuthenticationManager, AuditLogger};
use crate::api::{RateLimitConfig, RateLimiter};
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
    pub(crate) config: Arc<RwLock<GatewayConfig>>,
    /// Route registry
    pub(crate) routes: Arc<RwLock<HashMap<String, Route>>>,
    /// Authentication manager
    pub(crate) auth_manager: Arc<AuthenticationManager>,
    /// Authorization engine
    pub(crate) authz_engine: Arc<AuthorizationEngine>,
    /// Rate limiter
    pub(crate) rate_limiter: Arc<RateLimiter>,
    /// Security filter
    pub(crate) security_filter: Arc<SecurityFilter>,
    /// Service registry
    pub(crate) service_registry: Arc<RwLock<ServiceRegistry>>,
    /// Request metrics
    pub(crate) metrics: Arc<RwLock<GatewayMetrics>>,
    /// Audit logger
    pub(crate) audit_logger: Arc<Mutex<AuditLogger>>,
}

/// Service registry
pub struct ServiceRegistry {
    /// Registered services
    pub(crate) services: HashMap<String, BackendService>,
    /// Service health status
    pub(crate) health_status: HashMap<String, bool>,
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self {
            services: HashMap::new(),
            health_status: HashMap::new(),
        }
    }
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

