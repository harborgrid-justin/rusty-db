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
    
    #[error("Deadlock detected")]
    Deadlock,
}

pub type Result<T> = std::result::Result<T, DbError>;
