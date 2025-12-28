// FFI C API Implementation
//
// Provides the main C-compatible API functions for RustyDB.
// All functions use #[no_mangle] and extern "C" for C compatibility.

use std::os::raw::{c_char, c_int};
use std::ptr;
use crate::error::DbError;
use super::types::{
    rustydb_handle_t, rustydb_result_t, RustyDbHandle, RustyDbResult,
    ConnectionState, c_char_to_string, string_to_c_char,
    RUSTYDB_OK, RUSTYDB_ERROR,
};

// ============================================================================
// Connection Management
// ============================================================================

/// Connect to a RustyDB database
///
/// Creates a new database connection using the provided connection string.
///
/// # Parameters
/// - `connection_string`: Null-terminated C string with connection parameters
///   Format: "host=localhost;port=5432;database=mydb;user=admin;password=secret"
///
/// # Returns
/// - Non-null pointer to rustydb_handle_t on success
/// - NULL on failure
///
/// # Memory Management
/// The returned handle must be freed by calling rustydb_disconnect().
///
/// # Safety
/// The connection_string pointer must be a valid null-terminated C string.
/// The caller is responsible for freeing the returned handle.
///
/// # Example (C)
/// ```c
/// rustydb_handle_t* handle = rustydb_connect("host=localhost;port=5432");
/// if (handle == NULL) {
///     fprintf(stderr, "Failed to connect\n");
///     return 1;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn rustydb_connect(connection_string: *const c_char) -> *mut rustydb_handle_t {
    // Validate input
    if connection_string.is_null() {
        return ptr::null_mut();
    }

    // Convert C string to Rust String
    let conn_str = match c_char_to_string(connection_string) {
        Some(s) => s,
        None => return ptr::null_mut(),
    };

    // Create a new handle
    // In a real implementation, this would establish an actual connection
    // For v0.6, we create a mock connection
    let handle = RustyDbHandle::new(conn_str);

    // Box the handle and convert to raw pointer
    let boxed_handle = Box::new(handle);
    Box::into_raw(boxed_handle) as *mut rustydb_handle_t
}

/// Disconnect from the database and free the handle
///
/// Closes the database connection and releases all associated resources.
/// After this call, the handle pointer is invalid and must not be used.
///
/// # Parameters
/// - `handle`: Pointer to a valid rustydb_handle_t
///
/// # Safety
/// The handle pointer must be valid and have been returned by rustydb_connect().
/// The handle must not be used after this call.
/// Calling this function with NULL is safe (no-op).
///
/// # Example (C)
/// ```c
/// rustydb_disconnect(handle);
/// handle = NULL; // Good practice
/// ```
#[no_mangle]
pub unsafe extern "C" fn rustydb_disconnect(handle: *mut rustydb_handle_t) {
    if handle.is_null() {
        return;
    }

    // Convert back to Box and let it drop
    let handle_ptr = handle as *mut RustyDbHandle;
    let boxed_handle = Box::from_raw(handle_ptr);

    // In a real implementation, we would close the connection here
    // For now, the Box will be dropped and memory freed
    drop(boxed_handle);
}

// ============================================================================
// Query Execution
// ============================================================================

/// Execute a SQL query
///
/// Executes the provided SQL statement and returns the results.
///
/// # Parameters
/// - `handle`: Pointer to a valid rustydb_handle_t
/// - `sql`: Null-terminated C string containing the SQL query
///
/// # Returns
/// - Non-null pointer to rustydb_result_t on success
/// - NULL on failure (check rustydb_error_message for details)
///
/// # Memory Management
/// The returned result must be freed by calling rustydb_free_result().
///
/// # Safety
/// Both handle and sql pointers must be valid.
/// The sql pointer must point to a null-terminated string.
///
/// # Example (C)
/// ```c
/// rustydb_result_t* result = rustydb_query(handle, "SELECT * FROM users");
/// if (result == NULL) {
///     fprintf(stderr, "Query failed: %s\n", rustydb_error_message(handle));
///     return 1;
/// }
/// rustydb_free_result(result);
/// ```
#[no_mangle]
pub unsafe extern "C" fn rustydb_query(
    handle: *mut rustydb_handle_t,
    sql: *const c_char,
) -> *mut rustydb_result_t {
    // Validate inputs
    if handle.is_null() || sql.is_null() {
        return ptr::null_mut();
    }

    let handle_ptr = handle as *mut RustyDbHandle;
    let handle_ref = &mut *handle_ptr;

    // Clear previous errors
    handle_ref.clear_error();

    // Convert SQL string
    let sql_str = match c_char_to_string(sql) {
        Some(s) => s,
        None => {
            handle_ref.set_error(DbError::InvalidInput("Invalid SQL string".to_string()));
            return ptr::null_mut();
        }
    };

    // Execute query
    // In a real implementation, this would parse and execute the SQL
    // For v0.6, we return a mock result
    let result = match execute_sql_query(&sql_str) {
        Ok(res) => res,
        Err(e) => {
            handle_ref.set_error(e);
            return ptr::null_mut();
        }
    };

    // Box the result and return
    let boxed_result = Box::new(result);
    Box::into_raw(boxed_result) as *mut rustydb_result_t
}

/// Free a query result
///
/// Releases all resources associated with a query result.
/// After this call, the result pointer is invalid and must not be used.
///
/// # Parameters
/// - `result`: Pointer to a rustydb_result_t
///
/// # Safety
/// The result pointer must be valid and have been returned by rustydb_query().
/// Calling this function with NULL is safe (no-op).
///
/// # Example (C)
/// ```c
/// rustydb_free_result(result);
/// result = NULL; // Good practice
/// ```
#[no_mangle]
pub unsafe extern "C" fn rustydb_free_result(result: *mut rustydb_result_t) {
    if result.is_null() {
        return;
    }

    // Convert back to Box and let it drop
    let result_ptr = result as *mut RustyDbResult;
    let boxed_result = Box::from_raw(result_ptr);
    drop(boxed_result);
}

/// Get the number of rows affected by the last query
///
/// Returns the number of rows affected by the last INSERT, UPDATE, DELETE,
/// or the number of rows returned by a SELECT query.
///
/// # Parameters
/// - `result`: Pointer to a valid rustydb_result_t
///
/// # Returns
/// - Number of rows affected/returned
/// - -1 if result is NULL
///
/// # Safety
/// The result pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn rustydb_result_rows_affected(result: *const rustydb_result_t) -> i64 {
    if result.is_null() {
        return -1;
    }

    let result_ptr = result as *const RustyDbResult;
    let result_ref = &*result_ptr;

    result_ref.rows_affected
}

/// Get the result data as a JSON string
///
/// Returns the query results as a JSON-formatted string.
/// The string is owned by the result and should NOT be freed by the caller.
///
/// # Parameters
/// - `result`: Pointer to a valid rustydb_result_t
///
/// # Returns
/// - Pointer to a null-terminated JSON string
/// - NULL if no data or if result is NULL
///
/// # Safety
/// The result pointer must be valid.
/// The returned string pointer is only valid until rustydb_free_result is called.
#[no_mangle]
pub unsafe extern "C" fn rustydb_result_data_json(result: *const rustydb_result_t) -> *const c_char {
    if result.is_null() {
        return ptr::null();
    }

    let result_ptr = result as *const RustyDbResult;
    let result_ref = &*result_ptr;

    match &result_ref.data {
        Some(json) => {
            // Use thread-local storage to keep the CString alive
            thread_local! {
                static JSON_BUF: std::cell::RefCell<Option<std::ffi::CString>> = std::cell::RefCell::new(None);
            }

            JSON_BUF.with(|buf| {
                let c_str = std::ffi::CString::new(json.as_str()).unwrap_or_else(|_| {
                    std::ffi::CString::new("{}").unwrap()
                });
                let ptr = c_str.as_ptr();
                *buf.borrow_mut() = Some(c_str);
                ptr
            })
        }
        None => ptr::null(),
    }
}

// ============================================================================
// Transaction Control
// ============================================================================

/// Begin a new transaction
///
/// Starts a new transaction on the given connection.
/// Transactions provide ACID guarantees for database operations.
///
/// # Parameters
/// - `handle`: Pointer to a valid rustydb_handle_t
///
/// # Returns
/// - RUSTYDB_OK (0) on success
/// - RUSTYDB_ERROR (-1) on failure (check rustydb_error_message for details)
///
/// # Safety
/// The handle pointer must be valid.
///
/// # Example (C)
/// ```c
/// if (rustydb_begin(handle) != RUSTYDB_OK) {
///     fprintf(stderr, "Failed to begin transaction: %s\n",
///             rustydb_error_message(handle));
///     return 1;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn rustydb_begin(handle: *mut rustydb_handle_t) -> c_int {
    if handle.is_null() {
        return RUSTYDB_ERROR;
    }

    let handle_ptr = handle as *mut RustyDbHandle;
    let handle_ref = &mut *handle_ptr;

    // Clear previous errors
    handle_ref.clear_error();

    // Check if already in transaction
    if handle_ref.state == ConnectionState::InTransaction {
        handle_ref.set_error(DbError::InvalidOperation(
            "Already in a transaction".to_string()
        ));
        return RUSTYDB_ERROR;
    }

    // Begin transaction
    // In a real implementation, this would start a database transaction
    // For v0.6, we just update the state
    handle_ref.state = ConnectionState::InTransaction;
    handle_ref.transaction_id = Some(generate_transaction_id());

    RUSTYDB_OK
}

/// Commit the current transaction
///
/// Commits all changes made in the current transaction, making them permanent.
///
/// # Parameters
/// - `handle`: Pointer to a valid rustydb_handle_t
///
/// # Returns
/// - RUSTYDB_OK (0) on success
/// - RUSTYDB_ERROR (-1) on failure (check rustydb_error_message for details)
///
/// # Safety
/// The handle pointer must be valid.
///
/// # Example (C)
/// ```c
/// if (rustydb_commit(handle) != RUSTYDB_OK) {
///     fprintf(stderr, "Failed to commit: %s\n", rustydb_error_message(handle));
///     rustydb_rollback(handle); // Try to rollback
///     return 1;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn rustydb_commit(handle: *mut rustydb_handle_t) -> c_int {
    if handle.is_null() {
        return RUSTYDB_ERROR;
    }

    let handle_ptr = handle as *mut RustyDbHandle;
    let handle_ref = &mut *handle_ptr;

    // Clear previous errors
    handle_ref.clear_error();

    // Check if in transaction
    if handle_ref.state != ConnectionState::InTransaction {
        handle_ref.set_error(DbError::InvalidOperation(
            "Not in a transaction".to_string()
        ));
        return RUSTYDB_ERROR;
    }

    // Commit transaction
    // In a real implementation, this would commit the database transaction
    // For v0.6, we just update the state
    handle_ref.state = ConnectionState::Active;
    handle_ref.transaction_id = None;

    RUSTYDB_OK
}

/// Rollback the current transaction
///
/// Rolls back all changes made in the current transaction, discarding them.
///
/// # Parameters
/// - `handle`: Pointer to a valid rustydb_handle_t
///
/// # Returns
/// - RUSTYDB_OK (0) on success
/// - RUSTYDB_ERROR (-1) on failure (check rustydb_error_message for details)
///
/// # Safety
/// The handle pointer must be valid.
///
/// # Example (C)
/// ```c
/// if (rustydb_rollback(handle) != RUSTYDB_OK) {
///     fprintf(stderr, "Failed to rollback: %s\n", rustydb_error_message(handle));
///     return 1;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn rustydb_rollback(handle: *mut rustydb_handle_t) -> c_int {
    if handle.is_null() {
        return RUSTYDB_ERROR;
    }

    let handle_ptr = handle as *mut RustyDbHandle;
    let handle_ref = &mut *handle_ptr;

    // Clear previous errors
    handle_ref.clear_error();

    // Check if in transaction
    if handle_ref.state != ConnectionState::InTransaction {
        handle_ref.set_error(DbError::InvalidOperation(
            "Not in a transaction".to_string()
        ));
        return RUSTYDB_ERROR;
    }

    // Rollback transaction
    // In a real implementation, this would rollback the database transaction
    // For v0.6, we just update the state
    handle_ref.state = ConnectionState::Active;
    handle_ref.transaction_id = None;

    RUSTYDB_OK
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Free a string returned by the RustyDB API
///
/// Some API functions return strings that must be freed by the caller.
/// This function should be used to free those strings.
///
/// # Parameters
/// - `str`: Pointer to a string returned by a RustyDB function
///
/// # Safety
/// The string pointer must have been allocated by RustyDB.
/// Calling this function with NULL is safe (no-op).
#[no_mangle]
pub unsafe extern "C" fn rustydb_free_string(str: *mut c_char) {
    if str.is_null() {
        return;
    }

    // Convert back to CString and let it drop
    let _ = std::ffi::CString::from_raw(str);
}

/// Get the RustyDB version string
///
/// Returns a static string containing the RustyDB version.
/// The string does not need to be freed.
///
/// # Returns
/// - Pointer to a null-terminated version string
///
/// # Safety
/// This function is always safe to call.
///
/// # Example (C)
/// ```c
/// printf("RustyDB version: %s\n", rustydb_version());
/// ```
#[no_mangle]
pub unsafe extern "C" fn rustydb_version() -> *const c_char {
    concat!("RustyDB ", env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

// ============================================================================
// Internal Helper Functions
// ============================================================================

/// Execute a SQL query (internal implementation)
fn execute_sql_query(sql: &str) -> Result<RustyDbResult, DbError> {
    // For v0.6, this is a mock implementation
    // A real implementation would:
    // 1. Parse the SQL using the parser module
    // 2. Create an execution plan using the execution module
    // 3. Execute the plan and collect results
    // 4. Format results as JSON

    // Simple mock: return success for any query
    Ok(RustyDbResult::success(
        0,
        Some(format!(r#"{{"status":"success","query":"{}"}}"#, sql)),
        vec!["status".to_string(), "query".to_string()],
    ))
}

/// Generate a unique transaction ID
fn generate_transaction_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connect_disconnect() {
        unsafe {
            let conn_str = std::ffi::CString::new("test").unwrap();
            let handle = rustydb_connect(conn_str.as_ptr());
            assert!(!handle.is_null());

            rustydb_disconnect(handle);
        }
    }

    #[test]
    fn test_transaction_lifecycle() {
        unsafe {
            let conn_str = std::ffi::CString::new("test").unwrap();
            let handle = rustydb_connect(conn_str.as_ptr());
            assert!(!handle.is_null());

            // Begin transaction
            assert_eq!(rustydb_begin(handle), RUSTYDB_OK);

            // Commit
            assert_eq!(rustydb_commit(handle), RUSTYDB_OK);

            // Begin again
            assert_eq!(rustydb_begin(handle), RUSTYDB_OK);

            // Rollback
            assert_eq!(rustydb_rollback(handle), RUSTYDB_OK);

            rustydb_disconnect(handle);
        }
    }

    #[test]
    fn test_query_execution() {
        unsafe {
            let conn_str = std::ffi::CString::new("test").unwrap();
            let handle = rustydb_connect(conn_str.as_ptr());
            assert!(!handle.is_null());

            let sql = std::ffi::CString::new("SELECT 1").unwrap();
            let result = rustydb_query(handle, sql.as_ptr());
            assert!(!result.is_null());

            let rows = rustydb_result_rows_affected(result);
            assert_eq!(rows, 0);

            rustydb_free_result(result);
            rustydb_disconnect(handle);
        }
    }
}
