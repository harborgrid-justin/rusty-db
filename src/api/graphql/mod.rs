// GraphQL API Module
//
// Comprehensive GraphQL API implementation for RustyDB

pub mod types;
pub mod models;
pub mod queries;
pub mod mutations;
pub mod subscriptions;
pub mod complexity;
pub mod engine;
pub mod schema;
pub mod builders;
pub mod helpers;
pub mod monitoring_types;
pub mod websocket_transport;

// Re-export main types and functions
// Note: types::DatabaseSchema is shadowed by models::DatabaseSchema
pub use types::{DateTime, Json, Binary, BigInt, DataType, SortOrder, FilterOp, AggregateFunc, JoinType, IsolationLevel};
pub use models::*;
pub use queries::{QueryRoot, JoinInput, SearchResult, SearchMatch, QueryPlan, PlanOperation};
pub use mutations::{
    MutationRoot,
    TransactionOperation,
    TransactionOpType,
    TransactionResult,
    TransactionExecutionResult,
    DdlResult,
    DdlSuccess,
    DdlError,
    ProcedureParameter,
    ParameterMode,
    ProcedureResult,
    ProcedureSuccess,
    ProcedureError,
    ColumnDefinitionInput,
    ConstraintInput,
    ConstraintTypeEnum,
    StringFunctionInput,
    StringFunctionTypeEnum,
    StringFunctionResult,
    BatchStringFunctionResult,
};
pub use subscriptions::{
    SubscriptionRoot,
    ChangeType,
    TableChange,
    RowInserted,
    RowUpdated,
    RowDeleted,
    RowChange,
    AggregateChange,
    QueryChange,
    Heartbeat,
    SubscriptionManager,
    SubscriptionInfo,
    QueryExecutionEvent,
    QueryExecutionStatus,
    TableModification,
    SystemMetrics,
    MetricType,
    ReplicationStatusEvent,
    ReplicationRole,
    ReplicationState,
};
pub use complexity::*;
pub use engine::*;
pub use schema::*;
pub use builders::*;
pub use helpers::*;
pub use monitoring_types::{
    MetricsResponse,
    SessionStats,
    QueryStats,
    PerformanceData,
    ActiveQuery,
    SlowQuery,
    ClusterNode,
    ClusterTopology,
    ReplicationStatus,
    ClusterConfig,
    StorageStatus,
    BufferPoolStats,
    Tablespace,
    IoStats,
    ActiveTransaction,
    Lock,
    Deadlock,
    MvccStatus,
    ServerConfig,
    User,
    Role,
    HealthStatus,
    ComponentHealth,
    ConnectionPool,
    PoolStats,
    Connection,
    Session,
    Partition,
    AlertSeverity,
    Alert,
    ServerInfo,
};
pub use websocket_transport::{
    GraphQLWsMessage,
    ConnectionInitPayload,
    SubscribePayload,
    GraphQLError,
    ErrorLocation,
    WebSocketConfig,
    WebSocketSubscriptionManager,
    ConnectionMetrics,
    graphql_ws_handler,
    graphql_ws_handler_with_config,
};
