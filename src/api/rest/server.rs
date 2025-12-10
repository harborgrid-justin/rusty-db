// # REST API Server Implementation
//
// Server setup, routing, and core functionality for the REST API.
// Uses dependency injection and proper error handling.

use axum::{
    Router,
    routing::{get, post, put, delete},
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::{Response, IntoResponse, Html},
    http::Method,
    middleware,
};
use futures::SinkExt;
use tower_http::{
    cors::{CorsLayer, Any},
    trace::TraceLayer,
    timeout::TimeoutLayer,
    limit::RequestBodyLimitLayer,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration};
use tokio::sync::{RwLock, Semaphore};
use async_graphql::{Schema, http::GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use crate::api::ApiConfig;
use crate::api::graphql::{QueryRoot, MutationRoot, SubscriptionRoot, GraphQLEngine, AuthorizationContext};
use crate::error::DbError;
use super::types::{ApiState, ApiMetrics, RateLimiter, QueryRequest};
use super::handlers::db::{execute_query, execute_batch, get_table, create_table, update_table, delete_table, get_schema, begin_transaction, commit_transaction, rollback_transaction};
use super::handlers::admin::{get_config, update_config, create_backup, get_health, run_maintenance, get_users, create_user, get_user, update_user, delete_user, get_roles, create_role, get_role, update_role, delete_role};
use super::handlers::monitoring::{get_metrics, get_prometheus_metrics, get_session_stats, get_query_stats, get_performance_data, get_logs, get_alerts, acknowledge_alert};
use super::handlers::pool::{get_pools, get_pool, update_pool, get_pool_stats, drain_pool, get_connections, get_connection, kill_connection, get_sessions, get_session, terminate_session};
use super::handlers::cluster::{get_cluster_nodes, add_cluster_node, get_cluster_node, remove_cluster_node, get_cluster_topology, trigger_failover, get_replication_status, get_cluster_config, update_cluster_config};
use super::middleware::{request_logger_middleware, rate_limit_middleware};
use super::handlers::{CATALOG, TXN_MANAGER, SQL_PARSER};
use crate::execution::Executor;


// Type alias for the GraphQL schema
type GraphQLSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

// REST API server with dependency injection
pub struct RestApiServer {
    config: ApiConfig,
    state: Arc<ApiState>,
    graphql_schema: GraphQLSchema,
}

impl RestApiServer {
    // Create a new REST API server with injected dependencies
    pub async fn new(config: ApiConfig) -> Result<Self, DbError> {
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

        // Build GraphQL schema with engine and authorization context
        let graphql_engine = Arc::new(GraphQLEngine::new());

        // Create admin authorization context
        let auth_context = Arc::new(AuthorizationContext::new(
            "admin".to_string(),
            vec!["admin".to_string()],
            vec!["admin.*".to_string()],
        ));

        let graphql_schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
            .data(graphql_engine)
            .data(auth_context)
            .finish();

        Ok(Self { config, state, graphql_schema })
    }

    // Build the router with all endpoints and middleware
    fn build_router(&self) -> Router {
        // Create GraphQL router with its own state
        let graphql_schema = self.graphql_schema.clone();
        let graphql_router = Router::new()
            .route("/graphql", post(graphql_handler).get(graphql_playground))
            .route("/graphql/ws", get(graphql_subscription))
            .with_state(graphql_schema);

        let mut router = Router::new()
            // Merge GraphQL router
            .merge(graphql_router)

            // Core Database Operations API
            .route("/api/v1/query", post(execute_query))
            .route("/api/v1/batch", post(execute_batch))
            .route("/api/v1/tables/{name}", get(get_table))
            .route("/api/v1/tables/{name}", post(create_table))
            .route("/api/v1/tables/{name}", put(update_table))
            .route("/api/v1/tables/{name}", delete(delete_table))
            .route("/api/v1/schema", get(get_schema))
            .route("/api/v1/transactions", post(begin_transaction))
            .route("/api/v1/transactions/{id}/commit", post(commit_transaction))
            .route("/api/v1/transactions/{id}/rollback", post(rollback_transaction))
            .route("/api/v1/stream", get(websocket_stream))

            // Administration API
            .route("/api/v1/admin/config", get(get_config))
            .route("/api/v1/admin/config", put(update_config))
            .route("/api/v1/admin/backup", post(create_backup))
            .route("/api/v1/admin/health", get(get_health))
            .route("/api/v1/admin/maintenance", post(run_maintenance))
            .route("/api/v1/admin/users", get(get_users))
            .route("/api/v1/admin/users", post(create_user))
            .route("/api/v1/admin/users/{id}", get(get_user))
            .route("/api/v1/admin/users/{id}", put(update_user))
            .route("/api/v1/admin/users/{id}", delete(delete_user))
            .route("/api/v1/admin/roles", get(get_roles))
            .route("/api/v1/admin/roles", post(create_role))
            .route("/api/v1/admin/roles/{id}", get(get_role))
            .route("/api/v1/admin/roles/{id}", put(update_role))
            .route("/api/v1/admin/roles/{id}", delete(delete_role))

            // Monitoring & Metrics API
            .route("/api/v1/metrics", get(get_metrics))
            .route("/api/v1/metrics/prometheus", get(get_prometheus_metrics))
            .route("/api/v1/stats/sessions", get(get_session_stats))
            .route("/api/v1/stats/queries", get(get_query_stats))
            .route("/api/v1/stats/performance", get(get_performance_data))
            .route("/api/v1/logs", get(get_logs))
            .route("/api/v1/alerts", get(get_alerts))
            .route("/api/v1/alerts/{id}/acknowledge", post(acknowledge_alert))

            // Pool & Connection Management API
            .route("/api/v1/pools", get(get_pools))
            .route("/api/v1/pools/{id}", get(get_pool))
            .route("/api/v1/pools/{id}", put(update_pool))
            .route("/api/v1/pools/{id}/stats", get(get_pool_stats))
            .route("/api/v1/pools/{id}/drain", post(drain_pool))
            .route("/api/v1/connections", get(get_connections))
            .route("/api/v1/connections/{id}", get(get_connection))
            .route("/api/v1/connections/{id}", delete(kill_connection))
            .route("/api/v1/sessions", get(get_sessions))
            .route("/api/v1/sessions/{id}", get(get_session))
            .route("/api/v1/sessions/{id}", delete(terminate_session))

            // Cluster Management API
            .route("/api/v1/cluster/nodes", get(get_cluster_nodes))
            .route("/api/v1/cluster/nodes", post(add_cluster_node))
            .route("/api/v1/cluster/nodes/{id}", get(get_cluster_node))
            .route("/api/v1/cluster/nodes/{id}", delete(remove_cluster_node))
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
            .layer(RequestBodyLimitLayer::new(self.config.max_body_size as u64 as usize))
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

    // Run the API server
    pub async fn run(&self, addr: &str) -> Result<(), DbError> {
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
            Self::new(ApiConfig::default()).await.unwrap()
        })
    }
}

// WebSocket handler for streaming query results
async fn websocket_stream(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

fn get_executor() -> Executor {
    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone())
}

async fn handle_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(request) = serde_json::from_str::<QueryRequest>(&text) {
                        let executor = get_executor();

                        let response = match SQL_PARSER.parse(&request.sql) {
                            Ok(stmts) => {
                                let stmt = match stmts.into_iter().next() {
                                    Some(s) => s,
                                    None => {
                                        let _ = socket.send(Message::Text(json!({"status": "error", "message": "No valid SQL statement"}).to_string().into())).await;
                                        continue;
                                    }
                                };
                                match executor.execute(stmt) {
                                    Ok(result) => {
                                        let rows: Vec<Vec<serde_json::Value>> = result.rows.iter().map(|row| {
                                            row.iter().map(|val| serde_json::Value::String(val.clone())).collect()
                                        }).collect();

                                        json!({
                                            "status": "success",
                                            "rows": rows,
                                            "columns": result.columns,
                                            "rows_affected": result.rows_affected
                                        })
                                    },
                                    Err(e) => json!({
                                        "status": "error",
                                        "message": e.to_string()
                                    })
                                }
                            },
                            Err(e) => json!({
                                "status": "error",
                                "message": e.to_string()
                            })
                        };

                        if socket.send(Message::Text(response.to_string().into())).await.is_err() {
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

// ============================================================================
// GRAPHQL HANDLERS
// ============================================================================

// GraphQL query/mutation handler
async fn graphql_handler(
    State(schema): State<GraphQLSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

// GraphQL Playground UI
async fn graphql_playground() -> impl IntoResponse {
    Html(async_graphql::http::playground_source(
        GraphQLPlaygroundConfig::new("/graphql").subscription_endpoint("/graphql/ws")
    ))
}

// GraphQL WebSocket subscriptions
async fn graphql_subscription(
    ws: WebSocketUpgrade,
    State(schema): State<GraphQLSchema>,
) -> impl IntoResponse {
    use futures_util::StreamExt;
    use axum::extract::ws::{Message, Utf8Bytes};

    ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();

        while let Some(msg) = stream.next().await {
            if let Ok(msg) = msg {
                if let Message::Text(text) = msg {
                    if let Ok(request) = serde_json::from_str::<async_graphql::Request>(&text) {
                        let response = schema.execute(request).await;
                        let response_text = serde_json::to_string(&response).unwrap_or_default();
                        // Convert String to Utf8Bytes for axum 0.8
                        if let Ok(utf8_bytes) = Utf8Bytes::try_from(response_text.into_bytes()) {
                            let _ = sink.send(Message::Text(utf8_bytes)).await;
                        }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::api::{ApiConfig, ApiError, RestApiServer};
    use crate::api::rest_api::{PaginatedResponse, PaginationParams, RateLimiter};

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
