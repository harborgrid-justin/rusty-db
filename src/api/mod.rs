//! # API Layer
//!
//! Comprehensive API layer exposing all database functionality via multiple interfaces.
//!
//! ## Modules
//!
//! - **rest**: REST API endpoints for database operations
//! - **graphql_api**: GraphQL API layer with real-time subscriptions
//! - **monitoring**: Monitoring, metrics, and health check APIs
//! - **gateway**: API gateway for request routing and management
//! - **enterprise_integration**: Master integration layer coordinating all enterprise modules
//!
//! ## Features
//!
//! - Core database operations (queries, transactions, CRUD)
//! - Administration endpoints (config, backup, maintenance)
//! - Monitoring and metrics (Prometheus, health checks)
//! - Pool and connection management
//! - Cluster management
//! - OpenAPI/Swagger documentation
//! - Request validation and response pagination
//! - Rate limiting and CORS support
//! - API Gateway with authentication, authorization, and security
//! - Enterprise service integration and orchestration
//!
//! ## Usage
//!
//! ```rust,no_run
//! use rusty_db::api::rest_api::{RestApiServer, ApiConfig};
//! use rusty_db::api::enterprise_integration::{EnterpriseIntegrator, IntegratorConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Start enterprise integration layer
//!     let integrator_config = IntegratorConfig::default();
//!     let integrator = EnterpriseIntegrator::new(integrator_config).await.unwrap();
//!     integrator.start().await.unwrap();
//!
//!     // Start REST API
//!     let config = ApiConfig::default();
//!     let server = RestApiServer::new(config).await.unwrap();
//!     server.run("0.0.0.0:8080").await.unwrap();
//! }
//! ```

pub mod rest_api;
pub mod graphql_api;
pub mod monitoring;
pub mod gateway;
pub mod enterprise_integration;

pub use rest_api::{
    RestApiServer,
    ApiConfig,
    ApiError,
    ApiResult,
};

pub use monitoring::{
    MonitoringApi,
    MonitoringConfig,
    MetricsRegistry,
    HealthCheckCoordinator,
    AlertManager,
    DashboardManager,
    PrometheusExporter,
    TimeSeriesDatabase,
    HealthStatus,
    HealthCheckResult,
    Alert,
    AlertSeverity,
    Dashboard,
    TimeSeriesQuery,
    TimeSeriesResult,
    ExportFormat,
};

pub use gateway::{
    ApiGateway,
    GatewayConfig,
    ApiRequest,
    ApiResponse,
    Route,
    BackendService,
    Protocol,
    Session,
    RateLimitConfig,
    RateLimitType,
    GatewayMetrics,
};

pub use graphql_api::{
    // Schema building
    build_schema,
    build_schema_with_config,
    SchemaConfig,

    // Root types
    QueryRoot,
    MutationRoot,
    SubscriptionRoot,

    // Core engine
    GraphQLEngine,

    // Schema types
    DatabaseSchema,
    TableType,
    ColumnType,
    RowType,
    FieldValue,
    DataType,
    IndexInfo,
    ConstraintInfo,

    // Scalar types
    DateTime,
    Json,
    Binary,
    BigInt,

    // Enums
    SortOrder,
    FilterOp,
    AggregateFunc,
    JoinType,
    IsolationLevel,
    ChangeType,

    // Input types
    FilterCondition,
    WhereClause,
    OrderBy,
    AggregateInput,
    JoinInput,
    TransactionOperation,
    TransactionOpType,

    // Result types
    QueryResult,
    QuerySuccess,
    QueryError,
    MutationResult,
    MutationSuccess,
    MutationError,
    AggregateResult,
    TransactionResult,
    TransactionExecutionResult,

    // Pagination
    PageInfo,
    RowEdge,
    RowConnection,

    // Statistics
    TableStatistics,
    ColumnStatistics,
    HistogramBucket,

    // Search
    SearchResult,
    SearchMatch,

    // Query planning
    QueryPlan,
    PlanOperation,

    // Subscription events
    TableChange,
    RowInserted,
    RowUpdated,
    RowDeleted,
    RowChange,
    AggregateChange,
    QueryChange,
    Heartbeat,

    // Subscription management
    SubscriptionManager,
    SubscriptionInfo,

    // Performance & Security
    ComplexityAnalyzer,
    ComplexityMetrics,
    RateLimiter as GraphQLRateLimiter,
    RateLimit,
    AuthorizationContext,
    QueryCache,
    DataLoader,
    PersistedQueries,
    PerformanceExtension,
    DepthLimitExtension,

    // Utilities & Helpers
    QueryBuilder,
    BuiltQuery,
    MutationBuilder,
    BuiltMutation,
    MutationOperation,
    SchemaIntrospector,
    TypeInfo,
    TypeKind,
    FieldInfo,
    ArgumentInfo,
    QueryOptimizer,
    OptimizationSuggestions,
    TableStats,
    RequestValidator,
    BatchExecutor,
    ResultFormatter,
    FilterEvaluator,
    MetricsCollector,
    Metrics,
};

// Enterprise Integration exports
pub use enterprise_integration::{
    // Main integrator
    EnterpriseIntegrator,
    IntegratorConfig,

    // Service Registry
    ServiceRegistry,
    ServiceMetadata,
    ServiceState,
    ServiceRegistration,
    ServiceLifecycleHandler,
    HealthCheck,
    HealthCheckStatus,
    DependencyContainer,
    FeatureFlagManager,
    VersionCompatibilityChecker,
    ConfigurationAggregator,

    // Cross-Cutting Concerns
    CorrelationId,
    TraceContext,
    Span,
    DistributedTracingManager,
    CentralizedLogger,
    LogLevel,
    LogEntry,
    ErrorHandlingPolicy,
    RetryPolicyExecutor,
    CircuitBreaker,
    CircuitBreakerCoordinator,
    CircuitState,

    // Resource Orchestration
    ResourceBudget,
    ResourceOrchestrator,
    MemoryBudgetAllocator,
    ConnectionQuotaManager,
    ThreadPoolCoordinator,
    IoScheduler,
    IoOperation,
    IoOperationType,
    PriorityManager,
    ResourceContentionHandler,

    // API Facade
    UnifiedApiRequest,
    UnifiedApiResponse,
    BatchRequestHandler,
    BatchRequest,
    BatchResponse,
    ApiVersionManager,
    BackwardCompatibilityLayer,
    ApiGatewayCoordinator,
    RateLimiter,

    // Lifecycle Management
    SystemState,
    SystemLifecycleManager,
    StartupOrchestrator,
    ShutdownCoordinator,
    HotReloadManager,
    RollingUpgradeCoordinator,
    StatePersistenceManager,
    RecoveryOrchestrator,
    StartupPhase,
    ShutdownPhase,
    UpgradeState,

    // Health and Status
    SystemHealthStatus,
    ResourceUsageSnapshot,
};
