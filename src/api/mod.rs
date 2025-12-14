// # API Layer
//
// Comprehensive API layer exposing all database functionality via multiple interfaces.
//
// ## Modules
//
// - **rest**: REST API endpoints for database operations
// - **graphql_api**: GraphQL API layer with real-time subscriptions
// - **monitoring**: Monitoring, metrics, and health check APIs
// - **gateway**: API gateway for request routing and management
// - **enterprise_integration**: Master integration layer coordinating all enterprise modules
//
// ## Features
//
// - Core database operations (queries, transactions, CRUD)
// - Administration endpoints (config, backup, maintenance)
// - Monitoring and metrics (Prometheus, health checks)
// - Pool and connection management
// - Cluster management
// - OpenAPI/Swagger documentation
// - Request validation and response pagination
// - Rate limiting and CORS support
// - API Gateway with authentication, authorization, and security
// - Enterprise service integration and orchestration
//
// ## Usage
//
// ```rust,no_run
// use rusty_db::api::rest_api::{RestApiServer, ApiConfig};
// use rusty_db::api::enterprise_integration::{EnterpriseIntegrator, IntegratorConfig};
//
// #[tokio::main]
// async fn main() {
//     // Start enterprise integration layer
//     let integrator_config = IntegratorConfig::default();
//     let integrator = EnterpriseIntegrator::new(integrator_config).await.unwrap();
//     integrator.start().await.unwrap();
//
//     // Start REST API
//     let config = ApiConfig::default();
//     let server = RestApiServer::new(config).await.unwrap();
//     server.run("0.0.0.0:8080").await.unwrap();
// }
// ```

// Refactored modular API structure
pub mod enterprise;
pub mod gateway;
pub mod graphql;
pub mod monitoring;
pub mod rest;

// For backward compatibility, create module aliases
pub use enterprise as enterprise_integration;
pub use graphql as graphql_api;
pub use rest as rest_api;

pub use rest::{ApiConfig, ApiError, ApiResult, RestApiServer};

pub use monitoring::{
    Alert, AlertManager, AlertSeverity, Dashboard, DashboardManager, ExportFormat,
    HealthCheckCoordinator, HealthCheckResult, HealthStatus, MetricsRegistry, MonitoringApi,
    MonitoringConfig, PrometheusExporter, TimeSeriesDatabase, TimeSeriesQuery, TimeSeriesResult,
};

pub use gateway::{
    ApiGateway, ApiRequest, ApiResponse, BackendService, GatewayConfig, GatewayMetrics, Protocol,
    RateLimitConfig, RateLimitType, Route, Session,
};

pub use graphql::{
    // Schema building
    build_schema,
    build_schema_with_config,
    AggregateChange,
    AggregateFunc,
    AggregateInput,
    AggregateResult,
    ArgumentInfo,
    AuthorizationContext,
    BatchExecutor,
    BigInt,

    Binary,
    BuiltMutation,
    BuiltQuery,
    ChangeType,

    ColumnStatistics,
    ColumnType,
    // Performance & Security
    ComplexityAnalyzer,
    ComplexityMetrics,
    ConstraintInfo,

    DataLoader,
    DataType,
    // Schema types
    DatabaseSchema,
    // Scalar types
    DateTime,
    DepthLimitExtension,

    FieldInfo,
    FieldValue,
    // Input types
    FilterCondition,
    FilterEvaluator,
    FilterOp,
    // Core engine
    GraphQLEngine,

    Heartbeat,

    HistogramBucket,

    IndexInfo,
    IsolationLevel,
    JoinInput,
    JoinType,
    Json,
    Metrics,
    MetricsCollector,
    MutationBuilder,
    MutationError,
    MutationOperation,
    MutationResult,
    MutationRoot,
    MutationSuccess,
    OptimizationSuggestions,
    OrderBy,
    // Pagination
    PageInfo,
    PerformanceExtension,
    PersistedQueries,
    PlanOperation,

    // Utilities & Helpers
    QueryBuilder,
    QueryCache,
    QueryChange,
    QueryError,
    QueryOptimizer,
    // Query planning
    QueryPlan,
    // Result types
    QueryResult,
    // Root types
    QueryRoot,
    QuerySuccess,
    RateLimit,
    RateLimiter as GraphQLRateLimiter,
    RequestValidator,
    ResultFormatter,
    RowChange,
    RowConnection,

    RowDeleted,
    RowEdge,
    RowInserted,
    RowType,
    RowUpdated,
    SchemaConfig,

    SchemaIntrospector,
    SearchMatch,

    // Search
    SearchResult,
    // Enums
    SortOrder,
    SubscriptionInfo,

    // Subscription management
    SubscriptionManager,
    SubscriptionRoot,

    // Subscription events
    TableChange,
    // Statistics
    TableStatistics,
    TableStats,
    TableType,
    TransactionExecutionResult,

    TransactionOpType,

    TransactionOperation,
    TransactionResult,
    TypeInfo,
    TypeKind,
    WhereClause,
};

// Enterprise Integration exports
pub use enterprise::{
    ApiGatewayCoordinator,
    ApiVersionManager,
    BackwardCompatibilityLayer,
    BatchRequest,
    BatchRequestHandler,
    BatchResponse,
    CentralizedLogger,
    CircuitBreaker,
    CircuitBreakerCoordinator,
    CircuitState,

    ConfigurationAggregator,

    ConnectionQuotaManager,
    // Cross-Cutting Concerns
    CorrelationId,
    DependencyContainer,
    DistributedTracingManager,
    // Main integrator
    EnterpriseIntegrator,
    ErrorHandlingPolicy,
    FeatureFlagManager,
    HealthCheck,
    HealthCheckStatus,
    HotReloadManager,
    IntegratorConfig,

    IoOperation,
    IoOperationType,
    IoScheduler,
    LogEntry,
    LogLevel,
    MemoryBudgetAllocator,
    PriorityManager,
    RateLimiter,

    RecoveryOrchestrator,
    // Resource Orchestration
    ResourceBudget,
    ResourceContentionHandler,

    ResourceOrchestrator,
    ResourceUsageSnapshot,
    RetryPolicyExecutor,
    RollingUpgradeCoordinator,
    ServiceLifecycleHandler,
    ServiceMetadata,
    ServiceRegistration,
    // Service Registry
    ServiceRegistry,
    ServiceState,
    ShutdownCoordinator,
    ShutdownPhase,
    Span,
    StartupOrchestrator,
    StartupPhase,
    StatePersistenceManager,
    // Health and Status
    SystemHealthStatus,
    SystemLifecycleManager,
    // Lifecycle Management
    SystemState,
    ThreadPoolCoordinator,
    TraceContext,
    // API Facade
    UnifiedApiRequest,
    UnifiedApiResponse,
    UpgradeState,

    VersionCompatibilityChecker,
};
