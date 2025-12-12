// Gateway Module
//
// Part of the API Gateway and Security system for RustyDB

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use parking_lot::RwLock;
use reqwest::Client;
use crate::api::gateway::{AuthorizationEngine, SecurityEvent, SecurityEventType, SecurityFilter, AuthenticationManager, AuditLogger};
use crate::api::RateLimiter;
use crate::error::DbError;
use super::types::*;

impl ApiGateway {
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

    // Register a new route
    pub fn register_route(&self, route: Route) {
        let mut routes = self.routes.write();
        routes.insert(route.id.clone(), route);
    }

    // Remove a route
    pub fn remove_route(&self, route_id: &str) -> bool {
        let mut routes = self.routes.write();
        routes.remove(route_id).is_some()
    }

    // Get all routes
    pub fn get_routes(&self) -> Vec<Route> {
        let routes = self.routes.read();
        routes.values().cloned().collect()
    }

    // Find matching route for request
    pub fn find_route(&self, request: &ApiRequest) -> Option<Route> {
        let routes = self.routes.read();

        for route in routes.values() {
            if self.matches_route(route, request) {
                return Some(route.clone());
            }
        }

        None
    }

    // Check if request matches route
    fn matches_route(&self, route: &Route, request: &ApiRequest) -> bool {
        // Check method
        if !route.methods.contains(&request.method) {
            return false;
        }

        // Check path pattern (simplified wildcard matching)
        self.matches_path_pattern(&route.path_pattern, &request.path)
    }

    // Match path against pattern
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

    // Process incoming request
    pub async fn process_request(&self, request: ApiRequest) -> Result<ApiResponse, DbError> {
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

    // Transform request before forwarding
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

    // Transform response before returning
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

    // Forward request to backend service
    async fn forward_to_backend(&self, request: &ApiRequest, route: &Route) -> Result<ApiResponse, DbError> {
        // Select endpoint using load balancing
        let endpoint = self.select_endpoint(&route.backend)?;

        match endpoint.protocol {
            Protocol::Http => {
                let client = Client::new();
                let url = format!("http://{}:{}{}", endpoint.host, endpoint.port, request.path);

                let method = match request.method {
                    HttpMethod::Get => reqwest::Method::GET,
                    HttpMethod::Post => reqwest::Method::POST,
                    HttpMethod::Put => reqwest::Method::PUT,
                    HttpMethod::Delete => reqwest::Method::DELETE,
                    HttpMethod::Patch => reqwest::Method::PATCH,
                    HttpMethod::Head => reqwest::Method::HEAD,
                    HttpMethod::Options => reqwest::Method::OPTIONS,
                };

                let mut req_builder = client.request(method, &url);

                for (k, v) in &request.headers {
                    req_builder = req_builder.header(k, v);
                }

                if !request.body.is_empty() {
                    req_builder = req_builder.body(request.body.clone());
                }

                let start = Instant::now();
                let response = req_builder.send().await
                    .map_err(|e| DbError::Network(e.to_string()))?;
                let duration = start.elapsed();

                let status = response.status();
                let headers = response.headers().clone();
                let body = response.bytes().await
                    .map_err(|e| DbError::Network(e.to_string()))?
                    .to_vec();

                let mut response_headers = HashMap::new();
                for (k, v) in headers.iter() {
                    if let Ok(v_str) = v.to_str() {
                        response_headers.insert(k.to_string(), v_str.to_string());
                    }
                }

                Ok(ApiResponse {
                    status_code: status.as_u16(),
                    headers: response_headers,
                    body,
                    duration,
                })
            },
            _ => Err(DbError::InvalidOperation("Protocol not supported".to_string())),
        }
    }

    // Select backend endpoint using load balancing strategy
    fn select_endpoint(&self, backend: &BackendService) -> Result<ServiceEndpoint, DbError> {
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
                use rand::Rng;
                let idx = rand::rng().random_range(0..healthy_endpoints.len());
                Ok(healthy_endpoints[idx].clone())
            },
            LoadBalancingStrategy::RoundRobin => {
                // Simplified round-robin
                Ok(healthy_endpoints[0].clone())
            },
            _ => Ok(healthy_endpoints[0].clone()),
        }
    }

    // Get gateway metrics
    pub fn get_metrics(&self) -> GatewayMetrics {
        self.metrics.read().clone()
    }

    // Register backend service
    pub fn register_service(&self, service: BackendService) {
        let mut registry = self.service_registry.write();
        registry.services.insert(service.name.clone(), service);
    }

    // Start health checking for all services
    pub async fn start_health_checks(&self) {
        let registry = self.service_registry.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;

                let services = {
                    let reg = registry.read();
                    reg.services.values().cloned().collect::<Vec<_>>()
                };

                for service in services {
                    for endpoint in service.endpoints {
                        // Check health
                        let healthy = Self::check_endpoint_health(&endpoint).await;
                        let mut h = endpoint.healthy.write();
                        *h = healthy;
                        let mut last = endpoint.last_health_check.write();
                        *last = Instant::now();
                    }
                }
            }
        });
    }

    async fn check_endpoint_health(endpoint: &ServiceEndpoint) -> bool {
        match endpoint.protocol {
            Protocol::Http => {
                let client = Client::new();
                let url = format!("http://{}:{}/health", endpoint.host, endpoint.port);
                match client.get(&url).timeout(Duration::from_secs(5)).send().await {
                    Ok(resp) => resp.status().is_success(),
                    Err(_) => false,
                }
            },
            _ => true, // Assume healthy for other protocols for now
        }
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
