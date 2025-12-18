use crate::execution::QueryResult;
use serde::{Deserialize, Serialize};

// ============================================================================
// Constants - Bounds for Network Protocol
// ============================================================================

/// Maximum SQL query length (1MB) - prevents memory exhaustion from unbounded queries
/// SECURITY ISSUE FIXED: EA5-U1 - Unbounded SQL String
/// Previous code had no limit on SQL string size in Request::Query
/// See: diagrams/06_network_api_flow.md - Section 5 (Open-Ended Data Segments)
pub const MAX_SQL_LENGTH: usize = 1_048_576; // 1MB

/// Maximum bincode deserialization size (16MB) - prevents memory exhaustion
/// SECURITY ISSUE FIXED: EA5-U8 - Bincode No Size Limit
/// Previous code had no size limit on bincode deserialization (line 138 in server.rs)
/// Attacker could send arbitrarily large serialized objects
/// See: diagrams/06_network_api_flow.md - Issue #3.3
pub const MAX_BINCODE_SIZE: usize = 16_777_216; // 16MB

// Client request
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum Request {
    /// Execute SQL query
    /// NOTE: SQL string should be validated against MAX_SQL_LENGTH before processing
    /// to prevent memory exhaustion attacks
    Query { sql: String },
    BeginTransaction,
    Commit,
    Rollback,
    Ping,
}

// Server response
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum Response {
    QueryResult(QueryResult),
    TransactionId(u64),
    Ok,
    Error(String),
    Pong,
}
