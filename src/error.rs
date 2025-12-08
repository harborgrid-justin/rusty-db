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
}

pub type Result<T> = std::result::Result<T, DbError>;
