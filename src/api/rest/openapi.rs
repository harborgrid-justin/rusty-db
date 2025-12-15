// # OpenAPI Documentation
//
// Minimal OpenAPI specification for RustyDB REST API
// Auto-generated documentation using utoipa

use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

/// OpenAPI documentation struct
///
/// This struct defines the complete OpenAPI specification for RustyDB's REST API.
/// It includes all endpoints, request/response schemas, security schemes, and metadata.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "RustyDB API",
        version = "0.2.640",
        description = "Enterprise-grade Rust-based database management system REST API\n\n\
                      RustyDB is an Oracle-compatible database with advanced features including:\n\
                      - Multi-Version Concurrency Control (MVCC)\n\
                      - Distributed clustering with Raft consensus\n\
                      - Real-time replication (async, sync, semi-sync)\n\
                      - Transparent Data Encryption (TDE)\n\
                      - Advanced security features (RBAC, FGAC, VPD, Data Masking)\n\
                      - Full-text search and spatial indexes\n\
                      - GraphQL and REST APIs\n\
                      - In-database machine learning\n\
                      - Graph and document database capabilities",
        license(
            name = "MIT OR Apache-2.0",
            url = "https://github.com/rustydb/rustydb/blob/main/LICENSE"
        ),
        contact(
            name = "RustyDB Contributors",
            email = "contributors@rustydb.io"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server"),
        (url = "https://api.rustydb.io", description = "Production server")
    ),
    tags(
        (name = "auth", description = "Authentication and session management"),
        (name = "database", description = "Core database operations - tables, queries, transactions"),
        (name = "sql", description = "SQL operations - DDL, DML, stored procedures, views"),
        (name = "admin", description = "Administrative operations - configuration, users, roles, backups"),
        (name = "system", description = "System information - server info, configuration, features"),
        (name = "health", description = "Health checks and monitoring - liveness, readiness, startup probes"),
        (name = "websocket", description = "WebSocket connections for real-time data streaming"),
        (name = "websocket-management", description = "WebSocket connection and subscription management"),
        (name = "ml", description = "Machine Learning - model creation, training, prediction, evaluation"),
        (name = "ml-websocket", description = "ML WebSocket streams - training progress, predictions, AutoML, lifecycle events"),
        (name = "analytics", description = "Analytics - OLAP, data quality, materialized views, workload analysis"),
        (name = "analytics-websocket", description = "Analytics WebSocket streams - OLAP queries, time series, profiling, workload"),
        (name = "inmemory", description = "In-Memory Column Store - enable/disable, population, eviction, statistics"),
    ),
    paths(
        // Authentication endpoints
        crate::api::rest::handlers::auth::login,
        crate::api::rest::handlers::auth::logout,
        crate::api::rest::handlers::auth::refresh,
        crate::api::rest::handlers::auth::validate,

        // Core database endpoints (from db.rs)
        crate::api::rest::handlers::db::execute_query,
        crate::api::rest::handlers::db::execute_batch,
        crate::api::rest::handlers::db::get_table,
        crate::api::rest::handlers::db::create_table,
        crate::api::rest::handlers::db::update_table,
        crate::api::rest::handlers::db::delete_table,
        crate::api::rest::handlers::db::get_schema,
        crate::api::rest::handlers::db::begin_transaction,
        crate::api::rest::handlers::db::commit_transaction,
        crate::api::rest::handlers::db::rollback_transaction,

        // SQL operations endpoints
        crate::api::rest::handlers::sql::create_database,
        crate::api::rest::handlers::sql::drop_database,
        crate::api::rest::handlers::sql::backup_database,
        crate::api::rest::handlers::sql::alter_table,
        crate::api::rest::handlers::sql::create_view,
        crate::api::rest::handlers::sql::drop_view,
        crate::api::rest::handlers::sql::create_index,
        crate::api::rest::handlers::sql::drop_index,
        crate::api::rest::handlers::sql::create_procedure,
        crate::api::rest::handlers::sql::execute_procedure,
        crate::api::rest::handlers::sql::execute_union,
        crate::api::rest::handlers::sql::truncate_table,

        // Admin endpoints
        crate::api::rest::handlers::admin::get_config,
        crate::api::rest::handlers::admin::update_config,
        crate::api::rest::handlers::admin::create_backup,
        crate::api::rest::handlers::admin::get_health,
        crate::api::rest::handlers::admin::run_maintenance,
        crate::api::rest::handlers::admin::get_users,
        crate::api::rest::handlers::admin::create_user,
        crate::api::rest::handlers::admin::get_user,
        crate::api::rest::handlers::admin::update_user,
        crate::api::rest::handlers::admin::delete_user,
        crate::api::rest::handlers::admin::get_roles,
        crate::api::rest::handlers::admin::create_role,
        crate::api::rest::handlers::admin::get_role,
        crate::api::rest::handlers::admin::update_role,
        crate::api::rest::handlers::admin::delete_role,
        crate::api::rest::handlers::admin::get_metrics,

        // System endpoints
        crate::api::rest::handlers::system::get_server_config,
        crate::api::rest::handlers::system::get_clustering_status,
        crate::api::rest::handlers::system::get_replication_status_info,
        crate::api::rest::handlers::system::get_security_features,
        crate::api::rest::handlers::system::get_server_info,

        // Health check endpoints
        crate::api::rest::handlers::health_handlers::liveness_probe,
        crate::api::rest::handlers::health_handlers::readiness_probe,
        crate::api::rest::handlers::health_handlers::startup_probe,
        crate::api::rest::handlers::health_handlers::full_health_check,

        // WebSocket endpoints
        crate::api::rest::handlers::websocket_handlers::ws_upgrade_handler,
        crate::api::rest::handlers::websocket_handlers::ws_query_stream,
        crate::api::rest::handlers::websocket_handlers::ws_metrics_stream,
        crate::api::rest::handlers::websocket_handlers::ws_events_stream,
        crate::api::rest::handlers::websocket_handlers::ws_replication_stream,

        // WebSocket Management endpoints
        crate::api::rest::handlers::websocket_handlers::get_websocket_status,
        crate::api::rest::handlers::websocket_handlers::list_connections,
        crate::api::rest::handlers::websocket_handlers::get_connection,
        crate::api::rest::handlers::websocket_handlers::disconnect_connection,
        crate::api::rest::handlers::websocket_handlers::broadcast_message,
        crate::api::rest::handlers::websocket_handlers::list_subscriptions,
        crate::api::rest::handlers::websocket_handlers::create_subscription,
        crate::api::rest::handlers::websocket_handlers::delete_subscription,

        // Machine Learning endpoints
        crate::api::rest::handlers::ml_handlers::create_model,
        crate::api::rest::handlers::ml_handlers::list_models,
        crate::api::rest::handlers::ml_handlers::get_model,
        crate::api::rest::handlers::ml_handlers::delete_model,
        crate::api::rest::handlers::ml_handlers::train_model,
        crate::api::rest::handlers::ml_handlers::predict,
        crate::api::rest::handlers::ml_handlers::get_model_metrics,
        crate::api::rest::handlers::ml_handlers::evaluate_model,
        crate::api::rest::handlers::ml_handlers::export_model,

        // ML WebSocket endpoints
        crate::api::rest::handlers::ml_websocket_handlers::ws_ml_training_progress,
        crate::api::rest::handlers::ml_websocket_handlers::ws_ml_predictions,
        crate::api::rest::handlers::ml_websocket_handlers::ws_ml_automl_progress,
        crate::api::rest::handlers::ml_websocket_handlers::ws_ml_lifecycle_events,

        // Analytics endpoints
        crate::api::rest::handlers::analytics_handlers::create_olap_cube,
        crate::api::rest::handlers::analytics_handlers::list_olap_cubes,
        crate::api::rest::handlers::analytics_handlers::query_olap_cube,
        crate::api::rest::handlers::analytics_handlers::delete_olap_cube,
        crate::api::rest::handlers::analytics_handlers::get_query_statistics,
        crate::api::rest::handlers::analytics_handlers::analyze_workload,
        crate::api::rest::handlers::analytics_handlers::get_recommendations,
        crate::api::rest::handlers::analytics_handlers::profile_table,
        crate::api::rest::handlers::analytics_handlers::get_quality_metrics,
        crate::api::rest::handlers::analytics_handlers::get_quality_issues,
        crate::api::rest::handlers::analytics_handlers::create_materialized_view,
        crate::api::rest::handlers::analytics_handlers::list_materialized_views,
        crate::api::rest::handlers::analytics_handlers::refresh_materialized_view,

        // Analytics WebSocket endpoints
        crate::api::rest::handlers::analytics_websocket_handlers::ws_analytics_olap,
        crate::api::rest::handlers::analytics_websocket_handlers::ws_analytics_timeseries,
        crate::api::rest::handlers::analytics_websocket_handlers::ws_analytics_profiling,
        crate::api::rest::handlers::analytics_websocket_handlers::ws_analytics_workload,
        crate::api::rest::handlers::analytics_websocket_handlers::ws_analytics_cache_events,

        // InMemory Column Store endpoints
        crate::api::rest::handlers::inmemory_handlers::enable_inmemory,
        crate::api::rest::handlers::inmemory_handlers::disable_inmemory,
        crate::api::rest::handlers::inmemory_handlers::inmemory_status,
        crate::api::rest::handlers::inmemory_handlers::inmemory_stats,
        crate::api::rest::handlers::inmemory_handlers::populate_table,
        crate::api::rest::handlers::inmemory_handlers::get_table_status,
        crate::api::rest::handlers::inmemory_handlers::evict_tables,
        crate::api::rest::handlers::inmemory_handlers::compact_memory,
        crate::api::rest::handlers::inmemory_handlers::get_inmemory_config,
        crate::api::rest::handlers::inmemory_handlers::update_inmemory_config,
    ),
    components(
        schemas(
            // Core types
            crate::api::rest::types::ApiError,
            crate::api::rest::types::SessionId,
            crate::api::rest::types::TransactionId,

            // Database operation types
            crate::api::rest::types::QueryRequest,
            crate::api::rest::types::QueryResponse,
            crate::api::rest::types::ColumnMetadata,
            crate::api::rest::types::BatchRequest,
            crate::api::rest::types::BatchResponse,
            crate::api::rest::types::BatchStatementResult,
            crate::api::rest::types::TableRequest,
            crate::api::rest::types::TableColumn,
            crate::api::rest::types::IndexDefinition,
            crate::api::rest::types::SchemaResponse,
            crate::api::rest::types::TableInfo,
            crate::api::rest::types::ViewInfo,
            crate::api::rest::types::ProcedureInfo,
            crate::api::rest::types::IndexInfo,
            crate::api::rest::types::TransactionRequest,
            crate::api::rest::types::TransactionResponse,

            // SQL operation types
            crate::api::rest::handlers::sql::AlterTableRequest,
            crate::api::rest::handlers::sql::ColumnDefinition,
            crate::api::rest::handlers::sql::ConstraintDefinition,
            crate::api::rest::handlers::sql::DatabaseBackupRequest,
            crate::api::rest::handlers::sql::ProcedureRequest,
            crate::api::rest::handlers::sql::ParameterDef,
            crate::api::rest::handlers::sql::ExecProcedureRequest,
            crate::api::rest::handlers::sql::UnionRequest,
            crate::api::rest::handlers::sql::ViewRequest,

            // Authentication types
            crate::api::rest::handlers::auth::LoginRequest,
            crate::api::rest::handlers::auth::LoginResponse,
            crate::api::rest::handlers::auth::UserInfo,
            crate::api::rest::handlers::auth::RoleInfo,
            crate::api::rest::handlers::auth::SessionInfo,

            // Admin types
            crate::api::rest::types::ConfigResponse,
            crate::api::rest::types::BackupRequest,
            crate::api::rest::types::BackupResponse,
            crate::api::rest::types::HealthResponse,
            crate::api::rest::types::ComponentHealth,
            crate::api::rest::types::MaintenanceRequest,
            crate::api::rest::types::UserRequest,
            crate::api::rest::types::UserResponse,
            crate::api::rest::types::RoleRequest,
            crate::api::rest::types::RoleResponse,

            // System information types
            crate::api::rest::types::ServerConfigResponse,
            crate::api::rest::types::ClusterStatusResponse,
            crate::api::rest::types::ClusterNodeStatus,
            crate::api::rest::types::ReplicationStatusInfoResponse,
            crate::api::rest::types::ReplicaInfo,
            crate::api::rest::types::SecurityFeaturesResponse,
            crate::api::rest::types::SecurityFeatureStatus,
            crate::api::rest::types::ServerInfoResponse,

            // Health check types
            crate::api::rest::handlers::health_handlers::LivenessProbeResponse,
            crate::api::rest::handlers::health_handlers::ReadinessProbeResponse,
            crate::api::rest::handlers::health_handlers::StartupProbeResponse,
            crate::api::rest::handlers::health_handlers::FullHealthResponse,
            crate::api::rest::handlers::health_handlers::ComponentHealthDetail,

            // WebSocket types
            crate::api::rest::handlers::websocket_handlers::WebSocketMessage,
            crate::api::rest::handlers::websocket_handlers::QueryStreamRequest,
            crate::api::rest::handlers::websocket_handlers::MetricsStreamConfig,
            crate::api::rest::handlers::websocket_handlers::EventsStreamConfig,
            crate::api::rest::handlers::websocket_handlers::ReplicationStreamConfig,

            // WebSocket Management types
            crate::api::rest::handlers::websocket_types::WebSocketStatus,
            crate::api::rest::handlers::websocket_types::ConnectionInfo,
            crate::api::rest::handlers::websocket_types::ConnectionList,
            crate::api::rest::handlers::websocket_types::SubscriptionInfo,
            crate::api::rest::handlers::websocket_types::SubscriptionList,
            crate::api::rest::handlers::websocket_types::CreateSubscriptionRequest,
            crate::api::rest::handlers::websocket_types::CreateSubscriptionResponse,
            crate::api::rest::handlers::websocket_types::BroadcastRequest,
            crate::api::rest::handlers::websocket_types::BroadcastResponse,
            crate::api::rest::handlers::websocket_types::DisconnectRequest,
            crate::api::rest::handlers::websocket_types::DisconnectResponse,
            crate::api::rest::handlers::websocket_types::DeleteSubscriptionResponse,

            // Pagination types
            crate::api::rest::types::PaginationParams,

            // Machine Learning types
            crate::api::rest::handlers::ml_handlers::CreateModelRequest,
            crate::api::rest::handlers::ml_handlers::ModelResponse,
            crate::api::rest::handlers::ml_handlers::TrainModelRequest,
            crate::api::rest::handlers::ml_handlers::TrainModelResponse,
            crate::api::rest::handlers::ml_handlers::PredictRequest,
            crate::api::rest::handlers::ml_handlers::PredictResponse,
            crate::api::rest::handlers::ml_handlers::ModelListResponse,
            crate::api::rest::handlers::ml_handlers::ModelSummary,
            crate::api::rest::handlers::ml_handlers::ModelMetricsResponse,
            crate::api::rest::handlers::ml_handlers::FeatureImportance,
            crate::api::rest::handlers::ml_handlers::ModelEvaluationRequest,
            crate::api::rest::handlers::ml_handlers::ModelEvaluationResponse,

            // ML WebSocket types
            crate::api::rest::handlers::ml_websocket_handlers::MLWebSocketMessage,
            crate::api::rest::handlers::ml_websocket_handlers::TrainingProgressUpdate,
            crate::api::rest::handlers::ml_websocket_handlers::PredictionResult,
            crate::api::rest::handlers::ml_websocket_handlers::AutoMLProgressUpdate,
            crate::api::rest::handlers::ml_websocket_handlers::ModelLifecycleEvent,

            // Analytics types
            crate::api::rest::handlers::analytics_handlers::CreateCubeRequest,
            crate::api::rest::handlers::analytics_handlers::MeasureSpec,
            crate::api::rest::handlers::analytics_handlers::CubeResponse,
            crate::api::rest::handlers::analytics_handlers::CubeListResponse,
            crate::api::rest::handlers::analytics_handlers::CubeQueryRequest,
            crate::api::rest::handlers::analytics_handlers::CubeQueryResponse,
            crate::api::rest::handlers::analytics_handlers::QueryStatsFilter,
            crate::api::rest::handlers::analytics_handlers::QueryStatisticsResponse,
            crate::api::rest::handlers::analytics_handlers::QueryStatEntry,
            crate::api::rest::handlers::analytics_handlers::WorkloadAnalysisResponse,
            crate::api::rest::handlers::analytics_handlers::RecommendationEntry,
            crate::api::rest::handlers::analytics_handlers::ProfileTableRequest,
            crate::api::rest::handlers::analytics_handlers::ProfileTableResponse,
            crate::api::rest::handlers::analytics_handlers::ColumnProfileEntry,
            crate::api::rest::handlers::analytics_handlers::IndexSuggestionEntry,
            crate::api::rest::handlers::analytics_handlers::QualityMetricsResponse,
            crate::api::rest::handlers::analytics_handlers::QualityIssuesResponse,
            crate::api::rest::handlers::analytics_handlers::QualityIssueEntry,
            crate::api::rest::handlers::analytics_handlers::CreateMaterializedViewRequest,
            crate::api::rest::handlers::analytics_handlers::RefreshScheduleSpec,
            crate::api::rest::handlers::analytics_handlers::MaterializedViewResponse,
            crate::api::rest::handlers::analytics_handlers::MaterializedViewListResponse,
            crate::api::rest::handlers::analytics_handlers::RefreshMaterializedViewResponse,

            // Analytics WebSocket types
            crate::api::rest::handlers::analytics_websocket_handlers::AnalyticsWebSocketMessage,
            crate::api::rest::handlers::analytics_websocket_handlers::OLAPQueryRequest,
            crate::api::rest::handlers::analytics_websocket_handlers::OLAPQueryResult,
            crate::api::rest::handlers::analytics_websocket_handlers::TimeSeriesAnalysisRequest,
            crate::api::rest::handlers::analytics_websocket_handlers::TimeSeriesAnalysisUpdate,
            crate::api::rest::handlers::analytics_websocket_handlers::DataProfilingRequest,
            crate::api::rest::handlers::analytics_websocket_handlers::DataProfilingUpdate,
            crate::api::rest::handlers::analytics_websocket_handlers::ColumnProfile,
            crate::api::rest::handlers::analytics_websocket_handlers::WorkloadAnalysisRequest,
            crate::api::rest::handlers::analytics_websocket_handlers::WorkloadAnalysisUpdate,
            crate::api::rest::handlers::analytics_websocket_handlers::WorkloadRecommendation,
            crate::api::rest::handlers::analytics_websocket_handlers::QueryCacheEvent,

            // InMemory Column Store types
            crate::api::rest::handlers::inmemory_handlers::EnableInMemoryRequest,
            crate::api::rest::handlers::inmemory_handlers::EnableInMemoryResponse,
            crate::api::rest::handlers::inmemory_handlers::InMemoryStatusResponse,
            crate::api::rest::handlers::inmemory_handlers::InMemoryTableInfo,
            crate::api::rest::handlers::inmemory_handlers::PopulateRequest,
            crate::api::rest::handlers::inmemory_handlers::PopulateResponse,
            crate::api::rest::handlers::inmemory_handlers::EvictRequest,
            crate::api::rest::handlers::inmemory_handlers::EvictResponse,
            crate::api::rest::handlers::inmemory_handlers::InMemoryStatsResponse,
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// Security scheme configuration
///
/// Adds Bearer token and API key authentication schemes to the OpenAPI spec
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            // Add Bearer token authentication
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("JWT token for authentication. Obtain from /api/v1/auth/login endpoint."))
                        .build(),
                ),
            );

            // Add API key authentication
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-API-Key"))),
            );
        }
    }
}

/// Helper function to get OpenAPI JSON specification
///
/// # Returns
/// JSON string containing the complete OpenAPI specification
///
/// # Example
/// ```no_run
/// let spec = get_openapi_json();
/// println!("{}", spec);
/// ```
pub fn get_openapi_json() -> String {
    ApiDoc::openapi()
        .to_pretty_json()
        .unwrap_or_else(|e| format!(r#"{{"error": "Failed to generate OpenAPI spec: {}"}}"#, e))
}

/// Helper function to get OpenAPI specification as pretty-printed JSON
///
/// # Returns
/// Formatted JSON string containing the complete OpenAPI specification
///
/// # Example
/// ```no_run
/// let spec = get_openapi_pretty();
/// println!("{}", spec);
/// ```
pub fn get_openapi_pretty() -> String {
    serde_json::to_string_pretty(&ApiDoc::openapi())
        .unwrap_or_else(|e| format!("{{ \"error\": \"Failed to generate OpenAPI: {}\" }}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_generation() {
        let openapi = ApiDoc::openapi();
        assert_eq!(openapi.info.title, "RustyDB API");
        assert_eq!(openapi.info.version, "0.2.640");
    }

    #[test]
    fn test_openapi_json() {
        let json = get_openapi_json();
        assert!(!json.is_empty());
        assert!(json.contains("RustyDB API"));
    }

    #[test]
    fn test_openapi_yaml() {
        let yaml = get_openapi_yaml();
        assert!(!yaml.is_empty());
        assert!(yaml.contains("RustyDB API"));
    }

    fn get_openapi_yaml() -> _ {
        todo!()
    }

    #[test]
    fn test_security_schemes() {
        let openapi = ApiDoc::openapi();
        let components = openapi.components.expect("Components should be present");

        // Check for security schemes
        let security_schemes = components.security_schemes;
        assert!(security_schemes.contains_key("bearer_auth"));
        assert!(security_schemes.contains_key("api_key"));
    }

    #[test]
    fn test_paths_exist() {
        let openapi = ApiDoc::openapi();
        let paths = openapi.paths.paths;

        // Verify some core paths exist
        assert!(paths.contains_key("/api/v1/auth/login"));
        assert!(paths.contains_key("/api/v1/health/live"));
        assert!(paths.contains_key("/api/v1/ws"));
    }
}
