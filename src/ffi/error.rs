// FFI Error Handling
//
// Provides C-compatible error handling functions for the RustyDB FFI layer.

use std::os::raw::c_char;
use super::types::{rustydb_handle_t, RustyDbHandle, string_to_c_char};

/// Get the last error message from a handle
///
/// Returns a pointer to a null-terminated string containing the error message.
/// The string is owned by the handle and should NOT be freed by the caller.
/// Returns NULL if there is no error.
///
/// # Safety
/// The handle pointer must be valid and point to a live RustyDbHandle.
/// The returned string pointer is only valid until the next API call on this handle.
#[no_mangle]
pub unsafe extern "C" fn rustydb_error_message(handle: *const rustydb_handle_t) -> *const c_char {
    if handle.is_null() {
        return std::ptr::null();
    }

    let handle_ptr = handle as *const RustyDbHandle;
    let handle_ref = &*handle_ptr;

    match &handle_ref.last_error_message {
        Some(msg) => {
            // We need to return a stable pointer, so we'll use a thread-local storage
            // This is a simple implementation; a production version might use a more
            // sophisticated approach
            thread_local! {
                static ERROR_BUF: std::cell::RefCell<Option<std::ffi::CString>> = std::cell::RefCell::new(None);
            }

            ERROR_BUF.with(|buf| {
                let c_str = std::ffi::CString::new(msg.as_str()).unwrap_or_else(|_| {
                    std::ffi::CString::new("Error message contains null bytes").unwrap()
                });
                let ptr = c_str.as_ptr();
                *buf.borrow_mut() = Some(c_str);
                ptr
            })
        }
        None => std::ptr::null(),
    }
}

/// Get the last error code from a handle
///
/// Returns the error code of the last error that occurred on this handle.
/// Returns 0 if there is no error.
///
/// Error codes:
/// - 0: No error
/// - 1000-1999: I/O errors
/// - 2000-2999: SQL parsing and validation errors
/// - 3000-3999: Transaction errors
/// - 4000-4999: Storage errors
/// - 5000-5999: Catalog errors
/// - 6000-6999: Index errors
/// - 7000-7999: Execution errors
/// - 8000-8999: Network errors
/// - 9000-9999: Serialization and internal errors
/// - 10000-10999: Backup errors
/// - 11000-11999: Replication errors
/// - 12000-12999: Encryption errors
/// - 13000-13999: Configuration errors
/// - 14000-14999: Security and authentication errors
///
/// # Safety
/// The handle pointer must be valid and point to a live RustyDbHandle.
#[no_mangle]
pub unsafe extern "C" fn rustydb_error_code(handle: *const rustydb_handle_t) -> i32 {
    if handle.is_null() {
        return -1;
    }

    let handle_ptr = handle as *const RustyDbHandle;
    let handle_ref = &*handle_ptr;

    handle_ref.last_error_code
}

/// Clear the last error from a handle
///
/// Clears any error state on the handle, resetting the error code to 0
/// and clearing the error message.
///
/// # Safety
/// The handle pointer must be valid and point to a live RustyDbHandle.
#[no_mangle]
pub unsafe extern "C" fn rustydb_clear_error(handle: *mut rustydb_handle_t) {
    if handle.is_null() {
        return;
    }

    let handle_ptr = handle as *mut RustyDbHandle;
    let handle_ref = &mut *handle_ptr;

    handle_ref.clear_error();
}

/// Get a human-readable description of an error code
///
/// Returns a static string describing the error category.
/// The returned string does not need to be freed.
///
/// # Safety
/// This function is safe to call from C.
#[no_mangle]
pub unsafe extern "C" fn rustydb_error_description(error_code: i32) -> *const c_char {
    let description = match error_code {
        0 => "No error\0",
        1000..=1999 => "I/O error\0",
        2000..=2999 => "SQL parsing or validation error\0",
        3000..=3999 => "Transaction error\0",
        4000..=4999 => "Storage error\0",
        5000..=5999 => "Catalog error\0",
        6000..=6999 => "Index error\0",
        7000..=7999 => "Execution error\0",
        8000..=8999 => "Network error\0",
        9000..=9999 => "Internal error\0",
        10000..=10999 => "Backup error\0",
        11000..=11999 => "Replication error\0",
        12000..=12999 => "Encryption error\0",
        13000..=13999 => "Configuration error\0",
        14000..=14999 => "Security or authentication error\0",
        _ => "Unknown error\0",
    };

    description.as_ptr() as *const c_char
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DbError;

    #[test]
    fn test_error_handling() {
        let mut handle = RustyDbHandle::new("test".to_string());

        // Initially no error
        assert_eq!(handle.last_error_code, 0);
        assert!(handle.last_error_message.is_none());

        // Set an error
        handle.set_error(DbError::InvalidInput("test error".to_string()));
        assert_eq!(handle.last_error_code, 2001);
        assert!(handle.last_error_message.is_some());

        // Clear error
        handle.clear_error();
        assert_eq!(handle.last_error_code, 0);
        assert!(handle.last_error_message.is_none());
    }

    #[test]
    fn test_error_codes() {
        let io_error = DbError::Io(std::sync::Arc::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "test",
        )));
        assert_eq!(RustyDbHandle::error_to_code(&io_error), 1000);

        let sql_error = DbError::SqlParse("test".to_string());
        assert_eq!(RustyDbHandle::error_to_code(&sql_error), 2000);

        let txn_error = DbError::Transaction("test".to_string());
        assert_eq!(RustyDbHandle::error_to_code(&txn_error), 3000);
    }
}
