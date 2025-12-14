// GraphQL API Module
//
// Comprehensive GraphQL API implementation for RustyDB

pub mod builders;
pub mod cluster_subscriptions;
pub mod complexity;
pub mod config_queries;
pub mod config_types;
pub mod ddl_subscriptions;
pub mod engine;
pub mod enterprise_subscriptions;
pub mod helpers;
pub mod ml_analytics_subscriptions;
pub mod models;
pub mod monitoring_types;
pub mod mutations;
pub mod performance_subscriptions;
pub mod queries;
pub mod query_subscriptions;
pub mod schema;
pub mod security_subscriptions;
pub mod session_subscriptions;
pub mod subscriptions;
pub mod transaction_subscriptions;
pub mod types;
pub mod websocket_transport;

// Re-export main types and functions
// Note: types::DatabaseSchema is shadowed by models::DatabaseSchema
pub use builders::*;
pub use complexity::*;
pub use engine::*;
pub use helpers::*;
pub use models::*;
pub use monitoring_types::{
    ActiveQuery, ActiveTransaction, Alert, AlertSeverity, BufferPoolStats, ClusterConfig,
    ClusterNode, ClusterTopology, ComponentHealth, Connection, ConnectionPool, Deadlock,
    HealthStatus, IoStats, Lock, MetricsResponse, MvccStatus, Partition, PerformanceData,
    PoolStats, QueryStats, ReplicationStatus, Role, ServerConfig, ServerInfo, Session,
    SessionStats, SlowQuery, StorageStatus, Tablespace, User,
};
pub use mutations::{
    BatchStringFunctionResult, ColumnDefinitionInput, ConstraintInput, ConstraintTypeEnum,
    DdlError, DdlResult, DdlSuccess, MutationRoot, ParameterMode, ProcedureError,
    ProcedureParameter, ProcedureResult, ProcedureSuccess, StringFunctionInput,
    StringFunctionResult, StringFunctionTypeEnum, TransactionExecutionResult, TransactionOpType,
    TransactionOperation, TransactionResult,
};
pub use queries::{JoinInput, PlanOperation, QueryPlan, QueryRoot, SearchMatch, SearchResult};
pub use schema::*;
pub use security_subscriptions::{
    AuditLogEvent, AuditSeverity, AuthAction, AuthenticationEvent, AuthorizationEvent, AuthzAction,
    CircuitBreakerEvent, CircuitState, EncryptionAction, EncryptionEvent, InsiderThreatEvent,
    MemoryEventType, MemoryHardeningEvent, RateLimitEvent,
    SecurityMetrics as SecurityMetricsSubscription, SecurityPosture, SecuritySubscriptionRoot,
    ThreatLevel, ThreatType,
};
pub use subscriptions::{
    AggregateChange, ChangeType, Heartbeat, MetricType, QueryChange, QueryExecutionEvent,
    QueryExecutionStatus, ReplicationRole, ReplicationState, ReplicationStatusEvent, RowChange,
    RowDeleted, RowInserted, RowUpdated, SubscriptionInfo, SubscriptionManager, SubscriptionRoot,
    SystemMetrics, TableChange, TableModification,
};
pub use types::{
    AggregateFunc, BigInt, Binary, DataType, DateTime, FilterOp, IsolationLevel, JoinType, Json,
    SortOrder,
};
pub use websocket_transport::{
    graphql_ws_handler, graphql_ws_handler_with_config, ConnectionInitPayload, ConnectionMetrics,
    ErrorLocation, GraphQLError, GraphQLWsMessage, SubscribePayload, WebSocketConfig,
    WebSocketSubscriptionManager,
};

// DDL Subscriptions
pub use ddl_subscriptions::{
    DdlOperationType, DdlSubscriptionRoot, PartitionOperation, PartitionOperationEvent,
    PartitionOperationStatus, PartitionType, SchemaChangeEvent, SchemaObjectType,
};

// Performance Subscriptions
pub use performance_subscriptions::{
    ActiveQueryEvent, AlertCategory, AlertSeverity as PerformanceAlertSeverity,
    BufferPoolMetricsEvent, HealthMetric, HealthStatus as PerformanceHealthStatus,
    HealthStatusChangeEvent, IoStatisticsEvent, PerformanceSubscriptionRoot, PlanChangeReason,
    QueryPlanChangeEvent, QueryState, SlowQueryEvent, StorageStatus, StorageStatusChangeEvent,
    SystemAlertEvent,
};

// Session Subscriptions
pub use session_subscriptions::{
    ConnectionEvent, ConnectionHealthStatus, ConnectionPoolStateEvent, ConnectionState,
    PoolEventType, SessionEventType, SessionLifecycleEvent, SessionResourceEvent,
    SessionSubscriptionRoot,
};

// Transaction Subscriptions
pub use transaction_subscriptions::{
    DeadlockEventGql, LockEventGql, MvccVersionEvent, TransactionLifecycleEvent, TransactionStats,
    TransactionSubscriptions, TwoPhaseCommitEventGql, WalOperationEvent,
};

// Cluster Subscriptions
pub use cluster_subscriptions::{
    BlockMode, CacheFusionEvent, CacheFusionEventType, ClusterHealthEvent, ClusterStatus,
    ClusterSubscriptionRoot, ConflictEvent, ConflictType, FailoverEvent, FailoverType,
    LeaderElectionEvent, LockEvent as ClusterLockEvent, LockEventType, LockType, NodeRole,
    NodeStatus, NodeStatusEvent, ParallelQueryEvent, QueryEventType, RebalancePhase,
    RebalanceProgressEvent, RecoveryEvent, RecoveryEventType, RecoveryPhase, ReplicaStatus,
    ReplicaStatusEvent, ReplicationLagEvent,
};

// Enterprise Subscriptions
pub use enterprise_subscriptions::{
    AutonomousTuningEvent, BackupProgressUpdate, BlockchainVerificationEvent, CEPPatternMatch,
    EnterpriseSubscriptions, FlashbackOperation, MultiTenantEvent, SelfHealingEvent,
};

// ML/Analytics Subscriptions
pub use ml_analytics_subscriptions::{
    AnalyticsQueryProgress, DocumentChangeEvent, GraphAlgorithmProgress, GraphTraversalUpdate,
    MLAnalyticsSubscription, MLModelLifecycleEvent, MLPredictionEvent, MLTrainingProgress,
    SpatialQueryUpdate, TimeSeriesAnomalyAlert, WorkloadRecommendation,
};

// Query Execution Subscriptions
pub use query_subscriptions::{
    AdaptiveOptimizationEvent, CompilationPhase, CompilationStatus, CostEstimate,
    CteEvaluationEvent, CteEvaluationType, ExecutionPlanNode, OptimizerHintEvent,
    ParallelWorkerEvent, PlanChangeEvent, QueryCompilationEvent, QueryExecutionSubscription,
    QueryProgressUpdate, ResultChunk, WorkerEventType,
};
