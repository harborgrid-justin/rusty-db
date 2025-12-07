use serde::{Deserialize, Serialize};
use crate::execution::QueryResult;

/// Client request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    Query { sql: String },
    BeginTransaction,
    Commit,
    Rollback,
    Ping,
}

/// Server response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    QueryResult(QueryResult),
    TransactionId(u64),
    Ok,
    Error(String),
    Pong,
}
