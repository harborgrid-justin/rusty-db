//! # REST API Server Implementation
//!
//! Server setup, routing, and core functionality for the REST API.
//! Uses dependency injection and proper error handling.

use axum::{
    Router,
    routing::{get, post, put, delete},
    extract::{Path, Query, State, WebSocketUpgrade, ws::WebSocket},
    response::{Response, IntoResponse, Json as AxumJson},
    http::{StatusCode, HeaderMap, Method},
    middleware::{self, Next},
    body::Body,
};
use tower::{ServiceBuilder, ServiceExt};
use tower_http::{
    cors::{CorsLayer, Any},
    trace::TraceLayer,
    timeout::TimeoutLayer,
    limit::RequestBodyLimitLayer,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, Semaphore};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

use crate::error::DbError;
use crate::common::*;

use super::types::*;
use super::handlers::*;
use super::middleware::*;

/// REST API server with dependency injection
pub struct RestApiServer {
    config: ApiConfig,
    state: Arc<ApiState>,
}

impl RestApiServer {
    /// Create a new REST API server with injected dependencies
    pub async fn new(config: ApiConfig) -> std::result::Result<Self, DbError> {
        let state = Arc::new(ApiState {
            config: config.clone(),
            connection_semaphore: Arc::new(Semaphore::new(config.max_connections)),
            active_queries: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(ApiMetrics::default())),
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new(
                config.rate_limit_rps,
                1,
            ))),
        });

        Ok(Self { config, state })
    }

    /// Build the router with all endpoints and middleware
    fn build_router(&self) -> Router {
        let mut router = Router::new()
            // Core Database Operations API
            .route("/api/v1/query", post(execute_query))
            .route("/api/v1/batch", post(execute_batch))
            .route("/api/v1/tables/:name", get(get_table))
            .route("/api/v1/tables/:name", post(create_table))
            .route("/api/v1/tables/:name", put(update_table))
            .route("/api/v1/tables/:name", delete(delete_table))
            .route("/api/v1/schema", get(get_schema))
            .route("/api/v1/transactions", post(begin_transaction))
            .route("/api/v1/transactions/:id/commit", post(commit_transaction))
            .route("/api/v1/transactions/:id/rollback", post(rollback_transaction))
            .route("/api/v1/stream", get(websocket_stream))

            // Administration API
            .route("/api/v1/admin/config", get(get_config))
            .route("/api/v1/admin/config", put(update_config))
            .route("/api/v1/admin/backup", post(create_backup))
            .route("/api/v1/admin/health", get(get_health))
            .route("/api/v1/admin/maintenance", post(run_maintenance))
            .route("/api/v1/admin/users", get(get_users))
            .route("/api/v1/admin/users", post(create_user))
            .route("/api/v1/admin/users/:id", get(get_user))
            .route("/api/v1/admin/users/:id", put(update_user))
            .route("/api/v1/admin/users/:id", delete(delete_user))
            .route("/api/v1/admin/roles", get(get_roles))
            .route("/api/v1/admin/roles", post(create_role))
            .route("/api/v1/admin/roles/:id", get(get_role))
            .route("/api/v1/admin/roles/:id", put(update_role))
            .route("/api/v1/admin/roles/:id", delete(delete_role))

            // Monitoring & Metrics API
            .route("/api/v1/metrics", get(get_metrics))
            .route("/api/v1/metrics/prometheus", get(get_prometheus_metrics))
            .route("/api/v1/stats/sessions", get(get_session_stats))
            .route("/api/v1/stats/queries", get(get_query_stats))
            .route("/api/v1/stats/performance", get(get_performance_data))
            .route("/api/v1/logs", get(get_logs))
            .route("/api/v1/alerts", get(get_alerts))
            .route("/api/v1/alerts/:id/acknowledge", post(acknowledge_alert))

            // Pool & Connection Management API
            .route("/api/v1/pools", get(get_pools))
            .route("/api/v1/pools/:id", get(get_pool))
            .route("/api/v1/pools/:id", put(update_pool))
            .route("/api/v1/pools/:id/stats", get(get_pool_stats))
            .route("/api/v1/pools/:id/drain", post(drain_pool))
            .route("/api/v1/connections", get(get_connections))
            .route("/api/v1/connections/:id", get(get_connection))
            .route("/api/v1/connections/:id", delete(kill_connection))
            .route("/api/v1/sessions", get(get_sessions))
            .route("/api/v1/sessions/:id", get(get_session))
            .route("/api/v1/sessions/:id", delete(terminate_session))

            // Cluster Management API
            .route("/api/v1/cluster/nodes", get(get_cluster_nodes))
            .route("/api/v1/cluster/nodes", post(add_cluster_node))
            .route("/api/v1/cluster/nodes/:id", get(get_cluster_node))
            .route("/api/v1/cluster/nodes/:id", delete(remove_cluster_node))
            .route("/api/v1/cluster/topology", get(get_cluster_topology))
            .route("/api/v1/cluster/failover", post(trigger_failover))
            .route("/api/v1/cluster/replication", get(get_replication_status))
            .route("/api/v1/cluster/config", get(get_cluster_config))
            .route("/api/v1/cluster/config", put(update_cluster_config))

            .with_state(self.state.clone());

        // Add middleware layers
        router = router
            .layer(TraceLayer::new_for_http())
            .layer(TimeoutLayer::new(Duration::from_secs(self.config.request_timeout_secs)))
            .layer(RequestBodyLimitLayer::new(self.config.max_body_size as u64))
            .layer(middleware::from_fn_with_state(
                self.state.clone(),
                request_logger_middleware,
            ))
            .layer(middleware::from_fn_with_state(
                self.state.clone(),
                rate_limit_middleware,
            ));

        // Add CORS if enabled
        if self.config.enable_cors {
            router = router.layer(
                CorsLayer::new()
                    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                    .allow_headers(Any)
                    .allow_origin(Any)
            );
        }

        // Add Swagger UI if enabled
        // FIXME: SwaggerUi integration disabled - needs proper Router conversion
        // See: https://docs.rs/utoipa-swagger-ui/latest/utoipa_swagger_ui/
        // if self.config.enable_swagger {
        //     router = router.merge(
        //         SwaggerUi::new("/swagger-ui")
        //             .url("/api-docs/openapi.json", ApiDoc::openapi())
        //     );
        // }

        router
    }

    /// Run the API server
    pub async fn run(&self, addr: &str) -> std::result::Result<(), DbError> {
        let router = self.build_router();

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| DbError::Network(format!("Failed to bind to {}: {}", addr, e)))?;

        tracing::info!("REST API server listening on {}", addr);

        axum::serve(listener, router)
            .await
            .map_err(|e| DbError::Network(format!("Server error: {}", e)))?;

        Ok(())
    }
}

impl Default for RestApiServer {
    fn default() -> Self {
        // This would normally inject dependencies, but for default we create them
        futures::executor::block_on(async {
            Self::new(ApiConfig::default()).unwrap()
        })
    }
}

/// WebSocket handler for streaming query results
async fn websocket_stream(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(_request) = serde_json::from_str::<QueryRequest>(&text) {
                        // TODO: Execute query and stream results
                        let response = json!({
                            "status": "success",
                            "rows": []
                        });

                        if socket.send(Message::Text(response.to_string())).await.is_err() {
                            break;
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.enable_cors, true);
        assert_eq!(config.rate_limit_rps, 100);
    }

    #[test]
    fn test_api_error_creation() {
        let error = ApiError::new("TEST_ERROR", "Test message");
        assert_eq!(error.code, "TEST_ERROR");
        assert_eq!(error.message, "Test message");
        assert!(error.details.is_none());
    }

    #[test]
    fn test_pagination_params() {
        let params = PaginationParams {
            page: 2,
            page_size: 25,
            sort_by: None,
            sort_order: None,
        };
        assert_eq!(params.page, 2);
        assert_eq!(params.page_size, 25);
    }

    #[test]
    fn test_paginated_response() {
        let data = vec![1, 2, 3, 4, 5];
        let response = PaginatedResponse::new(data, 1, 5, 20);

        assert_eq!(response.page, 1);
        assert_eq!(response.page_size, 5);
        assert_eq!(response.total_pages, 4);
        assert_eq!(response.total_count, 20);
        assert!(response.has_next);
        assert!(!response.has_prev);
    }

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

    #[tokio::test]
    async fn test_server_creation() {
        let config = ApiConfig::default();
        let server = RestApiServer::new(config).await;
        assert!(server.is_ok());
    }
}
