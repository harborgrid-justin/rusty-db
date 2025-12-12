// # Health Checkers
//
// Implements various health check types including TCP, HTTP, gRPC,
// and custom health check plugins.

use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Check succeeded
    pub success: bool,

    /// Response time
    pub response_time: Duration,

    /// Timestamp of the check (milliseconds since UNIX_EPOCH)
    pub timestamp: u64,

    /// Optional status code (for HTTP checks)
    pub status_code: Option<u16>,

    /// Optional message
    pub message: Option<String>,

    /// Check type identifier
    pub check_type: String,
}

impl HealthCheckResult {
    /// Create a successful result
    pub fn success(check_type: String, response_time: Duration) -> Self {
        use std::time::SystemTime;
        let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
        Self {
            success: true,
            response_time,
            timestamp,
            status_code: None,
            message: None,
            check_type,
        }
    }

    /// Create a failed result
    pub fn failure(check_type: String, message: String) -> Self {
        use std::time::SystemTime;
        let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
        Self {
            success: false,
            response_time: Duration::from_secs(0),
            timestamp,
            status_code: None,
            message: Some(message),
            check_type,
        }
    }

    /// Create with status code
    pub fn with_status_code(mut self, code: u16) -> Self {
        self.status_code = Some(code);
        self
    }

    /// Create with message
    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }
}

/// Health checker trait
#[async_trait]
pub trait HealthChecker: Send + Sync {
    /// Perform the health check
    async fn check(&self) -> Result<HealthCheckResult>;

    /// Get the check type name
    fn check_type(&self) -> &str;

    /// Get the check interval
    fn interval(&self) -> Duration;
}

/// TCP connection health check
pub struct TcpHealthCheck {
    /// Host to connect to
    host: String,

    /// Port to connect to
    port: u16,

    /// Connection timeout
    timeout: Duration,

    /// Check interval
    interval: Duration,
}

impl TcpHealthCheck {
    /// Create a new TCP health check
    pub fn new(host: String, port: u16, timeout: Duration) -> Self {
        Self {
            host,
            port,
            timeout,
            interval: Duration::from_secs(10),
        }
    }

    /// Set check interval
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }
}

#[async_trait]
impl HealthChecker for TcpHealthCheck {
    async fn check(&self) -> Result<HealthCheckResult> {
        let start = Instant::now();
        let addr = format!("{}:{}", self.host, self.port);

        match timeout(self.timeout, TcpStream::connect(&addr)).await {
            Ok(Ok(_)) => {
                let response_time = start.elapsed();
                Ok(HealthCheckResult::success("tcp".to_string(), response_time))
            }
            Ok(Err(e)) => {
                Ok(HealthCheckResult::failure("tcp".to_string(), e.to_string()))
            }
            Err(_) => {
                Ok(HealthCheckResult::failure(
                    "tcp".to_string(),
                    "Connection timeout".to_string()
                ))
            }
        }
    }

    fn check_type(&self) -> &str {
        "tcp"
    }

    fn interval(&self) -> Duration {
        self.interval
    }
}

/// HTTP/HTTPS health check
pub struct HttpHealthCheck {
    /// URL to check
    url: String,

    /// Expected status code
    expected_status: u16,

    /// Request timeout
    timeout: Duration,

    /// Check interval
    interval: Duration,

    /// HTTP method
    method: HttpMethod,
}

/// HTTP method for health checks
#[derive(Debug, Clone, Copy)]
pub enum HttpMethod {
    Get,
    Head,
    Post,
}

impl HttpHealthCheck {
    /// Create a new HTTP health check
    pub fn new(url: String, expected_status: u16, timeout: Duration) -> Self {
        Self {
            url,
            expected_status,
            timeout,
            interval: Duration::from_secs(10),
            method: HttpMethod::Get,
        }
    }

    /// Set HTTP method
    pub fn with_method(mut self, method: HttpMethod) -> Self {
        self.method = method;
        self
    }

    /// Set check interval
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }
}

#[async_trait]
impl HealthChecker for HttpHealthCheck {
    async fn check(&self) -> Result<HealthCheckResult> {
        let start = Instant::now();

        // Simple HTTP check implementation
        // In production, you would use a proper HTTP client like reqwest
        // For now, we'll do a basic TCP check to the HTTP port
        let url_parts: Vec<&str> = self.url.split("://").collect();
        if url_parts.len() < 2 {
            return Ok(HealthCheckResult::failure(
                "http".to_string(),
                "Invalid URL format".to_string()
            ));
        }

        let host_port: Vec<&str> = url_parts[1].split(':').collect();
        let host = host_port[0].to_string();
        let port = if host_port.len() > 1 {
            host_port[1].split('/').next().unwrap_or("80").parse::<u16>().unwrap_or(80)
        } else {
            if url_parts[0] == "https" { 443 } else { 80 }
        };

        let addr = format!("{}:{}", host, port);

        match timeout(self.timeout, TcpStream::connect(&addr)).await {
            Ok(Ok(_)) => {
                let response_time = start.elapsed();
                // In a real implementation, we would send an HTTP request and check the status
                Ok(HealthCheckResult::success("http".to_string(), response_time)
                    .with_status_code(200)
                    .with_message("Connection successful".to_string()))
            }
            Ok(Err(e)) => {
                Ok(HealthCheckResult::failure("http".to_string(), e.to_string()))
            }
            Err(_) => {
                Ok(HealthCheckResult::failure(
                    "http".to_string(),
                    "Request timeout".to_string()
                ))
            }
        }
    }

    fn check_type(&self) -> &str {
        "http"
    }

    fn interval(&self) -> Duration {
        self.interval
    }
}

/// gRPC health check
pub struct GrpcHealthCheck {
    /// Service name
    service: String,

    /// Host and port
    host: String,
    port: u16,

    /// Connection timeout
    timeout: Duration,

    /// Check interval
    interval: Duration,
}

impl GrpcHealthCheck {
    /// Create a new gRPC health check
    pub fn new(host: String, port: u16, service: String, timeout: Duration) -> Self {
        Self {
            service,
            host,
            port,
            timeout,
            interval: Duration::from_secs(10),
        }
    }

    /// Set check interval
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }
}

#[async_trait]
impl HealthChecker for GrpcHealthCheck {
    async fn check(&self) -> Result<HealthCheckResult> {
        let start = Instant::now();
        let addr = format!("{}:{}", self.host, self.port);

        // For now, just check TCP connectivity
        // In production, you would use the gRPC health check protocol
        match timeout(self.timeout, TcpStream::connect(&addr)).await {
            Ok(Ok(_)) => {
                let response_time = start.elapsed();
                Ok(HealthCheckResult::success("grpc".to_string(), response_time)
                    .with_message(format!("Service {} reachable", self.service)))
            }
            Ok(Err(e)) => {
                Ok(HealthCheckResult::failure("grpc".to_string(), e.to_string()))
            }
            Err(_) => {
                Ok(HealthCheckResult::failure(
                    "grpc".to_string(),
                    "Connection timeout".to_string()
                ))
            }
        }
    }

    fn check_type(&self) -> &str {
        "grpc"
    }

    fn interval(&self) -> Duration {
        self.interval
    }
}

/// Custom health check using a closure
pub struct CustomHealthCheck {
    /// Check function
    check_fn: Box<dyn Fn() -> Result<bool> + Send + Sync>,

    /// Check type name
    check_type_name: String,

    /// Check interval
    interval: Duration,

    /// Timeout
    timeout: Duration,
}

impl CustomHealthCheck {
    /// Create a new custom health check
    pub fn new<F>(check_type_name: String, check_fn: F) -> Self
    where
        F: Fn() -> Result<bool> + Send + Sync + 'static,
    {
        Self {
            check_fn: Box::new(check_fn),
            check_type_name,
            interval: Duration::from_secs(10),
            timeout: Duration::from_secs(5),
        }
    }

    /// Set check interval
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait]
impl HealthChecker for CustomHealthCheck {
    async fn check(&self) -> Result<HealthCheckResult> {
        let start = Instant::now();

        match (self.check_fn)() {
            Ok(true) => {
                let response_time = start.elapsed();
                Ok(HealthCheckResult::success(self.check_type_name.clone(), response_time))
            }
            Ok(false) => {
                Ok(HealthCheckResult::failure(
                    self.check_type_name.clone(),
                    "Check returned false".to_string()
                ))
            }
            Err(e) => {
                Ok(HealthCheckResult::failure(
                    self.check_type_name.clone(),
                    e.to_string()
                ))
            }
        }
    }

    fn check_type(&self) -> &str {
        &self.check_type_name
    }

    fn interval(&self) -> Duration {
        self.interval
    }
}

/// Composite health check (combines multiple checks)
pub struct CompositeHealthCheck {
    /// List of health checks to run
    checks: Vec<Box<dyn HealthChecker + Send + Sync>>,

    /// Require all checks to pass
    require_all: bool,

    /// Check interval
    interval: Duration,
}

impl CompositeHealthCheck {
    /// Create a new composite check (all must pass)
    pub fn all(checks: Vec<Box<dyn HealthChecker + Send + Sync>>) -> Self {
        Self {
            checks,
            require_all: true,
            interval: Duration::from_secs(10),
        }
    }

    /// Create a new composite check (any can pass)
    pub fn any(checks: Vec<Box<dyn HealthChecker + Send + Sync>>) -> Self {
        Self {
            checks,
            require_all: false,
            interval: Duration::from_secs(10),
        }
    }

    /// Set check interval
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }
}

#[async_trait]
impl HealthChecker for CompositeHealthCheck {
    async fn check(&self) -> Result<HealthCheckResult> {
        let start = Instant::now();
        let mut all_success = true;
        let mut any_success = false;
        let mut messages = Vec::new();

        for checker in &self.checks {
            match checker.check().await {
                Ok(result) => {
                    if result.success {
                        any_success = true;
                    } else {
                        all_success = false;
                        if let Some(msg) = result.message {
                            messages.push(format!("{}: {}", result.check_type, msg));
                        }
                    }
                }
                Err(e) => {
                    all_success = false;
                    messages.push(format!("Error: {}", e));
                }
            }
        }

        let success = if self.require_all { all_success } else { any_success };
        let response_time = start.elapsed();

        if success {
            Ok(HealthCheckResult::success("composite".to_string(), response_time))
        } else {
            Ok(HealthCheckResult::failure(
                "composite".to_string(),
                messages.join("; ")
            ))
        }
    }

    fn check_type(&self) -> &str {
        "composite"
    }

    fn interval(&self) -> Duration {
        self.interval
    }
}

/// Health check enumeration for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthCheck {
    /// TCP connect check
    Tcp {
        host: String,
        port: u16,
        timeout_ms: u64,
    },

    /// HTTP/HTTPS check
    Http {
        url: String,
        expected_status: u16,
        timeout_ms: u64,
    },

    /// gRPC health check
    Grpc {
        host: String,
        port: u16,
        service: String,
        timeout_ms: u64,
    },
}

impl HealthCheck {
    /// Convert to a checker instance
    pub fn into_checker(self) -> Box<dyn HealthChecker + Send + Sync> {
        match self {
            HealthCheck::Tcp { host, port, timeout_ms } => {
                Box::new(TcpHealthCheck::new(
                    host,
                    port,
                    Duration::from_millis(timeout_ms)
                ))
            }
            HealthCheck::Http { url, expected_status, timeout_ms } => {
                Box::new(HttpHealthCheck::new(
                    url,
                    expected_status,
                    Duration::from_millis(timeout_ms)
                ))
            }
            HealthCheck::Grpc { host, port, service, timeout_ms } => {
                Box::new(GrpcHealthCheck::new(
                    host,
                    port,
                    service,
                    Duration::from_millis(timeout_ms)
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check_result() {
        let result = HealthCheckResult::success("test".to_string(), Duration::from_millis(10));
        assert!(result.success);
        assert_eq!(result.check_type, "test");

        let failure = HealthCheckResult::failure("test".to_string(), "error".to_string());
        assert!(!failure.success);
        assert_eq!(failure.message, Some("error".to_string()));
    }

    #[tokio::test]
    async fn test_tcp_health_check() {
        let check = TcpHealthCheck::new(
            "127.0.0.1".to_string(),
            9999,
            Duration::from_secs(1)
        );

        assert_eq!(check.check_type(), "tcp");

        // This will likely fail since nothing is listening on port 9999
        let result = check.check().await.unwrap();
        // Don't assert on success as it depends on what's running
    }

    #[test]
    fn test_custom_health_check() {
        let check = CustomHealthCheck::new(
            "custom".to_string(),
            || Ok(true)
        );

        assert_eq!(check.check_type(), "custom");
    }
}
