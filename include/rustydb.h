/*
 * RustyDB C API Header
 *
 * This header provides a C-compatible API for RustyDB, allowing the database
 * to be used from C, C++, and other languages that support C FFI.
 *
 * Version: 0.6.0
 * License: MIT OR Apache-2.0
 *
 * Copyright (c) 2025 RustyDB Contributors
 */

#ifndef RUSTYDB_H
#define RUSTYDB_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Version Information
 * ========================================================================== */

#define RUSTYDB_VERSION_MAJOR 0
#define RUSTYDB_VERSION_MINOR 6
#define RUSTYDB_VERSION_PATCH 0
#define RUSTYDB_API_VERSION 1

/* ============================================================================
 * Constants
 * ========================================================================== */

/** Success return code */
#define RUSTYDB_OK 0

/** Error return code */
#define RUSTYDB_ERROR -1

/* ============================================================================
 * Error Codes
 * ========================================================================== */

/** Error code categories */
#define RUSTYDB_ERR_NONE 0              /* No error */
#define RUSTYDB_ERR_IO 1000             /* I/O errors (1000-1999) */
#define RUSTYDB_ERR_PARSE 2000          /* SQL parsing errors (2000-2999) */
#define RUSTYDB_ERR_TRANSACTION 3000    /* Transaction errors (3000-3999) */
#define RUSTYDB_ERR_STORAGE 4000        /* Storage errors (4000-4999) */
#define RUSTYDB_ERR_CATALOG 5000        /* Catalog errors (5000-5999) */
#define RUSTYDB_ERR_INDEX 6000          /* Index errors (6000-6999) */
#define RUSTYDB_ERR_EXECUTION 7000      /* Execution errors (7000-7999) */
#define RUSTYDB_ERR_NETWORK 8000        /* Network errors (8000-8999) */
#define RUSTYDB_ERR_INTERNAL 9000       /* Internal errors (9000-9999) */
#define RUSTYDB_ERR_BACKUP 10000        /* Backup errors (10000-10999) */
#define RUSTYDB_ERR_REPLICATION 11000   /* Replication errors (11000-11999) */
#define RUSTYDB_ERR_ENCRYPTION 12000    /* Encryption errors (12000-12999) */
#define RUSTYDB_ERR_CONFIG 13000        /* Configuration errors (13000-13999) */
#define RUSTYDB_ERR_SECURITY 14000      /* Security errors (14000-14999) */

/* Specific error codes */
#define RUSTYDB_ERR_LOCK_TIMEOUT 3001   /* Lock acquisition timeout */
#define RUSTYDB_ERR_LOCK_ERROR 3002     /* Lock error */
#define RUSTYDB_ERR_DEADLOCK 3003       /* Deadlock detected */
#define RUSTYDB_ERR_NOT_FOUND 4001      /* Resource not found */
#define RUSTYDB_ERR_ALREADY_EXISTS 4002 /* Resource already exists */
#define RUSTYDB_ERR_INVALID_INPUT 2001  /* Invalid input */
#define RUSTYDB_ERR_INVALID_OP 2002     /* Invalid operation */
#define RUSTYDB_ERR_AUTH_FAILED 14001   /* Authentication failed */

/* ============================================================================
 * Opaque Types
 * ========================================================================== */

/**
 * Opaque handle to a database connection.
 *
 * This handle represents a connection to a RustyDB database.
 * It must be created with rustydb_connect() and freed with rustydb_disconnect().
 */
typedef struct rustydb_handle_t rustydb_handle_t;

/**
 * Opaque handle to a query result.
 *
 * This handle represents the result of a query execution.
 * It must be created by rustydb_query() and freed with rustydb_free_result().
 */
typedef struct rustydb_result_t rustydb_result_t;

/* ============================================================================
 * Connection Management
 * ========================================================================== */

/**
 * Connect to a RustyDB database.
 *
 * Creates a new database connection using the provided connection string.
 *
 * Connection string format:
 *   "host=<hostname>;port=<port>;database=<dbname>;user=<username>;password=<password>"
 *
 * Example:
 *   "host=localhost;port=5432;database=mydb;user=admin;password=secret"
 *
 * @param connection_string Null-terminated connection string
 * @return Non-NULL handle on success, NULL on failure
 *
 * @note The returned handle must be freed with rustydb_disconnect()
 * @note This function is thread-safe
 *
 * Example:
 *   rustydb_handle_t* db = rustydb_connect("host=localhost;port=5432");
 *   if (db == NULL) {
 *       fprintf(stderr, "Failed to connect\n");
 *       return 1;
 *   }
 */
rustydb_handle_t* rustydb_connect(const char* connection_string);

/**
 * Disconnect from the database and free the handle.
 *
 * Closes the database connection and releases all associated resources.
 * After this call, the handle pointer is invalid and must not be used.
 *
 * @param handle Database handle (can be NULL)
 *
 * @note Calling with NULL is safe (no-op)
 * @note The handle must not be used after this call
 *
 * Example:
 *   rustydb_disconnect(db);
 *   db = NULL;  // Good practice
 */
void rustydb_disconnect(rustydb_handle_t* handle);

/* ============================================================================
 * Query Execution
 * ========================================================================== */

/**
 * Execute a SQL query.
 *
 * Executes the provided SQL statement and returns the results.
 *
 * @param handle Database handle
 * @param sql Null-terminated SQL query string
 * @return Non-NULL result handle on success, NULL on failure
 *
 * @note The returned result must be freed with rustydb_free_result()
 * @note On failure, use rustydb_error_message() to get error details
 *
 * Example:
 *   rustydb_result_t* result = rustydb_query(db, "SELECT * FROM users");
 *   if (result == NULL) {
 *       fprintf(stderr, "Query failed: %s\n", rustydb_error_message(db));
 *       return 1;
 *   }
 */
rustydb_result_t* rustydb_query(rustydb_handle_t* handle, const char* sql);

/**
 * Free a query result.
 *
 * Releases all resources associated with a query result.
 * After this call, the result pointer is invalid and must not be used.
 *
 * @param result Result handle (can be NULL)
 *
 * @note Calling with NULL is safe (no-op)
 * @note The result must not be used after this call
 *
 * Example:
 *   rustydb_free_result(result);
 *   result = NULL;  // Good practice
 */
void rustydb_free_result(rustydb_result_t* result);

/**
 * Get the number of rows affected by the query.
 *
 * Returns the number of rows affected by an INSERT, UPDATE, or DELETE,
 * or the number of rows returned by a SELECT query.
 *
 * @param result Result handle
 * @return Number of rows affected/returned, or -1 if result is NULL
 *
 * Example:
 *   int64_t rows = rustydb_result_rows_affected(result);
 *   printf("Rows affected: %lld\n", (long long)rows);
 */
int64_t rustydb_result_rows_affected(const rustydb_result_t* result);

/**
 * Get the result data as a JSON string.
 *
 * Returns the query results formatted as a JSON string.
 *
 * @param result Result handle
 * @return Pointer to JSON string, or NULL if no data
 *
 * @note The string is owned by the result and should NOT be freed
 * @note The string is only valid until rustydb_free_result() is called
 *
 * Example:
 *   const char* json = rustydb_result_data_json(result);
 *   if (json != NULL) {
 *       printf("Results: %s\n", json);
 *   }
 */
const char* rustydb_result_data_json(const rustydb_result_t* result);

/* ============================================================================
 * Transaction Control
 * ========================================================================== */

/**
 * Begin a new transaction.
 *
 * Starts a new transaction on the given connection.
 * Transactions provide ACID guarantees for database operations.
 *
 * @param handle Database handle
 * @return RUSTYDB_OK on success, RUSTYDB_ERROR on failure
 *
 * @note Use rustydb_error_message() to get error details on failure
 * @note Only one transaction can be active per handle at a time
 *
 * Example:
 *   if (rustydb_begin(db) != RUSTYDB_OK) {
 *       fprintf(stderr, "Failed to begin: %s\n", rustydb_error_message(db));
 *       return 1;
 *   }
 */
int rustydb_begin(rustydb_handle_t* handle);

/**
 * Commit the current transaction.
 *
 * Commits all changes made in the current transaction, making them permanent.
 *
 * @param handle Database handle
 * @return RUSTYDB_OK on success, RUSTYDB_ERROR on failure
 *
 * @note Use rustydb_error_message() to get error details on failure
 *
 * Example:
 *   if (rustydb_commit(db) != RUSTYDB_OK) {
 *       fprintf(stderr, "Commit failed: %s\n", rustydb_error_message(db));
 *       rustydb_rollback(db);
 *       return 1;
 *   }
 */
int rustydb_commit(rustydb_handle_t* handle);

/**
 * Rollback the current transaction.
 *
 * Rolls back all changes made in the current transaction, discarding them.
 *
 * @param handle Database handle
 * @return RUSTYDB_OK on success, RUSTYDB_ERROR on failure
 *
 * @note Use rustydb_error_message() to get error details on failure
 *
 * Example:
 *   if (rustydb_rollback(db) != RUSTYDB_OK) {
 *       fprintf(stderr, "Rollback failed: %s\n", rustydb_error_message(db));
 *       return 1;
 *   }
 */
int rustydb_rollback(rustydb_handle_t* handle);

/* ============================================================================
 * Error Handling
 * ========================================================================== */

/**
 * Get the last error message from a handle.
 *
 * Returns a detailed error message for the last error that occurred
 * on this handle.
 *
 * @param handle Database handle
 * @return Pointer to error message, or NULL if no error
 *
 * @note The string is owned by the handle and should NOT be freed
 * @note The string is only valid until the next API call on this handle
 *
 * Example:
 *   if (rustydb_query(db, "INVALID SQL") == NULL) {
 *       fprintf(stderr, "Error: %s\n", rustydb_error_message(db));
 *   }
 */
const char* rustydb_error_message(const rustydb_handle_t* handle);

/**
 * Get the last error code from a handle.
 *
 * Returns the error code of the last error that occurred on this handle.
 *
 * @param handle Database handle
 * @return Error code, or 0 if no error
 *
 * @see Error code constants (RUSTYDB_ERR_*)
 *
 * Example:
 *   int code = rustydb_error_code(db);
 *   if (code != 0) {
 *       fprintf(stderr, "Error code: %d\n", code);
 *   }
 */
int rustydb_error_code(const rustydb_handle_t* handle);

/**
 * Clear the last error from a handle.
 *
 * Clears any error state on the handle, resetting the error code to 0
 * and clearing the error message.
 *
 * @param handle Database handle (can be NULL)
 *
 * @note Calling with NULL is safe (no-op)
 *
 * Example:
 *   rustydb_clear_error(db);
 */
void rustydb_clear_error(rustydb_handle_t* handle);

/**
 * Get a human-readable description of an error code.
 *
 * Returns a static string describing the error category.
 *
 * @param error_code Error code
 * @return Static string (does not need to be freed)
 *
 * Example:
 *   int code = rustydb_error_code(db);
 *   printf("Error category: %s\n", rustydb_error_description(code));
 */
const char* rustydb_error_description(int error_code);

/* ============================================================================
 * Utility Functions
 * ========================================================================== */

/**
 * Free a string returned by the RustyDB API.
 *
 * Some API functions return strings that must be freed by the caller.
 * This function should be used to free those strings.
 *
 * @param str String pointer (can be NULL)
 *
 * @note Only use this for strings that are documented as requiring freeing
 * @note Calling with NULL is safe (no-op)
 */
void rustydb_free_string(char* str);

/**
 * Get the RustyDB version string.
 *
 * Returns a static string containing the RustyDB version.
 *
 * @return Static version string (does not need to be freed)
 *
 * Example:
 *   printf("RustyDB version: %s\n", rustydb_version());
 */
const char* rustydb_version(void);

/* ============================================================================
 * Platform-Specific Notes
 * ========================================================================== */

/*
 * WINDOWS:
 * --------
 * When using RustyDB from Windows, link against rustydb.dll.
 * The import library (rustydb.lib) is automatically generated.
 *
 * MSVC:
 *   cl myapp.c /link /LIBPATH:target\release rustydb.lib
 *
 * MinGW:
 *   gcc -o myapp.exe myapp.c -L./target/release -lrustydb
 *
 * Make sure rustydb.dll is in your PATH or in the same directory as your executable.
 *
 *
 * LINUX:
 * ------
 * When using RustyDB from Linux, link against librustydb.so.
 *
 * GCC:
 *   gcc -o myapp myapp.c -L./target/release -lrustydb
 *
 * Make sure librustydb.so is in your LD_LIBRARY_PATH or /usr/lib.
 *
 *
 * MACOS:
 * ------
 * When using RustyDB from macOS, link against librustydb.dylib.
 *
 * Clang:
 *   clang -o myapp myapp.c -L./target/release -lrustydb
 *
 * Make sure librustydb.dylib is in your DYLD_LIBRARY_PATH or /usr/local/lib.
 */

#ifdef __cplusplus
}
#endif

#endif /* RUSTYDB_H */
