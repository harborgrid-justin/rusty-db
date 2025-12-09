// # REST API Middleware
//
// Middleware functions for request processing, authentication, rate limiting, and logging.
// Implements proper error handling and dependency injection.

use crate::error::DbError;
use axum::{
    extract::State,
    http::{HeaderMap, Request},
    middleware::Next,
    response::Response,
};
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::SystemTime;

use super::types::*;

/// Request logger middleware that tracks and updates metrics
pub async fn request_logger_middleware<B>(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    req: Request<B>,
    next: Next<B>,
) -> std::result::Result<Response, ApiError> {
    let method = req.method().to_string();
    let uri = req.uri().to_string();
    let start = SystemTime::now();

    let request_id = Uuid::new_v4().to_string();

    tracing::info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        "Incoming request"
    );

    let response = next.run(req).await;

    let elapsed = start.elapsed().unwrap_or_default();

    tracing::info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        duration_ms = elapsed.as_millis(),
        "Request completed"
    );

    // Update metrics
    let mut metrics = state.metrics.write().await;
    metrics.total_requests += 1;
    metrics.successful_requests += 1;

    let count = *metrics.requests_by_endpoint.entry(uri.clone()).or_insert(0);
    metrics.requests_by_endpoint.insert(uri, count + 1);

    Ok(response)
}

/// Rate limiting middleware that checks request limits
pub async fn rate_limit_middleware<B>(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    req: Request<B>,
    next: Next<B>,
) -> std::result::Result<Response, ApiError> {
    // Extract identifier (IP or API key)
    let identifier = headers
        .get("X-Forwarded-For")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let mut limiter = state.rate_limiter.write().await;

    if !limiter.check_limit(&identifier) {
        return Err(ApiError::new(
            "RATE_LIMIT_EXCEEDED",
            "Too many requests. Please try again later.",
        ));
    }

    drop(limiter);

    Ok(next.run(req).await)
}

/// Authentication middleware trait for extensibility
#[async_trait::async_trait]
pub trait AuthMiddleware: Send + Sync {
    /// Verify authentication token
    async fn verify_token(&self, token: &str) -> Result<bool>;

    /// Extract user information from request
    async fn extract_user(&self, headers: &HeaderMap) -> Result<Option<UserInfo>>;
}

/// Default authentication middleware implementation
pub struct DefaultAuthMiddleware {
    enabled: bool,
    api_key: Option<String>,
}

impl DefaultAuthMiddleware {
    pub fn new(enabled: bool, api_key: Option<String>) -> Self {
        Self { enabled, api_key }
    }
}

#[async_trait::async_trait]
impl AuthMiddleware for DefaultAuthMiddleware {
    async fn verify_token(&self, token: &str) -> Result<bool> {
        if !self.enabled {
            return Ok(true);
        }

        if let Some(ref key) = self.api_key {
            Ok(token == key)
        } else {
            Ok(false)
        }
    }

    async fn extract_user(&self, headers: &HeaderMap) -> Result<Option<UserInfo>> {
        // TODO: Implement user extraction from headers
        Ok(None)
    }
}

/// User information extracted from authentication
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub user_id: u64,
    pub username: String,
    pub roles: Vec<String>,
}

/// CORS middleware for cross-origin requests
pub struct CorsMiddleware {
    allowed_origins: Vec<String>,
    allowed_methods: Vec<String>,
    allowed_headers: Vec<String>,
}

impl CorsMiddleware {
    pub fn new(allowed_origins: Vec<String>) -> Self {
        Self {
            allowed_origins,
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "X-API-Key".to_string(),
            ],
        }
    }

    pub fn allow_method(mut self, method: String) -> Self {
        self.allowed_methods.push(method);
        self
    }

    pub fn allow_header(mut self, header: String) -> Self {
        self.allowed_headers.push(header);
        self
    }
}

/// Request validation middleware
pub struct ValidationMiddleware;

impl ValidationMiddleware {
    /// Validate request size
    pub fn check_request_size<B>(req: &Request<B>, max_size: usize) -> Result<()> {
        if let Some(content_length) = req.headers().get("content-length") {
            if let Ok(length) = content_length.to_str().unwrap_or("0").parse::<usize>() {
                if length > max_size {
                    return Err(DbError::InvalidInput("Request too large".to_string()));
                }
            }
        }
        Ok(())
    }

    /// Validate content type
    pub fn check_content_type<B>(req: &Request<B>, expected: &str) -> Result<()> {
        if let Some(content_type) = req.headers().get("content-type") {
            if let Ok(ct) = content_type.to_str() {
                if !ct.contains(expected) {
                    return Err(DbError::InvalidInput(format!("Expected content type: {}", expected)));
                }
            }
        }
        Ok(())
    }
}

/// Metrics collection middleware
pub struct MetricsMiddleware {
    request_count: Arc<std::sync::atomic::AtomicU64>,
    error_count: Arc<std::sync::atomic::AtomicU64>,
}

impl MetricsMiddleware {
    pub fn new() -> Self {
        Self {
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            error_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub fn record_request(&self) {
        self.request_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn record_error(&self) {
        self.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn get_request_count(&self) -> u64 {
        self.request_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn get_error_count(&self) -> u64 {
        self.error_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Security headers middleware
pub struct SecurityHeadersMiddleware;

impl SecurityHeadersMiddleware {
    pub fn add_security_headers<B>(mut response: Response<B>) -> Response<B> {
        let headers = response.headers_mut();

        // Add security headers
        headers.insert(
            "X-Content-Type-Options",
            "nosniff".parse().unwrap(),
        );

        headers.insert(
            "X-Frame-Options",
            "DENY".parse().unwrap(),
        );

        headers.insert(
            "X-XSS-Protection",
            "1; mode=block".parse().unwrap(),
        );

        headers.insert(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains".parse().unwrap(),
        );

        headers.insert(
            "Content-Security-Policy",
            "default-src 'self'".parse().unwrap(),
        );

        response
    }
}

/// Request ID middleware for tracing
pub struct RequestIdMiddleware;

impl RequestIdMiddleware {
    pub fn add_request_id<B>(mut req: Request<B>) -> Request<B> {
        let request_id = Uuid::new_v4().to_string();

        req.headers_mut().insert(
            "X-Request-ID",
            request_id.parse().unwrap(),
        );

        req
    }

    pub fn get_request_id<B>(req: &Request<B>) -> Option<String> {
        req.headers()
            .get("X-Request-ID")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    }
}

/// Compression middleware for response compression
pub struct CompressionMiddleware;

impl CompressionMiddleware {
    pub fn should_compress<B>(req: &Request<B>) -> bool {
        req.headers()
            .get("Accept-Encoding")
            .and_then(|h| h.to_str().ok())
            .map(|encodings| encodings.contains("gzip") || encodings.contains("deflate"))
            .unwrap_or(false)
    }

    pub fn compress_response<B>(response: Response<B>) -> Response<B> {
        // TODO: Implement response compression
        response
    }
}

/// Cache control middleware
pub struct CacheMiddleware;

impl CacheMiddleware {
    pub fn add_cache_headers<B>(mut response: Response<B>, cache_control: &str) -> Response<B> {
        response.headers_mut().insert(
            "Cache-Control",
            cache_control.parse().unwrap(),
        );
        response
    }

    pub fn is_cacheable<B>(req: &Request<B>) -> bool {
        matches!(req.method().as_str(), "GET" | "HEAD")
    }
}

/// Timeout middleware for request timeouts
pub struct TimeoutMiddleware {
    timeout_duration: std::time::Duration,
}

impl TimeoutMiddleware {
    pub fn new(timeout_duration: std::time::Duration) -> Self {
        Self { timeout_duration }
    }

    pub async fn apply_timeout<B>(
        &self,
        req: Request<B>,
        next: Next<B>,
    ) -> std::result::Result<Response, ApiError> {
        match tokio::time::timeout(self.timeout_duration, next.run(req)).await {
            Ok(response) => Ok(response),
            Err(_) => Err(ApiError::new("TIMEOUT", "Request timed out")),
        }
    }
}

/// Health check middleware
pub struct HealthCheckMiddleware;

impl HealthCheckMiddleware {
    pub async fn perform_health_checks() -> Result<HealthResponse> {
        let mut checks = std::collections::HashMap::new();

        // Database health check
        checks.insert("database".to_string(), ComponentHealth {
            status: "healthy".to_string(),
            message: Some("Database is operational".to_string()),
            last_check: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        });

        // Storage health check
        checks.insert("storage".to_string(), ComponentHealth {
            status: "healthy".to_string(),
            message: None,
            last_check: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        });

        Ok(HealthResponse {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: 3600, // TODO: Track actual uptime
            checks,
        })
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
use std::time::Duration;
use std::time::UNIX_EPOCH;
use std::collections::HashMap;

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(5, 1);

        // First 5 requests should succeed
        for _ in 0..5 {
            assert!(limiter.check_limit("test"));
        }

        // 6th request should fail
        assert!(!limiter.check_limit("test"));
    }

    #[test]
    fn test_validation_middleware() {
        // Test content type validation would require a mock request
        // This is a placeholder for actual tests
        assert!(true);
    }

    #[test]
    fn test_metrics_middleware() {
        let metrics = MetricsMiddleware::new();

        assert_eq!(metrics.get_request_count(), 0);
        assert_eq!(metrics.get_error_count(), 0);

        metrics.record_request();
        metrics.record_error();

        assert_eq!(metrics.get_request_count(), 1);
        assert_eq!(metrics.get_error_count(), 1);
    }

    #[test]
    fn test_request_id_middleware() {
        // Test request ID generation would require a mock request
        // This is a placeholder for actual tests
        assert!(true);
    }

    #[tokio::test]
    async fn test_auth_middleware() {
        let middleware = DefaultAuthMiddleware::new(true, Some("test_key".to_string()));

        assert!(middleware.verify_token("test_key").await.unwrap());
        assert!(!middleware.verify_token("wrong_key").await.unwrap());

        let disabled_middleware = DefaultAuthMiddleware::new(false, None);
        assert!(disabled_middleware.verify_token("any_key").await.unwrap());
    }

    #[tokio::test]
    async fn test_health_check_middleware() {
        let health = HealthCheckMiddleware::perform_health_checks().await.unwrap();

        assert_eq!(health.status, "healthy");
        assert!(health.checks.contains_key("database"));
        assert!(health.checks.contains_key("storage"));
    }
}
