// FFI Type Definitions
//
// C-compatible type definitions for the RustyDB FFI layer.
// All types are designed to be safely passed across the FFI boundary.

use std::os::raw::{c_char, c_int};
use std::sync::{Arc, Mutex};
use crate::error::DbError;

/// Opaque handle to a database connection
///
/// This is an opaque pointer type that hides the Rust implementation details
/// from C callers. The actual connection state is stored in a Box.
#[repr(C)]
pub struct rustydb_handle_t {
    _private: [u8; 0],
}

/// Internal representation of a database handle
///
/// This struct contains the actual connection state and error information.
/// It's wrapped in a Box and exposed as an opaque pointer to C callers.
pub struct RustyDbHandle {
    /// Connection identifier (for future use with actual connection pooling)
    pub connection_id: u64,

    /// Last error that occurred on this handle
    pub last_error: Option<DbError>,

    /// Last error message (cached for C API)
    pub last_error_message: Option<String>,

    /// Last error code (cached for C API)
    pub last_error_code: i32,

    /// Connection state
    pub state: ConnectionState,

    /// Transaction ID if in transaction, None otherwise
    pub transaction_id: Option<u64>,

    /// Connection string used to establish this connection
    pub connection_string: String,
}

/// Connection state enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection is active and ready
    Active,

    /// Connection is in a transaction
    InTransaction,

    /// Connection is closed
    Closed,

    /// Connection encountered an error
    Error,
}

impl RustyDbHandle {
    /// Create a new handle from a connection string
    pub fn new(connection_string: String) -> Self {
        Self {
            connection_id: Self::generate_connection_id(),
            last_error: None,
            last_error_message: None,
            last_error_code: 0,
            state: ConnectionState::Active,
            transaction_id: None,
            connection_string,
        }
    }

    /// Generate a unique connection ID
    fn generate_connection_id() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Set the last error on this handle
    pub fn set_error(&mut self, error: DbError) {
        let error_code = Self::error_to_code(&error);
        let error_message = error.to_string();

        self.last_error_code = error_code;
        self.last_error_message = Some(error_message);
        self.last_error = Some(error);
        self.state = ConnectionState::Error;
    }

    /// Clear the last error
    pub fn clear_error(&mut self) {
        self.last_error = None;
        self.last_error_message = None;
        self.last_error_code = 0;
        if self.state == ConnectionState::Error {
            self.state = ConnectionState::Active;
        }
    }

    /// Convert DbError to error code
    pub fn error_to_code(error: &DbError) -> i32 {
        match error {
            DbError::Io(_) => 1000,
            DbError::SqlParse(_) => 2000,
            DbError::Transaction(_) => 3000,
            DbError::Storage(_) => 4000,
            DbError::Catalog(_) => 5000,
            DbError::Index(_) => 6000,
            DbError::Execution(_) => 7000,
            DbError::Network(_) => 8000,
            DbError::Serialization(_) => 9000,
            DbError::LockTimeout => 3001,
            DbError::LockError(_) => 3002,
            DbError::Unavailable(_) => 8001,
            DbError::Deadlock => 3003,
            DbError::NotFound(_) => 4001,
            DbError::AlreadyExists(_) => 4002,
            DbError::InvalidInput(_) => 2001,
            DbError::InvalidOperation(_) => 2002,
            DbError::NotImplemented(_) => 9999,
            DbError::Internal(_) => 9998,
            DbError::Validation(_) => 2003,
            DbError::BackupError(_) => 10000,
            DbError::Runtime(_) => 9997,
            DbError::Replication(_) => 11000,
            DbError::InvalidArgument(_) => 2004,
            DbError::ResourceExhausted(_) => 9996,
            DbError::BsonError(_) => 9001,
            DbError::Encryption(_) => 12000,
            DbError::OutOfMemory(_) => 9995,
            DbError::LimitExceeded(_) => 9994,
            DbError::Configuration(_) => 13000,
            DbError::PermissionDenied(_) => 14000,
            DbError::Authentication(_) => 14001,
            DbError::Timeout(_) => 8002,
            DbError::Cluster(_) => 8003,
            DbError::Security(_) => 14002,
            DbError::ConstraintViolation(_) => 4003,
            DbError::Buffer(_) => 2005,
            DbError::Simd(_) => 9993,
            DbError::Concurrent(_) => 9992,
            DbError::CircuitBreakerOpen(_) => 9991,
            DbError::ParseError(_) => 2006,
            DbError::BulkheadFull(_) => 9990,
            DbError::InjectionAttempt(_) => 14003,
            DbError::InvalidRequest => 2007,
            DbError::InvalidState(_) => 2008,
            DbError::QuotaExceeded(_) => 9989,
            DbError::PageNotFound(_) => 4004,
            DbError::Other(_) => 9988,
            DbError::Authorization(_) => 14004,
            DbError::Compression(_) => 9002,
            DbError::Recovery(_) => 10001,
            DbError::Memory(_) => 9987,
            DbError::Corruption(_) => 4005,
            DbError::Conflict(_) => 4006,
        }
    }
}

/// Opaque handle to a query result
///
/// Represents the result set from a query execution.
#[repr(C)]
pub struct rustydb_result_t {
    _private: [u8; 0],
}

/// Internal representation of a query result
///
/// Contains the actual result data and metadata.
pub struct RustyDbResult {
    /// Number of rows affected or returned
    pub rows_affected: i64,

    /// Result data as JSON (for simplicity in v0.6)
    pub data: Option<String>,

    /// Column names
    pub column_names: Vec<String>,

    /// Whether this is a successful result
    pub success: bool,

    /// Error if the query failed
    pub error: Option<DbError>,
}

impl RustyDbResult {
    /// Create a successful result
    pub fn success(rows_affected: i64, data: Option<String>, column_names: Vec<String>) -> Self {
        Self {
            rows_affected,
            data,
            column_names,
            success: true,
            error: None,
        }
    }

    /// Create an error result
    pub fn error(error: DbError) -> Self {
        Self {
            rows_affected: 0,
            data: None,
            column_names: Vec::new(),
            success: false,
            error: Some(error),
        }
    }
}

/// FFI success code
pub const RUSTYDB_OK: c_int = 0;

/// FFI error code
pub const RUSTYDB_ERROR: c_int = -1;

/// Convert a Rust string to a C string (caller must free)
///
/// # Safety
/// The returned pointer must be freed by calling rustydb_free_string
pub unsafe fn string_to_c_char(s: String) -> *mut c_char {
    let c_str = std::ffi::CString::new(s).unwrap_or_else(|_| {
        std::ffi::CString::new("Error: String contains null bytes").unwrap()
    });
    c_str.into_raw()
}

/// Convert a C string to a Rust String
///
/// # Safety
/// The pointer must be a valid null-terminated C string
pub unsafe fn c_char_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    let c_str = std::ffi::CStr::from_ptr(ptr);
    c_str.to_str().ok().map(|s| s.to_string())
}

/// Check if a pointer is null and return an appropriate error code
pub fn check_null_ptr<T>(ptr: *const T, param_name: &str) -> Result<(), String> {
    if ptr.is_null() {
        Err(format!("Null pointer: {}", param_name))
    } else {
        Ok(())
    }
}
