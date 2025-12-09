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
    SubscriptionInfo
};
pub use complexity::*;
pub use engine::*;
pub use schema::*;
pub use builders::*;
pub use helpers::*;
