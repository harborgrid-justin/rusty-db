use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("SQL parsing error: {0}")]
    SqlParse(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Catalog error: {0}")]
    Catalog(String),

    #[error("Index error: {0}")]
    Index(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Lock timeout")]
    LockTimeout,

    #[error("Lock error: {0}")]
    LockError(String),

    #[error("Service unavailable: {0}")]
    Unavailable(String),

    #[error("Deadlock detected")]
    Deadlock,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Backup error: {0}")]
    BackupError(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("Replication error: {0}")]
    Replication(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Out of memory: {0}")]
    OutOfMemory(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Limit exceeded: {0}")]
    LimitExceeded(String),

    #[error("IO error: {0}")]
    IOError(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Cluster error: {0}")]
    Cluster(String),

    #[error("Buffer error: {0}")]
    Buffer(String),

    #[error("SIMD error: {0}")]
    Simd(String),

    #[error("Concurrent operation error: {0}")]
    Concurrent(String),

    #[error("Circuit breaker open: {0}")]
    CircuitBreakerOpen(String),

    #[error("Bulkhead full: {0}")]
    BulkheadFull(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Injection attempt detected: {0}")]
    InjectionAttempt(String),

    #[error("Invalid request")]
    InvalidRequest,

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),

    #[error("Page not found: {0}")]
    PageNotFound(String),

    #[error("Other error: {0}")]
    Other(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Recovery error: {0}")]
    Recovery(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Corruption error: {0}")]
    CorruptionError(String),

    #[error("Deadlock detected: {0}")]
    DeadlockDetected(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

impl DbError {
    pub(crate) fn not_supported(p0: String) -> DbError {
        DbError::NotImplemented(p0)
    }
}

impl Clone for DbError {
    fn clone(&self) -> Self {
        match self {
            DbError::Io(e) => DbError::IoError(e.to_string()),
            DbError::SqlParse(s) => DbError::SqlParse(s.clone()),
            DbError::Transaction(s) => DbError::Transaction(s.clone()),
            DbError::Storage(s) => DbError::Storage(s.clone()),
            DbError::Catalog(s) => DbError::Catalog(s.clone()),
            DbError::Index(s) => DbError::Index(s.clone()),
            DbError::Execution(s) => DbError::Execution(s.clone()),
            DbError::Network(s) => DbError::Network(s.clone()),
            DbError::Serialization(s) => DbError::Serialization(s.clone()),
            DbError::LockTimeout => DbError::LockTimeout,
            DbError::LockError(s) => DbError::LockError(s.clone()),
            DbError::Unavailable(s) => DbError::Unavailable(s.clone()),
            DbError::Deadlock => DbError::Deadlock,
            DbError::NotFound(s) => DbError::NotFound(s.clone()),
            DbError::AlreadyExists(s) => DbError::AlreadyExists(s.clone()),
            DbError::InvalidInput(s) => DbError::InvalidInput(s.clone()),
            DbError::InvalidOperation(s) => DbError::InvalidOperation(s.clone()),
            DbError::NotImplemented(s) => DbError::NotImplemented(s.clone()),
            DbError::Internal(s) => DbError::Internal(s.clone()),
            DbError::Validation(s) => DbError::Validation(s.clone()),
            DbError::BackupError(s) => DbError::BackupError(s.clone()),
            DbError::Runtime(s) => DbError::Runtime(s.clone()),
            DbError::Replication(s) => DbError::Replication(s.clone()),
            DbError::InvalidArgument(s) => DbError::InvalidArgument(s.clone()),
            DbError::ResourceExhausted(s) => DbError::ResourceExhausted(s.clone()),
            DbError::SerializationError(s) => DbError::SerializationError(s.clone()),
            DbError::Encryption(s) => DbError::Encryption(s.clone()),
            DbError::IoError(s) => DbError::IoError(s.clone()),
            DbError::OutOfMemory(s) => DbError::OutOfMemory(s.clone()),
            DbError::TransactionError(s) => DbError::TransactionError(s.clone()),
            DbError::LimitExceeded(s) => DbError::LimitExceeded(s.clone()),
            DbError::IOError(s) => DbError::IOError(s.clone()),
            DbError::Configuration(s) => DbError::Configuration(s.clone()),
            DbError::PermissionDenied(s) => DbError::PermissionDenied(s.clone()),
            DbError::Timeout(s) => DbError::Timeout(s.clone()),
            DbError::Cluster(s) => DbError::Cluster(s.clone()),
            DbError::Buffer(s) => DbError::Buffer(s.clone()),
            DbError::Simd(s) => DbError::Simd(s.clone()),
            DbError::Concurrent(s) => DbError::Concurrent(s.clone()),
            DbError::CircuitBreakerOpen(s) => DbError::CircuitBreakerOpen(s.clone()),
            DbError::BulkheadFull(s) => DbError::BulkheadFull(s.clone()),
            DbError::Security(s) => DbError::Security(s.clone()),
            DbError::InjectionAttempt(s) => DbError::InjectionAttempt(s.clone()),
            DbError::InvalidRequest => DbError::InvalidRequest,
            DbError::InvalidState(s) => DbError::InvalidState(s.clone()),
            DbError::QuotaExceeded(s) => DbError::QuotaExceeded(s.clone()),
            DbError::PageNotFound(s) => DbError::PageNotFound(s.clone()),
            DbError::Other(s) => DbError::Other(s.clone()),
            DbError::Authentication(s) => DbError::Authentication(s.clone()),
            DbError::Authorization(s) => DbError::Authorization(s.clone()),
            DbError::Compression(s) => DbError::Compression(s.clone()),
            DbError::Recovery(s) => DbError::Recovery(s.clone()),
            DbError::Memory(s) => DbError::Memory(s.clone()),
            DbError::CorruptionError(s) => DbError::CorruptionError(s.clone()),
            DbError::DeadlockDetected(s) => DbError::DeadlockDetected(s.clone()),
            DbError::Conflict(s) => DbError::Conflict(s.clone()),
            DbError::ConstraintViolation(s) => DbError::ConstraintViolation(s.clone()),
            DbError::ParseError(s) => DbError::ParseError(s.clone()),
        }
    }
}

pub type Result<T> = std::result::Result<T, DbError>;

// Error conversions for common error types

impl From<bincode::error::EncodeError> for DbError {
    fn from(e: bincode::error::EncodeError) -> Self {
        DbError::Serialization(e.to_string())
    }
}

impl From<bincode::error::DecodeError> for DbError {
    fn from(e: bincode::error::DecodeError) -> Self {
        DbError::Serialization(e.to_string())
    }
}

impl From<serde_json::Error> for DbError {
    fn from(e: serde_json::Error) -> Self {
        DbError::Serialization(e.to_string())
    }
}

// Note: bson serde features not enabled, removed conversions
// impl From<bson::ser::Error> for DbError {
//     fn from(e: bson::ser::Error) -> Self {
//         DbError::Serialization(e.to_string())
//     }
// }

// impl From<bson::de::Error> for DbError {
//     fn from(e: bson::de::Error) -> Self {
//         DbError::Serialization(e.to_string())
//     }
// }
