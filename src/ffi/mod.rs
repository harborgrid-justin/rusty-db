// FFI Module - Foreign Function Interface for RustyDB
//
// This module provides a C-compatible API for RustyDB, allowing the database
// to be used from C, C++, and other languages that support C FFI.
//
// # Architecture
//
// The FFI layer is designed with the following principles:
//
// 1. **Safety**: All unsafe operations are carefully contained and documented.
// 2. **Memory Management**: Clear ownership semantics with explicit allocation/deallocation.
// 3. **Error Handling**: Thread-safe error reporting via handle-based error storage.
// 4. **Opaque Types**: Rust implementation details are hidden behind opaque pointers.
// 5. **Thread Safety**: All exposed functions are thread-safe when used correctly.
//
// # Memory Management
//
// The FFI layer uses the following memory management strategy:
//
// - **Handles** (`rustydb_handle_t`): Allocated by `rustydb_connect()`, freed by `rustydb_disconnect()`
// - **Results** (`rustydb_result_t`): Allocated by `rustydb_query()`, freed by `rustydb_free_result()`
// - **Strings**: Most strings are owned by handles/results and should NOT be freed.
//   Exceptions are documented per function.
//
// # Error Handling
//
// Errors are reported in two ways:
//
// 1. **Return Values**: Functions return NULL or RUSTYDB_ERROR on failure
// 2. **Error Storage**: Detailed error information is stored in the handle and can be
//    retrieved using `rustydb_error_message()` and `rustydb_error_code()`
//
// # Thread Safety
//
// - Each `rustydb_handle_t` should only be used by one thread at a time
// - Multiple handles can be used concurrently from different threads
// - All global state is protected by atomic operations
//
// # Example Usage (C)
//
// ```c
// #include "rustydb.h"
//
// int main() {
//     // Connect to database
//     rustydb_handle_t* db = rustydb_connect("host=localhost;port=5432");
//     if (db == NULL) {
//         fprintf(stderr, "Failed to connect\n");
//         return 1;
//     }
//
//     // Begin transaction
//     if (rustydb_begin(db) != RUSTYDB_OK) {
//         fprintf(stderr, "Failed to begin transaction: %s\n",
//                 rustydb_error_message(db));
//         rustydb_disconnect(db);
//         return 1;
//     }
//
//     // Execute query
//     rustydb_result_t* result = rustydb_query(db, "SELECT * FROM users");
//     if (result == NULL) {
//         fprintf(stderr, "Query failed: %s\n", rustydb_error_message(db));
//         rustydb_rollback(db);
//         rustydb_disconnect(db);
//         return 1;
//     }
//
//     // Process results
//     printf("Rows affected: %lld\n", rustydb_result_rows_affected(result));
//     const char* json = rustydb_result_data_json(result);
//     if (json != NULL) {
//         printf("Results: %s\n", json);
//     }
//
//     // Clean up
//     rustydb_free_result(result);
//     rustydb_commit(db);
//     rustydb_disconnect(db);
//
//     return 0;
// }
// ```
//
// # Building
//
// To build RustyDB as a shared library:
//
// ```bash
// # Linux
// cargo build --release
// # Output: target/release/librustydb.so
//
// # Windows
// cargo build --release
// # Output: target/release/rustydb.dll
//
// # macOS
// cargo build --release
// # Output: target/release/librustydb.dylib
// ```
//
// # Linking
//
// ## Linux/macOS
// ```bash
// gcc -o myapp myapp.c -L./target/release -lrustydb
// ```
//
// ## Windows (MSVC)
// ```cmd
// cl myapp.c /link /LIBPATH:target\release rustydb.lib
// ```
//
// ## Windows (MinGW)
// ```bash
// gcc -o myapp.exe myapp.c -L./target/release -lrustydb
// ```

pub mod types;
pub mod error;
pub mod c_api;

// Re-export the main API functions for convenience
pub use c_api::{
    // Connection management
    rustydb_connect,
    rustydb_disconnect,

    // Query execution
    rustydb_query,
    rustydb_free_result,
    rustydb_result_rows_affected,
    rustydb_result_data_json,

    // Transaction control
    rustydb_begin,
    rustydb_commit,
    rustydb_rollback,

    // Utility functions
    rustydb_free_string,
    rustydb_version,
};

pub use error::{
    rustydb_error_message,
    rustydb_error_code,
    rustydb_clear_error,
    rustydb_error_description,
};

pub use types::{
    rustydb_handle_t,
    rustydb_result_t,
    RUSTYDB_OK,
    RUSTYDB_ERROR,
};

// Export version information
pub const FFI_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const FFI_API_VERSION: u32 = 1; // Increment when making breaking changes

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_module_exports() {
        // Test that all exports are accessible
        assert!(!FFI_VERSION.is_empty());
        assert_eq!(FFI_API_VERSION, 1);
    }
}
